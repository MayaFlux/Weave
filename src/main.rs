mod cli;
mod community;
mod git;
mod gui;
mod install;
mod platform;
mod project;
mod update;

slint::include_modules!();

use std::process::ExitCode;

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments.is_empty() {
        return match gui::run_gui() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("Weave could not start: {error}");
                ExitCode::FAILURE
            }
        };
    }
    cli::run_cli(&arguments)
}
