//! Makes the packaged Weave executable available as `Weave` in new terminals.
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub fn install() -> Result<String, String> {
    #[cfg(target_os = "linux")]
    {
        let home = home("HOME")?;
        let source = match std::env::var_os("APPIMAGE").filter(|value| !value.is_empty()) {
            Some(path) => PathBuf::from(path),
            None => std::env::current_exe()
                .map_err(|error| format!("Could not locate Weave executable: {error}"))?,
        };
        let destination = home.join(".local/bin/Weave");
        replace_cli_file(&source, &destination)
            .map_err(|error| format!("Could not install {}: {error}", destination.display()))?;
        ensure_unix_path(&home, "$HOME/.local/bin")?;
        Ok(format!(
            "Weave CLI installed at {}. Open a new terminal to use `Weave`.",
            destination.display()
        ))
    }
    #[cfg(target_os = "macos")]
    {
        let destination = Path::new("/Applications/Weave.app/Contents/MacOS/Weave");
        if !destination.is_file() {
            return Err("Install Weave.app in /Applications, then retry CLI setup. The CLI is inside the app bundle.".into());
        }
        ensure_unix_path(&home("HOME")?, "/Applications/Weave.app/Contents/MacOS")?;
        Ok("Weave CLI is available from /Applications/Weave.app. Open a new terminal to use `Weave`. Replacing the app updates the CLI.".into())
    }
    #[cfg(target_os = "windows")]
    {
        let home = home("USERPROFILE")?;
        let source =
            std::env::current_exe().map_err(|error| format!("Could not locate Weave: {error}"))?;
        let destination = home.join(".local").join("bin").join("Weave.exe");
        if !same_windows_path(&source, &destination) {
            replace_cli_file(&source, &destination).map_err(|error| {
                format!(
                    "Could not install {}: {error}. Close any running CLI and retry.",
                    destination.display()
                )
            })?;
        }
        ensure_windows_path(destination.parent().unwrap())?;
        Ok(format!(
            "Weave CLI installed at {}. Open a new terminal to use `Weave`.",
            destination.display()
        ))
    }
}

#[cfg(target_os = "windows")]
fn same_windows_path(left: &Path, right: &Path) -> bool {
    left.to_string_lossy()
        .eq_ignore_ascii_case(&right.to_string_lossy())
}

fn home(variable: &str) -> Result<PathBuf, String> {
    let home = std::env::var_os(variable)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| format!("Could not determine user home from {variable}"))?;
    if !home.is_absolute() || !home.is_dir() {
        return Err(format!("User home is unavailable: {}", home.display()));
    }
    Ok(home)
}

