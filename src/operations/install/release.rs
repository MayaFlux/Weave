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
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|e| e.to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
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
    let installed = if development {
        Channel::Development
    } else {
        Channel::Stable
    };
    (installed != channel).then_some(installed)
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

pub(crate) fn is_current(channel: Channel, release: &Release, local: &Value, now: u64) -> bool {
    let Some((number, dev)) = local
        .get("version")
        .and_then(Value::as_str)
        .and_then(|v| version(v).ok())
    else {
        return false;
    };
    if number < release.version || dev != (channel == Channel::Development) {
        return false;
    }
    if channel == Channel::Stable {
        return true;
    }
    local
        .get("build_time")
        .and_then(Value::as_str)
        .and_then(utc_timestamp)
        .and_then(|build| now.checked_sub(build))
        .is_some_and(|age| age < 86_400)
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
