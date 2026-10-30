use std::io::Read;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;

pub const PACKAGES: &[(&str, &str)] = &[
    ("Kitware.CMake", "cmake.exe"),
    ("Git.Git", "git.exe"),
    ("Ninja-build.Ninja", "ninja.exe"),
    ("LLVM.LLVM", "clang.exe"),
    ("Gyan.FFmpeg.Shared", "ffmpeg.exe"),
    ("KhronosGroup.VulkanSDK", "glslc.exe"),
];
pub fn usable(id: &str, output: &str) -> bool {
    let text = output.to_ascii_lowercase();
    let prefix = match id {
        "Kitware.CMake" => "cmake version ",
        "Git.Git" => "git version ",
        "LLVM.LLVM" => "clang version ",
        "Gyan.FFmpeg.Shared" => "ffmpeg version ",
        "KhronosGroup.VulkanSDK" => "shaderc ",
        "Ninja-build.Ninja" => "",
        _ => return false,
    };
    let Some(banner) = text
        .split_once(prefix)
        .map(|(_, value)| value)
        .filter(|_| !prefix.is_empty())
        .or_else(|| (id == "Ninja-build.Ninja").then_some(text.as_str()))
    else {
        return false;
    };
    let token = banner.split_whitespace().next().unwrap_or("");
    if id == "Gyan.FFmpeg.Shared" {
        return token.bytes().any(|b| b.is_ascii_digit());
    }
    let token = token.trim_start_matches('v');
    let numbers: Vec<_> = token
        .split('.')
        .take(2)
        .map(|part| part.parse::<u64>())
        .collect();
    if numbers.len() != 2 || numbers.iter().any(Result::is_err) {
        return false;
    }
    id != "Kitware.CMake"
        || [
            numbers[0].as_ref().copied().unwrap(),
            numbers[1].as_ref().copied().unwrap(),
        ] >= [3, 31]
}

pub fn shared_ffmpeg(root: &Path) -> bool {
    if !root.join("bin/ffmpeg.exe").is_file() {
        return false;
    }
    let Ok(entries) = std::fs::read_dir(root.join("bin")) else {
        return false;
    };
    let files: Vec<_> = entries
        .flatten()
        .filter(|e| e.path().is_file())
        .map(|e| e.file_name().to_string_lossy().to_ascii_lowercase())
        .collect();
    [
        "avcodec",
        "avformat",
        "avutil",
        "swresample",
        "swscale",
        "avdevice",
    ]
    .iter()
    .all(|component| {
        root.join(format!("lib/{component}.lib")).is_file()
            && root
                .join(format!("include/lib{component}/{component}.h"))
                .is_file()
            && files
                .iter()
                .any(|name| name.starts_with(&format!("{component}-")) && name.ends_with(".dll"))
    })
}

pub fn merge_paths(existing: &str, additions: &[PathBuf], prefer: bool) -> Result<String, String> {
    fn normalize(value: &str) -> String {
        let text = value.trim().trim_matches('"').replace('/', "\\");
        let path = Path::new(&text);
        let mut normalized = PathBuf::new();
        for component in path.components() {
            match component {
                std::path::Component::ParentDir => {
                    normalized.pop();
                }
                std::path::Component::CurDir => {}
                _ => normalized.push(component.as_os_str()),
            }
        }
        normalized
            .to_string_lossy()
            .trim_end_matches('\\')
            .to_ascii_lowercase()
    }
    for path in additions {
        super::files::safe_directory(path)?;
    }
    let new: Vec<_> = additions
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    let old: Vec<_> = existing
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    let ordered = if prefer {
        new.into_iter().chain(old).collect::<Vec<_>>()
    } else {
        old.into_iter().chain(new).collect()
    };
    let mut entries = Vec::<String>::new();
    for entry in ordered {
        if !entries
            .iter()
            .any(|old| normalize(old) == normalize(&entry))
        {
            entries.push(entry);
        }
    }
    Ok(entries.join(";"))
}

pub fn powershell(script: &str) -> Command {
    let system = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    let mut command = Command::new(system.join("System32/WindowsPowerShell/v1.0/powershell.exe"));
    command.args(["-NoProfile", "-NonInteractive", "-Command", &format!("$ErrorActionPreference='Stop'; [Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); {script}")]);
    command
}

pub fn checked(
    command: &mut Command,
    installer: bool,
    log: &(impl Fn(&str) + Sync),
) -> Result<Output, String> {
    let result = run(
        command,
        installer,
        Duration::from_secs(if installer { 3600 } else { 30 }),
        log,
    )?;
    if result.status.success() || installer && result.status.code() == Some(3010) {
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

pub fn run(
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
        .stderr(Stdio::piped())
        .creation_flags(0x08000000);
    let label = command.get_program().to_string_lossy().into_owned();
    let mut child = command
        .spawn()
        .map_err(|e| format!("Could not start {label}: {e}. Check the host tool and retry"))?;
    let job = if installer {
        None
    } else {
        Some(super::job::ProbeJob::attach(&mut child)?)
    };
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
        let mut status = None;
        let start = std::time::Instant::now();
        let mut timed_out = false;
        loop {
            match receiver.recv_timeout(Duration::from_millis(50)) {
                Ok((error, bytes)) => {
                    log(&String::from_utf8_lossy(&bytes));
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
mod tests {
    use super::*;
    #[test]
    fn only_working_supported_tool_banners_are_accepted() {
        for (id, banner) in [
            ("Kitware.CMake", "cmake version 4.3.0-rc1"),
            ("Git.Git", "git version 2.39.5.windows.1"),
            ("Ninja-build.Ninja", "1.10.2.git.kitware.jobserver-1"),
            ("LLVM.LLVM", "Vendor LLVM clang version 20.1.8"),
            ("Gyan.FFmpeg.Shared", "ffmpeg version N-123456-gabcdef"),
            (
                "KhronosGroup.VulkanSDK",
                "shaderc 2023.2\nspirv-tools v2023.2",
            ),
        ] {
            assert!(usable(id, banner), "{id}");
            assert!(!usable(id, "unrelated output"));
        }
        assert!(!usable("Kitware.CMake", "cmake version 3.25.0"));
    }
    #[test]
    fn environment_merge_deduplicates_whole_paths_and_preserves_other_entries() {
        let paths = [
            PathBuf::from(r"D:\Tools\bin"),
            PathBuf::from(r"D:\LLVM Tools\bin"),
        ];
        assert_eq!(
            merge_paths(r"E:\Old;d:\tools\BIN\;D:\Tools\binary", &paths, true).unwrap(),
            r"D:\Tools\bin;D:\LLVM Tools\bin;E:\Old;D:\Tools\binary"
        );
        assert_eq!(
            merge_paths("", &paths[..1], false).unwrap(),
            r"D:\Tools\bin"
        );
        for value in [
            "relative",
            "D:relative",
            "D:\\Bad;Injected",
            "D:\\Bad\nInjected",
        ] {
            assert!(merge_paths("", &[PathBuf::from(value)], false).is_err());
        }
    }
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
        let directory = super::super::files::Temporary::create().unwrap();
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
