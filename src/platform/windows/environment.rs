use super::{discovery, process};
use crate::filesystem as files;
use crate::operations::install::dependencies::{Tool, Tools};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use crate::environment::{Store, apply};
pub fn configure(
    root: &Path,
    selected: &Tools,
    log: &(impl Fn(&str) + Sync),
) -> Result<(), String> {
    let require = |path: &Path| -> Result<(), String> {
        files::safe_directory(path)?;
        if path.is_dir() {
            Ok(())
        } else {
            Err(format!(
                "Required directory is missing: {}. Retry dependency setup",
                path.display()
            ))
        }
    };
    for relative in ["bin", "include", "lib"] {
        require(&root.join(relative))?;
    }
    let dependency_root = |package: Tool| -> Result<PathBuf, String> {
        let executable = selected.get(&package).ok_or_else(|| {
            format!("No verified executable was selected for {package}; retry dependency setup")
        })?;
        if !executable.is_file() {
            return Err(format!(
                "The verified executable disappeared: {}",
                executable.display()
            ));
        }
        let bin = executable
            .parent()
            .filter(|bin| {
                bin.file_name()
                    .is_some_and(|n| n.eq_ignore_ascii_case("bin"))
            })
            .ok_or_else(|| format!("{package} executable is outside its bin directory"))?;
        let root = bin.parent().ok_or("Dependency root is missing")?.to_owned();
        require(&root)?;
        require(bin)?;
        Ok(root)
    };
    let llvm = dependency_root(Tool::Clang)?;
    let vulkan = dependency_root(Tool::ShaderCompiler)?;
    let ffmpeg = dependency_root(Tool::Ffmpeg)?;
    for path in [
        vulkan.join("Include"),
        ffmpeg.join("include"),
        ffmpeg.join("lib"),
    ] {
        require(&path)?;
    }
    let compiler = process::checked(
        &mut process::powershell(include_str!("scripts/compiler.ps1")),
        false,
        &|_| {},
    )?;
    let compiler: serde_json::Value = serde_json::from_slice(&compiler.stdout)
        .map_err(|e| format!("Invalid Microsoft compiler discovery response: {e}"))?;
    let paths = |field: &str| -> Result<Vec<PathBuf>, String> {
        let list = compiler
            .get(field)
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| format!("Microsoft discovery is missing {field}"))?;
        if list.is_empty() {
            return Err(format!("Microsoft discovery returned no {field}"));
        }
        list.iter()
            .map(|value| {
                let path = PathBuf::from(
                    value
                        .as_str()
                        .ok_or("Invalid Microsoft compiler directory")?,
                );
                require(&path)?;
                Ok(path)
            })
            .collect()
    };
    let includes = paths("includes")?;
    let mut libraries = paths("libraries")?;
    libraries.push(ffmpeg.join("lib"));
    let snapshot = discovery::Environment::read()?;
    let mut store = MachineStore { snapshot };
    let mut values = BTreeMap::new();
    for (name, path) in [
        ("MAYAFLUX_ROOT", root.to_owned()),
        ("VULKAN_SDK", vulkan.clone()),
        ("VK_SDK_PATH", vulkan.clone()),
        ("LLVM_ROOT", llvm.clone()),
        ("LLVM_DIR", llvm.join("lib/cmake/llvm")),
        ("Clang_DIR", llvm.join("lib/cmake/clang")),
        ("FFMPEG_ROOT", ffmpeg.clone()),
    ] {
        values.insert(name.to_owned(), path.to_string_lossy().into_owned());
    }
    let mut binaries = vec![
        root.join("bin"),
        vulkan.join("Bin"),
        llvm.join("bin"),
        ffmpeg.join("bin"),
    ];
    for package in [Tool::CMake, Tool::Git, Tool::Ninja] {
        let executable = selected
            .get(&package)
            .ok_or_else(|| format!("No verified tool selected for {package}"))?;
        if !executable.is_file() {
            return Err(format!(
                "Verified {package} executable is missing; retry dependency setup"
            ));
        }
        let directory = executable
            .parent()
            .ok_or("Selected tool has no directory")?;
        require(directory)?;
        binaries.insert(0, directory.to_owned());
    }
    for (name, additions, prefer) in [
        ("PATH", binaries, true),
        (
            "CPATH",
            vec![
                root.join("include"),
                vulkan.join("Include"),
                ffmpeg.join("include"),
            ],
            false,
        ),
        ("LIBRARY_PATH", vec![root.join("lib")], false),
        ("INCLUDE", includes, false),
        ("LIB", libraries, false),
    ] {
        values.insert(
            name.to_owned(),
            merge_paths(&store.read(name)?, &additions, prefer)?,
        );
    }
    apply(&values, &mut store)?;
    for name in values.keys() {
        log(&format!("Configured {name}."));
    }
    log("Machine environment configured. Restart your terminal to load the selected tools.");
    Ok(())
}

struct MachineStore {
    snapshot: discovery::Environment,
}
impl Store for MachineStore {
    fn read(&self, name: &str) -> Result<String, String> {
        Ok(self
            .snapshot
            .expand(self.snapshot.value("Machine", name).unwrap_or("")))
    }
    fn write(&mut self, name: &str, value: &str) -> Result<(), String> {
        let mut command = process::powershell(
            "[Environment]::SetEnvironmentVariable($env:WEAVE_ENV_NAME,$env:WEAVE_ENV_VALUE,'Machine')",
        );
        command
            .env("WEAVE_ENV_NAME", name)
            .env("WEAVE_ENV_VALUE", value);
        process::checked(&mut command, false, &|_| {})?;
        Ok(())
    }
}

fn normalize(value: &str) -> String {
    let text = value.trim().trim_matches('"').replace('/', "\\");
    let path = Path::new(&text);
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::CurDir => {}
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
        .to_string_lossy()
        .trim_end_matches('\\')
        .to_ascii_lowercase()
}
pub fn merge_paths(existing: &str, additions: &[PathBuf], prefer: bool) -> Result<String, String> {
    crate::environment::merge_paths(existing, additions, prefer, ';', normalize)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn environment_merge_deduplicates_whole_paths_and_preserves_other_entries() {
        let paths = [
            PathBuf::from(r"D:\Tools\bin"),
            PathBuf::from(r"D:\LLVM Tools\bin"),
        ];
        assert_eq!(
            merge_paths(r"E:\Old;d:\tools\BIN\;D:\Tools\binary", &paths, true).unwrap(),
            r"D:\Tools\bin;D:\LLVM Tools\bin;E:\Old;D:\Tools\binary"
        );
        assert_eq!(
            merge_paths("", &paths[..1], false).unwrap(),
            r"D:\Tools\bin"
        );
        for value in [
            "relative",
            "D:relative",
            "D:\\Bad;Injected",
            "D:\\Bad\nInjected",
        ] {
            assert!(merge_paths("", &[PathBuf::from(value)], false).is_err());
        }
    }
}
