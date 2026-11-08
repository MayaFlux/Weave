//! Finds the installed MayaFlux version used for module compatibility checks.

use super::{ModuleAddError, fail};
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::Command;

/// Parses the three numeric components used for compatibility comparisons.
pub(super) fn version(value: &str, label: &str) -> Result<[u64; 3], ModuleAddError> {
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() != 3
        || parts.iter().any(|part| {
            part.is_empty()
                || (part.len() > 1 && part.starts_with('0'))
                || !part.bytes().all(|byte| byte.is_ascii_digit())
        })
    {
        return Err(fail(format!("Malformed {label} version: {value}")));
    }
    let parsed: Result<Vec<_>, _> = parts.iter().map(|part| part.parse::<u64>()).collect();
    let parsed = parsed.map_err(|_| fail(format!("Malformed {label} version: {value}")))?;
    Ok([parsed[0], parsed[1], parsed[2]])
}

/// Finds MayaFlux through the override or the platform's installation roots.
pub(super) fn installed_version() -> Result<(String, [u64; 3]), ModuleAddError> {
    let roots: Vec<PathBuf> = if let Some(root) =
        std::env::var_os("MAYAFLUX_ROOT").filter(|value| !value.is_empty())
    {
        vec![PathBuf::from(root)]
    } else {
        #[cfg(target_os = "macos")]
        let roots = {
            let mut result = Vec::new();
            let paths = std::env::var_os("PATH").unwrap_or_default();
            let mut brews: Vec<_> = std::env::split_paths(&paths)
                .map(|path| path.join("brew"))
                .collect();
            if let Ok(entries) = fs::read_dir("/opt") {
                brews.extend(entries.flatten().map(|entry| entry.path().join("bin/brew")));
            }
            brews.retain(|path| path.is_file() && !path.starts_with("/usr/local"));
            brews.sort();
            brews.dedup();
            for brew in brews {
                for formula in ["mayaflux", "mayaflux-dev"] {
                    if let Ok(output) = Command::new(&brew).args(["--prefix", formula]).output() {
                        if output.status.success() {
                            result.push(PathBuf::from(
                                String::from_utf8_lossy(&output.stdout).trim(),
                            ));
                        }
                    }
                }
            }
            result
        };
        #[cfg(target_os = "linux")]
        let roots = vec![PathBuf::from("/usr/local"), PathBuf::from("/usr")];
        #[cfg(target_os = "windows")]
        let roots = vec![PathBuf::from(r"C:\MayaFlux")];
        roots
    };
    let mut checked = Vec::new();
    for root in roots {
        for library in ["lib", "lib64"] {
            let path = root
                .join(library)
                .join("cmake/MayaFlux/MayaFluxConfigVersion.cmake");
            checked.push(path.display().to_string());
            if !path.is_file() {
                continue;
            }
            let source = fs::read_to_string(&path).map_err(|error| {
                fail(format!(
                    "Could not read MayaFlux version file {}: {error}",
                    path.display()
                ))
            })?;
            return parse_installed_version(&source, &path);
        }
    }
    Err(fail(format!(
        "MayaFlux version file not found. Checked: {}",
        checked.join(", ")
    )))
}

/// Reads `PACKAGE_VERSION` from the installed CMake version file.
pub(super) fn parse_installed_version(
    source: &str,
    path: &Path,
) -> Result<(String, [u64; 3]), ModuleAddError> {
    let marker = "set(PACKAGE_VERSION";
    let tail = source
        .split_once(marker)
        .map(|(_, tail)| tail)
        .ok_or_else(|| {
            fail(format!(
                "Malformed MayaFlux version file: {}",
                path.display()
            ))
        })?;
    let text = tail
        .trim_start()
        .strip_prefix('"')
        .and_then(|tail| tail.split_once('"').map(|(value, _)| value))
        .ok_or_else(|| {
            fail(format!(
                "Malformed MayaFlux version file: {}",
                path.display()
            ))
        })?;
    Ok((text.to_owned(), version(text, "installed MayaFlux")?))
}
