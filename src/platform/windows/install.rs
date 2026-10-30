pub const CORE: &[&str] = &[
    "lib/MayaFluxLib.lib",
    "bin/MayaFluxLib.dll",
    "include/MayaFlux/MayaFlux.hpp",
    "lib/cmake/MayaFlux/MayaFluxConfig.cmake",
    "lib/cmake/MayaFlux/MayaFluxConfigVersion.cmake",
];

#[cfg(test)]
use super::discovery::shared_ffmpeg;
use super::{discovery::Environment, process};
#[cfg(test)]
use crate::filesystem::version_directories;
#[cfg(test)]
use crate::operations::install::acquisition::{ArchiveFormat, validate_archive_for};
use crate::operations::install::dependencies::{self, Dependency, DependencyHost, Tool};
use crate::operations::install::workflow::{Channel, Host, Release, Tools};
use crate::{filesystem as files, resources};
use serde_json::Value;
#[cfg(test)]
use std::fs;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

pub fn winget_arguments(id: &str, source: Option<&str>, force_ffmpeg: bool) -> Vec<String> {
    let mut args: Vec<_> = [
        "install",
        "--id",
        id,
        "--exact",
        "--accept-package-agreements",
        "--accept-source-agreements",
        "--silent",
    ]
    .map(str::to_owned)
    .into();
    if let Some(source) = source {
        args.extend(["--source".into(), source.into()]);
    }
    if id == "Gyan.FFmpeg.Shared" {
        args.extend([
            "--location".into(),
            r"C:\Program Files\FFmpeg_Shared".into(),
        ]);
        if force_ffmpeg {
            args.push("--force".into());
        }
    }
    args
}
pub struct WindowsHost<'a, L: Fn(&str) + Sync> {
    pub root: PathBuf,
    pub log: &'a L,
}

fn ensure_microsoft(
    probe: impl Fn() -> Result<(), String>,
    acquire: impl FnOnce() -> Result<bool, String>,
) -> Result<bool, String> {
    if probe().is_ok() {
        return Ok(false);
    }
    let reboot = acquire()?;
    probe().map_err(|error| {
        format!("Microsoft SDK/runtime setup is incomplete: {error}. Retry prerequisite repair")
    })?;
    Ok(reboot)
}
impl<L: Fn(&str) + Sync> WindowsHost<'_, L> {
    fn powershell(&self, script: &str) -> Result<String, String> {
        let output = process::checked(&mut process::powershell(script), false, &|_| {})?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }
    fn downloader(&self) -> crate::download::Curl<super::process::Runtime> {
        crate::download::Curl {
            executable: process::system_directory().join("curl.exe"),
            runtime: super::process::Runtime,
        }
    }
}

