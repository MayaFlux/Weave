use std::path::PathBuf;
#[cfg(not(target_os = "windows"))]
use std::process::Child;
use std::process::{Command, Output};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

/// PowerShell's filesystem provider does not consistently accept verbatim paths.
/// Convert only at the script boundary, after the caller's filesystem checks.
pub(crate) fn validation_script_path(path: &std::path::Path) -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::{OsStrExt, OsStringExt};
        use std::path::{Component, Prefix};
        let wide: Vec<u16> = path.as_os_str().encode_wide().collect();
        let converted: Option<PathBuf> = match path.components().next() {
            Some(Component::Prefix(prefix)) => match prefix.kind() {
                Prefix::VerbatimDisk(_) => Some(std::ffi::OsString::from_wide(&wide[4..]).into()),
                Prefix::VerbatimUNC(_, _) => {
                    let mut normal = vec![b'\\' as u16, b'\\' as u16];
                    normal.extend_from_slice(&wide[8..]);
                    Some(std::ffi::OsString::from_wide(&normal).into())
                }
                _ => None,
            },
            _ => None,
        };
        if let Some(converted) = converted {
            for component in converted.components() {
                if let Component::Normal(name) = component {
                    let name = name
                        .to_str()
                        .ok_or("Project script paths must be valid Unicode.")?;
                    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
                    let reserved = ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"]
                        .contains(&stem.as_str())
                        || (stem.len() == 4
                            && (stem.starts_with("COM") || stem.starts_with("LPT"))
                            && matches!(stem.as_bytes()[3], b'1'..=b'9'));
                    if reserved
                        || name.ends_with(['.', ' '])
                        || name
                            .chars()
                            .any(|c| c.is_control() || "<>:\"|?*".contains(c))
                    {
                        return Err(format!(
                            "Windows PowerShell cannot safely address this path: {}. Use a project and tool location with ordinary Windows filenames.",
                            path.display()
                        ));
                    }
                }
            }
            if let Ok(original) = std::fs::canonicalize(path) {
                let normal = std::fs::canonicalize(&converted).map_err(|error| {
                    format!(
                        "Windows PowerShell cannot resolve {} without changing its path: {error}",
                        path.display()
                    )
                })?;
                if normal != original {
                    return Err(format!(
                        "Windows PowerShell would resolve {} to a different location. Use an ordinary Windows project path.",
                        path.display()
                    ));
                }
            }
            return Ok(converted);
        }
    }
    Ok(path.to_path_buf())
}

#[cfg(all(test, target_os = "windows"))]
mod path_tests {
    use super::*;
    #[test]
    fn script_paths_preserve_drive_unc_spaces_and_unicode() {
        for (input, expected) in [
            (
                r"\\?\C:\Project with spaces\音",
                r"C:\Project with spaces\音",
            ),
            (r"\\?\UNC\server\share\Project", r"\\server\share\Project"),
            (r"C:\Project", r"C:\Project"),
        ] {
            assert_eq!(
                validation_script_path(input.as_ref()).unwrap(),
                PathBuf::from(expected)
            );
        }
    }
    #[test]
    fn verbatim_only_names_are_rejected_instead_of_resolving_to_another_path() {
        for input in [
            r"\\?\C:\Project.\validate.ps1",
            r"\\?\C:\Project \validate.ps1",
            r"\\?\C:\NUL\app.exe",
        ] {
            assert!(
                validation_script_path(input.as_ref())
                    .unwrap_err()
                    .contains("cannot safely address")
            );
        }
    }
}

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
