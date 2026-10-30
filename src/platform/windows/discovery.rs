//! Windows tool locations, environment snapshots and package layouts.
use super::install::{package_tool, vswhere};
use super::{install::WindowsHost, process};
use crate::{
    filesystem as files,
    filesystem::{numeric_version, version_directories},
    operations::install::dependencies,
};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
pub fn shared_ffmpeg(root: &Path) -> bool {
    let components = [
        "avcodec",
        "avformat",
        "avutil",
        "swresample",
        "swscale",
        "avdevice",
    ];
    let mut required = vec![PathBuf::from("bin/ffmpeg.exe")];
    for component in components {
        required.extend([
            PathBuf::from(format!("lib/{component}.lib")),
            PathBuf::from(format!("include/lib{component}/{component}.h")),
        ]);
    }
    let runtime_patterns: Vec<_> = components
        .iter()
        .map(|component| (format!("{component}-"), ".dll".to_owned()))
        .collect();
    files::development_package_present(root, &required, Path::new("bin"), &runtime_patterns, false)
}

impl<L: Fn(&str) + Sync> WindowsHost<'_, L> {
    pub(super) fn seven_zip(&self) -> Option<PathBuf> {
        ["ProgramFiles", "ProgramFiles(x86)"]
            .iter()
            .filter_map(std::env::var_os)
            .map(PathBuf::from)
            .chain([
                PathBuf::from(r"C:\Program Files"),
                PathBuf::from(r"C:\Program Files (x86)"),
            ])
            .map(|root| root.join("7-Zip/7z.exe"))
            .find(|path| path.is_file())
    }
    pub(super) fn compiler(&self) -> Option<PathBuf> {
        let discovery = vswhere()?;
        let output = process::checked(
            Command::new(discovery).args([
                "-latest",
                "-prerelease",
                "-products",
                "*",
                "-requires",
                "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
                "-find",
                r"VC\Tools\MSVC\**\bin\Hostx64\x64\cl.exe",
            ]),
            false,
            &|_| {},
        )
        .ok()?;
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .map(PathBuf::from)
            .find(|path| {
                path.is_absolute()
                    && path
                        .to_string_lossy()
                        .to_ascii_lowercase()
                        .ends_with(r"\bin\hostx64\x64\cl.exe")
                    && path.is_file()
            })
    }
    pub(super) fn find_tool(
        &self,
        id: &str,
        executable: &str,
        environment: &Environment,
    ) -> Option<PathBuf> {
        let mut candidates = Vec::new();
        match id {
            "LLVM.LLVM" => {
                candidates.extend(
                    environment
                        .roots("LLVM_ROOT")
                        .map(|root| root.join("bin/clang.exe")),
                );
                candidates.push(PathBuf::from(r"C:\Program Files\LLVM\bin\clang.exe"));
            }
            "KhronosGroup.VulkanSDK" => {
                candidates.extend(
                    environment
                        .roots("VULKAN_SDK")
                        .chain(environment.roots("VK_SDK_PATH"))
                        .map(|root| root.join("Bin/glslc.exe")),
                );
                candidates.extend(
                    version_directories(Path::new(r"C:\VulkanSDK"))
                        .into_iter()
                        .map(|root| root.join("Bin/glslc.exe")),
                );
            }
            "Gyan.FFmpeg.Shared" => {
                let roots = version_directories(Path::new(r"C:\Program Files\FFmpeg_Shared"));
                candidates.extend(
                    roots
                        .into_iter()
                        .chain(environment.roots("FFMPEG_ROOT"))
                        .filter(|root| shared_ffmpeg(root))
                        .map(|root| root.join("bin/ffmpeg.exe")),
                );
            }
            _ => {}
        }
        if id != "Gyan.FFmpeg.Shared" {
            candidates.extend(environment.paths().map(|root| root.join(executable)));
        }
        dependencies::probe_candidates(&process::Runtime, package_tool(id), candidates)
    }
    pub(super) fn normalize_ffmpeg(&self) -> Result<(), String> {
        let root = Path::new(r"C:\Program Files\FFmpeg_Shared");
        if !root.is_dir() {
            return Ok(());
        }
        files::safe_directory(root)?;
        for entry in fs::read_dir(root).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some(version) = name
                .strip_prefix("ffmpeg-")
                .and_then(|n| n.strip_suffix("-full_build-shared"))
            else {
                continue;
            };
            if numeric_version(version).is_none() {
                continue;
            }
            let target = root.join(version);
            if target.exists() {
                continue;
            }
            files::no_links(&entry.path())?;
            if !shared_ffmpeg(&entry.path()) {
                return Err(format!(
                    "FFmpeg Shared package is incomplete: {}",
                    entry.path().display()
                ));
            }
            fs::rename(entry.path(), &target)
                .map_err(|e| format!("Cannot normalize FFmpeg Shared directory: {e}"))?;
        }
        Ok(())
    }
}
pub struct Environment(pub(super) Value);
impl Environment {
    pub fn read() -> Result<Self, String> {
        let script = "$result=@{}; foreach($scope in @('Machine','User','Process')) { $result[$scope]=[Environment]::GetEnvironmentVariables($scope) }; $result | ConvertTo-Json -Depth 5 -Compress";
        let output = process::checked(&mut process::powershell(script), false, &|_| {})?;
        serde_json::from_slice(&output.stdout)
            .map(Self)
            .map_err(|e| format!("Cannot read Windows environment: {e}"))
    }
    pub fn value(&self, scope: &str, name: &str) -> Option<&str> {
        self.0
            .get(scope)?
            .as_object()?
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))?
            .1
            .as_str()
    }
    pub fn expand(&self, input: &str) -> String {
        let mut value = input.trim().trim_matches('"').to_owned();
        for _ in 0..8 {
            let Some(start) = value.find('%') else {
                break;
            };
            let Some(end) = value[start + 1..].find('%').map(|n| n + start + 1) else {
                break;
            };
            let replacement = ["Process", "User", "Machine"]
                .into_iter()
                .find_map(|scope| self.value(scope, &value[start + 1..end]));
            let Some(replacement) = replacement else {
                break;
            };
            value.replace_range(start..=end, replacement);
        }
        value
    }
    fn roots<'a>(&'a self, name: &'a str) -> impl Iterator<Item = PathBuf> + 'a {
        ["Process", "User", "Machine"]
            .into_iter()
            .filter_map(move |scope| self.value(scope, name))
            .map(|text| PathBuf::from(self.expand(text)))
            .filter(|path| path.is_absolute())
    }
    fn paths(&self) -> impl Iterator<Item = PathBuf> + '_ {
        ["Machine", "User", "Process"]
            .into_iter()
            .filter_map(|scope| self.value(scope, "PATH"))
            .flat_map(|path| path.split(';'))
            .map(|text| PathBuf::from(self.expand(text)))
            .filter(|path| path.is_absolute())
    }
}
