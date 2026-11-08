//! Collects Manage Project input and delegates work to shared operations.

mod tasks;

use super::controller::GuiController;
use super::state::OperationKind;
use super::view::publish_result;
use crate::operations::community_modules;
use crate::operations::project_options::ShaderConflictPolicy;
use crate::operations::project_refresh::{self, Difference};
use crate::{AppScreen, MainWindow, RefreshChoice, ResultKind, RetryKind};
use slint::Model;
use std::path::PathBuf;

/// Connects Manage Project actions to the application controller.
/// Registration is grouped by the action a person takes on the screen.
pub(super) fn register(window: &MainWindow, controller: &GuiController) {
    register_selection(window, controller);
    register_modules(window, controller);
    register_options(window, controller);
    register_refresh(window, controller);
    register_processes(window, controller);
    register_conflicts(window, controller);
}

fn register_selection(window: &MainWindow, controller: &GuiController) {
    let picker = controller.clone();
    window.on_choose_manage_project(move || {
        picker.with_window(|window| {
            if window.get_busy() {
                return;
            }

            if let Some(project) = rfd::FileDialog::new().pick_folder() {
                picker.select_project(project.clone());

                window.set_refresh_dialog_open(false);
                window.set_managed_project_path(project.display().to_string().into());
                window.set_screen(AppScreen::ManageProject);
                window.set_add_lila_option(false);
                window.set_add_shader_sources_option(false);
                window.set_run_report_configuration("".into());
                window.set_run_report_executable("".into());
                window.set_run_report_mayaflux_root("".into());

                tasks::inspect_options(&picker, window);
            }
        });
    });
}

fn register_modules(window: &MainWindow, controller: &GuiController) {
    let modules = controller.clone();
    window.on_add_community_modules(move || {
        modules.with_window(|window| {
            let Some(project) = modules.selected_project() else {
                return;
            };

            let Some(ticket) = modules.begin(OperationKind::AddModules) else {
                return;
            };

            let names = window.get_module_names().to_string();
            let worker = modules.clone();

            std::thread::spawn(move || {
                let mut output = Vec::new();
                let result =
                    community_modules::add_to_project(&project, &[names], |line| output.push(line));
                let output = output.join("\n");

                let (kind, title, message) = match result {
                    Ok(()) => (ResultKind::Success, "Community modules added", output),
                    Err(error) if output.is_empty() => (
                        ResultKind::Error,
                        "Could not add community modules",
                        error.to_string(),
                    ),
                    Err(error) => (
                        ResultKind::Error,
                        "Could not add community modules",
                        format!("{output}\n\n{error}"),
                    ),
                };

                worker.complete(ticket.id, move |window| {
                    publish_result(window, kind, title, message);
                });
            });
        });
    });
}

fn register_options(window: &MainWindow, controller: &GuiController) {
    let options = controller.clone();
    window.on_apply_project_options(move || {
        options.with_window(|window| {
            tasks::apply_options(&options, window, ShaderConflictPolicy::Reject);
        });
    });
}

