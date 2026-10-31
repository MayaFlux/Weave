use std::path::PathBuf;
use std::process::{Child, Command, Output};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

pub(crate) fn validation_interpreter() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        let system_root = std::env::var_os("SystemRoot")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
        let powershell = system_root.join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
        if !powershell.is_file() {
            return Err(format!(
                "Windows PowerShell was not found at {}. Repair Windows PowerShell, then retry.",
                powershell.display()
            ));
        }
        Ok(powershell)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let bash = PathBuf::from("/bin/bash");
        if !bash.is_file() {
            return Err("Bash was not found at /bin/bash. Install Bash, then retry.".to_owned());
        }
        Ok(bash)
    }
}

pub(crate) fn run_validation_command(
    command: &mut Command,
    cancellation: &AtomicBool,
    log: &(impl Fn(&str) + Sync),
) -> Result<Output, String> {
    #[cfg(target_os = "windows")]
    let runtime = crate::platform::windows::process::Runtime;
    #[cfg(not(target_os = "windows"))]
    let runtime = ValidationRuntime;
    crate::process::run_cancellable_with(
        &runtime,
        command,
        false,
        Duration::from_secs(900),
        cancellation,
        log,
    )
}

pub(crate) fn run_project_process(
    command: &mut Command,
    cancellation: &AtomicBool,
    log: &(impl Fn(&str) + Sync),
) -> Result<Output, String> {
    #[cfg(target_os = "windows")]
    let runtime = crate::platform::windows::process::Runtime;
    #[cfg(not(target_os = "windows"))]
    let runtime = ValidationRuntime;
    crate::process::run_cancellable_until_exit_with(&runtime, command, false, cancellation, log)
}

#[cfg(not(target_os = "windows"))]
struct ValidationRuntime;

#[cfg(unix)]
struct ValidationProcessGroup(i32);

#[cfg(unix)]
unsafe extern "C" {
    fn kill(pid: i32, signal: i32) -> i32;
}

#[cfg(not(target_os = "windows"))]
impl crate::process::ProcessGuard for ValidationProcessGroup {
    fn terminate(&self) {
        unsafe {
            kill(-self.0, 9);
        }
    }
}

#[cfg(not(target_os = "windows"))]
impl crate::process::ProcessRuntime for ValidationRuntime {
    type Guard = ValidationProcessGroup;
    fn prepare(&self, command: &mut Command) {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    fn supervise(
        &self,
        child: &mut Child,
        _installer: bool,
    ) -> Result<Option<Self::Guard>, String> {
        Ok(Some(ValidationProcessGroup(child.id() as i32)))
    }
}
