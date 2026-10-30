//! Release streams, metadata, version comparisons and freshness policy.
use crate::{download::Downloader, filesystem};
use serde_json::Value;
use std::{fs, path::Path};

pub fn fetch(
    channel: Channel,
    downloader: &impl Downloader,
    log: &(impl Fn(&str) + Sync),
) -> Result<Value, String> {
    let temporary = filesystem::Temporary::create()?;
    let metadata = temporary.0.join("release.json");
    downloader.download(
        if channel == Channel::Stable {
            "https://api.github.com/repos/MayaFlux/MayaFlux/releases/latest"
        } else {
            "https://api.github.com/repos/MayaFlux/MayaFlux/releases"
        },
        &metadata,
        16 * 1024 * 1024,
        log,
    )?;
    serde_json::from_slice(&fs::read(metadata).map_err(|e| e.to_string())?)
        .map_err(|e| format!("Invalid GitHub release metadata: {e}"))
}
pub fn installed(root: &Path) -> Result<Option<Value>, String> {
    let path = root.join("share/MayaFlux/.version");
    filesystem::no_links(&path)?;
    let mut local: Value = match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| e.to_string())?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    let receipt_path = root.join("share/weave/mayaflux-install.json");
    filesystem::no_links(&receipt_path)?;
    let receipt = match fs::read(receipt_path) {
        Ok(bytes) => Some(
            serde_json::from_slice::<Value>(&bytes)
                .map_err(|e| format!("Cannot read Weave's installed-release record: {e}"))?,
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.to_string()),
    };
    // A receipt belongs only to the payload whose metadata was recorded.
    // Manual replacement must not inherit the previous installation's stream.
    let recorded_release = receipt
        .as_ref()
        .filter(|receipt| receipt.get("payload") == Some(&local))
        .and_then(|receipt| receipt.get("release"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    if let Some(object) = local.as_object_mut() {
        object.remove("weave_release");
        if let Some(tag) = recorded_release {
            object.insert("weave_release".into(), tag.into());
        }
    }
    Ok(Some(local))
}

/// Record release provenance without changing MayaFlux's own .version metadata.
/// Called on the staged payload before replacing an existing installation.
pub(crate) fn record_install(root: &Path, release: &Release) -> Result<(), String> {
    let marker = root.join("share/MayaFlux/.version");
    filesystem::no_links(&marker)?;
    let payload: Value = serde_json::from_slice(
        &fs::read(&marker)
            .map_err(|e| format!("Cannot read staged MayaFlux version metadata: {e}"))?,
    )
    .map_err(|e| format!("Invalid staged MayaFlux version metadata: {e}"))?;
    let payload_version = payload
        .get("version")
        .and_then(Value::as_str)
        .ok_or("Staged MayaFlux metadata has no version")?;
    if version(payload_version)?.0 != release.version || version(&release.tag)?.0 != release.version
    {
        return Err("Staged MayaFlux version does not match the selected release".into());
    }
    let path = root.join("share/weave/mayaflux-install.json");
    filesystem::no_links(&path)?;
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    let receipt = serde_json::json!({"release":release.tag,"payload":payload});
    filesystem::atomic_write(&path, receipt.to_string().as_bytes()).map_err(|e| e.to_string())
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Channel {
    Stable,
    Development,
}

impl Channel {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "stable" => Ok(Self::Stable),
            "development" => Ok(Self::Development),
            _ => Err(format!("Unsupported MayaFlux install channel: {value}")),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Development => "development",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Release {
    pub tag: String,
    pub url: String,
    pub size: u64,
    pub version: [u64; 3],
}

pub fn conflicting_stream(channel: Channel, local: &Value) -> Option<Channel> {
    let (_, development) = local
        .get("version")
        .and_then(Value::as_str)
        .and_then(|text| version(text).ok())?;
    let installed = installed_channel(local, development);
    (installed != channel).then_some(installed)
}

fn installed_channel(local: &Value, development: bool) -> Channel {
    let recorded = local
        .get("weave_release")
        .and_then(Value::as_str)
        .and_then(|tag| version(tag).ok())
        .filter(|(number, _)| {
            local
                .get("version")
                .and_then(Value::as_str)
                .and_then(|text| version(text).ok())
                .is_some_and(|(local, _)| local == *number)
        });
    if recorded.map_or(development, |(_, dev)| dev) {
        Channel::Development
    } else {
        Channel::Stable
    }
}

pub fn version(text: &str) -> Result<([u64; 3], bool), String> {
    let text = text.strip_prefix('v').unwrap_or(text);
    let dev = text.ends_with("-dev");
    let number = text.strip_suffix("-dev").unwrap_or(text);
    let parts: Vec<_> = number.split('.').collect();
    if parts.len() != 3
        || parts
            .iter()
            .any(|v| v.is_empty() || !v.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(format!("Invalid MayaFlux version: {text}"));
    }
    let mut result = [0; 3];
    for (index, part) in parts.iter().enumerate() {
        result[index] = part
            .parse()
            .map_err(|_| format!("Invalid MayaFlux version: {text}"))?;
    }
    Ok((result, dev))
}

pub fn select_release_for(
    channel: Channel,
    metadata: &Value,
    suffix: &str,
) -> Result<Release, String> {
    let selected = match channel {
        Channel::Stable => metadata,
        Channel::Development => metadata
            .as_array()
            .ok_or("Development releases must be an array")?
            .iter()
            .find(|r| {
                r.get("draft").and_then(Value::as_bool) == Some(false)
                    && r.get("tag_name")
                        .and_then(Value::as_str)
                        .is_some_and(|t| t.ends_with("-dev"))
            })
            .ok_or("No non-draft development release found")?,
    };
    if selected.get("draft").and_then(Value::as_bool) != Some(false)
        || channel == Channel::Stable
            && selected.get("prerelease").and_then(Value::as_bool) != Some(false)
    {
        return Err("Release is draft, malformed, or not in the selected stream".into());
    }
    let tag = selected
        .get("tag_name")
        .and_then(Value::as_str)
        .ok_or("Release has no tag_name")?;
    let (number, dev) = version(tag)?;
    if dev != (channel == Channel::Development) {
        return Err("Release does not match the selected stream".into());
    }
    let name = format!("MayaFlux-{}-{suffix}", tag.strip_prefix('v').unwrap_or(tag));
    let assets = selected
        .get("assets")
        .and_then(Value::as_array)
        .ok_or("Release has no assets array")?;
    let matches: Vec<_> = assets
        .iter()
        .filter(|a| a.get("name").and_then(Value::as_str) == Some(&name))
        .collect();
    if matches.len() != 1 {
        return Err(
            "Expected exactly one asset for the selected target, version and stream".into(),
        );
    }
    let asset = matches[0];
    let expected = format!("https://github.com/MayaFlux/MayaFlux/releases/download/{tag}/{name}");
    let url = asset
        .get("browser_download_url")
        .and_then(Value::as_str)
        .ok_or("Missing archive URL")?;
    if url != expected {
        return Err(
            "Archive URL is not the HTTPS download for this MayaFlux release and asset".into(),
        );
    }
    let size = asset
        .get("size")
        .and_then(Value::as_u64)
        .filter(|s| *s > 0)
        .ok_or("Invalid archive size")?;
    Ok(Release {
        tag: tag.to_owned(),
        url: url.to_owned(),
        size,
        version: number,
    })
}

pub(crate) fn is_current(
    channel: Channel,
    release: &Release,
    local: &Value,
    now: u64,
    log: &impl Fn(&str),
) -> bool {
    let Some((number, dev)) = local
        .get("version")
        .and_then(Value::as_str)
        .and_then(|v| version(v).ok())
    else {
        log("Installed MayaFlux version metadata is invalid; downloading the selected release.");
        return false;
    };
    log(&format!(
        "Local version: {}. Selected release: {}.",
        local["version"].as_str().unwrap(),
        release.tag
    ));
    if number < release.version {
        log("The selected release is newer; updating MayaFlux.");
        return false;
    }
    if installed_channel(local, dev) != channel {
        log("The installed stream differs from the requested stream; replacing the payload.");
        return false;
    }
    if channel == Channel::Stable {
        return true;
    }
    let Some(build) = local
        .get("build_time")
        .and_then(Value::as_str)
        .and_then(utc_timestamp)
    else {
        log("Development build_time is missing or invalid; downloading the selected release.");
        return false;
    };
    let Some(age) = now.checked_sub(build) else {
        log("Development build_time is in the future; downloading the selected release.");
        return false;
    };
    log(&format!(
        "Build age: {:.1}h (threshold: 24h).",
        age as f64 / 3600.0
    ));
    if age < 86_400 {
        log("Development build is fresh; verifying the existing payload.");
        true
    } else {
        log("Development build is stale; updating MayaFlux.");
        false
    }
}

fn utc_timestamp(text: &str) -> Option<u64> {
    // The release .version contract uses exactly yyyy-MM-dd HH:mm:ss UTC.
    if text.len() != 23 || !text.ends_with(" UTC") {
        return None;
    }
    let fields: Vec<_> = text[..19]
        .split(['-', ' ', ':'])
        .map(str::parse::<u64>)
        .collect();
    if fields.len() != 6 {
        return None;
    }
    let values: Vec<_> = fields.into_iter().collect::<Result<_, _>>().ok()?;
    let (year, month, day, hour, minute, second) = (
        values[0], values[1], values[2], values[3], values[4], values[5],
    );
    if !(1970..=9999).contains(&year)
        || !(1..=12).contains(&month)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    let leap = |y: u64| y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400));
    let days_in_month = [
        31,
        if leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if day == 0 || day > days_in_month[month as usize - 1] {
        return None;
    }
    let days: u64 = (1970..year)
        .map(|y| if leap(y) { 366 } else { 365 })
        .sum::<u64>()
        + days_in_month[..month as usize - 1].iter().sum::<u64>()
        + day
        - 1;
    Some(days * 86_400 + hour * 3600 + minute * 60 + second)
}

#[cfg(test)]
mod target_tests {
    use super::*;
    #[test]
    fn installed_development_release_keeps_its_stream_when_payload_version_has_no_suffix() {
        let temporary = filesystem::Temporary::create().unwrap();
        let marker = temporary.0.join("share/MayaFlux/.version");
        fs::create_dir_all(marker.parent().unwrap()).unwrap();
        let payload = serde_json::json!({"version":"0.5.0", "build_time":"2026-10-13 11:00:00 UTC", "commit":"abc123"});
        fs::write(&marker, payload.to_string()).unwrap();
        let release = Release {
            tag: "v0.5.0-dev".into(),
            url: "https://example.invalid/package.7z".into(),
            size: 32,
            version: [0, 5, 0],
        };
        record_install(&temporary.0, &release).unwrap();
        let local = installed(&temporary.0).unwrap().unwrap();
        assert_eq!(conflicting_stream(Channel::Development, &local), None);
        assert_eq!(
            conflicting_stream(Channel::Stable, &local),
            Some(Channel::Development)
        );
        assert!(is_current(
            Channel::Development,
            &release,
            &local,
            1_791_892_800,
            &|_| {}
        ));
        assert_eq!(
            serde_json::from_slice::<Value>(&fs::read(&marker).unwrap()).unwrap(),
            payload
        );

        // A manual payload replacement invalidates the old stream receipt.
        let mut changed = payload;
        changed["commit"] = "different".into();
        fs::write(&marker, changed.to_string()).unwrap();
        let local = installed(&temporary.0).unwrap().unwrap();
        assert!(local.get("weave_release").is_none());

        // A different stream/version must never inherit the development receipt.
        assert_eq!(
            conflicting_stream(Channel::Development, &local),
            Some(Channel::Stable)
        );
        let receipt = temporary.0.join("share/weave/mayaflux-install.json");
        fs::write(receipt, "invalid JSON").unwrap();
        assert!(
            installed(&temporary.0)
                .unwrap_err()
                .contains("installed-release record")
        );
    }
    #[test]
    fn recording_a_release_requires_matching_payload_metadata() {
        let temporary = filesystem::Temporary::create().unwrap();
        let marker = temporary.0.join("share/MayaFlux/.version");
        fs::create_dir_all(marker.parent().unwrap()).unwrap();
        let release = Release {
            tag: "v0.5.0-dev".into(),
            url: "https://example.invalid/package.7z".into(),
            size: 32,
            version: [0, 5, 0],
        };
        for source in ["invalid JSON", r#"{"version":"0.6.0"}"#, r#"{}"#] {
            fs::write(&marker, source).unwrap();
            assert!(record_install(&temporary.0, &release).is_err());
            assert!(
                !temporary
                    .0
                    .join("share/weave/mayaflux-install.json")
                    .exists()
            );
        }
    }
    #[test]
    fn explicit_stable_receipt_keeps_cross_stream_replacement_required() {
        let local = serde_json::json!({"version":"0.5.0", "weave_release":"v0.5.0"});
        assert_eq!(
            conflicting_stream(Channel::Development, &local),
            Some(Channel::Stable)
        );
        assert_eq!(conflicting_stream(Channel::Stable, &local), None);
    }
    #[test]
    fn stream_conflicts_are_shared_policy_in_both_directions() {
        let stable = serde_json::json!({"version":"0.5.0"});
        let development = serde_json::json!({"version":"0.5.0-dev"});
        assert_eq!(
            conflicting_stream(Channel::Stable, &development),
            Some(Channel::Development)
        );
        assert_eq!(
            conflicting_stream(Channel::Development, &stable),
            Some(Channel::Stable)
        );
        assert_eq!(conflicting_stream(Channel::Stable, &stable), None);
        assert_eq!(
            conflicting_stream(Channel::Stable, &serde_json::json!({"version":"invalid"})),
            None
        );
    }
    #[test]
    fn native_target_selects_payload_without_changing_shared_release_policy() {
        let metadata = serde_json::json!({"tag_name":"v0.5.0","draft":false,"prerelease":false,"assets":[{"name":"MayaFlux-0.5.0-linux-x64.tar.gz","size":32,"browser_download_url":"https://github.com/MayaFlux/MayaFlux/releases/download/v0.5.0/MayaFlux-0.5.0-linux-x64.tar.gz"}]});
        let selected = select_release_for(Channel::Stable, &metadata, "linux-x64.tar.gz").unwrap();
        assert!(selected.url.ends_with("linux-x64.tar.gz"));
    }
}
