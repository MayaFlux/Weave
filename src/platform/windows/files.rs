use std::os::windows::fs::MetadataExt;
use std::path::Path;
use std::{fs, io::Write};

pub const CORE: &[&str] = &[
    "lib/MayaFluxLib.lib",
    "bin/MayaFluxLib.dll",
    "include/MayaFlux/MayaFlux.hpp",
    "lib/cmake/MayaFlux/MayaFluxConfig.cmake",
    "lib/cmake/MayaFlux/MayaFluxConfigVersion.cmake",
];

pub fn verify(root: &Path) -> Result<(), String> {
    safe_directory(root)?;
    for relative in CORE {
        let path = root.join(relative);
        no_links(&path)?;
        if !fs::metadata(&path).is_ok_and(|m| m.is_file() && m.len() > 0) {
            return Err(format!(
                "Required MayaFlux file is missing or empty: {relative}. Retry the download to reinstall the complete package"
            ));
        }
    }
    Ok(())
}

pub fn no_links(path: &Path) -> Result<(), String> {
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) if metadata.file_attributes() & 0x400 != 0 => {
                return Err(format!(
                    "Path crosses a reparse point: {}",
                    ancestor.display()
                ));
            }
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                return Err(format!("Cannot inspect {}: {error}", ancestor.display()));
            }
            _ => {}
        }
    }
    Ok(())
}

pub fn safe_directory(path: &Path) -> Result<(), String> {
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
    no_links(path)
}

pub const RESOURCES: &[(&str, &str)] = &[
    (
        "CMakeLists.txt",
        include_str!("../../../templates/CMakeLists.txt"),
    ),
    (
        "CMakePresets.json",
        include_str!("../../../templates/CMakePresets.json"),
    ),
    ("main.cpp", include_str!("../../../templates/main.cpp")),
    (
        "user_project.hpp",
        include_str!("../../../templates/user_project.hpp"),
    ),
    (".gitignore", include_str!("../../../templates/.gitignore")),
    (
        "validate.ps1",
        include_str!("../../../templates/validate.ps1"),
    ),
    (
        "run-report.ps1",
        include_str!("../../../templates/run-report.ps1"),
    ),
    (
        "cmake/mayaflux.cmake",
        include_str!("../../../templates/cmake/mayaflux.cmake"),
    ),
    (
        "cmake/shaders.cmake",
        include_str!("../../../templates/cmake/shaders.cmake"),
    ),
    (
        "cmake/build_community.cmake",
        include_str!("../../../templates/cmake/build_community.cmake"),
    ),
    (
        "community/module.cmake",
        include_str!("../../../templates/community/module.cmake"),
    ),
    (
        "community/community.json",
        include_str!("../../../templates/community/community.json"),
    ),
    (
        "community/CMakePresets.json",
        include_str!("../../../templates/community/CMakePresets.json"),
    ),
    (
        "community/test/CMakeLists.txt",
        include_str!("../../../templates/community/test/CMakeLists.txt"),
    ),
    (
        "vscode/settings.json",
        include_str!("../../../templates/vscode/settings.json"),
    ),
    (
        "vscode/tasks.json",
        include_str!("../../../templates/vscode/tasks.json"),
    ),
    (
        "vscode/launch.json",
        include_str!("../../../templates/vscode/launch.json"),
    ),
];

pub fn deploy_resources(root: &Path) -> Result<(), String> {
    safe_directory(root)?;
    let templates = root.join("share/weave/templates");
    for (relative, contents) in RESOURCES {
        if contents.is_empty() {
            return Err(format!("Packaged template is empty: {relative}"));
        }
        no_links(&templates.join(relative))?;
    }
    for (relative, contents) in RESOURCES {
        let target = templates.join(relative);
        no_links(&target)?;
        fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
        let temporary =
            target.with_extension(format!("weave-{}-{}.tmp", std::process::id(), unique_id()));
        let result = (|| {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)?;
            file.write_all(contents.as_bytes())?;
            file.sync_all()?;
            drop(file);
            no_links(&target).map_err(std::io::Error::other)?;
            fs::rename(&temporary, &target)
        })();
        if let Err(error) = result {
            let cleanup = fs::remove_file(&temporary);
            return Err(format!(
                "Could not install {}: {error}. Temporary file: {} (cleanup: {cleanup:?})",
                target.display(),
                temporary.display()
            ));
        }
    }
    for (relative, contents) in RESOURCES {
        if fs::read(templates.join(relative)).map_err(|e| e.to_string())? != contents.as_bytes() {
            return Err(format!(
                "Installed template verification failed: {relative}"
            ));
        }
    }
    Ok(())
}

pub fn unique_id() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

pub struct Temporary(pub std::path::PathBuf);
impl Temporary {
    pub fn create() -> Result<Self, String> {
        let base = std::env::temp_dir();
        safe_directory(&base)?;
        let path = base.join(format!(
            "weave-install-{}-{}",
            std::process::id(),
            unique_id()
        ));
        fs::create_dir(&path)
            .map_err(|e| format!("Cannot create installer working directory: {e}"))?;
        Ok(Self(path))
    }
}
impl Drop for Temporary {
    fn drop(&mut self) {
        if no_links(&self.0).is_ok() {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "weave-install-files-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
    #[test]
    fn verifies_every_required_nonempty_core_file() {
        let root = Fixture::new();
        for relative in CORE {
            let path = root.0.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "fixture").unwrap();
        }
        assert!(verify(&root.0).is_ok());
        for relative in CORE {
            let path = root.0.join(relative);
            fs::write(&path, "").unwrap();
            assert!(verify(&root.0).unwrap_err().contains(relative));
            fs::write(path, "fixture").unwrap();
        }
    }
    #[test]
    fn resources_are_complete_and_retries_preserve_custom_files() {
        let root = Fixture::new();
        deploy_resources(&root.0).unwrap();
        let templates = root.0.join("share/weave/templates");
        for relative in [
            "main.cpp",
            "CMakePresets.json",
            "validate.ps1",
            "run-report.ps1",
            "cmake/build_community.cmake",
            "community/test/CMakeLists.txt",
            "vscode/launch.json",
        ] {
            assert!(fs::metadata(templates.join(relative)).unwrap().len() > 0);
        }
        fs::write(templates.join("custom.txt"), "user content").unwrap();
        deploy_resources(&root.0).unwrap();
        assert_eq!(
            fs::read_to_string(templates.join("custom.txt")).unwrap(),
            "user content"
        );
    }
    #[test]
    fn unsafe_installation_roots_are_rejected_before_mutation() {
        for path in [
            "",
            "relative",
            "D:relative",
            "C:\\",
            "\\\\server\\share",
            "D:\\Bad;Root",
            "D:\\Bad\nRoot",
            "D:\\CON",
            "D:\\trailing.",
        ] {
            assert!(safe_directory(Path::new(path)).is_err(), "{path}");
        }
        let root = Fixture::new();
        assert!(safe_directory(&root.0).is_ok());
    }
}
