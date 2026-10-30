use crate::operations::community::{
    CommunityCreationResult, CommunityModuleOptions, create_community_module,
};
use crate::operations::install;
use crate::operations::project::{ProjectCreationResult, ProjectOptions, create_project};
use crate::operations::update;
use std::path::Path;
use std::process::ExitCode;

pub fn run_cli(arguments: &[String]) -> ExitCode {
    if arguments[0] == "--license" {
        print!("{}", include_str!("../LICENSE.txt"));
        return ExitCode::SUCCESS;
    }
    if arguments[0] == "--package-info" {
        let mut info = serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
            "architecture": std::env::consts::ARCH,
            "platform": std::env::consts::OS,
            "resources": "embedded"
        });
        if cfg!(target_os = "windows") {
            info["architecture"] = "x64".into();
            info["minimumWindows"] = "10.0.19045".into();
            info["runtime"] = if cfg!(target_feature = "crt-static") {
                "static-msvc"
            } else {
                "dynamic-msvc"
            }
            .into();
        }
        println!("{info}");
        return ExitCode::SUCCESS;
    }
    if arguments[0] == "--help" || arguments[0] == "-h" {
        print_usage();
        return ExitCode::SUCCESS;
    }

    if arguments[0] == "install" {
        return match run_install(&arguments[1..]) {
            Ok(message) => {
                println!("{message}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("[Weave ERROR] {error}");
                ExitCode::FAILURE
            }
        };
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

    if arguments[0] == "manage" {
        return match run_manage(&arguments[1..]) {
            Ok(()) => ExitCode::SUCCESS,
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
        Ok(result) => {
            println!("Project created: {}", result.path.display());
            if let Some(warning) = result.git_warning {
                eprintln!("[Weave WARNING] {warning}");
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("[Weave ERROR] {error}");
            ExitCode::FAILURE
        }
    }
}

fn run_install(arguments: &[String]) -> Result<String, String> {
    let mut channel = None;
    let mut replace_conflict = false;
    #[cfg(target_os = "windows")]
    let mut install_pipe = None;
    for argument in arguments {
        match argument.as_str() {
            "--replace-conflict" => replace_conflict = true,
            #[cfg(target_os = "windows")]
            value if value.starts_with("--windows-install-pipe=") => {
                install_pipe = value.strip_prefix("--windows-install-pipe=")
            }
            value if value.starts_with('-') => return Err(format!("Unknown option: {value}")),
            value if channel.is_none() => channel = Some(value),
            value => return Err(format!("Unexpected argument: {value}")),
        }
    }
    let channel = channel.ok_or_else(|| {
        "Usage: weave install <stable|development> [--replace-conflict]".to_owned()
    })?;
    #[cfg(target_os = "windows")]
    if let Some(pipe) = install_pipe {
        return crate::platform::install_windows_worker(channel, replace_conflict, pipe, &|line| {
            println!("{line}")
        })
        .map_err(|error| error.to_string());
    }
    install::install_mayaflux(channel, replace_conflict, None, |line| println!("{line}"))
        .map_err(|error| error.to_string())
}

fn run_new(arguments: &[String]) -> Result<ProjectCreationResult, String> {
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

    let name = name.ok_or_else(|| {
        "Project name required: weave new <name> [destination] [options]".to_owned()
    })?;
    let destination = match destination {
        Some(path) => Path::new(path).to_path_buf(),
        None => std::env::current_dir().map_err(|error| error.to_string())?,
    };
    create_project(name, &destination, options).map_err(|error| error.to_string())
}

fn run_manage(arguments: &[String]) -> Result<(), String> {
    if arguments.len() < 2 {
        return Err(
            "Usage: weave manage <project-dir> <module-name>[,<module-name>...]".to_owned(),
        );
    }
    update::update_project(Path::new(&arguments[0]), &arguments[1..], |message| {
        println!("{message}")
    })
    .map_err(|error| error.to_string())
}

fn run_community(arguments: &[String]) -> Result<CommunityCreationResult, String> {
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

fn print_usage() {
    println!("{}", usage_text());
}

fn usage_text() -> &'static str {
    "Weave: MayaFlux project tool\n\nUsage:\n  weave                 Open the Projects screen\n  weave new <name> [destination] [options]\n  weave manage <project-dir> <module-name>[,<module-name>...]\n  weave community <module-name> [destination-dir] [options]\n  weave install <stable|development> [--replace-conflict]\n  weave --help\n  weave --license         Show the embedded license\n  weave --package-info    Show build and package metadata\n\nOptions for 'new':\n  --with-lila             Enable Live Coding\n  --no-vscode             Skip VS Code configuration\n  --copy-shader-sources   Copy installed MayaFlux shader sources\n\nOptions for 'community':\n  --description <text>    Module description\n  --min-version <version> Minimum MayaFlux version (default 0.5.0)\n  --with-lila             Mark the module as requiring Lila"
}

#[cfg(test)]
mod tests {
    use super::usage_text;

    #[test]
    fn usage_advertises_manage_instead_of_update() {
        let usage = usage_text();
        assert!(usage.contains("weave manage <project-dir> <module-name>[,<module-name>...]"));
        assert!(!usage.contains("weave update"));
    }
}
