use crate::operations::install::progress::Stage;
use crate::operations::install::run_command_with_log;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub fn shader_source_root() -> Option<PathBuf> {
    for brew in brew_candidates() {
        for formula in ["mayaflux", "mayaflux-dev"] {
            if let Ok(output) = Command::new(&brew).args(["--prefix", formula]).output() {
                if output.status.success() {
                    if let Ok(prefix) = String::from_utf8(output.stdout) {
                        let path = PathBuf::from(prefix.trim()).join("share/MayaFlux/src/shaders");
                        if path.is_dir() {
                            return Some(path);
                        }
                    }
                }
            }
        }
    }
    None
}

fn brew_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path) = std::env::var_os("PATH") {
        candidates.extend(
            std::env::split_paths(&path)
                .map(|directory| directory.join("brew"))
                .filter(|candidate| !candidate.starts_with("/usr/local")),
        );
    }
    if let Ok(entries) = fs::read_dir("/opt") {
        candidates.extend(entries.flatten().map(|entry| entry.path().join("bin/brew")));
    }
    candidates.retain(|candidate| candidate.is_file());
    candidates
}

#[cfg(test)]
mod install_tests {
    use super::{
        formula_for_channel, install_formula_with_brew, resolve_zshenv_path, run_brew_with_log,
        update_shell_environment,
    };
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    #[test]
    fn maps_supported_channels_to_exact_mayaflux_formulas() {
        assert_eq!(formula_for_channel("stable").unwrap(), "mayaflux");
        assert_eq!(formula_for_channel("development").unwrap(), "mayaflux-dev");
    }

    #[test]
    fn developer_tool_check_only_requests_native_installation_when_missing() {
        use std::cell::RefCell;
        for installed in [true, false] {
            let calls = RefCell::new(Vec::new());
            let result = super::check_developer_tools_with(|args| {
                calls.borrow_mut().push(args.join(" "));
                Ok(args == ["--install"] || installed)
            });
            if installed {
                assert!(result.is_ok());
                assert_eq!(*calls.borrow(), ["-p"]);
            } else {
                assert!(result.unwrap_err().contains("retry"));
                assert_eq!(*calls.borrow(), ["-p", "--install"]);
            }
        }
        assert!(
            super::check_developer_tools_with(|_| Ok(false))
                .unwrap_err()
                .contains("Software Update")
        );
        assert!(
            super::check_developer_tools_with(|_| Err("cancelled".into()))
                .unwrap_err()
                .contains("cancelled")
        );
    }

