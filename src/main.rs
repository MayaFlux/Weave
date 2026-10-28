mod project;
mod platform;
mod community;

use community::{CommunityModuleOptions, create_community_module};
use project::{ProjectOptions, create_project};
use slint::ComponentHandle;
use std::path::Path;
use std::process::ExitCode;

slint::slint! {
    import { Button, CheckBox, HorizontalBox, LineEdit, VerticalBox } from "std-widgets.slint";

    export component MainWindow inherits Window {
        title: "Weave";
        width: 720px;
        height: 620px;

        in-out property <string> project_name: "MyProject";
        in-out property <string> module_name: "my_module";
        in-out property <string> module_description: "";
        in-out property <string> module_min_version: "0.5.0";
        in-out property <string> destination: "";
        in-out property <string> status: "Ready";
        in-out property <bool> community_mode: false;
        in-out property <bool> community_needs_lila: false;
        in-out property <bool> with_lila: false;
        in-out property <bool> with_vscode: true;
        in-out property <bool> copy_shader_sources: false;
        callback choose_destination();
        callback create_project();
        callback create_community_module();

        VerticalBox {
            padding: 24px;
            spacing: 12px;

            HorizontalBox {
                Button {
                    text: "Project";
                    clicked => { root.community_mode = false; }
                }
                Button {
                    text: "Community Module";
                    clicked => { root.community_mode = true; }
                }
            }

            if root.community_mode: VerticalBox {
                spacing: 12px;
                Text {
                    text: "Create a MayaFlux community module";
                    font-size: 24px;
                }

                LineEdit {
                    text <=> root.module_name;
                    placeholder-text: "Module name (snake_case)";
                }

                LineEdit {
                    text <=> root.module_description;
                    placeholder-text: "Description";
                }

                LineEdit {
                    text <=> root.module_min_version;
                    placeholder-text: "Minimum MayaFlux version (0.5.0)";
                }

                Text {
                    text: "Module will be created in the selected location as " + root.module_name;
                    wrap: word-wrap;
                }

                CheckBox {
                    text: "Requires Lila (live coding)";
                    checked <=> root.community_needs_lila;
                }

                HorizontalBox {
                    spacing: 8px;
                    Text {
                        text: root.destination;
                        vertical-alignment: center;
                        overflow: elide;
                    }
                    Button {
                        text: "Choose Location…";
                        clicked => { root.choose_destination(); }
                    }
                }

                Button {
                    text: "Create Community Module";
                    clicked => { root.create_community_module(); }
                }
            }

            if !root.community_mode: VerticalBox {
                spacing: 12px;
                Text {
                    text: "Create a MayaFlux project";
                    font-size: 24px;
                }

                LineEdit {
                    text <=> root.project_name;
                    placeholder-text: "Project name";
                }

                HorizontalBox {
                    spacing: 8px;
                    Text {
                        text: root.destination;
                        vertical-alignment: center;
                        overflow: elide;
                    }
                    Button {
                        text: "Choose Location…";
                        clicked => { root.choose_destination(); }
                    }
                }

                Text {
                    text: "Project will be created at " + root.destination + "/" + root.project_name;
                    wrap: word-wrap;
                }

                CheckBox {
                    text: "Enable Live Coding (Lila)";
                    checked <=> root.with_lila;
                }

                CheckBox {
                    text: "Create VS Code configuration";
                    checked <=> root.with_vscode;
                }

                CheckBox {
                    text: "Copy MayaFlux shader sources";
                    checked <=> root.copy_shader_sources;
                }

                Button {
                    text: "Create Project";
                    clicked => { root.create_project(); }
                }
            }

            Text {
                text: root.status;
                wrap: word-wrap;
            }
        }
    }
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments.is_empty() {
        return match run_gui() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("Weave could not start: {error}");
                ExitCode::FAILURE
            }
        };
    }

    if arguments[0] == "--help" || arguments[0] == "-h" {
        print_usage();
        return ExitCode::SUCCESS;
    }

    if arguments[0] == "community" {
        return match run_community(&arguments[1..]) {
            Ok(result) => {
                println!("Community module created: {}", result.path.display());
                if let Some(warning) = result.git_warning {
                    eprintln!("[Weave WARNING] {warning}");
                }
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("[Weave ERROR] {error}");
                ExitCode::FAILURE
            }
        };
    }

    if arguments[0] != "new" {
        eprintln!("Unknown command: {}", arguments[0]);
        print_usage();
        return ExitCode::FAILURE;
    }

    match run_new(&arguments[1..]) {
        Ok(path) => {
            println!("Project created: {}", path.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("[Weave ERROR] {error}");
            ExitCode::FAILURE
        }
    }
}

