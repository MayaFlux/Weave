//! Manage Project operations and result presentation.

use super::super::controller::GuiController;
use super::super::state::OperationKind;
use super::super::view::publish_result;
use super::report_output::{ReportOutput, format_run_report_output};
use crate::operations::assets::fetch_project_assets;
use crate::operations::project_options::{
    FeatureStatus, ProjectOptionError, ShaderConflictPolicy, apply_project_options,
    inspect_project_options,
};
use crate::operations::project_run_report::{
    RunReportOptions, RunReportResult, run_project_report, supports_stop_and_collect,
};
use crate::operations::project_validation::{ValidationResult, validate_project};
use crate::{MainWindow, ResultKind, RetryKind};
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::Ordering;

/// Reads optional features from the current project.
/// The result updates only the Manage screen's availability and status.
pub(super) fn inspect_options(controller: &GuiController, window: &MainWindow) {
    let Some(project) = controller.selected_project() else {
        return;
    };

    let Some(ticket) = controller.begin(OperationKind::InspectOptions) else {
        return;
    };

    window.set_lila_option_status("Checking Lila integration…".into());
    window.set_shader_option_status("Checking MayaFlux shaders…".into());
    window.set_can_add_lila(false);
    window.set_can_copy_shader_sources(false);

    let worker = controller.clone();
    std::thread::spawn(move || {
        let result = inspect_project_options(&project);
        worker.complete(ticket.id, move |window| match result {
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

/// Applies the selected optional features through the shared operation.
/// A shader conflict returns to the user for an explicit choice.
pub(super) fn apply_options(
    controller: &GuiController,
    window: &MainWindow,
    policy: ShaderConflictPolicy,
) {
    let with_lila = window.get_add_lila_option();
    let copy_shaders = window.get_add_shader_sources_option();
    let fetch_assets = window.get_fetch_assets_option();
    if !with_lila && !copy_shaders && !fetch_assets {
        return;
    }

    let Some(project) = controller.selected_project() else {
        return;
    };

    let Some(ticket) = controller.begin(OperationKind::ApplyOptions) else {
        return;
    };

    let worker = controller.clone();
    std::thread::spawn(move || {
        let result = (|| {
            let mut messages = if with_lila || copy_shaders {
                apply_project_options(&project, with_lila, copy_shaders, policy)?
            } else {
                Vec::new()
            };
            if fetch_assets {
                fetch_project_assets(&project, &mut |line| messages.push(line)).map_err(
                    |error| {
                        ProjectOptionError::Failure(format!("{}\n{error}", messages.join("\n")))
                    },
                )?;
            }
            Ok(messages)
        })();
        let follow_up = worker.clone();
        worker.complete(ticket.id, move |window| match result {
            Ok(messages) => {
                window.set_add_lila_option(false);
                window.set_add_shader_sources_option(false);
                window.set_fetch_assets_option(false);
                publish_result(
                    window,
                    ResultKind::Success,
                    "Project options applied",
                    messages.join("\n"),
                );
                inspect_options(&follow_up, window);
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
            Err(error) => publish_result(
                window,
                ResultKind::Error,
                "Could not apply project options",
                error.to_string(),
            ),
        });
    });
}

/// Managed project processes that stream output and support cancellation.
enum ProjectTask {
    Validation,
    RunReport(RunReportOptions),
}

impl ProjectTask {
    fn kind(&self) -> OperationKind {
        match self {
            Self::Validation => OperationKind::Validation,
            Self::RunReport(_) => OperationKind::RunReport,
        }
    }

    fn title(&self) -> &'static str {
        match self {
            Self::Validation => "Validating project",
            Self::RunReport(_) => "Running project and collecting diagnostics",
        }
    }

    fn retry_kind(&self) -> RetryKind {
        match self {
            Self::Validation => RetryKind::Validation,
            Self::RunReport(_) => RetryKind::RunReport,
        }
    }
}

/// Starts the selected project's bundled validation script.
pub(super) fn start_validation(controller: &GuiController, window: &MainWindow) {
    start_project_task(controller, window, ProjectTask::Validation);
}

/// Runs an already built project and collects its diagnostics.
pub(super) fn start_report(controller: &GuiController, window: &MainWindow) {
    start_project_task(
        controller,
        window,
        ProjectTask::RunReport(run_report_options(window)),
    );
}

/// Gives validation and Run & Report one cancellation and output lifecycle.
/// The shared lifecycle keeps retries and late worker messages consistent.
fn start_project_task(controller: &GuiController, window: &MainWindow, task: ProjectTask) {
    let Some(project) = controller.selected_project() else {
        return;
    };

    let Some(ticket) = controller.begin(task.kind()) else {
        return;
    };

    let stop_supported =
        task.kind() == OperationKind::RunReport && supports_stop_and_collect(&project);

    window.set_result_kind(ResultKind::Success);
    window.set_result_title(task.title().into());
    window.set_result_message(if task.kind() == OperationKind::RunReport && !stop_supported {
        "This project's run-report script predates Stop and collect. Refresh the project script to enable it. Force stop remains available."
    } else {
        ""
    }.into());
    window.set_result_busy(true);
    window.set_result_cancel_enabled(task.kind() == OperationKind::Validation);
    window.set_result_stop_enabled(stop_supported);
    window.set_result_force_enabled(task.kind() == OperationKind::RunReport);
    window.set_result_retry_visible(false);
    window.set_result_retry_kind(task.retry_kind());
    window.set_result_open(true);

    let worker = controller.clone();
    std::thread::spawn(move || {
        let task_kind = task.kind();
        let streamed = Mutex::new(String::new());
        let report_output = Mutex::new(ReportOutput::default());
        let output_controller = Mutex::new(worker.clone());
        let log = |line: &str| {
            {
                let mut output = streamed.lock().unwrap();
                if !output.is_empty() {
                    output.push('\n');
                }
                output.push_str(line);
            }

            let line = if task_kind == OperationKind::RunReport {
                report_output.lock().unwrap().push(line)
            } else {
                line.to_owned()
            };
            output_controller
                .lock()
                .unwrap()
                .post(ticket.id, move |window| {
                    let mut output = window.get_result_message().to_string();
                    if !output.is_empty() {
                        output.push('\n');
                    }
                    output.push_str(&line);
                    window.set_result_message(output.into());
                });
        };

        let result = match task {
            ProjectTask::Validation => validate_project(&project, &ticket.cancellation, &log)
                .map(ProjectTaskResult::Validation),
            ProjectTask::RunReport(options) => run_project_report(
                &project,
                &options,
                stop_supported.then_some(ticket.stop_requested.as_ref()),
                &ticket.cancellation,
                &log,
            )
            .map(ProjectTaskResult::RunReport),
        };

        let cancelled = ticket.cancellation.load(Ordering::Relaxed);
        let stopped = ticket.stop_requested.load(Ordering::Relaxed);
        let streamed = streamed.into_inner().unwrap();

        worker.complete(ticket.id, move |window| {
            present_project_result(window, task_kind, result, cancelled, stopped, streamed);
        });
    });
}

/// Shows the final process result and keeps retry available.
fn present_project_result(
    window: &MainWindow,
    task_kind: OperationKind,
    result: Result<ProjectTaskResult, String>,
    cancelled: bool,
    stopped: bool,
    streamed: String,
) {
    window.set_result_cancel_enabled(false);
    window.set_result_stop_enabled(false);
    window.set_result_force_enabled(false);
    window.set_result_retry_visible(true);
    match result {
        Ok(ProjectTaskResult::Validation(ValidationResult {
            findings: false,
            output,
        })) => publish_result(
            window,
            ResultKind::Success,
            "Project validation passed",
            output,
        ),
        Ok(ProjectTaskResult::Validation(ValidationResult {
            findings: true,
            output,
        })) => publish_result(
            window,
            ResultKind::Warning,
            "Validation found items to address",
            output,
        ),
        Ok(ProjectTaskResult::RunReport(RunReportResult {
            application_exit_code,
            output,
            ..
        })) if stopped => publish_result(
            window,
            if application_exit_code == 0 {
                ResultKind::Success
            } else {
                ResultKind::Warning
            },
            "Run stopped, report collected",
            format_run_report_output(&output),
        ),
        Ok(ProjectTaskResult::RunReport(RunReportResult {
            application_exit_code: 0,
            output,
            ..
        })) => publish_result(
            window,
            ResultKind::Success,
            "Run & Report complete",
            format_run_report_output(&output),
        ),
        Ok(ProjectTaskResult::RunReport(RunReportResult {
            application_exit_code,
            output,
            ..
        })) => publish_result(
            window,
            ResultKind::Warning,
            &format!("Project exited with code {application_exit_code}"),
            format_run_report_output(&output),
        ),
        Err(error) if cancelled => publish_result(
            window,
            ResultKind::Warning,
            failure_title(task_kind, true),
            display_project_output(task_kind, append_error(streamed, error)),
        ),
        Err(error) => publish_result(
            window,
            ResultKind::Error,
            failure_title(task_kind, false),
            display_project_output(task_kind, append_error(streamed, error)),
        ),
    }
}

fn display_project_output(kind: OperationKind, output: String) -> String {
    if kind == OperationKind::RunReport {
        format_run_report_output(&output)
    } else {
        output
    }
}

enum ProjectTaskResult {
    Validation(ValidationResult),
    RunReport(RunReportResult),
}

fn failure_title(kind: OperationKind, cancelled: bool) -> &'static str {
    match (kind, cancelled) {
        (OperationKind::Validation, true) => "Validation cancelled",
        (OperationKind::Validation, false) => "Could not run project validation",
        (OperationKind::RunReport, true) => "Project force stopped",
        (OperationKind::RunReport, false) => "Could not run project and collect diagnostics",
        _ => unreachable!("only project processes use this title"),
    }
}

fn append_error(mut output: String, error: String) -> String {
    if !output.is_empty() {
        output.push_str("\n\n");
    }

    output.push_str(&error);
    output
}

fn run_report_options(window: &MainWindow) -> RunReportOptions {
    let optional_path = |value: String| {
        let value = value.trim();
        (!value.is_empty()).then(|| PathBuf::from(value))
    };

    let configuration = window.get_run_report_configuration().to_string();
    RunReportOptions {
        configuration: (!configuration.trim().is_empty()).then(|| configuration.trim().to_owned()),
        executable: optional_path(window.get_run_report_executable().to_string()),
        mayaflux_root: optional_path(window.get_run_report_mayaflux_root().to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{OperationKind, failure_title};

    #[test]
    fn project_failures_keep_the_operation_name_and_cancellation_distinct() {
        assert_eq!(
            failure_title(OperationKind::Validation, true),
            "Validation cancelled"
        );
        assert_eq!(
            failure_title(OperationKind::Validation, false),
            "Could not run project validation"
        );
        assert_eq!(
            failure_title(OperationKind::RunReport, true),
            "Project force stopped"
        );
        assert_eq!(
            failure_title(OperationKind::RunReport, false),
            "Could not run project and collect diagnostics"
        );
    }
}
