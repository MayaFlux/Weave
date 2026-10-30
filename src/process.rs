//! Shared output streaming, execution deadlines and installer lifetime policy.
use std::{
    io::Read,
    process::{Command, Output},
    time::Duration,
};
pub trait ProcessGuard {
    fn terminate(&self);
}
pub trait ProcessRuntime {
    type Guard: ProcessGuard;
    fn prepare(&self, command: &mut Command);
    fn supervise(
        &self,
        child: &mut std::process::Child,
        installer: bool,
    ) -> Result<Option<Self::Guard>, String>;
    fn successful(&self, output: &Output, _installer: bool) -> bool {
        output.status.success()
    }
}

#[derive(Default)]
struct Lines {
    pending: Vec<u8>,
}

impl Lines {
    fn push(&mut self, bytes: &[u8], log: &mut impl FnMut(&str)) {
        for byte in bytes {
            if matches!(byte, b'\r' | b'\n') {
                self.finish(log);
            } else {
                self.pending.push(*byte);
                // A tool may produce unbounded output without a newline. Keep
                // memory bounded while retaining an incomplete UTF-8 character.
                if self.pending.len() >= 16 * 1024 {
                    let ready = match std::str::from_utf8(&self.pending) {
                        Err(error) if error.error_len().is_none() => error.valid_up_to(),
                        _ => self.pending.len(),
                    };
                    log(&String::from_utf8_lossy(&self.pending[..ready]));
                    self.pending.drain(..ready);
                }
            }
        }
    }

    fn finish(&mut self, log: &mut impl FnMut(&str)) {
        if !self.pending.is_empty() {
            log(&String::from_utf8_lossy(&self.pending));
            self.pending.clear();
        }
    }
}
#[cfg(target_os = "windows")]
pub fn run(
    command: &mut Command,
    installer: bool,
    timeout: Duration,
    log: &(impl Fn(&str) + Sync),
) -> Result<Output, String> {
    run_with(
        &crate::platform::windows::process::Runtime,
        command,
        installer,
        timeout,
        log,
    )
}
#[cfg(target_os = "windows")]
pub fn checked(
    command: &mut Command,
    installer: bool,
    log: &(impl Fn(&str) + Sync),
) -> Result<Output, String> {
    checked_with(
        &crate::platform::windows::process::Runtime,
        command,
        installer,
        log,
    )
}
pub fn checked_with<R: ProcessRuntime>(
    runtime: &R,
    command: &mut Command,
    installer: bool,
    log: &(impl Fn(&str) + Sync),
) -> Result<Output, String> {
    let result = run_with(
        runtime,
        command,
        installer,
        Duration::from_secs(if installer { 3600 } else { 30 }),
        log,
    )?;
    if runtime.successful(&result, installer) {
        Ok(result)
    } else {
        Err(format!(
            "{} exited with {}: {}",
            command.get_program().to_string_lossy(),
            result.status,
            String::from_utf8_lossy(&result.stderr)
        ))
    }
}

