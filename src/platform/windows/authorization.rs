use super::{install, process};
use crate::operations::install::{
    InstallError,
    workflow::{Channel, install},
};
use std::os::windows::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::{fs, io::Write, sync::Mutex};

pub fn install_windows(
    channel: &str,
    replace_conflict: bool,
    log: &(impl Fn(&str) + Sync),
) -> Result<String, InstallError> {
    install_windows_worker(channel, replace_conflict, None, log)
}

pub fn install_windows_worker(
    channel: &str,
    replace_conflict: bool,
    pipe: Option<&str>,
    log: &(impl Fn(&str) + Sync),
) -> Result<String, InstallError> {
    // Reject malformed input before probing the host, requesting consent or writing logs.
    let channel = Channel::parse(channel)?;
    if !replace_conflict
        && let Ok(Some(local)) =
            crate::operations::install::release::installed(std::path::Path::new(r"C:\MayaFlux"))
        && let Some(installed) =
            crate::operations::install::release::conflicting_stream(channel, &local)
    {
        return Err(InstallError::Conflict {
            installed_formula: format!("MayaFlux {}", installed.name()),
        });
    }
    if !cfg!(target_arch = "x86_64") {
        return Err(
            "An x64 Weave executable is required for Windows installation"
                .to_owned()
                .into(),
        );
    }
    let admin = process::checked(
        &mut process::powershell(
            "$identity=[Security.Principal.WindowsIdentity]::GetCurrent(); $principal=[Security.Principal.WindowsPrincipal]::new($identity); $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)",
        ),
        false,
        &|_| {},
    )?;
    let is_admin = String::from_utf8_lossy(&admin.stdout)
        .trim()
        .eq_ignore_ascii_case("true");
    if pipe.is_some() && !is_admin {
        return Err(
            "The installation worker needs administrator permission; retry installation from Weave"
                .to_owned()
                .into(),
        );
    }
    let pipe = pipe
        .map(|name| -> Result<_, String> {
            if !name.starts_with("Weave-install-")
                || name.len() > 100
                || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            {
                return Err("Invalid installation progress pipe".into());
            }
            fs::OpenOptions::new()
                .write(true)
                .security_qos_flags(0x00010000)
                .open(format!(r"\\.\pipe\{name}"))
                .map(Mutex::new)
                .map_err(|e| format!("Cannot connect installation progress: {e}"))
        })
        .transpose()?;
    let local = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| {
            "LOCALAPPDATA was not found; cannot create the installation log".to_owned()
        })?;
    crate::filesystem::safe_directory(&local)?;
    let log_path = local.join("weave_install.log");
    crate::filesystem::no_links(&log_path)?;
    if !is_admin {
        return elevate(channel, replace_conflict, &log_path, log).map_err(InstallError::Failed);
    }
    let file = Mutex::new(
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(|e| format!("Cannot open installation log {}: {e}", log_path.display()))?,
    );
    let logger = |line: &str| {
        log(line);
        if let Some(pipe) = &pipe
            && let Ok(mut pipe) = pipe.lock()
        {
            let _ = writeln!(pipe, "{line}");
        }
        if let Ok(mut file) = file.lock()
            && let Err(error) = writeln!(file, "{line}")
        {
            log(&format!("Warning: installation log write failed: {error}"));
        }
    };
    let _guard = InstallationLock::acquire()
        .inspect_err(|error| logger(&format!("[Weave ERROR] {error}")))?;
    logger(&format!(
        "\n=== Weave Windows installation: {} ===",
        channel.name()
    ));
    let mut host = install::WindowsHost {
        root: PathBuf::from(r"C:\MayaFlux"),
        log: &logger,
    };
    let result = install(channel, &mut host, &logger);
    match &result {
        Ok(message) => logger(message),
        Err(error) => logger(&format!("[Weave ERROR] {error}")),
    }
    result
        .map(|message| format!("{message}\nInstallation log: {}", log_path.display()))
        .map_err(InstallError::Failed)
}

fn elevate(
    channel: Channel,
    replace_conflict: bool,
    log_path: &std::path::Path,
    log: &(impl Fn(&str) + Sync),
) -> Result<String, String> {
    log(
        "Windows will request administrator consent for MayaFlux and host-tool installation. Cancelling leaves project and community operations available.",
    );
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let name = format!(
        "Weave-install-{}-{}",
        std::process::id(),
        crate::filesystem::unique_id()
    );
    crate::filesystem::no_links(log_path)?;
    let file = Mutex::new(
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .map_err(|e| format!("Cannot open installation log: {e}"))?,
    );
    let logger = |line: &str| {
        log(line);
        if let Ok(mut file) = file.lock() {
            let _ = writeln!(file, "{line}");
        }
    };
    let mut request = process::powershell(include_str!("scripts/elevate.ps1"));
    request
        .env("WEAVE_INSTALL_EXECUTABLE", executable)
        .env(
            "WEAVE_INSTALL_ARGUMENTS",
            format!(
                "install {} {} --windows-install-pipe={name}",
                channel.name(),
                if replace_conflict {
                    "--replace-conflict"
                } else {
                    ""
                }
            ),
        )
        .env("WEAVE_INSTALL_PIPE", &name);
    // The elevated worker holds the machine-wide lock and owns all effects. The parent
    // stays responsive and never starts a second operation while that worker is alive.
    let result = process::run(
        &mut request,
        true,
        std::time::Duration::from_secs(24 * 3600),
        &logger,
    )?;
    match result.status.code() {
        Some(0) => Ok(format!("MayaFlux {} installation completed. Review the completion details above and restart Windows if requested. Installation log: {}", channel.name(), log_path.display())),
        Some(1223) => Err("Administrator consent was cancelled. No installation was started. Retry installation or continue with project and community operations.".into()),
        _ => Err(format!("Windows installation did not complete ({}). Review {} and the output above, correct the problem and retry. Project and community operations remain available.", result.status, log_path.display())),
    }
}

