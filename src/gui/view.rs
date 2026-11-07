//! Shared result presentation for GUI operations.

use crate::{MainWindow, ResultKind};

/// Shows a completed operation in the shared result dialog.
/// The kind controls its success, warning, or error styling.
pub(super) fn publish_result(window: &MainWindow, kind: ResultKind, title: &str, message: String) {
    window.set_result_busy(false);
    window.set_result_kind(kind);
    window.set_result_title(title.into());
    window.set_result_message(message.into());
    window.set_result_open(true);
}
