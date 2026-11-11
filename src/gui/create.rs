//! Project and community module creation from the desktop UI.

use super::controller::GuiController;
use super::state::OperationKind;
use super::view::publish_result;
use crate::operations::community::{self, CommunityModuleOptions, create_community_module};
use crate::operations::project::{
    self, ProjectCreationError, ProjectCreationResult, ProjectOptions, create_project,
};
use crate::{AppScreen, MainWindow, ResultKind};
use slint::ComponentHandle;
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CreationField {
    ProjectName,
    Destination,
    ModuleName,
    ModuleVersion,
}

struct CreationFieldError {
    field: CreationField,
    message: String,
}

/// Returns the first project field the user can correct before creation.
fn project_form_error(name: &str, destination: &Path) -> Option<CreationFieldError> {
    if let Err(error) = project::validate_name(name) {
        return Some(CreationFieldError {
            field: CreationField::ProjectName,
            message: error.to_string(),
        });
    }

    if destination.as_os_str().is_empty() {
        return Some(CreationFieldError {
            field: CreationField::Destination,
            message: "Choose a project destination.".to_owned(),
        });
    }

    project::resolve_destination(destination)
        .err()
        .map(|error| CreationFieldError {
            field: CreationField::Destination,
            message: error.to_string(),
        })
}

/// Returns the first module field the user can correct before creation.
fn community_form_error(
    name: &str,
    version: &str,
    destination: &Path,
) -> Option<CreationFieldError> {
    if let Err(error) = community::validate_name(name) {
        return Some(CreationFieldError {
            field: CreationField::ModuleName,
            message: error.to_string(),
        });
    }

    if let Err(error) = community::validate_min_version(if version.trim().is_empty() {
        "0.5.0"
    } else {
        version.trim()
    }) {
        return Some(CreationFieldError {
            field: CreationField::ModuleVersion,
            message: error.to_string(),
        });
    }

    destination
        .as_os_str()
        .is_empty()
        .then(|| CreationFieldError {
            field: CreationField::Destination,
            message: "Choose a community module destination.".to_owned(),
        })
}

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
            window.set_destination_error("".into());
        }
    });

    let creation = controller.clone();
    window.on_create_project(move || {
        creation.with_window(|window| {
            if window.get_busy() {
                return;
            }

            window.set_project_name_error("".into());
            window.set_destination_error("".into());

            let Some(ticket) = creation.begin(OperationKind::CreateProject) else {
                return;
            };

            let name = window.get_project_name().to_string();
            let destination = window.get_destination().to_string();
            let options = ProjectOptions {
                with_lila: window.get_with_lila(),
                with_vscode: window.get_with_vscode(),
                copy_shader_sources: window.get_copy_shader_sources(),
                fetch_assets: window.get_fetch_assets(),
            };

            let worker = creation.clone();
            std::thread::spawn(move || {
                if let Some(error) = project_form_error(&name, Path::new(&destination)) {
                    worker.complete(ticket.id, move |window| match error.field {
                        CreationField::ProjectName => {
                            window.set_project_name_error(error.message.into());
                        }
                        CreationField::Destination => {
                            window.set_destination_error(error.message.into());
                        }
                        _ => unreachable!(),
                    });
                    return;
                }

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
            if window.get_busy() {
                return;
            }

            window.set_module_name_error("".into());
            window.set_module_min_version_error("".into());
            window.set_destination_error("".into());

            let name = window.get_module_name().to_string();
            let version = window.get_module_min_version().to_string();
            let destination = window.get_destination().to_string();
            if let Some(error) = community_form_error(&name, &version, Path::new(&destination)) {
                match error.field {
                    CreationField::ModuleName => window.set_module_name_error(error.message.into()),
                    CreationField::ModuleVersion => {
                        window.set_module_min_version_error(error.message.into())
                    }
                    CreationField::Destination => {
                        window.set_destination_error(error.message.into())
                    }
                    _ => unreachable!(),
                }
                return;
            }

            let Some(ticket) = community.begin(OperationKind::CreateCommunityModule) else {
                return;
            };

            let options = CommunityModuleOptions {
                description: window.get_module_description().to_string(),
                min_version: version,
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
        Ok(result) => {
            let warnings = [result.git_warning, result.asset_warning]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            if !warnings.is_empty() {
                ProjectCreationOutcome {
                    return_to_projects: true,
                    kind: ResultKind::Warning,
                    title: "Project created with a warning",
                    message: format!(
                        "Created at:\n{}\n\n{}",
                        result.path.display(),
                        warnings.join("\n\n")
                    ),
                }
            } else {
                ProjectCreationOutcome {
                    return_to_projects: true,
                    kind: ResultKind::Success,
                    title: "Project created",
                    message: format!("Created at:\n{}", result.path.display()),
                }
            }
        }
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

#[cfg(test)]
mod form_tests {
    use super::{CreationField, community_form_error, project_form_error};
    use std::path::Path;

    #[test]
    fn project_form_reports_invalid_name_and_destination() {
        let destination = std::env::temp_dir();
        assert_eq!(
            project_form_error("bad name", &destination).map(|error| error.field),
            Some(CreationField::ProjectName)
        );

        let missing = destination.join(format!(
            "weave-missing-form-destination-{}",
            crate::filesystem::unique_id()
        ));
        assert_eq!(
            project_form_error("GoodName", &missing).map(|error| error.field),
            Some(CreationField::Destination)
        );
        assert!(project_form_error("GoodName", &destination).is_none());
    }

    #[test]
    fn community_form_identifies_invalid_name_and_version() {
        assert_eq!(
            community_form_error("Bad Name", "0.5.0", Path::new(".")).map(|error| error.field),
            Some(CreationField::ModuleName)
        );
        assert_eq!(
            community_form_error("good_name", "latest", Path::new(".")).map(|error| error.field),
            Some(CreationField::ModuleVersion)
        );
        assert!(community_form_error("good_name", "", Path::new(".")).is_none());
    }
}
