//! Startup release notice and external release links.

use crate::MainWindow;
use crate::operations::weave_release;
use slint::ComponentHandle;

/// Checks for a release and connects the notice's view and dismiss actions.
/// The startup check runs off the GUI thread.
pub(super) fn register(window: &MainWindow) {
    let weak_window_for_update = window.as_weak();
    std::thread::spawn(move || {
        if let Some(notice) = weave_release::check_on_startup(env!("CARGO_PKG_VERSION")) {
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(window) = weak_window_for_update.upgrade() {
                    show_update_notice(&window, notice);
                }
            });
        }
    });

    let weak_window_for_view_update = window.as_weak();
    window.on_view_update(move || {
        if let Some(window) = weak_window_for_view_update.upgrade() {
            let url = window.get_update_url().to_string();
            std::thread::spawn(move || {
                let _ = webbrowser::open(&url);
            });
        }
    });
    let weak_window_for_dismiss_update = window.as_weak();
    window.on_dismiss_update(move || {
        if let Some(window) = weak_window_for_dismiss_update.upgrade() {
            let version = window.get_update_version().to_string();
            window.set_update_available(false);
            weave_release::dismiss(&version);
        }
    });
}

pub(super) fn show_update_notice(window: &MainWindow, notice: weave_release::ReleaseNotice) {
    window.set_update_version(notice.version.into());
    window.set_update_url(notice.url.into());
    window.set_update_available(true);
}
