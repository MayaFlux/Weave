//! Community-module addition shared by the GUI and CLI.
//! The operation validates every requested module before changing registration.

mod installed;
mod registry;
#[cfg(test)]
mod tests;

#[cfg(test)]
use installed::parse_installed_version;
use installed::{installed_version, version};
#[cfg(test)]
use registry::{REGISTRY_URL, parse_registry, registry_request};
use registry::{RegistryEntry, registry};
use serde_json::Value;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Reports why community modules could not be added to a project.
/// Clone cleanup failures include the paths that need manual attention.
#[derive(Debug)]
pub struct ModuleAddError(String);

impl std::fmt::Display for ModuleAddError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ModuleAddError {}

fn fail(message: impl Into<String>) -> ModuleAddError {
    ModuleAddError(message.into())
}

/// Checks the portable module naming rules used by requests and registry entries.
fn valid_module_name(name: &str) -> bool {
    let mut characters = name.chars();
    let valid = characters
        .next()
        .is_some_and(|character| character.is_ascii_lowercase())
        && characters.all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        });
    let reserved = ["con", "prn", "aux", "nul"].contains(&name)
        || (name.len() == 4
            && (name.starts_with("com") || name.starts_with("lpt"))
            && matches!(name.as_bytes()[3], b'1'..=b'9'));
    valid && !reserved
}

/// Expands comma-separated names in request order and removes duplicates.
fn module_names(values: &[String]) -> Result<Vec<String>, ModuleAddError> {
    let mut names = Vec::new();
    for value in values {
        for part in value.split(',') {
            let name = part.trim();
            if name.is_empty() {
                return Err(fail("Empty module name in request"));
            }
            if !valid_module_name(name) {
                return Err(fail(format!("Invalid module name: {name}")));
            }
            if !names.iter().any(|existing| existing == name) {
                names.push(name.to_owned());
            }
        }
    }
    if names.is_empty() {
        return Err(fail("At least one module name is required"));
    }
    Ok(names)
}

/// Builds a clone command with separate arguments for the repository and path.
fn clone_request(repository: &str, destination: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .args(["clone", "--progress", "--depth=1", repository])
        .arg(destination);
    command
}

/// Reads a module manifest and returns its minimum MayaFlux version and data.
fn read_manifest(path: &Path, name: &str) -> Result<([u64; 3], Value), ModuleAddError> {
    let manifest_path = path.join("community.json");
    let source = fs::read_to_string(&manifest_path)
        .map_err(|error| fail(format!("Could not read manifest for {name}: {error}")))?;
    let manifest = crate::operations::community_manifest::parse_manifest(&source, name)
        .map_err(|error| fail(format!("Invalid manifest for {name}: {error}")))?;
    let minimum = version(&manifest.minimum_version, &format!("manifest for {name}"))?;
    Ok((minimum, manifest.data))
}

/// Supplies registry, installation, and clone operations to the workflow.
trait ModuleServices {
    fn registry(&self) -> Result<std::collections::HashMap<String, RegistryEntry>, ModuleAddError>;
    fn installed_version(&self) -> Result<(String, [u64; 3]), ModuleAddError>;
    fn clone_module(&self, repository: &str, destination: &Path) -> Result<(), ModuleAddError>;
}

/// Uses the host tools and filesystem for a real module addition.
struct LiveModuleServices;

