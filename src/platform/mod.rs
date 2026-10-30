use std::path::PathBuf;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
pub fn install_windows(
    channel: &str,
    replace_conflict: bool,
    log: &(impl Fn(&str) + Sync),
) -> Result<String, crate::install::InstallError> {
    windows::install_windows(channel, replace_conflict, log)
}

#[cfg(target_os = "windows")]
pub fn install_windows_worker(
    channel: &str,
    replace_conflict: bool,
    pipe: &str,
    log: &(impl Fn(&str) + Sync),
) -> Result<String, crate::install::InstallError> {
    windows::install_windows_worker(channel, replace_conflict, Some(pipe), log)
}

#[cfg(target_os = "macos")]
pub fn install_macos(
    channel: &str,
    replace_conflict: bool,
    log: &(impl Fn(&str) + Sync),
) -> Result<String, crate::install::InstallError> {
    macos::install_macos(channel, replace_conflict, log)
}

#[cfg(target_os = "linux")]
pub fn install_linux(
    channel: &str,
    replace_conflict: bool,
    password: Option<&str>,
    log: &(impl Fn(&str) + Sync),
) -> Result<String, crate::install::InstallError> {
    linux::install_linux(channel, replace_conflict, password, log)
}

#[cfg(target_os = "linux")]
pub fn check_linux_conflict(channel: &str) -> Result<Option<String>, crate::install::InstallError> {
    linux::check_linux_conflict(channel)
}

pub fn shader_source_root() -> Result<PathBuf, String> {
    if let Some(root) = std::env::var_os("MAYAFLUX_ROOT").filter(|root| !root.is_empty()) {
        return Ok(PathBuf::from(root).join("share/MayaFlux/src/shaders"));
    }

    #[cfg(target_os = "linux")]
    if let Some(root) = linux::shader_source_root() {
        return Ok(root);
    }

    #[cfg(target_os = "macos")]
    if let Some(root) = macos::shader_source_root() {
        return Ok(root);
    }

    #[cfg(target_os = "windows")]
    if let Some(root) = windows::shader_source_root() {
        return Ok(root);
    }

    Err("MayaFlux shader sources could not be located. Set MAYAFLUX_ROOT or omit --copy-shader-sources.".to_owned())
}