struct InstallationLock(*mut std::ffi::c_void);
#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateMutexW(
        attributes: *const std::ffi::c_void,
        owner: i32,
        name: *const u16,
    ) -> *mut std::ffi::c_void;
    fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
    fn GetLastError() -> u32;
}
impl InstallationLock {
    fn acquire() -> Result<Self, String> {
        let name: Vec<u16> = "Global\\Weave.MayaFlux.Install\0".encode_utf16().collect();
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        let error = unsafe { GetLastError() };
        if handle.is_null() {
            return Err(format!(
                "Cannot acquire the MayaFlux installation lock (Windows error {error}). Check whether another installation is active and retry"
            ));
        }
        if error == 183 {
            unsafe {
                CloseHandle(handle);
            }
            return Err("Another MayaFlux installation is still running. Wait for it to finish before retrying".into());
        }
        Ok(Self(handle))
    }
}
impl Drop for InstallationLock {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

#[cfg(test)]
mod elevation_tests {
    use super::*;
    #[test]
    fn native_consent_cancellation_is_recoverable_without_starting_installation() {
        let script = format!(
            "function Start-Process {{ throw [ComponentModel.Win32Exception]::new(1223) }}\n{}",
            include_str!("scripts/elevate.ps1")
        );
        let mut command = process::powershell(&script);
        command
            .env(
                "WEAVE_INSTALL_PIPE",
                format!("Weave-install-cancel-{}", crate::filesystem::unique_id()),
            )
            .env("WEAVE_INSTALL_EXECUTABLE", r"D:\User Tools\Weave.exe")
            .env("WEAVE_INSTALL_ARGUMENTS", "install stable");
        let result = process::run(
            &mut command,
            false,
            std::time::Duration::from_secs(15),
            &|_| {},
        )
        .unwrap();
        assert_eq!(result.status.code(), Some(1223));
        assert!(String::from_utf8_lossy(&result.stderr).contains("project operations"));
    }
    #[test]
    fn elevated_worker_progress_does_not_depend_on_the_workers_profile_directory() {
        let fake = r#"
            function Start-Process {
                param($FilePath,$ArgumentList,$Verb,[switch]$PassThru,$WindowStyle)
                if ($FilePath -ne 'D:\User Tools\Weave.exe' -or $Verb -ne 'RunAs' -or $WindowStyle -ne 'Hidden') { throw 'Incorrect native handoff' }
                $fixture = @'
$env:LOCALAPPDATA='D:\Different Administrator Profile'
$client=[IO.Pipes.NamedPipeClientStream]::new('.', $env:WEAVE_INSTALL_PIPE, [IO.Pipes.PipeDirection]::Out)
$client.Connect(3000)
$writer=[IO.StreamWriter]::new($client)
$writer.AutoFlush=$true
for ($stage=1; $stage -le 6; $stage++) {
    $writer.WriteLine("Step $stage/6: controlled installation progress")
    Start-Sleep -Milliseconds 100
}
$writer.WriteLine('Different administrator profile: Restart Windows before building projects.')
[IO.File]::WriteAllText($env:WEAVE_TEST_COMPLETED_MARKER,'finished')
$writer.Dispose()
'@
                $request=[Diagnostics.ProcessStartInfo]::new('powershell.exe')
                $request.UseShellExecute=$false
                $request.CreateNoWindow=$true
                $request.Arguments='-NoProfile -NonInteractive -EncodedCommand '+[Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($fixture))
                return [Diagnostics.Process]::Start($request)
            }
        "#;
        let mut command =
            process::powershell(&format!("{fake}\n{}", include_str!("scripts/elevate.ps1")));
        let temporary = crate::filesystem::Temporary::create().unwrap();
        let completed_marker = temporary.0.join("completed.txt");
        command
            .env(
                "WEAVE_INSTALL_PIPE",
                format!("Weave-install-progress-{}", crate::filesystem::unique_id()),
            )
            .env("WEAVE_INSTALL_EXECUTABLE", r"D:\User Tools\Weave.exe")
            .env("WEAVE_INSTALL_ARGUMENTS", "install development")
            .env("WEAVE_TEST_COMPLETED_MARKER", &completed_marker);
        let progress = std::sync::Mutex::new(String::new());
        let received_while_running = std::sync::atomic::AtomicBool::new(false);
        let result = process::run(
            &mut command,
            false,
            std::time::Duration::from_secs(15),
            &|line| {
                if line.contains("Step 1/6:") && !completed_marker.exists() {
                    received_while_running.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                progress.lock().unwrap().push_str(line);
            },
        )
        .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(
            String::from_utf8_lossy(&result.stdout)
                .contains("Restart Windows before building projects")
        );
        let progress = progress.into_inner().unwrap();
        assert!(
            received_while_running.load(std::sync::atomic::Ordering::Relaxed),
            "Progress was buffered until installation finished"
        );
        for stage in 1..=6 {
            assert!(
                progress.contains(&format!("Step {stage}/6:")),
                "Missing live popup progress for stage {stage}: {progress}"
            );
        }
    }
}
