use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

const SHADER_EXTENSIONS: &[&str] = &[
    "glsl", "vert", "frag", "comp", "geom", "tesc", "tese", "mesh", "task", "rgen",
    "rchit", "rahit", "rmiss", "rint", "rcall",
];

#[derive(Clone, Copy, Debug)]
pub struct ProjectOptions {
    pub with_lila: bool,
    pub with_vscode: bool,
    pub copy_shader_sources: bool,
}

impl Default for ProjectOptions {
    fn default() -> Self {
        Self {
            with_lila: false,
            with_vscode: true,
            copy_shader_sources: false,
        }
    }
}

#[derive(Debug)]
pub struct ProjectCreationError(String);

impl Display for ProjectCreationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for ProjectCreationError {}

impl From<io::Error> for ProjectCreationError {
    fn from(error: io::Error) -> Self {
        Self(error.to_string())
    }
}

pub fn create_project(
    name: &str,
    destination: &Path,
    options: ProjectOptions,
) -> Result<PathBuf, ProjectCreationError> {
    validate_name(name)?;
    let destination = resolve_destination(destination)?;
    let project_path = destination.join(name);
    if fs::symlink_metadata(&project_path).is_ok() {
        return Err(ProjectCreationError(format!(
            "Project destination already exists: {}",
            project_path.display()
        )));
    }

    let shader_sources = if options.copy_shader_sources {
        Some(read_shader_sources()?)
    } else {
        None
    };

    fs::create_dir(&project_path).map_err(|error| {
        ProjectCreationError(format!(
            "Could not create project directory {}: {error}",
            project_path.display()
        ))
    })?;

    let result = populate_project(&project_path, name, options, shader_sources.as_deref());
    if let Err(error) = result {
        if let Err(cleanup) = fs::remove_dir_all(&project_path) {
            return Err(ProjectCreationError(format!(
                "Project creation failed: {error}. Incomplete project could not be removed: {}: {cleanup}",
                project_path.display()
            )));
        }
        return Err(error);
    }

    Ok(project_path)
}

fn resolve_destination(destination: &Path) -> Result<PathBuf, ProjectCreationError> {
    let absolute = if destination.is_absolute() {
        destination.to_path_buf()
    } else {
        std::env::current_dir()?.join(destination)
    };

    let mut current = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => current.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                current.pop();
            }
            Component::Normal(part) => {
                current.push(part);
                match fs::symlink_metadata(&current) {
                    Ok(metadata) if metadata.file_type().is_symlink() => {
                        return Err(ProjectCreationError(format!(
                            "Project destination contains a symbolic link: {}",
                            current.display()
                        )));
                    }
                    Ok(_) => {}
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {
                        return Err(ProjectCreationError(format!(
                            "Project destination does not exist: {}",
                            current.display()
                        )));
                    }
                    Err(error) => return Err(error.into()),
                }
            }
        }
    }

    if !absolute.is_dir() {
        return Err(ProjectCreationError(format!(
            "Project destination must be an existing directory: {}",
            absolute.display()
        )));
    }

    fs::canonicalize(absolute).map_err(Into::into)
}

