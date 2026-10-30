use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

#[test]
fn standalone_binary_exposes_its_license_and_package_metadata() {
    let temporary = TestDirectory::new();
    let license = Command::new(env!("CARGO_BIN_EXE_weave"))
        .arg("--license")
        .current_dir(&temporary.0)
        .output()
        .unwrap();
    assert!(license.status.success());
    assert_eq!(license.stdout, include_bytes!("../LICENSE.txt"));
    let info = Command::new(env!("CARGO_BIN_EXE_weave"))
        .arg("--package-info")
        .current_dir(&temporary.0)
        .output()
        .unwrap();
    assert!(info.status.success());
    let metadata: serde_json::Value = serde_json::from_slice(&info.stdout).unwrap();
    assert_eq!(metadata["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(metadata["resources"], "embedded");
    #[cfg(target_os = "windows")]
    {
        assert_eq!(metadata["architecture"], "x64");
        assert_eq!(metadata["minimumWindows"], "10.0.19045");
        assert_eq!(
            metadata["runtime"],
            if cfg!(target_feature = "crt-static") {
                "static-msvc"
            } else {
                "dynamic-msvc"
            }
        );
    }
}

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

fn create_manage_project(root: &Path) -> PathBuf {
    let project = root.join("Project with spaces");
    fs::create_dir_all(project.join("cmake")).unwrap();
    fs::create_dir_all(project.join("data/shaders")).unwrap();
    fs::write(project.join("CMakeLists.txt"), "project(ManageFixture)\n").unwrap();
    let cmake = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/resources/templates/project/cmake/mayaflux.cmake"
    ))
    .replace("@LILA_LINK_BLOCK@", "")
    .replace("@LILA_DEBUGGER_PATH@", "")
    .replace("@LILA_DLL_COPY@", "");
    fs::write(project.join("cmake/mayaflux.cmake"), cmake).unwrap();
    project
}

