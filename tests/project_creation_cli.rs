use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("weave-cli-test-{}-{unique}", std::process::id()));
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

    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    let project = temporary.0.join("CliDemo");
    assert!(project.join("src/main.cpp").is_file());
    assert!(project.join("cmake/mayaflux.cmake").is_file());
    assert!(!project.join(".vscode").exists());
    assert!(fs::read_to_string(project.join("cmake/mayaflux.cmake"))
        .unwrap()
        .contains("MayaFlux::MayaFluxHost"));

    #[cfg(target_os = "windows")]
    for script in ["validate.ps1", "run-report.ps1", "validate.bat", "run-report.bat"] {
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
    assert_eq!(fs::read_to_string(project.join("keep.txt")).unwrap(), "preserve");
}
