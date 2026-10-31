use super::project_validation::{ValidationPlatform, current_platform};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::AtomicBool;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RunReportOptions {
    pub configuration: Option<String>,
    pub executable: Option<PathBuf>,
    pub mayaflux_root: Option<PathBuf>,
}

#[derive(Debug)]
pub struct RunReportCommand {
    pub program: PathBuf,
    pub arguments: Vec<OsString>,
    pub working_directory: PathBuf,
    pub script_path: PathBuf,
}

impl RunReportCommand {
    pub fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command
            .args(&self.arguments)
            .current_dir(&self.working_directory);
        command
    }
}

#[derive(Debug)]
pub struct RunReportResult {
    pub application_exit_code: i32,
    pub console_log: Option<String>,
    pub application_report: Option<String>,
    pub output: String,
}

pub fn run_project_report(
    project: &Path,
    options: &RunReportOptions,
    cancellation: &AtomicBool,
    log: &(impl Fn(&str) + Sync),
) -> Result<RunReportResult, String> {
    let platform = current_platform();
    let interpreter = crate::platform::validation_interpreter()?;
    let command = build_run_report_command_for(project, options, platform, &interpreter)?;
    crate::filesystem::no_links(&command.working_directory.join("mayaflux_run.txt"))?;
    crate::filesystem::no_links(
        &command
            .working_directory
            .join("data/log/mayaflux_report.log"),
    )?;
    let mut command = command.command();
    let output = crate::platform::run_project_process(&mut command, cancellation, log)?;
    parse_run_report_output(output)
}

fn parse_run_report_output(output: Output) -> Result<RunReportResult, String> {
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.trim().is_empty() {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&stderr);
    }
    let application_exit_code = text.lines().find_map(|line| {
        line.trim()
            .strip_prefix("Application exit code: ")
            .and_then(|code| code.parse::<i32>().ok())
    });
    let Some(application_exit_code) = application_exit_code else {
        return Err(format!(
            "The run/report script ended before confirming that the project was launched.{}",
            if text.trim().is_empty() {
                " Check the selected build and MayaFlux installation, then retry.".to_owned()
            } else {
                format!("\n{text}")
            }
        ));
    };
    let path_after = |prefix: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(prefix).map(str::trim).map(str::to_owned))
    };
    Ok(RunReportResult {
        application_exit_code,
        console_log: path_after("Console log:"),
        application_report: path_after("Application report:"),
        output: text,
    })
}