fn register_refresh(window: &MainWindow, controller: &GuiController) {
    let refresh = controller.clone();
    window.on_preview_project_refresh(move || {
        refresh.with_window(|_| {
            let Some(project) = refresh.selected_project() else {
                return;
            };

            let Some(ticket) = refresh.begin(OperationKind::PreviewRefresh) else {
                return;
            };

            let worker = refresh.clone();
            std::thread::spawn(move || {
                let result = project_refresh::preview(&project);
                let presentation = worker.clone();

                worker.complete(ticket.id, move |window| match result {
                    Ok(preview) if preview.files.is_empty() => publish_result(
                        window,
                        ResultKind::Success,
                        "Project files are current",
                        "All refreshable files already match the current Weave templates."
                            .to_owned(),
                    ),
                    Ok(preview) => {
                        let choices = preview
                            .files
                            .iter()
                            .map(|file| RefreshChoice {
                                path: file.relative.display().to_string().into(),
                                status: match file.difference {
                                    Difference::Missing => "missing",
                                    Difference::Changed => "will replace",
                                }
                                .into(),
                                selected: true,
                            })
                            .collect::<Vec<_>>();

                        presentation.store_refresh_preview(preview);
                        window.set_refresh_files(
                            std::rc::Rc::new(slint::VecModel::from(choices)).into(),
                        );
                        window.set_refresh_dialog_open(true);
                    }
                    Err(error) => publish_result(
                        window,
                        ResultKind::Error,
                        "Could not compare project files",
                        error,
                    ),
                });
            });
        });
    });

    let toggle = controller.clone();
    window.on_toggle_refresh_file(move |index| {
        toggle.with_window(|window| {
            let model = window.get_refresh_files();
            if let Some(mut row) = model.row_data(index as usize) {
                row.selected = !row.selected;
                model.set_row_data(index as usize, row);
            }
        });
    });

    let apply_refresh = controller.clone();
    window.on_apply_project_refresh(move || {
        apply_refresh.with_window(|window| {
            let preview = apply_refresh.refresh_preview();
            let Some(preview) = preview else {
                return;
            };

            let model = window.get_refresh_files();
            let selected = (0..model.row_count())
                .filter_map(|index| model.row_data(index))
                .filter(|row| row.selected)
                .map(|row| PathBuf::from(row.path.as_str()))
                .collect::<Vec<_>>();

            window.set_refresh_dialog_open(false);
            if selected.is_empty() {
                return;
            }

            let Some(ticket) = apply_refresh.begin(OperationKind::ApplyRefresh) else {
                return;
            };

            apply_refresh.clear_refresh_preview();
            let worker = apply_refresh.clone();

            std::thread::spawn(move || {
                let result = project_refresh::apply(&preview, &selected);
                worker.complete(ticket.id, move |window| match result {
                    Ok(result) => {
                        let mut message = format!("Refreshed {} project file(s).", result.replaced);
                        if let Some(backup) = result.backup {
                            message.push_str(&format!("\nOriginal files: {}", backup.display()));
                        }
                        publish_result(
                            window,
                            ResultKind::Success,
                            "Project files refreshed",
                            message,
                        );
                    }
                    Err(error) => publish_result(
                        window,
                        ResultKind::Error,
                        "Could not refresh project files",
                        error,
                    ),
                });
            });
        });
    });
}

fn register_processes(window: &MainWindow, controller: &GuiController) {
    let validate = controller.clone();
    window.on_validate_project(move || {
        validate.with_window(|window| tasks::start_validation(&validate, window));
    });

    let report = controller.clone();
    window.on_run_report(move || {
        report.with_window(|window| tasks::start_report(&report, window));
    });

    let executable_picker = controller.clone();
    window.on_choose_run_report_executable(move || {
        executable_picker.with_window(|window| {
            let mut picker = rfd::FileDialog::new();
            if !window.get_run_report_executable().is_empty() {
                picker = picker.set_file_name(window.get_run_report_executable().to_string());
            }

            if let Some(executable) = picker.pick_file() {
                window.set_run_report_executable(executable.display().to_string().into());
            }
        });
    });

    let root_picker = controller.clone();
    window.on_choose_run_report_mayaflux_root(move || {
        root_picker.with_window(|window| {
            let mut picker = rfd::FileDialog::new();
            if !window.get_run_report_mayaflux_root().is_empty() {
                picker = picker.set_directory(window.get_run_report_mayaflux_root());
            }

            if let Some(root) = picker.pick_folder() {
                window.set_run_report_mayaflux_root(root.display().to_string().into());
            }
        });
    });

    let cancel = controller.clone();
    window.on_cancel_project_operation(move || {
        if cancel.cancel_active() {
            cancel.with_window(|window| {
                window.set_result_cancel_enabled(false);
                window.set_result_title("Stopping project operation…".into());
            });
        }
    });

    let retry = controller.clone();
    window.on_retry_project_operation(move || {
        retry.with_window(|window| match window.get_result_retry_kind() {
            RetryKind::Validation => tasks::start_validation(&retry, window),
            RetryKind::RunReport => tasks::start_report(&retry, window),
            _ => {}
        });
    });
}

fn register_conflicts(window: &MainWindow, controller: &GuiController) {
    let replace = controller.clone();
    window.on_replace_shader_conflicts(move || {
        replace.with_window(|window| {
            window.set_shader_conflict_open(false);
            tasks::apply_options(&replace, window, ShaderConflictPolicy::Replace);
        });
    });

    let skip = controller.clone();
    window.on_skip_shader_conflicts(move || {
        skip.with_window(|window| {
            window.set_shader_conflict_open(false);
            tasks::apply_options(&skip, window, ShaderConflictPolicy::Skip);
        });
    });

    let cancel_shaders = controller.clone();
    window.on_cancel_shader_conflicts(move || {
        cancel_shaders.with_window(|window| {
            window.set_shader_conflict_open(false);
            window.set_shader_conflict_files("".into());
            publish_result(
                window,
                ResultKind::Warning,
                "Shader copy cancelled",
                "No project files were changed. Select the option again if you want to review the conflicts later."
                    .to_owned(),
            );
        });
    });
}
