use crate::install::run_command_with_log;
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
    fn uses_verified_zdotdir_before_home_fallback() {
        let root = temp_dir();
        let home = root.join("home");
        let zdot = root.join("dotfiles");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&zdot).unwrap();
        assert_eq!(
            resolve_zshenv_path(&home, Some(zdot.to_str().unwrap())),
            zdot.join(".zshenv")
        );
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
) -> Result<String, crate::install::InstallError> {
    let formula = formula_for_channel(channel).map_err(crate::install::InstallError::Failed)?;
    log(&format!("Selected MayaFlux edition: {formula}"));
    let brew = match find_brew() {
        Some(path) => {
            log(&format!("Homebrew found at {}", path.display()));
            path
        }
        None => {
            log("Homebrew was not found; starting its signed Apple Silicon package installer.");
            bootstrap_homebrew(log)?;
            let path = find_brew().ok_or_else(|| {
                "Homebrew's installer finished, but Homebrew was not found. If you cancelled the installer, retry; otherwise use Homebrew's installation troubleshooting guide: https://docs.brew.sh/Installation".to_owned()
            })?;
            log(&format!("Homebrew installed at {}", path.display()));
            path
        }
    };
    install_formula_with_brew(
        &brew,
        formula,
        replace_conflict,
        log,
        configure_shell_environment,
    )
}

fn install_formula_with_brew(
    brew: &std::path::Path,
    formula: &str,
    replace_conflict: bool,
    log: &(impl Fn(&str) + Sync),
    configure_environment: impl FnOnce(&std::path::Path, &str) -> Result<(), String>,
) -> Result<String, crate::install::InstallError> {
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
            return Err(crate::install::InstallError::Conflict {
                installed_formula: conflicting.to_owned(),
            });
        }
        log(&format!("Removing {conflicting} as requested…"));
        run_brew_with_log(brew, &["uninstall", conflicting], log).map_err(|error| {
            crate::install::InstallError::Failed(format!("Could not remove {conflicting}: {error}"))
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
        crate::install::InstallError::Failed(format!("Homebrew could not install {formula}: {error}\nCheck network access and retry.{recovery}"))
    })?;
    log("Verifying the installed formula…");
    if !brew_formula_installed(brew, formula, log) {
        return Err(crate::install::InstallError::Failed(format!(
            "Homebrew reported success, but '{formula}' is not listed as installed."
        )));
    }
    log("Homebrew confirms the formula is installed.");
    log("Resolving zsh configuration (login-shell ZDOTDIR first)…");
    configure_environment(brew, formula).map_err(|error| {
        crate::install::InstallError::Failed(format!("{formula} is installed and verified, but zsh environment setup failed: {error}\nThe formula can be used directly with Homebrew while this is repaired."))
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
        .arg(&package)
        .arg(PACKAGE_URL);
    let download_status = run_command_with_log(&mut download, "curl", log)
        .map_err(|error| format!("Could not download Homebrew's official installer: {error}"))?;
    if !download_status.success() {
        let _ = fs::remove_file(&package);
        return Err("Could not download Homebrew's official Apple Silicon installer. Check network access and retry.".to_owned());
    }
    log("Download complete; checking the macOS package signature…");
    let signature = Command::new("/usr/sbin/pkgutil")
        .args(["--check-signature"])
        .arg(&package)
        .output()
        .map_err(|error| format!("Could not verify the Homebrew installer signature: {error}"))?;
    if !signature.status.success() {
        let _ = fs::remove_file(&package);
        return Err(format!(
            "Homebrew's downloaded package did not pass macOS package signature verification: {}",
            String::from_utf8_lossy(&signature.stderr).trim()
        ));
    }
    log("Package signature verified. Opening macOS Installer for authorization…");
    // Installer.app owns the native authorization and cancellation flow.
    let opened = Command::new("/usr/bin/open")
        .args(["-W", "-a", "Installer"])
        .arg(&package)
        .status()
        .map_err(|error| format!("Could not open Homebrew's package in Installer.app: {error}"));
    let _ = fs::remove_file(&package);
    let status = opened?;
    if !status.success() {
        return Err("Homebrew's native installer did not complete successfully.".to_owned());
    }
    log("macOS Installer closed; checking for Homebrew…");
    Ok(())
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
    fs::write(path, content)
}