pub fn run_with<R: ProcessRuntime>(
    runtime: &R,
    command: &mut Command,
    installer: bool,
    timeout: Duration,
    log: &(impl Fn(&str) + Sync),
) -> Result<Output, String> {
    use std::process::Stdio;
    use std::sync::mpsc;
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    runtime.prepare(command);
    let label = command.get_program().to_string_lossy().into_owned();
    let mut child = command
        .spawn()
        .map_err(|e| format!("Could not start {label}: {e}. Check the host tool and retry"))?;
    let job = runtime.supervise(&mut child, installer)?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    std::thread::scope(|scope| {
        for (error, mut reader) in [
            (false, Box::new(stdout) as Box<dyn Read + Send>),
            (true, Box::new(stderr) as Box<dyn Read + Send>),
        ] {
            let sender = sender.clone();
            scope.spawn(move || {
                let mut buffer = [0; 4096];
                loop {
                    match reader.read(&mut buffer) {
                        Ok(0) | Err(_) => break,
                        Ok(count) => {
                            if sender.send((error, buffer[..count].to_vec())).is_err() {
                                break;
                            }
                        }
                    }
                }
            });
        }
        drop(sender);
        let mut output = Vec::new();
        let mut errors = Vec::new();
        let mut stdout_lines = Lines::default();
        let mut stderr_lines = Lines::default();
        let mut logger = |line: &str| log(line);
        let mut status = None;
        let start = std::time::Instant::now();
        let mut timed_out = false;
        loop {
            match receiver.recv_timeout(Duration::from_millis(50)) {
                Ok((error, bytes)) => {
                    if error {
                        stderr_lines.push(&bytes, &mut logger);
                    } else {
                        stdout_lines.push(&bytes, &mut logger);
                    }
                    let target = if error { &mut errors } else { &mut output };
                    target.extend(bytes);
                    if target.len() > 262144 {
                        target.drain(..target.len() - 262144);
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) if status.is_some() => break,
                _ => {}
            }
            if status.is_none() {
                status = child.try_wait().map_err(|e| {
                    if let Some(job) = &job {
                        job.terminate();
                    }
                    format!("Cannot wait for {label}: {e}")
                })?;
            }
            if !timed_out && start.elapsed() >= timeout {
                timed_out = true;
                if installer {
                    log(
                        "Installer reporting deadline reached. Waiting for the active installer to finish before allowing retry; do not start another installation.",
                    );
                } else {
                    if let Some(job) = &job {
                        job.terminate();
                    }
                    let _ = child.kill();
                }
            }
        }
        stdout_lines.finish(&mut logger);
        stderr_lines.finish(&mut logger);
        if timed_out {
            return Err(format!(
                "{label} exceeded its reporting deadline; the operation has ended. Retry to verify the resulting host state"
            ));
        }
        Ok(Output {
            status: status.unwrap(),
            stdout: output,
            stderr: errors,
        })
    })
}

#[cfg(test)]
mod framing_tests {
    use super::*;
    #[test]
    fn pipe_framing_preserves_split_unicode_and_coalesced_events() {
        let mut frames = Lines::default();
        let mut output = Vec::new();
        let mut log = |line: &str| output.push(line.to_owned());
        let message = "@weave-progress:{\"stage\":\"install\",\"detail\":\"MayaFlux…\"}\r\nnext\n";
        for byte in message.as_bytes() {
            frames.push(&[*byte], &mut log);
        }
        frames.push(b"final", &mut log);
        frames.finish(&mut log);
        assert_eq!(
            output,
            [
                "@weave-progress:{\"stage\":\"install\",\"detail\":\"MayaFlux…\"}",
                "next",
                "final"
            ]
        );
        let mut coalesced = Lines::default();
        let mut combined = Vec::new();
        coalesced.push(message.as_bytes(), &mut |line| {
            combined.push(line.to_owned())
        });
        assert_eq!(combined, output[..2]);
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
    use crate::platform::windows::process::powershell;
    #[test]
    fn controlled_process_drains_both_pipes_and_reports_failure() {
        let mut command = Command::new("powershell.exe");
        command.args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "[Console]::Out.Write(('o' * 524288)); [Console]::Error.Write(('e' * 524288)); exit 7",
        ]);
        let result = run(&mut command, false, Duration::from_secs(15), &|_| {}).unwrap();
        assert_eq!(result.status.code(), Some(7));
        assert!(!result.stdout.is_empty());
        assert!(!result.stderr.is_empty());
    }
    #[test]
    fn live_installer_is_observed_until_exit_after_reporting_deadline() {
        let directory = crate::filesystem::Temporary::create().unwrap();
        let marker = directory.0.join("finished.txt");
        let mut command = powershell(
            "Start-Sleep -Milliseconds 300; [IO.File]::WriteAllText($env:WEAVE_TEST_MARKER,'finished')",
        );
        command.env("WEAVE_TEST_MARKER", &marker);
        let result = run(&mut command, true, Duration::from_millis(20), &|_| {});
        assert!(result.is_err());
        assert_eq!(std::fs::read_to_string(marker).unwrap(), "finished");
    }
    #[test]
    fn controlled_probe_is_stopped_on_timeout() {
        let mut command = powershell("Start-Sleep -Seconds 20");
        let start = std::time::Instant::now();
        assert!(run(&mut command, false, Duration::from_millis(100), &|_| {}).is_err());
        assert!(start.elapsed() < Duration::from_secs(10));
    }
    #[test]
    fn exited_probe_cannot_leave_descendants_holding_output_pipes_open() {
        let mut command = powershell(
            "$p=[Diagnostics.ProcessStartInfo]::new('powershell.exe'); $p.UseShellExecute=$false; $p.CreateNoWindow=$true; $p.Arguments='-NoProfile -NonInteractive -Command Start-Sleep -Seconds 5'; [void][Diagnostics.Process]::Start($p); exit 0",
        );
        let start = std::time::Instant::now();
        assert!(run(&mut command, false, Duration::from_millis(1500), &|_| {}).is_err());
        assert!(
            start.elapsed() < Duration::from_secs(4),
            "Inherited pipes kept a finished probe alive"
        );
    }
}
