//! Shared HTTPS download transport, bounds and progress reporting.
use crate::{
    filesystem,
    process::{self, ProcessRuntime},
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

pub trait Downloader {
    fn download(
        &self,
        url: &str,
        destination: &Path,
        limit: u64,
        log: &(impl Fn(&str) + Sync),
    ) -> Result<(), String>;
}

/// The native adapter supplies the host curl executable and process primitives.
pub struct Curl<R> {
    pub executable: PathBuf,
    pub runtime: R,
}
impl<R: ProcessRuntime> Downloader for Curl<R> {
    fn download(
        &self,
        url: &str,
        destination: &Path,
        limit: u64,
        log: &(impl Fn(&str) + Sync),
    ) -> Result<(), String> {
        if !url.starts_with("https://") {
            return Err("Downloads require HTTPS".into());
        }
        filesystem::no_links(destination)?;
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)
            .map_err(|e| e.to_string())?;
        let mut command = Command::new(&self.executable);
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
        let output = process::run_with(
            &self.runtime,
            &mut command,
            false,
            Duration::from_secs(1900),
            log,
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
}
