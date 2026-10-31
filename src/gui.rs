use crate::MainWindow;
use crate::operations::community::{CommunityModuleOptions, create_community_module};
use crate::operations::install;
use crate::operations::install::progress::{Event, Platform, Progress, Status};
use crate::operations::project::{
    ProjectCreationError, ProjectCreationResult, ProjectOptions, create_project,
};
use crate::operations::project_options::{
    FeatureStatus, ProjectOptionError, ShaderConflictPolicy, apply_project_options,
    inspect_project_options,
};
use crate::operations::project_validation::{ValidationResult, validate_project};
use crate::operations::update;
use slint::{ComponentHandle, Model};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub fn run_gui() -> Result<(), Box<dyn std::error::Error>> {
    let window = MainWindow::new()?;
    let destination = default_gui_destination().display().to_string();
    window.set_install_available(true);
    #[cfg(target_os = "windows")]
    window.set_windows_install(true);
    #[cfg(target_os = "linux")]
    window.set_linux_install(true);
    window.set_destination(destination.into());

    let weak_window = window.as_weak();
    let validation_cancellation: Arc<Mutex<Option<Arc<AtomicBool>>>> = Arc::new(Mutex::new(None));
    let weak_window_for_picker = window.as_weak();
    window.on_choose_destination(move || {
        let Some(window) = weak_window_for_picker.upgrade() else {
            return;
        };
        let current = window.get_destination().to_string();
        let mut picker = rfd::FileDialog::new();
        if !current.is_empty() {
            picker = picker.set_directory(current);
        }
        if let Some(destination) = picker.pick_folder() {
            window.set_destination(destination.display().to_string().into());
        }
    });

    let weak_window_for_manage_picker = window.as_weak();
    window.on_choose_manage_project(move || {
        let Some(window) = weak_window_for_manage_picker.upgrade() else {
            return;
        };
        if let Some(project_path) = rfd::FileDialog::new().pick_folder() {
            window.set_managed_project_path(project_path.display().to_string().into());
            window.set_screen(3);
            window.set_add_lila_option(false);
            window.set_add_shader_sources_option(false);
            start_manage_option_inspection(&window);
        }
    });

    let weak_window_for_install = window.as_weak();
    window.on_install_mayaflux(move |channel| {
        let Some(window) = weak_window_for_install.upgrade() else {
            return;
        };
        let replace_conflict = window.get_install_replace_confirmed();
        start_mayaflux_install(&window, channel.to_string(), replace_conflict);
    });

    let weak_window_for_retry = window.as_weak();
    window.on_retry_install(move || {
        let Some(window) = weak_window_for_retry.upgrade() else {
            return;
        };
        if window.get_busy() {
            return;
        }
        window.set_install_progress_open(false);
        #[cfg(target_os = "linux")]
        {
            window.set_install_replace_confirmed(window.get_install_retry_replace_confirmed());
            window.set_install_password("".into());
            window.set_password_dialog_open(true);
        }
        #[cfg(not(target_os = "linux"))]
        start_mayaflux_install(
            &window,
            window.get_install_channel().to_string(),
            window.get_install_retry_replace_confirmed(),
        );
    });

    #[cfg(target_os = "linux")]
    {
        let weak_window_for_prepare = window.as_weak();
        window.on_prepare_install_mayaflux(move |channel| {
            let Some(window) = weak_window_for_prepare.upgrade() else {
                return;
            };
            let channel = channel.to_string();
            window.set_busy(true);
            window.set_install_password("".into());
            window.set_install_replace_confirmed(false);
            let weak = window.as_weak();
            std::thread::spawn(move || {
                let result = install::check_mayaflux_conflict(&channel);
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(window) = weak.upgrade() {
                        window.set_busy(false);
                        match result {
                            Ok(Some(installed_package)) => {
                                show_install_conflict(&window, &channel, &installed_package);
                            }
                            Ok(None) => window.set_password_dialog_open(true),
                            Err(error) => publish_result(
                                &window,
                                2,
                                "Could not check MayaFlux packages",
                                error.to_string(),
                            ),
                        }
                    }
                });
            });
        });
    }

    let weak_window_for_replace = window.as_weak();
    window.on_confirm_install_replacement(move || {
        let Some(window) = weak_window_for_replace.upgrade() else {
            return;
        };
        #[cfg(not(target_os = "linux"))]
        let channel = window.get_pending_install_channel().to_string();
        window.set_conflict_confirmation_open(false);
        #[cfg(target_os = "linux")]
        {
            window.set_install_replace_confirmed(true);
            window.set_password_dialog_open(true);
            return;
        }
        #[cfg(not(target_os = "linux"))]
        start_mayaflux_install(&window, channel, true);
    });

    let weak_window_for_cancel_install = window.as_weak();
    window.on_cancel_install_replacement(move || {
        let Some(window) = weak_window_for_cancel_install.upgrade() else {
            return;
        };
        window.set_conflict_confirmation_open(false);
        window.set_install_replace_confirmed(false);
        window.set_install_password("".into());
        window.set_password_dialog_open(false);
        publish_result(
            &window,
            1,
            "Installation cancelled",
            "No MayaFlux package was changed.".to_owned(),
        );
    });

    window.on_create_project(move || {
        let Some(window) = weak_window.upgrade() else {
            return;
        };
        let name = window.get_project_name().to_string();
        let destination = window.get_destination().to_string();
        let options = ProjectOptions {
            with_lila: window.get_with_lila(),
            with_vscode: window.get_with_vscode(),
            copy_shader_sources: window.get_copy_shader_sources(),
        };
        let outcome =
            project_creation_outcome(create_project(&name, Path::new(&destination), options));
        if outcome.return_to_projects {
            window.set_screen(1);
        }
        publish_result(&window, outcome.kind, outcome.title, outcome.message);
    });

    let weak_window_for_add_modules = window.as_weak();
    window.on_add_community_modules(move || {
        let Some(window) = weak_window_for_add_modules.upgrade() else {
            return;
        };
        let project_path = window.get_managed_project_path().to_string();
        let names = window.get_module_names().to_string();
        window.set_busy(true);
        let weak = window.as_weak();
        std::thread::spawn(move || {
            let mut output = Vec::new();
            let result = update::update_project(Path::new(&project_path), &[names], |line| {
                output.push(line)
            });
            let output = output.join("\n");
            let (kind, title, message) = match result {
                Ok(()) => (0, "Community modules added", output),
                Err(error) if output.is_empty() => {
                    (2, "Could not add community modules", error.to_string())
                }
                Err(error) => (
                    2,
                    "Could not add community modules",
                    format!("{output}\n\n{error}"),
                ),
            };
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(window) = weak.upgrade() {
                    window.set_busy(false);
                    publish_result(&window, kind, title, message);
                }
            });
        });
    });

    let weak_window_for_options = window.as_weak();
    window.on_apply_project_options(move || {
        let Some(window) = weak_window_for_options.upgrade() else {
            return;
        };
        start_project_option_application(&window, ShaderConflictPolicy::Reject);
    });

    let weak_window_for_validate = window.as_weak();
    let validation_for_start = validation_cancellation.clone();
    window.on_validate_project(move || {
        let Some(window) = weak_window_for_validate.upgrade() else {
            return;
        };
        let project_path = window.get_managed_project_path().to_string();
        start_project_validation(&window, project_path, validation_for_start.clone());
    });

    let weak_window_for_cancel_validation = window.as_weak();
    let validation_for_cancel = validation_cancellation.clone();
    window.on_cancel_validation(move || {
        let Some(window) = weak_window_for_cancel_validation.upgrade() else {
            return;
        };
        if let Some(cancellation) = validation_for_cancel.lock().unwrap().as_ref() {
            cancellation.store(true, Ordering::Relaxed);
            window.set_result_cancel_enabled(false);
            window.set_result_title("Cancelling validation…".into());
        }
    });

    let weak_window_for_retry_validation = window.as_weak();
    let validation_for_retry = validation_cancellation.clone();
    window.on_retry_validation(move || {
        let Some(window) = weak_window_for_retry_validation.upgrade() else {
            return;
        };
        let project_path = window.get_managed_project_path().to_string();
        start_project_validation(&window, project_path, validation_for_retry.clone());
    });

    let weak_window_for_replace_shaders = window.as_weak();
    window.on_replace_shader_conflicts(move || {
        let Some(window) = weak_window_for_replace_shaders.upgrade() else {
            return;
        };
        window.set_shader_conflict_open(false);
        start_project_option_application(&window, ShaderConflictPolicy::Replace);
    });

    let weak_window_for_skip_shaders = window.as_weak();
    window.on_skip_shader_conflicts(move || {
        let Some(window) = weak_window_for_skip_shaders.upgrade() else {
            return;
        };
        window.set_shader_conflict_open(false);
        start_project_option_application(&window, ShaderConflictPolicy::Skip);
    });

    let weak_window_for_cancel_shaders = window.as_weak();
    window.on_cancel_shader_conflicts(move || {
        let Some(window) = weak_window_for_cancel_shaders.upgrade() else {
            return;
        };
        window.set_shader_conflict_open(false);
        window.set_shader_conflict_files("".into());
        publish_result(
            &window,
            1,
            "Shader copy cancelled",
            "No project files were changed. Select the option again if you want to review the conflicts later.".to_owned(),
        );
    });

    let weak_window_for_community = window.as_weak();
    window.on_create_community_module(move || {
        let Some(window) = weak_window_for_community.upgrade() else {
            return;
        };
        let name = window.get_module_name().to_string();
        let destination = window.get_destination().to_string();
        let options = CommunityModuleOptions {
            description: window.get_module_description().to_string(),
            min_version: window.get_module_min_version().to_string(),
            needs_lila: window.get_community_needs_lila(),
        };
        let (kind, title, message) =
            match create_community_module(&name, Path::new(&destination), &options) {
                Ok(result) => match result.git_warning {
                    Some(warning) => (
                        1,
                        "Module created with a warning",
                        format!("Created at:\n{}\n\n{warning}", result.path.display()),
                    ),
                    None => (
                        0,
                        "Community module created",
                        format!("Created at:\n{}", result.path.display()),
                    ),
                },
                Err(error) => (2, "Could not create community module", error.to_string()),
            };
        publish_result(&window, kind, title, message);
    });

    window.run()?;
    Ok(())
}

