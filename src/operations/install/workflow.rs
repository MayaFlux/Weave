//! Shared installation lifecycle; native adapters supply host effects.
use super::progress::Stage;
pub use super::release::{Channel, Release, version};
use super::release::{is_current, select_release_for};
use serde_json::Value;
#[cfg(test)]
use std::collections::BTreeMap;
#[cfg(test)]
use std::path::PathBuf;

pub use super::dependencies::{Tool, Tools};

pub trait Host {
    fn asset_suffix(&self) -> &str;
    fn prerequisites(&mut self) -> Result<bool, String>;
    fn releases(&mut self, channel: Channel) -> Result<Value, String>;
    fn installed(&mut self) -> Result<Option<Value>, String>;
    fn verify(&mut self) -> Result<(), String>;
    fn acquire(&mut self, release: &Release) -> Result<(), String>;
    fn dependencies(&mut self) -> Result<Tools, String>;
    fn environment(&mut self, tools: &Tools) -> Result<(), String>;
    fn resources(&mut self) -> Result<(), String>;
    fn now(&self) -> u64;
    fn completion_advice(&self, reboot: bool) -> String {
        format!(
            "{} Build your project with `cmake --preset release`, then `cmake --build --preset release`.",
            if reboot {
                "Restart your computer before building projects."
            } else {
                "Restart your terminal before building projects."
            }
        )
    }
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
    Stage::Prerequisites.report("Checking platform prerequisites...", log);
    let reboot = host
        .prerequisites()
        .map_err(|e| stage("System prerequisites", e))?;
    Stage::Framework.report(
        &format!("Checking the {} MayaFlux stream...", channel.name()),
        log,
    );
    let metadata = host
        .releases(channel)
        .map_err(|e| stage("Release metadata", e))?;
    let release = select_release_for(channel, &metadata, host.asset_suffix())
        .map_err(|e| stage("Release metadata", e))?;
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
    Stage::Dependencies.report("Verifying/installing project build dependencies...", log);
    let tools = host
        .dependencies()
        .map_err(|e| stage("Build dependencies", e))?;
    Stage::Environment.report("Configuring MayaFlux and dependency environment...", log);
    host.environment(&tools)
        .map_err(|e| stage("Environment configuration", e))?;
    Stage::Resources.report("Installing and verifying Weave templates and tools...", log);
    host.resources()
        .map_err(|e| stage("Templates and tools", e))?;
    Stage::Completion.report("Installation complete.", log);
    Ok(format!(
        "MayaFlux {} {} and verified. {}",
        channel.name(),
        if configured {
            "configured"
        } else {
            "installed"
        },
        host.completion_advice(reboot)
    ))
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
        fn asset_suffix(&self) -> &str {
            "windows-x64.7z"
        }
        fn completion_advice(&self, reboot: bool) -> String {
            if reboot {
                "Restart Windows before building projects.".into()
            } else {
                "Restart your terminal before building projects.".into()
            }
        }
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
                Tool::Clang,
                PathBuf::from("D:/Custom LLVM/bin/clang.exe"),
            )]))
        }
        fn environment(&mut self, tools: &Tools) -> Result<(), String> {
            assert_eq!(
                tools[&Tool::Clang],
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
