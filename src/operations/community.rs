use crate::git::initialize_repository;
use serde_json::Value;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct CommunityModuleOptions {
    pub description: String,
    pub min_version: String,
    pub needs_lila: bool,
}

impl Default for CommunityModuleOptions {
    fn default() -> Self {
        Self {
            description: String::new(),
            min_version: "0.5.0".to_owned(),
            needs_lila: false,
        }
    }
}

#[derive(Debug)]
pub struct CommunityCreationResult {
    pub path: PathBuf,
    pub git_warning: Option<String>,
}

#[derive(Debug)]
pub struct CommunityCreationError(String);

impl Display for CommunityCreationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for CommunityCreationError {}

impl From<io::Error> for CommunityCreationError {
    fn from(error: io::Error) -> Self {
        Self(error.to_string())
    }
}

pub fn create_community_module(
    name: &str,
    destination: &Path,
    options: &CommunityModuleOptions,
) -> Result<CommunityCreationResult, CommunityCreationError> {
    validate_name(name)?;
    let options = CommunityModuleOptions {
        description: options.description.trim().to_owned(),
        min_version: if options.min_version.trim().is_empty() {
            "0.5.0".to_owned()
        } else {
            options.min_version.trim().to_owned()
        },
        needs_lila: options.needs_lila,
    };
    validate_min_version(&options.min_version)?;
    let destination = resolve_destination(destination)?;
    let module_path = destination.join(name);
    crate::filesystem::no_links(&module_path).map_err(CommunityCreationError)?;
    if fs::symlink_metadata(&module_path).is_ok() {
        return Err(CommunityCreationError(format!(
            "Community module destination already exists: {}",
            module_path.display()
        )));
    }

    fs::create_dir(&module_path).map_err(|error| {
        CommunityCreationError(format!(
            "Could not create community module directory {}: {error}",
            module_path.display()
        ))
    })?;

    let result = populate_module(&module_path, name, &options);
    let git_warning = match result {
        Ok(warning) => warning,
        Err(error) => {
            if let Err(cleanup) = fs::remove_dir_all(&module_path) {
                return Err(CommunityCreationError(format!(
                    "Community module creation failed: {error}. Incomplete module could not be removed: {}: {cleanup}",
                    module_path.display()
                )));
            }
            return Err(error);
        }
    };

    Ok(CommunityCreationResult {
        path: module_path,
        git_warning,
    })
}

fn validate_name(name: &str) -> Result<(), CommunityCreationError> {
    let mut characters = name.chars();
    let valid_start = characters
        .next()
        .is_some_and(|character| character.is_ascii_lowercase());
    let valid_rest = characters.all(|character| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
    });
    let reserved = ["con", "prn", "aux", "nul"].contains(&name)
        || (name.len() == 4
            && (name.starts_with("com") || name.starts_with("lpt"))
            && matches!(name.as_bytes()[3], b'1'..=b'9'));
    if !valid_start || !valid_rest || reserved {
        return Err(CommunityCreationError(
            "Module name must use snake_case; Windows device names are not allowed.".to_owned(),
        ));
    }
    Ok(())
}

fn validate_min_version(version: &str) -> Result<(), CommunityCreationError> {
    let components: Vec<_> = version.split('.').collect();
    let valid = components.len() == 3
        && components.iter().all(|component| {
            !component.is_empty()
                && component.bytes().all(|byte| byte.is_ascii_digit())
                && (*component == "0" || !component.starts_with('0'))
        });
    if !valid {
        return Err(CommunityCreationError(
            "Minimum MayaFlux version must use major.minor.patch format, for example 0.5.0."
                .to_owned(),
        ));
    }
    Ok(())
}