fn start_mayaflux_install(window: &MainWindow, channel: String, replace_conflict: bool) {
    if window.get_busy() {
        return;
    }
    #[cfg(target_os = "linux")]
    let mut password = window.get_install_password().to_string();
    #[cfg(not(target_os = "linux"))]
    let mut password = String::new();
    #[cfg(target_os = "linux")]
    if password.is_empty() {
        publish_result(
            window,
            2,
            "Password required",
            "Enter your sudo password before installing MayaFlux.".to_owned(),
        );
        return;
    }
    window.set_install_password("".into());
    window.set_password_dialog_open(false);
    window.set_install_retry_replace_confirmed(replace_conflict);
    window.set_install_replace_confirmed(false);
    window.set_busy(true);
    window.set_install_channel(channel.clone().into());
    window.set_result_open(false);
    window.set_install_failed(false);
    window.set_install_selected_page(0);
    window.set_install_active_page(0);
    let mut initial = Progress::new(Platform::current());
    initial.receive(Event::Output("Starting MayaFlux installation…".into()));
    sync_install_progress(window, &initial);
    let progress = Arc::new(Mutex::new(initial));
    window.set_install_progress_open(true);
    let weak = window.as_weak();
    std::thread::spawn(move || {
        let log_window = weak.clone();
        let log_progress = progress.clone();
        let result = install::install_with_events(
            &channel,
            replace_conflict,
            if password.is_empty() {
                None
            } else {
                Some(&password)
            },
            move |event| {
                let weak = log_window.clone();
                let progress = log_progress.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(window) = weak.upgrade() {
                        let mut progress = progress.lock().unwrap();
                        progress.receive(event);
                        sync_install_progress(&window, &progress);
                    }
                });
            },
        );
        unsafe {
            password.as_mut_vec().fill(0);
        }
        match result {
            Err(install::InstallError::Conflict { installed_formula }) => {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(window) = weak.upgrade() {
                        window.set_busy(false);
                        window.set_install_progress_open(false);
                        show_install_conflict(&window, &channel, &installed_formula);
                    }
                });
            }
            result => {
                let failed = result.is_err();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(window) = weak.upgrade() {
                        window.set_busy(false);
                        window.set_install_password("".into());
                        if !failed {
                            window.set_install_retry_replace_confirmed(false);
                            window.set_screen(0);
                        }
                        let mut progress = progress.lock().unwrap();
                        progress.finish(result.map_err(|error| error.to_string()));
                        window.set_install_failed(failed);
                        sync_install_progress(&window, &progress);
                        window.set_install_selected_page(progress.active as i32);
                    }
                });
            }
        }
    });
}

