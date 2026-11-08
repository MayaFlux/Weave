//! Fetches and validates the community registry.

use super::{ModuleAddError, fail, valid_module_name, version};
use serde_json::Value;
use std::process::Command;

pub(super) const REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/MayaFlux/community-sources-registry/main/registry.json";

/// A registry module's repository and minimum MayaFlux version.
#[derive(Clone, Debug)]
pub(super) struct RegistryEntry {
    pub(super) repository: String,
    pub(super) minimum: String,
}

/// Validates registry JSON before any repository is cloned.
pub(super) fn parse_registry(
    source: &str,
) -> Result<std::collections::HashMap<String, RegistryEntry>, ModuleAddError> {
    let data: Value = serde_json::from_str(source.trim_start_matches('\u{FEFF}'))
        .map_err(|error| fail(format!("Invalid community registry JSON: {error}")))?;
    let records = data
        .as_array()
        .ok_or_else(|| fail("Community registry must be a JSON array"))?;
    let mut entries = std::collections::HashMap::new();
    for record in records {
        let name = record
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| fail("Community registry contains an invalid module name"))?;
        let repo = record
            .get("repo")
            .and_then(Value::as_str)
            .ok_or_else(|| fail(format!("Invalid repository for {name}")))?;
        let minimum = record
            .get("min_version")
            .and_then(Value::as_str)
            .ok_or_else(|| fail(format!("Missing minimum version for {name}")))?;
        let valid_name = valid_module_name(name);
        let authority = repo
            .strip_prefix("https://")
            .and_then(|rest| rest.split(['/', '?']).next());
        let valid_repo = authority.is_some_and(|host| !host.is_empty() && !host.contains('@'))
            && !repo.chars().any(char::is_whitespace)
            && !repo.contains('#')
            && !repo.contains('\\');
        if !valid_name || !valid_repo {
            return Err(fail("Community registry contains an invalid entry"));
        }
        version(minimum, &format!("registry entry for {name}"))?;
        if entries
            .insert(
                name.to_owned(),
                RegistryEntry {
                    repository: repo.to_owned(),
                    minimum: minimum.to_owned(),
                },
            )
            .is_some()
        {
            return Err(fail(format!("Duplicate registry entry for {name}")));
        }
    }
    Ok(entries)
}

/// Downloads and parses the community registry.
pub(super) fn registry() -> Result<std::collections::HashMap<String, RegistryEntry>, ModuleAddError>
{
    let output = registry_request()
        .output()
        .map_err(|error| fail(format!("Could not fetch community registry: {error}")))?;
    if !output.status.success() {
        return Err(fail(format!(
            "Could not fetch community registry: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    parse_registry(
        std::str::from_utf8(&output.stdout)
            .map_err(|error| fail(format!("Community registry is not UTF-8: {error}")))?,
    )
}

/// Builds the bounded `curl` request used to retrieve the registry.
pub(super) fn registry_request() -> Command {
    let mut command = Command::new("curl");
    command.args([
        "--fail",
        "--location",
        "--silent",
        "--show-error",
        "--max-time",
        "30",
        REGISTRY_URL,
    ]);
    command
}
