use serde_json::json;
use weave::operations::weave_release::{CheckState, release_notice};

#[test]
fn newer_stable_release_is_offered_with_a_repo_release_link() {
    let release = json!({"tag_name":"v0.10.0","draft":false,"prerelease":false,
        "html_url":"https://github.com/MayaFlux/Weave/releases/tag/v0.10.0"});
    let notice = release_notice("0.9.9", &release).unwrap();
    assert_eq!(notice.version, "0.10.0");
    assert_eq!(
        notice.url,
        "https://github.com/MayaFlux/Weave/releases/tag/v0.10.0"
    );
}

#[test]
fn prereleases_drafts_and_equal_or_older_versions_are_not_offered() {
    for (tag, draft, prerelease) in [
        ("v0.7.0-dev", false, true),
        ("v0.7.0", true, false),
        ("v0.6.0", false, false),
        ("v0.5.9", false, false),
        ("garbage", false, false),
    ] {
        let release = json!({"tag_name":tag,"draft":draft,"prerelease":prerelease});
        assert!(release_notice("0.6.0", &release).is_none(), "{tag}");
    }
}

#[test]
fn cached_notice_survives_check_cooldown_until_dismissed() {
    let mut state = CheckState::default();
    state.latest_version = "0.7.0".into();
    state.last_check = 1_000_000;
    assert!(!state.should_check(1_000_001));
    assert!(state.notice("0.6.0").is_some());
    state.dismissed_version = "0.7.0".into();
    assert!(state.notice("0.6.0").is_none());
    assert!(state.should_check(1_086_400));
}
