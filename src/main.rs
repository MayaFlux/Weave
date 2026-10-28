mod project;
mod platform;
mod community;
mod git;
mod update;

use community::{CommunityModuleOptions, create_community_module};
use project::{ProjectOptions, create_project};
use slint::ComponentHandle;
use std::path::Path;
use std::process::ExitCode;

slint::slint! {
    import { CheckBox, HorizontalBox, LineEdit, VerticalBox } from "std-widgets.slint";

    component ActionCard inherits Rectangle {
        in property <string> eyebrow;
        in property <string> heading;
        in property <string> detail;
        in property <color> accent: #66b3ff;
        callback clicked;

        height: 104px;
        border-radius: 6px;
        border-width: 1px;
        border-color: #2a2a2a;
        background: #181818;

        Rectangle {
            x: 0px;
            y: 18px;
            width: 4px;
            height: parent.height - 36px;
            border-radius: 2px;
            background: root.accent;
        }

        VerticalBox {
            x: 24px;
            y: 18px;
            width: parent.width - 76px;
            height: parent.height - 36px;
            spacing: 3px;
            Text { text: root.eyebrow; color: #a7a7a7; font-size: 10px; font-weight: 600; }
            Text { text: root.heading; color: #a0c8aa; font-size: 17px; font-weight: 600; }
            Text { text: root.detail; color: #a7a7a7; font-size: 12px; wrap: word-wrap; }
        }

        Text {
            x: parent.width - 48px;
            y: (parent.height - self.height) / 2;
            text: "→";
            color: root.accent;
            font-size: 25px;
        }

        touch-area := TouchArea { clicked => { root.clicked(); } }

        states [
            hovered when touch-area.has-hover: {
                background: #202020;
                border-color: root.accent;
            }
            pressed when touch-area.pressed: {
                opacity: 0.78;
            }
        ]
    }

    component AppButton inherits Rectangle {
        in property <string> text;
        in property <bool> primary: false;
        in property <bool> enabled: true;
        in property <color> accent: #66b3ff;
        callback clicked;

        height: 42px;
        min-width: 116px;
        border-radius: 4px;
        background: root.primary ? root.accent : #181818;
        border-width: root.primary ? 0px : 1px;
        border-color: #2a2a2a;
        opacity: root.enabled ? 1 : 0.48;

        Text {
            x: 18px;
            y: (parent.height - self.height) / 2;
            width: parent.width - 36px;
            text: root.text;
            color: root.primary ? #111111 : #f4f2ef;
            font-size: 14px;
            font-weight: 650;
            horizontal-alignment: center;
        }

        touch-area := TouchArea {
            enabled: root.enabled;
            clicked => { root.clicked(); }
        }

        states [
            hovered when touch-area.has-hover && root.enabled: {
                background: root.primary ? root.accent.darker(0.08) : #202020;
                border-color: root.primary ? root.accent : #777777;
            }
            pressed when touch-area.pressed && root.enabled: {
                opacity: 0.78;
            }
        ]
    }

    export component MainWindow inherits Window {
        title: "Weave";
        width: 900px;
        height: 760px;
        default-font-family: "monospace";
        background: #111111;

        in-out property <string> project_name: "MyProject";
        in-out property <string> module_name: "my_module";
        in-out property <string> module_description: "";
        in-out property <string> module_min_version: "0.5.0";
        in-out property <string> destination: "";
        in-out property <int> screen: 0;
        in-out property <string> module_names: "";
        in-out property <bool> result_open: false;
        in-out property <int> result_kind: 0;
        in-out property <string> result_title: "";
        in-out property <string> result_message: "";
        in-out property <bool> busy: false;
        in-out property <bool> community_needs_lila: false;
        in-out property <bool> with_lila: false;
        in-out property <bool> with_vscode: true;
        in-out property <bool> copy_shader_sources: false;
        callback choose_destination();
        callback create_project();
        callback create_community_module();
        callback update_project();

        Rectangle {
            width: 100%;
            height: 100%;
            background: #111111;

            VerticalBox {
                padding: 30px;
                spacing: 18px;

                HorizontalBox {
                    height: 48px;
                    spacing: 12px;
                    VerticalBox {
                        spacing: 2px;
                        Text { text: "MAYAFLUX"; color: #a0c8aa; font-size: 11px; font-weight: 700; }
                        Text { text: "WEAVE  /  PROJECT STUDIO"; color: #f4f2ef; font-size: 14px; font-weight: 650; }
                    }
                    Rectangle { horizontal-stretch: 1; background: transparent; }
                    Text { text: "0.6"; color: #a7a7a7; font-size: 11px; vertical-alignment: center; }
                }

                if root.screen == 0: VerticalBox {
                    spacing: 16px;
                    Text { text: "Projects"; color: #a0c8aa; font-size: 31px; font-weight: 650; }
                    Text { text: "Create and maintain portable MayaFlux projects and community modules."; color: #a7a7a7; font-size: 14px; wrap: word-wrap; }
                    Rectangle { height: 8px; background: transparent; }
                    ActionCard {
                        eyebrow: "START SOMETHING NEW";
                        heading: "Create Project";
                        detail: "Generate a ready-to-build CMake project with your preferred tools and options.";
                        accent: #66b3ff;
                        clicked => { root.screen = 1; }
                    }
                    ActionCard {
                        eyebrow: "COMMUNITY MODULES";
                        heading: "Update Project";
                        detail: "Add and validate community modules in an existing MayaFlux project.";
                        accent: #66b3ff;
                        clicked => { root.screen = 2; }
                    }
                    ActionCard {
                        eyebrow: "SHARE YOUR WORK";
                        heading: "Create Community Module";
                        detail: "Scaffold a portable module with a manifest, CMake setup, and tests.";
                        accent: #66b3ff;
                        clicked => { root.screen = 3; }
                    }
                    Rectangle { vertical-stretch: 1; background: transparent; }
                    Text { text: "Projects remain ordinary CMake directories and can be opened in any editor."; color: #777777; font-size: 11px; wrap: word-wrap; }
                }

                if root.screen == 1: VerticalBox {
                    spacing: 14px;
                    AppButton { text: "←  Projects"; clicked => { root.screen = 0; } }
                    Text { text: "Create a project"; color: #a0c8aa; font-size: 28px; font-weight: 650; }
                    Text { text: "Choose a name and location, then select the project tools to include."; color: #a7a7a7; font-size: 13px; }
                    Rectangle {
                        height: 360px;
                        background: #181818;
                        border-width: 1px;
                        border-color: #2a2a2a;
                        border-radius: 6px;
                        VerticalBox {
                            x: 24px; y: 20px; width: parent.width - 48px; height: parent.height - 40px; spacing: 10px;
                            Text { text: "PROJECT DETAILS"; color: #a0c8aa; font-size: 10px; font-weight: 700; }
                            Text { text: "Project name"; color: #a7a7a7; font-size: 11px; }
                            LineEdit { text <=> root.project_name; placeholder-text: "For example, AudioDemo"; }
                            Text { text: "Create in"; color: #a7a7a7; font-size: 11px; }
                            HorizontalBox {
                                spacing: 12px;
                                Text { text: root.destination; color: #f4f2ef; vertical-alignment: center; overflow: elide; }
                                AppButton { text: "Choose folder"; clicked => { root.choose_destination(); } }
                            }
                            Text { text: "PROJECT OPTIONS"; color: #a0c8aa; font-size: 10px; font-weight: 700; }
                            HorizontalBox {
                                spacing: 18px;
                                CheckBox { text: "Live coding (Lila)"; checked <=> root.with_lila; }
                                CheckBox { text: "VS Code setup"; checked <=> root.with_vscode; }
                            }
                            CheckBox { text: "Copy MayaFlux shader sources"; checked <=> root.copy_shader_sources; }
                        }
                    }
                    HorizontalBox {
                        AppButton { text: "Back"; clicked => { root.screen = 0; } }
                        Rectangle { horizontal-stretch: 1; background: transparent; }
                        AppButton { text: "Create project"; primary: true; clicked => { root.create_project(); } }
                    }
                }

                if root.screen == 2: VerticalBox {
                    spacing: 14px;
                    AppButton { text: "←  Projects"; clicked => { root.screen = 0; } }
                    Text { text: "Update a project"; color: #a0c8aa; font-size: 28px; font-weight: 650; }
                    Text { text: "Select a project and enter one or more community module names."; color: #a7a7a7; font-size: 13px; }
                    Rectangle {
                        height: 250px;
                        background: #181818;
                        border-width: 1px;
                        border-color: #2a2a2a;
                        border-radius: 6px;
                        VerticalBox {
                            x: 24px; y: 22px; width: parent.width - 48px; height: parent.height - 44px; spacing: 12px;
                            Text { text: "PROJECT FOLDER"; color: #a0c8aa; font-size: 10px; font-weight: 700; }
                            HorizontalBox {
                                spacing: 12px;
                                Text { text: root.destination; color: #f4f2ef; vertical-alignment: center; overflow: elide; }
                                AppButton { text: "Choose project"; clicked => { root.choose_destination(); } }
                            }
                            Text { text: "COMMUNITY MODULES"; color: #a0c8aa; font-size: 10px; font-weight: 700; }
                            LineEdit { text <=> root.module_names; placeholder-text: "terrain_field, audio_tools"; }
                            Text { text: "Weave checks registry and MayaFlux compatibility, validates module files, then registers the modules in community.cmake."; color: #a7a7a7; font-size: 12px; wrap: word-wrap; }
                        }
                    }
                    HorizontalBox {
                        AppButton { text: "Back"; clicked => { root.screen = 0; } }
                        Rectangle { horizontal-stretch: 1; background: transparent; }
                        AppButton { text: root.busy ? "Updating…" : "Update project"; primary: true; enabled: !root.busy; clicked => { root.update_project(); } }
                    }
                }

                if root.screen == 3: VerticalBox {
                    spacing: 14px;
                    AppButton { text: "←  Projects"; clicked => { root.screen = 0; } }
                    Text { text: "Create a community module"; color: #a0c8aa; font-size: 28px; font-weight: 650; }
                    Text { text: "Set up a reusable module with its manifest and standalone CMake test project."; color: #a7a7a7; font-size: 13px; }
                    Rectangle {
                        height: 420px;
                        background: #181818;
                        border-width: 1px;
                        border-color: #2a2a2a;
                        border-radius: 6px;
                        VerticalBox {
                            x: 24px; y: 20px; width: parent.width - 48px; height: parent.height - 40px; spacing: 9px;
                            Text { text: "MODULE DETAILS"; color: #a0c8aa; font-size: 10px; font-weight: 700; }
                            Text { text: "Module name"; color: #a7a7a7; font-size: 11px; }
                            LineEdit { text <=> root.module_name; placeholder-text: "snake_case name"; }
                            Text { text: "Description"; color: #a7a7a7; font-size: 11px; }
                            LineEdit { text <=> root.module_description; placeholder-text: "What does this module do?"; }
                            Text { text: "Minimum MayaFlux version"; color: #a7a7a7; font-size: 11px; }
                            LineEdit { text <=> root.module_min_version; placeholder-text: "0.5.0"; }
                            Text { text: "Create in"; color: #a7a7a7; font-size: 11px; }
                            HorizontalBox {
                                spacing: 12px;
                                Text { text: root.destination; color: #f4f2ef; vertical-alignment: center; overflow: elide; }
                                AppButton { text: "Choose folder"; clicked => { root.choose_destination(); } }
                            }
                            CheckBox { text: "Requires Lila live coding"; checked <=> root.community_needs_lila; }
                        }
                    }
                    HorizontalBox {
                        AppButton { text: "Back"; clicked => { root.screen = 0; } }
                        Rectangle { horizontal-stretch: 1; background: transparent; }
                        AppButton { text: "Create module"; primary: true; clicked => { root.create_community_module(); } }
                    }
                }
            }

            if root.result_open: Rectangle {
                x: 0px;
                y: 0px;
                width: 100%;
                height: 100%;
                z: 100;
                background: #080808.with-alpha(0.78);
                TouchArea { }
                Rectangle {
                    x: (parent.width - self.width) / 2;
                    y: (parent.height - self.height) / 2;
                    width: 520px;
                    height: 270px;
                    border-radius: 6px;
                    border-width: 1px;
                    border-color: #383838;
                    background: #181818;
                    VerticalBox {
                        x: 28px; y: 26px; width: parent.width - 56px; height: parent.height - 52px; spacing: 12px;
                        HorizontalBox {
                            spacing: 14px;
                            Rectangle {
                                width: 34px; height: 34px; border-radius: 4px;
                                background: root.result_kind == 2 ? #6b3838 : root.result_kind == 1 ? #5c4828 : #35493a;
                                Text {
                                    text: root.result_kind == 2 ? "!" : root.result_kind == 1 ? "!" : "✓";
                                    color: root.result_kind == 2 ? #e08a8a : root.result_kind == 1 ? #d5ad68 : #a0c8aa; font-size: 18px; font-weight: 700;
                                    horizontal-alignment: center; vertical-alignment: center;
                                }
                            }
                            VerticalBox {
                                spacing: 2px;
                                Text { text: root.result_kind == 2 ? "FAILED" : root.result_kind == 1 ? "COMPLETED WITH WARNING" : "COMPLETED"; color: root.result_kind == 2 ? #e08a8a : root.result_kind == 1 ? #d5ad68 : #a0c8aa; font-size: 10px; font-weight: 700; }
                                Text { text: root.result_title; color: #f4f2ef; font-size: 19px; font-weight: 650; wrap: word-wrap; }
                            }
                        }
                        Rectangle { height: 1px; background: #2a2a2a; }
                        Text { text: root.result_message; color: #c5c3bf; font-size: 12px; wrap: word-wrap; vertical-stretch: 1; }
                        HorizontalBox {
                            Rectangle { horizontal-stretch: 1; background: transparent; }
                            AppButton { text: "Close"; primary: true; clicked => { root.result_open = false; } }
                        }
                    }
                }
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

    if arguments[0] == "update" {
        return match run_update(&arguments[1..]) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => { eprintln!("[Weave ERROR] {error}"); ExitCode::FAILURE }
        };
    }

    if arguments[0] != "new" {
        eprintln!("Unknown command: {}", arguments[0]);
        print_usage();
        return ExitCode::FAILURE;
    }

    match run_new(&arguments[1..]) {
        Ok(result) => {
            println!("Project created: {}", result.path.display());
            if let Some(warning) = result.git_warning { eprintln!("[Weave WARNING] {warning}"); }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("[Weave ERROR] {error}");
            ExitCode::FAILURE
        }
    }
}

fn run_new(arguments: &[String]) -> Result<project::ProjectCreationResult, String> {
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

fn run_update(arguments: &[String]) -> Result<(), String> {
    if arguments.len() < 2 {
        return Err("Usage: weave update <project-dir> <module-name>[,<module-name>...]".to_owned());
    }
    update::update_project(Path::new(&arguments[0]), &arguments[1..], |message| println!("{message}"))
        .map_err(|error| error.to_string())
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
        let (kind, title, message) = match create_project(&name, Path::new(&destination), options) {
            Ok(result) => match result.git_warning {
                Some(warning) => (1, "Project created with a warning", format!("Created at:\n{}\n\n{warning}", result.path.display())),
                None => (0, "Project created", format!("Created at:\n{}", result.path.display())),
            },
            Err(error) => (2, "Could not create project", error.to_string()),
        };
        publish_result(&window, kind, title, message);
    });

    let weak_window_for_update = window.as_weak();
    window.on_update_project(move || {
        let Some(window) = weak_window_for_update.upgrade() else { return; };
        let project_path = window.get_destination().to_string();
        let names = window.get_module_names().to_string();
        window.set_busy(true);
        let weak = window.as_weak();
        std::thread::spawn(move || {
            let mut output = Vec::new();
            let result = update::update_project(Path::new(&project_path), &[names], |line| output.push(line));
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
        let (kind, title, message) = match create_community_module(&name, Path::new(&destination), &options) {
            Ok(result) => match result.git_warning {
                Some(warning) => (1, "Module created with a warning", format!("Created at:\n{}\n\n{warning}", result.path.display())),
                None => (0, "Community module created", format!("Created at:\n{}", result.path.display())),
            },
            Err(error) => (2, "Could not create community module", error.to_string()),
        };
        publish_result(&window, kind, title, message);
    });

    window.run()?;
    Ok(())
}

fn publish_result(window: &MainWindow, kind: i32, title: &str, message: String) {
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

fn print_usage() {
    println!("Weave - MayaFlux project tool\n\nUsage:\n  weave                 Open the Projects screen\n  weave new <name> [destination] [options]\n  weave update <project-dir> <module-name>[,<module-name>...]\n  weave community <module-name> [destination-dir] [options]\n  weave --help\n\nOptions for 'new':\n  --with-lila             Enable Live Coding\n  --no-vscode             Skip VS Code configuration\n  --copy-shader-sources   Copy installed MayaFlux shader sources\n\nOptions for 'community':\n  --description <text>    Module description\n  --min-version <version> Minimum MayaFlux version (default 0.5.0)\n  --with-lila             Mark the module as requiring Lila");
}
