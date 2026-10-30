#[cfg(not(target_os = "windows"))]
use std::io::Read;
pub mod acquisition;
pub mod dependencies;
pub mod progress;
pub mod release;
pub mod workflow;
#[cfg(target_os = "macos")]
use std::process::{Command, Stdio};

/// GUI and CLI consume the same event stream. The native implementation keeps
/// its line transport so elevated Windows workers can forward events unchanged.
pub fn install_with_events(
    channel: &str,
    replace_conflict: bool,
    password: Option<&str>,
    events: impl Fn(progress::Event) + Sync,
) -> Result<String, InstallError> {
    native_install(channel, replace_conflict, password, |line| {
        events(progress::Event::decode(line));
    })
}

pub fn install_mayaflux(
    channel: &str,
    replace_conflict: bool,
    password: Option<&str>,
    log: impl Fn(&str) + Sync,
) -> Result<String, InstallError> {
    install_with_events(channel, replace_conflict, password, |event| {
        log(&event.text())
    })
}

pub(crate) fn verify_weave_resources(log: &(impl Fn(&str) + Sync)) -> Result<(), String> {
    progress::Stage::Resources.report("Verifying bundled Weave templates and CLI…", log);
    crate::resources::verify_embedded()?;
    let executable =
        std::env::current_exe().map_err(|e| format!("Cannot locate Weave CLI: {e}"))?;
    if !executable.is_file() {
        return Err(
            "Weave executable is missing. Restore the downloaded Weave package and retry.".into(),
        );
    }
    log(&format!(
        "Verified {} embedded templates and scripts. No template copy is required for project creation.",
        crate::resources::RESOURCES.len()
    ));
    log(&format!(
        "The same executable provides the GUI and standalone CLI: {}. Its location is not automatically added to PATH.",
        executable.display()
    ));
    Ok(())
}

#[derive(Debug)]
pub enum InstallError {
    Conflict { installed_formula: String },
    Failed(String),
}

impl std::fmt::Display for InstallError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conflict { installed_formula } => write!(
                formatter,
                "The conflicting MayaFlux package '{installed_formula}' is installed. Rerun with --replace-conflict to remove it."
            ),
            Self::Failed(message) => formatter.write_str(message),
        }
    }
}

impl From<String> for InstallError {
    fn from(message: String) -> Self {
        Self::Failed(message)
    }
}

#[cfg(target_os = "macos")]
pub fn run_command_with_log(
    command: &mut Command,
    label: &str,
    log: &(impl Fn(&str) + Sync),
) -> Result<std::process::ExitStatus, String> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| format!("could not launch {label}: {error}"))?;
    let stdout = child.stdout.take().expect("stdout was piped");
    let stderr = child.stderr.take().expect("stderr was piped");
    let status = std::thread::scope(|scope| {
        scope.spawn(|| stream_output(stdout, log));
        scope.spawn(|| stream_output(stderr, log));
        child.wait()
    })
    .map_err(|error| format!("could not wait for {label}: {error}"))?;
    Ok(status)
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn stream_output(mut output: impl Read, log: &(impl Fn(&str) + Sync)) {
    let mut buffer = [0_u8; 4096];
    let mut pending = Vec::new();
    loop {
        let count = match output.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(count) => count,
        };
        for byte in &buffer[..count] {
            if *byte == b'\n' || *byte == b'\r' {
                if !pending.is_empty() {
                    log(&String::from_utf8_lossy(&pending));
                    pending.clear();
                }
            } else {
                pending.push(*byte);
            }
        }
    }
    if !pending.is_empty() {
        log(&String::from_utf8_lossy(&pending));
    }
}

#[cfg(target_os = "macos")]
fn native_install(
    channel: &str,
    replace_conflict: bool,
    _password: Option<&str>,
    log: impl Fn(&str) + Sync,
) -> Result<String, InstallError> {
    crate::platform::install_macos(channel, replace_conflict, &log)
}

#[cfg(target_os = "linux")]
pub fn check_mayaflux_conflict(channel: &str) -> Result<Option<String>, InstallError> {
    crate::platform::check_linux_conflict(channel)
}

#[cfg(target_os = "linux")]
fn native_install(
    channel: &str,
    replace_conflict: bool,
    password: Option<&str>,
    log: impl Fn(&str) + Sync,
) -> Result<String, InstallError> {
    crate::platform::install_linux(channel, replace_conflict, password, &log)
}

#[cfg(target_os = "windows")]
fn native_install(
    channel: &str,
    replace_conflict: bool,
    _password: Option<&str>,
    log: impl Fn(&str) + Sync,
) -> Result<String, InstallError> {
    crate::platform::install_windows(channel, replace_conflict, &log)
}