fn replace_cli_file(source: &Path, destination: &Path) -> io::Result<()> {
    if source == destination {
        return Ok(());
    }
    let source_metadata = fs::symlink_metadata(source)?;
    if !source_metadata.is_file() || source_metadata.len() == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "CLI source must be a nonempty regular file",
        ));
    }
    let parent = destination.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "CLI destination has no parent")
    })?;
    fs::create_dir_all(parent)?;
    if fs::symlink_metadata(parent)?.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "refusing to install through a symbolic link",
        ));
    }
    match fs::symlink_metadata(destination) {
        Ok(metadata) if !metadata.is_file() => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "refusing to replace a non-regular CLI file",
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let temporary = parent.join(format!(
        ".weave-{}-{}.tmp",
        std::process::id(),
        crate::filesystem::unique_id()
    ));
    let result = (|| {
        fs::copy(source, &temporary)?;
        fs::set_permissions(&temporary, source_metadata.permissions())?;
        #[cfg(not(target_os = "windows"))]
        {
            fs::rename(&temporary, destination)
        }
        #[cfg(target_os = "windows")]
        {
            if !destination.exists() {
                return fs::rename(&temporary, destination);
            }
            let backup = parent.join(format!(
                ".weave-{}-{}.old",
                std::process::id(),
                crate::filesystem::unique_id()
            ));
            fs::rename(destination, &backup)?;
            match fs::rename(&temporary, destination) {
                Ok(()) => {
                    let _ = fs::remove_file(&backup);
                    Ok(())
                }
                Err(error) => {
                    let _ = fs::rename(&backup, destination);
                    Err(error)
                }
            }
        }
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(any(unix, test))]
fn add_shell_path(original: &str, directory: &str) -> String {
    if original.contains("# Weave CLI\n") || original.contains(&format!("{directory}:$PATH")) {
        return original.to_owned();
    }
    let mut result = original.to_owned();
    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }
    result.push_str(&format!("# Weave CLI\ncase :\"$PATH\": in\n  *:\"{directory}\":*) ;;\n  *) export PATH=\"{directory}:$PATH\" ;;\nesac\n"));
    result
}

#[cfg(unix)]
fn ensure_unix_path(home: &Path, directory: &str) -> Result<(), String> {
    let shell_path = std::env::var_os("SHELL").unwrap_or_default();
    let shell = Path::new(&shell_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_owned();
    #[cfg(target_os = "macos")]
    let path = crate::platform::macos::user_zshenv_path(home);
    #[cfg(target_os = "linux")]
    let path = match shell.as_str() {
        "zsh" => std::process::Command::new(&shell_path)
            .args(["-lc", "printf %s \"${ZDOTDIR:-}\""])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| PathBuf::from(String::from_utf8_lossy(&output.stdout).trim()))
            .filter(|path| path.is_dir())
            .unwrap_or_else(|| home.to_path_buf())
            .join(".zshenv"),
        "fish" => home.join(".config/fish/config.fish"),
        "bash" => home.join(".bashrc"),
        _ => home.join(".profile"),
    };
    #[cfg(target_os = "macos")]
    let _ = shell;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create {}: {error}", parent.display()))?;
    }
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(format!("Refusing to edit symlink {}", path.display()));
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("Could not inspect {}: {error}", path.display())),
    }
    let original = match fs::read_to_string(&path) {
        Ok(value) => value,
        Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(format!("Could not read {}: {error}", path.display())),
    };
    #[cfg(target_os = "linux")]
    let updated = if shell == "fish" {
        if original.contains("$HOME/.local/bin") {
            original.clone()
        } else {
            format!(
                "{}{}# Weave CLI\nfish_add_path -g $HOME/.local/bin\n",
                original,
                if original.is_empty() || original.ends_with('\n') {
                    ""
                } else {
                    "\n"
                }
            )
        }
    } else {
        add_shell_path(&original, directory)
    };
    #[cfg(target_os = "macos")]
    let updated = add_shell_path(&original, directory);
    if updated != original {
        crate::filesystem::atomic_write(&path, updated.as_bytes())
            .map_err(|error| format!("Could not update {}: {error}", path.display()))?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn ensure_windows_path(directory: &Path) -> Result<(), String> {
    use crate::platform::windows::process;
    let mut read = process::powershell(
        "[Console]::Write(( [Environment]::GetEnvironmentVariable('Path','User') | ConvertTo-Json -Compress ))",
    );
    let output = read
        .output()
        .map_err(|error| format!("Could not read user PATH: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Could not read user PATH: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let existing: Option<String> = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("Invalid user PATH response: {error}"))?;
    let existing = existing.unwrap_or_default();
    let updated = crate::platform::windows::environment::merge_paths(
        &existing,
        &[directory.to_path_buf()],
        false,
    )?;
    if updated != existing {
        let status = process::powershell(
            "[Environment]::SetEnvironmentVariable('Path',$env:WEAVE_USER_PATH,'User')",
        )
        .env("WEAVE_USER_PATH", updated)
        .status()
        .map_err(|error| format!("Could not update user PATH: {error}"))?;
        if !status.success() {
            return Err(
                "Could not update user PATH. Retry CLI setup from your user account.".into(),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn replacing_cli_overwrites_old_artifact_and_keeps_executable_permissions() {
        let root =
            std::env::temp_dir().join(format!("weave-cli-copy-{}", crate::filesystem::unique_id()));
        fs::create_dir_all(&root).unwrap();
        let source = root.join("source");
        let destination = root.join("weave");
        fs::write(&source, b"new release").unwrap();
        fs::write(&destination, b"old release").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&source, fs::Permissions::from_mode(0o755)).unwrap();
        }
        replace_cli_file(&source, &destination).unwrap();
        assert_eq!(fs::read(&destination).unwrap(), b"new release");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&destination).unwrap().permissions().mode() & 0o111,
                0o111
            );
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn shell_path_entry_is_idempotent_and_preserves_existing_text() {
        let original = "export USER_VALUE=yes\n";
        let once = add_shell_path(original, "$HOME/.local/bin");
        let twice = add_shell_path(&once, "$HOME/.local/bin");
        assert_eq!(once, twice);
        assert!(once.starts_with(original));
        assert!(once.contains("export PATH=\"$HOME/.local/bin:$PATH\""));
        assert_eq!(
            add_shell_path(
                "export PATH=\"$HOME/.local/bin:$PATH\"\n",
                "$HOME/.local/bin"
            ),
            "export PATH=\"$HOME/.local/bin:$PATH\"\n"
        );
    }

    #[test]
    fn missing_source_does_not_change_existing_cli() {
        let root = std::env::temp_dir().join(format!(
            "weave-cli-missing-{}",
            crate::filesystem::unique_id()
        ));
        fs::create_dir_all(&root).unwrap();
        let destination = root.join("weave");
        fs::write(&destination, b"working release").unwrap();
        assert!(replace_cli_file(&root.join("missing"), &destination).is_err());
        assert_eq!(fs::read(&destination).unwrap(), b"working release");
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn existing_cli_symlink_is_never_replaced() {
        let root =
            std::env::temp_dir().join(format!("weave-cli-link-{}", crate::filesystem::unique_id()));
        fs::create_dir_all(&root).unwrap();
        let source = root.join("source");
        let target = root.join("target");
        let destination = root.join("weave");
        fs::write(&source, b"new release").unwrap();
        fs::write(&target, b"other app").unwrap();
        std::os::unix::fs::symlink(&target, &destination).unwrap();
        assert!(replace_cli_file(&source, &destination).is_err());
        assert_eq!(fs::read(&target).unwrap(), b"other app");
        fs::remove_dir_all(root).unwrap();
    }
}
