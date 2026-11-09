use super::*;
use std::cell::{Cell, RefCell};
use std::sync::atomic::{AtomicU64, Ordering};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);

        loop {
            let sequence = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("weave-modules-{}-{sequence}", std::process::id()));
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

struct FixtureServices {
    entries: std::collections::HashMap<String, RegistryEntry>,
    installed: Result<(String, [u64; 3]), String>,
    registry_error: Cell<bool>,
    registry_calls: Cell<u32>,
    clone_error: Cell<bool>,
    partial_clone: Cell<bool>,
    invalid_clone: RefCell<Option<String>>,
    clone_requests: RefCell<Vec<(String, PathBuf)>>,
    opened_urls: RefCell<Vec<String>>,
    open_error: Cell<bool>,
    allow_browser: Cell<bool>,
}

impl FixtureServices {
    fn new(names: &[&str]) -> Self {
        let entries = names
            .iter()
            .map(|name| {
                (
                    (*name).to_owned(),
                    RegistryEntry {
                        repository: format!("https://example.invalid/{name}.git"),
                        minimum: "0.5.0".to_owned(),
                    },
                )
            })
            .collect();
        Self {
            entries,
            installed: Ok(("0.8.0".to_owned(), [0, 8, 0])),
            registry_error: Cell::new(false),
            registry_calls: Cell::new(0),
            clone_error: Cell::new(false),
            partial_clone: Cell::new(false),
            invalid_clone: RefCell::new(None),
            clone_requests: RefCell::new(Vec::new()),
            opened_urls: RefCell::new(Vec::new()),
            open_error: Cell::new(false),
            allow_browser: Cell::new(true),
        }
    }
}

impl ModuleServices for FixtureServices {
    fn registry(&self) -> Result<std::collections::HashMap<String, RegistryEntry>, ModuleAddError> {
        self.registry_calls.set(self.registry_calls.get() + 1);
        if self.registry_error.get() {
            Err(fail("fixture registry unavailable"))
        } else {
            Ok(self.entries.clone())
        }
    }

    fn installed_version(&self) -> Result<(String, [u64; 3]), ModuleAddError> {
        self.installed.clone().map_err(fail)
    }