#[cfg(target_os = "linux")]
fn write_fake_tool(bin: &Path, name: &str, script: &str) {
    use std::os::unix::fs::PermissionsExt;
    let path = bin.join(name);
    fs::write(&path, script).unwrap();
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
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
fn cli_help_exposes_project_and_install_operations() {
    let result = Command::new(env!("CARGO_BIN_EXE_weave"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(result.status.success());
    let help = String::from_utf8_lossy(&result.stdout);
    assert!(help.contains("weave new"));
    assert!(help.contains("weave manage"));
    assert!(!help.contains("weave update"));
    assert!(help.contains("--with-lila             Add Lila live-coding integration"));
    assert!(help.contains("--replace-shader-conflicts"));
    assert!(help.contains("weave community"));
    assert!(help.contains("weave install <stable|development>"));
}

#[test]
fn cli_rejects_invalid_install_channel_without_starting_an_install() {
    let result = Command::new(env!("CARGO_BIN_EXE_weave"))
        .args(["install", "nightly"])
        .output()
        .unwrap();
    assert!(!result.status.success());
    let error = String::from_utf8_lossy(&result.stderr);
    #[cfg(target_os = "macos")]
    assert!(error.contains("Unsupported MayaFlux install channel: nightly"));
    #[cfg(target_os = "linux")]
    assert!(error.contains("Unsupported MayaFlux install channel: nightly"));
    #[cfg(target_os = "windows")]
    assert!(error.contains("Unsupported MayaFlux install channel: nightly"));
}

#[cfg(target_os = "linux")]
#[test]
fn cli_installs_on_ubuntu_with_fake_package_tools_only() {
    use std::os::unix::fs::PermissionsExt;
    let temporary = TestDirectory::new();
    let bin = temporary.0.join("bin");
    fs::create_dir(&bin).unwrap();
    let log = temporary.0.join("commands.log");
    let state = temporary.0.join("installed");
    for (name, script) in [
        ("sudo", "#!/bin/sh\nexec \"$@\"\n"),
        (
            "apt-get",
            "#!/bin/sh\nprintf '%s\\n' \"apt-get $*\" >> \"$WEAVE_TEST_LOG\"\nif [ \"$1\" = install ]; then printf installed > \"$WEAVE_TEST_STATE\"; fi\n",
        ),
        (
            "add-apt-repository",
            "#!/bin/sh\nprintf '%s\\n' \"add-apt-repository $*\" >> \"$WEAVE_TEST_LOG\"\n",
        ),
        (
            "dpkg-query",
            "#!/bin/sh\nif [ \"$3\" = mayaflux ] && [ -f \"$WEAVE_TEST_STATE\" ]; then printf 'install ok installed'; else exit 1; fi\n",
        ),
    ] {
        let path = bin.join(name);
        fs::write(&path, script).unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }
    let result = Command::new(env!("CARGO_BIN_EXE_weave"))
        .args(["install", "stable"])
        .env("PATH", &bin)
        .env("WEAVE_TEST_LOG", &log)
        .env("WEAVE_TEST_STATE", &state)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("mayaflux installed and verified"));
    assert_eq!(
        fs::read_to_string(log).unwrap(),
        "add-apt-repository -y ppa:mayaflux/mayaflux\napt-get update\napt-get install -y mayaflux\n"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn cli_fedora_requires_conflict_approval_then_removes_and_installs() {
    let temporary = TestDirectory::new();
    let bin = temporary.0.join("bin");
    fs::create_dir(&bin).unwrap();
    let log = temporary.0.join("commands.log");
    let state = temporary.0.join("installed");
    fs::write(&state, "mayaflux-dev").unwrap();
    write_fake_tool(&bin, "sudo", "#!/bin/sh\nexec \"$@\"\n");
    write_fake_tool(
        &bin,
        "rpm",
        "#!/bin/sh\nread -r installed < \"$WEAVE_TEST_STATE\"\n[ \"$1\" = -q ] && [ \"$2\" = \"$installed\" ]\n",
    );
    write_fake_tool(
        &bin,
        "dnf",
        "#!/bin/sh\nprintf 'dnf %s\\n' \"$*\" >> \"$WEAVE_TEST_LOG\"\nif [ \"$1\" = \"$WEAVE_FAIL\" ]; then exit 9; fi\nif [ \"$1\" = remove ]; then : > \"$WEAVE_TEST_STATE\"; fi\nif [ \"$1\" = install ]; then printf '%s' \"$3\" > \"$WEAVE_TEST_STATE\"; fi\n",
    );
    let run = |replace: bool, fail_step: &str| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_weave"));
        command
            .args(["install", "stable"])
            .env("PATH", &bin)
            .env("WEAVE_TEST_LOG", &log)
            .env("WEAVE_TEST_STATE", &state)
            .env("WEAVE_FAIL", fail_step);
        if replace {
            command.arg("--replace-conflict");
        }
        command.output().unwrap()
    };
    let refused = run(false, "");
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("mayaflux-dev"));
    assert_eq!(fs::read_to_string(&state).unwrap(), "mayaflux-dev");
    assert!(!log.exists());

    let failed = run(true, "copr");
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("restore"));
    assert_eq!(fs::read_to_string(&state).unwrap(), "");

    let approved = run(true, "");
    assert!(
        approved.status.success(),
        "{}",
        String::from_utf8_lossy(&approved.stderr)
    );
    assert_eq!(fs::read_to_string(&state).unwrap(), "mayaflux");
    assert_eq!(
        fs::read_to_string(&log).unwrap(),
        "dnf remove -y mayaflux-dev\ndnf copr enable -y ranjithshegde/spirv-cross\ndnf copr enable -y ranjithshegde/spirv-cross\ndnf copr enable -y ranjithshegde/asio-standalone\ndnf copr enable -y ranjithshegde/mayaflux\ndnf install -y mayaflux\n"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn cli_arch_uses_fresh_checkout_and_cleans_it_after_install() {
    let temporary = TestDirectory::new();
    let bin = temporary.0.join("bin");
    fs::create_dir(&bin).unwrap();
    let log = temporary.0.join("commands.log");
    let state = temporary.0.join("installed");
    fs::write(&state, "mayaflux-dev-bin").unwrap();
    write_fake_tool(&bin, "sudo", "#!/bin/sh\nexec \"$@\"\n");
    write_fake_tool(
        &bin,
        "pacman",
        "#!/bin/sh\nread -r installed < \"$WEAVE_TEST_STATE\"\nif [ \"$1\" = -Qq ]; then [ \"$2\" = \"$installed\" ] && printf '%s\\n' \"$2\"; exit; fi\nprintf 'pacman %s\\n' \"$*\" >> \"$WEAVE_TEST_LOG\"\n: > \"$WEAVE_TEST_STATE\"\n",
    );
    write_fake_tool(
        &bin,
        "git",
        "#!/bin/sh\nprintf 'git %s\\n' \"$*\" >> \"$WEAVE_TEST_LOG\"\n[ -d \"$3\" ]\n",
    );
    write_fake_tool(
        &bin,
        "makepkg",
        "#!/bin/sh\nprintf 'makepkg %s\\n' \"$*\" >> \"$WEAVE_TEST_LOG\"\n[ -d \"$2\" ] || exit 3\nprintf mayaflux-bin > \"$WEAVE_TEST_STATE\"\n",
    );
    let result = Command::new(env!("CARGO_BIN_EXE_weave"))
        .args(["install", "stable", "--replace-conflict"])
        .env("PATH", &bin)
        .env("WEAVE_TEST_LOG", &log)
        .env("WEAVE_TEST_STATE", &state)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(fs::read_to_string(&state).unwrap(), "mayaflux-bin");
    let commands = fs::read_to_string(&log).unwrap();
    let lines: Vec<_> = commands.lines().collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "pacman -R --noconfirm mayaflux-dev-bin");
    assert!(lines[1].starts_with("git clone https://aur.archlinux.org/mayaflux-bin.git "));
    let checkout = Path::new(lines[1].split_whitespace().last().unwrap());
    assert!(lines[2].starts_with(&format!(
        "makepkg --dir {} -si --noconfirm",
        checkout.display()
    )));
    assert!(!checkout.exists());
}

#[cfg(target_os = "linux")]
#[test]
fn cli_reports_missing_host_tool_before_package_query_or_change() {
    let temporary = TestDirectory::new();
    let bin = temporary.0.join("bin");
    fs::create_dir(&bin).unwrap();
    let log = temporary.0.join("commands.log");
    for name in ["sudo", "apt-get", "dpkg-query"] {
        write_fake_tool(
            &bin,
            name,
            "#!/bin/sh\nprintf 'called\\n' >> \"$WEAVE_TEST_LOG\"\n",
        );
    }
    let result = Command::new(env!("CARGO_BIN_EXE_weave"))
        .args(["install", "stable", "--replace-conflict"])
        .env("PATH", &bin)
        .env("WEAVE_TEST_LOG", &log)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("add-apt-repository"));
    assert!(!log.exists());
}

#[test]
fn cli_reports_project_manage_validation_errors_without_network_access() {
    let temporary = TestDirectory::new();
    let old_command = Command::new(env!("CARGO_BIN_EXE_weave"))
        .arg("update")
        .output()
        .unwrap();
    assert!(!old_command.status.success());
    assert!(String::from_utf8_lossy(&old_command.stderr).contains("Unknown command: update"));

    let missing = Command::new(env!("CARGO_BIN_EXE_weave"))
        .arg("manage")
        .output()
        .unwrap();
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("Usage: weave manage"));

    let unsafe_name = Command::new(env!("CARGO_BIN_EXE_weave"))
        .args(["manage", temporary.0.to_str().unwrap(), "../escape"])
        .output()
        .unwrap();
    assert!(!unsafe_name.status.success());
    assert!(String::from_utf8_lossy(&unsafe_name.stderr).contains("Invalid module name"));
    assert_eq!(fs::read_dir(&temporary.0).unwrap().count(), 0);

    let invalid_project = Command::new(env!("CARGO_BIN_EXE_weave"))
        .args(["manage", temporary.0.to_str().unwrap(), "terrain_field"])
        .output()
        .unwrap();
    assert!(!invalid_project.status.success());
    assert!(String::from_utf8_lossy(&invalid_project.stderr).contains("Not a MayaFlux project"));
    assert_eq!(fs::read_dir(&temporary.0).unwrap().count(), 0);
}

