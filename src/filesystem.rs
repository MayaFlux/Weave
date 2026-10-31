//! Shared path, payload, temporary-directory and filesystem operations.
use std::{
    fs, io,
    path::{Path, PathBuf},
};

/// Replace a file through a sibling temporary file so readers never see a partial write.
pub fn atomic_write(path: &Path, contents: &[u8]) -> io::Result<()> {
    atomic_write_with(
        path,
        contents,
        |file, contents| {
            use std::io::Write;
            file.write_all(contents)?;
            file.sync_all()
        },
        |temporary, destination| fs::rename(temporary, destination),
    )
}

fn atomic_write_with(
    path: &Path,
    contents: &[u8],
    write_contents: impl FnOnce(&mut fs::File, &[u8]) -> io::Result<()>,
    replace: impl FnOnce(&Path, &Path) -> io::Result<()>,
) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "destination has no parent directory",
        )
    })?;
    let existing = match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "refusing to replace a symbolic link",
            ));
        }
        Ok(metadata) => Some(metadata),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    static NEXT_TEMP: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let mut temporary = None;
    for _ in 0..100 {
        let id = NEXT_TEMP.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let candidate = parent.join(format!(".weave-{}-{id}.tmp", std::process::id()));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(mut file) => {
                let result = write_contents(&mut file, contents);
                drop(file);
                if let Err(error) = result {
                    let _ = fs::remove_file(&candidate);
                    return Err(error);
                }
                if let Some(metadata) = &existing {
                    if let Err(error) = fs::set_permissions(&candidate, metadata.permissions()) {
                        let _ = fs::remove_file(&candidate);
                        return Err(error);
                    }
                }
                temporary = Some(candidate);
                break;
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    let temporary = temporary.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not allocate a temporary file",
        )
    })?;
    match replace(&temporary, path) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = fs::remove_file(temporary);
            Err(error)
        }
    }
}
pub fn verify_payload(root: &Path, required: &[&str]) -> Result<(), String> {
    safe_directory(root)?;
    for relative in required {
        validate_relative(Path::new(relative))?;
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

pub fn validate_relative(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err(format!(
            "Payload paths must be relative and remain inside their root: {}",
            path.display()
        ));
    }
    Ok(())
}

/// Validate a native development-package layout without knowing its library naming rules.
pub fn development_package_present(
    root: &Path,
    required: &[PathBuf],
    runtime_directory: &Path,
    runtime_patterns: &[(String, String)],
    case_sensitive: bool,
) -> bool {
    if required
        .iter()
        .any(|relative| !root.join(relative).is_file())
    {
        return false;
    }
    let Ok(entries) = fs::read_dir(root.join(runtime_directory)) else {
        return false;
    };
    let names: Vec<_> = entries
        .flatten()
        .filter(|entry| entry.path().is_file())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    runtime_patterns.iter().all(|(prefix, suffix)| {
        names.iter().any(|name| {
            if case_sensitive {
                name.starts_with(prefix) && name.ends_with(suffix)
            } else {
                let name = name.to_ascii_lowercase();
                name.starts_with(&prefix.to_ascii_lowercase())
                    && name.ends_with(&suffix.to_ascii_lowercase())
            }
        })
    })
}

pub fn no_links(path: &Path) -> Result<(), String> {
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata)
                if crate::platform::native_filesystem::is_link(&metadata)
                    && !is_macos_system_alias(ancestor) =>
            {
                return Err(format!(
                    "Path crosses a reparse point or symbolic link: {}",
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

#[cfg(target_os = "macos")]
fn is_macos_system_alias(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if path.parent() != Some(Path::new("/")) {
        return false;
    }
    let target = match name {
        "var" => Path::new("/private/var"),
        "tmp" => Path::new("/private/tmp"),
        "etc" => Path::new("/private/etc"),
        _ => return false,
    };
    matches!(
        (fs::canonicalize(path), fs::canonicalize(target)),
        (Ok(path), Ok(target)) if path == target
    )
}

#[cfg(not(target_os = "macos"))]
fn is_macos_system_alias(_path: &Path) -> bool {
    false
}

pub fn safe_directory(path: &Path) -> Result<(), String> {
    if !path.is_absolute() || path.parent().is_none() {
        return Err(
            "Installation paths must be absolute directories below a filesystem root".into(),
        );
    }
    crate::platform::native_filesystem::validate_directory(path)?;
    no_links(path)
}
pub fn unique_id() -> u128 {
    use std::sync::atomic::{AtomicU32, Ordering};

    static NEXT_ID: AtomicU32 = AtomicU32::new(0);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let process = std::process::id() as u128;
    let sequence = NEXT_ID.fetch_add(1, Ordering::Relaxed) as u128;

    ((timestamp as u128) << 64) | (process << 32) | sequence
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

pub fn collect_files(
    root: &Path,
    directory: &Path,
    output: &mut Vec<PathBuf>,
) -> Result<(), String> {
    no_links(directory)?;
    for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        no_links(&path)?;
        if path.is_dir() {
            collect_files(root, &path, output)?;
        } else {
            output.push(
                path.strip_prefix(root)
                    .map_err(|e| e.to_string())?
                    .to_owned(),
            );
        }
    }
    Ok(())
}

pub fn numeric_version(text: &str) -> Option<Vec<u64>> {
    let values: Vec<_> = text
        .split('.')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    (values.len() >= 2).then_some(values)
}
#[cfg(test)]
mod atomic_write_tests {
    use super::*;
    use std::io;

    #[test]
    fn unique_ids_do_not_collide_when_requested_concurrently() {
        let workers = 16;
        let ids = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..workers)
                .map(|_| scope.spawn(|| (0..128).map(|_| unique_id()).collect::<Vec<_>>()))
                .collect();
            handles
                .into_iter()
                .flat_map(|handle| handle.join().unwrap())
                .collect::<std::collections::HashSet<_>>()
        });
        assert_eq!(ids.len(), workers * 128);
    }

    fn temporary_directory() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "weave-atomic-write-{}-{}",
            std::process::id(),
            unique_id()
        ));
        fs::create_dir(&path).unwrap();
        path
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn path_guard_accepts_the_system_temporary_directory_alias() {
        assert!(no_links(&std::env::temp_dir()).is_ok());
    }

    #[test]
    fn replacement_failure_preserves_original_cleans_temporary_and_allows_retry() {
        let root = temporary_directory();
        let destination = root.join("settings");
        fs::write(&destination, "original").unwrap();

        let error = atomic_write_with(
            &destination,
            b"replacement",
            |file, contents| {
                use std::io::Write;
                file.write_all(contents)
            },
            |_, _| Err(io::Error::other("controlled replacement failure")),
        )
        .unwrap_err();

        assert_eq!(error.to_string(), "controlled replacement failure");
        assert_eq!(fs::read_to_string(&destination).unwrap(), "original");
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        atomic_write(&destination, b"replacement").unwrap();
        assert_eq!(fs::read_to_string(&destination).unwrap(), "replacement");
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn write_failure_preserves_original_and_cleans_temporary() {
        let root = temporary_directory();
        let destination = root.join("settings");
        fs::write(&destination, "original").unwrap();

        let error = atomic_write_with(
            &destination,
            b"replacement",
            |_, _| Err(io::Error::other("controlled write failure")),
            |temporary, destination| fs::rename(temporary, destination),
        )
        .unwrap_err();

        assert_eq!(error.to_string(), "controlled write failure");
        assert_eq!(fs::read_to_string(&destination).unwrap(), "original");
        assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn refuses_symlink_destination_without_modifying_target() {
        use std::os::unix::fs::symlink;

        let root = temporary_directory();
        let target = root.join("target");
        let destination = root.join("settings");
        fs::write(&target, "user data").unwrap();
        symlink(&target, &destination).unwrap();

        assert!(atomic_write(&destination, b"replacement").is_err());
        assert_eq!(fs::read_to_string(&target).unwrap(), "user data");
        assert_eq!(fs::read_dir(&root).unwrap().count(), 2);
        fs::remove_dir_all(root).unwrap();
    }
}

pub fn version_directories(root: &Path) -> Vec<PathBuf> {
    let mut directories: Vec<_> = fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            numeric_version(&entry.file_name().to_string_lossy()).map(|v| (v, entry.path()))
        })
        .collect();
    directories.sort_by(|a, b| b.0.cmp(&a.0));
    directories.into_iter().map(|(_, path)| path).collect()
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
    #[test]
    fn payload_contract_cannot_verify_a_file_outside_its_root() {
        let temporary = Temporary::create().unwrap();
        let root = temporary.0.join("payload");
        fs::create_dir(&root).unwrap();
        fs::write(temporary.0.join("outside.dll"), "outside payload").unwrap();
        assert!(verify_payload(&root, &["../outside.dll"]).is_err());
    }
    pub const CORE: &[&str] = &[
        "lib/MayaFluxLib.lib",
        "bin/MayaFluxLib.dll",
        "include/MayaFlux/MayaFlux.hpp",
        "lib/cmake/MayaFlux/MayaFluxConfig.cmake",
        "lib/cmake/MayaFlux/MayaFluxConfigVersion.cmake",
    ];

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
        assert!(verify_payload(&root.0, CORE).is_ok());
        for relative in CORE {
            let path = root.0.join(relative);
            fs::write(&path, "").unwrap();
            assert!(
                verify_payload(&root.0, CORE)
                    .unwrap_err()
                    .contains(relative)
            );
            fs::write(path, "fixture").unwrap();
        }
    }
    #[test]
    fn resources_are_complete_and_retries_preserve_custom_files() {
        let root = Fixture::new();
        crate::resources::deploy_resources(&root.0).unwrap();
        let templates = root.0.join("share/weave/templates");
        for relative in [
            "main.cpp",
            "CMakePresets.json",
            "validate.ps1",
            "run-report.ps1",
            "validate.sh",
            "validate-macos.sh",
            "cmake/build_community.cmake",
            "community/test/CMakeLists.txt",
            "vscode/launch.json",
        ] {
            assert!(fs::metadata(templates.join(relative)).unwrap().len() > 0);
        }
        fs::write(templates.join("custom.txt"), "user content").unwrap();
        crate::resources::deploy_resources(&root.0).unwrap();
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