fn populate_project(
    project_path: &Path,
    name: &str,
    options: ProjectOptions,
    shader_sources: Option<&[(PathBuf, Vec<u8>)]>,
) -> Result<(), ProjectCreationError> {
    fs::create_dir(project_path.join("src"))?;
    fs::create_dir(project_path.join("cmake"))?;
    fs::create_dir_all(project_path.join("data/shaders"))?;

    let lila_link = if options.with_lila {
        "target_link_libraries(${PROJECT_NAME} PRIVATE MayaFlux::MayaFluxHost)"
    } else {
        ""
    };
    let lila_debugger = if options.with_lila {
        "$<TARGET_FILE_DIR:MayaFlux::MayaFluxHost>;"
    } else {
        ""
    };
    let lila_dll_copy = if options.with_lila {
        r#"if(EXISTS "$ENV{MAYAFLUX_ROOT}/bin/MayaFluxHost.dll")
        add_custom_command(TARGET ${PROJECT_NAME} POST_BUILD
            COMMAND ${CMAKE_COMMAND} -E copy_if_different
                "$ENV{MAYAFLUX_ROOT}/bin/MayaFluxHost.dll"
                $<TARGET_FILE_DIR:${PROJECT_NAME}>
        )
    endif()"#
    } else {
        ""
    };

    let mayaflux_cmake = include_str!("../templates/cmake/mayaflux.cmake")
        .replace("@LILA_LINK_BLOCK@", lila_link)
        .replace("@LILA_DEBUGGER_PATH@", lila_debugger)
        .replace("@LILA_DLL_COPY@", lila_dll_copy);
    let mut files = vec![
        ("CMakeLists.txt", include_str!("../templates/CMakeLists.txt").replace("@PROJECT_NAME@", name), false),
        ("CMakePresets.json", include_str!("../templates/CMakePresets.json").to_owned(), false),
        ("src/main.cpp", include_str!("../templates/main.cpp").to_owned(), false),
        ("src/user_project.hpp", include_str!("../templates/user_project.hpp").to_owned(), false),
        ("cmake/mayaflux.cmake", mayaflux_cmake, false),
        ("cmake/shaders.cmake", include_str!("../templates/cmake/shaders.cmake").to_owned(), false),
        ("cmake/build_community.cmake", include_str!("../templates/cmake/build_community.cmake").to_owned(), false),
        ("community.cmake", String::new(), false),
        (".gitignore", include_str!("../templates/.gitignore").to_owned(), false),
    ];

    #[cfg(target_os = "windows")]
    files.extend([
        ("validate.ps1", include_str!("../templates/validate.ps1").to_owned(), false),
        ("run-report.ps1", include_str!("../templates/run-report.ps1").replace("@PROJECT_NAME@", name), false),
        ("validate.bat", batch_launcher("validate.ps1"), false),
        ("run-report.bat", batch_launcher("run-report.ps1"), false),
    ]);

    #[cfg(target_os = "macos")]
    files.extend([
        ("validate.sh", include_str!("../templates/validate-macos.sh").to_owned(), true),
        ("run-report.sh", include_str!("../templates/run-report-macos.sh").replace("@PROJECT_NAME@", name), true),
    ]);

    #[cfg(target_os = "linux")]
    files.extend([
        ("validate.sh", include_str!("../templates/validate.sh").to_owned(), true),
        ("run-report.sh", include_str!("../templates/run-report.sh").replace("@PROJECT_NAME@", name), true),
    ]);

    if options.with_vscode {
        files.extend([
            (".vscode/settings.json", include_str!("../templates/vscode/settings.json").replace("@PROJECT_NAME@", name), false),
            (".vscode/tasks.json", include_str!("../templates/vscode/tasks.json").replace("@PROJECT_NAME@", name), false),
            (".vscode/launch.json", include_str!("../templates/vscode/launch.json").replace("@PROJECT_NAME@", name), false),
        ]);
    }

    files.push(("README.md", project_readme(name), false));

    for (relative, contents, executable) in files {
        let path = project_path.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, contents.as_bytes())?;
        #[cfg(unix)]
        if executable {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755))?;
        }
        #[cfg(not(unix))]
        let _ = executable;
    }

    if let Some(sources) = shader_sources {
        for (relative, contents) in sources {
            let destination = project_path.join("data/shaders").join(relative);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(destination, contents)?;
        }
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn batch_launcher(script: &str) -> String {
    format!(
        "@echo off\r\nsetlocal DisableDelayedExpansion\r\n\"%SystemRoot%\\System32\\WindowsPowerShell\\v1.0\\powershell.exe\" -NoProfile -ExecutionPolicy Bypass -File \"%~dp0{script}\" %*\r\nexit /b %ERRORLEVEL%\r\n"
    )
}

fn project_readme(name: &str) -> String {
    format!(
        "# {name}\n\nA MayaFlux multimedia DSP project.\n\n## Building\n\n```sh\ncmake --preset release\ncmake --build --preset release --parallel\n```\n\nRun `validate` from this folder to check the project and MayaFlux installation. Run `run-report` after building to collect diagnostics.\n"
    )
}

fn read_shader_sources() -> Result<Vec<(PathBuf, Vec<u8>)>, ProjectCreationError> {
    let root = crate::platform::shader_source_root().map_err(ProjectCreationError)?;
    if fs::symlink_metadata(&root)
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        return Err(ProjectCreationError(format!(
            "MayaFlux shader source directory is a symbolic link: {}",
            root.display()
        )));
    }
    let canonical_root = fs::canonicalize(&root).map_err(|error| {
        ProjectCreationError(format!(
            "MayaFlux shader sources were not found at {}: {error}",
            root.display()
        ))
    })?;
    let mut pending = vec![canonical_root.clone()];
    let mut sources = Vec::new();

    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_symlink() {
                return Err(ProjectCreationError(format!(
                    "MayaFlux shader sources contain a symbolic link: {}",
                    entry.path().display()
                )));
            }
            if file_type.is_dir() {
                pending.push(entry.path());
            } else if file_type.is_file()
                && entry
                    .path()
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|extension| SHADER_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str()))
            {
                let path = entry.path();
                let relative = path
                    .strip_prefix(&canonical_root)
                    .map_err(|error| ProjectCreationError(error.to_string()))?
                    .to_path_buf();
                sources.push((relative, fs::read(path)?));
            }
        }
    }

    if sources.is_empty() {
        return Err(ProjectCreationError(format!(
            "MayaFlux shader sources are empty at {}. Update MayaFlux or omit --copy-shader-sources.",
            canonical_root.display()
        )));
    }

    Ok(sources)
}

