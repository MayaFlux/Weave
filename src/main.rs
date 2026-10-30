use std::process::ExitCode;

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments.is_empty() {
        return match weave::gui::run_gui() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("Weave could not start: {error}");
                ExitCode::FAILURE
            }
        };
    }
    weave::cli::run_cli(&arguments)
}