#[test]
fn cli_manage_adds_lila_idempotently_to_project_paths_with_spaces() {
    let temporary = TestDirectory::new();
    let project = create_manage_project(&temporary.0);
    for _ in 0..2 {
        let result = Command::new(env!("CARGO_BIN_EXE_weave"))
            .args(["manage", project.to_str().unwrap(), "--with-lila"])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let cmake = fs::read_to_string(project.join("cmake/mayaflux.cmake")).unwrap();
    assert_eq!(cmake.matches("MayaFlux::MayaFluxHost)").count(), 1);
    assert!(cmake.contains("$<TARGET_FILE_DIR:MayaFlux::MayaFluxHost>"));
    assert!(cmake.contains("MayaFluxHost.dll"));
}

#[test]
fn cli_manage_copies_shader_sources_from_a_controlled_installation() {
    let temporary = TestDirectory::new();
    let project = create_manage_project(&temporary.0);
    let install = temporary
        .0
        .join("fake MayaFlux")
        .join("share/MayaFlux/src/shaders");
    fs::create_dir_all(install.join("nested")).unwrap();
    fs::write(install.join("nested/effect.frag"), "installed shader").unwrap();
    fs::write(project.join("data/shaders/custom.frag"), "user shader").unwrap();

    let result = Command::new(env!("CARGO_BIN_EXE_weave"))
        .args(["manage", project.to_str().unwrap(), "--copy-shader-sources"])
        .env("MAYAFLUX_ROOT", temporary.0.join("fake MayaFlux"))
        .output()
        .unwrap();

    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        fs::read(project.join("data/shaders/nested/effect.frag")).unwrap(),
        b"installed shader"
    );
    assert_eq!(
        fs::read(project.join("data/shaders/custom.frag")).unwrap(),
        b"user shader"
    );
}