impl<L: Fn(&str) + Sync> Host for WindowsHost<'_, L> {
    fn asset_suffix(&self) -> &str {
        "windows-x64.7z"
    }
    fn completion_advice(&self, reboot: bool) -> String {
        format!(
            "{} Open an x64 Visual Studio developer shell in your project and run `cmake --preset release`, then `cmake --build --preset release`.",
            if reboot {
                "Restart Windows before building projects."
            } else {
                "Restart your terminal before building projects."
            }
        )
    }
    fn prerequisites(&mut self) -> Result<bool, String> {
        files::safe_directory(&self.root)?;
        if !cfg!(target_arch = "x86_64") {
            return Err("An x64 Weave executable is required".into());
        }
        let build: u32 = self
            .powershell("[Environment]::OSVersion.Version.Build")?
            .parse()
            .map_err(|_| "Cannot determine the Windows build")?;
        if build < 19045 {
            return Err("Windows 10 22H2 (build 19045) or later is required".into());
        }
        let mut reboot = false;
        (self.log)("Checking 7-Zip...");
        if self.seven_zip().is_none() {
            (self.log)("Installing 7-Zip for MayaFlux package extraction...");
            let result = process::checked(
                Command::new("winget").args(winget_arguments("7zip.7zip", None, false)),
                true,
                self.log,
            )?;
            reboot |= result.status.code() == Some(3010);
            if self.seven_zip().is_none() {
                return Err(
                    "7-Zip installer finished but 7z.exe was not found; retry prerequisites".into(),
                );
            }
        }
        (self.log)("7-Zip is available for package extraction.");
        (self.log)("Checking Microsoft x64 C++ tools, runtime libraries and Windows SDK...");
        let has_compiler = self.compiler().is_some();
        let microsoft_reboot = ensure_microsoft(
            || {
                if self.compiler().is_none() {
                    return Err("No usable x64 C++ compiler was found".into());
                }
                self.powershell(include_str!("scripts/compiler.ps1"))
                    .map(|_| ())
            },
            || {
                (self.log)(
                    "Windows needs Microsoft C++ libraries and SDK files for MayaFlux builds and live compilation, even with LLVM. Microsoft's installer will display progress; Weave will wait for it to finish.",
                );
                let package = if build >= 22000 {
                    "Microsoft.VisualStudio.BuildTools"
                } else {
                    "Microsoft.VisualStudio.2022.BuildTools"
                };
                let mut command = Command::new("winget");
                command.args(["install", "--id", package, "--exact", "--accept-package-agreements", "--accept-source-agreements", "--override", "--wait --passive --norestart --add Microsoft.VisualStudio.Component.VC.Tools.x86.x64 --add Microsoft.VisualStudio.Component.Windows11SDK.22621 --add Microsoft.VisualStudio.Component.VC.Redist.14.Latest"]);
                if has_compiler {
                    command.arg("--force");
                }
                process::checked(&mut command, true, self.log)
                    .map(|result| result.status.code() == Some(3010))
            },
        )?;
        (self.log)("Microsoft x64 C++ tools, runtime libraries and Windows SDK verified.");
        Ok(reboot || microsoft_reboot)
    }
    fn releases(&mut self, channel: Channel) -> Result<Value, String> {
        crate::operations::install::release::fetch(channel, &self.downloader(), self.log)
    }
    fn installed(&mut self) -> Result<Option<Value>, String> {
        crate::operations::install::release::installed(&self.root)
    }
    fn verify(&mut self) -> Result<(), String> {
        files::verify_payload(&self.root, CORE)
    }
    fn acquire(&mut self, release: &Release) -> Result<(), String> {
        let layout = crate::operations::install::acquisition::PayloadLayout {
            archive_format: crate::operations::install::acquisition::ArchiveFormat::SEVEN_ZIP,
            required: CORE,
            wrapper: "dist_staging",
            version_marker: "share/MayaFlux/.version",
        };
        crate::operations::install::acquisition::acquire(
            &self.root,
            release,
            &layout,
            &self.downloader(),
            self,
            self.log,
        )
    }
    fn dependencies(&mut self) -> Result<Tools, String> {
        let source = process::run(
            Command::new("winget").args(["source", "list"]),
            false,
            std::time::Duration::from_secs(30),
            &|_| {},
        )
        .ok()
        .filter(|r| r.status.success())
        .and_then(|r| {
            String::from_utf8_lossy(&r.stdout).lines().find_map(|line| {
                let columns: Vec<_> = line.split_whitespace().collect();
                (columns.len() >= 2
                    && ["winget", "Microsoft.Winget.Source"].contains(&columns[0])
                    && columns[1].starts_with("https://"))
                .then(|| columns[0].to_owned())
            })
        });
        let adapter = WindowsDependencies { host: self, source };
        let requirements: Vec<_> = PACKAGES
            .iter()
            .map(|(id, _)| Dependency {
                tool: package_tool(id),
                refresh_existing: *id == "Gyan.FFmpeg.Shared",
            })
            .collect();
        dependencies::ensure(&adapter, &requirements, self.log)
    }
    fn environment(&mut self, selected: &Tools) -> Result<(), String> {
        super::environment::configure(&self.root, selected, self.log)
    }
    fn resources(&mut self) -> Result<(), String> {
        resources::deploy_resources(&self.root)?;
        (self.log)(&format!(
            "Installed and verified {} Weave templates and tools.",
            resources::RESOURCES.len()
        ));
        crate::operations::install::verify_weave_resources(self.log)
    }
    fn now(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

pub fn vswhere() -> Option<PathBuf> {
    ["ProgramFiles(x86)", "ProgramFiles"]
        .iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .map(|root| root.join("Microsoft Visual Studio/Installer/vswhere.exe"))
        .find(|path| path.is_file())
}
impl<L: Fn(&str) + Sync> crate::operations::install::acquisition::Extractor for WindowsHost<'_, L> {
    fn extract(
        &self,
        archive: &Path,
        extracted: &Path,
        log: &(impl Fn(&str) + Sync),
    ) -> Result<(), String> {
        let seven_zip = self
            .seven_zip()
            .ok_or("7-Zip is missing; retry prerequisites")?;
        process::run(
            Command::new(seven_zip)
                .arg("x")
                .arg(archive)
                .arg(format!("-o{}", extracted.display()))
                .arg("-y"),
            false,
            std::time::Duration::from_secs(900),
            log,
        )
        .and_then(|r| {
            if r.status.success() {
                Ok(())
            } else {
                Err(format!(
                    "7-Zip extraction failed: {}",
                    String::from_utf8_lossy(&r.stderr)
                ))
            }
        })
    }
}

