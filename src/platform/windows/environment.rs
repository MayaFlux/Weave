use super::{files, host, tools};
use crate::install::windows::Tools;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub trait Store {
    fn read(&self, name: &str) -> Result<String, String>;
    fn write(&mut self, name: &str, value: &str) -> Result<(), String>;
}

pub fn apply(values: &BTreeMap<String, String>, store: &mut impl Store) -> Result<(), String> {
    for (name, value) in values {
        let existing = store
            .read(name)
            .map_err(|e| format!("Cannot read {name}: {e}; retry environment setup"))?;
        if existing != *value {
            store.write(name, value).map_err(|e| format!("Cannot update {name}: {e}. Earlier environment changes may already be applied; correct permissions and retry environment setup"))?;
        }
    }
    Ok(())
}

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
    let dependency_root = |package: &str| -> Result<PathBuf, String> {
        let executable = selected.get(package).ok_or_else(|| {
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
    let llvm = dependency_root("LLVM.LLVM")?;
    let vulkan = dependency_root("KhronosGroup.VulkanSDK")?;
    let ffmpeg = dependency_root("Gyan.FFmpeg.Shared")?;
    for path in [
        vulkan.join("Include"),
        ffmpeg.join("include"),
        ffmpeg.join("lib"),
    ] {
        require(&path)?;
    }
    let compiler = tools::checked(
        &mut tools::powershell(include_str!("compiler.ps1")),
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
    let snapshot = host::Environment::read()?;
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
    for package in ["Kitware.CMake", "Git.Git", "Ninja-build.Ninja"] {
        let executable = selected
            .get(package)
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
            tools::merge_paths(&store.read(name)?, &additions, prefer)?,
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
    snapshot: host::Environment,
}
impl Store for MachineStore {
    fn read(&self, name: &str) -> Result<String, String> {
        Ok(self
            .snapshot
            .expand(self.snapshot.value("Machine", name).unwrap_or("")))
    }
    fn write(&mut self, name: &str, value: &str) -> Result<(), String> {
        let mut command = tools::powershell(
            "[Environment]::SetEnvironmentVariable($env:WEAVE_ENV_NAME,$env:WEAVE_ENV_VALUE,'Machine')",
        );
        command
            .env("WEAVE_ENV_NAME", name)
            .env("WEAVE_ENV_VALUE", value);
        tools::checked(&mut command, false, &|_| {})?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Fake {
        values: BTreeMap<String, String>,
        fail: Option<String>,
    }
    impl Store for Fake {
        fn read(&self, name: &str) -> Result<String, String> {
            Ok(self.values.get(name).cloned().unwrap_or_default())
        }
        fn write(&mut self, name: &str, value: &str) -> Result<(), String> {
            if self.fail.as_deref() == Some(name) {
                return Err("Access denied".into());
            }
            self.values.insert(name.into(), value.into());
            Ok(())
        }
    }
    #[test]
    fn failed_machine_write_reports_partial_state_and_retry_is_idempotent() {
        let values = BTreeMap::from([
            ("MAYAFLUX_ROOT".into(), "D:\\Maya Flux".into()),
            ("PATH".into(), "D:\\Tools".into()),
        ]);
        let mut store = Fake {
            fail: Some("PATH".into()),
            ..Default::default()
        };
        let error = apply(&values, &mut store).unwrap_err();
        assert!(error.contains("PATH"));
        assert!(error.contains("retry"));
        assert_eq!(
            store.values.get("MAYAFLUX_ROOT").map(String::as_str),
            Some("D:\\Maya Flux")
        );
        store.fail = None;
        apply(&values, &mut store).unwrap();
        apply(&values, &mut store).unwrap();
        assert_eq!(store.values, values);
        store.fail = Some("PATH".into());
        let changed = BTreeMap::from([("PATH".into(), "D:\\New Tools".into())]);
        assert!(apply(&changed, &mut store).is_err());
    }
}
