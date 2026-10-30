//! Shared staged payload acquisition and replacement. Native adapters extract archives.
use super::release::Release;
use crate::{download::Downloader, filesystem as files};
use std::{fs, path::Path};

#[derive(Clone, Copy)]
pub struct ArchiveFormat {
    pub extension: &'static str,
    pub magic: &'static [u8],
    pub minimum_size: u64,
}
impl ArchiveFormat {
    pub const SEVEN_ZIP: Self = Self {
        extension: "7z",
        magic: &[0x37, 0x7a, 0xbc, 0xaf, 0x27, 0x1c],
        minimum_size: 32,
    };
}

pub trait Extractor {
    fn extract(
        &self,
        archive: &Path,
        destination: &Path,
        log: &(impl Fn(&str) + Sync),
    ) -> Result<(), String>;
}
pub struct PayloadLayout<'a> {
    pub archive_format: ArchiveFormat,
    pub required: &'a [&'a str],
    pub wrapper: &'a str,
    pub version_marker: &'a str,
}

pub fn acquire(
    root: &Path,
    release: &Release,
    layout: &PayloadLayout<'_>,
    downloader: &impl Downloader,
    extractor: &impl Extractor,
    log: &(impl Fn(&str) + Sync),
) -> Result<(), String> {
    files::validate_relative(Path::new(layout.wrapper))?;
    files::validate_relative(Path::new(layout.version_marker))?;
    for required in layout.required {
        files::validate_relative(Path::new(required))?;
    }
    let extension = layout.archive_format.extension;
    if extension.is_empty()
        || extension.starts_with('.')
        || !extension
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.')
    {
        return Err("Invalid archive filename extension".into());
    }
    files::safe_directory(root)?;
    let temporary = files::Temporary::create()?;
    if temporary.0.starts_with(root) || root.starts_with(&temporary.0) {
        return Err("Installation and temporary directories overlap".into());
    }
    log(&format!("Downloading {}...", release.tag));
    let archive = temporary.0.join(format!("mayaflux.{extension}"));
    downloader.download(&release.url, &archive, release.size, log)?;
    validate_archive_for(&archive, release.size, layout.archive_format)?;
    log("Download complete; archive size and signature verified. Extracting MayaFlux...");
    let extracted = temporary.0.join("extracted");
    fs::create_dir(&extracted).map_err(|e| e.to_string())?;
    extractor.extract(&archive, &extracted, log)?;
    let payload = if extracted.join(layout.wrapper).is_dir() {
        extracted.join(layout.wrapper)
    } else {
        extracted
    };
    files::verify_payload(&payload, layout.required)?;
    super::release::record_install(&payload, release)?;
    log("Staged MayaFlux runtime, headers and CMake package verified. Installing package files...");
    let mut all = Vec::new();
    files::collect_files(&payload, &payload, &mut all)?;
    files::safe_directory(root)?;
    if root.exists() {
        fs::remove_dir_all(root).map_err(|e| {
            format!(
                "Cannot replace old MayaFlux at {}: {e}. Close programs using MayaFlux and retry",
                root.display()
            )
        })?;
    }
    all.sort_by_key(|relative| relative == Path::new(layout.version_marker));
    for relative in all {
        let target = root.join(&relative);
        files::no_links(&target)?;
        fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
        fs::copy(payload.join(&relative),&target).map_err(|e|format!("MayaFlux replacement is incomplete at {}: {e}. Retry installation to restore the complete payload",target.display()))?;
    }
    files::verify_payload(root, layout.required)?;
    log("Installed MayaFlux runtime, headers and CMake package verified.");
    Ok(())
}

pub fn validate_archive_for(path: &Path, size: u64, format: ArchiveFormat) -> Result<(), String> {
    use std::io::Read;
    files::no_links(path)?;
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    if format.magic.is_empty() {
        return Err("Archive signature contract is empty".into());
    }
    let mut magic = vec![0; format.magic.len()];
    file.read_exact(&mut magic).map_err(|e| e.to_string())?;
    if size < format.minimum_size
        || file.metadata().map_err(|e| e.to_string())?.len() != size
        || magic != format.magic
    {
        return Err("Downloaded MayaFlux archive is invalid or incomplete".into());
    }
    Ok(())
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
    #[test]
    fn archive_validation_uses_the_supplied_format_contract() {
        let temporary = files::Temporary::create().unwrap();
        let target = temporary.0.join("fixture.zip");
        let mut bytes = vec![0; 32];
        bytes[..4].copy_from_slice(b"PK\x03\x04");
        fs::write(&target, bytes).unwrap();
        let zip = ArchiveFormat {
            extension: "zip",
            magic: b"PK\x03\x04",
            minimum_size: 22,
        };
        assert!(validate_archive_for(&target, 32, zip).is_ok());
        assert!(validate_archive_for(&target, 32, ArchiveFormat::SEVEN_ZIP).is_err());
    }
    struct Download;
    impl Downloader for Download {
        fn download(
            &self,
            _: &str,
            target: &Path,
            _: u64,
            _: &(impl Fn(&str) + Sync),
        ) -> Result<(), String> {
            let mut bytes = vec![0; 32];
            bytes[..6].copy_from_slice(&[0x37, 0x7a, 0xbc, 0xaf, 0x27, 0x1c]);
            fs::write(target, bytes).map_err(|e| e.to_string())
        }
    }
    struct Archive {
        complete: bool,
    }
    impl Extractor for Archive {
        fn extract(
            &self,
            _: &Path,
            target: &Path,
            _: &(impl Fn(&str) + Sync),
        ) -> Result<(), String> {
            fs::create_dir_all(target.join("dist_staging/bin")).unwrap();
            if self.complete {
                fs::write(target.join("dist_staging/bin/runtime.dll"), "new runtime").unwrap();
                let marker = target.join("dist_staging/share/MayaFlux/.version");
                fs::create_dir_all(marker.parent().unwrap()).unwrap();
                fs::write(marker, r#"{"version":"0.5.0"}"#).unwrap();
            }
            Ok(())
        }
    }
    #[test]
    fn invalid_staged_payload_preserves_existing_installation_and_retry_replaces_it() {
        let directory = files::Temporary::create().unwrap();
        let root = directory.0.join("installed");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("old.txt"), "old payload").unwrap();
        let release = Release {
            tag: "v0.5.0".into(),
            url: "https://example.invalid/package.7z".into(),
            size: 32,
            version: [0, 5, 0],
        };
        let layout = PayloadLayout {
            archive_format: ArchiveFormat::SEVEN_ZIP,
            required: &["bin/runtime.dll"],
            wrapper: "dist_staging",
            version_marker: "share/MayaFlux/.version",
        };
        assert!(
            acquire(
                &root,
                &release,
                &layout,
                &Download,
                &Archive { complete: false },
                &|_| {}
            )
            .is_err()
        );
        assert_eq!(
            fs::read_to_string(root.join("old.txt")).unwrap(),
            "old payload"
        );
        acquire(
            &root,
            &release,
            &layout,
            &Download,
            &Archive { complete: true },
            &|_| {},
        )
        .unwrap();
        assert!(!root.join("old.txt").exists());
        assert_eq!(
            super::super::release::installed(&root).unwrap().unwrap()["weave_release"],
            "v0.5.0"
        );
        assert_eq!(
            fs::read_to_string(root.join("bin/runtime.dll")).unwrap(),
            "new runtime"
        );
    }
}
