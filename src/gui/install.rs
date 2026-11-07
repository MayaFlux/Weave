//! Installation input, conflict decisions, and progress presentation.

use super::controller::GuiController;
use super::state::OperationKind;
use super::view::publish_result;
use crate::operations::install;
use crate::operations::install::progress::{Event, Platform, Progress, Status};
use crate::{AppScreen, MainWindow, ResultKind};
use slint::{ComponentHandle, Model};
use std::sync::{Arc, Mutex};

/// Connects installation actions to the shared installer.
/// Password and replacement decisions remain in this GUI flow.
pub(super) fn register(window: &MainWindow, controller: &GuiController) {
    window.on_open_driver_download(|url| {
        if [
            "https://www.nvidia.com/",
            "https://www.amd.com/",
            "https://www.intel.com/",
        ]
        .iter()
        .any(|prefix| url.starts_with(prefix))
        {
            let url = url.to_string();
            std::thread::spawn(move || {
                let _ = webbrowser::open(&url);
            });
        }
    });

    let weak_window_for_install = window.as_weak();
    let install_controller = controller.clone();
    window.on_install_mayaflux(move |channel| {
        let Some(window) = weak_window_for_install.upgrade() else {
            return;
        };

        let replace_conflict = window.get_install_replace_confirmed();
        start_mayaflux_install(
            &install_controller,
            &window,
            channel.to_string(),
            replace_conflict,
        );
    });

    let weak_window_for_retry = window.as_weak();
    #[cfg(not(target_os = "linux"))]
    let retry_controller = controller.clone();
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
            &retry_controller,
            &window,
            window.get_install_channel().to_string(),
            window.get_install_retry_replace_confirmed(),
        );
    });

    #[cfg(target_os = "linux")]
    {
        let prepare_controller = controller.clone();
        window.on_prepare_install_mayaflux(move |channel| {
            prepare_controller.with_window(|window| {
                let Some(ticket) = prepare_controller.begin(OperationKind::Install) else {
                    return;
                };

                let channel = channel.to_string();
                window.set_install_password("".into());
                window.set_install_replace_confirmed(false);

                let worker = prepare_controller.clone();
                std::thread::spawn(move || {
                    let result = install::check_mayaflux_conflict(&channel);
                    worker.complete(ticket.id, move |window| match result {
                        Ok(Some(installed_package)) => {
                            show_install_conflict(window, &channel, &installed_package);
                        }
                        Ok(None) => window.set_password_dialog_open(true),
                        Err(error) => publish_result(
                            window,
                            ResultKind::Error,
                            "Could not check MayaFlux packages",
                            error.to_string(),
                        ),
                    });
                });
            });
        });
    }

    let weak_window_for_replace = window.as_weak();
    #[cfg(not(target_os = "linux"))]
    let replace_controller = controller.clone();
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
        }
        #[cfg(not(target_os = "linux"))]
        start_mayaflux_install(&replace_controller, &window, channel, true);
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
            ResultKind::Warning,
            "Installation cancelled",
            "No MayaFlux package was changed.".to_owned(),
        );
    });
}

/// Starts installation after required credentials and choices are available.
fn start_mayaflux_install(
    controller: &GuiController,
    window: &MainWindow,
    channel: String,
    replace_conflict: bool,
) {
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
            ResultKind::Error,
            "Password required",
            "Enter your sudo password before installing MayaFlux.".to_owned(),
        );
        return;
    }
    let Some(ticket) = controller.begin(OperationKind::Install) else {
        return;
    };

    window.set_install_password("".into());
    window.set_password_dialog_open(false);
    window.set_install_retry_replace_confirmed(replace_conflict);
    window.set_install_replace_confirmed(false);
    window.set_install_channel(channel.clone().into());
    let install_cli_access = window.get_install_cli_access();
    window.set_result_open(false);
    window.set_install_failed(false);
    window.set_install_selected_page(0);
    window.set_install_active_page(0);
    let mut initial = Progress::new(Platform::current());
    initial.receive(Event::Output("Starting MayaFlux installation…".into()));
    sync_install_progress(window, &initial);
    let progress = Arc::new(Mutex::new(initial));
    window.set_install_progress_open(true);

    let worker = controller.clone();
    std::thread::spawn(move || {
        let log_controller = worker.clone();
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
                let progress = log_progress.clone();
                log_controller.post(ticket.id, move |window| {
                    let mut progress = progress.lock().unwrap();
                    progress.receive(event);
                    sync_install_progress(window, &progress);
                });
            },
        );
        unsafe {
            password.as_mut_vec().fill(0);
        }
        let result = result.map(|message| {
            if !install_cli_access {
                return message;
            }
            let cli_message = match crate::operations::cli_access::install() {
                Ok(message) => message,
                Err(error) => format!("Weave CLI setup needs attention: {error}"),
            };
            let log_progress = progress.clone();
            let line = cli_message.clone();
            worker.post(ticket.id, move |window| {
                let mut progress = log_progress.lock().unwrap();
                progress.receive(Event::Output(line));
                sync_install_progress(window, &progress);
            });
            format!("{message}\n\n{cli_message}")
        });
        match result {
            Err(install::InstallError::Conflict { installed_formula }) => {
                worker.complete(ticket.id, move |window| {
                    window.set_install_progress_open(false);
                    show_install_conflict(window, &channel, &installed_formula);
                    let progress = progress.lock().unwrap();
                    let mut message = window.get_conflict_confirmation_message().to_string();
                    for notice in &progress.advisories {
                        message.push_str(&format!("\n\n{}\n{}", notice.message, notice.url));
                    }
                    window.set_conflict_confirmation_message(message.into());
                });
            }
            result => {
                let failed = result.is_err();
                worker.complete(ticket.id, move |window| {
                    window.set_install_password("".into());
                    if !failed {
                        window.set_install_retry_replace_confirmed(false);
                        window.set_screen(AppScreen::Landing);
                    }
                    let mut progress = progress.lock().unwrap();
                    progress.finish(result.map_err(|error| error.to_string()));
                    window.set_install_failed(failed);
                    sync_install_progress(window, &progress);
                    window.set_install_selected_page(progress.active as i32);
                });
            }
        }
    });
}

/// Updates visible steps while preserving the page the user selected.
pub(super) fn sync_install_progress(window: &MainWindow, progress: &Progress) {
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
    let advisories: Vec<_> = progress
        .advisories
        .iter()
        .map(|notice| crate::InstallationAdvisory {
            vendor: notice.vendor.clone().into(),
            message: notice.message.clone().into(),
            url: notice.url.clone().into(),
        })
        .collect();
    window.set_install_advisories(std::rc::Rc::new(slint::VecModel::from(advisories)).into());
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