fn run_new(arguments: &[String]) -> Result<std::path::PathBuf, String> {
    let mut name = None;
    let mut destination = None;
    let mut options = ProjectOptions::default();

    for argument in arguments {
        match argument.as_str() {
            "--with-lila" => options.with_lila = true,
            "--no-vscode" => options.with_vscode = false,
            "--copy-shader-sources" => options.copy_shader_sources = true,
            value if value.starts_with('-') => return Err(format!("Unknown option: {value}")),
            value if name.is_none() => name = Some(value),
            value if destination.is_none() => destination = Some(value),
            value => return Err(format!("Unexpected argument: {value}")),
        }
    }

    let name = name.ok_or_else(|| "Project name required: weave new <name> [destination] [options]".to_owned())?;
    let destination = match destination {
        Some(path) => Path::new(path).to_path_buf(),
        None => std::env::current_dir().map_err(|error| error.to_string())?,
    };
    create_project(name, &destination, options).map_err(|error| error.to_string())
}

fn run_community(arguments: &[String]) -> Result<community::CommunityCreationResult, String> {
    let mut name = None;
    let mut destination = None;
    let mut options = CommunityModuleOptions::default();
    let mut arguments = arguments.iter();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--with-lila" => options.needs_lila = true,
            "--description" => {
                options.description = arguments
                    .next()
                    .ok_or_else(|| "Missing value for --description".to_owned())?
                    .to_string();
            }
            "--min-version" => {
                options.min_version = arguments
                    .next()
                    .ok_or_else(|| "Missing value for --min-version".to_owned())?
                    .to_string();
            }
            value if value.starts_with('-') => return Err(format!("Unknown option: {value}")),
            value if name.is_none() => name = Some(value),
            value if destination.is_none() => destination = Some(value),
            value => return Err(format!("Unexpected argument: {value}")),
        }
    }
    let name = name.ok_or_else(|| {
        "Module name required: weave community <module-name> [destination-dir] [options]".to_owned()
    })?;
    let destination = match destination {
        Some(path) => Path::new(path).to_path_buf(),
        None => std::env::current_dir().map_err(|error| error.to_string())?,
    };
    create_community_module(name, &destination, &options).map_err(|error| error.to_string())
}

fn run_gui() -> Result<(), Box<dyn std::error::Error>> {
    let window = MainWindow::new()?;
    let destination = default_gui_destination().display().to_string();
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
        let status = match create_project(&name, Path::new(&destination), options) {
            Ok(path) => format!("Project created: {}", path.display()),
            Err(error) => format!("Could not create project: {error}"),
        };
        window.set_status(status.into());
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
        let status = match create_community_module(&name, Path::new(&destination), &options) {
            Ok(result) => {
                let message = format!("Community module created: {}", result.path.display());
                match result.git_warning {
                    Some(warning) => format!("{message}\n{warning}"),
                    None => message,
                }
            }
            Err(error) => format!("Could not create community module: {error}"),
        };
        window.set_status(status.into());
    });

    window.run()?;
    Ok(())
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

fn print_usage() {
    println!("Weave - MayaFlux project tool\n\nUsage:\n  weave                 Open the graphical creator\n  weave new <name> [destination] [options]\n  weave community <module-name> [destination-dir] [options]\n  weave --help\n\nOptions for 'new':\n  --with-lila             Enable Live Coding\n  --no-vscode             Skip VS Code configuration\n  --copy-shader-sources   Copy installed MayaFlux shader sources\n\nOptions for 'community':\n  --description <text>    Module description\n  --min-version <version> Minimum MayaFlux version (default 0.5.0)\n  --with-lila             Mark the module as requiring Lila");
}
