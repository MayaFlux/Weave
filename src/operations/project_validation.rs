use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidationPlatform {
    Linux,
    MacOS,
    Windows,
}

#[derive(Debug)]
pub struct ValidationCommand {
    pub program: PathBuf,
    pub arguments: Vec<std::ffi::OsString>,
    pub working_directory: PathBuf,
    pub script_path: PathBuf,
}

impl ValidationCommand {
    pub fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command
            .args(&self.arguments)
            .current_dir(&self.working_directory);
        command
    }
}

#[derive(Debug)]
pub struct ValidationResult {
    pub findings: bool,
    pub output: String,
}

pub fn validate_project(
    project: &Path,
    cancellation: &std::sync::atomic::AtomicBool,
    log: &(impl Fn(&str) + Sync),
) -> Result<ValidationResult, String> {
    let interpreter = crate::platform::validation_interpreter()?;
    let platform = current_platform();
    let command = build_validation_command_for(project, platform, &interpreter)?;
    refresh_legacy_optional_checks(&command.script_path, platform, log)?;
    let mut command = command.command();
    let output = crate::platform::run_validation_command(&mut command, cancellation, log)?;
    classify_validation_output(output)
}

fn classify_validation_output(output: Output) -> Result<ValidationResult, String> {
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.trim().is_empty() {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&stderr);
    }
    match output.status.code() {
        Some(0) => Ok(ValidationResult {
            findings: false,
            output: text,
        }),
        Some(1) if text.contains("item(s) need attention.") => Ok(ValidationResult {
            findings: true,
            output: text,
        }),
        code => Err(format!(
            "Validation script failed to run correctly (exit code {}).{}",
            code.map_or_else(|| "unknown".to_owned(), |code| code.to_string()),
            if text.trim().is_empty() {
                " Check the script and retry.".to_owned()
            } else {
                format!("\n{text}")
            }
        )),
    }
}

fn current_platform() -> ValidationPlatform {
    #[cfg(target_os = "windows")]
    {
        ValidationPlatform::Windows
    }
    #[cfg(target_os = "macos")]
    {
        ValidationPlatform::MacOS
    }
    #[cfg(target_os = "linux")]
    {
        ValidationPlatform::Linux
    }
}

pub fn build_validation_command_for(
    project: &Path,
    platform: ValidationPlatform,
    interpreter: &Path,
) -> Result<ValidationCommand, String> {
    let absolute_project = if project.is_absolute() {
        project.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| format!("Cannot resolve the selected project folder: {error}"))?
            .join(project)
    };
    crate::filesystem::no_links(&absolute_project)?;
    let metadata = std::fs::metadata(&absolute_project).map_err(|error| {
        format!(
            "Cannot open selected project {}: {error}",
            absolute_project.display()
        )
    })?;
    if !metadata.is_dir() {
        return Err(format!(
            "Selected project is not a folder: {}",
            project.display()
        ));
    }
    let project = std::fs::canonicalize(&absolute_project)
        .map_err(|error| format!("Cannot resolve selected project folder: {error}"))?;
    crate::filesystem::no_links(&project)?;
    let (script_name, arguments): (&str, Vec<std::ffi::OsString>) = match platform {
        ValidationPlatform::Linux => ("validate.sh", vec![project.as_os_str().to_owned()]),
        ValidationPlatform::MacOS => ("validate.sh", vec![project.as_os_str().to_owned()]),
        ValidationPlatform::Windows => (
            "validate.ps1",
            vec![
                "-NoProfile".into(),
                "-NonInteractive".into(),
                "-ExecutionPolicy".into(),
                "Bypass".into(),
                "-File".into(),
                project.join("validate.ps1").into_os_string(),
                "-ProjectRoot".into(),
                project.as_os_str().to_owned(),
            ],
        ),
    };
    let script = project.join(script_name);
    crate::filesystem::no_links(&script)?;
    if !std::fs::metadata(&script).is_ok_and(|metadata| metadata.is_file()) {
        return Err(format!(
            "The project validation script is missing: {}. Restore it from your project backup or the matching Weave template, then retry. Weave will not replace project scripts automatically.",
            script.display()
        ));
    }
    let arguments = if platform == ValidationPlatform::Windows {
        arguments
    } else {
        vec![
            script.clone().into_os_string(),
            project.as_os_str().to_owned(),
        ]
    };
    Ok(ValidationCommand {
        program: interpreter.to_path_buf(),
        arguments,
        working_directory: project,
        script_path: script,
    })
}

