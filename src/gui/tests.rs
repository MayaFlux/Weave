//! GUI state and presentation regression tests.

use super::create::project_creation_outcome;
use super::state::{GuiState, OperationKind};
use super::update::show_update_notice;
use crate::operations::project::{ProjectCreationResult, ProjectOptions, create_project};
use crate::operations::weave_release;
use crate::{MainWindow, ResultKind};
use slint::Model;
use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
use slint::platform::{Platform, PlatformError, WindowAdapter, set_platform};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::Ordering;

#[test]
fn one_operation_owns_the_gui_until_its_matching_completion() {
    let mut state = GuiState::default();
    let first = state.begin(OperationKind::Validation).unwrap();
    assert!(state.begin(OperationKind::RunReport).is_none());
    assert!(!state.finish(first.id + 1));
    assert!(state.is_current(first.id));
    assert!(state.finish(first.id));
    assert!(!state.is_current(first.id));
    assert!(state.begin(OperationKind::RunReport).is_some());
}

#[test]
fn cancellation_only_targets_the_current_operation() {
    let mut state = GuiState::default();
    let first = state.begin(OperationKind::Validation).unwrap();
    assert!(state.cancel_active());
    assert!(first.cancellation.load(Ordering::Relaxed));
    assert!(state.finish(first.id));
    let second = state.begin(OperationKind::RunReport).unwrap();
    assert!(!second.cancellation.load(Ordering::Relaxed));
}

#[test]
fn stopping_a_run_requests_collection_without_forcing_termination() {
    let mut state = GuiState::default();
    let run = state.begin(OperationKind::RunReport).unwrap();

    assert!(state.stop_active_run());
    assert!(run.stop_requested.load(Ordering::Relaxed));
    assert!(!run.cancellation.load(Ordering::Relaxed));

    assert!(state.cancel_active());
    assert!(run.cancellation.load(Ordering::Relaxed));
}

#[test]
fn project_cancel_does_not_stop_an_installation() {
    let mut state = GuiState::default();
    let installation = state.begin(OperationKind::Install).unwrap();
    assert!(!state.cancel_active());
    assert!(!installation.cancellation.load(Ordering::Relaxed));
}

#[test]
fn selecting_a_new_project_discards_its_previous_refresh_preview() {
    let mut state = GuiState::default();
    state.select_project(PathBuf::from("first"));
    state.store_refresh_preview(crate::operations::project_refresh::RefreshPreview {
        project: PathBuf::from("first"),
        files: Vec::new(),
    });
    state.select_project(PathBuf::from("second"));
    assert_eq!(state.selected_project(), Some(Path::new("second")));
    assert!(state.refresh_preview().is_none());
}

struct HeadlessPlatform;

impl Platform for HeadlessPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        Ok(MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer))
    }
}

#[test]
fn fake_release_turns_on_the_gui_update_banner() {
    set_platform(Box::new(HeadlessPlatform)).unwrap();
    let window = MainWindow::new().unwrap();
    assert!(!window.get_update_available());
    assert!(window.get_install_cli_access());

    let directory = std::env::temp_dir().join(format!(
        "weave-gui-update-{}",
        crate::filesystem::unique_id()
    ));
    let state_path = directory.join("update-check.json");

    let release = serde_json::json!({
        "tag_name": "v0.7.0", "draft": false, "prerelease": false,
        "html_url": "https://github.com/MayaFlux/Weave/releases/tag/v0.7.0"
    });
    let notice =
        weave_release::check_with(&state_path, 1_000_000, "0.6.0", || Some(release)).unwrap();
    show_update_notice(&window, notice);

    assert!(window.get_update_available());
    assert_eq!(window.get_update_version().as_str(), "0.7.0");
    assert_eq!(
        window.get_update_url().as_str(),
        "https://github.com/MayaFlux/Weave/releases/tag/v0.7.0"
    );

    for result in [Ok("Installed".into()), Err("Consent cancelled".into())] {
        let mut progress = crate::operations::install::progress::Progress::new(
            crate::operations::install::progress::Platform::Windows,
        );
        progress.receive(crate::operations::install::progress::Event::Advisory(
            crate::operations::install::progress::Advisory {
                vendor: "NVIDIA".into(),
                message: "Update the installed NVIDIA driver".into(),
                url: "https://www.nvidia.com/en-us/drivers/".into(),
            },
        ));
        progress.finish(result);
        super::install::sync_install_progress(&window, &progress);
        assert_eq!(window.get_install_advisories().row_count(), 1);
        assert_eq!(
            window.get_install_advisories().row_data(0).unwrap().vendor,
            "NVIDIA"
        );
    }
    super::install::sync_install_progress(
        &window,
        &crate::operations::install::progress::Progress::new(
            crate::operations::install::progress::Platform::Windows,
        ),
    );
    assert_eq!(window.get_install_advisories().row_count(), 0);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn project_creation_outcome_returns_to_projects_only_after_success() {
    let success = project_creation_outcome(Ok(ProjectCreationResult {
        asset_warning: None,
        path: PathBuf::from("created project"),
        git_warning: None,
    }));
    let warning = project_creation_outcome(Ok(ProjectCreationResult {
        asset_warning: None,
        path: PathBuf::from("created project with warning"),
        git_warning: Some("Git could not be initialized".to_owned()),
    }));
    let failure = project_creation_outcome(create_project(
        "invalid name",
        Path::new("."),
        ProjectOptions::default(),
    ));

    assert!(success.return_to_projects);
    assert!(warning.return_to_projects);
    assert!(!failure.return_to_projects);
    assert_eq!(failure.kind, ResultKind::Error);
}
