use crate::MainWindow;
use crate::community::{CommunityModuleOptions, create_community_module};
use crate::install;
use crate::project::{ProjectOptions, create_project};
use crate::update;
use slint::ComponentHandle;
use std::path::Path;

pub fn run_gui() -> Result<(), Box<dyn std::error::Error>> {
    let window = MainWindow::new()?;
    let destination = default_gui_destination().display().to_string();
    #[cfg(target_os = "macos")]
    window.set_install_available(true);
    #[cfg(not(target_os = "macos"))]
    window.set_install_available(false);
    window.set_destination(destination.into());

    let weak_window = window.as_weak();
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

    let weak_window_for_install = window.as_weak();
    window.on_install_mayaflux(move |channel| {
        let Some(window) = weak_window_for_install.upgrade() else {
            return;
        };
        start_mayaflux_install(&window, channel.to_string(), false);
    });

    let weak_window_for_replace = window.as_weak();
    window.on_confirm_install_replacement(move || {
        let Some(window) = weak_window_for_replace.upgrade() else {
            return;
        };
        let channel = window.get_pending_install_channel().to_string();
        window.set_conflict_confirmation_open(false);
        start_mayaflux_install(&window, channel, true);
    });

    let weak_window_for_cancel_install = window.as_weak();
    window.on_cancel_install_replacement(move || {
        let Some(window) = weak_window_for_cancel_install.upgrade() else {
            return;
        };
        window.set_conflict_confirmation_open(false);
        window.set_result_kind(1);
        window.set_result_title("Installation cancelled".into());
        let previous = window.get_result_message().to_string();
        window.set_result_message(format!("{previous}\n\nNo MayaFlux formula was changed.").into());
        window.set_result_open(true);
    });

    window.on_create_project(move || {
        let Some(window) = weak_window.upgrade() else {
            return;
        };
        let name = window.get_project_name().to_string();
        let destination = window.get_destination().to_string();
        let options = ProjectOptions {
            with_lila: window.get_with_lila(),
            with_vscode: window.get_with_vscode(),
            copy_shader_sources: window.get_copy_shader_sources(),
        };
        let (kind, title, message) = match create_project(&name, Path::new(&destination), options) {
            Ok(result) => match result.git_warning {
                Some(warning) => (
                    1,
                    "Project created with a warning",
                    format!("Created at:\n{}\n\n{warning}", result.path.display()),
                ),
                None => (
                    0,
                    "Project created",
                    format!("Created at:\n{}", result.path.display()),
                ),
            },
            Err(error) => (2, "Could not create project", error.to_string()),
        };
        publish_result(&window, kind, title, message);
    });

    let weak_window_for_update = window.as_weak();
    window.on_update_project(move || {
        let Some(window) = weak_window_for_update.upgrade() else {
            return;
        };
        let project_path = window.get_destination().to_string();
        let names = window.get_module_names().to_string();
        window.set_busy(true);
        let weak = window.as_weak();
        std::thread::spawn(move || {
            let mut output = Vec::new();
            let result = update::update_project(Path::new(&project_path), &[names], |line| {
                output.push(line)
            });
            let output = output.join("\n");
            let (kind, title, message) = match result {
                Ok(()) => (0, "Project updated", output),
                Err(error) if output.is_empty() => (2, "Project update failed", error.to_string()),
                Err(error) => (2, "Project update failed", format!("{output}\n\n{error}")),
            };
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(window) = weak.upgrade() {
                    window.set_busy(false);
                    publish_result(&window, kind, title, message);
                }
            });
        });
    });

    let weak_window_for_community = window.as_weak();
    window.on_create_community_module(move || {
        let Some(window) = weak_window_for_community.upgrade() else {
            return;
        };
        let name = window.get_module_name().to_string();
        let destination = window.get_destination().to_string();
        let options = CommunityModuleOptions {
            description: window.get_module_description().to_string(),
            min_version: window.get_module_min_version().to_string(),
            needs_lila: window.get_community_needs_lila(),
        };
        let (kind, title, message) =
            match create_community_module(&name, Path::new(&destination), &options) {
                Ok(result) => match result.git_warning {
                    Some(warning) => (
                        1,
                        "Module created with a warning",
                        format!("Created at:\n{}\n\n{warning}", result.path.display()),
                    ),
                    None => (
                        0,
                        "Community module created",
                        format!("Created at:\n{}", result.path.display()),
                    ),
                },
                Err(error) => (2, "Could not create community module", error.to_string()),
            };
        publish_result(&window, kind, title, message);
    });

    window.run()?;
    Ok(())
}

fn start_mayaflux_install(window: &MainWindow, channel: String, replace_conflict: bool) {
    window.set_busy(true);
    window.set_result_busy(true);
    window.set_result_kind(1);
    window.set_result_title("Installing MayaFlux".into());
    window.set_result_message("Starting the MayaFlux installation…".into());
    window.set_result_open(true);
    let weak = window.as_weak();
    std::thread::spawn(move || {
        let log_window = weak.clone();
        let result = install::install_mayaflux(&channel, replace_conflict, move |line| {
            let line = line.to_owned();
            let weak = log_window.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(window) = weak.upgrade() {
                    let previous = window.get_result_message().to_string();
                    window.set_result_message(format!("{previous}\n{line}").into());
                }
            });
        });
        match result {
            Err(install::InstallError::Conflict { installed_formula }) => {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(window) = weak.upgrade() {
                        window.set_busy(false);
                        window.set_result_busy(false);
                        window.set_result_open(false);
                        window.set_pending_install_channel(channel.into());
                        window.set_conflict_confirmation_title(
                            format!("{installed_formula} is already installed").into(),
                        );
                        window.set_conflict_confirmation_message(
                            format!(
                                "Installing this edition requires removing {installed_formula}. Replace it and continue?"
                            )
                            .into(),
                        );
                        window.set_conflict_confirmation_open(true);
                    }
                });
            }
            result => {
                let (kind, title, message) = match result {
                    Ok(message) => (0, "MayaFlux installed", message),
                    Err(error) => (2, "MayaFlux installation failed", error.to_string()),
                };
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(window) = weak.upgrade() {
                        window.set_busy(false);
                        window.set_result_busy(false);
                        window.set_result_kind(kind);
                        window.set_result_title(title.into());
                        let previous = window.get_result_message().to_string();
                        window.set_result_message(format!("{previous}\n\n{message}").into());
                        window.set_result_open(true);
                    }
                });
            }
        }
    });
}

fn publish_result(window: &MainWindow, kind: i32, title: &str, message: String) {
    window.set_result_busy(false);
    window.set_result_kind(kind);
    window.set_result_title(title.into());
    window.set_result_message(message.into());
    window.set_result_open(true);
}

fn default_gui_destination() -> std::path::PathBuf {
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