fn sync_install_progress(window: &MainWindow, progress: &Progress) {
    let previous = window.get_install_active_page();
    let follow = window.get_install_selected_page() == previous;
    let rows: Vec<_> = progress
        .steps
        .iter()
        .map(|step| crate::InstallationStep {
            title: step.stage.title().into(),
            status: match step.status {
                Status::Pending => 0,
                Status::Running => 1,
                Status::Complete => 2,
                Status::Failed => 3,
            },
            output: step.output.clone().into(),
        })
        .collect();
    let model = window.get_install_progress_steps();
    if let Some(existing) = model
        .as_any()
        .downcast_ref::<slint::VecModel<crate::InstallationStep>>()
    {
        for (index, row) in rows.into_iter().enumerate() {
            existing.set_row_data(index, row);
        }
    } else {
        window.set_install_progress_steps(std::rc::Rc::new(slint::VecModel::from(rows)).into());
    }
    window.set_install_active_page(progress.active as i32);
    if follow {
        window.set_install_selected_page(progress.active as i32);
    }
    window.set_install_summary(progress.summary.clone().into());
    window.set_install_fraction(progress.fraction.unwrap_or(-1.0));
}

fn start_manage_option_inspection(window: &MainWindow) {
    window.set_busy(true);
    window.set_lila_option_status("Checking Lila integration…".into());
    window.set_shader_option_status("Checking MayaFlux shaders…".into());
    window.set_can_add_lila(false);
    window.set_can_copy_shader_sources(false);
    let project_path = window.get_managed_project_path().to_string();
    let weak = window.as_weak();
    std::thread::spawn(move || {
        let result = inspect_project_options(Path::new(&project_path));
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(window) = weak.upgrade() {
                window.set_busy(false);
                match result {
                    Ok(inspection) => {
                        window.set_lila_option_status(feature_status_text(&inspection.lila).into());
                        window.set_shader_option_status(
                            feature_status_text(&inspection.shader_sources).into(),
                        );
                        window.set_can_add_lila(inspection.can_add_lila);
                        window.set_can_copy_shader_sources(inspection.can_copy_shader_sources);
                    }
                    Err(error) => {
                        window.set_lila_option_status("Project could not be inspected.".into());
                        window.set_shader_option_status(error.to_string().into());
                    }
                }
            }
        });
    });
}

