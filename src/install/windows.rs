//! Windows installation policy. Host effects are supplied by the platform adapter.
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;

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

pub type Tools = BTreeMap<String, PathBuf>;

pub trait Host {
    fn prerequisites(&mut self) -> Result<bool, String>;
    fn releases(&mut self, channel: Channel) -> Result<Value, String>;
    fn installed(&mut self) -> Result<Option<Value>, String>;
    fn verify(&mut self) -> Result<(), String>;
    fn acquire(&mut self, release: &Release) -> Result<(), String>;
    fn dependencies(&mut self) -> Result<Tools, String>;
    fn environment(&mut self, tools: &Tools) -> Result<(), String>;
    fn resources(&mut self) -> Result<(), String>;
    fn now(&self) -> u64;
}

pub fn install(
    channel: Channel,
    host: &mut impl Host,
    log: &impl Fn(&str),
) -> Result<String, String> {
    let stage = |name: &str, error: String| {
        format!(
            "{name}: {error}. Installation is incomplete; earlier steps may have changed the host. Correct the reported problem and retry Install MayaFlux. Project and community operations remain available."
        )
    };
    log("Step 1/6: Checking x64 Windows, 7-Zip and Microsoft C++ prerequisites...");
    let reboot = host
        .prerequisites()
        .map_err(|e| stage("System prerequisites", e))?;
    log(&format!(
        "Step 2/6: Checking the {} MayaFlux stream...",
        channel.name()
    ));
    let metadata = host
        .releases(channel)
        .map_err(|e| stage("Release metadata", e))?;
    let release = select_release(channel, &metadata).map_err(|e| stage("Release metadata", e))?;
    let local = match host.installed() {
        Ok(local) => local,
        Err(error) => {
            log(&format!(
                "Warning: could not read installed version: {error}; downloading the selected release."
            ));
            None
        }
    };
    let current = local
        .as_ref()
        .is_some_and(|local| is_current(channel, &release, local, host.now()));
    let configured = current && host.verify().is_ok();
    if configured {
        log("The existing MayaFlux payload is current and verified; continuing with host setup.");
    } else {
        host.acquire(&release)
            .map_err(|e| stage("MayaFlux acquisition", e))?;
        host.verify()
            .map_err(|e| stage("MayaFlux verification", e))?;
    }
    log("Step 3/6: Verifying/installing CMake, Git, Ninja, LLVM, FFmpeg Shared and Vulkan SDK...");
    let tools = host
        .dependencies()
        .map_err(|e| stage("Build dependencies", e))?;
    log("Step 4/6: Configuring MayaFlux and dependency environment...");
    host.environment(&tools)
        .map_err(|e| stage("Environment configuration", e))?;
    log("Step 5/6: Installing and verifying Weave templates and tools...");
    host.resources()
        .map_err(|e| stage("Templates and tools", e))?;
    log("Step 6/6: Installation complete.");
    Ok(format!(
        "MayaFlux {} {} and verified. {} Open an x64 Visual Studio developer shell in your project and run `cmake --preset release`, then `cmake --build --preset release`.",
        channel.name(),
        if configured {
            "configured"
        } else {
            "installed"
        },
        if reboot {
            "Restart Windows before building projects."
        } else {
            "Restart your terminal before building projects."
        }
    ))
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

pub fn select_release(channel: Channel, metadata: &Value) -> Result<Release, String> {
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
    let name = format!(
        "MayaFlux-{}-windows-x64.7z",
        tag.strip_prefix('v').unwrap_or(tag)
    );
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
            "Expected exactly one Windows x64 asset for the selected version and stream".into(),
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
        .filter(|s| *s >= 32)
        .ok_or("Invalid archive size")?;
    Ok(Release {
        tag: tag.to_owned(),
        url: url.to_owned(),
        size,
        version: number,
    })
}

fn is_current(channel: Channel, release: &Release, local: &Value, now: u64) -> bool {
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
mod tests {
    use super::*;
    use serde_json::json;

    struct Fake {
        calls: Vec<&'static str>,
        metadata: Value,
        local: Option<Value>,
        fail: Option<&'static str>,
        reboot: bool,
    }
    impl Fake {
        fn new() -> Self {
            Self {
                calls: vec![],
                metadata: release("v0.5.0"),
                local: None,
                fail: None,
                reboot: false,
            }
        }
        fn step(&mut self, name: &'static str) -> Result<(), String> {
            self.calls.push(name);
            if self.fail == Some(name) {
                Err(format!("{name} failed"))
            } else {
                Ok(())
            }
        }
    }
    fn release(tag: &str) -> Value {
        let name = format!("MayaFlux-{}-windows-x64.7z", &tag[1..]);
        json!({"tag_name": tag, "draft": false, "prerelease": tag.ends_with("-dev"),
            "assets": [{"name": name, "size": 32, "browser_download_url": format!("https://github.com/MayaFlux/MayaFlux/releases/download/{tag}/{name}")}]})
    }
    impl Host for Fake {
        fn prerequisites(&mut self) -> Result<bool, String> {
            self.step("prerequisites")?;
            Ok(self.reboot)
        }
        fn releases(&mut self, _: Channel) -> Result<Value, String> {
            self.step("metadata")?;
            Ok(self.metadata.clone())
        }
        fn installed(&mut self) -> Result<Option<Value>, String> {
            self.step("installed")?;
            Ok(self.local.clone())
        }
        fn verify(&mut self) -> Result<(), String> {
            self.step("verify")
        }
        fn acquire(&mut self, r: &Release) -> Result<(), String> {
            assert!(r.url.ends_with("windows-x64.7z"));
            self.step("acquire")
        }
        fn dependencies(&mut self) -> Result<Tools, String> {
            self.step("dependencies")?;
            Ok(BTreeMap::from([(
                "LLVM.LLVM".into(),
                PathBuf::from("D:/Custom LLVM/bin/clang.exe"),
            )]))
        }
        fn environment(&mut self, tools: &Tools) -> Result<(), String> {
            assert_eq!(
                tools["LLVM.LLVM"],
                PathBuf::from("D:/Custom LLVM/bin/clang.exe")
            );
            self.step("environment")
        }
        fn resources(&mut self) -> Result<(), String> {
            self.step("resources")
        }
        fn now(&self) -> u64 {
            1_791_892_800
        } // 2026-10-13 12:00 UTC
    }

    #[test]
    fn stable_runs_every_stage_in_order() {
        let mut host = Fake::new();
        assert!(install(Channel::Stable, &mut host, &|_| {}).is_ok());
        assert_eq!(
            host.calls,
            [
                "prerequisites",
                "metadata",
                "installed",
                "acquire",
                "verify",
                "dependencies",
                "environment",
                "resources"
            ]
        );
    }
    #[test]
    fn verified_current_install_still_configures_dependencies_environment_and_resources() {
        let mut host = Fake::new();
        host.local = Some(json!({"version":"0.5.0"}));
        assert!(install(Channel::Stable, &mut host, &|_| {}).is_ok());
        assert!(!host.calls.contains(&"acquire"));
        assert!(
            host.calls
                .ends_with(&["dependencies", "environment", "resources"])
        );
    }
    #[test]
    fn development_selects_dev_after_stable_and_draft_entries() {
        let mut host = Fake::new();
        let mut draft = release("v0.6.0-dev");
        draft["draft"] = json!(true);
        host.metadata = json!([release("v0.6.0"), draft, release("v0.5.0-dev")]);
        assert!(install(Channel::Development, &mut host, &|_| {}).is_ok());
        assert!(host.calls.contains(&"acquire"));
    }
    #[test]
    fn failures_stop_later_stages_and_retry_runs_the_workflow_again() {
        for stage in [
            "prerequisites",
            "metadata",
            "acquire",
            "verify",
            "dependencies",
            "environment",
            "resources",
        ] {
            let mut host = Fake::new();
            host.fail = Some(stage);
            let error = install(Channel::Stable, &mut host, &|_| {}).unwrap_err();
            assert!(error.contains(stage), "{error}");
            assert_eq!(host.calls.last(), Some(&stage));
            host.fail = None;
            host.calls.clear();
            assert!(install(Channel::Stable, &mut host, &|_| {}).is_ok());
        }
    }
    #[test]
    fn invalid_or_wrong_channel_metadata_never_acquires_a_payload() {
        for metadata in [
            json!(null),
            json!([]),
            release("v0.5.0-dev"),
            json!({"tag_name":"v0.5.0","assets":[]}),
        ] {
            let mut host = Fake::new();
            host.metadata = metadata;
            assert!(install(Channel::Stable, &mut host, &|_| {}).is_err());
            assert!(!host.calls.contains(&"acquire"));
        }
    }
    #[test]
    fn untrusted_download_locations_and_zero_sizes_are_rejected() {
        for url in [
            "http://github.com/package.7z",
            "https://attacker.example/package.7z",
            "https://github.com/Other/release.7z",
        ] {
            let mut host = Fake::new();
            host.metadata["assets"][0]["browser_download_url"] = json!(url);
            assert!(install(Channel::Stable, &mut host, &|_| {}).is_err());
            assert!(!host.calls.contains(&"acquire"));
        }
        let mut host = Fake::new();
        host.metadata["assets"][0]["size"] = json!(0);
        assert!(install(Channel::Stable, &mut host, &|_| {}).is_err());
    }
    #[test]
    fn numeric_versions_and_stable_development_switches_trigger_acquisition() {
        for local in ["0.5.0", "0.10.0-dev", "malformed"] {
            let mut host = Fake::new();
            host.metadata = release("v0.10.0");
            host.local = Some(json!({"version":local}));
            assert!(install(Channel::Stable, &mut host, &|_| {}).is_ok());
            assert!(host.calls.contains(&"acquire"));
        }
    }
    #[test]
    fn development_freshness_uses_strict_24_hour_utc_boundary() {
        for (timestamp, acquire) in [
            ("2026-10-12 12:00:01 UTC", false),
            ("2026-10-12 12:00:00 UTC", true),
            ("2026-10-14 12:00:00 UTC", true),
            ("invalid", true),
        ] {
            let mut host = Fake::new();
            host.metadata = json!([release("v0.5.0-dev")]);
            host.local = Some(json!({"version":"0.5.0-dev", "build_time":timestamp}));
            assert!(install(Channel::Development, &mut host, &|_| {}).is_ok());
            assert_eq!(host.calls.contains(&"acquire"), acquire, "{timestamp}");
        }
    }
    #[test]
    fn malformed_unicode_timestamp_cannot_panic_or_suppress_a_download() {
        let mut host = Fake::new();
        host.metadata = json!([release("v0.5.0-dev")]);
        host.local = Some(json!({"version":"0.5.0-dev", "build_time":"aaaaaaaaaaaaaaaaaaé UTC"}));
        assert!(install(Channel::Development, &mut host, &|_| {}).is_ok());
        assert!(host.calls.contains(&"acquire"));
    }
    #[test]
    fn completion_discloses_restart_and_configured_existing_installation() {
        let mut host = Fake::new();
        host.reboot = true;
        host.local = Some(json!({"version":"0.5.0"}));
        let result = install(Channel::Stable, &mut host, &|_| {}).unwrap();
        assert!(result.contains("Restart Windows"));
        assert!(result.contains("configured"));
    }
}