impl ModuleServices for LiveModuleServices {
    fn registry(&self) -> Result<std::collections::HashMap<String, RegistryEntry>, ModuleAddError> {
        registry()
    }
    fn installed_version(&self) -> Result<(String, [u64; 3]), ModuleAddError> {
        installed_version()
    }
    fn clone_module(&self, repository: &str, destination: &Path) -> Result<(), ModuleAddError> {
        let output = clone_request(repository, destination)
            .output()
            .map_err(|error| fail(format!("Could not run Git clone: {error}")))?;
        if !output.status.success() {
            return Err(fail(format!(
                "Git clone failed for {}: {}",
                destination
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy(),
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok(())
    }
}

/// Adds the requested community modules to an existing MayaFlux project.
/// Reports progress and removes newly cloned modules if validation fails.
pub fn add_to_project(
    project_path: &Path,
    values: &[String],
    report: impl FnMut(String),
) -> Result<(), ModuleAddError> {
    add_to_project_with(project_path, values, &LiveModuleServices, report)
}

/// Resolves a project and rejects linked community paths before any network work.
fn project_paths(project_path: &Path) -> Result<(PathBuf, PathBuf), ModuleAddError> {
    crate::filesystem::no_links(project_path).map_err(fail)?;
    if std::fs::symlink_metadata(project_path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err(fail(format!(
            "Project path is a symbolic link: {}",
            project_path.display()
        )));
    }
    let project = fs::canonicalize(project_path).map_err(|error| {
        fail(format!(
            "Could not open project path {}: {error}",
            project_path.display()
        ))
    })?;
    crate::filesystem::no_links(&project).map_err(fail)?;
    if !project.join("CMakeLists.txt").is_file() {
        return Err(fail(format!(
            "Not a MayaFlux project: {}",
            project.display()
        )));
    }
    let community = project.join("community");
    let registration = project.join("community.cmake");
    for path in [&community, &registration] {
        crate::filesystem::no_links(path).map_err(fail)?;
        if fs::symlink_metadata(path)
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false)
        {
            return Err(fail(format!(
                "Community path is a symbolic link: {}",
                path.display()
            )));
        }
    }
    Ok((community, registration))
}

/// Rejects unavailable or incompatible modules before cloning any of them.
fn check_registry_minimum(
    names: &[String],
    entries: &std::collections::HashMap<String, RegistryEntry>,
    installed: [u64; 3],
    installed_text: &str,
) -> Result<(), ModuleAddError> {
    for name in names {
        let entry = entries
            .get(name)
            .ok_or_else(|| fail(format!("Module {name} is not in the community registry")))?;
        let registry_minimum = version(&entry.minimum, &format!("registry entry for {name}"))?;
        if installed < registry_minimum {
            return Err(fail(format!(
                "Module {name} requires MayaFlux >= {}.{}.{}; found {installed_text}",
                registry_minimum[0], registry_minimum[1], registry_minimum[2]
            )));
        }
    }
    Ok(())
}

/// Clones missing modules and validates their files and manifest requirements.
fn prepare_modules(
    names: &[String],
    entries: &std::collections::HashMap<String, RegistryEntry>,
    community: &Path,
    installed: [u64; 3],
    installed_text: &str,
    services: &impl ModuleServices,
    cloned: &mut Vec<PathBuf>,
    report: &mut impl FnMut(String),
) -> Result<(), ModuleAddError> {
    for name in names {
        let entry = entries
            .get(name)
            .ok_or_else(|| fail(format!("Module {name} is not in the community registry")))?;
        let module = community.join(name);
        report(format!("Preparing {name}..."));
        crate::filesystem::no_links(&module).map_err(fail)?;
        if fs::symlink_metadata(&module)
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false)
        {
            return Err(fail(format!(
                "Community module path is a symbolic link: {}",
                module.display()
            )));
        }
        if !module.exists() {
            crate::filesystem::no_links(&community).map_err(fail)?;
            fs::create_dir_all(&community)
                .map_err(|error| fail(format!("Could not create community directory: {error}")))?;
            report(format!(
                "Cloning {} into {}",
                entry.repository,
                module.display()
            ));
            cloned.push(module.clone());
            services.clone_module(&entry.repository, &module)?;
        } else if !module.is_dir() {
            return Err(fail(format!(
                "Community path for {name} is not a directory: {}",
                module.display()
            )));
        }
        let cmake_file = module.join(format!("{name}.cmake"));
        let source_directory = module.join("src");
        let manifest_file = module.join("community.json");
        for path in [&cmake_file, &source_directory, &manifest_file] {
            crate::filesystem::no_links(path).map_err(fail)?;
            if fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
                return Err(fail(format!(
                    "Community module contains a symbolic link: {}",
                    path.display()
                )));
            }
        }
        if !cmake_file.is_file() {
            return Err(fail(format!("Module {name} is missing {name}.cmake")));
        }
        if !source_directory.is_dir() {
            return Err(fail(format!("Module {name} is missing src/")));
        }
        let (declared_min, manifest) = read_manifest(&module, name)?;
        let required_min =
            version(&entry.minimum, &format!("registry entry for {name}"))?.max(declared_min);
        if installed < required_min {
            return Err(fail(format!(
                "Module {name} requires MayaFlux >= {}.{}.{}; found {installed_text}",
                required_min[0], required_min[1], required_min[2]
            )));
        }
        let has_requirements = manifest
            .get("packages")
            .is_some_and(|value| value.as_object().is_some_and(|items| !items.is_empty()))
            || manifest
                .get("manual_installs")
                .is_some_and(|value| value.as_array().is_some_and(|items| !items.is_empty()))
            || manifest
                .get("assets")
                .is_some_and(|value| value.as_array().is_some_and(|items| !items.is_empty()));
        if has_requirements {
            report(format!(
                "{name} declares requirements in community/{name}/community.json. Weave does not install them; install them with your package manager."
            ));
        }
        report(format!("Validated {name}"));
    }
    Ok(())
}

/// Atomically appends new names while preserving existing registration content.
fn register_modules(
    registration: &Path,
    names: &[String],
    report: &mut impl FnMut(String),
) -> Result<(), ModuleAddError> {
    let current = if registration.exists() {
        fs::read_to_string(&registration)
            .map_err(|error| fail(format!("Could not read community.cmake: {error}")))?
    } else {
        String::new()
    };
    let existing: Vec<_> = current.lines().map(str::trim).collect();
    let additions: Vec<_> = names
        .iter()
        .filter(|name| !existing.contains(&name.as_str()))
        .collect();
    let added: std::collections::HashSet<_> = additions.iter().map(|name| name.as_str()).collect();
    if !additions.is_empty() {
        let mut updated = current;
        if !updated.is_empty() && !updated.ends_with('\n') {
            updated.push('\n');
        }
        for name in additions {
            updated.push_str(name);
            updated.push('\n');
        }
        crate::filesystem::atomic_write(&registration, updated.as_bytes())
            .map_err(|error| fail(format!("Could not update community.cmake: {error}")))?;
    }
    for name in names {
        report(if added.contains(name.as_str()) {
            format!("Registered {name} in community.cmake")
        } else {
            format!("{name} was already registered")
        });
    }
    report("Done. Rebuild the project to include the validated modules.".to_owned());
    Ok(())
}

/// Runs the shared workflow with replaceable services for controlled tests.
fn add_to_project_with(
    project_path: &Path,
    values: &[String],
    services: &impl ModuleServices,
    mut report: impl FnMut(String),
) -> Result<(), ModuleAddError> {
    let names = module_names(values)?;
    let (community, registration) = project_paths(project_path)?;
    report("Fetching community registry...".to_owned());
    let entries = services.registry()?;
    let (installed_text, installed) = services.installed_version()?;
    check_registry_minimum(&names, &entries, installed, &installed_text)?;
    let community_existed = community.exists();
    let mut cloned: Vec<PathBuf> = Vec::new();
    let result = (|| {
        prepare_modules(
            &names,
            &entries,
            &community,
            installed,
            &installed_text,
            services,
            &mut cloned,
            &mut report,
        )?;
        register_modules(&registration, &names, &mut report)?;
        Ok(())
    })();
    if let Err(error) = result {
        let mut leftovers = Vec::new();
        for path in cloned.iter().rev() {
            if let Err(cleanup) = fs::remove_dir_all(path) {
                if cleanup.kind() != io::ErrorKind::NotFound {
                    leftovers.push(path.display().to_string());
                }
            }
        }
        if !community_existed {
            let _ = fs::remove_dir(&community);
        }
        if leftovers.is_empty() {
            return Err(error);
        }
        return Err(fail(format!(
            "{error}. Incomplete clones could not be removed: {}",
            leftovers.join(", ")
        )));
    }
    Ok(())
}