pub const PACKAGES: &[(&str, &str)] = &[
    ("Kitware.CMake", "cmake.exe"),
    ("Git.Git", "git.exe"),
    ("Ninja-build.Ninja", "ninja.exe"),
    ("LLVM.LLVM", "clang.exe"),
    ("Gyan.FFmpeg.Shared", "ffmpeg.exe"),
    ("KhronosGroup.VulkanSDK", "glslc.exe"),
];
pub(super) fn package_tool(id: &str) -> Tool {
    match id {
        "Kitware.CMake" => Tool::CMake,
        "Git.Git" => Tool::Git,
        "Ninja-build.Ninja" => Tool::Ninja,
        "LLVM.LLVM" => Tool::Clang,
        "Gyan.FFmpeg.Shared" => Tool::Ffmpeg,
        "KhronosGroup.VulkanSDK" => Tool::ShaderCompiler,
        _ => unreachable!("Unknown native package"),
    }
}
struct WindowsDependencies<'a, 'b, L: Fn(&str) + Sync> {
    host: &'a WindowsHost<'b, L>,
    source: Option<String>,
}
impl<L: Fn(&str) + Sync> DependencyHost for WindowsDependencies<'_, '_, L> {
    fn probe(&self, tool: Tool) -> Result<Option<PathBuf>, String> {
        let (id, executable) = PACKAGES
            .iter()
            .find(|(id, _)| package_tool(id) == tool)
            .ok_or("No native package mapping for tool")?;
        Ok(self.host.find_tool(id, executable, &Environment::read()?))
    }
    fn acquire(&self, tool: Tool, previously_usable: bool) -> Result<(), String> {
        let (id, _) = PACKAGES
            .iter()
            .find(|(id, _)| package_tool(id) == tool)
            .ok_or("No native package mapping for tool")?;
        let result = process::run(
            Command::new("winget").args(winget_arguments(
                id,
                self.source.as_deref(),
                !previously_usable,
            )),
            true,
            std::time::Duration::from_secs(3600),
            self.host.log,
        )?;
        if !result.status.success() && result.status.code() != Some(-1978335189) {
            return Err(format!(
                "winget {id} exited with {}: {}",
                result.status,
                String::from_utf8_lossy(&result.stderr)
            ));
        }
        if tool == Tool::Ffmpeg {
            self.host.normalize_ffmpeg()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_sdk_is_repaired_even_when_a_compiler_is_already_present() {
        let sdk = std::cell::Cell::new(false);
        let result = ensure_microsoft(
            || {
                if sdk.get() {
                    Ok(())
                } else {
                    Err("Compiler exists but SDK/runtime libraries are missing".into())
                }
            },
            || {
                sdk.set(true);
                Ok(true)
            },
        );
        assert_eq!(result, Ok(true));
        assert!(sdk.get());
        assert!(
            ensure_microsoft(|| Err("SDK is still missing".into()), || Ok(false))
                .unwrap_err()
                .contains("SDK")
        );
    }
    #[test]
    fn winget_uses_exact_packages_and_one_spaced_ffmpeg_location_argument() {
        let args = winget_arguments("Gyan.FFmpeg.Shared", Some("winget"), false);
        assert_eq!(
            args,
            [
                "install",
                "--id",
                "Gyan.FFmpeg.Shared",
                "--exact",
                "--accept-package-agreements",
                "--accept-source-agreements",
                "--silent",
                "--source",
                "winget",
                "--location",
                r"C:\Program Files\FFmpeg_Shared"
            ]
        );
        assert!(
            winget_arguments("Gyan.FFmpeg.Shared", None, true)
                .iter()
                .any(|a| a == "--force")
        );
        assert!(
            !winget_arguments("LLVM.LLVM", None, true)
                .iter()
                .any(|a| a == "--force")
        );
    }
    #[test]
    fn archive_magic_and_declared_size_are_both_required() {
        let directory = files::Temporary::create().unwrap();
        let archive = directory.0.join("fixture.7z");
        let mut payload = vec![0; 32];
        payload[..6].copy_from_slice(&[0x37, 0x7a, 0xbc, 0xaf, 0x27, 0x1c]);
        fs::write(&archive, &payload).unwrap();
        assert!(validate_archive_for(&archive, 32, ArchiveFormat::SEVEN_ZIP).is_ok());
        assert!(validate_archive_for(&archive, 33, ArchiveFormat::SEVEN_ZIP).is_err());
        payload[0] = 0;
        fs::write(&archive, payload).unwrap();
        assert!(validate_archive_for(&archive, 32, ArchiveFormat::SEVEN_ZIP).is_err());
    }
    #[test]
    fn shared_ffmpeg_requires_development_headers_libraries_and_dlls() {
        let root = files::Temporary::create().unwrap();
        fs::create_dir_all(root.0.join("bin")).unwrap();
        fs::create_dir_all(root.0.join("lib")).unwrap();
        fs::write(root.0.join("bin/ffmpeg.exe"), "fixture").unwrap();
        assert!(!shared_ffmpeg(&root.0));
        for component in [
            "avcodec",
            "avformat",
            "avutil",
            "swresample",
            "swscale",
            "avdevice",
        ] {
            fs::create_dir_all(root.0.join(format!("include/lib{component}"))).unwrap();
            for relative in [
                format!("bin/{component}-62.dll"),
                format!("lib/{component}.lib"),
                format!("include/lib{component}/{component}.h"),
            ] {
                fs::write(root.0.join(relative), "fixture").unwrap();
            }
        }
        assert!(shared_ffmpeg(&root.0));
        fs::remove_file(root.0.join("lib/avdevice.lib")).unwrap();
        assert!(!shared_ffmpeg(&root.0));
    }
    #[test]
    fn version_selection_orders_numbers_and_ignores_incomplete_ffmpeg() {
        let root = files::Temporary::create().unwrap();
        for name in ["9.9.0", "9.10.0", "not-a-version"] {
            fs::create_dir(root.0.join(name)).unwrap();
        }
        assert_eq!(
            version_directories(&root.0),
            [root.0.join("9.10.0"), root.0.join("9.9.0")]
        );
    }
    #[test]
    fn executable_discovery_accepts_verified_tools_through_package_alias_directories() {
        let root = files::Temporary::create().unwrap();
        let actual = root.0.join("actual tools");
        fs::create_dir(&actual).unwrap();
        let source = root.0.join("fixture.rs");
        fs::write(&source, "fn main() { println!(\"1.13.1\"); }").unwrap();
        let compiler = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
        let compiled = Command::new(compiler)
            .arg(&source)
            .arg("-o")
            .arg(actual.join("ninja.exe"))
            .output()
            .unwrap();
        assert!(
            compiled.status.success(),
            "{}",
            String::from_utf8_lossy(&compiled.stderr)
        );
        let alias = root.0.join("package aliases");
        let mut command = process::powershell(
            "New-Item -ItemType Junction -Path $env:WEAVE_ALIAS -Target $env:WEAVE_TARGET | Out-Null",
        );
        command
            .env("WEAVE_ALIAS", &alias)
            .env("WEAVE_TARGET", &actual);
        process::checked(&mut command, false, &|_| {}).unwrap();
        let environment = Environment(
            serde_json::json!({"Machine":{"PATH":alias.to_string_lossy()},"User":{},"Process":{}}),
        );
        let host = WindowsHost {
            root: root.0.clone(),
            log: &|_| {},
        };
        assert_eq!(
            host.find_tool("Ninja-build.Ninja", "ninja.exe", &environment),
            Some(alias.join("ninja.exe"))
        );
        assert!(
            files::no_links(&alias).is_err(),
            "fixture must actually cross a reparse point"
        );
        // Removing the alias itself preserves its target before the fixture tree is cleaned.
        fs::remove_dir(&alias).unwrap();
        assert!(actual.join("ninja.exe").is_file());
    }
}