    fn clone_module(&self, repository: &str, destination: &Path) -> Result<(), ModuleAddError> {
        self.clone_requests
            .borrow_mut()
            .push((repository.to_owned(), destination.to_path_buf()));
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

    fn open_url(&self, url: &str) -> Result<(), ModuleAddError> {
        self.opened_urls.borrow_mut().push(url.to_owned());
        if self.open_error.get() {
            Err(fail("fixture browser failed"))
        } else {
            Ok(())
        }
    }

    fn confirm_open_url(&self, _url: &str) -> bool {
        self.allow_browser.get()
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
        Self {
            directory,
            services: FixtureServices::new(names),
        }
    }

    fn module(&self, name: &str, manifest: &str) -> PathBuf {
        let module = self.directory.0.join("community").join(name);
        fs::create_dir_all(module.join("src")).unwrap();
        fs::write(module.join(format!("{name}.cmake")), "module").unwrap();
        fs::write(module.join("notes.txt"), "preserve me").unwrap();
        fs::write(module.join("community.json"), manifest).unwrap();
        module
    }

    fn run(&self, names: &[&str], output: &mut Vec<String>) -> Result<(), ModuleAddError> {
        let names: Vec<_> = names.iter().map(|name| (*name).to_owned()).collect();
        add_to_project_with(&self.directory.0, &names, &self.services, |line| {
            output.push(line)
        })
    }
}

#[test]
fn module_names_keep_order_deduplicate_and_reject_empty_or_unsafe_parts() {
    assert_eq!(
        module_names(&[" terrain_field, noise ".to_owned(), "noise,sky".to_owned()]).unwrap(),
        vec![
            "terrain_field".to_owned(),
            "noise".to_owned(),
            "sky".to_owned()
        ]
    );
    for value in [
        "",
        "terrain_field,",
        "../terrain",
        "con",
        "Bad_Name",
        "name/name",
    ] {
        assert!(
            module_names(&[value.to_owned()]).is_err(),
            "accepted {value:?}"
        );
    }
    assert!(module_names(&[]).is_err());
}

#[test]
fn versions_accept_three_numeric_components_and_reject_malformed_values() {
    assert_eq!(version("0.8.12", "test").unwrap(), [0, 8, 12]);
    for value in [
        "",
        "0.8",
        "0.8.1.2",
        "01.8.0",
        "0.x.0",
        "18446744073709551616.0.0",
    ] {
        assert!(version(value, "test").is_err(), "accepted {value:?}");
    }
}

#[test]
fn installed_version_file_requires_a_well_formed_cmake_version_value() {
    assert_eq!(
        parse_installed_version(
            "set(PACKAGE_VERSION \"0.8.0\")",
            Path::new("MayaFluxConfigVersion.cmake")
        )
        .unwrap(),
        ("0.8.0".to_owned(), [0, 8, 0])
    );
    for source in [
        "set(PACKAGE_VERSION 0.8.0)",
        "set(PACKAGE_VERSION \"0.8\")",
        "set(PACKAGE_VERSION nope)",
    ] {
        assert!(parse_installed_version(source, Path::new("MayaFluxConfigVersion.cmake")).is_err());
    }
}

#[test]
fn registry_parser_accepts_bom_and_rejects_bad_shape_duplicates_urls_and_versions() {
    let valid = "\u{FEFF} [ { \"name\": \"terrain_field\", \"repo\": \"https://example.invalid/terrain.git\", \"min_version\": \"0.5.0\" } ]";
    assert_eq!(
        parse_registry(valid).unwrap()["terrain_field"]
            .repository
            .as_str(),
        "https://example.invalid/terrain.git"
    );
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
    assert_eq!(
        request
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        vec![
            "--fail".to_owned(),
            "--location".to_owned(),
            "--silent".to_owned(),
            "--show-error".to_owned(),
            "--max-time".to_owned(),
            "30".to_owned(),
            REGISTRY_URL.to_owned(),
        ]
    );

    let clone = clone_request(
        "https://example.invalid/module.git",
        Path::new("/tmp/module"),
    );
    assert_eq!(clone.get_program(), "git");
    assert_eq!(
        clone
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
        vec![
            "clone".to_owned(),
            "--progress".to_owned(),
            "--depth=1".to_owned(),
            "https://example.invalid/module.git".to_owned(),
            "/tmp/module".to_owned(),
        ]
    );
}

#[test]
fn invalid_project_is_rejected_before_registry_access() {
    let fixture = ProjectFixture::new(&["terrain_field"]);
    fs::remove_file(fixture.directory.0.join("CMakeLists.txt")).unwrap();
    let error = fixture
        .run(&["terrain_field"], &mut Vec::new())
        .unwrap_err();
    assert!(error.to_string().contains("Not a MayaFlux project"));
    assert_eq!(fixture.services.registry_calls.get(), 0);
    assert!(fixture.services.clone_requests.borrow().is_empty());
}

#[test]
fn registry_and_installed_version_failures_leave_project_untouched() {
    let mut fixture = ProjectFixture::new(&["terrain_field"]);
    fixture.services.registry_error.set(true);
    let error = fixture
        .run(&["terrain_field"], &mut Vec::new())
        .unwrap_err();
    assert!(error.to_string().contains("fixture registry unavailable"));
    assert!(!fixture.directory.0.join("community").exists());

    fixture.services.registry_error.set(false);
    fixture.services.installed = Err("malformed MayaFlux version file".to_owned());
    let error = fixture
        .run(&["terrain_field"], &mut Vec::new())
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("malformed MayaFlux version file")
    );
    assert!(!fixture.directory.0.join("community").exists());
    assert!(fixture.services.clone_requests.borrow().is_empty());
}

