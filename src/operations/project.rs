use crate::git::initialize_repository;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const SHADER_EXTENSIONS: &[&str] = &[
    "glsl", "vert", "frag", "comp", "geom", "tesc", "tese", "mesh", "task", "rgen", "rchit",
    "rahit", "rmiss", "rint", "rcall",
];

#[derive(Clone, Copy, Debug)]
pub struct ProjectOptions {
    pub with_lila: bool,
    pub with_vscode: bool,
    pub copy_shader_sources: bool,
}

#[derive(Debug)]
pub struct ProjectCreationResult {
    pub path: PathBuf,
    pub git_warning: Option<String>,
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
) -> Result<ProjectCreationResult, ProjectCreationError> {
    create_project_with_shader_root(name, destination, options, None)
}

fn create_project_with_shader_root(
    name: &str,
    destination: &Path,
    options: ProjectOptions,
    shader_root: Option<&Path>,
) -> Result<ProjectCreationResult, ProjectCreationError> {
    validate_name(name)?;
    let destination = resolve_destination(destination)?;
    let project_path = destination.join(name);
    crate::filesystem::no_links(&project_path).map_err(ProjectCreationError)?;
    if fs::symlink_metadata(&project_path).is_ok() {
        return Err(ProjectCreationError(format!(
            "Project destination already exists: {}",
            project_path.display()
        )));
    }

    let shader_sources = if options.copy_shader_sources {
        Some(match shader_root {
            Some(root) => read_shader_sources_at(root)?,
            None => read_shader_sources()?,
        })
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

    let git_warning = initialize_repository(&project_path, "project");
    Ok(ProjectCreationResult {
        path: project_path,
        git_warning,
    })
}

fn resolve_destination(destination: &Path) -> Result<PathBuf, ProjectCreationError> {
    let absolute = if destination.is_absolute() {
        destination.to_path_buf()
    } else {
        std::env::current_dir()?.join(destination)
    };

    crate::filesystem::no_links(&absolute).map_err(ProjectCreationError)?;
    let metadata = fs::symlink_metadata(&absolute).map_err(|error| {
        ProjectCreationError(format!(
            "Project destination does not exist: {}: {error}",
            absolute.display()
        ))
    })?;
    if metadata.file_type().is_symlink() {
        return Err(ProjectCreationError(format!(
            "Project destination is a symbolic link: {}",
            absolute.display()
        )));
    }
    if !metadata.is_dir() {
        return Err(ProjectCreationError(format!(
            "Project destination must be an existing directory: {}",
            absolute.display()
        )));
    }

    crate::filesystem::no_links(&absolute).map_err(ProjectCreationError)?;
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

    let mayaflux_cmake = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/resources/templates/project/cmake/mayaflux.cmake"
    ))
    .replace("@LILA_LINK_BLOCK@", lila_link)
    .replace("@LILA_DEBUGGER_PATH@", lila_debugger)
    .replace("@LILA_DLL_COPY@", lila_dll_copy);
    let mut files = vec![
        (
            "CMakeLists.txt",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/templates/project/CMakeLists.txt"
            ))
            .replace("@PROJECT_NAME@", name),
            false,
        ),
        (
            "CMakePresets.json",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/templates/project/CMakePresets.json"
            ))
            .to_owned(),
            false,
        ),
        (
            "src/main.cpp",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/templates/project/main.cpp"
            ))
            .to_owned(),
            false,
        ),
        (
            "src/user_project.hpp",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/templates/project/user_project.hpp"
            ))
            .to_owned(),
            false,
        ),
        ("cmake/mayaflux.cmake", mayaflux_cmake, false),
        (
            "cmake/shaders.cmake",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/templates/project/cmake/shaders.cmake"
            ))
            .to_owned(),
            false,
        ),
        (
            "cmake/build_community.cmake",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/templates/project/cmake/build_community.cmake"
            ))
            .to_owned(),
            false,
        ),
        ("community.cmake", String::new(), false),
        (
            ".gitignore",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/templates/project/.gitignore"
            ))
            .to_owned(),
            false,
        ),
    ];

    #[cfg(target_os = "windows")]
    files.extend([
        (
            "validate.ps1",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/templates/scripts/windows/validate.ps1"
            ))
            .to_owned(),
            false,
        ),
        (
            "run-report.ps1",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/templates/scripts/windows/run-report.ps1"
            ))
            .replace("@PROJECT_NAME@", name),
            false,
        ),
        ("validate.bat", batch_launcher("validate.ps1"), false),
        ("run-report.bat", batch_launcher("run-report.ps1"), false),
    ]);

    #[cfg(target_os = "macos")]
    files.extend([
        (
            "validate.sh",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/templates/scripts/macos/validate-macos.sh"
            ))
            .to_owned(),
            true,
        ),
        (
            "run-report.sh",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/templates/scripts/macos/run-report-macos.sh"
            ))
            .replace("@PROJECT_NAME@", name),
            true,
        ),
    ]);

    #[cfg(target_os = "linux")]
    files.extend([
        (
            "validate.sh",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/templates/scripts/linux/validate.sh"
            ))
            .to_owned(),
            true,
        ),
        (
            "run-report.sh",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/templates/scripts/linux/run-report.sh"
            ))
            .replace("@PROJECT_NAME@", name),
            true,
        ),
    ]);

    if options.with_vscode {
        files.extend([
            (
                ".vscode/settings.json",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/resources/templates/project/vscode/settings.json"
                ))
                .replace("@PROJECT_NAME@", name),
                false,
            ),
            (
                ".vscode/tasks.json",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/resources/templates/project/vscode/tasks.json"
                ))
                .replace("@PROJECT_NAME@", name),
                false,
            ),
            (
                ".vscode/launch.json",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/resources/templates/project/vscode/launch.json"
                ))
                .replace("@PROJECT_NAME@", name),
                false,
            ),
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
        "# {name}\n\nA MayaFlux multimedia DSP project.\n\n## Building\n\n```sh\ncmake --preset release\ncmake --build --preset release --parallel\n```\n\nRun `validate` from this folder to check the project and MayaFlux installation. Run `run-report` after building to collect diagnostics. When Git is available, Weave initializes a repository and stages the generated files.\n"
    )
}