fn refresh_legacy_optional_checks(
    script: &Path,
    platform: ValidationPlatform,
    log: &(impl Fn(&str) + Sync),
) -> Result<bool, String> {
    const WINDOWS_COMMUNITY_WARNING: &str = "Write-ValidationResult 'WARN' 'community.cmake is missing.' 'No modules will be discovered through the standard project module list.'";
    const WINDOWS_COMMUNITY_INFO: &str = "Write-ValidationResult 'INFO' 'No community.cmake is present.' 'No community modules are configured.'";
    const WINDOWS_CONFIG_OK: &str =
        "Write-ValidationResult 'OK' 'No mayaflux.json in the project.' 'Engine defaults apply.'";
    const WINDOWS_CONFIG_INFO: &str =
        "Write-ValidationResult 'INFO' 'No mayaflux.json in the project.' 'Engine defaults apply.'";
    const UNIX_OLD_FILE_LIST: &str = "src/user_project.hpp community.cmake .gitignore; do";
    const UNIX_NEW_FILE_LIST: &str = "src/user_project.hpp .gitignore; do";

    crate::filesystem::no_links(script)?;
    let original = std::fs::read_to_string(script).map_err(|error| {
        format!(
            "Could not read validation script {}: {error}",
            script.display()
        )
    })?;
    let mut updated = original.clone();
    match platform {
        ValidationPlatform::Windows => {
            updated = updated.replace(WINDOWS_COMMUNITY_WARNING, WINDOWS_COMMUNITY_INFO);
            updated = updated.replace(WINDOWS_CONFIG_OK, WINDOWS_CONFIG_INFO);
        }
        ValidationPlatform::Linux | ValidationPlatform::MacOS => {
            updated = updated.replace(UNIX_OLD_FILE_LIST, UNIX_NEW_FILE_LIST);
        }
    }
    if updated == original {
        return Ok(false);
    }
    crate::filesystem::atomic_write(script, updated.as_bytes()).map_err(|error| {
        format!(
            "Could not update the known optional-file checks in {}: {error}. Check that the script is writable, then retry.",
            script.display()
        )
    })?;
    log(&format!(
        "Updated only the known optional-file checks in {}. Other script content was preserved.",
        script.display()
    ));
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::time::Duration;

    #[test]
    fn validation_command_uses_project_working_directory_and_preserves_spaces() {
        let root = std::env::temp_dir().join(format!(
            "weave validation {}",
            crate::filesystem::unique_id()
        ));
        fs::create_dir_all(&root).unwrap();
        let script = root.join("validate.sh");
        fs::write(&script, "#!/bin/sh\nexit 0\n").unwrap();

        let command =
            build_validation_command_for(&root, ValidationPlatform::Linux, Path::new("/bin/bash"))
                .unwrap();

        assert_eq!(command.program, Path::new("/bin/bash"));
        let root = fs::canonicalize(&root).unwrap();
        assert_eq!(
            command.arguments,
            [root.join("validate.sh").as_os_str(), root.as_os_str()]
        );
        assert_eq!(command.working_directory, fs::canonicalize(&root).unwrap());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn validation_command_passes_windows_project_root_as_a_separate_argument() {
        let root = std::env::temp_dir().join(format!(
            "weave validation {}",
            crate::filesystem::unique_id()
        ));
        fs::create_dir_all(&root).unwrap();
        let script = root.join("validate.ps1");
        fs::write(&script, "Write-Output ok").unwrap();

        let command = build_validation_command_for(
            &root,
            ValidationPlatform::Windows,
            Path::new(r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe"),
        )
        .unwrap();

        assert_eq!(
            command.program,
            Path::new(r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe")
        );
        let root = fs::canonicalize(&root).unwrap();
        assert_eq!(
            command.arguments,
            [
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ]
            .map(Into::into)
            .into_iter()
            .chain([
                root.join("validate.ps1").into_os_string(),
                "-ProjectRoot".into(),
                root.as_os_str().to_owned()
            ])
            .collect::<Vec<_>>()
        );
        assert_eq!(command.working_directory, fs::canonicalize(&root).unwrap());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn validation_command_reports_missing_script_with_recovery_guidance() {
        let root = std::env::temp_dir().join(format!(
            "weave validation {}",
            crate::filesystem::unique_id()
        ));
        fs::create_dir_all(&root).unwrap();
        let error =
            build_validation_command_for(&root, ValidationPlatform::Linux, Path::new("/bin/bash"))
                .unwrap_err();
        assert!(error.contains("validate.sh"));
        assert!(error.to_lowercase().contains("restore"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn optional_project_files_are_reported_as_information_on_every_platform() {
        let unix_templates = [
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/templates/scripts/linux/validate.sh"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/resources/templates/scripts/macos/validate-macos.sh"
            )),
        ];
        for script in unix_templates {
            let required_files = script
                .lines()
                .find(|line| line.starts_with("for relative in "))
                .unwrap();
            assert!(
                !required_files
                    .split_whitespace()
                    .any(|file| file.trim_matches([',', '\'', ')', '(']) == "community.cmake")
            );
            assert!(script.contains(
                "[INFO] No community.cmake is present. No community modules are configured."
            ));
            assert!(
                script.contains("[INFO] No mayaflux.json in the project. Engine defaults apply.")
            );
        }

        let windows = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/templates/scripts/windows/validate.ps1"
        ));
        let required_files = windows
            .lines()
            .find(|line| line.contains("foreach ($relative in @("))
            .unwrap();
        assert!(!required_files.contains("'community.cmake'"));
        assert!(windows.contains("Write-ValidationResult 'INFO' 'No community.cmake is present.'"));
        assert!(
            windows.contains("Write-ValidationResult 'INFO' 'No mayaflux.json in the project.'")
        );
    }

    #[test]
    fn legacy_validator_refresh_changes_only_the_known_optional_file_checks() {
        let root = std::env::temp_dir().join(format!(
            "weave validation {}",
            crate::filesystem::unique_id()
        ));
        fs::create_dir_all(&root).unwrap();
        let script = root.join("validate.ps1");
        fs::write(
            &script,
            "custom user validation\nWrite-ValidationResult 'WARN' 'community.cmake is missing.' 'No modules will be discovered through the standard project module list.'\nWrite-ValidationResult 'OK' 'No mayaflux.json in the project.' 'Engine defaults apply.'\n",
        )
        .unwrap();
        let messages = std::sync::Mutex::new(Vec::new());

        let updated =
            refresh_legacy_optional_checks(&script, ValidationPlatform::Windows, &|message| {
                messages.lock().unwrap().push(message.to_owned());
            })
            .unwrap();
        let content = fs::read_to_string(&script).unwrap();

        assert!(updated);
        assert!(content.starts_with("custom user validation\n"));
        assert!(content.contains("Write-ValidationResult 'INFO' 'No community.cmake is present.'"));
        assert!(
            content.contains("Write-ValidationResult 'INFO' 'No mayaflux.json in the project.'")
        );
        assert!(messages.lock().unwrap()[0].contains("preserved"));
        assert!(
            !refresh_legacy_optional_checks(&script, ValidationPlatform::Windows, &|_| {}).unwrap()
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn legacy_unix_validator_refresh_removes_only_community_from_required_files() {
        let root = std::env::temp_dir().join(format!(
            "weave validation {}",
            crate::filesystem::unique_id()
        ));
        fs::create_dir_all(&root).unwrap();
        let script = root.join("validate.sh");
        fs::write(
            &script,
            "custom project validation\nfor relative in CMakeLists.txt src/user_project.hpp community.cmake .gitignore; do\n  check \"$relative\"\ndone\n",
        )
        .unwrap();

        assert!(
            refresh_legacy_optional_checks(&script, ValidationPlatform::MacOS, &|_| {}).unwrap()
        );
        let content = fs::read_to_string(&script).unwrap();

        assert!(content.starts_with("custom project validation\n"));
        assert!(content.contains("src/user_project.hpp .gitignore; do"));
        assert!(!content.contains("src/user_project.hpp community.cmake .gitignore; do"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn optional_file_messages_do_not_turn_clean_validation_into_findings() {
        let output = Output {
            status: std::process::Command::new(std::env::current_exe().unwrap())
                .arg("--help")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .unwrap(),
            stdout: b"[INFO] No community.cmake is present. No community modules are configured.\n[INFO] No mayaflux.json in the project. Engine defaults apply.\n[WARN] Optional diagnostic detail.\nNo problems found in these checks.\n".to_vec(),
            stderr: Vec::new(),
        };

        let result = classify_validation_output(output).unwrap();

        assert!(!result.findings);
        assert!(result.output.contains("[WARN] Optional diagnostic detail."));
    }

    #[test]
    fn validation_streams_output_and_classifies_script_findings() {
        let root = std::env::temp_dir().join(format!(
            "weave validation {}",
            crate::filesystem::unique_id()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("validate.sh"),
            "printf 'cwd=%s\\n' \"$PWD\"; printf '[FIX] fixture finding\\n1 item(s) need attention.\\n'; exit 1\n",
        )
        .unwrap();
        let streamed = std::sync::Mutex::new(Vec::new());
        let cancellation = std::sync::atomic::AtomicBool::new(false);

        let result = validate_project(&root, &cancellation, &|line| {
            streamed.lock().unwrap().push(line.to_owned());
        })
        .unwrap();

        assert!(result.findings);
        assert!(result.output.contains("[FIX] fixture finding"));
        assert!(
            result
                .output
                .contains(&fs::canonicalize(&root).unwrap().display().to_string())
        );
        assert!(
            streamed
                .lock()
                .unwrap()
                .iter()
                .any(|line| line.contains("fixture finding"))
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn unexpected_script_exit_is_reported_as_a_launch_failure() {
        let root = std::env::temp_dir().join(format!(
            "weave validation {}",
            crate::filesystem::unique_id()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("validate.sh"),
            "printf 'script crashed\\n'; exit 2\n",
        )
        .unwrap();
        let cancellation = std::sync::atomic::AtomicBool::new(false);

        let error = validate_project(&root, &cancellation, &|_| {}).unwrap_err();

        assert!(error.contains("exit code 2"));
        assert!(error.contains("script crashed"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn exit_one_without_the_validator_findings_summary_is_a_script_failure() {
        let root = std::env::temp_dir().join(format!(
            "weave validation {}",
            crate::filesystem::unique_id()
        ));
        fs::create_dir_all(&root).unwrap();
        #[cfg(target_os = "windows")]
        fs::write(
            root.join("validate.ps1"),
            "Write-Output '[FIX] unexpected script exit'; exit 1\n",
        )
        .unwrap();
        #[cfg(not(target_os = "windows"))]
        fs::write(
            root.join("validate.sh"),
            "printf '[FIX] unexpected script exit\\n'; exit 1\n",
        )
        .unwrap();
        let cancellation = std::sync::atomic::AtomicBool::new(false);

        let error = validate_project(&root, &cancellation, &|_| {}).unwrap_err();

        assert!(error.contains("exit code 1"));
        assert!(error.contains("unexpected script exit"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn validation_can_be_cancelled_and_retried() {
        let root = std::env::temp_dir().join(format!(
            "weave validation {}",
            crate::filesystem::unique_id()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("validate.sh"), "exec sleep 20\n").unwrap();
        let cancellation = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let signal = cancellation.clone();
        let cancel_after_start = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            signal.store(true, std::sync::atomic::Ordering::Relaxed);
        });

        let error = validate_project(&root, &cancellation, &|_| {}).unwrap_err();
        cancel_after_start.join().unwrap();

        assert!(error.contains("cancelled"));
        fs::write(
            root.join("validate.sh"),
            "printf 'retry passed\\n'; exit 0\n",
        )
        .unwrap();
        let retry = std::sync::atomic::AtomicBool::new(false);
        assert!(!validate_project(&root, &retry, &|_| {}).unwrap().findings);
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn validation_rejects_symlinked_projects() {
        use std::os::unix::fs::symlink;
        let root = std::env::temp_dir().join(format!(
            "weave validation {}",
            crate::filesystem::unique_id()
        ));
        let linked = root.with_extension("link");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("validate.sh"), "exit 0\n").unwrap();
        symlink(&root, &linked).unwrap();

        let error = build_validation_command_for(
            &linked,
            ValidationPlatform::Linux,
            Path::new("/bin/bash"),
        )
        .unwrap_err();

        assert!(error.contains("symbolic link"));
        let _ = fs::remove_file(linked);
        let _ = fs::remove_dir_all(root);
    }
}