#[test]
fn unknown_module_is_rejected_before_any_clone_or_registration() {
    let fixture = ProjectFixture::new(&["terrain_field"]);
    let error = fixture
        .run(&["terrain_field", "unknown_module"], &mut Vec::new())
        .unwrap_err();
    assert!(error.to_string().contains("not in the community registry"));
    assert!(fixture.services.clone_requests.borrow().is_empty());
    assert!(!fixture.directory.0.join("community.cmake").exists());
}

#[test]
fn addition_clones_validates_registers_and_preserves_existing_project_content() {
    let fixture = ProjectFixture::new(&["terrain_field", "noise"]);
    fixture.module(
        "terrain_field",
        "{\"name\":\"terrain_field\",\"min_version\":\"0.5.0\"}",
    );
    fs::write(
        fixture.directory.0.join("community.cmake"),
        "# user entry\r\nterrain_field",
    )
    .unwrap();
    let mut output = Vec::new();

    fixture
        .run(&["terrain_field,noise", "noise"], &mut output)
        .unwrap();

    assert_eq!(
        fs::read_to_string(fixture.directory.0.join("community.cmake")).unwrap(),
        "# user entry\r\nterrain_field\nnoise\n"
    );
    assert_eq!(
        fs::read_to_string(fixture.directory.0.join("src.cpp")).unwrap(),
        "user content"
    );
    assert_eq!(
        fs::read_to_string(
            fixture
                .directory
                .0
                .join("community/terrain_field/notes.txt")
        )
        .unwrap(),
        "preserve me"
    );
    assert_eq!(
        fixture.services.clone_requests.borrow().as_slice(),
        &[(
            "https://example.invalid/noise.git".to_owned(),
            fs::canonicalize(&fixture.directory.0)
                .unwrap()
                .join("community/noise")
        )]
    );
    assert!(output.iter().any(|line| line.contains("Validated noise")));

    fixture.run(&["noise"], &mut output).unwrap();
    assert_eq!(
        fs::read_to_string(fixture.directory.0.join("community.cmake")).unwrap(),
        "# user entry\r\nterrain_field\nnoise\n"
    );
    assert!(
        output
            .iter()
            .any(|line| line.contains("noise was already registered"))
    );
}

#[test]
fn utf8_bom_manifest_is_accepted_and_declared_requirements_are_reported() {
    let fixture = ProjectFixture::new(&["terrain_field"]);
    fixture.module("terrain_field", "\u{FEFF}{\"schema_version\":1,\"name\":\"terrain_field\",\"min_version\":\"0.5.0\",\"packages\":{\"macos\":{\"brew\":{\"formulae\":[\"example\"]}}}}");
    let mut output = Vec::new();
    fixture.run(&["terrain_field"], &mut output).unwrap();
    assert!(
        output
            .iter()
            .any(|line| line.contains("declares requirements"))
    );
    assert_eq!(
        fs::read_to_string(fixture.directory.0.join("community.cmake")).unwrap(),
        "terrain_field\n"
    );
}

