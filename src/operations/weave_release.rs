//! Stable Weave release notifications, independent of MayaFlux installation.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const RELEASE_API: &str = "https://api.github.com/repos/MayaFlux/Weave/releases/latest";
const CHECK_INTERVAL: u64 = 24 * 60 * 60;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseNotice {
    pub version: String,
    pub url: String,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
pub struct CheckState {
    pub last_check: u64,
    pub latest_version: String,
    pub dismissed_version: String,
}

fn numeric_version(value: &str) -> Option<[u64; 3]> {
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() != 3
        || parts
            .iter()
            .any(|part| part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    Some([
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
    ])
}

fn notice_for(current: &str, latest: &str) -> Option<ReleaseNotice> {
    (numeric_version(latest)? > numeric_version(current)?).then(|| ReleaseNotice {
        version: latest.to_owned(),
        url: format!("https://github.com/MayaFlux/Weave/releases/tag/v{latest}"),
    })
}

pub fn release_notice(current: &str, metadata: &Value) -> Option<ReleaseNotice> {
    notice_for(current, stable_version(metadata)?)
}

fn stable_version(metadata: &Value) -> Option<&str> {
    if metadata.get("draft")?.as_bool()? || metadata.get("prerelease")?.as_bool()? {
        return None;
    }
    let latest = metadata.get("tag_name")?.as_str()?.strip_prefix('v')?;
    numeric_version(latest)?;
    Some(latest)
}

impl CheckState {
    pub fn should_check(&self, now: u64) -> bool {
        now.saturating_sub(self.last_check) >= CHECK_INTERVAL || self.last_check > now
    }

    pub fn notice(&self, current: &str) -> Option<ReleaseNotice> {
        if self.latest_version == self.dismissed_version {
            return None;
        }
        notice_for(current, &self.latest_version)
    }

    fn load(path: &Path) -> Self {
        fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    fn save(&self, path: &Path) {
        if crate::filesystem::no_links(path).is_err() {
            return;
        }
        if let Some(parent) = path.parent() {
            if fs::create_dir_all(parent).is_ok() {
                if let Ok(bytes) = serde_json::to_vec(self) {
                    let _ = crate::filesystem::atomic_write(path, &bytes);
                }
            }
        }
    }
}

fn state_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("LOCALAPPDATA")
            .map(|root| PathBuf::from(root).join("Weave/update-check.json"))
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").map(|root| {
            PathBuf::from(root).join("Library/Application Support/Weave/update-check.json")
        })
    }
    #[cfg(target_os = "linux")]
    {
        let root = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
        Some(root.join("weave/update-check.json"))
    }
}

fn fetch_latest() -> Option<Value> {
    #[cfg(target_os = "windows")]
    let executable = crate::platform::windows::process::system_directory().join("curl.exe");
    #[cfg(not(target_os = "windows"))]
    let executable = "curl";
    let output = Command::new(executable)
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--connect-timeout",
            "5",
            "--max-time",
            "10",
            "--max-filesize",
            "65536",
            "--header",
            "Accept: application/vnd.github+json",
            "--header",
            "User-Agent: Weave-update-check",
            RELEASE_API,
        ])
        .output()
        .ok()?;
    if !output.status.success() || output.stdout.len() > 65_536 {
        return None;
    }
    serde_json::from_slice(&output.stdout).ok()
}

pub fn check_on_startup(current: &str) -> Option<ReleaseNotice> {
    let path = state_path()?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    check_with(&path, now, current, fetch_latest)
}

pub(crate) fn check_with(
    path: &Path,
    now: u64,
    current: &str,
    fetch: impl FnOnce() -> Option<Value>,
) -> Option<ReleaseNotice> {
    let mut state = CheckState::load(path);
    if state.should_check(now) {
        if let Some(metadata) = fetch() {
            if let Some(version) = stable_version(&metadata) {
                state.latest_version = version.to_owned();
                state.last_check = now;
                state.save(path);
            }
        }
    }
    state.notice(current)
}

pub fn dismiss(version: &str) {
    if let Some(path) = state_path() {
        let mut state = CheckState::load(&path);
        state.dismissed_version = version.to_owned();
        state.save(&path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dismissed_version_remains_hidden_after_state_is_reloaded() {
        let directory = std::env::temp_dir().join(format!(
            "weave-update-check-{}",
            crate::filesystem::unique_id()
        ));
        let path = directory.join("update-check.json");
        let state = CheckState {
            latest_version: "0.7.0".into(),
            dismissed_version: "0.7.0".into(),
            last_check: 1_000_000,
        };
        state.save(&path);
        let restored = CheckState::load(&path);
        assert!(restored.notice("0.6.0").is_none());
        assert_eq!(restored.last_check, 1_000_000);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn fake_stable_release_produces_notice_and_dismissal_suppresses_it() {
        let directory = std::env::temp_dir().join(format!(
            "weave-update-check-{}",
            crate::filesystem::unique_id()
        ));
        let path = directory.join("update-check.json");
        let fake_release = serde_json::json!({
            "tag_name": "v0.7.0", "draft": false, "prerelease": false,
            "html_url": "https://github.com/MayaFlux/Weave/releases/tag/v0.7.0"
        });
        let notice = check_with(&path, 1_000_000, "0.6.0", || Some(fake_release)).unwrap();
        assert_eq!(notice.version, "0.7.0");
        assert_eq!(
            notice.url,
            "https://github.com/MayaFlux/Weave/releases/tag/v0.7.0"
        );

        let mut state = CheckState::load(&path);
        state.dismissed_version = "0.7.0".into();
        state.save(&path);
        assert!(check_with(&path, 1_000_001, "0.6.0", || panic!("checked too soon")).is_none());
        fs::remove_dir_all(directory).unwrap();
    }
}
