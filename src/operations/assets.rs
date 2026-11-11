//! Shared MayaFlux and community asset manifest rules and acquisition.

mod fetch;
pub use fetch::{ApplyResult, AssetTransport, HostTransport, apply_assets_with};

use crate::operations::community_manifest::parse_json_strict;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::Path;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetContext {
    Shipped,
    Community,
}

#[derive(Debug)]
pub struct AssetError(pub String);
impl Display for AssetError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for AssetError {}
fn fail(message: impl Into<String>) -> AssetError {
    AssetError(message.into())
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub id: String,
    pub description: String,
    #[serde(default)]
    pub platforms: Option<Vec<String>>,
    #[serde(default)]
    pub optional: bool,
    #[serde(default = "data_root")]
    pub root: String,
    pub source: Source,
    #[serde(default)]
    pub files: Vec<AssetFile>,
    #[serde(default)]
    pub license: Option<AssetLicense>,
}
fn data_root() -> String {
    "data".to_owned()
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum Source {
    Url {
        url: Option<String>,
        archive: Option<String>,
        max_extracted_size_bytes: Option<u64>,
        max_files: Option<u64>,
    },
    Git {
        repo: String,
        r#ref: String,
    },
    Browser {
        url: String,
    },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetFile {
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    pub to: String,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub size_bytes: Option<u64>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetLicense {
    pub spdx: String,
    pub notice: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ShippedList {
    #[serde(rename = "$schema")]
    schema: Option<String>,
    schema_version: u64,
    assets: Vec<Asset>,
}

/// Parses the shared asset definitions; installed lists have a wrapper, community manifests an array.
pub fn parse_asset_list(source: &str, context: AssetContext) -> Result<Vec<Asset>, AssetError> {
    let value = parse_json_strict(source).map_err(|error| fail(error.to_string()))?;
    parse_asset_value(value, context)
}

pub fn parse_asset_value(value: Value, context: AssetContext) -> Result<Vec<Asset>, AssetError> {
    let assets = match context {
        AssetContext::Shipped => {
            let list: ShippedList =
                serde_json::from_value(value).map_err(|error| fail(error.to_string()))?;
            if list.schema_version != 1 {
                return Err(fail("Asset schema_version must be 1"));
            }
            let _ = list.schema;
            list.assets
        }
        AssetContext::Community => {
            serde_json::from_value(value).map_err(|error| fail(error.to_string()))?
        }
    };
    validate_assets(&assets, context)?;
    Ok(assets)
}

fn validate_assets(assets: &[Asset], context: AssetContext) -> Result<(), AssetError> {
    let mut ids = HashSet::new();
    let mut destinations = HashSet::new();
    for asset in assets {
        let name = &asset.id;
        if !valid_identifier(name) {
            return Err(fail(format!("Invalid asset id: {name}")));
        }
        if !ids.insert(name) {
            return Err(fail(format!("Duplicate asset id: {name}")));
        }
        if asset.description.is_empty() || has_control(&asset.description) {
            return Err(fail(format!("{name}: description is required")));
        }
        if !matches!(asset.root.as_str(), "data" | "module")
            || (context == AssetContext::Shipped && asset.root != "data")
        {
            return Err(fail(format!("{name}: unsupported destination root")));
        }
        if let Some(platforms) = &asset.platforms
            && (platforms.is_empty()
                || platforms
                    .iter()
                    .any(|platform| !matches!(platform.as_str(), "linux" | "macos" | "windows"))
                || platforms.iter().collect::<HashSet<_>>().len() != platforms.len())
        {
            return Err(fail(format!("{name}: invalid platforms")));
        }
        let browser = matches!(asset.source, Source::Browser { .. });
        if browser && context == AssetContext::Shipped {
            return Err(fail(format!("{name}: browser source is community only")));
        }
        if !browser && asset.files.is_empty() {
            return Err(fail(format!("{name}: no files")));
        }
        match &asset.source {
            Source::Url {
                url,
                archive,
                max_extracted_size_bytes,
                max_files,
            } => {
                if let Some(url) = url {
                    valid_https(url, false)?;
                }
                if let Some(archive) = archive {
                    if archive != "zip"
                        || url.is_none()
                        || !valid_size(*max_extracted_size_bytes)
                        || !valid_size(*max_files)
                    {
                        return Err(fail(format!("{name}: zip source needs archive limits")));
                    }
                } else if max_extracted_size_bytes.is_some()
                    || max_files.is_some()
                    || (url.is_some() && asset.files.len() != 1)
                {
                    return Err(fail(format!(
                        "{name}: source URL needs one file and no archive limits"
                    )));
                }
            }
            Source::Git { repo, r#ref } => {
                valid_https(repo, false)?;
                if r#ref.len() != 40
                    || !r#ref.bytes().all(|byte| {
                        byte.is_ascii_lowercase() && byte.is_ascii_hexdigit()
                            || byte.is_ascii_digit()
                    })
                {
                    return Err(fail(format!(
                        "{name}: Git ref must be a full lowercase commit SHA"
                    )));
                }
            }
            Source::Browser { url } => valid_https(url, true)?,
        }
        for file in &asset.files {
            valid_path(&file.to)?;
            if destinations.iter().any(|(root, existing): &(&str, &str)| {
                *root == asset.root
                    && (file.to.starts_with(&format!("{existing}/"))
                        || existing.starts_with(&format!("{}/", file.to)))
            }) {
                return Err(fail(format!(
                    "{name}: destination conflicts with another path: {}",
                    file.to
                )));
            }
            if !destinations.insert((asset.root.as_str(), file.to.as_str())) {
                return Err(fail(format!("{name}: duplicate destination {}", file.to)));
            }
            let needs_from = matches!(asset.source, Source::Git { .. })
                || matches!(
                    asset.source,
                    Source::Url {
                        archive: Some(_),
                        ..
                    }
                );
            if needs_from != file.from.is_some() {
                return Err(fail(format!("{name}: incorrect from path for {}", file.to)));
            }
            if let Some(from) = &file.from {
                valid_path(from)?;
            }
            if let Some(url) = &file.url {
                valid_https(url, false)?;
            }
            if browser
                && (file.from.is_some()
                    || file.url.is_some()
                    || file.sha256.is_some()
                    || file.size_bytes.is_some())
            {
                return Err(fail(format!(
                    "{name}: browser files are destination hints only"
                )));
            }
            if matches!(asset.source, Source::Url { .. })
                && (file.sha256.is_none() || !valid_size(file.size_bytes))
            {
                return Err(fail(format!(
                    "{name}: URL file needs sha256 and size_bytes"
                )));
            }
            let needs_url = matches!(
                asset.source,
                Source::Url {
                    url: None,
                    archive: None,
                    ..
                }
            );
            if needs_url != file.url.is_some() {
                return Err(fail(format!("{name}: incorrect file URL for {}", file.to)));
            }
            if let Some(sha) = &file.sha256
                && (sha.len() != 64 || !sha.bytes().all(|byte| byte.is_ascii_hexdigit()))
            {
                return Err(fail(format!("{name}: invalid sha256 for {}", file.to)));
            }
            if file.size_bytes.is_some() && !valid_size(file.size_bytes) {
                return Err(fail(format!("{name}: invalid size for {}", file.to)));
            }
        }
        if browser {
            if asset.license.is_some() {
                return Err(fail(format!(
                    "{name}: browser source cannot declare a license"
                )));
            }
        } else {
            let license = asset
                .license
                .as_ref()
                .ok_or_else(|| fail(format!("{name}: license is required")))?;
            if license.spdx.is_empty() || has_control(&license.spdx) {
                return Err(fail(format!("{name}: invalid SPDX license")));
            }
            valid_path(&license.notice)?;
            if !asset.files.iter().any(|file| file.to == license.notice) {
                return Err(fail(format!(
                    "{name}: license notice is not one of the files"
                )));
            }
        }
    }
    Ok(())
}

fn valid_identifier(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes.next().is_some_and(|first| first.is_ascii_lowercase())
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}
fn valid_size(value: Option<u64>) -> bool {
    value.is_some_and(|n| (1..=9_007_199_254_740_991).contains(&n))
}
fn has_control(value: &str) -> bool {
    value.chars().any(|c| c.is_control())
}
fn valid_https(value: &str, fragment: bool) -> Result<(), AssetError> {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len()
                || !bytes[index + 1].is_ascii_hexdigit()
                || !bytes[index + 2].is_ascii_hexdigit()
            {
                return Err(fail(format!("Invalid HTTPS URL: {value}")));
            }
            index += 3;
        } else {
            index += 1;
        }
    }
    let parsed = url::Url::parse(value).map_err(|_| fail(format!("Invalid HTTPS URL: {value}")))?;
    if !value.starts_with("https://")
        || parsed.scheme() != "https"
        || parsed.host().is_none()
        || parsed.port() == Some(0)
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || (!fragment && parsed.fragment().is_some())
        || value
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || c == '\\')
    {
        return Err(fail(format!("Invalid HTTPS URL: {value}")));
    }
    Ok(())
}
fn valid_path(value: &str) -> Result<(), AssetError> {
    if value.is_empty()
        || value
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        || value
            .chars()
            .any(|c| c.is_control() || "\\:<>\"|?*".contains(c))
        || Path::new(value).is_absolute()
    {
        return Err(fail(format!("Unsafe asset path: {value}")));
    }
    for part in value.split('/') {
        let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        if part.ends_with([' ', '.'])
            || matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || (stem.len() == 4
                && (stem.starts_with("COM") || stem.starts_with("LPT"))
                && matches!(stem.as_bytes()[3], b'1'..=b'9'))
        {
            return Err(fail(format!("Unsafe asset path: {value}")));
        }
    }
    Ok(())
}