#[test]
fn all_urls_open_in_browser_and_destination_remains_pending() {
    let fixture = ProjectFixture::new(&["terrain_field"]);
    let module = fixture.module("terrain_field", r#"{"schema_version":1,"name":"terrain_field","min_version":"0.6.0","urls":[{"url":"https://example.org/data.bin","destination":"assets/data.bin","destination_root":"module"},{"url":"https://example.org/account/download"}]}"#);
    // The fixture spelling need not match the resolved path used in diagnostics.
    let module = module.join(".");
    let mut output = Vec::new();
    fixture.run(&["terrain_field"], &mut output).unwrap();
    assert!(!module.join("assets/data.bin").exists());
    assert_eq!(
        fixture.services.opened_urls.borrow().as_slice(),
        &[
            "https://example.org/data.bin",
            "https://example.org/account/download"
        ]
    );
    let destination = fs::canonicalize(&module)
        .unwrap()
        .join("assets")
        .join("data.bin")
        .display()
        .to_string();
    assert!(
        output
            .iter()
            .any(|line| line.contains(&destination) && line.contains("pending")),
        "Expected pending destination {destination}; output: {output:?}"
    );
}

#[test]
fn existing_url_destination_is_preserved_without_browser_handoff() {
    let fixture = ProjectFixture::new(&["terrain_field"]);
    let module = fixture.module("terrain_field", r#"{"schema_version":1,"name":"terrain_field","min_version":"0.6.0","urls":[{"url":"https://example.org/data.bin","destination":"assets/data.bin","destination_root":"module"}]}"#);
    fs::create_dir_all(module.join("assets")).unwrap();
    fs::write(module.join("assets/data.bin"), b"existing").unwrap();
    fixture.run(&["terrain_field"], &mut Vec::new()).unwrap();
    assert_eq!(
        fs::read(module.join("assets/data.bin")).unwrap(),
        b"existing"
    );
    assert!(fixture.services.opened_urls.borrow().is_empty());
}

#[test]
fn browser_failure_reports_manual_url_and_keeps_registration() {
    let fixture = ProjectFixture::new(&["terrain_field"]);
    fixture.module("terrain_field", r#"{"schema_version":1,"name":"terrain_field","min_version":"0.6.0","urls":[{"url":"https://example.org/data","destination":"data.bin","destination_root":"module"}]}"#);
    fixture.services.open_error.set(true);
    let mut output = Vec::new();
    fixture.run(&["terrain_field"], &mut output).unwrap();
    assert!(
        output
            .iter()
            .any(|line| line.contains("https://example.org/data")
                && line.contains("pending")
                && line.contains("fixture browser failed"))
    );
    assert_eq!(
        fs::read_to_string(fixture.directory.0.join("community.cmake")).unwrap(),
        "terrain_field\n"
    );
}

#[test]
fn declining_browser_handoff_leaves_destination_pending() {
    let fixture = ProjectFixture::new(&["terrain_field"]);
    let module = fixture.module("terrain_field", r#"{"schema_version":1,"name":"terrain_field","min_version":"0.6.0","urls":[{"url":"https://example.org/data","destination":"data.bin","destination_root":"module"}]}"#);
    fixture.services.allow_browser.set(false);
    let mut output = Vec::new();
    fixture.run(&["terrain_field"], &mut output).unwrap();
    assert!(fixture.services.opened_urls.borrow().is_empty());
    assert!(!module.join("data.bin").exists());
    assert!(
        output
            .iter()
            .any(|line| line.contains("https://example.org/data") && line.contains("pending"))
    );
}

#[test]
fn live_url_boundaries_reject_invalid_targets_before_open() {
    let services = LiveModuleServices {
        confirm_url: &|_| true,
    };
    for url in [
        "file:///tmp/secret",
        "http://example.org/file",
        "https://user@example.org/file",
        "https://example.org/with\\backslash",
        "https://example.org/%xx",
    ] {
        assert!(services.open_url(url).is_err(), "opened {url}");
    }
    assert!(webbrowser::open("file:///tmp/secret").is_err());
}

#[cfg(target_os = "linux")]
#[test]
fn browser_handoff_does_not_wait_for_launcher_process() {
    let mut command = Command::new("sleep");
    command.arg("2");
    let started = std::time::Instant::now();
    spawn_detached_browser_command(&mut command).unwrap();
    assert!(started.elapsed() < std::time::Duration::from_secs(1));
}

#[cfg(unix)]
#[test]
fn linked_url_destination_is_rejected_without_touching_target() {
    use std::os::unix::fs::symlink;
    let fixture = ProjectFixture::new(&["terrain_field"]);
    let module = fixture.module("terrain_field", r#"{"schema_version":1,"name":"terrain_field","min_version":"0.6.0","urls":[{"url":"https://example.org/data","destination":"data.bin","destination_root":"module"}]}"#);
    let target = fixture.directory.0.join("target.bin");
    fs::write(&target, b"original").unwrap();
    symlink(&target, module.join("data.bin")).unwrap();
    assert!(fixture.run(&["terrain_field"], &mut Vec::new()).is_err());
    assert_eq!(fs::read(&target).unwrap(), b"original");
    assert!(fixture.services.opened_urls.borrow().is_empty());
}

#[test]
fn module_validation_and_registry_minimum_fail_without_changing_registration() {
    let mut fixture = ProjectFixture::new(&["terrain_field"]);
    fixture.module(
        "terrain_field",
        "{\"name\":\"other\",\"min_version\":\"0.5.0\"}",
    );
    fs::write(
        fixture.directory.0.join("community.cmake"),
        "existing_entry\n",
    )
    .unwrap();
    let error = fixture
        .run(&["terrain_field"], &mut Vec::new())
        .unwrap_err();
    assert!(error.to_string().contains("name does not match"));
    assert_eq!(
        fs::read_to_string(fixture.directory.0.join("community.cmake")).unwrap(),
        "existing_entry\n"
    );

    fixture
        .services
        .entries
        .get_mut("terrain_field")
        .unwrap()
        .minimum = "0.10.0".to_owned();
    fixture.module(
        "terrain_field",
        "{\"name\":\"terrain_field\",\"min_version\":\"0.5.0\"}",
    );
    let error = fixture
        .run(&["terrain_field"], &mut Vec::new())
        .unwrap_err();
    assert!(error.to_string().contains("requires MayaFlux >= 0.10.0"));
    assert_eq!(
        fs::read_to_string(fixture.directory.0.join("community.cmake")).unwrap(),
        "existing_entry\n"
    );
}

#[test]
fn malformed_and_incomplete_existing_modules_preserve_registration_and_user_files() {
    for kind in [
        "missing-cmake",
        "missing-src",
        "missing-manifest",
        "bad-json",
        "duplicate-key",
        "wrong-name",
        "bad-version",
        "unknown-field",
    ] {
        let fixture = ProjectFixture::new(&["terrain_field"]);
        let module = fixture.module(
            "terrain_field",
            "{\"name\":\"terrain_field\",\"min_version\":\"0.5.0\"}",
        );
        match kind {
            "missing-cmake" => fs::remove_file(module.join("terrain_field.cmake")).unwrap(),
            "missing-src" => fs::remove_dir(module.join("src")).unwrap(),
            "missing-manifest" => fs::remove_file(module.join("community.json")).unwrap(),
            "bad-json" => fs::write(module.join("community.json"), "{bad").unwrap(),
            "duplicate-key" => fs::write(
                module.join("community.json"),
                "{\"name\":\"terrain_field\",\"name\":\"terrain_field\",\"min_version\":\"0.5.0\"}",
            )
            .unwrap(),
            "wrong-name" => fs::write(
                module.join("community.json"),
                "{\"name\":\"other\",\"min_version\":\"0.5.0\"}",
            )
            .unwrap(),
            "bad-version" => fs::write(
                module.join("community.json"),
                "{\"name\":\"terrain_field\",\"min_version\":\"0.5\"}",
            )
            .unwrap(),
            "unknown-field" => fs::write(
                module.join("community.json"),
                "{\"name\":\"terrain_field\",\"min_version\":\"0.5.0\",\"unexpected\":true}",
            )
            .unwrap(),
            _ => unreachable!(),
        }
        let registration = fixture.directory.0.join("community.cmake");
        fs::write(&registration, "user_entry\n").unwrap();
        let error = fixture
            .run(&["terrain_field"], &mut Vec::new())
            .unwrap_err();
        assert!(!error.to_string().is_empty(), "{kind}");
        assert_eq!(
            fs::read_to_string(&registration).unwrap(),
            "user_entry\n",
            "{kind}"
        );
        assert_eq!(
            fs::read_to_string(module.join("notes.txt")).unwrap(),
            "preserve me",
            "{kind}"
        );
    }
}

#[test]
fn manifest_minimum_version_is_enforced_independently_of_registry_minimum() {
    let fixture = ProjectFixture::new(&["terrain_field"]);
    fixture.module(
        "terrain_field",
        "{\"name\":\"terrain_field\",\"min_version\":\"2.0.0\"}",
    );
    let error = fixture
        .run(&["terrain_field"], &mut Vec::new())
        .unwrap_err();
    assert!(error.to_string().contains("requires MayaFlux >= 2.0.0"));
    assert!(!fixture.directory.0.join("community.cmake").exists());
}

#[test]
fn incompatible_registry_minimum_stops_before_creating_or_cloning_modules() {
    let mut fixture = ProjectFixture::new(&["terrain_field"]);
    fixture
        .services
        .entries
        .get_mut("terrain_field")
        .unwrap()
        .minimum = "0.10.0".to_owned();

    let error = fixture
        .run(&["terrain_field"], &mut Vec::new())
        .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("requires MayaFlux >= 0.10.0; found 0.8.0")
    );
    assert!(fixture.services.clone_requests.borrow().is_empty());
    assert!(!fixture.directory.0.join("community").exists());
    assert!(!fixture.directory.0.join("community.cmake").exists());
}

#[test]
fn failed_partial_clone_is_removed_and_a_later_retry_succeeds() {
    let fixture = ProjectFixture::new(&["terrain_field"]);
    fixture.services.clone_error.set(true);
    fixture.services.partial_clone.set(true);
    let error = fixture
        .run(&["terrain_field"], &mut Vec::new())
        .unwrap_err();
    assert!(error.to_string().contains("fixture clone failed"));
    assert!(!fixture.directory.0.join("community/terrain_field").exists());
    assert!(!fixture.directory.0.join("community.cmake").exists());

    fixture.services.clone_error.set(false);
    fixture.services.partial_clone.set(false);
    fixture.run(&["terrain_field"], &mut Vec::new()).unwrap();
    assert_eq!(
        fs::read_to_string(fixture.directory.0.join("community.cmake")).unwrap(),
        "terrain_field\n"
    );
}

#[test]
fn validation_failure_rolls_back_new_clones_but_keeps_preexisting_modules() {
    let fixture = ProjectFixture::new(&["terrain_field", "noise"]);
    let existing = fixture.module("noise", "{\"name\":\"noise\",\"min_version\":\"0.5.0\"}");
    fixture
        .services
        .invalid_clone
        .replace(Some("terrain_field".to_owned()));
    fs::write(fixture.directory.0.join("community.cmake"), "noise\n").unwrap();

    let error = fixture
        .run(&["noise", "terrain_field"], &mut Vec::new())
        .unwrap_err();

    assert!(error.to_string().contains("Invalid manifest"));
    assert!(!fixture.directory.0.join("community/terrain_field").exists());
    assert!(existing.join("notes.txt").is_file());
    assert_eq!(
        fs::read_to_string(fixture.directory.0.join("community.cmake")).unwrap(),
        "noise\n"
    );
}

#[test]
fn registration_read_failure_rolls_back_only_modules_cloned_by_this_addition() {
    let fixture = ProjectFixture::new(&["terrain_field", "noise"]);
    let existing = fixture.module("noise", "{\"name\":\"noise\",\"min_version\":\"0.5.0\"}");
    let registration = fixture.directory.0.join("community.cmake");
    fs::create_dir(&registration).unwrap();
    fs::write(registration.join("user-file"), "preserve").unwrap();

    let error = fixture
        .run(&["noise", "terrain_field"], &mut Vec::new())
        .unwrap_err();

    assert!(error.to_string().contains("Could not read community.cmake"));
    assert!(!fixture.directory.0.join("community/terrain_field").exists());
    assert!(existing.join("notes.txt").is_file());
    assert_eq!(
        fs::read_to_string(registration.join("user-file")).unwrap(),
        "preserve"
    );
}

#[cfg(unix)]
#[test]
fn linked_project_community_and_registration_paths_are_rejected() {
    use std::os::unix::fs::symlink;

    let fixture = ProjectFixture::new(&["terrain_field"]);
    let target = fixture.directory.0.join("outside");
    fs::create_dir(&target).unwrap();
    symlink(&target, fixture.directory.0.join("community")).unwrap();
    let error = fixture
        .run(&["terrain_field"], &mut Vec::new())
        .unwrap_err();
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
    let error = add_to_project_with(
        &project_link,
        &["terrain_field".to_owned()],
        &fixture.services,
        |_| {},
    )
    .unwrap_err();
    assert!(error.to_string().contains("symbolic link"));
    fs::remove_file(project_link).unwrap();

    let outside = fixture.directory.0.join("outside");
    fs::create_dir(&outside).unwrap();
    fs::create_dir_all(fixture.directory.0.join("community")).unwrap();
    symlink(
        &outside,
        fixture.directory.0.join("community/terrain_field"),
    )
    .unwrap();
    let error = fixture
        .run(&["terrain_field"], &mut Vec::new())
        .unwrap_err();
    assert!(error.to_string().contains("symbolic link"));
    assert!(fs::read_dir(&outside).unwrap().next().is_none());
    fs::remove_file(fixture.directory.0.join("community/terrain_field")).unwrap();
    fs::remove_dir(outside).unwrap();
}

#[cfg(unix)]
#[test]
fn linked_project_ancestor_is_rejected_before_registry_access() {
    use std::os::unix::fs::symlink;

    let fixture = ProjectFixture::new(&["terrain_field"]);
    let linked_parent = fixture.directory.0.join("linked-parent");
    let project_name = fixture.directory.0.file_name().unwrap();
    symlink(fixture.directory.0.parent().unwrap(), &linked_parent).unwrap();
    let linked_project = linked_parent.join(project_name);

    let error = add_to_project_with(
        &linked_project,
        &["terrain_field".to_owned()],
        &fixture.services,
        |_| {},
    )
    .unwrap_err();

    assert!(error.to_string().contains("reparse point"));
    assert_eq!(fixture.services.registry_calls.get(), 0);
    assert!(fixture.services.clone_requests.borrow().is_empty());
    assert!(!fixture.directory.0.join("community.cmake").exists());
}

#[cfg(unix)]
#[test]
fn linked_module_manifest_sources_and_cmake_files_are_rejected_without_writes() {
    use std::os::unix::fs::symlink;

    for kind in ["cmake", "src", "manifest"] {
        let fixture = ProjectFixture::new(&["terrain_field"]);
        let module = fixture.module(
            "terrain_field",
            "{\"name\":\"terrain_field\",\"min_version\":\"0.5.0\"}",
        );
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
                fs::write(
                    &target,
                    "{\"name\":\"terrain_field\",\"min_version\":\"0.5.0\"}",
                )
                .unwrap();
                let path = module.join("community.json");
                fs::remove_file(&path).unwrap();
                path
            }
        };
        symlink(&target, &linked).unwrap();
        fs::write(fixture.directory.0.join("community.cmake"), "keep_entry\n").unwrap();

        let error = fixture
            .run(&["terrain_field"], &mut Vec::new())
            .unwrap_err();

        assert!(
            error.to_string().contains("symbolic link"),
            "{kind}: {error}"
        );
        assert_eq!(
            fs::read_to_string(fixture.directory.0.join("community.cmake")).unwrap(),
            "keep_entry\n"
        );
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

    let error = fixture
        .run(&["terrain_field"], &mut Vec::new())
        .unwrap_err();

    assert!(error.to_string().contains("symbolic link"));
    assert_eq!(fs::read_to_string(target).unwrap(), "keep_entry\n");
    assert!(fixture.services.clone_requests.borrow().is_empty());
}
