//! Tool identity, verification policy and dependency execution shared by native plans.
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Tool {
    CMake,
    Git,
    Ninja,
    Clang,
    Ffmpeg,
    ShaderCompiler,
}
impl std::fmt::Display for Tool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::CMake => "CMake",
            Self::Git => "Git",
            Self::Ninja => "Ninja",
            Self::Clang => "LLVM",
            Self::Ffmpeg => "FFmpeg Shared",
            Self::ShaderCompiler => "shader compiler",
        })
    }
}
pub type Tools = BTreeMap<Tool, PathBuf>;

pub fn probe_candidates<R: crate::process::ProcessRuntime>(
    runtime: &R,
    tool: Tool,
    candidates: impl IntoIterator<Item = PathBuf>,
) -> Option<PathBuf> {
    for path in candidates {
        if !path.is_file() {
            continue;
        }
        let result = crate::process::run_with(
            runtime,
            std::process::Command::new(&path).arg(if tool == Tool::Ffmpeg {
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
            && accepts_banner(
                tool,
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
#[derive(Clone, Copy)]
pub struct Dependency {
    pub tool: Tool,
    pub refresh_existing: bool,
}
pub trait DependencyHost {
    /// Return only an executable that passed the native discovery and common banner checks.
    fn probe(&self, tool: Tool) -> Result<Option<PathBuf>, String>;
    fn acquire(&self, tool: Tool, previously_usable: bool) -> Result<(), String>;
}
pub fn ensure(
    host: &impl DependencyHost,
    requirements: &[Dependency],
    log: &impl Fn(&str),
) -> Result<Tools, String> {
    let mut selected = Tools::new();
    for requirement in requirements {
        let tool = requirement.tool;
        log(&format!("Checking {tool}..."));
        let previous = host.probe(tool)?;
        if !requirement.refresh_existing
            && let Some(path) = previous
        {
            log(&format!(
                "{tool}: verified existing tool at {}",
                path.display()
            ));
            selected.insert(tool, path);
            continue;
        }
        log(&format!("Installing/checking {tool}..."));
        let result = host.acquire(tool, previous.is_some());
        let refreshed = host.probe(tool)?;
        match (result, refreshed) {
            (Ok(()), Some(path)) => {
                log(&format!(
                    "{tool}: installed tool verified at {}",
                    path.display()
                ));
                selected.insert(tool, path);
            }
            (result, Some(path)) if previous.is_some() => {
                log(&format!(
                    "Warning: {tool} refresh failed ({result:?}); retaining the verified existing package"
                ));
                selected.insert(tool, path);
            }
            (result, _) => {
                return Err(format!(
                    "{tool} is not usable after installation: {result:?}. Correct the error and retry dependency setup"
                ));
            }
        }
    }
    Ok(selected)
}

pub fn accepts_banner(id: Tool, output: &str) -> bool {
    let text = output.to_ascii_lowercase();
    let prefix = match id {
        Tool::CMake => "cmake version ",
        Tool::Git => "git version ",
        Tool::Clang => "clang version ",
        Tool::Ffmpeg => "ffmpeg version ",
        Tool::ShaderCompiler => "shaderc ",
        Tool::Ninja => "",
    };
    let Some(banner) = text
        .split_once(prefix)
        .map(|(_, value)| value)
        .filter(|_| !prefix.is_empty())
        .or_else(|| (id == Tool::Ninja).then_some(text.as_str()))
    else {
        return false;
    };
    let token = banner.split_whitespace().next().unwrap_or("");
    if id == Tool::Ffmpeg {
        return token.bytes().any(|b| b.is_ascii_digit());
    }
    let token = token.trim_start_matches('v');
    let numbers: Vec<_> = token
        .split('.')
        .take(2)
        .map(|part| part.parse::<u64>())
        .collect();
    if numbers.len() != 2 || numbers.iter().any(Result::is_err) {
        return false;
    }
    id != Tool::CMake
        || [
            numbers[0].as_ref().copied().unwrap(),
            numbers[1].as_ref().copied().unwrap(),
        ] >= [3, 31]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_working_supported_tool_banners_are_accepted() {
        for (id, banner) in [
            (Tool::CMake, "cmake version 4.3.0-rc1"),
            (Tool::Git, "git version 2.39.5.windows.1"),
            (Tool::Ninja, "1.10.2.git.kitware.jobserver-1"),
            (Tool::Clang, "Vendor LLVM clang version 20.1.8"),
            (Tool::Ffmpeg, "ffmpeg version N-123456-gabcdef"),
            (Tool::ShaderCompiler, "shaderc 2023.2\nspirv-tools v2023.2"),
        ] {
            assert!(accepts_banner(id, banner), "{id}");
            assert!(!accepts_banner(id, "unrelated output"));
        }
        assert!(!accepts_banner(Tool::CMake, "cmake version 3.25.0"));
    }

    use std::cell::Cell;
    struct Fixture {
        ready: Cell<bool>,
        fail_install: bool,
        retains_existing: bool,
    }
    impl DependencyHost for Fixture {
        fn probe(&self, _: Tool) -> Result<Option<PathBuf>, String> {
            Ok(self.ready.get().then(|| PathBuf::from("verified-tool")))
        }
        fn acquire(&self, _: Tool, _: bool) -> Result<(), String> {
            if self.fail_install {
                self.ready.set(self.retains_existing);
                Err("package acquisition failed".into())
            } else {
                self.ready.set(true);
                Ok(())
            }
        }
    }
    #[test]
    fn failed_refresh_preserves_a_reverified_existing_tool() {
        let fixture = Fixture {
            ready: Cell::new(true),
            fail_install: true,
            retains_existing: true,
        };
        let tools = ensure(
            &fixture,
            &[Dependency {
                tool: Tool::Ffmpeg,
                refresh_existing: true,
            }],
            &|_| {},
        )
        .unwrap();
        assert_eq!(tools[&Tool::Ffmpeg], PathBuf::from("verified-tool"));
    }
    #[test]
    fn failed_acquisition_blocks_progress_and_retry_requires_verified_tool() {
        let mut fixture = Fixture {
            ready: Cell::new(false),
            fail_install: true,
            retains_existing: false,
        };
        let requirement = [Dependency {
            tool: Tool::Clang,
            refresh_existing: false,
        }];
        assert!(
            ensure(&fixture, &requirement, &|_| {})
                .unwrap_err()
                .contains("LLVM")
        );
        fixture.fail_install = false;
        assert!(
            ensure(&fixture, &requirement, &|_| {})
                .unwrap()
                .contains_key(&Tool::Clang)
        );
    }
}