/// Locates the manifest shipped by the selected MayaFlux installation.
pub fn installed_asset_list_path() -> Result<PathBuf, AssetError> {
    let roots =
        if let Some(root) = std::env::var_os("MAYAFLUX_ROOT").filter(|root| !root.is_empty()) {
            vec![PathBuf::from(root)]
        } else {
            #[cfg(target_os = "linux")]
            let roots = vec![PathBuf::from("/usr/local"), PathBuf::from("/usr")];
            #[cfg(target_os = "windows")]
            let roots = vec![PathBuf::from(r"C:\MayaFlux")];
            #[cfg(target_os = "macos")]
            let roots = crate::platform::macos::mayaflux_prefixes();
            roots
        };
    let checked = roots
        .iter()
        .map(|root| root.join("share/MayaFlux/assets/assets.json"))
        .collect::<Vec<_>>();
    for path in &checked {
        crate::filesystem::no_links(path).map_err(fail)?;
        if path.is_file() {
            return Ok(path.clone());
        }
    }
    Err(fail(format!(
        "MayaFlux assets.json was not found. Install a MayaFlux release with assets or set MAYAFLUX_ROOT. Checked: {}",
        checked
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    )))
}

/// Fetches the installed MayaFlux asset list for a project.
pub fn fetch_project_assets(
    project: &Path,
    report: &mut impl FnMut(String),
) -> Result<ApplyResult, AssetError> {
    let path = installed_asset_list_path()?;
    let source = fs::read_to_string(&path)
        .map_err(|error| fail(format!("Could not read {}: {error}", path.display())))?;
    let assets = parse_asset_list(&source, AssetContext::Shipped)?;
    apply_assets_with(
        project,
        None,
        &assets,
        true,
        &HostTransport,
        &|_| Ok(false),
        report,
    )
}
