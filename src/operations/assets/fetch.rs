//! Verifies and places asset files without executing or extracting untrusted paths.

use super::{Asset, AssetError, AssetFile, Source, fail};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::process::Command;
use zip::ZipArchive;

#[derive(Debug, Default, Clone, Copy)]
pub struct ApplyResult {
    pub fetched: usize,
    pub conflicts: usize,
    pub pending_browser: usize,
    pub optional_failures: usize,
}

/// Network and Git boundary, replaceable by controlled fixtures.
pub trait AssetTransport {
    fn download(&self, url: &str, limit: u64) -> Result<Vec<u8>, AssetError>;
    fn git_files(
        &self,
        repo: &str,
        commit: &str,
        paths: &[String],
    ) -> Result<HashMap<String, Vec<u8>>, AssetError>;
}

pub struct HostTransport;
impl AssetTransport for HostTransport {
    fn download(&self, url: &str, limit: u64) -> Result<Vec<u8>, AssetError> {
        let temporary = crate::filesystem::Temporary::create().map_err(fail)?;
        let target = temporary.0.join("download");
        let output = Command::new("curl")
            .args([
                "--fail",
                "--location",
                "--silent",
                "--show-error",
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
            .arg(&target)
            .arg(url)
            .output()
            .map_err(|error| {
                fail(format!(
                    "Could not start curl. Install curl and retry: {error}"
                ))
            })?;
        if !output.status.success() {
            return Err(fail(format!(
                "Could not download {url}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        let size = fs::metadata(&target)
            .map_err(|error| fail(error.to_string()))?
            .len();
        if size > limit {
            return Err(fail(format!("Download exceeds {limit} bytes: {url}")));
        }
        fs::read(target).map_err(|error| fail(error.to_string()))
    }

    fn git_files(
        &self,
        repo: &str,
        commit: &str,
        paths: &[String],
    ) -> Result<HashMap<String, Vec<u8>>, AssetError> {
        let temporary = crate::filesystem::Temporary::create().map_err(fail)?;
        let directory = &temporary.0;
        run_git(directory, &["init", "-q"])?;
        run_git(directory, &["remote", "add", "origin", repo])?;
        if run_git(
            directory,
            &[
                "fetch",
                "-q",
                "--depth=1",
                "--filter=blob:none",
                "origin",
                commit,
            ],
        )
        .is_err()
        {
            run_git(directory, &["fetch", "-q", "--depth=1", "origin", commit])?;
        }
        let mut result = HashMap::new();
        for path in paths {
            let spec = format!("{commit}:{path}");
            result.insert(
                path.clone(),
                run_git(directory, &["cat-file", "blob", &spec])?,
            );
        }
        Ok(result)
    }
}

fn run_git(directory: &Path, args: &[&str]) -> Result<Vec<u8>, AssetError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(directory)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|error| {
            fail(format!(
                "Could not start Git. Install Git and retry: {error}"
            ))
        })?;
    if !output.status.success() {
        return Err(fail(format!(
            "Git {} failed: {}",
            args.first().copied().unwrap_or("command"),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(output.stdout)
}

/// Applies a validated list to a real project. Every file is checked before transport or placement.
pub fn apply_assets_with(
    project_path: &Path,
    module_path: Option<&Path>,
    assets: &[Asset],
    replace: bool,
    transport: &impl AssetTransport,
    open_browser: &impl Fn(&str) -> Result<bool, AssetError>,
    report: &mut impl FnMut(String),
) -> Result<ApplyResult, AssetError> {
    crate::filesystem::no_links(project_path).map_err(fail)?;
    let project = fs::canonicalize(project_path)
        .map_err(|error| fail(format!("Could not open project: {error}")))?;
    if !project.join("CMakeLists.txt").is_file() {
        return Err(fail(format!(
            "Not a MayaFlux project: {}",
            project.display()
        )));
    }
    let module = if let Some(path) = module_path {
        crate::filesystem::no_links(path).map_err(fail)?;
        let canonical = fs::canonicalize(path)
            .map_err(|error| fail(format!("Could not open module: {error}")))?;
        if !canonical.starts_with(&project) {
            return Err(fail("Module path is outside project"));
        }
        Some(canonical)
    } else {
        None
    };
    let mut outcome = ApplyResult::default();
    for asset in assets {
        if asset.platforms.as_ref().is_some_and(|platforms| {
            !platforms
                .iter()
                .any(|platform| platform == std::env::consts::OS)
        }) {
            report(format!("{}: skipped on this platform", asset.id));
            continue;
        }
        let root = if asset.root == "module" {
            module
                .as_ref()
                .ok_or_else(|| fail(format!("{} needs a community module root", asset.id)))?
        } else {
            &project.join("data")
        };
        if let Source::Browser { url } = &asset.source {
            let mut missing = false;
            let mut hints = Vec::new();
            for file in &asset.files {
                let target = root.join(&file.to);
                crate::filesystem::no_links(&target).map_err(fail)?;
                match fs::symlink_metadata(&target) {
                    Ok(metadata) if metadata.is_file() => report(format!(
                        "{}: file exists at {}. Review it against {url}.",
                        asset.id,
                        target.display()
                    )),
                    Ok(_) => {
                        return Err(fail(format!(
                            "Asset destination is not a regular file: {}",
                            target.display()
                        )));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        missing = true;
                        hints.push(target.display().to_string());
                    }
                    Err(error) => {
                        return Err(fail(format!(
                            "Could not inspect {}: {error}",
                            target.display()
                        )));
                    }
                }
            }
            if asset.files.is_empty() || missing {
                let destination = if hints.is_empty() {
                    String::new()
                } else {
                    format!(" Place downloaded files at {}.", hints.join(", "))
                };
                match open_browser(url) {
                    Ok(true) => report(format!(
                        "{}: handed {url} to the browser; user action remains pending.{destination}",
                        asset.id
                    )),
                    Ok(false) => report(format!(
                        "{}: open {url} manually; user action remains pending.{destination}",
                        asset.id
                    )),
                    Err(error) => report(format!(
                        "{}: open {url} manually; user action remains pending: {error}.{destination}",
                        asset.id
                    )),
                }
                outcome.pending_browser += 1;
            }
            continue;
        }
        let mut wanted = Vec::new();
        for file in &asset.files {
            let target = root.join(&file.to);
            crate::filesystem::no_links(&target).map_err(fail)?;
            match file_status(file, &target)? {
                FileStatus::Present => report(format!("present  {}", target.display())),
                FileStatus::Differs if !replace => {
                    report(format!(
                        "differs  {} (kept; use explicit replacement to overwrite)",
                        target.display()
                    ));
                    outcome.conflicts += 1;
                }
                _ => wanted.push((file, target)),
            }
        }
        if wanted.is_empty() {
            continue;
        }
        let fetched = fetch_files(asset, &wanted, transport);
        let fetched = match fetched {
            Ok(files) => files,
            Err(error) if asset.optional => {
                report(format!("{}: optional asset failed: {error}", asset.id));
                outcome.optional_failures += 1;
                continue;
            }
            Err(error) => return Err(fail(format!("{}: {error}", asset.id))),
        };
        for (target, bytes) in fetched {
            crate::filesystem::no_links(&target).map_err(fail)?;
            fs::create_dir_all(
                target
                    .parent()
                    .ok_or_else(|| fail("Asset destination has no parent"))?,
            )
            .map_err(|error| fail(format!("Could not create asset directory: {error}")))?;
            crate::filesystem::atomic_write(&target, &bytes)
                .map_err(|error| fail(format!("Could not write {}: {error}", target.display())))?;
            report(format!("fetched  {}", target.display()));
            outcome.fetched += 1;
        }
    }
    Ok(outcome)
}

enum FileStatus {
    Present,
    Missing,
    Differs,
}
fn file_status(file: &AssetFile, target: &Path) -> Result<FileStatus, AssetError> {
    let metadata = match fs::symlink_metadata(target) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(FileStatus::Missing);
        }
        Err(error) => {
            return Err(fail(format!(
                "Could not inspect {}: {error}",
                target.display()
            )));
        }
    };
    if !metadata.is_file() {
        return Err(fail(format!(
            "Asset destination is not a regular file: {}",
            target.display()
        )));
    }
    let Some(expected) = &file.sha256 else {
        return Ok(FileStatus::Present);
    };
    let contents = fs::read(target).map_err(|error| fail(error.to_string()))?;
    Ok(if hash(&contents).eq_ignore_ascii_case(expected) {
        FileStatus::Present
    } else {
        FileStatus::Differs
    })
}
fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn verify(file: &AssetFile, bytes: &[u8]) -> Result<(), AssetError> {
    if file
        .size_bytes
        .is_some_and(|size| bytes.len() as u64 != size)
    {
        return Err(fail(format!("{} has wrong size", file.to)));
    }
    if file
        .sha256
        .as_ref()
        .is_some_and(|sha| !hash(bytes).eq_ignore_ascii_case(sha))
    {
        return Err(fail(format!("{} does not match SHA-256", file.to)));
    }
    Ok(())
}
fn fetch_files(
    asset: &Asset,
    wanted: &[(&AssetFile, PathBuf)],
    transport: &impl AssetTransport,
) -> Result<Vec<(PathBuf, Vec<u8>)>, AssetError> {
    let mut result = Vec::new();
    match &asset.source {
        Source::Url {
            url, archive: None, ..
        } => {
            let (file, target) = &wanted[0];
            let bytes = transport.download(url, file.size_bytes.expect("validated size"))?;
            verify(file, &bytes)?;
            result.push((target.clone(), bytes));
        }
        Source::Url {
            url,
            archive: Some(_),
            max_extracted_size_bytes,
            max_files,
        } => {
            let limit = max_extracted_size_bytes.expect("validated zip limit");
            let bytes = transport.download(url, limit)?;
            if bytes.len() as u64 > limit {
                return Err(fail("ZIP download exceeds declared limit"));
            }
            let mut archive = ZipArchive::new(Cursor::new(bytes))
                .map_err(|error| fail(format!("Invalid ZIP archive: {error}")))?;
            if archive.len() as u64 > max_files.expect("validated ZIP member limit")
                || archive.has_duplicate_names()
            {
                return Err(fail(
                    "ZIP archive exceeds member limit or has duplicate names",
                ));
            }
            let mut total = 0_u64;
            for index in 0..archive.len() {
                let file = archive
                    .by_index(index)
                    .map_err(|error| fail(error.to_string()))?;
                if !file.is_dir() {
                    total = total
                        .checked_add(file.size())
                        .ok_or_else(|| fail("ZIP expanded size overflow"))?;
                }
                if total > limit {
                    return Err(fail("ZIP archive exceeds expanded size limit"));
                }
            }
            for (file, target) in wanted {
                let mut member = archive
                    .by_name(file.from.as_deref().expect("validated ZIP path"))
                    .map_err(|error| fail(format!("ZIP member missing: {error}")))?;
                if !member.is_file() {
                    return Err(fail(format!(
                        "ZIP member is not a regular file: {}",
                        file.to
                    )));
                }
                let mut bytes = Vec::new();
                member
                    .by_ref()
                    .take(limit + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|error| fail(error.to_string()))?;
                if bytes.len() as u64 > limit {
                    return Err(fail("ZIP member exceeds expanded size limit"));
                }
                verify(file, &bytes)?;
                result.push((target.clone(), bytes));
            }
        }
        Source::Git { repo, r#ref } => {
            let paths = wanted
                .iter()
                .map(|(file, _)| file.from.clone().expect("validated Git path"))
                .collect::<Vec<_>>();
            let files = transport.git_files(repo, r#ref, &paths)?;
            for (file, target) in wanted {
                let from = file.from.as_ref().expect("validated Git path");
                let bytes = files
                    .get(from)
                    .ok_or_else(|| fail(format!("Git file missing: {from}")))?;
                verify(file, bytes)?;
                result.push((target.clone(), bytes.clone()));
            }
        }
        Source::Browser { .. } => unreachable!(),
    }
    Ok(result)
}
