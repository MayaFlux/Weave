use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use weave::operations::assets::{
    AssetContext, AssetError, AssetTransport, apply_assets_with, parse_asset_list,
};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "weave-asset-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct FixedTransport;
impl AssetTransport for FixedTransport {
    fn download(&self, _url: &str, _limit: u64) -> Result<Vec<u8>, AssetError> {
        Ok(b"hello".to_vec())
    }
    fn git_files(
        &self,
        _repo: &str,
        _commit: &str,
        paths: &[String],
    ) -> Result<HashMap<String, Vec<u8>>, AssetError> {
        Ok(paths
            .iter()
            .map(|path| {
                (
                    path.clone(),
                    if path == "LICENSE" {
                        b"MIT".to_vec()
                    } else {
                        b"hello".to_vec()
                    },
                )
            })
            .collect())
    }
}

struct ZipTransport;
impl AssetTransport for ZipTransport {
    fn download(&self, _url: &str, _limit: u64) -> Result<Vec<u8>, AssetError> {
        Ok(include_bytes!("fixtures/assets/small.zip").to_vec())
    }
    fn git_files(
        &self,
        _repo: &str,
        _commit: &str,
        _paths: &[String],
    ) -> Result<HashMap<String, Vec<u8>>, AssetError> {
        unreachable!()
    }
}

fn zip_asset(max_files: u64, max_expanded: u64) -> Vec<weave::operations::assets::Asset> {
    let manifest = format!(
        r#"{{"schema_version":1,"assets":[{{"id":"zip_data","description":"ZIP data","source":{{"kind":"url","url":"https://example.org/data.zip","archive":"zip","max_extracted_size_bytes":{max_expanded},"max_files":{max_files}}},"files":[{{"from":"weights.bin","to":"models/weights.bin","sha256":"5f70bf18a086007016e948b04aed3b82103a36bea41755b6cddfaf10ace3c6ef","size_bytes":1024}},{{"from":"LICENSE","to":"models/LICENSE","sha256":"e5dcffe836b6ec8a58e492419b550e65fb8cbdc308503979e5dacb33ac7ea3b7","size_bytes":3}}],"license":{{"spdx":"MIT","notice":"models/LICENSE"}}}}]}}"#
    );
    parse_asset_list(&manifest, AssetContext::Shipped).unwrap()
}

fn git_asset() -> Vec<weave::operations::assets::Asset> {
    let source = r#"{"schema_version":1,"assets":[{"id":"weights","description":"Weights","source":{"kind":"git","repo":"https://example.org/repo","ref":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},"files":[{"from":"model.bin","to":"tensors/model.bin","sha256":"2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824","size_bytes":5},{"from":"LICENSE","to":"tensors/LICENSE"}],"license":{"spdx":"MIT","notice":"tensors/LICENSE"}}]}"#;
    parse_asset_list(source, AssetContext::Shipped).unwrap()
}

