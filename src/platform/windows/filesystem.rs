use std::os::windows::fs::MetadataExt;
use std::path::Path;
pub fn is_link(metadata: &std::fs::Metadata) -> bool {
    metadata.file_attributes() & 0x400 != 0
}
pub fn validate_directory(path: &Path) -> Result<(), String> {
    use std::path::{Component, Prefix};
    let text = path
        .to_str()
        .ok_or("Installation path is not valid Unicode")?;
    if !path.is_absolute()
        || !matches!(path.components().next(), Some(Component::Prefix(p)) if matches!(p.kind(), Prefix::Disk(_)))
        || path.parent().is_none()
    {
        return Err(
            "Installation paths must be local, absolute directories below a drive root".into(),
        );
    }
    for component in path.components() {
        if matches!(component, Component::ParentDir | Component::CurDir) {
            return Err("Installation path contains a relative component".into());
        }
        if let Component::Normal(name) = component {
            let name = name.to_str().ok_or("Invalid path component")?;
            let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
            if name.ends_with(['.', ' '])
                || name
                    .chars()
                    .any(|c| c.is_control() || "<>:\"|?*;".contains(c))
                || ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
                || (stem.len() == 4
                    && (stem.starts_with("COM") || stem.starts_with("LPT"))
                    && matches!(stem.as_bytes()[3], b'1'..=b'9'))
            {
                return Err(format!("Unsafe Windows directory name: {name}"));
            }
        }
    }
    for variable in [
        "WINDIR",
        "SystemRoot",
        "ProgramFiles",
        "ProgramFiles(x86)",
        "USERPROFILE",
    ] {
        if std::env::var(variable).is_ok_and(|protected| {
            protected
                .trim_end_matches(['\\', '/'])
                .eq_ignore_ascii_case(text.trim_end_matches(['\\', '/']))
        }) {
            return Err("A system or profile directory cannot be an installation directory".into());
        }
    }
    Ok(())
}