fn read_shader_sources() -> Result<Vec<(PathBuf, Vec<u8>)>, ProjectCreationError> {
    let root = crate::platform::shader_source_root().map_err(ProjectCreationError)?;
    read_shader_sources_at(&root)
}

fn read_shader_sources_at(root: &Path) -> Result<Vec<(PathBuf, Vec<u8>)>, ProjectCreationError> {
    crate::filesystem::no_links(root).map_err(ProjectCreationError)?;
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
            let path = entry.path();
            crate::filesystem::no_links(&path).map_err(ProjectCreationError)?;
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                pending.push(entry.path());
            } else if file_type.is_file()
                && entry
                    .path()
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|extension| {
                        SHADER_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
                    })
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
    let valid_rest =
        characters.all(|character| character.is_ascii_alphanumeric() || character == '_');
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
    use std::sync::atomic::{AtomicU64, Ordering};

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            static NEXT_ID: AtomicU64 = AtomicU64::new(0);

            loop {
                let sequence = NEXT_ID.fetch_add(1, Ordering::Relaxed);
                let path = std::env::temp_dir()
                    .join(format!("weave-test-{}-{sequence}", std::process::id()));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!(
                        "Could not create test directory {}: {error}",
                        path.display()
                    ),
                }
            }
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
        let project = create_project("AudioDemo", &temporary.0, ProjectOptions::default())
            .unwrap()
            .path;

        assert_eq!(
            project,
            fs::canonicalize(&temporary.0).unwrap().join("AudioDemo")
        );
        assert!(project.join("src/main.cpp").is_file());
        assert!(project.join("src/user_project.hpp").is_file());
        assert!(project.join("CMakePresets.json").is_file());
        assert!(project.join("community.cmake").is_file());
        assert!(project.join("cmake/shaders.cmake").is_file());
        assert!(project.join(".vscode/settings.json").is_file());
        assert!(project.join(".vscode/tasks.json").is_file());
        assert!(project.join(".vscode/launch.json").is_file());
        assert!(project.join(".gitignore").is_file());
        assert!(project.join("README.md").is_file());
        #[cfg(target_os = "windows")]
        for script in [
            "validate.ps1",
            "run-report.ps1",
            "validate.bat",
            "run-report.bat",
        ] {
            assert!(project.join(script).is_file());
        }
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        for script in ["validate.sh", "run-report.sh"] {
            assert!(project.join(script).is_file());
        }
        assert!(
            fs::read_to_string(project.join("CMakeLists.txt"))
                .unwrap()
                .contains("project(AudioDemo")
        );
        let presets: serde_json::Value =
            serde_json::from_slice(&fs::read(project.join("CMakePresets.json")).unwrap()).unwrap();
        let configure: Vec<_> = presets["configurePresets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|preset| preset["name"].as_str().unwrap())
            .collect();
        assert!(configure.contains(&"debug"));
        assert!(configure.contains(&"release"));
        for preset in presets["buildPresets"].as_array().unwrap() {
            assert!(configure.contains(&preset["configurePreset"].as_str().unwrap()));
        }
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
        .unwrap()
        .path;

        let cmake = fs::read_to_string(project.join("cmake/mayaflux.cmake")).unwrap();
        assert!(cmake.contains("MayaFlux::MayaFluxHost"));
        assert!(!cmake.contains("@LILA_"));
        assert!(project.join(".vscode/tasks.json").is_file());
        let tasks: serde_json::Value =
            serde_json::from_slice(&fs::read(project.join(".vscode/tasks.json")).unwrap()).unwrap();
        let launch: serde_json::Value =
            serde_json::from_slice(&fs::read(project.join(".vscode/launch.json")).unwrap())
                .unwrap();
        let settings: serde_json::Value =
            serde_json::from_slice(&fs::read(project.join(".vscode/settings.json")).unwrap())
                .unwrap();
        assert_eq!(tasks["version"], "2.0.0");
        assert!(
            launch["configurations"]
                .as_array()
                .is_some_and(|items| !items.is_empty())
        );
        assert!(settings.is_object());
        assert!(
            fs::read_to_string(project.join(".vscode/launch.json"))
                .unwrap()
                .contains("AudioDemo")
        );
    }

    #[test]
    fn copies_only_supported_shader_files_from_the_selected_source_root() {
        let temporary = TestDirectory::new();
        let shader_root = temporary.0.join("shader-source");
        fs::create_dir_all(shader_root.join("nested")).unwrap();
        fs::write(shader_root.join("nested/effect.frag"), "shader source").unwrap();
        fs::write(shader_root.join("readme.txt"), "not a shader").unwrap();

        let result = create_project_with_shader_root(
            "ShaderDemo",
            &temporary.0,
            ProjectOptions {
                copy_shader_sources: true,
                ..ProjectOptions::default()
            },
            Some(&shader_root),
        )
        .unwrap();

        assert_eq!(
            fs::read_to_string(result.path.join("data/shaders/nested/effect.frag")).unwrap(),
            "shader source"
        );
        assert!(!result.path.join("data/shaders/readme.txt").exists());
    }

    #[test]
    fn shader_copy_rejects_empty_sources_before_creating_project() {
        let temporary = TestDirectory::new();
        let shader_root = temporary.0.join("empty-shaders");
        fs::create_dir(&shader_root).unwrap();

        let error = create_project_with_shader_root(
            "ShaderDemo",
            &temporary.0,
            ProjectOptions {
                copy_shader_sources: true,
                ..ProjectOptions::default()
            },
            Some(&shader_root),
        )
        .unwrap_err();

        assert!(error.to_string().contains("shader sources are empty"));
        assert!(!temporary.0.join("ShaderDemo").exists());
    }

    #[cfg(unix)]
    #[test]
    fn shader_copy_rejects_linked_source_files() {
        use std::os::unix::fs::symlink;

        let temporary = TestDirectory::new();
        let shader_root = temporary.0.join("shader-source");
        fs::create_dir(&shader_root).unwrap();
        let target = temporary.0.join("outside.frag");
        fs::write(&target, "external").unwrap();
        symlink(&target, shader_root.join("linked.frag")).unwrap();

        let error = create_project_with_shader_root(
            "ShaderDemo",
            &temporary.0,
            ProjectOptions {
                copy_shader_sources: true,
                ..ProjectOptions::default()
            },
            Some(&shader_root),
        )
        .unwrap_err();

        assert!(error.to_string().contains("symbolic link"));
        assert!(!temporary.0.join("ShaderDemo").exists());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_destination_with_a_linked_ancestor_without_writing_through_it() {
        use std::os::unix::fs::symlink;

        let temporary = TestDirectory::new();
        let target = temporary.0.join("target");
        let linked_parent = temporary.0.join("linked-parent");
        let destination = linked_parent.join("projects");
        fs::create_dir_all(&target).unwrap();
        fs::create_dir(target.join("projects")).unwrap();
        symlink(&target, &linked_parent).unwrap();

        let error =
            create_project("AudioDemo", &destination, ProjectOptions::default()).unwrap_err();

        assert!(error.to_string().contains("reparse point"));
        assert!(!target.join("projects/AudioDemo").exists());
        assert_eq!(fs::read_dir(&target).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn shader_copy_rejects_a_linked_ancestor_before_creating_project() {
        use std::os::unix::fs::symlink;

        let temporary = TestDirectory::new();
        let target = temporary.0.join("shader-target");
        let linked_parent = temporary.0.join("linked-shader-parent");
        let shader_root = linked_parent.join("shaders");
        fs::create_dir_all(target.join("shaders/nested")).unwrap();
        fs::write(target.join("shaders/nested/effect.frag"), "shader source").unwrap();
        symlink(&target, &linked_parent).unwrap();

        let error = create_project_with_shader_root(
            "ShaderDemo",
            &temporary.0,
            ProjectOptions {
                copy_shader_sources: true,
                ..ProjectOptions::default()
            },
            Some(&shader_root),
        )
        .unwrap_err();

        assert!(error.to_string().contains("reparse point"));
        assert!(!temporary.0.join("ShaderDemo").exists());
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
        assert_eq!(
            fs::read_to_string(existing.join("keep.txt")).unwrap(),
            "preserve"
        );
    }

    #[test]
    fn missing_or_non_directory_parent_is_rejected_without_creating_a_project() {
        let temporary = TestDirectory::new();
        let file = temporary.0.join("not-a-directory");
        fs::write(&file, "preserve").unwrap();
        for destination in [temporary.0.join("missing"), file.clone()] {
            assert!(create_project("AudioDemo", &destination, ProjectOptions::default()).is_err());
        }
        assert_eq!(fs::read_dir(&temporary.0).unwrap().count(), 1);
        assert_eq!(fs::read_to_string(file).unwrap(), "preserve");
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