fn feature_status_text(status: &FeatureStatus) -> String {
    match status {
        FeatureStatus::Enabled => "Already enabled".to_owned(),
        FeatureStatus::Available => "Available to add".to_owned(),
        FeatureStatus::Unknown(reason) => format!("Cannot determine reliably: {reason}"),
    }
}

fn start_project_option_application(window: &MainWindow, conflict_policy: ShaderConflictPolicy) {
    let with_lila = window.get_add_lila_option();
    let copy_shader_sources = window.get_add_shader_sources_option();
    if !with_lila && !copy_shader_sources {
        return;
    }
    let project_path = window.get_managed_project_path().to_string();
    window.set_busy(true);
    let weak = window.as_weak();
    std::thread::spawn(move || {
        let result = apply_project_options(
            Path::new(&project_path),
            with_lila,
            copy_shader_sources,
            conflict_policy,
        );
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(window) = weak.upgrade() {
                window.set_busy(false);
                match result {
                    Ok(messages) => {
                        window.set_add_lila_option(false);
                        window.set_add_shader_sources_option(false);
                        publish_result(&window, 0, "Project options applied", messages.join("\n"));
                        start_manage_option_inspection(&window);
                    }
                    Err(ProjectOptionError::ShaderConflicts(paths)) => {
                        window.set_shader_conflict_files(
                            paths
                                .iter()
                                .map(|path| path.display().to_string())
                                .collect::<Vec<_>>()
                                .join("\n")
                                .into(),
                        );
                        window.set_shader_conflict_open(true);
                    }
                    Err(error) => {
                        publish_result(
                            &window,
                            2,
                            "Could not apply project options",
                            error.to_string(),
                        );
                    }
                }
            }
        });
    });
}

