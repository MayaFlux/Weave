//! Shared HTTPS download transport, bounds and progress reporting.
use crate::operations::install::progress::Event;
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

fn transfer_progress(line: &str) -> Option<f32> {
    let token = line.split_whitespace().next_back()?;
    let prefix = line.strip_suffix(token)?;
    if !prefix.chars().all(|c| c == '#' || c.is_whitespace()) {
        return None;
    }
    let value = token.strip_suffix('%')?.parse::<f32>().ok()?;
    (value.is_finite() && (0.0..=100.0).contains(&value)).then_some(value / 100.0)
}

pub trait Downloader {
    fn download(
        &self,
        url: &str,
        destination: &Path,
        limit: u64,
        log: &(impl Fn(&str) + Sync),
    ) -> Result<(), String>;
}

pub(crate) fn report_progress(line: &str, log: &(impl Fn(&str) + Sync)) {
    if let Some(value) = transfer_progress(line) {
        log(&Event::Transfer(Some(value))
            .encode()
            .expect("transfer is encodable"));
    } else if !is_curl_animation(line) {
        log(line);
    }
}

fn is_curl_animation(line: &str) -> bool {
    line.contains('#')
        && line.chars().all(|character| {
            matches!(character, '#' | '=' | '-' | 'O') || character.is_whitespace()
        })
}

fn curl_error(stderr: &str) -> String {
    stderr
        .split(['\r', '\n'])
        .map(str::trim)
        .filter(|line| {
            !line.is_empty() && !is_curl_animation(line) && transfer_progress(line).is_none()
        })
        .collect::<Vec<_>>()
        .join("\n")
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
                "--progress-bar",
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
            &|line| report_progress(line, log),
        );
        log(&Event::Transfer(None)
            .encode()
            .expect("transfer is encodable"));
        let output = output?;
        if !output.status.success() {
            return Err(format!(
                "Download failed: {}. Check the connection and retry",
                curl_error(&String::from_utf8_lossy(&output.stderr))
            ));
        }
        if fs::metadata(destination).map_err(|e| e.to_string())?.len() > limit {
            return Err("Download exceeds its declared maximum size".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_size_curl_animation_is_not_logged_or_repeated_in_errors() {
        let frames = [
            "#=#=#    ",
            "##O#-#    ",
            "##=O#- #    ",
            "# #=O#- #    ",
            "    # -#O=-#   #    ",
        ];
        let captured = std::sync::Mutex::new(Vec::new());
        for line in frames {
            report_progress(line, &|line| captured.lock().unwrap().push(line.to_owned()));
        }
        report_progress("######## 42.5%", &|line| {
            captured.lock().unwrap().push(line.to_owned())
        });
        report_progress("curl: (22) HTTP error 403", &|line| {
            captured.lock().unwrap().push(line.to_owned())
        });
        let captured = captured.into_inner().unwrap();
        assert_eq!(
            captured.len(),
            2,
            "Animation frames leaked into the log: {captured:?}"
        );
        assert_eq!(Event::decode(&captured[0]), Event::Transfer(Some(0.425)));
        assert_eq!(captured[1], "curl: (22) HTTP error 403");
        assert_eq!(
            curl_error("#=#=#\r##O#-#\r######## 42.5%\rcurl: (22) HTTP error 403\n"),
            "curl: (22) HTTP error 403"
        );
    }
    #[test]
    fn only_measured_curl_bars_produce_a_percentage() {
        assert_eq!(
            transfer_progress("###########           42.5%"),
            Some(0.425)
        );
        assert_eq!(transfer_progress("                       0.0%"), Some(0.0));
        for line in [
            "curl: disk is 100% full",
            "### 120%",
            "### NaN%",
            "-#O#-",
            "failed",
        ] {
            assert_eq!(transfer_progress(line), None, "{line}");
        }
    }
}
