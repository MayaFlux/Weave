use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);

        loop {
            let sequence = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("weave-cli-test-{}-{sequence}", std::process::id()));
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
fn cli_creates_a_project_with_requested_options_and_platform_scripts() {
    let temporary = TestDirectory::new();
    let result = Command::new(env!("CARGO_BIN_EXE_weave"))
        .args([
            "new",
            "CliDemo",
            temporary.0.to_str().unwrap(),
            "--with-lila",
            "--no-vscode",
        ])
        .output()
        .unwrap();

    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let project = temporary.0.join("CliDemo");
    assert!(project.join("src/main.cpp").is_file());
    assert!(project.join("cmake/mayaflux.cmake").is_file());
    if Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
    {
        let git_root = Command::new("git")
            .args([
                "-C",
                project.to_str().unwrap(),
                "rev-parse",
                "--show-toplevel",
            ])
            .output()
            .unwrap();
        assert!(git_root.status.success());
        let staged = Command::new("git")
            .args([
                "-C",
                project.to_str().unwrap(),
                "diff",
                "--cached",
                "--name-only",
            ])
            .output()
            .unwrap();
        assert!(staged.status.success());
        assert!(!String::from_utf8_lossy(&staged.stdout).trim().is_empty());
    }
    assert!(!project.join(".vscode").exists());
    assert!(
        fs::read_to_string(project.join("cmake/mayaflux.cmake"))
            .unwrap()
            .contains("MayaFlux::MayaFluxHost")
    );

    #[cfg(target_os = "windows")]
    for script in [
        "validate.ps1",
        "run-report.ps1",
        "validate.bat",
        "run-report.bat",
    ] {
        assert!(project.join(script).is_file());
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    for script in ["validate.sh", "run-report.sh"] {
        assert!(project.join(script).is_file());
    }
}

#[test]
fn cli_rejects_unsafe_project_names_without_writing_files() {
    let temporary = TestDirectory::new();
    let result = Command::new(env!("CARGO_BIN_EXE_weave"))
        .args(["new", "../escape", temporary.0.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("Project name"));
    assert_eq!(fs::read_dir(&temporary.0).unwrap().count(), 0);
}

#[test]
fn cli_does_not_overwrite_an_existing_project() {
    let temporary = TestDirectory::new();
    let project = temporary.0.join("CliDemo");
    fs::create_dir(&project).unwrap();
    fs::write(project.join("keep.txt"), "preserve").unwrap();

    let result = Command::new(env!("CARGO_BIN_EXE_weave"))
        .args(["new", "CliDemo", temporary.0.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(!result.status.success());
    assert_eq!(
        fs::read_to_string(project.join("keep.txt")).unwrap(),
        "preserve"
    );
}

#[test]
fn cli_help_exposes_the_three_project_operations() {
    let result = Command::new(env!("CARGO_BIN_EXE_weave"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(result.status.success());
    let help = String::from_utf8_lossy(&result.stdout);
    assert!(help.contains("weave new"));
    assert!(help.contains("weave update"));
    assert!(help.contains("weave community"));
}

#[test]
fn cli_reports_project_update_validation_errors_without_network_access() {
    let temporary = TestDirectory::new();
    let missing = Command::new(env!("CARGO_BIN_EXE_weave"))
        .arg("update")
        .output()
        .unwrap();
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("Usage: weave update"));

    let unsafe_name = Command::new(env!("CARGO_BIN_EXE_weave"))
        .args(["update", temporary.0.to_str().unwrap(), "../escape"])
        .output()
        .unwrap();
    assert!(!unsafe_name.status.success());
    assert!(String::from_utf8_lossy(&unsafe_name.stderr).contains("Invalid module name"));
    assert_eq!(fs::read_dir(&temporary.0).unwrap().count(), 0);

    let invalid_project = Command::new(env!("CARGO_BIN_EXE_weave"))
        .args(["update", temporary.0.to_str().unwrap(), "terrain_field"])
        .output()
        .unwrap();
    assert!(!invalid_project.status.success());
    assert!(String::from_utf8_lossy(&invalid_project.stderr).contains("Not a MayaFlux project"));
    assert_eq!(fs::read_dir(&temporary.0).unwrap().count(), 0);
}

#[test]
fn cli_creates_a_community_module_from_embedded_templates() {
    let temporary = TestDirectory::new();
    let result = Command::new(env!("CARGO_BIN_EXE_weave"))
        .args([
            "community",
            "audio_tools",
            temporary.0.to_str().unwrap(),
            "--description",
            "Audio \"helpers\"",
            "--min-version",
            "0.6.0",
            "--with-lila",
        ])
        .output()
        .unwrap();

    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let module = temporary.0.join("audio_tools");
    assert!(module.join("src").is_dir());
    assert!(!module.join("src/audio_tools.cpp").exists());
    assert!(module.join("CMakeLists.txt").is_file());
    assert!(module.join("README.md").is_file());
    assert!(module.join("test/test_audio_tools.cpp").is_file());
    assert!(module.join("audio_tools.cmake").is_file());
    if Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
    {
        let git_root = Command::new("git")
            .args([
                "-C",
                module.to_str().unwrap(),
                "rev-parse",
                "--show-toplevel",
            ])
            .output()
            .unwrap();
        assert!(git_root.status.success());
        let staged = Command::new("git")
            .args([
                "-C",
                module.to_str().unwrap(),
                "diff",
                "--cached",
                "--name-only",
            ])
            .output()
            .unwrap();
        assert!(staged.status.success());
        assert!(!String::from_utf8_lossy(&staged.stdout).trim().is_empty());
    }
    let manifest_text = fs::read_to_string(module.join("community.json")).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&manifest_text).unwrap();
    assert_eq!(manifest["name"], "audio_tools");
    assert_eq!(manifest["min_version"], "0.6.0");
    assert_eq!(manifest["needs_lila"], true);
    assert_eq!(manifest["description"], "Audio \"helpers\"");
    let cmake = fs::read_to_string(module.join("audio_tools.cmake")).unwrap();
    assert!(cmake.contains("set(MF_NEEDS_LILA ON)"));
    assert!(cmake.contains("set(MF_MIN_VERSION \"0.6.0\")"));
    let test_cmake = fs::read_to_string(module.join("test/CMakeLists.txt")).unwrap();
    assert!(test_cmake.contains("set(MF_NEEDS_LILA ON)"));
    assert!(test_cmake.contains("set(MF_MIN_VERSION \"0.6.0\")"));
    assert!(
        fs::read_to_string(module.join("CMakeLists.txt"))
            .unwrap()
            .contains("add_subdirectory(test)")
    );
    assert!(
        fs::read_to_string(module.join("README.md"))
            .unwrap()
            .contains("Audio \"helpers\"")
    );
    assert!(!manifest_text.contains("@MODULE_NAME@"));

    let default_result = Command::new(env!("CARGO_BIN_EXE_weave"))
        .args(["community", "default_module", temporary.0.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        default_result.status.success(),
        "{}",
        String::from_utf8_lossy(&default_result.stderr)
    );
    let default_module = temporary.0.join("default_module");
    let default_manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(default_module.join("community.json")).unwrap()).unwrap();
    assert_eq!(default_manifest["min_version"], "0.5.0");
    assert_eq!(default_manifest["needs_lila"], false);
    assert!(
        fs::read_to_string(default_module.join("default_module.cmake"))
            .unwrap()
            .contains("set(MF_NEEDS_LILA OFF)")
    );
}

#[test]
fn cli_rejects_invalid_community_names_without_writing_files() {
    let temporary = TestDirectory::new();
    let result = Command::new(env!("CARGO_BIN_EXE_weave"))
        .args(["community", "Bad-Name", temporary.0.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("Module name"));
    assert_eq!(fs::read_dir(&temporary.0).unwrap().count(), 0);
}

#[test]
fn cli_rejects_invalid_community_minimum_versions_without_writing_files() {
    let temporary = TestDirectory::new();
    let result = Command::new(env!("CARGO_BIN_EXE_weave"))
        .args([
            "community",
            "audio_tools",
            temporary.0.to_str().unwrap(),
            "--min-version",
            "01.2.3",
        ])
        .output()
        .unwrap();

    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("major.minor.patch"));
    assert_eq!(fs::read_dir(&temporary.0).unwrap().count(), 0);
}

#[test]
fn cli_rejects_existing_community_module_without_overwriting_user_files() {
    let temporary = TestDirectory::new();
    let module = temporary.0.join("audio_tools");
    fs::create_dir(&module).unwrap();
    fs::write(module.join("keep.txt"), "user content").unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_weave"))
        .args(["community", "audio_tools", temporary.0.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("already exists"));
    assert_eq!(
        fs::read_to_string(module.join("keep.txt")).unwrap(),
        "user content"
    );
}

#[cfg(unix)]
#[test]
fn cli_rejects_symlinked_project_creation_destination() {
    use std::os::unix::fs::symlink;

    let temporary = TestDirectory::new();
    let target = temporary.0.join("target");
    let link = temporary.0.join("link");
    fs::create_dir(&target).unwrap();
    symlink(&target, &link).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_weave"))
        .args(["new", "Demo", link.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("symbolic link"));
    assert!(fs::read_dir(target).unwrap().next().is_none());
}
