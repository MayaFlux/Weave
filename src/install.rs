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
                "The conflicting MayaFlux formula '{installed_formula}' is installed. Rerun with --replace-conflict to remove it."
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
pub fn install_mayaflux(
    channel: &str,
    replace_conflict: bool,
    log: impl Fn(&str) + Sync,
) -> Result<String, InstallError> {
    crate::platform::install_macos(channel, replace_conflict, &log)
}

#[cfg(not(target_os = "macos"))]
pub fn install_mayaflux(
    _channel: &str,
    _replace_conflict: bool,
    _log: impl Fn(&str) + Sync,
) -> Result<String, InstallError> {
    Err(InstallError::Failed(
        "MayaFlux package installation is not yet supported on this operating system.".to_owned(),
    ))
}
