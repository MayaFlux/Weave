use super::{
    FeatureStatus, FileChange, ShaderConflictPolicy, apply_project_options_at, commit_changes_with,
    inspect_project_options_at,
};
use crate::operations::project::{lila_debugger_path, lila_dll_copy_block, lila_link_block};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

struct Temporary(PathBuf);

impl Temporary {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("weave-manage-options-{}-{id}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }
}

impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn project(root: &Path) -> PathBuf {
    let project = root.join("Project with spaces");
    fs::create_dir_all(project.join("cmake")).unwrap();
    fs::create_dir_all(project.join("data/shaders")).unwrap();
    fs::write(project.join("CMakeLists.txt"), "project(Sample)\n").unwrap();
    let cmake = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/resources/templates/project/cmake/mayaflux.cmake"
    ))
    .replace("@LILA_LINK_BLOCK@", "")
    .replace("@LILA_DEBUGGER_PATH@", "")
    .replace("@LILA_DLL_COPY@", "");
    fs::write(project.join("cmake/mayaflux.cmake"), cmake).unwrap();
    project
}

fn shader_root(root: &Path) -> PathBuf {
    let shaders = root.join("installed shaders");
    fs::create_dir_all(shaders.join("nested")).unwrap();
    fs::write(shaders.join("nested/effect.frag"), "shader source").unwrap();
    shaders
}

#[test]
fn lila_edits_preserve_line_endings_and_custom_content() {
    let template = include_str!("../../../resources/templates/project/cmake/mayaflux.cmake")
        .replace("\r\n", "\n");
    for newline in ["\n", "\r\n"] {
        let base = template.replace('\n', newline);
        let custom = "\n# Custom setting\r\nset(USER_OPTION ON)\n";
        let original = base
            .replace("@LILA_LINK_BLOCK@", "")
            .replace("@LILA_DEBUGGER_PATH@", "")
            .replace("@LILA_DLL_COPY@", "")
            + custom;
        let expected = base
            .replace("@LILA_LINK_BLOCK@", lila_link_block())
            .replace("@LILA_DEBUGGER_PATH@", lila_debugger_path())
            .replace("@LILA_DLL_COPY@", lila_dll_copy_block())
            + custom;
        let enabled = super::enable_lila(&original).unwrap();
        assert_eq!(enabled, expected);
        assert_eq!(super::enable_lila(&enabled).unwrap(), enabled);
        let all_crlf = enabled.replace("\r\n", "\n").replace('\n', "\r\n");
        assert_eq!(super::enable_lila(&all_crlf).unwrap(), all_crlf);
    }
}

#[test]
fn inspect_reports_options_available_for_a_standard_project() {
    let temporary = Temporary::new();
    let project = project(&temporary.0);
    let shaders = shader_root(&temporary.0);

    let status = inspect_project_options_at(&project, Some(&shaders)).unwrap();

    assert_eq!(status.lila, FeatureStatus::Available);
    assert_eq!(status.shader_sources, FeatureStatus::Available);
    assert!(status.can_add_lila);
    assert!(status.can_copy_shader_sources);
}

