use std::path::{Component, Path};
pub fn is_link(metadata: &std::fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}
pub fn validate_directory(path: &Path) -> Result<(), String> {
    if path
        .components()
        .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
        || path
            .as_os_str()
            .to_string_lossy()
            .chars()
            .any(char::is_control)
    {
        return Err("Installation path contains unsafe components".into());
    }
    Ok(())
}