#[test]
fn cli_manage_requires_explicit_shader_conflict_replacement() {
    let temporary = TestDirectory::new();
    let project = create_manage_project(&temporary.0);
    let install = temporary
        .0
        .join("fake MayaFlux")
        .join("share/MayaFlux/src/shaders");
    fs::create_dir_all(&install).unwrap();
    fs::write(install.join("effect.frag"), "installed shader").unwrap();
    let destination = project.join("data/shaders/effect.frag");
    fs::write(&destination, "user shader").unwrap();
    let install_root = temporary.0.join("fake MayaFlux");

    let refused = Command::new(env!("CARGO_BIN_EXE_weave"))
        .args(["manage", project.to_str().unwrap(), "--copy-shader-sources"])
        .env("MAYAFLUX_ROOT", &install_root)
        .output()
        .unwrap();
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("explicit replacement choice"));
    assert_eq!(fs::read(&destination).unwrap(), b"user shader");

    let replaced = Command::new(env!("CARGO_BIN_EXE_weave"))
        .args([
            "manage",
            project.to_str().unwrap(),
            "--copy-shader-sources",
            "--replace-shader-conflicts",
        ])
        .env("MAYAFLUX_ROOT", install_root)
        .output()
        .unwrap();
    assert!(
        replaced.status.success(),
        "{}",
        String::from_utf8_lossy(&replaced.stderr)
    );
    assert_eq!(fs::read(destination).unwrap(), b"installed shader");
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