#[test]
fn applies_lila_and_shader_sources_idempotently_preserving_custom_cmake() {
    let temporary = Temporary::new();
    let project = project(&temporary.0);
    let shaders = shader_root(&temporary.0);
    let cmake_path = project.join("cmake/mayaflux.cmake");
    let mut cmake = fs::read_to_string(&cmake_path).unwrap();
    cmake.push_str("\n# User custom CMake setting\nset(USER_SETTING ON)\n");
    fs::write(&cmake_path, cmake).unwrap();

    apply_project_options_at(
        &project,
        true,
        true,
        ShaderConflictPolicy::Reject,
        Some(&shaders),
    )
    .unwrap();
    let after_first = fs::read_to_string(&cmake_path).unwrap();
    apply_project_options_at(
        &project,
        true,
        true,
        ShaderConflictPolicy::Reject,
        Some(&shaders),
    )
    .unwrap();
    let after_second = fs::read_to_string(&cmake_path).unwrap();
    let status = inspect_project_options_at(&project, Some(&shaders)).unwrap();
    let mut expected = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/resources/templates/project/cmake/mayaflux.cmake"
    ))
    .replace("@LILA_LINK_BLOCK@", lila_link_block())
    .replace("@LILA_DEBUGGER_PATH@", lila_debugger_path())
    .replace("@LILA_DLL_COPY@", lila_dll_copy_block());
    expected.push_str("\n# User custom CMake setting\nset(USER_SETTING ON)\n");

    assert_eq!(after_first, after_second);
    assert_eq!(after_second, expected);
    assert_eq!(status.lila, FeatureStatus::Enabled);
    assert_eq!(status.shader_sources, FeatureStatus::Enabled);
    assert!(after_second.contains("set(USER_SETTING ON)"));
    assert_eq!(
        fs::read_to_string(project.join("data/shaders/nested/effect.frag")).unwrap(),
        "shader source"
    );
}

#[test]
fn shader_conflicts_are_reported_before_any_project_mutation() {
    let temporary = Temporary::new();
    let project = project(&temporary.0);
    let shaders = shader_root(&temporary.0);
    let destination = project.join("data/shaders/nested/effect.frag");
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    fs::write(&destination, "user shader").unwrap();
    let cmake_path = project.join("cmake/mayaflux.cmake");
    let before_cmake = fs::read(&cmake_path).unwrap();

    let error = apply_project_options_at(
        &project,
        true,
        true,
        ShaderConflictPolicy::Reject,
        Some(&shaders),
    )
    .unwrap_err();

    assert!(
        matches!(&error, super::ProjectOptionError::ShaderConflicts(paths) if paths == &[PathBuf::from("nested").join("effect.frag")])
    );
    assert_eq!(fs::read(&destination).unwrap(), b"user shader");
    assert_eq!(fs::read(&cmake_path).unwrap(), before_cmake);
}

#[test]
fn explicit_shader_replacement_replaces_only_the_conflicting_file() {
    let temporary = Temporary::new();
    let project = project(&temporary.0);
    let shaders = shader_root(&temporary.0);
    let destination = project.join("data/shaders/nested/effect.frag");
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    fs::write(&destination, "user shader").unwrap();
    fs::write(project.join("data/shaders/custom.frag"), "keep this").unwrap();

    apply_project_options_at(
        &project,
        false,
        true,
        ShaderConflictPolicy::Replace,
        Some(&shaders),
    )
    .unwrap();

    assert_eq!(fs::read(&destination).unwrap(), b"shader source");
    assert_eq!(
        fs::read(project.join("data/shaders/custom.frag")).unwrap(),
        b"keep this"
    );
}

#[test]
fn skipping_shader_conflicts_copies_other_files_and_preserves_existing_content() {
    let temporary = Temporary::new();
    let project = project(&temporary.0);
    let shaders = shader_root(&temporary.0);
    fs::write(shaders.join("nested/other.vert"), "other installed shader").unwrap();
    let conflict = project.join("data/shaders/nested/effect.frag");
    fs::create_dir_all(conflict.parent().unwrap()).unwrap();
    fs::write(&conflict, "user shader").unwrap();

    let messages = apply_project_options_at(
        &project,
        false,
        true,
        ShaderConflictPolicy::Skip,
        Some(&shaders),
    )
    .unwrap();

    assert_eq!(fs::read(conflict).unwrap(), b"user shader");
    assert_eq!(
        fs::read(project.join("data/shaders/nested/other.vert")).unwrap(),
        b"other installed shader"
    );
    assert!(
        messages
            .iter()
            .any(|line| line.contains("Skipped 1 shader file"))
    );
}