fn resolve_destination(destination: &Path) -> Result<PathBuf, CommunityCreationError> {
    let absolute = if destination.is_absolute() {
        destination.to_path_buf()
    } else {
        std::env::current_dir()?.join(destination)
    };
    crate::filesystem::no_links(&absolute).map_err(CommunityCreationError)?;
    fs::create_dir_all(&absolute).map_err(|error| {
        CommunityCreationError(format!(
            "Could not create community destination {}: {error}",
            absolute.display()
        ))
    })?;
    let metadata = fs::symlink_metadata(&absolute).map_err(|error| {
        CommunityCreationError(format!(
            "Community destination must be a directory: {}: {error}",
            absolute.display()
        ))
    })?;
    if metadata.file_type().is_symlink() {
        return Err(CommunityCreationError(format!(
            "Community destination is a symbolic link: {}",
            absolute.display()
        )));
    }
    if !metadata.is_dir() {
        return Err(CommunityCreationError(format!(
            "Community destination must be a directory: {}",
            absolute.display()
        )));
    }
    crate::filesystem::no_links(&absolute).map_err(CommunityCreationError)?;
    fs::canonicalize(absolute).map_err(Into::into)
}

fn populate_module(
    module_path: &Path,
    name: &str,
    options: &CommunityModuleOptions,
) -> Result<Option<String>, CommunityCreationError> {
    fs::create_dir(module_path.join("src"))?;
    fs::create_dir(module_path.join("test"))?;

    let mut manifest: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/resources/templates/community/community.json"
    )))
    .map_err(|error| {
        CommunityCreationError(format!(
            "Embedded community manifest template is invalid: {error}"
        ))
    })?;
    manifest["name"] = Value::String(name.to_owned());
    manifest["description"] = Value::String(options.description.clone());
    manifest["min_version"] = Value::String(options.min_version.clone());
    manifest["needs_lila"] = Value::Bool(options.needs_lila);

    let module_cmake = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/resources/templates/community/module.cmake"
    ))
    .replace("@MODULE_NAME@", name)
    .replace(
        "set(MF_MIN_VERSION \"0.5.0\")",
        &format!("set(MF_MIN_VERSION \"{}\")", options.min_version),
    )
    .replace(
        "set(MF_NEEDS_LILA OFF)",
        &format!(
            "set(MF_NEEDS_LILA {})",
            if options.needs_lila { "ON" } else { "OFF" }
        ),
    );

    let mut test_cmake = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/resources/templates/community/test/CMakeLists.txt"
    ))
    .replace("@MODULE_NAME@", name);
    let settings = format!(
        "set(MF_MIN_VERSION \"{}\")\nset(MF_NEEDS_LILA {})",
        options.min_version,
        if options.needs_lila { "ON" } else { "OFF" }
    );
    let minimum_line_end = test_cmake.find('\n').ok_or_else(|| {
        CommunityCreationError(
            "Community test CMake template has no minimum-version line.".to_owned(),
        )
    })?;
    test_cmake.insert_str(minimum_line_end + 1, &format!("\n{settings}\n"));

    let root_cmake = format!(
        "cmake_minimum_required(VERSION 3.25)\nproject({name} LANGUAGES C CXX)\nenable_testing()\nadd_subdirectory(test)\n"
    );
    let readme = format!(
        "# {name}\n\n{}\n\nAdd module implementation files under `src/` and checks in `test/test_{name}.cpp`.\n\nWith MayaFlux and its required build tools installed, configure, build, and test with:\n\n```sh\ncmake --preset release\ncmake --build --preset release\nctest --preset release\n```\n\nWhen Git is available, Weave initializes a repository and stages the generated files. Git is optional for building.\n",
        if options.description.is_empty() {
            "A MayaFlux community module."
        } else {
            options.description.as_str()
        }
    );

    let files = [
        (
            ".gitignore".to_owned(),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/templates/project/.gitignore"
            ))
            .to_owned(),
        ),
        (
            "CMakePresets.json".to_owned(),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/templates/community/CMakePresets.json"
            ))
            .to_owned(),
        ),
        ("CMakeLists.txt".to_owned(), root_cmake),
        (
            "community.json".to_owned(),
            format!(
                "{}\n",
                serde_json::to_string_pretty(&manifest).map_err(|error| CommunityCreationError(
                    format!("Could not serialize community manifest: {error}")
                ))?
            ),
        ),
        ("README.md".to_owned(), readme),
        (format!("{name}.cmake"), module_cmake),
        ("test/CMakeLists.txt".to_owned(), test_cmake),
        (
            format!("test/test_{name}.cpp"),
            "int main()\n{\n    return 0;\n}\n".to_owned(),
        ),
    ];

    for (relative, contents) in files {
        fs::write(module_path.join(relative), contents.as_bytes())?;
    }

    Ok(initialize_git(module_path))
}