fn start_project_validation(
    window: &MainWindow,
    project_path: String,
    active_cancellation: Arc<Mutex<Option<Arc<AtomicBool>>>>,
) {
    let cancellation = Arc::new(AtomicBool::new(false));
    *active_cancellation.lock().unwrap() = Some(cancellation.clone());
    window.set_busy(true);
    window.set_result_kind(0);
    window.set_result_title("Validating project".into());
    window.set_result_message("".into());
    window.set_result_busy(true);
    window.set_result_cancel_enabled(true);
    window.set_result_retry_visible(false);
    window.set_result_open(true);
    let weak = window.as_weak();
    std::thread::spawn(move || {
        let output_window = weak.clone();
        let result = validate_project(Path::new(&project_path), &cancellation, &|line| {
            let weak = output_window.clone();
            let line = line.to_owned();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(window) = weak.upgrade() {
                    let mut output = window.get_result_message().to_string();
                    if !output.is_empty() {
                        output.push('\n');
                    }
                    output.push_str(&line);
                    window.set_result_message(output.into());
                }
            });
        });
        let cancelled = cancellation.load(Ordering::Relaxed);
        *active_cancellation.lock().unwrap() = None;
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(window) = weak.upgrade() {
                window.set_busy(false);
                window.set_result_cancel_enabled(false);
                window.set_result_retry_visible(true);
                match result {
                    Ok(ValidationResult {
                        findings: false,
                        output,
                    }) => publish_result(&window, 0, "Project validation passed", output),
                    Ok(ValidationResult {
                        findings: true,
                        output,
                    }) => publish_result(&window, 1, "Validation found items to address", output),
                    Err(error) if cancelled => {
                        let output = window.get_result_message().to_string();
                        publish_result(
                            &window,
                            1,
                            "Validation cancelled",
                            append_validation_error(output, error),
                        )
                    }
                    Err(error) => {
                        let output = window.get_result_message().to_string();
                        publish_result(
                            &window,
                            2,
                            "Could not run project validation",
                            append_validation_error(output, error),
                        )
                    }
                }
            }
        });
    });
}

fn append_validation_error(mut output: String, error: String) -> String {
    if !output.is_empty() {
        output.push_str("\n\n");
    }
    output.push_str(&error);
    output
}

fn show_install_conflict(window: &MainWindow, channel: &str, installed_package: &str) {
    window.set_pending_install_channel(channel.into());
    window.set_conflict_confirmation_title(
        format!("{installed_package} is already installed").into(),
    );
    window.set_conflict_confirmation_message(
        format!(
            "Installing this edition requires removing {installed_package}. Replace it and continue?"
        )
        .into(),
    );
    window.set_conflict_confirmation_open(true);
}

fn publish_result(window: &MainWindow, kind: i32, title: &str, message: String) {
    window.set_result_busy(false);
    window.set_result_kind(kind);
    window.set_result_title(title.into());
    window.set_result_message(message.into());
    window.set_result_open(true);
}

struct ProjectCreationOutcome {
    return_to_projects: bool,
    kind: i32,
    title: &'static str,
    message: String,
}

fn project_creation_outcome(
    result: Result<ProjectCreationResult, ProjectCreationError>,
) -> ProjectCreationOutcome {
    match result {
        Ok(result) => match result.git_warning {
            Some(warning) => ProjectCreationOutcome {
                return_to_projects: true,
                kind: 1,
                title: "Project created with a warning",
                message: format!("Created at:\n{}\n\n{warning}", result.path.display()),
            },
            None => ProjectCreationOutcome {
                return_to_projects: true,
                kind: 0,
                title: "Project created",
                message: format!("Created at:\n{}", result.path.display()),
            },
        },
        Err(error) => ProjectCreationOutcome {
            return_to_projects: false,
            kind: 2,
            title: "Could not create project",
            message: error.to_string(),
        },
    }
}

fn default_gui_destination() -> std::path::PathBuf {
    #[cfg(windows)]
    let home = std::env::var_os("USERPROFILE").map(std::path::PathBuf::from);
    #[cfg(not(windows))]
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);

    if let Some(home) = home.filter(|path| path.is_dir()) {
        #[cfg(windows)]
        let preferred = home.join("Documents");
        #[cfg(not(windows))]
        let preferred = home.join("Projects");
        if preferred.is_dir() {
            return preferred;
        }
        return home;
    }

    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::project_creation_outcome;
    use crate::operations::project::{ProjectCreationResult, ProjectOptions, create_project};
    use std::path::{Path, PathBuf};

    #[test]
    fn project_creation_outcome_returns_to_projects_only_after_success() {
        let success = project_creation_outcome(Ok(ProjectCreationResult {
            path: PathBuf::from("created project"),
            git_warning: None,
        }));
        let warning = project_creation_outcome(Ok(ProjectCreationResult {
            path: PathBuf::from("created project with warning"),
            git_warning: Some("Git could not be initialized".to_owned()),
        }));
        let failure = project_creation_outcome(create_project(
            "invalid name",
            Path::new("."),
            ProjectOptions::default(),
        ));

        assert_eq!(success.return_to_projects, true);
        assert_eq!(warning.return_to_projects, true);
        assert_eq!(failure.return_to_projects, false);
        assert_eq!(failure.kind, 2);
    }
}