#[test]
fn custom_lila_sections_are_reported_as_unknown_and_left_untouched() {
    let temporary = Temporary::new();
    let project = project(&temporary.0);
    let shaders = shader_root(&temporary.0);
    let cmake_path = project.join("cmake/mayaflux.cmake");
    let cmake = fs::read_to_string(&cmake_path).unwrap().replace(
        "# Optional: Lila (Live Coding JIT)",
        "# User customized Lila integration",
    );
    fs::write(&cmake_path, &cmake).unwrap();

    let status = inspect_project_options_at(&project, Some(&shaders)).unwrap();
    let error = apply_project_options_at(
        &project,
        true,
        false,
        ShaderConflictPolicy::Reject,
        Some(&shaders),
    )
    .unwrap_err();

    assert!(matches!(status.lila, FeatureStatus::Unknown(_)));
    assert!(error.to_string().contains("could not be identified safely"));
    assert_eq!(fs::read_to_string(cmake_path).unwrap(), cmake);
}

#[cfg(unix)]
#[test]
fn linked_project_paths_are_rejected_before_option_mutation() {
    use std::os::unix::fs::symlink;

    let temporary = Temporary::new();
    let project = project(&temporary.0);
    let shaders = shader_root(&temporary.0);
    let linked = temporary.0.join("linked project");
    symlink(&project, &linked).unwrap();
    let cmake = fs::read(project.join("cmake/mayaflux.cmake")).unwrap();

    let error = apply_project_options_at(
        &linked,
        true,
        true,
        ShaderConflictPolicy::Reject,
        Some(&shaders),
    )
    .unwrap_err();

    assert!(error.to_string().contains("link"));
    assert_eq!(
        fs::read(project.join("cmake/mayaflux.cmake")).unwrap(),
        cmake
    );
    assert!(!project.join("data/shaders/nested/effect.frag").exists());
}

#[cfg(unix)]
#[test]
fn linked_shader_targets_are_rejected_without_writing_through_them() {
    use std::os::unix::fs::symlink;

    let temporary = Temporary::new();
    let project = project(&temporary.0);
    let shaders = shader_root(&temporary.0);
    let external = temporary.0.join("user shader.frag");
    fs::write(&external, "user shader").unwrap();
    let target = project.join("data/shaders/nested/effect.frag");
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    symlink(&external, target).unwrap();

    let error = apply_project_options_at(
        &project,
        false,
        true,
        ShaderConflictPolicy::Reject,
        Some(&shaders),
    )
    .unwrap_err();

    assert!(error.to_string().contains("link"));
    assert_eq!(fs::read(external).unwrap(), b"user shader");
}

#[test]
fn partial_multi_file_failure_rolls_back_and_allows_retry() {
    let temporary = Temporary::new();
    let project = temporary.0.join("project");
    fs::create_dir_all(&project).unwrap();
    fs::write(project.join("existing.txt"), "original").unwrap();
    let changes = [
        FileChange {
            relative: PathBuf::from("existing.txt"),
            contents: b"replacement".to_vec(),
        },
        FileChange {
            relative: PathBuf::from("nested/new.txt"),
            contents: b"new".to_vec(),
        },
    ];
    let mut writes = 0;

    let error = commit_changes_with(&project, &changes, |path, contents| {
        writes += 1;
        if writes == 2 {
            return Err(std::io::Error::other("controlled failure"));
        }
        fs::write(path, contents)
    })
    .unwrap_err();

    assert!(error.to_string().contains("rolled back"));
    assert_eq!(fs::read(project.join("existing.txt")).unwrap(), b"original");
    assert!(!project.join("nested/new.txt").exists());
    assert!(!project.join("nested").exists());
    commit_changes_with(&project, &changes, |path, contents| {
        fs::write(path, contents)
    })
    .unwrap();
    assert_eq!(
        fs::read(project.join("existing.txt")).unwrap(),
        b"replacement"
    );
    assert_eq!(fs::read(project.join("nested/new.txt")).unwrap(), b"new");
}
