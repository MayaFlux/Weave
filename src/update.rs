use serde_json::Value;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

const REGISTRY_URL: &str = "https://raw.githubusercontent.com/MayaFlux/community-sources-registry/main/registry.json";

#[derive(Clone, Debug)]
struct RegistryEntry {
    repository: String,
    minimum: String,
}

#[derive(Debug)]
pub struct UpdateError(String);

impl std::fmt::Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(&self.0) }
}

impl std::error::Error for UpdateError {}

fn fail(message: impl Into<String>) -> UpdateError { UpdateError(message.into()) }

fn valid_module_name(name: &str) -> bool {
    let mut characters = name.chars();
    let valid = characters.next().is_some_and(|character| character.is_ascii_lowercase())
        && characters.all(|character| character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_');
    let reserved = ["con", "prn", "aux", "nul"].contains(&name)
        || (name.len() == 4
            && (name.starts_with("com") || name.starts_with("lpt"))
            && matches!(name.as_bytes()[3], b'1'..=b'9'));
    valid && !reserved
}

fn version(value: &str, label: &str) -> Result<[u64; 3], UpdateError> {
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() != 3 || parts.iter().any(|part| part.is_empty() || (part.len() > 1 && part.starts_with('0')) || !part.bytes().all(|byte| byte.is_ascii_digit())) {
        return Err(fail(format!("Malformed {label} version: {value}")));
    }
    let parsed: Result<Vec<_>, _> = parts.iter().map(|part| part.parse::<u64>()).collect();
    let parsed = parsed.map_err(|_| fail(format!("Malformed {label} version: {value}")))?;
    Ok([parsed[0], parsed[1], parsed[2]])
}

fn module_names(values: &[String]) -> Result<Vec<String>, UpdateError> {
    let mut names = Vec::new();
    for value in values {
        for part in value.split(',') {
            let name = part.trim();
            if name.is_empty() { return Err(fail("Empty module name in request")); }
            if !valid_module_name(name) {
                return Err(fail(format!("Invalid module name: {name}")));
            }
            if !names.iter().any(|existing| existing == name) { names.push(name.to_owned()); }
        }
    }
    if names.is_empty() { return Err(fail("At least one module name is required")); }
    Ok(names)
}

fn installed_version() -> Result<(String, [u64; 3]), UpdateError> {
    let roots: Vec<PathBuf> = if let Some(root) = std::env::var_os("MAYAFLUX_ROOT").filter(|value| !value.is_empty()) {
        vec![PathBuf::from(root)]
    } else {
        #[cfg(target_os = "macos")]
        let roots = {
            let mut result = Vec::new();
            let paths = std::env::var_os("PATH").unwrap_or_default();
            let mut brews: Vec<_> = std::env::split_paths(&paths).map(|path| path.join("brew")).collect();
            if let Ok(entries) = fs::read_dir("/opt") { brews.extend(entries.flatten().map(|entry| entry.path().join("bin/brew"))); }
            brews.retain(|path| path.is_file() && !path.starts_with("/usr/local"));
            brews.sort(); brews.dedup();
            for brew in brews {
                for formula in ["mayaflux", "mayaflux-dev"] {
                    if let Ok(output) = Command::new(&brew).args(["--prefix", formula]).output() {
                        if output.status.success() { result.push(PathBuf::from(String::from_utf8_lossy(&output.stdout).trim())); }
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
            let path = root.join(library).join("cmake/MayaFlux/MayaFluxConfigVersion.cmake");
            checked.push(path.display().to_string());
            if !path.is_file() { continue; }
            let source = fs::read_to_string(&path).map_err(|error| fail(format!("Could not read MayaFlux version file {}: {error}", path.display())))?;
            return parse_installed_version(&source, &path);
        }
    }
    Err(fail(format!("MayaFlux version file not found. Checked: {}", checked.join(", "))))
}

fn parse_installed_version(source: &str, path: &Path) -> Result<(String, [u64; 3]), UpdateError> {
    let marker = "set(PACKAGE_VERSION";
    let tail = source.split_once(marker).map(|(_, tail)| tail).ok_or_else(|| fail(format!("Malformed MayaFlux version file: {}", path.display())))?;
    let text = tail.trim_start().strip_prefix('"').and_then(|tail| tail.split_once('"').map(|(value, _)| value)).ok_or_else(|| fail(format!("Malformed MayaFlux version file: {}", path.display())))?;
    Ok((text.to_owned(), version(text, "installed MayaFlux")?))
}

fn parse_registry(source: &str) -> Result<std::collections::HashMap<String, RegistryEntry>, UpdateError> {
    let data: Value = serde_json::from_str(source.trim_start_matches('\u{FEFF}')).map_err(|error| fail(format!("Invalid community registry JSON: {error}")))?;
    let records = data.as_array().ok_or_else(|| fail("Community registry must be a JSON array"))?;
    let mut entries = std::collections::HashMap::new();
    for record in records {
        let name = record.get("name").and_then(Value::as_str).ok_or_else(|| fail("Community registry contains an invalid module name"))?;
        let repo = record.get("repo").and_then(Value::as_str).ok_or_else(|| fail(format!("Invalid repository for {name}")))?;
        let minimum = record.get("min_version").and_then(Value::as_str).ok_or_else(|| fail(format!("Missing minimum version for {name}")))?;
        let valid_name = valid_module_name(name);
        let authority = repo.strip_prefix("https://").and_then(|rest| rest.split(['/', '?']).next());
        let valid_repo = authority.is_some_and(|host| !host.is_empty() && !host.contains('@'))
            && !repo.chars().any(char::is_whitespace)
            && !repo.contains('#')
            && !repo.contains('\\');
        if !valid_name || !valid_repo {
            return Err(fail("Community registry contains an invalid entry"));
        }
        version(minimum, &format!("registry entry for {name}"))?;
        if entries.insert(name.to_owned(), RegistryEntry { repository: repo.to_owned(), minimum: minimum.to_owned() }).is_some() { return Err(fail(format!("Duplicate registry entry for {name}"))); }
    }
    Ok(entries)
}

fn registry() -> Result<std::collections::HashMap<String, RegistryEntry>, UpdateError> {
    let output = registry_request().output().map_err(|error| fail(format!("Could not fetch community registry: {error}")))?;
    if !output.status.success() { return Err(fail(format!("Could not fetch community registry: {}", String::from_utf8_lossy(&output.stderr).trim()))); }
    parse_registry(std::str::from_utf8(&output.stdout).map_err(|error| fail(format!("Community registry is not UTF-8: {error}")))?)
}

fn registry_request() -> Command {
    let mut command = Command::new("curl");
    command.args(["--fail", "--location", "--silent", "--show-error", "--max-time", "30", REGISTRY_URL]);
    command
}

fn clone_request(repository: &str, destination: &Path) -> Command {
    let mut command = Command::new("git");
    command.args(["clone", "--progress", "--depth=1", repository]).arg(destination);
    command
}

fn read_manifest(path: &Path, name: &str) -> Result<([u64; 3], Value), UpdateError> {
    let manifest_path = path.join("community.json");
    let source = fs::read_to_string(&manifest_path).map_err(|error| fail(format!("Could not read manifest for {name}: {error}")))?;
    let source = source.trim_start_matches('\u{FEFF}');
    let manifest: Value = serde_json::from_str(source).map_err(|error| fail(format!("Invalid manifest for {name}: {error}")))?;
    if manifest.get("name").and_then(Value::as_str) != Some(name) { return Err(fail(format!("Invalid manifest for {name}: name does not match"))); }
    let minimum = manifest.get("min_version").and_then(Value::as_str).ok_or_else(|| fail(format!("Module {name} has no minimum version")))?;
    Ok((version(minimum, &format!("manifest for {name}"))?, manifest))
}

trait UpdateServices {
    fn registry(&self) -> Result<std::collections::HashMap<String, RegistryEntry>, UpdateError>;
    fn installed_version(&self) -> Result<(String, [u64; 3]), UpdateError>;
    fn clone_module(&self, repository: &str, destination: &Path) -> Result<(), UpdateError>;
}

struct LiveUpdateServices;

impl UpdateServices for LiveUpdateServices {
    fn registry(&self) -> Result<std::collections::HashMap<String, RegistryEntry>, UpdateError> { registry() }
    fn installed_version(&self) -> Result<(String, [u64; 3]), UpdateError> { installed_version() }
    fn clone_module(&self, repository: &str, destination: &Path) -> Result<(), UpdateError> {
        let output = clone_request(repository, destination).output().map_err(|error| fail(format!("Could not run Git clone: {error}")))?;
        if !output.status.success() { return Err(fail(format!("Git clone failed for {}: {}", destination.file_name().unwrap_or_default().to_string_lossy(), String::from_utf8_lossy(&output.stderr).trim()))); }
        Ok(())
    }
}

pub fn update_project(project_path: &Path, values: &[String], report: impl FnMut(String)) -> Result<(), UpdateError> {
    update_project_with(project_path, values, &LiveUpdateServices, report)
}

fn update_project_with(project_path: &Path, values: &[String], services: &impl UpdateServices, mut report: impl FnMut(String)) -> Result<(), UpdateError> {
    let names = module_names(values)?;
    if std::fs::symlink_metadata(project_path).map(|m| m.file_type().is_symlink()).unwrap_or(false) { return Err(fail(format!("Project path is a symbolic link: {}", project_path.display()))); }
    let project = fs::canonicalize(project_path).map_err(|error| fail(format!("Could not open project path {}: {error}", project_path.display())))?;
    if !project.join("CMakeLists.txt").is_file() { return Err(fail(format!("Not a MayaFlux project: {}", project.display()))); }
    let community = project.join("community");
    let registration = project.join("community.cmake");
    for path in [&community, &registration] {
        if fs::symlink_metadata(path).map(|m| m.file_type().is_symlink()).unwrap_or(false) { return Err(fail(format!("Community path is a symbolic link: {}", path.display()))); }
    }
    report("Fetching community registry...".to_owned());
    let entries = services.registry()?;
    let (installed_text, installed) = services.installed_version()?;
    for name in &names {
        let entry = entries.get(name).ok_or_else(|| fail(format!("Module {name} is not in the community registry")))?;
        let registry_minimum = version(&entry.minimum, &format!("registry entry for {name}"))?;
        if installed < registry_minimum {
            return Err(fail(format!("Module {name} requires MayaFlux >= {}.{}.{}; found {installed_text}", registry_minimum[0], registry_minimum[1], registry_minimum[2])));
        }
    }
    let community_existed = community.exists();
    let mut cloned: Vec<PathBuf> = Vec::new();
    let result = (|| {
        for name in &names {
            let entry = entries.get(name).ok_or_else(|| fail(format!("Module {name} is not in the community registry")))?;
            let module = community.join(name);
            report(format!("Preparing {name}..."));
            if fs::symlink_metadata(&module).map(|m| m.file_type().is_symlink()).unwrap_or(false) { return Err(fail(format!("Community module path is a symbolic link: {}", module.display()))); }
            if !module.exists() {
                fs::create_dir_all(&community).map_err(|error| fail(format!("Could not create community directory: {error}")))?;
                report(format!("Cloning {} into {}", entry.repository, module.display()));
                cloned.push(module.clone());
                services.clone_module(&entry.repository, &module)?;
            } else if !module.is_dir() { return Err(fail(format!("Community path for {name} is not a directory: {}", module.display()))); }
            let cmake_file = module.join(format!("{name}.cmake"));
            let source_directory = module.join("src");
            let manifest_file = module.join("community.json");
            for path in [&cmake_file, &source_directory, &manifest_file] {
                if fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
                    return Err(fail(format!("Community module contains a symbolic link: {}", path.display())));
                }
            }
            if !cmake_file.is_file() { return Err(fail(format!("Module {name} is missing {name}.cmake"))); }
            if !source_directory.is_dir() { return Err(fail(format!("Module {name} is missing src/"))); }
            let (declared_min, manifest) = read_manifest(&module, name)?;
            let required_min = version(&entry.minimum, &format!("registry entry for {name}"))?.max(declared_min);
            if installed < required_min { return Err(fail(format!("Module {name} requires MayaFlux >= {}.{}.{}; found {installed_text}", required_min[0], required_min[1], required_min[2]))); }
            let has_requirements = manifest.get("packages").is_some_and(|value| value.as_object().is_some_and(|items| !items.is_empty()))
                || manifest.get("manual_installs").is_some_and(|value| value.as_array().is_some_and(|items| !items.is_empty()))
                || manifest.get("assets").is_some_and(|value| value.as_array().is_some_and(|items| !items.is_empty()));
            if has_requirements { report(format!("{name} declares requirements in community/{name}/community.json. Weave does not install them; install them with your package manager.")); }
            report(format!("Validated {name}"));
        }
        let current = if registration.exists() { fs::read_to_string(&registration).map_err(|error| fail(format!("Could not read community.cmake: {error}")))? } else { String::new() };
        let existing: Vec<_> = current.lines().map(str::trim).collect();
        let additions: Vec<_> = names.iter().filter(|name| !existing.contains(&name.as_str())).collect();
        let added: std::collections::HashSet<_> = additions.iter().map(|name| name.as_str()).collect();
        if !additions.is_empty() {
            let mut updated = current;
            if !updated.is_empty() && !updated.ends_with('\n') { updated.push('\n'); }
            for name in additions { updated.push_str(name); updated.push('\n'); }
            fs::write(&registration, updated).map_err(|error| fail(format!("Could not update community.cmake: {error}")))?;
        }
        for name in &names {
            report(if added.contains(name.as_str()) {
                format!("Registered {name} in community.cmake")
            } else {
                format!("{name} was already registered")
            });
        }
        report("Done. Rebuild the project to include the validated modules.".to_owned());
        Ok(())
    })();
    if let Err(error) = result {
        let mut leftovers = Vec::new();
        for path in cloned.iter().rev() { if let Err(cleanup) = fs::remove_dir_all(path) { if cleanup.kind() != io::ErrorKind::NotFound { leftovers.push(path.display().to_string()); } } }
        if !community_existed { let _ = fs::remove_dir(&community); }
        if leftovers.is_empty() { return Err(error); }
        return Err(fail(format!("{error}. Incomplete clones could not be removed: {}", leftovers.join(", "))));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::sync::atomic::{AtomicU64, Ordering};

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            static NEXT_ID: AtomicU64 = AtomicU64::new(0);

            loop {
                let sequence = NEXT_ID.fetch_add(1, Ordering::Relaxed);
                let path = std::env::temp_dir().join(format!(
                    "weave-update-{}-{sequence}",
                    std::process::id()
                ));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!("Could not create test directory {}: {error}", path.display()),
                }
            }
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
    }

    struct FixtureServices {
        entries: std::collections::HashMap<String, RegistryEntry>,
        installed: Result<(String, [u64; 3]), String>,
        registry_error: Cell<bool>,
        registry_calls: Cell<u32>,
        clone_error: Cell<bool>,
        partial_clone: Cell<bool>,
        invalid_clone: RefCell<Option<String>>,
        clone_requests: RefCell<Vec<(String, PathBuf)>>,
    }

    impl FixtureServices {
        fn new(names: &[&str]) -> Self {
            let entries = names.iter().map(|name| (
                (*name).to_owned(),
                RegistryEntry { repository: format!("https://example.invalid/{name}.git"), minimum: "0.5.0".to_owned() },
            )).collect();
            Self {
                entries,
                installed: Ok(("0.8.0".to_owned(), [0, 8, 0])),
                registry_error: Cell::new(false),
                registry_calls: Cell::new(0),
                clone_error: Cell::new(false),
                partial_clone: Cell::new(false),
                invalid_clone: RefCell::new(None),
                clone_requests: RefCell::new(Vec::new()),
            }
        }
    }

    impl UpdateServices for FixtureServices {
        fn registry(&self) -> Result<std::collections::HashMap<String, RegistryEntry>, UpdateError> {
            self.registry_calls.set(self.registry_calls.get() + 1);
            if self.registry_error.get() { Err(fail("fixture registry unavailable")) } else { Ok(self.entries.clone()) }
        }

        fn installed_version(&self) -> Result<(String, [u64; 3]), UpdateError> {
            self.installed.clone().map_err(fail)
        }

        fn clone_module(&self, repository: &str, destination: &Path) -> Result<(), UpdateError> {
            self.clone_requests.borrow_mut().push((repository.to_owned(), destination.to_path_buf()));
            if self.clone_error.get() {
                if self.partial_clone.get() {
                    fs::create_dir_all(destination).unwrap();
                    fs::write(destination.join("partial.txt"), "incomplete").unwrap();
                }
                return Err(fail("fixture clone failed"));
            }
            fs::create_dir_all(destination.join("src")).unwrap();
            let name = destination.file_name().unwrap().to_string_lossy();
            fs::write(destination.join(format!("{name}.cmake")), "module").unwrap();
            let manifest = if self.invalid_clone.borrow().as_deref() == Some(name.as_ref()) {
                "{invalid".to_owned()
            } else {
                format!("{{\"name\":\"{name}\",\"min_version\":\"0.5.0\"}}")
            };
            fs::write(destination.join("community.json"), manifest).unwrap();
            Ok(())
        }
    }

    struct ProjectFixture {
        directory: TestDirectory,
        services: FixtureServices,
    }

    impl ProjectFixture {
        fn new(names: &[&str]) -> Self {
            let directory = TestDirectory::new();
            fs::write(directory.0.join("CMakeLists.txt"), "project(Fixture)").unwrap();
            fs::write(directory.0.join("src.cpp"), "user content").unwrap();
            Self { directory, services: FixtureServices::new(names) }
        }

        fn module(&self, name: &str, manifest: &str) -> PathBuf {
            let module = self.directory.0.join("community").join(name);
            fs::create_dir_all(module.join("src")).unwrap();
            fs::write(module.join(format!("{name}.cmake")), "module").unwrap();
            fs::write(module.join("notes.txt"), "preserve me").unwrap();
            fs::write(module.join("community.json"), manifest).unwrap();
            module
        }

        fn run(&self, names: &[&str], output: &mut Vec<String>) -> Result<(), UpdateError> {
            let names: Vec<_> = names.iter().map(|name| (*name).to_owned()).collect();
            update_project_with(&self.directory.0, &names, &self.services, |line| output.push(line))
        }
    }

    #[test]
    fn module_names_keep_order_deduplicate_and_reject_empty_or_unsafe_parts() {
        assert_eq!(module_names(&[" terrain_field, noise ".to_owned(), "noise,sky".to_owned()]).unwrap(), vec!["terrain_field".to_owned(), "noise".to_owned(), "sky".to_owned()]);
        for value in ["", "terrain_field,", "../terrain", "con", "Bad_Name", "name/name"] {
            assert!(module_names(&[value.to_owned()]).is_err(), "accepted {value:?}");
        }
        assert!(module_names(&[]).is_err());
    }

    #[test]
    fn versions_accept_three_numeric_components_and_reject_malformed_values() {
        assert_eq!(version("0.8.12", "test").unwrap(), [0, 8, 12]);
        for value in ["", "0.8", "0.8.1.2", "01.8.0", "0.x.0", "18446744073709551616.0.0"] {
            assert!(version(value, "test").is_err(), "accepted {value:?}");
        }
    }

    #[test]
    fn installed_version_file_requires_a_well_formed_cmake_version_value() {
        assert_eq!(parse_installed_version("set(PACKAGE_VERSION \"0.8.0\")", Path::new("MayaFluxConfigVersion.cmake")).unwrap(), ("0.8.0".to_owned(), [0, 8, 0]));
        for source in ["set(PACKAGE_VERSION 0.8.0)", "set(PACKAGE_VERSION \"0.8\")", "set(PACKAGE_VERSION nope)"] {
            assert!(parse_installed_version(source, Path::new("MayaFluxConfigVersion.cmake")).is_err());
        }
    }

    #[test]
    fn registry_parser_accepts_bom_and_rejects_bad_shape_duplicates_urls_and_versions() {
        let valid = "\u{FEFF} [ { \"name\": \"terrain_field\", \"repo\": \"https://example.invalid/terrain.git\", \"min_version\": \"0.5.0\" } ]";
        assert_eq!(parse_registry(valid).unwrap()["terrain_field"].repository.as_str(), "https://example.invalid/terrain.git");
        for source in [
            "not-json",
            "{}",
            "[{}]",
            "[{\"name\":\"Bad\",\"repo\":\"https://example.invalid/a.git\",\"min_version\":\"0.5.0\"}]",
            "[{\"name\":\"terrain_field\",\"repo\":\"file:///tmp/module\",\"min_version\":\"0.5.0\"}]",
            "[{\"name\":\"terrain_field\",\"repo\":\"https://user@example.invalid/a.git\",\"min_version\":\"0.5.0\"}]",
            "[{\"name\":\"terrain_field\",\"repo\":\"https://example.invalid/a.git\",\"min_version\":\"0.5\"}]",
            "[{\"name\":\"terrain_field\",\"repo\":\"https://example.invalid/a.git\",\"min_version\":\"0.5.0\"},{\"name\":\"terrain_field\",\"repo\":\"https://example.invalid/b.git\",\"min_version\":\"0.5.0\"}]",
        ] {
            assert!(parse_registry(source).is_err(), "accepted {source}");
        }
    }

    #[test]
    fn external_commands_use_separate_arguments_and_bounded_registry_wait() {
        let request = registry_request();
        assert_eq!(request.get_program(), "curl");
        assert_eq!(request.get_args().map(|arg| arg.to_string_lossy().into_owned()).collect::<Vec<_>>(), vec![
            "--fail".to_owned(), "--location".to_owned(), "--silent".to_owned(), "--show-error".to_owned(), "--max-time".to_owned(), "30".to_owned(), REGISTRY_URL.to_owned(),
        ]);

        let clone = clone_request("https://example.invalid/module.git", Path::new("/tmp/module"));
        assert_eq!(clone.get_program(), "git");
        assert_eq!(clone.get_args().map(|arg| arg.to_string_lossy().into_owned()).collect::<Vec<_>>(), vec![
            "clone".to_owned(), "--progress".to_owned(), "--depth=1".to_owned(), "https://example.invalid/module.git".to_owned(), "/tmp/module".to_owned(),
        ]);
    }

    #[test]
    fn invalid_project_is_rejected_before_registry_access() {
        let fixture = ProjectFixture::new(&["terrain_field"]);
        fs::remove_file(fixture.directory.0.join("CMakeLists.txt")).unwrap();
        let error = fixture.run(&["terrain_field"], &mut Vec::new()).unwrap_err();
        assert!(error.to_string().contains("Not a MayaFlux project"));
        assert_eq!(fixture.services.registry_calls.get(), 0);
        assert!(fixture.services.clone_requests.borrow().is_empty());
    }

    #[test]
    fn registry_and_installed_version_failures_leave_project_untouched() {
        let mut fixture = ProjectFixture::new(&["terrain_field"]);
        fixture.services.registry_error.set(true);
        let error = fixture.run(&["terrain_field"], &mut Vec::new()).unwrap_err();
        assert!(error.to_string().contains("fixture registry unavailable"));
        assert!(!fixture.directory.0.join("community").exists());

        fixture.services.registry_error.set(false);
        fixture.services.installed = Err("malformed MayaFlux version file".to_owned());
        let error = fixture.run(&["terrain_field"], &mut Vec::new()).unwrap_err();
        assert!(error.to_string().contains("malformed MayaFlux version file"));
        assert!(!fixture.directory.0.join("community").exists());
        assert!(fixture.services.clone_requests.borrow().is_empty());
    }

    #[test]
    fn unknown_module_is_rejected_before_any_clone_or_registration() {
        let fixture = ProjectFixture::new(&["terrain_field"]);
        let error = fixture.run(&["terrain_field", "unknown_module"], &mut Vec::new()).unwrap_err();
        assert!(error.to_string().contains("not in the community registry"));
        assert!(fixture.services.clone_requests.borrow().is_empty());
        assert!(!fixture.directory.0.join("community.cmake").exists());
    }

    #[test]
    fn update_clones_validates_registers_and_preserves_existing_project_content() {
        let fixture = ProjectFixture::new(&["terrain_field", "noise"]);
        fixture.module("terrain_field", "{\"name\":\"terrain_field\",\"min_version\":\"0.5.0\"}");
        fs::write(fixture.directory.0.join("community.cmake"), "# user entry\r\nterrain_field").unwrap();
        let mut output = Vec::new();

        fixture.run(&["terrain_field,noise", "noise"], &mut output).unwrap();

        assert_eq!(fs::read_to_string(fixture.directory.0.join("community.cmake")).unwrap(), "# user entry\r\nterrain_field\nnoise\n");
        assert_eq!(fs::read_to_string(fixture.directory.0.join("src.cpp")).unwrap(), "user content");
        assert_eq!(fs::read_to_string(fixture.directory.0.join("community/terrain_field/notes.txt")).unwrap(), "preserve me");
        assert_eq!(fixture.services.clone_requests.borrow().as_slice(), &[("https://example.invalid/noise.git".to_owned(), fs::canonicalize(&fixture.directory.0).unwrap().join("community/noise"))]);
        assert!(output.iter().any(|line| line.contains("Validated noise")));

        fixture.run(&["noise"], &mut output).unwrap();
        assert_eq!(fs::read_to_string(fixture.directory.0.join("community.cmake")).unwrap(), "# user entry\r\nterrain_field\nnoise\n");
        assert!(output.iter().any(|line| line.contains("noise was already registered")));
    }

    #[test]
    fn utf8_bom_manifest_is_accepted_and_declared_requirements_are_reported() {
        let fixture = ProjectFixture::new(&["terrain_field"]);
        fixture.module("terrain_field", "\u{FEFF}{\"name\":\"terrain_field\",\"min_version\":\"0.5.0\",\"packages\":{\"macos\":{\"brew\":{\"formulae\":[\"example\"]}}}}");
        let mut output = Vec::new();
        fixture.run(&["terrain_field"], &mut output).unwrap();
        assert!(output.iter().any(|line| line.contains("declares requirements")));
        assert_eq!(fs::read_to_string(fixture.directory.0.join("community.cmake")).unwrap(), "terrain_field\n");
    }

    #[test]
    fn module_validation_and_registry_minimum_fail_without_changing_registration() {
        let mut fixture = ProjectFixture::new(&["terrain_field"]);
        fixture.module("terrain_field", "{\"name\":\"other\",\"min_version\":\"0.5.0\"}");
        fs::write(fixture.directory.0.join("community.cmake"), "existing_entry\n").unwrap();
        let error = fixture.run(&["terrain_field"], &mut Vec::new()).unwrap_err();
        assert!(error.to_string().contains("name does not match"));
        assert_eq!(fs::read_to_string(fixture.directory.0.join("community.cmake")).unwrap(), "existing_entry\n");

        fixture.services.entries.get_mut("terrain_field").unwrap().minimum = "0.10.0".to_owned();
        fixture.module("terrain_field", "{\"name\":\"terrain_field\",\"min_version\":\"0.5.0\"}");
        let error = fixture.run(&["terrain_field"], &mut Vec::new()).unwrap_err();
        assert!(error.to_string().contains("requires MayaFlux >= 0.10.0"));
        assert_eq!(fs::read_to_string(fixture.directory.0.join("community.cmake")).unwrap(), "existing_entry\n");
    }

    #[test]
    fn malformed_and_incomplete_existing_modules_preserve_registration_and_user_files() {
        for kind in ["missing-cmake", "missing-src", "missing-manifest", "bad-json", "wrong-name", "bad-version"] {
            let fixture = ProjectFixture::new(&["terrain_field"]);
            let module = fixture.module("terrain_field", "{\"name\":\"terrain_field\",\"min_version\":\"0.5.0\"}");
            match kind {
                "missing-cmake" => fs::remove_file(module.join("terrain_field.cmake")).unwrap(),
                "missing-src" => fs::remove_dir(module.join("src")).unwrap(),
                "missing-manifest" => fs::remove_file(module.join("community.json")).unwrap(),
                "bad-json" => fs::write(module.join("community.json"), "{bad").unwrap(),
                "wrong-name" => fs::write(module.join("community.json"), "{\"name\":\"other\",\"min_version\":\"0.5.0\"}").unwrap(),
                "bad-version" => fs::write(module.join("community.json"), "{\"name\":\"terrain_field\",\"min_version\":\"0.5\"}").unwrap(),
                _ => unreachable!(),
            }
            let registration = fixture.directory.0.join("community.cmake");
            fs::write(&registration, "user_entry\n").unwrap();
            let error = fixture.run(&["terrain_field"], &mut Vec::new()).unwrap_err();
            assert!(!error.to_string().is_empty(), "{kind}");
            assert_eq!(fs::read_to_string(&registration).unwrap(), "user_entry\n", "{kind}");
            assert_eq!(fs::read_to_string(module.join("notes.txt")).unwrap(), "preserve me", "{kind}");
        }
    }

    #[test]
    fn manifest_minimum_version_is_enforced_independently_of_registry_minimum() {
        let fixture = ProjectFixture::new(&["terrain_field"]);
        fixture.module("terrain_field", "{\"name\":\"terrain_field\",\"min_version\":\"2.0.0\"}");
        let error = fixture.run(&["terrain_field"], &mut Vec::new()).unwrap_err();
        assert!(error.to_string().contains("requires MayaFlux >= 2.0.0"));
        assert!(!fixture.directory.0.join("community.cmake").exists());
    }

    #[test]
    fn incompatible_registry_minimum_stops_before_creating_or_cloning_modules() {
        let mut fixture = ProjectFixture::new(&["terrain_field"]);
        fixture.services.entries.get_mut("terrain_field").unwrap().minimum = "0.10.0".to_owned();

        let error = fixture.run(&["terrain_field"], &mut Vec::new()).unwrap_err();

        assert!(error.to_string().contains("requires MayaFlux >= 0.10.0; found 0.8.0"));
        assert!(fixture.services.clone_requests.borrow().is_empty());
        assert!(!fixture.directory.0.join("community").exists());
        assert!(!fixture.directory.0.join("community.cmake").exists());
    }

    #[test]
    fn failed_partial_clone_is_removed_and_a_later_retry_succeeds() {
        let fixture = ProjectFixture::new(&["terrain_field"]);
        fixture.services.clone_error.set(true);
        fixture.services.partial_clone.set(true);
        let error = fixture.run(&["terrain_field"], &mut Vec::new()).unwrap_err();
        assert!(error.to_string().contains("fixture clone failed"));
        assert!(!fixture.directory.0.join("community/terrain_field").exists());
        assert!(!fixture.directory.0.join("community.cmake").exists());

        fixture.services.clone_error.set(false);
        fixture.services.partial_clone.set(false);
        fixture.run(&["terrain_field"], &mut Vec::new()).unwrap();
        assert_eq!(fs::read_to_string(fixture.directory.0.join("community.cmake")).unwrap(), "terrain_field\n");
    }

    #[test]
    fn validation_failure_rolls_back_new_clones_but_keeps_preexisting_modules() {
        let fixture = ProjectFixture::new(&["terrain_field", "noise"]);
        let existing = fixture.module("noise", "{\"name\":\"noise\",\"min_version\":\"0.5.0\"}");
        fixture.services.invalid_clone.replace(Some("terrain_field".to_owned()));
        fs::write(fixture.directory.0.join("community.cmake"), "noise\n").unwrap();

        let error = fixture.run(&["noise", "terrain_field"], &mut Vec::new()).unwrap_err();

        assert!(error.to_string().contains("Invalid manifest"));
        assert!(!fixture.directory.0.join("community/terrain_field").exists());
        assert!(existing.join("notes.txt").is_file());
        assert_eq!(fs::read_to_string(fixture.directory.0.join("community.cmake")).unwrap(), "noise\n");
    }

    #[test]
    fn registration_read_failure_rolls_back_only_modules_cloned_by_this_update() {
        let fixture = ProjectFixture::new(&["terrain_field", "noise"]);
        let existing = fixture.module("noise", "{\"name\":\"noise\",\"min_version\":\"0.5.0\"}");
        let registration = fixture.directory.0.join("community.cmake");
        fs::create_dir(&registration).unwrap();
        fs::write(registration.join("user-file"), "preserve").unwrap();

        let error = fixture.run(&["noise", "terrain_field"], &mut Vec::new()).unwrap_err();

        assert!(error.to_string().contains("Could not read community.cmake"));
        assert!(!fixture.directory.0.join("community/terrain_field").exists());
        assert!(existing.join("notes.txt").is_file());
        assert_eq!(fs::read_to_string(registration.join("user-file")).unwrap(), "preserve");
    }

    #[cfg(unix)]
    #[test]
    fn linked_project_community_and_registration_paths_are_rejected() {
        use std::os::unix::fs::symlink;

        let fixture = ProjectFixture::new(&["terrain_field"]);
        let target = fixture.directory.0.join("outside");
        fs::create_dir(&target).unwrap();
        symlink(&target, fixture.directory.0.join("community")).unwrap();
        let error = fixture.run(&["terrain_field"], &mut Vec::new()).unwrap_err();
        assert!(error.to_string().contains("symbolic link"));
        assert!(fs::read_dir(target).unwrap().next().is_none());
    }

    #[cfg(unix)]
    #[test]
    fn linked_project_root_and_module_paths_are_rejected() {
        use std::os::unix::fs::symlink;

        let fixture = ProjectFixture::new(&["terrain_field"]);
        let project_link = fixture.directory.0.join("project-link");
        symlink(&fixture.directory.0, &project_link).unwrap();
        let error = update_project_with(&project_link, &["terrain_field".to_owned()], &fixture.services, |_| {}).unwrap_err();
        assert!(error.to_string().contains("symbolic link"));
        fs::remove_file(project_link).unwrap();

        let outside = fixture.directory.0.join("outside");
        fs::create_dir(&outside).unwrap();
        fs::create_dir_all(fixture.directory.0.join("community")).unwrap();
        symlink(&outside, fixture.directory.0.join("community/terrain_field")).unwrap();
        let error = fixture.run(&["terrain_field"], &mut Vec::new()).unwrap_err();
        assert!(error.to_string().contains("symbolic link"));
        assert!(fs::read_dir(&outside).unwrap().next().is_none());
        fs::remove_file(fixture.directory.0.join("community/terrain_field")).unwrap();
        fs::remove_dir(outside).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn linked_module_manifest_sources_and_cmake_files_are_rejected_without_writes() {
        use std::os::unix::fs::symlink;

        for kind in ["cmake", "src", "manifest"] {
            let fixture = ProjectFixture::new(&["terrain_field"]);
            let module = fixture.module("terrain_field", "{\"name\":\"terrain_field\",\"min_version\":\"0.5.0\"}");
            let target = fixture.directory.0.join("outside-target");
            let linked = match kind {
                "cmake" => {
                    fs::write(&target, "outside").unwrap();
                    let path = module.join("terrain_field.cmake");
                    fs::remove_file(&path).unwrap();
                    path
                }
                "src" => {
                    fs::create_dir(&target).unwrap();
                    let path = module.join("src");
                    fs::remove_dir(&path).unwrap();
                    path
                }
                _ => {
                    fs::write(&target, "{\"name\":\"terrain_field\",\"min_version\":\"0.5.0\"}").unwrap();
                    let path = module.join("community.json");
                    fs::remove_file(&path).unwrap();
                    path
                }
            };
            symlink(&target, &linked).unwrap();
            fs::write(fixture.directory.0.join("community.cmake"), "keep_entry\n").unwrap();

            let error = fixture.run(&["terrain_field"], &mut Vec::new()).unwrap_err();

            assert!(error.to_string().contains("symbolic link"), "{kind}: {error}");
            assert_eq!(fs::read_to_string(fixture.directory.0.join("community.cmake")).unwrap(), "keep_entry\n");
        }
    }

    #[cfg(unix)]
    #[test]
    fn linked_registration_is_rejected_without_writing_through_it() {
        use std::os::unix::fs::symlink;

        let fixture = ProjectFixture::new(&["terrain_field"]);
        let target = fixture.directory.0.join("user-owned-registration");
        fs::write(&target, "keep_entry\n").unwrap();
        symlink(&target, fixture.directory.0.join("community.cmake")).unwrap();

        let error = fixture.run(&["terrain_field"], &mut Vec::new()).unwrap_err();

        assert!(error.to_string().contains("symbolic link"));
        assert_eq!(fs::read_to_string(target).unwrap(), "keep_entry\n");
        assert!(fixture.services.clone_requests.borrow().is_empty());
    }
}
