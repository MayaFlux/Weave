//! Project and community module creation from the desktop UI.

use super::controller::GuiController;
use super::state::OperationKind;
use super::view::publish_result;
use crate::operations::community::{CommunityModuleOptions, create_community_module};
use crate::operations::project::{
    ProjectCreationError, ProjectCreationResult, ProjectOptions, create_project,
};
use crate::{AppScreen, MainWindow, ResultKind};
use slint::ComponentHandle;
use std::path::Path;

/// Connects both creation forms to the shared operations.
/// Filesystem work runs on workers so the forms stay responsive.
pub(super) fn register(window: &MainWindow, controller: &GuiController) {
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

    let creation = controller.clone();
    window.on_create_project(move || {
        creation.with_window(|window| {
            let Some(ticket) = creation.begin(OperationKind::CreateProject) else {
                return;
            };

            let name = window.get_project_name().to_string();
            let destination = window.get_destination().to_string();
            let options = ProjectOptions {
                with_lila: window.get_with_lila(),
                with_vscode: window.get_with_vscode(),
                copy_shader_sources: window.get_copy_shader_sources(),
            };

            let worker = creation.clone();
            std::thread::spawn(move || {
                let outcome = project_creation_outcome(create_project(
                    &name,
                    Path::new(&destination),
                    options,
                ));
                worker.complete(ticket.id, move |window| {
                    if outcome.return_to_projects {
                        window.set_screen(AppScreen::Projects);
                    }
                    publish_result(window, outcome.kind, outcome.title, outcome.message);
                });
            });
        });
    });

    let community = controller.clone();
    window.on_create_community_module(move || {
        community.with_window(|window| {
            let Some(ticket) = community.begin(OperationKind::CreateCommunityModule) else {
                return;
            };

            let name = window.get_module_name().to_string();
            let destination = window.get_destination().to_string();
            let options = CommunityModuleOptions {
                description: window.get_module_description().to_string(),
                min_version: window.get_module_min_version().to_string(),
                needs_lila: window.get_community_needs_lila(),
            };

            let worker = community.clone();
            std::thread::spawn(move || {
                let (kind, title, message) =
                    match create_community_module(&name, Path::new(&destination), &options) {
                        Ok(result) => match result.git_warning {
                            Some(warning) => (
                                ResultKind::Warning,
                                "Module created with a warning",
                                format!("Created at:\n{}\n\n{warning}", result.path.display()),
                            ),
                            None => (
                                ResultKind::Success,
                                "Community module created",
                                format!("Created at:\n{}", result.path.display()),
                            ),
                        },
                        Err(error) => (
                            ResultKind::Error,
                            "Could not create community module",
                            error.to_string(),
                        ),
                    };
                worker.complete(ticket.id, move |window| {
                    publish_result(window, kind, title, message);
                });
            });
        });
    });
}

/// Tells the form whether creation succeeded and which notice to show.
pub(super) struct ProjectCreationOutcome {
    pub return_to_projects: bool,
    pub kind: ResultKind,
    pub title: &'static str,
    pub message: String,
}

/// Returns to Projects only after a successful creation.
pub(super) fn project_creation_outcome(
    result: Result<ProjectCreationResult, ProjectCreationError>,
) -> ProjectCreationOutcome {
    match result {
        Ok(result) => match result.git_warning {
            Some(warning) => ProjectCreationOutcome {
                return_to_projects: true,
                kind: ResultKind::Warning,
                title: "Project created with a warning",
                message: format!("Created at:\n{}\n\n{warning}", result.path.display()),
            },
            None => ProjectCreationOutcome {
                return_to_projects: true,
                kind: ResultKind::Success,
                title: "Project created",
                message: format!("Created at:\n{}", result.path.display()),
            },
        },
        Err(error) => ProjectCreationOutcome {
            return_to_projects: false,
            kind: ResultKind::Error,
            title: "Could not create project",
            message: error.to_string(),
        },
    }
}

/// Chooses a familiar starting folder for the creation picker.
pub(super) fn default_gui_destination() -> std::path::PathBuf {
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