pub fn build_run_report_command_for(
    project: &Path,
    options: &RunReportOptions,
    platform: ValidationPlatform,
    interpreter: &Path,
) -> Result<RunReportCommand, String> {
    let absolute_project = if project.is_absolute() {
        project.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| format!("Cannot resolve the selected project folder: {error}"))?
            .join(project)
    };
    crate::filesystem::no_links(&absolute_project)?;
    if !std::fs::metadata(&absolute_project).is_ok_and(|metadata| metadata.is_dir()) {
        return Err(format!(
            "Selected project folder could not be opened: {}",
            absolute_project.display()
        ));
    }
    let project = std::fs::canonicalize(&absolute_project)
        .map_err(|error| format!("Cannot resolve selected project folder: {error}"))?;
    crate::filesystem::no_links(&project)?;
    if let Some(configuration) = options.configuration.as_deref()
        && !matches!(
            configuration,
            "Debug" | "Release" | "RelWithDebInfo" | "MinSizeRel"
        )
    {
        return Err(format!(
            "Unsupported build configuration: {configuration}. Choose Debug, Release, RelWithDebInfo, or MinSizeRel."
        ));
    }
    let script_name = match platform {
        ValidationPlatform::Linux | ValidationPlatform::MacOS => "run-report.sh",
        ValidationPlatform::Windows => "run-report.ps1",
    };
    let script = project.join(script_name);
    crate::filesystem::no_links(&script)?;
    if !std::fs::metadata(&script).is_ok_and(|metadata| metadata.is_file()) {
        return Err(format!(
            "The project run/report script is missing: {}. Restore it from your project backup or the matching Weave template. Weave will not replace project scripts automatically.",
            script.display()
        ));
    }
    let executable = match &options.executable {
        Some(executable) => {
            let path = if executable.is_absolute() {
                executable.clone()
            } else {
                project.join(executable)
            };
            crate::filesystem::no_links(&path)?;
            if !std::fs::metadata(&path).is_ok_and(|metadata| metadata.is_file()) {
                return Err(format!(
                    "The selected project executable is missing or is not a file: {}. Build the project or choose another executable.",
                    path.display()
                ));
            }
            Some(std::fs::canonicalize(path).map_err(|error| error.to_string())?)
        }
        None => None,
    };
    let mut arguments: Vec<OsString> = match platform {
        ValidationPlatform::Linux | ValidationPlatform::MacOS => vec![
            script.clone().into_os_string(),
            "--project-root".into(),
            project.as_os_str().to_owned(),
        ],
        ValidationPlatform::Windows => vec![
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-ExecutionPolicy".into(),
            "Bypass".into(),
            "-File".into(),
            script.clone().into_os_string(),
            "-ProjectRoot".into(),
            project.as_os_str().to_owned(),
        ],
    };
    let push_option = |arguments: &mut Vec<OsString>, option: &str, value: &std::ffi::OsStr| {
        arguments.push(option.into());
        arguments.push(value.to_owned());
    };
    if let Some(configuration) = options.configuration.as_deref() {
        push_option(
            &mut arguments,
            if platform == ValidationPlatform::Windows {
                "-Configuration"
            } else {
                "--configuration"
            },
            configuration.as_ref(),
        );
    }
    if let Some(executable) = executable {
        push_option(
            &mut arguments,
            if platform == ValidationPlatform::Windows {
                "-Executable"
            } else {
                "--executable"
            },
            executable.as_os_str(),
        );
    }
    if let Some(mayaflux_root) = &options.mayaflux_root {
        push_option(
            &mut arguments,
            if platform == ValidationPlatform::Windows {
                "-MayaFluxRoot"
            } else {
                "--mayaflux-root"
            },
            mayaflux_root.as_os_str(),
        );
    }
    Ok(RunReportCommand {
        program: interpreter.to_path_buf(),
        arguments,
        working_directory: project,
        script_path: script,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operations::project_validation::ValidationPlatform;
    use std::fs;
    use std::path::Path;

    #[test]
    fn command_keeps_project_options_as_distinct_arguments_with_spaces() {
        let root = std::env::temp_dir().join(format!(
            "weave run report {}",
            crate::filesystem::unique_id()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("run-report.sh"), "#!/bin/bash\nexit 0\n").unwrap();
        let executable = root.join("build/My App");
        fs::create_dir_all(executable.parent().unwrap()).unwrap();
        fs::write(&executable, "#!/bin/sh\nexit 0\n").unwrap();
        let install = root.join("MayaFlux Install");
        fs::create_dir(&install).unwrap();
        let options = RunReportOptions {
            configuration: Some("Debug".into()),
            executable: Some(executable),
            mayaflux_root: Some(install.clone()),
        };

        let command = build_run_report_command_for(
            &root,
            &options,
            ValidationPlatform::Linux,
            Path::new("/bin/bash"),
        )
        .unwrap();

        let root = fs::canonicalize(root).unwrap();
        assert_eq!(command.program, Path::new("/bin/bash"));
        assert_eq!(command.working_directory, root);
        assert_eq!(
            command.arguments,
            [
                root.join("run-report.sh").into_os_string(),
                "--project-root".into(),
                root.as_os_str().to_owned(),
                "--configuration".into(),
                "Debug".into(),
                "--executable".into(),
                root.join("build/My App").into_os_string(),
                "--mayaflux-root".into(),
                install.into_os_string(),
            ]
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn windows_command_uses_power_shell_arguments_without_batch_shell_concatenation() {
        let root = std::env::temp_dir().join(format!(
            "weave run report {}",
            crate::filesystem::unique_id()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("run-report.ps1"), "exit 0\n").unwrap();
        let command = build_run_report_command_for(
            &root,
            &RunReportOptions::default(),
            ValidationPlatform::Windows,
            Path::new(r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe"),
        )
        .unwrap();

        let root = fs::canonicalize(root).unwrap();
        assert_eq!(command.arguments[0], "-NoProfile");
        assert_eq!(command.arguments[1], "-NonInteractive");
        assert_eq!(command.arguments[2], "-ExecutionPolicy");
        assert_eq!(command.arguments[3], "Bypass");
        assert_eq!(command.arguments[4], "-File");
        assert_eq!(
            command.arguments[5],
            root.join("run-report.ps1").as_os_str()
        );
        assert_eq!(command.arguments[6], "-ProjectRoot");
        assert_eq!(command.arguments[7], root.as_os_str());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn command_rejects_unsupported_configuration_before_script_launch() {
        let root = std::env::temp_dir().join(format!(
            "weave run report {}",
            crate::filesystem::unique_id()
        ));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("run-report.sh"), "exit 0\n").unwrap();
        let error = build_run_report_command_for(
            &root,
            &RunReportOptions {
                configuration: Some("Profile".into()),
                ..RunReportOptions::default()
            },
            ValidationPlatform::Linux,
            Path::new("/bin/bash"),
        )
        .unwrap_err();

        assert!(error.contains("Unsupported build configuration: Profile"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn missing_run_report_script_has_recovery_guidance() {
        let root = std::env::temp_dir().join(format!(
            "weave run report {}",
            crate::filesystem::unique_id()
        ));
        fs::create_dir_all(&root).unwrap();
        let error = build_run_report_command_for(
            &root,
            &RunReportOptions::default(),
            ValidationPlatform::Linux,
            Path::new("/bin/bash"),
        )
        .unwrap_err();
        assert!(error.contains("run-report.sh"));
        assert!(error.to_lowercase().contains("restore"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn report_output_exposes_both_report_locations_and_application_exit_status() {
        let output = Output {
            status: std::process::Command::new(std::env::current_exe().unwrap())
                .arg("--help")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .unwrap(),
            stdout: b"Application exit code: 0\nConsole log: /project/mayaflux_run.txt\nApplication report: /project/data/log/mayaflux_report.log\n".to_vec(),
            stderr: Vec::new(),
        };

        let result = parse_run_report_output(output).unwrap();

        assert_eq!(result.application_exit_code, 0);
        assert_eq!(
            result.console_log.as_deref(),
            Some("/project/mayaflux_run.txt")
        );
        assert_eq!(
            result.application_report.as_deref(),
            Some("/project/data/log/mayaflux_report.log")
        );
    }

    #[test]
    fn report_output_keeps_nonzero_project_exit_code_and_report_paths() {
        let output = Output {
            status: std::process::Command::new(std::env::current_exe().unwrap())
                .arg("--help")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .unwrap(),
            stdout: b"Application exit code: 17\nConsole log: /project/mayaflux_run.txt\nApplication report: /project/data/log/mayaflux_report.log\n".to_vec(),
            stderr: Vec::new(),
        };

        let result = parse_run_report_output(output).unwrap();

        assert_eq!(result.application_exit_code, 17);
        assert!(result.console_log.is_some());
        assert!(result.application_report.is_some());
    }

    #[test]
    fn run_report_errors_before_launch_retain_actionable_script_output() {
        let output = Output {
            status: std::process::Command::new(std::env::current_exe().unwrap())
                .arg("--help")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .unwrap(),
            stdout: b"[ERROR] Build the project first, or provide --executable.\n".to_vec(),
            stderr: Vec::new(),
        };

        let error = parse_run_report_output(output).unwrap_err();

        assert!(error.contains("before confirming"));
        assert!(error.contains("Build the project first"));
    }

    #[cfg(unix)]
    #[test]
    fn bundled_unix_run_report_runs_a_built_fixture_and_saves_both_reports() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!(
            "weave run report {}",
            crate::filesystem::unique_id()
        ));
        let project = root.join("Project with spaces");
        let install = root.join("MayaFlux install");
        fs::create_dir_all(project.join("build")).unwrap();
        fs::create_dir_all(project.join("data/log")).unwrap();
        fs::create_dir_all(install.join("share/MayaFlux/report")).unwrap();
        #[cfg(target_os = "macos")]
        let template = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/templates/scripts/macos/run-report-macos.sh"
        ));
        #[cfg(target_os = "linux")]
        let template = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/templates/scripts/linux/run-report.sh"
        ));
        fs::write(
            project.join("run-report.sh"),
            template.replace("@PROJECT_NAME@", "FixtureApp"),
        )
        .unwrap();
        fs::write(
            project.join("build/FixtureApp"),
            "#!/bin/sh\nmkdir -p data/log\nprintf 'fixture report\\n' > data/log/mayaflux_report.log\nprintf 'fixture app output\\n'\nexit 0\n",
        )
        .unwrap();
        let executable = project.join("build/FixtureApp");
        let mut permissions = fs::metadata(&executable).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&executable, permissions).unwrap();
        fs::write(
            install.join("share/MayaFlux/report/mayaflux.report.json"),
            "{}\n",
        )
        .unwrap();
        fs::write(project.join("mayaflux_run.txt"), "previous console log\n").unwrap();
        fs::write(
            project.join("data/log/mayaflux_report.log"),
            "previous application report\n",
        )
        .unwrap();

        let result = run_project_report(
            &project,
            &RunReportOptions {
                mayaflux_root: Some(install),
                ..RunReportOptions::default()
            },
            &AtomicBool::new(false),
            &|_| {},
        )
        .unwrap();

        assert_eq!(result.application_exit_code, 0);
        assert!(result.output.contains("fixture app output"));
        assert!(result.output.contains("Previous log saved:"));
        assert!(
            result
                .console_log
                .as_ref()
                .unwrap()
                .ends_with("mayaflux_run.txt")
        );
        assert!(
            result
                .application_report
                .as_ref()
                .unwrap()
                .ends_with("data/log/mayaflux_report.log")
        );
        assert_eq!(
            fs::read_to_string(project.join("data/log/mayaflux_report.log")).unwrap(),
            "fixture report\n"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn report_path_preflight_rejects_linked_log_directories() {
        use std::os::unix::fs::symlink;
        let root = std::env::temp_dir().join(format!(
            "weave run report {}",
            crate::filesystem::unique_id()
        ));
        let external = root.with_extension("external");
        fs::create_dir_all(root.join("data")).unwrap();
        fs::create_dir_all(&external).unwrap();
        fs::write(root.join("run-report.sh"), "exit 0\n").unwrap();
        symlink(&external, root.join("data/log")).unwrap();
        let cancellation = AtomicBool::new(false);

        let error = run_project_report(&root, &RunReportOptions::default(), &cancellation, &|_| {})
            .unwrap_err();

        assert!(error.contains("symbolic link"));
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(external);
    }
}