fn validate_name(name: &str) -> Result<(), ProjectCreationError> {
    let mut characters = name.chars();
    let valid_start = characters
        .next()
        .is_some_and(|character| character.is_ascii_alphabetic() || character == '_');
    let valid_rest = characters.all(|character| character.is_ascii_alphanumeric() || character == '_');
    let uppercase = name.to_ascii_uppercase();
    let bytes = uppercase.as_bytes();
    let reserved = ["CON", "PRN", "AUX", "NUL"].contains(&uppercase.as_str())
        || (bytes.len() == 4
            && (bytes.starts_with(b"COM") || bytes.starts_with(b"LPT"))
            && matches!(bytes[3], b'1'..=b'9'));

    if !valid_start || !valid_rest || reserved {
        return Err(ProjectCreationError(
            "Project name must start with a letter or underscore and contain only letters, digits, or underscores; Windows device names are not allowed.".to_owned(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!("weave-test-{}-{unique}", std::process::id()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn creates_project_from_embedded_templates() {
        let temporary = TestDirectory::new();
        let project = create_project("AudioDemo", &temporary.0, ProjectOptions::default()).unwrap();

        assert_eq!(project, temporary.0.join("AudioDemo"));
        assert!(project.join("src/main.cpp").is_file());
        assert!(project.join("src/user_project.hpp").is_file());
        assert!(project.join("CMakePresets.json").is_file());
        assert!(project.join("community.cmake").is_file());
        assert!(project.join("cmake/shaders.cmake").is_file());
        assert!(fs::read_to_string(project.join("CMakeLists.txt"))
            .unwrap()
            .contains("project(AudioDemo"));
    }

    #[test]
    fn applies_lila_and_vscode_options() {
        let temporary = TestDirectory::new();
        let project = create_project(
            "AudioDemo",
            &temporary.0,
            ProjectOptions {
                with_lila: true,
                with_vscode: true,
                copy_shader_sources: false,
            },
        )
        .unwrap();

        let cmake = fs::read_to_string(project.join("cmake/mayaflux.cmake")).unwrap();
        assert!(cmake.contains("MayaFlux::MayaFluxHost"));
        assert!(!cmake.contains("@LILA_"));
        assert!(project.join(".vscode/tasks.json").is_file());
        assert!(fs::read_to_string(project.join(".vscode/launch.json"))
            .unwrap()
            .contains("AudioDemo"));
    }

    #[test]
    fn rejects_unsafe_and_reserved_names_without_creating_output() {
        let temporary = TestDirectory::new();
        for name in ["", "../escape", "bad-name", "NUL", "com1", "abé"] {
            assert!(create_project(name, &temporary.0, ProjectOptions::default()).is_err());
        }
        assert_eq!(fs::read_dir(&temporary.0).unwrap().count(), 0);
    }

    #[test]
    fn preserves_existing_destination() {
        let temporary = TestDirectory::new();
        let existing = temporary.0.join("AudioDemo");
        fs::create_dir(&existing).unwrap();
        fs::write(existing.join("keep.txt"), "preserve").unwrap();

        assert!(create_project("AudioDemo", &temporary.0, ProjectOptions::default()).is_err());
        assert_eq!(fs::read_to_string(existing.join("keep.txt")).unwrap(), "preserve");
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_destination() {
        let temporary = TestDirectory::new();
        let target = temporary.0.join("target");
        let link = temporary.0.join("link");
        fs::create_dir(&target).unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();

        assert!(create_project("AudioDemo", &link, ProjectOptions::default()).is_err());
        assert_eq!(fs::read_dir(target).unwrap().count(), 0);
    }
}
