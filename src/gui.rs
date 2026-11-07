//! Entry point for Weave's Slint desktop UI.

mod controller;
mod create;
mod install;
mod manage;
mod state;
mod update;
mod view;

use crate::MainWindow;
use controller::GuiController;
use slint::ComponentHandle;

/// Creates the desktop window and connects each GUI workflow.
/// The standalone CLI calls shared operations without constructing this window.
pub fn run_gui() -> Result<(), Box<dyn std::error::Error>> {
    let window = MainWindow::new()?;
    window.set_weave_version(env!("CARGO_PKG_VERSION").into());
    window.set_install_available(true);
    #[cfg(target_os = "windows")]
    window.set_windows_install(true);
    #[cfg(target_os = "linux")]
    window.set_linux_install(true);
    window.set_destination(
        create::default_gui_destination()
            .display()
            .to_string()
            .into(),
    );

    let controller = GuiController::new(&window);
    update::register(&window);
    create::register(&window, &controller);
    install::register(&window, &controller);
    manage::register(&window, &controller);
    window.run()?;
    Ok(())
}

#[cfg(test)]
mod tests;
