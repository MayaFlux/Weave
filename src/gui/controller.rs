//! Coordinates the Slint window and work running outside its event loop.

use super::state::{GuiState, OperationKind, OperationTicket};
use crate::MainWindow;
use crate::operations::project_refresh::RefreshPreview;
use slint::ComponentHandle;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Owns GUI state and a weak window handle shared by callback groups.
/// Workers use it to deliver progress without keeping the window alive.
#[derive(Clone)]
pub(super) struct GuiController {
    window: slint::Weak<MainWindow>,
    state: Arc<Mutex<GuiState>>,
}

impl GuiController {
    /// Creates the controller for one window instance.
    pub fn new(window: &MainWindow) -> Self {
        Self {
            window: window.as_weak(),
            state: Arc::new(Mutex::new(GuiState::default())),
        }
    }

    /// Runs a callback on the GUI thread while the window exists.
    pub fn with_window(&self, action: impl FnOnce(&MainWindow)) {
        if let Some(window) = self.window.upgrade() {
            action(&window);
        }
    }

    /// Starts a worker only when no other operation owns the window.
    pub fn begin(&self, kind: OperationKind) -> Option<OperationTicket> {
        let ticket = self.state.lock().unwrap().begin(kind)?;
        self.with_window(|window| window.set_busy(true));
        Some(ticket)
    }

    /// Delivers progress on the GUI thread if this worker still owns the slot.
    /// Late messages from a finished worker are ignored.
    pub fn post(&self, id: u64, action: impl FnOnce(&MainWindow) + Send + 'static) {
        let controller = self.clone();
        let _ = slint::invoke_from_event_loop(move || {
            let current = controller.state.lock().unwrap().is_current(id);
            if current {
                controller.with_window(action);
            }
        });
    }

    /// Releases the matching worker's slot and returns the GUI to idle.
    /// A stale completion cannot alter the next operation's view.
    pub fn complete(&self, id: u64, action: impl FnOnce(&MainWindow) + Send + 'static) {
        let controller = self.clone();
        let _ = slint::invoke_from_event_loop(move || {
            let finished = controller.state.lock().unwrap().finish(id);
            if finished {
                controller.with_window(|window| {
                    window.set_busy(false);
                    action(window);
                });
            }
        });
    }

    /// Sets the project used by every Manage Project action.
    pub fn select_project(&self, project: PathBuf) {
        self.state.lock().unwrap().select_project(project);
    }

    pub fn selected_project(&self) -> Option<PathBuf> {
        self.state
            .lock()
            .unwrap()
            .selected_project()
            .map(PathBuf::from)
    }

    pub fn store_refresh_preview(&self, preview: RefreshPreview) {
        self.state.lock().unwrap().store_refresh_preview(preview);
    }

    pub fn refresh_preview(&self) -> Option<RefreshPreview> {
        self.state.lock().unwrap().refresh_preview().cloned()
    }

    pub fn clear_refresh_preview(&self) {
        self.state.lock().unwrap().clear_refresh_preview();
    }

    pub fn cancel_active(&self) -> bool {
        self.state.lock().unwrap().cancel_active()
    }
}
