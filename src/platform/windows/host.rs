use super::{files, tools};
use crate::install::windows::{Channel, Host, Release, Tools};
use serde_json::Value;
use std::{
    fs,
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
pub fn validate_archive(path: &Path, size: u64) -> Result<(), String> {
    use std::io::Read;
    files::no_links(path)?;
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut magic = [0; 6];
    file.read_exact(&mut magic).map_err(|e| e.to_string())?;
    if size < 32
        || file.metadata().map_err(|e| e.to_string())?.len() != size
        || magic != [0x37, 0x7a, 0xbc, 0xaf, 0x27, 0x1c]
    {
        return Err("Downloaded MayaFlux archive is invalid or incomplete".into());
    }
    Ok(())
}

pub struct SystemHost<'a, L: Fn(&str) + Sync> {
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
impl<L: Fn(&str) + Sync> SystemHost<'_, L> {
    fn powershell(&self, script: &str) -> Result<String, String> {
        let output = tools::checked(&mut tools::powershell(script), false, &|_| {})?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }
    fn download(&self, url: &str, destination: &Path, limit: u64) -> Result<(), String> {
        files::no_links(destination)?;
        // Reserve this filename; curl writes only inside our uniquely owned temporary tree.
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)
            .map_err(|e| e.to_string())?;
        let mut command = Command::new(system_directory().join("curl.exe"));
        command
            .args([
                "--fail",
                "--location",
                "--proto",
                "=https",
                "--proto-redir",
                "=https",
                "--connect-timeout",
                "30",
                "--max-time",
                "1800",
                "--max-filesize",
                &limit.to_string(),
                "--output",
            ])
            .arg(destination)
            .arg(url);
        let output = tools::run(
            &mut command,
            false,
            std::time::Duration::from_secs(1900),
            self.log,
        )?;
        if !output.status.success() {
            return Err(format!(
                "Download failed: {}. Check the connection and retry",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        if fs::metadata(destination).map_err(|e| e.to_string())?.len() > limit {
            return Err("Download exceeds its declared maximum size".into());
        }
        Ok(())
    }
    fn seven_zip(&self) -> Option<PathBuf> {
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
    fn compiler(&self) -> Option<PathBuf> {
        let discovery = vswhere()?;
        let output = tools::checked(
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
    fn find_tool(&self, id: &str, executable: &str, environment: &Environment) -> Option<PathBuf> {
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
                        .filter(|root| tools::shared_ffmpeg(root))
                        .map(|root| root.join("bin/ffmpeg.exe")),
                );
            }
            _ => {}
        }
        if id != "Gyan.FFmpeg.Shared" {
            candidates.extend(environment.paths().map(|root| root.join(executable)));
        }
        for path in candidates {
            // WinGet portable packages expose executable symlinks through WinGet/Links.
            // These are read-only discovery inputs, unlike installer write destinations.
            if !path.is_file() {
                continue;
            }
            let result = tools::run(
                Command::new(&path).arg(if id == "Gyan.FFmpeg.Shared" {
                    "-version"
                } else {
                    "--version"
                }),
                false,
                std::time::Duration::from_secs(30),
                &|_| {},
            );
            if let Ok(result) = result
                && result.status.success()
                && tools::usable(
                    id,
                    &format!(
                        "{}\n{}",
                        String::from_utf8_lossy(&result.stdout),
                        String::from_utf8_lossy(&result.stderr)
                    ),
                )
            {
                return Some(path);
            }
        }
        None
    }
    fn normalize_ffmpeg(&self) -> Result<(), String> {
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
            if !tools::shared_ffmpeg(&entry.path()) {
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

impl<L: Fn(&str) + Sync> Host for SystemHost<'_, L> {
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
            let result = tools::checked(
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
                self.powershell(include_str!("compiler.ps1")).map(|_| ())
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
                tools::checked(&mut command, true, self.log)
                    .map(|result| result.status.code() == Some(3010))
            },
        )?;
        (self.log)("Microsoft x64 C++ tools, runtime libraries and Windows SDK verified.");
        Ok(reboot || microsoft_reboot)
    }
    fn releases(&mut self, channel: Channel) -> Result<Value, String> {
        let temporary = files::Temporary::create()?;
        let metadata = temporary.0.join("release.json");
        self.download(
            if channel == Channel::Stable {
                "https://api.github.com/repos/MayaFlux/MayaFlux/releases/latest"
            } else {
                "https://api.github.com/repos/MayaFlux/MayaFlux/releases"
            },
            &metadata,
            16 * 1024 * 1024,
        )?;
        serde_json::from_slice(&fs::read(metadata).map_err(|e| e.to_string())?)
            .map_err(|e| format!("Invalid GitHub release metadata: {e}"))
    }
    fn installed(&mut self) -> Result<Option<Value>, String> {
        let path = self.root.join("share/MayaFlux/.version");
        files::no_links(&path)?;
        match fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|e| e.to_string()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
    fn verify(&mut self) -> Result<(), String> {
        files::verify(&self.root)
    }
    fn acquire(&mut self, release: &Release) -> Result<(), String> {
        files::safe_directory(&self.root)?;
        let temporary = files::Temporary::create()?;
        if temporary.0.starts_with(&self.root) || self.root.starts_with(&temporary.0) {
            return Err("Installation and temporary directories overlap".into());
        }
        (self.log)(&format!("Downloading {}...", release.tag));
        let archive = temporary.0.join("mayaflux.7z");
        self.download(&release.url, &archive, release.size)?;
        validate_archive(&archive, release.size)?;
        (self.log)(
            "Download complete; archive size and signature verified. Extracting MayaFlux...",
        );
        let extracted = temporary.0.join("extracted");
        fs::create_dir(&extracted).map_err(|e| e.to_string())?;
        let seven_zip = self
            .seven_zip()
            .ok_or("7-Zip is missing; retry prerequisites")?;
        tools::run(
            Command::new(seven_zip)
                .arg("x")
                .arg(&archive)
                .arg(format!("-o{}", extracted.display()))
                .arg("-y"),
            false,
            std::time::Duration::from_secs(900),
            self.log,
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
        })?;
        let payload = if extracted.join("dist_staging").is_dir() {
            extracted.join("dist_staging")
        } else {
            extracted
        };
        files::verify(&payload)?;
        (self.log)(
            "Staged MayaFlux runtime, headers and CMake package verified. Installing package files...",
        );
        // Inspect the entire extracted tree before touching an existing installation.
        let mut all = Vec::new();
        collect_files(&payload, &payload, &mut all)?;
        files::safe_directory(&self.root)?;
        if self.root.exists() {
            fs::remove_dir_all(&self.root).map_err(|e| format!("Cannot replace old MayaFlux at {}: {e}. Close programs using MayaFlux and retry", self.root.display()))?;
        }
        all.sort_by_key(|relative| relative == Path::new("share/MayaFlux/.version"));
        for relative in all {
            let target = self.root.join(&relative);
            files::no_links(&target)?;
            fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
            fs::copy(payload.join(&relative), &target).map_err(|e| format!("MayaFlux replacement is incomplete at {}: {e}. Retry installation to restore the complete payload", target.display()))?;
        }
        files::verify(&self.root)?;
        (self.log)("Installed MayaFlux runtime, headers and CMake package verified.");
        Ok(())
    }
    fn dependencies(&mut self) -> Result<Tools, String> {
        let source = tools::run(
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
        let mut selected = Tools::new();
        for (id, executable) in tools::PACKAGES {
            (self.log)(&format!("Checking {id}..."));
            let environment = Environment::read()?;
            let previous = self.find_tool(id, executable, &environment);
            if *id != "Gyan.FFmpeg.Shared"
                && let Some(path) = &previous
            {
                (self.log)(&format!(
                    "{id}: verified existing tool at {}",
                    path.display()
                ));
                selected.insert(id.to_string(), path.clone());
                continue;
            }
            (self.log)(&format!("Installing/checking {id}..."));
            let result = tools::run(
                Command::new("winget").args(winget_arguments(
                    id,
                    source.as_deref(),
                    previous.is_none(),
                )),
                true,
                std::time::Duration::from_secs(3600),
                self.log,
            )
            .and_then(|r| {
                if r.status.success() || r.status.code() == Some(-1978335189) {
                    Ok(())
                } else {
                    Err(format!(
                        "winget {id} exited with {}: {}",
                        r.status,
                        String::from_utf8_lossy(&r.stderr)
                    ))
                }
            })
            .and_then(|()| {
                if *id == "Gyan.FFmpeg.Shared" {
                    self.normalize_ffmpeg()
                } else {
                    Ok(())
                }
            });
            let refreshed = self.find_tool(id, executable, &Environment::read()?);
            match (result, refreshed) {
                (Ok(()), Some(path)) => {
                    (self.log)(&format!(
                        "{id}: installed tool verified at {}",
                        path.display()
                    ));
                    selected.insert(id.to_string(), path);
                }
                (result, _) => {
                    // A failed optional refresh cannot displace a still-working FFmpeg Shared package.
                    if let Some(path) = previous.filter(|p| p.is_file())
                        && self.find_tool(id, executable, &environment).is_some()
                    {
                        (self.log)(&format!(
                            "Warning: {id} refresh failed ({result:?}); retaining the verified existing package"
                        ));
                        selected.insert(id.to_string(), path);
                        continue;
                    }
                    return Err(format!(
                        "{id} is not usable after installation: {result:?}. Correct the error and retry dependency setup"
                    ));
                }
            }
        }
        Ok(selected)
    }
    fn environment(&mut self, selected: &Tools) -> Result<(), String> {
        super::environment::configure(&self.root, selected, self.log)
    }
    fn resources(&mut self) -> Result<(), String> {
        files::deploy_resources(&self.root)?;
        (self.log)(&format!(
            "Installed and verified {} Weave templates and tools.",
            files::RESOURCES.len()
        ));
        Ok(())
    }
    fn now(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

pub fn system_directory() -> PathBuf {
    std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
        .join("System32")
}
pub fn vswhere() -> Option<PathBuf> {
    ["ProgramFiles(x86)", "ProgramFiles"]
        .iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .map(|root| root.join("Microsoft Visual Studio/Installer/vswhere.exe"))
        .find(|path| path.is_file())
}
fn numeric_version(text: &str) -> Option<Vec<u64>> {
    let values: Vec<_> = text
        .split('.')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    (values.len() >= 2).then_some(values)
}
fn version_directories(root: &Path) -> Vec<PathBuf> {
    let mut directories: Vec<_> = fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            numeric_version(&entry.file_name().to_string_lossy()).map(|v| (v, entry.path()))
        })
        .collect();
    directories.sort_by(|a, b| b.0.cmp(&a.0));
    directories.into_iter().map(|(_, path)| path).collect()
}
fn collect_files(root: &Path, directory: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
    files::no_links(directory)?;
    for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        files::no_links(&path)?;
        if path.is_dir() {
            collect_files(root, &path, output)?;
        } else {
            output.push(
                path.strip_prefix(root)
                    .map_err(|e| e.to_string())?
                    .to_owned(),
            );
        }
    }
    Ok(())
}

pub struct Environment(Value);
impl Environment {
    pub fn read() -> Result<Self, String> {
        let script = "$result=@{}; foreach($scope in @('Machine','User','Process')) { $result[$scope]=[Environment]::GetEnvironmentVariables($scope) }; $result | ConvertTo-Json -Depth 5 -Compress";
        let output = tools::checked(&mut tools::powershell(script), false, &|_| {})?;
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
        assert!(validate_archive(&archive, 32).is_ok());
        assert!(validate_archive(&archive, 33).is_err());
        payload[0] = 0;
        fs::write(&archive, payload).unwrap();
        assert!(validate_archive(&archive, 32).is_err());
    }
    #[test]
    fn shared_ffmpeg_requires_development_headers_libraries_and_dlls() {
        let root = files::Temporary::create().unwrap();
        fs::create_dir_all(root.0.join("bin")).unwrap();
        fs::create_dir_all(root.0.join("lib")).unwrap();
        fs::write(root.0.join("bin/ffmpeg.exe"), "fixture").unwrap();
        assert!(!tools::shared_ffmpeg(&root.0));
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
        assert!(tools::shared_ffmpeg(&root.0));
        fs::remove_file(root.0.join("lib/avdevice.lib")).unwrap();
        assert!(!tools::shared_ffmpeg(&root.0));
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
        let mut command = tools::powershell(
            "New-Item -ItemType Junction -Path $env:WEAVE_ALIAS -Target $env:WEAVE_TARGET | Out-Null",
        );
        command
            .env("WEAVE_ALIAS", &alias)
            .env("WEAVE_TARGET", &actual);
        tools::checked(&mut command, false, &|_| {}).unwrap();
        let environment = Environment(
            serde_json::json!({"Machine":{"PATH":alias.to_string_lossy()},"User":{},"Process":{}}),
        );
        let host = SystemHost {
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