fn initialize_git(module_path: &Path) -> Option<String> {
    initialize_repository(module_path, "community module")
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
                    .join(format!("weave-community-{}-{sequence}", std::process::id()));
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
    fn creates_complete_module_with_normalized_options_and_no_generated_implementation_file() {
        let temporary = TestDirectory::new();
        let destination = temporary.0.join("new-location");
        let result = create_community_module(
            "audio_tools",
            &destination,
            &CommunityModuleOptions {
                description: "  Audio helpers  ".to_owned(),
                min_version: " 0.6.0 ".to_owned(),
                needs_lila: true,
            },
        )
        .unwrap();

        assert_eq!(
            result.path,
            fs::canonicalize(&destination).unwrap().join("audio_tools")
        );
        for relative in [
            "src",
            "test",
            "CMakeLists.txt",
            "CMakePresets.json",
            "README.md",
            "audio_tools.cmake",
            "test/CMakeLists.txt",
            "test/test_audio_tools.cpp",
            ".gitignore",
            "community.json",
        ] {
            assert!(result.path.join(relative).exists(), "missing {relative}");
        }
        let presets: Value =
            serde_json::from_slice(&fs::read(result.path.join("CMakePresets.json")).unwrap())
                .unwrap();
        assert!(
            presets["configurePresets"]
                .as_array()
                .unwrap()
                .iter()
                .any(|preset| preset["name"] == "release")
        );
        assert!(
            presets["buildPresets"]
                .as_array()
                .unwrap()
                .iter()
                .any(|preset| preset["name"] == "release")
        );
        assert!(!result.path.join("src/audio_tools.cpp").exists());
        let manifest: Value =
            serde_json::from_slice(&fs::read(result.path.join("community.json")).unwrap()).unwrap();
        assert_eq!(manifest["name"], "audio_tools");
        assert_eq!(manifest["description"], "Audio helpers");
        assert_eq!(manifest["min_version"], "0.6.0");
        assert_eq!(manifest["needs_lila"], true);
        assert!(
            fs::read_to_string(result.path.join("audio_tools.cmake"))
                .unwrap()
                .contains("set(MF_NEEDS_LILA ON)")
        );
        assert!(
            fs::read_to_string(result.path.join("test/CMakeLists.txt"))
                .unwrap()
                .contains("set(MF_MIN_VERSION \"0.6.0\")")
        );
        assert!(
            !fs::read_to_string(result.path.join("audio_tools.cmake"))
                .unwrap()
                .contains("@MODULE_NAME@")
        );
        assert!(
            !fs::read_to_string(result.path.join("test/CMakeLists.txt"))
                .unwrap()
                .contains("@MODULE_NAME@")
        );
        assert!(
            fs::read_to_string(result.path.join("README.md"))
                .unwrap()
                .contains("Audio helpers")
        );
    }

    #[test]
    fn empty_minimum_uses_default_and_empty_description_uses_default_text() {
        let temporary = TestDirectory::new();
        let result = create_community_module(
            "default_module",
            &temporary.0,
            &CommunityModuleOptions {
                description: "  ".to_owned(),
                min_version: "  ".to_owned(),
                needs_lila: false,
            },
        )
        .unwrap();
        let manifest: Value =
            serde_json::from_slice(&fs::read(result.path.join("community.json")).unwrap()).unwrap();
        assert_eq!(manifest["min_version"], "0.5.0");
        assert_eq!(manifest["description"], "");
        assert_eq!(manifest["needs_lila"], false);
        assert!(
            fs::read_to_string(result.path.join("README.md"))
                .unwrap()
                .contains("A MayaFlux community module.")
        );
        assert!(
            fs::read_to_string(result.path.join("default_module.cmake"))
                .unwrap()
                .contains("set(MF_NEEDS_LILA OFF)")
        );
    }

    #[test]
    fn unsafe_names_and_invalid_versions_do_not_create_destinations() {
        let temporary = TestDirectory::new();
        for name in [
            "",
            "../escape",
            "Bad_Name",
            "con",
            "nul",
            "com1",
            "name/name",
        ] {
            assert!(
                create_community_module(name, &temporary.0, &CommunityModuleOptions::default())
                    .is_err(),
                "accepted {name:?}"
            );
        }
        for minimum in ["1.2", "01.2.3", "1.x.3", "1.2.3.4"] {
            let destination = temporary
                .0
                .join(format!("bad-{}", minimum.replace('.', "_")));
            let error = create_community_module(
                "unsafe_test",
                &destination,
                &CommunityModuleOptions {
                    min_version: minimum.to_owned(),
                    ..CommunityModuleOptions::default()
                },
            )
            .unwrap_err();
            assert!(error.to_string().contains("major.minor.patch"));
            assert!(!destination.exists());
        }
        assert_eq!(fs::read_dir(&temporary.0).unwrap().count(), 0);
    }

    #[test]
    fn existing_module_is_preserved() {
        let temporary = TestDirectory::new();
        let existing = temporary.0.join("audio_tools");
        fs::create_dir(&existing).unwrap();
        fs::write(existing.join("keep.txt"), "user file").unwrap();
        assert!(
            create_community_module(
                "audio_tools",
                &temporary.0,
                &CommunityModuleOptions::default()
            )
            .is_err()
        );
        assert_eq!(
            fs::read_to_string(existing.join("keep.txt")).unwrap(),
            "user file"
        );
    }

    #[test]
    fn file_used_as_destination_is_preserved() {
        let temporary = TestDirectory::new();
        let destination = temporary.0.join("destination-file");
        fs::write(&destination, "user file").unwrap();
        let error = create_community_module(
            "audio_tools",
            &destination,
            &CommunityModuleOptions::default(),
        )
        .unwrap_err();
        assert!(!error.to_string().is_empty());
        assert_eq!(fs::read_to_string(destination).unwrap(), "user file");
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_destination_is_rejected_without_writing_through_it() {
        use std::os::unix::fs::symlink;

        let temporary = TestDirectory::new();
        let target = temporary.0.join("target");
        let link = temporary.0.join("link");
        fs::create_dir(&target).unwrap();
        symlink(&target, &link).unwrap();
        assert!(
            create_community_module("audio_tools", &link, &CommunityModuleOptions::default())
                .is_err()
        );
        assert!(fs::read_dir(target).unwrap().next().is_none());
    }

    #[cfg(unix)]
    #[test]
    fn linked_ancestor_is_rejected_before_creating_destination() {
        use std::os::unix::fs::symlink;

        let temporary = TestDirectory::new();
        let target = temporary.0.join("target");
        let linked_parent = temporary.0.join("linked-parent");
        let destination = linked_parent.join("modules");
        fs::create_dir(&target).unwrap();
        symlink(&target, &linked_parent).unwrap();

        let error = create_community_module(
            "audio_tools",
            &destination,
            &CommunityModuleOptions::default(),
        )
        .unwrap_err();

        assert!(error.to_string().contains("reparse point"));
        assert!(!target.join("modules").exists());
        assert!(fs::read_dir(&target).unwrap().next().is_none());
    }
}