#[test]
fn parses_shipped_git_asset_and_browser_handoff() {
    let shipped = r#"{"schema_version":1,"assets":[{"id":"silero_vad","description":"VAD weights","source":{"kind":"git","repo":"https://github.com/snakers4/silero-vad","ref":"fb7d7c7f5df63235f0afd037d8397593829aeb41"},"files":[{"from":"src/silero.safetensors","to":"tensors/silero.safetensors","sha256":"c59271c284ae9c8335d795d60e0bfdb71aaaceec578d9bd9ffc1b8153c319ea1","size_bytes":1239748},{"from":"LICENSE","to":"tensors/LICENSE"}],"license":{"spdx":"MIT","notice":"tensors/LICENSE"}}]}"#;
    assert_eq!(
        parse_asset_list(shipped, AssetContext::Shipped)
            .unwrap()
            .len(),
        1
    );

    let community = r#"[{"id":"model","description":"Open provider page","source":{"kind":"browser","url":"https://example.org/download#models"},"files":[{"to":"models/model.bin"}]}]"#;
    assert_eq!(
        parse_asset_list(community, AssetContext::Community)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn rejects_traversal_and_missing_license_notice() {
    let template = r#"{"schema_version":1,"assets":[{"id":"weights","description":"Data","source":{"kind":"url","url":"https://example.org/file"},"files":[{"to":"TARGET","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","size_bytes":1}],"license":{"spdx":"MIT","notice":"LICENSE"}}]}"#;
    assert!(
        parse_asset_list(
            &template.replace("TARGET", "../escape"),
            AssetContext::Shipped
        )
        .is_err()
    );
    assert!(
        parse_asset_list(
            &template.replace("TARGET", "weights.bin"),
            AssetContext::Shipped
        )
        .is_err()
    );
}

#[test]
fn rejects_file_and_directory_destination_collision_before_fetch() {
    let manifest = r#"{"schema_version":1,"assets":[{"id":"collision","description":"Collision","source":{"kind":"git","repo":"https://example.org/repo","ref":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},"files":[{"from":"first","to":"models"},{"from":"second","to":"models/second"}],"license":{"spdx":"MIT","notice":"models"}}]}"#;
    assert!(parse_asset_list(manifest, AssetContext::Shipped).is_err());
}

#[test]
fn rejects_malformed_percent_escape_in_browser_url() {
    let asset = r#"[{"id":"download","description":"Open page","source":{"kind":"browser","url":"https://example.org/%xx"}}]"#;
    assert!(parse_asset_list(asset, AssetContext::Community).is_err());
    let zero_port = r#"[{"id":"download","description":"Open page","source":{"kind":"browser","url":"https://example.org:0/file"}}]"#;
    assert!(parse_asset_list(zero_port, AssetContext::Community).is_err());
}

#[test]
fn zip_asset_extracts_only_declared_verified_members() {
    let root = Temp::new();
    fs::write(root.0.join("CMakeLists.txt"), "project(Test)").unwrap();
    let result = apply_assets_with(
        &root.0,
        None,
        &zip_asset(2, 2000),
        false,
        &ZipTransport,
        &|_| Ok(false),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(result.fetched, 2);
    assert_eq!(
        fs::read(root.0.join("data/models/weights.bin")).unwrap(),
        vec![0; 1024]
    );
    assert_eq!(
        fs::read(root.0.join("data/models/LICENSE")).unwrap(),
        b"MIT"
    );
}

#[test]
fn zip_limits_reject_archive_before_placing_any_file() {
    for (members, expanded) in [(1, 2000), (2, 500)] {
        let root = Temp::new();
        fs::write(root.0.join("CMakeLists.txt"), "project(Test)").unwrap();
        assert!(
            apply_assets_with(
                &root.0,
                None,
                &zip_asset(members, expanded),
                false,
                &ZipTransport,
                &|_| Ok(false),
                &mut |_| {}
            )
            .is_err()
        );
        assert!(!root.0.join("data/models/weights.bin").exists());
        assert!(!root.0.join("data/models/LICENSE").exists());
    }
}

#[test]
fn fetches_verified_files_and_preserves_conflicts_until_replace() {
    let root = Temp::new();
    let project = root.0.join("Project with spaces");
    fs::create_dir(&project).unwrap();
    fs::write(project.join("CMakeLists.txt"), "project(Test)").unwrap();
    let assets = git_asset();
    let first = apply_assets_with(
        &project,
        None,
        &assets,
        false,
        &FixedTransport,
        &|_| Ok(false),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(first.fetched, 2);
    assert_eq!(
        fs::read(project.join("data/tensors/model.bin")).unwrap(),
        b"hello"
    );
    fs::write(project.join("data/tensors/model.bin"), "custom").unwrap();
    let second = apply_assets_with(
        &project,
        None,
        &assets,
        false,
        &FixedTransport,
        &|_| Ok(false),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(second.conflicts, 1);
    assert_eq!(
        fs::read(project.join("data/tensors/model.bin")).unwrap(),
        b"custom"
    );
    apply_assets_with(
        &project,
        None,
        &assets,
        true,
        &FixedTransport,
        &|_| Ok(false),
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(
        fs::read(project.join("data/tensors/model.bin")).unwrap(),
        b"hello"
    );
}

#[test]
fn browser_hint_rejects_existing_directory() {
    let root = Temp::new();
    fs::write(root.0.join("CMakeLists.txt"), "project(Test)").unwrap();
    fs::create_dir_all(root.0.join("data/models/model.bin")).unwrap();
    let assets = parse_asset_list(
        r#"[{"id":"model","description":"Open provider page","source":{"kind":"browser","url":"https://example.org/download"},"files":[{"to":"models/model.bin"}]}]"#,
        AssetContext::Community,
    )
    .unwrap();
    assert!(
        apply_assets_with(
            &root.0,
            None,
            &assets,
            false,
            &FixedTransport,
            &|_| Ok(false),
            &mut |_| {},
        )
        .is_err()
    );
}
