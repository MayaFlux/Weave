//! Application state shared by GUI callbacks and operation workers.

use crate::operations::project_refresh::RefreshPreview;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Work that temporarily owns the GUI's operation slot.
/// Only validation and Run & Report accept cancellation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum OperationKind {
    Install,
    CreateProject,
    CreateCommunityModule,
    AddModules,
    InspectOptions,
    ApplyOptions,
    PreviewRefresh,
    ApplyRefresh,
    Validation,
    RunReport,
}

/// Identifies one worker and carries its cancellation signal.
/// The ID prevents late updates from changing a newer operation's view.
#[derive(Clone)]
pub(super) struct OperationTicket {
    pub id: u64,
    pub cancellation: Arc<AtomicBool>,
    pub stop_requested: Arc<AtomicBool>,
}

struct ActiveOperation {
    id: u64,
    kind: OperationKind,
    cancellation: Arc<AtomicBool>,
    stop_requested: Arc<AtomicBool>,
}

/// Owns the selected project, its refresh preview, and one active operation.
/// All Manage Project actions read the same selected path.
#[derive(Default)]
pub(super) struct GuiState {
    selected_project: Option<PathBuf>,
    pending_refresh: Option<RefreshPreview>,
    active: Option<ActiveOperation>,
    next_operation_id: u64,
}

impl GuiState {
    /// Claims the operation slot, or returns `None` while it is occupied.
    pub fn begin(&mut self, kind: OperationKind) -> Option<OperationTicket> {
        if self.active.is_some() {
            return None;
        }

        self.next_operation_id = self.next_operation_id.wrapping_add(1);
        let ticket = OperationTicket {
            id: self.next_operation_id,
            cancellation: Arc::new(AtomicBool::new(false)),
            stop_requested: Arc::new(AtomicBool::new(false)),
        };

        self.active = Some(ActiveOperation {
            id: ticket.id,
            kind,
            cancellation: ticket.cancellation.clone(),
            stop_requested: ticket.stop_requested.clone(),
        });
        Some(ticket)
    }

    pub fn is_current(&self, id: u64) -> bool {
        self.active
            .as_ref()
            .is_some_and(|operation| operation.id == id)
    }

    /// Releases the slot only for the worker that currently owns it.
    pub fn finish(&mut self, id: u64) -> bool {
        if !self.is_current(id) {
            return false;
        }

        self.active = None;
        true
    }

    /// Signals cancellation only for a running project process.
    pub fn cancel_active(&self) -> bool {
        if let Some(operation) = &self.active
            && matches!(
                operation.kind,
                OperationKind::Validation | OperationKind::RunReport
            )
        {
            operation.cancellation.store(true, Ordering::Relaxed);
            return true;
        }

        false
    }

    /// Requests a graceful stop while leaving report collection active.
    pub fn stop_active_run(&self) -> bool {
        if let Some(operation) = &self.active
            && operation.kind == OperationKind::RunReport
            && !operation.cancellation.load(Ordering::Relaxed)
        {
            return !operation.stop_requested.swap(true, Ordering::Relaxed);
        }

        false
    }

    /// Changes the managed project and drops a preview from the old project.
    pub fn select_project(&mut self, project: PathBuf) {
        self.selected_project = Some(project);
        self.pending_refresh = None;
    }

    pub fn selected_project(&self) -> Option<&Path> {
        self.selected_project.as_deref()
    }

    pub fn store_refresh_preview(&mut self, preview: RefreshPreview) {
        self.pending_refresh = Some(preview);
    }

    pub fn refresh_preview(&self) -> Option<&RefreshPreview> {
        self.pending_refresh.as_ref()
    }

    pub fn clear_refresh_preview(&mut self) {
        self.pending_refresh = None;
    }
}