    #[test]
    fn formula_install_reports_environment_separately_and_stops_on_shell_failure() {
        use crate::operations::install::progress::{Event, Stage};
        let (root, brew, _) = fake_brew(false, false);
        let events = std::sync::Mutex::new(Vec::new());
        let error = install_formula_with_brew(
            &brew,
            "mayaflux",
            false,
            &|line| events.lock().unwrap().push(Event::decode(line)),
            |_, _| Err("controlled shell failure".into()),
        )
        .unwrap_err();
        assert!(error.to_string().contains("installed and verified"));
        let stages: Vec<_> = events
            .into_inner()
            .unwrap()
            .into_iter()
            .filter_map(|event| match event {
                Event::Step(stage, _) => Some(stage),
                _ => None,
            })
            .collect();
        assert_eq!(stages, [Stage::Framework, Stage::Environment]);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn successful_homebrew_bootstrap_rediscovers_the_installed_executable() {
        let root = temp_dir();
        let brew = root.join("bin/brew");
        let mut discoveries = 0;
        let mut bootstraps = 0;
        let selected = super::ensure_brew_with(
            &|_| {},
            || {
                discoveries += 1;
                (discoveries > 1).then(|| brew.clone())
            },
            || {
                bootstraps += 1;
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(selected, brew);
        assert_eq!(discoveries, 2);
        assert_eq!(bootstraps, 1);
    }

    #[test]
    fn homebrew_bootstrap_cancellation_stops_before_discovery_retry() {
        let mut discoveries = 0;
        let mut bootstraps = 0;
        let error = super::ensure_brew_with(
            &|_| {},
            || {
                discoveries += 1;
                None
            },
            || {
                bootstraps += 1;
                Err("Installer authorization was cancelled".to_owned())
            },
        )
        .unwrap_err();

        assert!(error.contains("cancelled"));
        assert_eq!(discoveries, 1);
        assert_eq!(bootstraps, 1);
    }

    #[test]
    fn successful_installer_without_discoverable_brew_reports_recovery() {
        let mut discoveries = 0;
        let error = super::ensure_brew_with(
            &|_| {},
            || {
                discoveries += 1;
                None
            },
            || Ok(()),
        )
        .unwrap_err();

        assert!(error.contains("installer finished, but Homebrew was not found"));
        assert!(error.contains("retry"));
        assert_eq!(discoveries, 2);
    }

    #[test]
    fn cancelled_homebrew_installer_removes_package_and_stops_the_flow() {
        let root = temp_dir();
        fs::create_dir_all(&root).unwrap();
        let package = root.join("Homebrew.pkg");
        let mut verified = false;
        let mut opened = false;
        let error = super::bootstrap_homebrew_package(
            &package,
            &|_| {},
            |path| {
                fs::write(path, "controlled package").map_err(|error| error.to_string())?;
                Ok(())
            },
            |_| {
                verified = true;
                Ok(())
            },
            |_| {
                opened = true;
                Err("Installer authorization was cancelled".to_owned())
            },
        )
        .unwrap_err();

        assert!(error.contains("cancelled"));
        assert!(verified);
        assert!(opened);
        assert!(!package.exists());
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn successful_homebrew_bootstrap_verifies_opens_and_removes_package() {
        use std::cell::RefCell;

        let root = temp_dir();
        fs::create_dir_all(&root).unwrap();
        let package = root.join("Homebrew.pkg");
        let events = RefCell::new(Vec::new());
        super::bootstrap_homebrew_package(
            &package,
            &|_| {},
            |path| {
                events.borrow_mut().push("download");
                fs::write(path, "signed package").map_err(|error| error.to_string())
            },
            |path| {
                events.borrow_mut().push("verify");
                if fs::read_to_string(path).ok().as_deref() == Some("signed package") {
                    Ok(())
                } else {
                    Err("downloaded package was missing".to_owned())
                }
            },
            |path| {
                events.borrow_mut().push("authorize");
                if path.is_file() {
                    Ok(())
                } else {
                    Err("package was missing before authorization".to_owned())
                }
            },
        )
        .unwrap();

        assert_eq!(*events.borrow(), ["download", "verify", "authorize"]);
        assert!(!package.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_homebrew_download_cleans_partial_package_and_stops() {
        use std::cell::Cell;

        let root = temp_dir();
        fs::create_dir_all(&root).unwrap();
        let package = root.join("Homebrew.pkg");
        let later_stage_called = Cell::new(false);
        let error = super::bootstrap_homebrew_package(
            &package,
            &|_| {},
            |path| {
                fs::write(path, "partial").map_err(|error| error.to_string())?;
                Err("download failed".to_owned())
            },
            |_| {
                later_stage_called.set(true);
                Ok(())
            },
            |_| {
                later_stage_called.set(true);
                Ok(())
            },
        )
        .unwrap_err();

        assert!(error.contains("download failed"));
        assert!(!later_stage_called.get());
        assert!(!package.exists());
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn signature_failure_removes_package_without_opening_installer() {
        let root = temp_dir();
        fs::create_dir_all(&root).unwrap();
        let package = root.join("Homebrew.pkg");
        let mut opened = false;
        let error = super::bootstrap_homebrew_package(
            &package,
            &|_| {},
            |path| fs::write(path, "unsigned package").map_err(|error| error.to_string()),
            |_| Err("signature rejected".to_owned()),
            |_| {
                opened = true;
                Ok(())
            },
        )
        .unwrap_err();

        assert!(error.contains("signature rejected"));
        assert!(!opened);
        assert!(!package.exists());
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn uses_verified_zdotdir_before_home_fallback() {
        let root = temp_dir();
        let home = root.join("home");
        let zdot = root.join("dotfiles");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&zdot).unwrap();
        let home_file = home.join(".zshenv");
        let zdot_file = zdot.join(".zshenv");
        fs::write(&home_file, "export HOME_VALUE=preserve\n").unwrap();
        fs::write(&zdot_file, "export ZDOT_VALUE=preserve\n").unwrap();
        let selected = resolve_zshenv_path(&home, Some(zdot.to_str().unwrap()));
        assert_eq!(selected, zdot_file);
        update_shell_environment(&selected, "/opt/homebrew").unwrap();
        assert_eq!(
            fs::read_to_string(home_file).unwrap(),
            "export HOME_VALUE=preserve\n"
        );
        let configured = fs::read_to_string(zdot_file).unwrap();
        assert!(configured.starts_with("export ZDOT_VALUE=preserve\n"));
        assert!(configured.contains("source '/opt/homebrew/env.sh'"));
        assert_eq!(
            resolve_zshenv_path(&home, Some(root.join("missing").to_str().unwrap())),
            home.join(".zshenv")
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn preserves_shell_content_and_backs_up_before_managed_block_update() {
        let root = temp_dir();
        fs::create_dir_all(&root).unwrap();
        let file = root.join(".zshenv");
        fs::write(&file, "export EDITOR=vim\n").unwrap();
        update_shell_environment(&file, "/opt/homebrew/opt/mayaflux").unwrap();
        let text = fs::read_to_string(&file).unwrap();
        assert!(text.starts_with("export EDITOR=vim"));
        assert!(text.contains("# MayaFlux (installed via Homebrew)"));
        assert_eq!(
            fs::read_to_string(PathBuf::from(format!("{}.weave-backup", file.display()))).unwrap(),
            "export EDITOR=vim\n"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn replaces_existing_managed_block_without_changing_user_path_entries() {
        let root = temp_dir();
        fs::create_dir_all(&root).unwrap();
        let file = root.join(".zshenv");
        fs::write(
            &file,
            "export USER_VALUE=yes\nexport PATH=\"$HOME/.local/bin:$PATH\"\n".to_owned()
                + "# MayaFlux (installed via Homebrew)\nsource '/opt/old/env.sh'\n",
        )
        .unwrap();

        update_shell_environment(&file, "/opt/Home brew/opt/mayaflux").unwrap();

        let text = fs::read_to_string(&file).unwrap();
        assert!(
            text.starts_with("export USER_VALUE=yes\nexport PATH=\"$HOME/.local/bin:$PATH\"\n")
        );
        assert_eq!(text.matches("$HOME/.local/bin").count(), 1);
        assert_eq!(text.matches("# MayaFlux").count(), 1);
        assert!(!text.contains("/opt/old/env.sh"));
        assert!(text.contains("source '/opt/Home brew/opt/mayaflux/env.sh'"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn malformed_shell_file_is_preserved_and_can_be_retried() {
        let root = temp_dir();
        fs::create_dir_all(&root).unwrap();
        let file = root.join(".zshenv");
        let original = [0x65, 0x78, 0x70, 0x6f, 0x72, 0x74, 0xff];
        fs::write(&file, original).unwrap();

        assert!(update_shell_environment(&file, "/opt/homebrew").is_err());
        assert_eq!(fs::read(&file).unwrap(), original);
        fs::write(&file, "export USER_VALUE=yes\n").unwrap();
        update_shell_environment(&file, "/opt/homebrew").unwrap();
        assert!(
            fs::read_to_string(&file)
                .unwrap()
                .contains("source '/opt/homebrew/env.sh'")
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn refuses_conflicting_formula_without_replacement_consent() {
        let (root, brew, log) = fake_brew(false, true);
        let error = install_formula_with_brew(&brew, "mayaflux", false, &|_| {}, |_, _| Ok(()))
            .unwrap_err();
        assert!(error.to_string().contains("mayaflux-dev"));
        let commands = fs::read_to_string(log).unwrap();
        assert!(!commands.contains("uninstall"));
        assert!(!commands.contains("install "));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn failed_install_after_replacement_reports_how_to_restore_previous_edition() {
        let (root, brew, log) = fake_brew(true, true);
        let error =
            install_formula_with_brew(&brew, "mayaflux", true, &|_| {}, |_, _| Ok(())).unwrap_err();
        let commands = fs::read_to_string(log).unwrap();
        assert!(commands.contains("uninstall mayaflux-dev"));
        assert!(commands.contains("install mayaflux/mayaflux/mayaflux"));
        assert!(error.to_string().contains("restore it with"));
        assert!(error.to_string().contains("mayaflux-dev"));
        let _ = fs::remove_dir_all(root);
    }

    fn fake_brew(fail_install: bool, conflicting_installed: bool) -> (PathBuf, PathBuf, PathBuf) {
        let root = temp_dir();
        fs::create_dir_all(root.join("state")).unwrap();
        if conflicting_installed {
            fs::write(root.join("state/mayaflux-dev"), "installed").unwrap();
        }
        let log = root.join("commands.log");
        let brew = root.join("brew");
        let state = root.join("state");
        let script = format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\ncase \"$1:$2\" in\n  list:--formula) test -f '{}/'$3 ;;\n  uninstall:*) rm -f '{}/'$2 ;;\n  install:*) if [ '{}' = yes ]; then echo simulated-install-failure >&2; exit 1; fi; touch '{}/mayaflux' ;;\n  --prefix:*) echo '{}/prefix' ;;\nesac\n",
            log.display(),
            state.display(),
            state.display(),
            if fail_install { "yes" } else { "no" },
            state.display(),
            state.display()
        );
        fs::write(&brew, script).unwrap();
        let mut permissions = fs::metadata(&brew).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&brew, permissions).unwrap();
        (root, brew, log)
    }

    fn temp_dir() -> PathBuf {
        std::env::temp_dir().join(format!(
            "weave-install-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn streams_command_output_lines_to_log_handler() {
        let root = temp_dir();
        fs::create_dir_all(&root).unwrap();
        let command = root.join("tool");
        fs::write(
            &command,
            "#!/bin/sh\necho brew-starting\necho brew-progress >&2\n",
        )
        .unwrap();
        let mut permissions = fs::metadata(&command).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&command, permissions).unwrap();
        let lines = std::sync::Mutex::new(Vec::new());
        run_brew_with_log(&command, &["install", "formula"], &|line| {
            lines.lock().unwrap().push(line.to_owned())
        })
        .unwrap();
        let lines = lines.into_inner().unwrap();
        assert!(lines.iter().any(|line| line == "brew-starting"));
        assert!(lines.iter().any(|line| line == "brew-progress"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_unknown_install_channels_before_running_commands() {
        assert!(formula_for_channel("nightly").is_err());
    }
}

pub fn install_macos(
    channel: &str,
    replace_conflict: bool,
    log: &(impl Fn(&str) + Sync),
) -> Result<String, crate::operations::install::InstallError> {
    let formula =
        formula_for_channel(channel).map_err(crate::operations::install::InstallError::Failed)?;
    Stage::Prerequisites.report("Checking Apple developer tools and Homebrew…", log);
    check_developer_tools_with(|arguments| {
        let output = Command::new("/usr/bin/xcode-select")
            .args(arguments)
            .output()
            .map_err(|error| format!("Could not run xcode-select: {error}"))?;
        Ok(output.status.success())
    })?;
    log("Apple developer tools are selected.");
    log(&format!("Selected MayaFlux edition: {formula}"));
    let brew = ensure_brew_with(log, find_brew, || bootstrap_homebrew(log))?;
    let message = install_formula_with_brew(
        &brew,
        formula,
        replace_conflict,
        log,
        configure_shell_environment,
    )?;
    crate::operations::install::verify_weave_resources(log).map_err(|error| {
        crate::operations::install::InstallError::Failed(format!("MayaFlux is installed and the shell is configured, but Weave resource verification failed: {error}"))
    })?;
    Ok(message)
}

fn check_developer_tools_with(run: impl Fn(&[&str]) -> Result<bool, String>) -> Result<(), String> {
    if run(&["-p"])? {
        return Ok(());
    }
    let requested = run(&["--install"])?;
    Err(if requested {
        "Apple Command Line Tools are missing. Complete the native installation dialog, then retry Install MayaFlux. No Homebrew formula was changed."
    } else {
        "Apple Command Line Tools are unavailable and their installer could not be opened. Install them through Xcode or macOS Software Update, then retry. No Homebrew formula was changed."
    }.into())
}

fn install_formula_with_brew(
    brew: &std::path::Path,
    formula: &str,
    replace_conflict: bool,
    log: &(impl Fn(&str) + Sync),
    configure_environment: impl FnOnce(&std::path::Path, &str) -> Result<(), String>,
) -> Result<String, crate::operations::install::InstallError> {
    Stage::Framework.report(
        "Installing and verifying the selected Homebrew formula and its dependencies…",
        log,
    );
    let conflicting = if formula == "mayaflux" {
        "mayaflux-dev"
    } else {
        "mayaflux"
    };

    log(&format!(
        "Checking whether conflicting formula {conflicting} is installed…"
    ));
    let mut removed_conflict = false;
    if brew_formula_installed(brew, conflicting, log) {
        log(&format!(
            "Found installed conflicting formula: {conflicting}"
        ));
        if !replace_conflict {
            return Err(crate::operations::install::InstallError::Conflict {
                installed_formula: conflicting.to_owned(),
            });
        }
        log(&format!("Removing {conflicting} as requested…"));
        run_brew_with_log(brew, &["uninstall", conflicting], log).map_err(|error| {
            crate::operations::install::InstallError::Failed(format!(
                "Could not remove {conflicting}: {error}"
            ))
        })?;
        log(&format!("Removed {conflicting}."));
        removed_conflict = true;
    } else {
        log("No conflicting MayaFlux formula is installed.");
    }

    let full_formula = format!("mayaflux/mayaflux/{formula}");
    log(&format!("Installing {full_formula} with Homebrew…"));
    run_brew_with_log(brew, &["install", &full_formula], log).map_err(|error| {
        let recovery = if removed_conflict {
            format!(" The previous edition was removed; restore it with `{} install mayaflux/mayaflux/{conflicting}` or retry this install.", brew.display())
        } else {
            String::new()
        };
        crate::operations::install::InstallError::Failed(format!("Homebrew could not install {formula}: {error}\nCheck network access and retry.{recovery}"))
    })?;
    log("Verifying the installed formula…");
    if !brew_formula_installed(brew, formula, log) {
        return Err(crate::operations::install::InstallError::Failed(format!(
            "Homebrew reported success, but '{formula}' is not listed as installed."
        )));
    }
    log("Homebrew confirms the formula is installed.");
    Stage::Environment.report("Configuring the managed MayaFlux shell environment…", log);
    log("Resolving zsh configuration (login-shell ZDOTDIR first)…");
    configure_environment(brew, formula).map_err(|error| {
        crate::operations::install::InstallError::Failed(format!("{formula} is installed and verified, but zsh environment setup failed: {error}\nThe formula can be used directly with Homebrew while this is repaired."))
    })?;
    log("Updated the managed MayaFlux block in the resolved .zshenv.");
    Ok(format!(
        "{formula} installed and verified with Homebrew. Your zsh environment is configured."
    ))
}

fn find_brew() -> Option<PathBuf> {
    brew_candidates()
        .into_iter()
        .find(|candidate| candidate.is_file())
}

fn ensure_brew_with(
    log: &(impl Fn(&str) + Sync),
    mut discover: impl FnMut() -> Option<PathBuf>,
    bootstrap: impl FnOnce() -> Result<(), String>,
) -> Result<PathBuf, String> {
    if let Some(path) = discover() {
        log(&format!("Homebrew found at {}", path.display()));
        return Ok(path);
    }
    log("Homebrew was not found; starting its signed Apple Silicon package installer.");
    bootstrap()?;
    let path = discover().ok_or_else(|| {
        "Homebrew's installer finished, but Homebrew was not found. If you cancelled the installer, retry; otherwise use Homebrew's installation troubleshooting guide: https://docs.brew.sh/Installation".to_owned()
    })?;
    log(&format!("Homebrew installed at {}", path.display()));
    Ok(path)
}

fn bootstrap_homebrew(log: &(impl Fn(&str) + Sync)) -> Result<(), String> {
    const PACKAGE_URL: &str =
        "https://github.com/Homebrew/brew/releases/latest/download/Homebrew.pkg";
    let package = std::env::temp_dir().join(format!(
        "weave-homebrew-{}-{}.pkg",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos()
    ));
    bootstrap_homebrew_package(
        &package,
        log,
        |path| {
            log("Downloading Homebrew.pkg from Homebrew's latest official release…");
            let mut download = Command::new("/usr/bin/curl");
            download
                .args([
                    "--fail",
                    "--location",
                    "--show-error",
                    "--progress-bar",
                    "--retry",
                    "2",
                    "--output",
                ])
                .arg(path)
                .arg(PACKAGE_URL);
            let status = run_command_with_log(&mut download, "curl", &|line| {
                crate::download::report_progress(line, log)
            });
            log(&crate::operations::install::progress::Event::Transfer(None)
                .encode()
                .expect("transfer is encodable"));
            let status = status.map_err(|error| {
                format!("Could not download Homebrew's official installer: {error}")
            })?;
            if status.success() {
                Ok(())
            } else {
                Err("Could not download Homebrew's official Apple Silicon installer. Check network access and retry.".to_owned())
            }
        },
        |path| {
            let signature = Command::new("/usr/sbin/pkgutil")
                .args(["--check-signature"])
                .arg(path)
                .output()
                .map_err(|error| {
                    format!("Could not verify the Homebrew installer signature: {error}")
                })?;
            if signature.status.success() {
                Ok(())
            } else {
                Err(format!(
                    "Homebrew's downloaded package did not pass macOS package signature verification: {}",
                    String::from_utf8_lossy(&signature.stderr).trim()
                ))
            }
        },
        |path| {
            // Installer.app owns the native authorization and cancellation flow.
            let status = Command::new("/usr/bin/open")
                .args(["-W", "-a", "Installer"])
                .arg(path)
                .status()
                .map_err(|error| {
                    format!("Could not open Homebrew's package in Installer.app: {error}")
                })?;
            if status.success() {
                Ok(())
            } else {
                Err("Homebrew's native installer did not complete successfully.".to_owned())
            }
        },
    )
}

fn bootstrap_homebrew_package(
    package: &std::path::Path,
    log: &(impl Fn(&str) + Sync),
    download: impl FnOnce(&std::path::Path) -> Result<(), String>,
    verify_signature: impl FnOnce(&std::path::Path) -> Result<(), String>,
    open_installer: impl FnOnce(&std::path::Path) -> Result<(), String>,
) -> Result<(), String> {
    let result = (|| {
        download(package)?;
        log("Download complete; checking the macOS package signature…");
        verify_signature(package)?;
        log("Package signature verified. Opening macOS Installer for authorization…");
        open_installer(package)?;
        log("macOS Installer closed; checking for Homebrew…");
        Ok(())
    })();
    let _ = fs::remove_file(package);
    result
}

fn formula_for_channel(channel: &str) -> Result<&'static str, String> {
    match channel {
        "stable" => Ok("mayaflux"),
        "development" => Ok("mayaflux-dev"),
        _ => Err(format!("Unsupported MayaFlux install channel: {channel}")),
    }
}

fn brew_formula_installed(
    brew: &std::path::Path,
    formula: &str,
    log: &(impl Fn(&str) + Sync),
) -> bool {
    let result = Command::new(brew)
        .args(["list", "--formula", formula])
        .output();
    match result {
        Ok(output) => {
            for line in String::from_utf8_lossy(&output.stderr).lines() {
                log(line);
            }
            output.status.success()
        }
        Err(error) => {
            log(&format!(
                "Could not check installed formula {formula}: {error}"
            ));
            false
        }
    }
}

fn run_brew_with_log(
    brew: &std::path::Path,
    arguments: &[&str],
    log: &(impl Fn(&str) + Sync),
) -> Result<(), String> {
    log(&format!("$ {} {}", brew.display(), arguments.join(" ")));
    let mut command = Command::new(brew);
    command.args(arguments);
    let status = run_command_with_log(&mut command, &brew.display().to_string(), log)?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("command exited with {status}"))
    }
}

fn configure_shell_environment(brew: &std::path::Path, formula: &str) -> Result<(), String> {
    let prefix = Command::new(brew)
        .args(["--prefix", formula])
        .output()
        .map_err(|error| format!("Could not query Homebrew formula prefix: {error}"))?;
    if !prefix.status.success() {
        return Err("Could not determine the installed MayaFlux Homebrew prefix.".to_owned());
    }
    let prefix = String::from_utf8_lossy(&prefix.stdout).trim().to_owned();
    if prefix.is_empty() {
        return Err("Homebrew returned an empty MayaFlux formula prefix.".to_owned());
    }

    // Resolve ZDOTDIR through a zsh login shell before choosing the target file.
    let zdotdir = Command::new("/bin/zsh")
        .args(["-lc", "printf %s \"${ZDOTDIR:-}\""])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .unwrap_or_default();
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "Could not determine the logged-in user's home directory.".to_owned())?;
    let path = resolve_zshenv_path(&home, Some(&zdotdir));
    update_shell_environment(&path, &prefix)
        .map_err(|error| format!("Failed to update {}: {error}", path.display()))
}

fn resolve_zshenv_path(home: &std::path::Path, zdotdir: Option<&str>) -> PathBuf {
    if let Some(directory) = zdotdir.filter(|value| !value.trim().is_empty()) {
        let path = PathBuf::from(directory);
        if path.is_dir() {
            return path.join(".zshenv");
        }
    }
    home.join(".zshenv")
}

fn update_shell_environment(path: &std::path::Path, brew_prefix: &str) -> std::io::Result<()> {
    use std::io;

    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "refusing to edit a symbolic link",
            ));
        }
    }
    let exists = path.exists();
    let original = if exists {
        fs::read_to_string(path)?
    } else {
        String::new()
    };
    let backup = PathBuf::from(format!("{}.weave-backup", path.display()));
    if exists && !original.contains("# MayaFlux") {
        fs::write(&backup, original.as_bytes())?;
    }

    let mut retained = Vec::new();
    let mut skip_managed_source = false;
    for line in original.lines() {
        if line.contains("# MayaFlux") {
            skip_managed_source = true;
            continue;
        }
        if skip_managed_source && line.trim_start().starts_with("source ") {
            skip_managed_source = false;
            continue;
        }
        skip_managed_source = false;
        if line.contains("MAYAFLUX_ROOT")
            || line.contains("mayaflux_env.sh")
            || line.contains("mayaflux") && line.contains("/env.sh")
        {
            continue;
        }
        retained.push(line);
    }
    while retained.last().is_some_and(|line| line.is_empty()) {
        retained.pop();
    }
    let source = format!("{}/env.sh", brew_prefix.trim_end_matches('/'));
    let quote = |value: &str| format!("'{}'", value.replace('\'', "'\\''"));
    let block = format!(
        "# MayaFlux (installed via Homebrew)\nsource {}\n",
        quote(&source)
    );
    let mut content = retained.join("\n");
    if !content.is_empty() {
        content.push_str("\n\n");
    }
    content.push_str(&block);
    crate::filesystem::atomic_write(path, content.as_bytes())
}
