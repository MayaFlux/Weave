//! Windows adapters contain only native mechanisms and installation requirements.
mod authorization;
mod discovery;
mod environment;
pub(crate) mod filesystem;
mod install;
pub(crate) mod process;
pub use authorization::{install_windows, install_windows_worker};
pub fn shader_source_root() -> Option<std::path::PathBuf> {
    Some(std::path::PathBuf::from(
        r"C:\MayaFlux\share\MayaFlux\src\shaders",
    ))
}
