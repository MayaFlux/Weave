//! Embedded Weave resources and platform-independent deployment.
use crate::filesystem::{no_links, safe_directory, unique_id};
use std::{fs, io::Write, path::Path};
pub const RESOURCES: &[(&str, &str)] = &[
    (
        "validate.sh",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/templates/scripts/linux/validate.sh"
        )),
    ),
    (
        "run-report.sh",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/templates/scripts/linux/run-report.sh"
        )),
    ),
    (
        "validate-macos.sh",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/templates/scripts/macos/validate-macos.sh"
        )),
    ),
    (
        "run-report-macos.sh",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/templates/scripts/macos/run-report-macos.sh"
        )),
    ),
    (
        "CMakeLists.txt",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/templates/project/CMakeLists.txt"
        )),
    ),
    (
        "CMakePresets.json",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/templates/project/CMakePresets.json"
        )),
    ),
    (
        "main.cpp",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/templates/project/main.cpp"
        )),
    ),
    (
        "user_project.hpp",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/templates/project/user_project.hpp"
        )),
    ),
    (
        ".gitignore",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/templates/project/.gitignore"
        )),
    ),
    (
        "validate.ps1",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/templates/scripts/windows/validate.ps1"
        )),
    ),
    (
        "run-report.ps1",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/templates/scripts/windows/run-report.ps1"
        )),
    ),
    (
        "cmake/mayaflux.cmake",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/templates/project/cmake/mayaflux.cmake"
        )),
    ),
    (
        "cmake/shaders.cmake",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/templates/project/cmake/shaders.cmake"
        )),
    ),
    (
        "cmake/build_community.cmake",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/templates/project/cmake/build_community.cmake"
        )),
    ),
    (
        "community/module.cmake",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/templates/community/module.cmake"
        )),
    ),
    (
        "community/community.json",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/templates/community/community.json"
        )),
    ),
    (
        "community/CMakePresets.json",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/templates/community/CMakePresets.json"
        )),
    ),
    (
        "community/test/CMakeLists.txt",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/templates/community/test/CMakeLists.txt"
        )),
    ),
    (
        "vscode/settings.json",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/templates/project/vscode/settings.json"
        )),
    ),
    (
        "vscode/tasks.json",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/templates/project/vscode/tasks.json"
        )),
    ),
    (
        "vscode/launch.json",
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/templates/project/vscode/launch.json"
        )),
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
