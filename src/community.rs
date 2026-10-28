use serde_json::Value;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

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
    let valid_start = characters.next().is_some_and(|character| character.is_ascii_lowercase());
    let valid_rest = characters.all(|character| character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_');
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
            "Minimum MayaFlux version must use major.minor.patch format, for example 0.5.0.".to_owned(),
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
    fs::canonicalize(absolute).map_err(Into::into)
}

fn populate_module(
    module_path: &Path,
    name: &str,
    options: &CommunityModuleOptions,
) -> Result<Option<String>, CommunityCreationError> {
    fs::create_dir(module_path.join("src"))?;
    fs::create_dir(module_path.join("test"))?;

    let mut manifest: Value = serde_json::from_str(include_str!("../templates/community/community.json"))
        .map_err(|error| CommunityCreationError(format!("Embedded community manifest template is invalid: {error}")))?;
    manifest["name"] = Value::String(name.to_owned());
    manifest["description"] = Value::String(options.description.clone());
    manifest["min_version"] = Value::String(options.min_version.clone());
    manifest["needs_lila"] = Value::Bool(options.needs_lila);

    let module_cmake = include_str!("../templates/community/module.cmake")
        .replace("@MODULE_NAME@", name)
        .replace(
            "set(MF_MIN_VERSION \"0.5.0\")",
            &format!("set(MF_MIN_VERSION \"{}\")", options.min_version),
        )
        .replace(
            "set(MF_NEEDS_LILA OFF)",
            &format!("set(MF_NEEDS_LILA {})", if options.needs_lila { "ON" } else { "OFF" }),
        );

    let mut test_cmake = include_str!("../templates/community/test/CMakeLists.txt")
        .replace("@MODULE_NAME@", name);
    let settings = format!(
        "set(MF_MIN_VERSION \"{}\")\nset(MF_NEEDS_LILA {})",
        options.min_version,
        if options.needs_lila { "ON" } else { "OFF" }
    );
    let minimum_line_end = test_cmake
        .find('\n')
        .ok_or_else(|| CommunityCreationError("Community test CMake template has no minimum-version line.".to_owned()))?;
    test_cmake.insert_str(minimum_line_end + 1, &format!("\n{settings}\n"));

    let root_cmake = format!(
        "cmake_minimum_required(VERSION 3.25)\nproject({name} LANGUAGES C CXX)\nenable_testing()\nadd_subdirectory(test)\n"
    );
    let readme = format!(
        "# {name}\n\n{}\n\nAdd module implementation files under `src/` and checks in `test/test_{name}.cpp`.\n\nWith MayaFlux and its required build tools installed, configure, build, and test with:\n\n```sh\ncmake --preset release\ncmake --build --preset release\nctest --preset release\n```\n\nGit is optional for building. To initialize the repository later, run `git init -b main` and `git add -A` from this directory.\n",
        if options.description.is_empty() {
            "A MayaFlux community module."
        } else {
            options.description.as_str()
        }
    );

    let files = [
        (".gitignore".to_owned(), include_str!("../templates/.gitignore").to_owned()),
        ("CMakePresets.json".to_owned(), include_str!("../templates/community/CMakePresets.json").to_owned()),
        ("CMakeLists.txt".to_owned(), root_cmake),
        ("community.json".to_owned(), format!("{}\n", serde_json::to_string_pretty(&manifest).map_err(|error| CommunityCreationError(format!("Could not serialize community manifest: {error}")))?)),
        ("README.md".to_owned(), readme),
        (format!("{name}.cmake"), module_cmake),
        ("test/CMakeLists.txt".to_owned(), test_cmake),
        (format!("test/test_{name}.cpp"), "int main()\n{\n    return 0;\n}\n".to_owned()),
    ];

    for (relative, contents) in files {
        fs::write(module_path.join(relative), contents.as_bytes())?;
    }

    Ok(initialize_git(module_path))
}

fn initialize_git(module_path: &Path) -> Option<String> {
    let init = match Command::new("git")
        .arg("-C")
        .arg(module_path)
        .args(["init", "-q", "-b", "main"])
        .output()
    {
        Ok(output) => output,
        Err(error) => return Some(format!("Git setup was skipped: {error}. Initialize it later with `git init -b main` and `git add -A` in the module directory.")),
    };
    if !init.status.success() {
        return Some(format!(
            "Git could not initialize the module repository: {}. The module files are ready; initialize Git later if needed.",
            String::from_utf8_lossy(&init.stderr).trim()
        ));
    }

    let add = match Command::new("git")
        .arg("-C")
        .arg(module_path)
        .args(["add", "--all"])
        .output()
    {
        Ok(output) => output,
        Err(error) => return Some(format!("Git could not stage the module files: {error}. The module files are ready; run `git add -A` in the module directory if needed.")),
    };
    if !add.status.success() {
        return Some(format!(
            "Git could not stage the module files: {}. The module files are ready; run `git add -A` in the module directory if needed.",
            String::from_utf8_lossy(&add.stderr).trim()
        ));
    }
    None
}
