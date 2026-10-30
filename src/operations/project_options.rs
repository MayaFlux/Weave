use super::project::{
    SHADER_EXTENSIONS, lila_debugger_path, lila_dll_copy_block, lila_link_block,
    read_shader_sources_at,
};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FeatureStatus {
    Enabled,
    Available,
    Unknown(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectOptionInspection {
    pub lila: FeatureStatus,
    pub shader_sources: FeatureStatus,
    pub can_add_lila: bool,
    pub can_copy_shader_sources: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShaderConflictPolicy {
    Reject,
    Skip,
    Replace,
}

#[derive(Debug)]
pub enum ProjectOptionError {
    Failure(String),
    ShaderConflicts(Vec<PathBuf>),
}

impl Display for ProjectOptionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failure(message) => formatter.write_str(message),
            Self::ShaderConflicts(paths) => write!(
                formatter,
                "Shader files already exist and need an explicit replacement choice: {}",
                paths
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

impl Error for ProjectOptionError {}

impl From<io::Error> for ProjectOptionError {
    fn from(error: io::Error) -> Self {
        Self::Failure(error.to_string())
    }
}

pub fn inspect_project_options(
    project_path: &Path,
) -> Result<ProjectOptionInspection, ProjectOptionError> {
    let shader_root = crate::platform::shader_source_root().ok();
    inspect_project_options_at(project_path, shader_root.as_deref())
}

pub fn inspect_project_options_at(
    project_path: &Path,
    shader_root: Option<&Path>,
) -> Result<ProjectOptionInspection, ProjectOptionError> {
    let project = resolve_project(project_path)?;
    let lila = inspect_lila(&project.join("cmake/mayaflux.cmake"))?;
    let shader_sources = inspect_shaders(&project, shader_root);
    let can_add_lila = lila == FeatureStatus::Available;
    let can_copy_shader_sources = shader_root.is_some_and(|root| {
        read_shader_sources_at(root).is_ok() && shader_sources != FeatureStatus::Enabled
    });
    Ok(ProjectOptionInspection {
        lila,
        shader_sources,
        can_add_lila,
        can_copy_shader_sources,
    })
}

pub fn apply_project_options(
    project_path: &Path,
    with_lila: bool,
    copy_shader_sources: bool,
    conflict_policy: ShaderConflictPolicy,
) -> Result<Vec<String>, ProjectOptionError> {
    let shader_root = if copy_shader_sources {
        Some(crate::platform::shader_source_root().map_err(ProjectOptionError::Failure)?)
    } else {
        None
    };
    apply_project_options_at(
        project_path,
        with_lila,
        copy_shader_sources,
        conflict_policy,
        shader_root.as_deref(),
    )
}

pub fn apply_project_options_at(
    project_path: &Path,
    with_lila: bool,
    copy_shader_sources: bool,
    conflict_policy: ShaderConflictPolicy,
    shader_root: Option<&Path>,
) -> Result<Vec<String>, ProjectOptionError> {
    if !with_lila && !copy_shader_sources {
        return Err(ProjectOptionError::Failure(
            "Select Lila or shader sources to add.".to_owned(),
        ));
    }
    let project = resolve_project(project_path)?;
    let mut changes = Vec::new();
    let mut messages = Vec::new();

    if with_lila {
        let cmake_path = project.join("cmake/mayaflux.cmake");
        let cmake = fs::read_to_string(&cmake_path).map_err(|error| {
            ProjectOptionError::Failure(format!("Could not read {}: {error}", cmake_path.display()))
        })?;
        let updated = enable_lila(&cmake).map_err(ProjectOptionError::Failure)?;
        if updated != cmake {
            changes.push(FileChange {
                relative: PathBuf::from("cmake/mayaflux.cmake"),
                contents: updated.into_bytes(),
            });
            messages.push("Enabled Lila integration in cmake/mayaflux.cmake.".to_owned());
        } else {
            messages.push("Lila integration is already enabled.".to_owned());
        }
    }

    if copy_shader_sources {
        let shader_root = shader_root.ok_or_else(|| {
            ProjectOptionError::Failure(
                "MayaFlux shader sources were not found. Install or repair MayaFlux, then retry."
                    .to_owned(),
            )
        })?;
        let sources = read_shader_sources_at(shader_root)
            .map_err(|error| ProjectOptionError::Failure(error.to_string()))?;
        let shader_directory = project.join("data/shaders");
        crate::filesystem::no_links(&shader_directory).map_err(ProjectOptionError::Failure)?;
        let mut conflicts = Vec::new();
        let mut shader_changes = Vec::new();
        for (relative, contents) in sources {
            crate::filesystem::validate_relative(&relative).map_err(ProjectOptionError::Failure)?;
            let target = shader_directory.join(&relative);
            crate::filesystem::no_links(&target).map_err(ProjectOptionError::Failure)?;
            match fs::read(&target) {
                Ok(existing) if existing == contents => {}
                Ok(_) => conflicts.push(relative.clone()),
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    shader_changes.push(FileChange {
                        relative: Path::new("data/shaders").join(relative),
                        contents,
                    });
                }
                Err(error) => return Err(ProjectOptionError::Failure(error.to_string())),
            }
        }
        if !conflicts.is_empty() && conflict_policy == ShaderConflictPolicy::Reject {
            return Err(ProjectOptionError::ShaderConflicts(conflicts));
        }
        if conflict_policy == ShaderConflictPolicy::Replace {
            for relative in &conflicts {
                let source = shader_root.join(relative);
                let contents = fs::read(&source).map_err(|error| {
                    ProjectOptionError::Failure(format!(
                        "Could not read MayaFlux shader {}: {error}",
                        source.display()
                    ))
                })?;
                shader_changes.push(FileChange {
                    relative: Path::new("data/shaders").join(relative),
                    contents,
                });
            }
        }
        if !shader_changes.is_empty() {
            messages.push(format!(
                "Copied {} MayaFlux shader source file(s) into data/shaders.",
                shader_changes.len()
            ));
            changes.extend(shader_changes);
        } else if conflicts.is_empty() {
            messages.push("Installed MayaFlux shader sources are already present.".to_owned());
        } else {
            messages.push("No non-conflicting MayaFlux shader files needed copying.".to_owned());
        }
        if conflict_policy == ShaderConflictPolicy::Skip && !conflicts.is_empty() {
            messages.push(format!(
                "Skipped {} shader file(s) that already contain project content.",
                conflicts.len()
            ));
        }
    }

    if !changes.is_empty() {
        commit_changes(&project, &changes)?;
    }
    Ok(messages)
}

fn resolve_project(project_path: &Path) -> Result<PathBuf, ProjectOptionError> {
    crate::filesystem::no_links(project_path).map_err(ProjectOptionError::Failure)?;
    let metadata = fs::symlink_metadata(project_path).map_err(|error| {
        ProjectOptionError::Failure(format!(
            "Could not inspect project folder {}: {error}",
            project_path.display()
        ))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(ProjectOptionError::Failure(format!(
            "Project path must be a real directory: {}",
            project_path.display()
        )));
    }
    let project = fs::canonicalize(project_path)?;
    crate::filesystem::no_links(&project).map_err(ProjectOptionError::Failure)?;
    if !project.join("CMakeLists.txt").is_file() {
        return Err(ProjectOptionError::Failure(format!(
            "Not a MayaFlux project: {}",
            project.display()
        )));
    }
    for relative in ["cmake/mayaflux.cmake", "data/shaders"] {
        crate::filesystem::no_links(&project.join(relative))
            .map_err(ProjectOptionError::Failure)?;
    }
    if !project.join("cmake/mayaflux.cmake").is_file() {
        return Err(ProjectOptionError::Failure(format!(
            "Unsupported project layout: {} is missing. Keep a backup and review the project's MayaFlux CMake integration before adding options.",
            project.join("cmake/mayaflux.cmake").display()
        )));
    }
    Ok(project)
}

fn inspect_lila(path: &Path) -> Result<FeatureStatus, ProjectOptionError> {
    let cmake = fs::read_to_string(path).map_err(|error| {
        ProjectOptionError::Failure(format!("Could not read {}: {error}", path.display()))
    })?;
    let has_link = cmake.contains(lila_link_block());
    let has_debugger_path = cmake.contains(lila_debugger_path());
    let has_host_runtime = cmake.contains("MayaFluxHost.dll")
        && cmake.contains("add_custom_command(TARGET ${PROJECT_NAME} POST_BUILD");
    if has_link && has_debugger_path && has_host_runtime {
        return Ok(FeatureStatus::Enabled);
    }
    if !has_link
        && !has_debugger_path
        && !has_host_runtime
        && cmake.contains("# Optional: Lila (Live Coding JIT)")
        && cmake.contains("VS_DEBUGGER_ENVIRONMENT")
        && cmake.contains("# Windows subsystem configuration")
    {
        return Ok(FeatureStatus::Available);
    }
    Ok(FeatureStatus::Unknown(
        "Some Lila integration is present or the expected CMake sections were changed. Review cmake/mayaflux.cmake before editing.".to_owned(),
    ))
}

fn inspect_shaders(project: &Path, shader_root: Option<&Path>) -> FeatureStatus {
    let Some(shader_root) = shader_root else {
        return FeatureStatus::Unknown(
            "MayaFlux shader sources could not be located. Install MayaFlux or set MAYAFLUX_ROOT."
                .to_owned(),
        );
    };
    let Ok(installed) = read_shader_sources_at(shader_root) else {
        return FeatureStatus::Unknown(
            "MayaFlux shader sources could not be read. Check the installation and retry."
                .to_owned(),
        );
    };
    let Ok(local) = shader_sources_in(&project.join("data/shaders")) else {
        return FeatureStatus::Unknown(
            "Project shader files could not be inspected safely.".to_owned(),
        );
    };
    if local.is_empty() {
        return FeatureStatus::Available;
    }
    if local.len() == installed.len()
        && installed.iter().all(|(path, contents)| {
            local
                .get(path)
                .is_some_and(|local_contents| local_contents == contents)
        })
    {
        return FeatureStatus::Enabled;
    }
    FeatureStatus::Unknown(
        "Project shaders are present and may include user files. Existing files will be listed before any replacement.".to_owned(),
    )
}

fn shader_sources_in(root: &Path) -> Result<std::collections::BTreeMap<PathBuf, Vec<u8>>, String> {
    crate::filesystem::no_links(root)?;
    if !root.exists() {
        return Ok(std::collections::BTreeMap::new());
    }
    let mut pending = vec![(root.to_path_buf(), PathBuf::new())];
    let mut files = std::collections::BTreeMap::new();
    while let Some((directory, relative_directory)) = pending.pop() {
        for entry in fs::read_dir(&directory).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            let path = entry.path();
            crate::filesystem::no_links(&path)?;
            let relative = relative_directory.join(entry.file_name());
            let kind = entry.file_type().map_err(|error| error.to_string())?;
            if kind.is_dir() {
                pending.push((path, relative));
            } else if kind.is_file()
                && path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| {
                        SHADER_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
                    })
            {
                files.insert(relative, fs::read(path).map_err(|error| error.to_string())?);
            }
        }
    }
    Ok(files)
}

fn enable_lila(cmake: &str) -> Result<String, String> {
    let has_link = cmake.contains(lila_link_block());
    let has_debugger = cmake.contains(lila_debugger_path());
    let has_host_runtime = cmake.contains("MayaFluxHost.dll")
        && cmake.contains("add_custom_command(TARGET ${PROJECT_NAME} POST_BUILD");
    if has_link && has_debugger && has_host_runtime {
        return Ok(cmake.to_owned());
    }
    if has_link || has_debugger || has_host_runtime {
        return Err("Lila is partially configured in cmake/mayaflux.cmake. Review the existing integration and resolve it manually before retrying.".to_owned());
    }
    let link_anchor = "# Optional: Lila (Live Coding JIT)\n";
    if cmake.matches(link_anchor).count() != 1 {
        return Err("Unsupported CMake layout: the Lila insertion section in cmake/mayaflux.cmake could not be identified safely.".to_owned());
    }
    let debugger_anchor = "$<TARGET_FILE_DIR:MayaFlux::MayaFluxLib>;";
    if cmake.matches(debugger_anchor).count() != 1 {
        return Err("Unsupported CMake layout: the Windows debugger path in cmake/mayaflux.cmake could not be identified safely.".to_owned());
    }
    let windows_anchor = "    # Windows subsystem configuration\n";
    if cmake.matches(windows_anchor).count() != 1 {
        return Err("Unsupported CMake layout: the Windows runtime section in cmake/mayaflux.cmake could not be identified safely.".to_owned());
    }

    let mut updated = cmake.replacen(
        link_anchor,
        &format!("{link_anchor}{}", lila_link_block()),
        1,
    );
    updated = updated.replacen(
        debugger_anchor,
        &format!("{debugger_anchor}{}", lila_debugger_path()),
        1,
    );
    let dll_block = lila_dll_copy_block();
    let runtime_marker = "    # WEAVE: Lila Windows runtime integration\n";
    match updated.matches(runtime_marker).count() {
        1 => {
            let empty_slot = format!("{runtime_marker}    \n");
            if updated.matches(&empty_slot).count() != 1 {
                return Err("The Lila Windows runtime section contains custom content. Review cmake/mayaflux.cmake before editing.".to_owned());
            }
            let replacement = format!("{runtime_marker}    {dll_block}\n");
            updated = updated.replacen(&empty_slot, &replacement, 1);
        }
        0 => {
            let insert_at = updated.find(windows_anchor).ok_or_else(|| {
                "Unsupported CMake layout: the Windows runtime section changed during update."
                    .to_owned()
            })?;
            updated.insert_str(insert_at, &format!("    {dll_block}\n\n"));
        }
        _ => {
            return Err("Unsupported CMake layout: duplicate Lila runtime markers were found. Review cmake/mayaflux.cmake before editing.".to_owned());
        }
    }
    Ok(updated)
}

#[derive(Clone)]
struct FileChange {
    relative: PathBuf,
    contents: Vec<u8>,
}

fn commit_changes(project: &Path, changes: &[FileChange]) -> Result<(), ProjectOptionError> {
    commit_changes_with(project, changes, crate::filesystem::atomic_write)
}

fn commit_changes_with(
    project: &Path,
    changes: &[FileChange],
    mut write: impl FnMut(&Path, &[u8]) -> io::Result<()>,
) -> Result<(), ProjectOptionError> {
    static NEXT_TRANSACTION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT_TRANSACTION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let transaction = project.join(format!(".weave-options-{}-{sequence}", std::process::id()));
    let mut backups = Vec::new();
    for change in changes {
        crate::filesystem::validate_relative(&change.relative)
            .map_err(ProjectOptionError::Failure)?;
        let target = project.join(&change.relative);
        crate::filesystem::no_links(&target).map_err(ProjectOptionError::Failure)?;
        if let Some(parent) = target.parent() {
            crate::filesystem::no_links(parent).map_err(ProjectOptionError::Failure)?;
        }
        if let Ok(metadata) = fs::symlink_metadata(&target)
            && !metadata.is_file()
        {
            return Err(ProjectOptionError::Failure(format!(
                "Option target is not a regular file: {}",
                target.display()
            )));
        }
        let original = match fs::read(&target) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(ProjectOptionError::Failure(error.to_string())),
        };
        backups.push(original);
    }
    fs::create_dir(&transaction)?;
    for (index, original) in backups.iter().enumerate() {
        if let Some(contents) = original {
            if let Err(error) = fs::write(transaction.join(format!("{index}.backup")), contents) {
                let _ = fs::remove_dir_all(&transaction);
                return Err(ProjectOptionError::Failure(format!(
                    "Could not prepare recovery data: {error}"
                )));
            }
        }
    }
    let manifest: Vec<_> = changes
        .iter()
        .enumerate()
        .map(|(index, change)| {
            serde_json::json!({
                "projectPath": change.relative,
                "originalFileBackup": if backups[index].is_some() {
                    Some(format!("{index}.backup"))
                } else {
                    None
                }
            })
        })
        .collect();
    let manifest = match serde_json::to_vec_pretty(&manifest) {
        Ok(manifest) => manifest,
        Err(error) => {
            let _ = fs::remove_dir_all(&transaction);
            return Err(ProjectOptionError::Failure(format!(
                "Could not prepare recovery manifest: {error}"
            )));
        }
    };
    if let Err(error) = fs::write(transaction.join("recovery.json"), manifest) {
        let _ = fs::remove_dir_all(&transaction);
        return Err(ProjectOptionError::Failure(format!(
            "Could not prepare recovery manifest: {error}"
        )));
    }

    let mut committed = 0usize;
    let mut created_directories = Vec::new();
    for (index, change) in changes.iter().enumerate() {
        let target = project.join(&change.relative);
        if let Some(parent) = target.parent()
            && let Err(error) = create_parent_directories(project, parent, &mut created_directories)
        {
            let rollback = rollback_changes(project, changes, &backups, committed);
            remove_empty_directories(&created_directories);
            return match rollback {
                Ok(()) => {
                    let _ = fs::remove_dir_all(&transaction);
                    Err(ProjectOptionError::Failure(format!(
                        "Could not prepare {}: {error}. Earlier changes were rolled back; retry the operation.",
                        change.relative.display()
                    )))
                }
                Err(rollback_error) => Err(ProjectOptionError::Failure(format!(
                    "Could not prepare {}: {error}. Rollback also failed: {rollback_error}. Recovery data and recovery.json remain at {}.",
                    change.relative.display(),
                    transaction.display()
                ))),
            };
        }
        if let Err(error) = write(&target, &change.contents) {
            let rollback = rollback_changes(project, changes, &backups, committed);
            remove_empty_directories(&created_directories);
            return match rollback {
                Ok(()) => {
                    let _ = fs::remove_dir_all(&transaction);
                    Err(ProjectOptionError::Failure(format!(
                        "Could not update {}: {error}. Earlier changes were rolled back; retry the operation.",
                        change.relative.display()
                    )))
                }
                Err(rollback_error) => Err(ProjectOptionError::Failure(format!(
                    "Could not update {}: {error}. Rollback also failed: {rollback_error}. Recovery data and recovery.json remain at {}.",
                    change.relative.display(),
                    transaction.display()
                ))),
            };
        }
        committed = index + 1;
    }
    fs::remove_dir_all(transaction)?;
    Ok(())
}

fn create_parent_directories(
    project: &Path,
    parent: &Path,
    created: &mut Vec<PathBuf>,
) -> io::Result<()> {
    let mut missing = Vec::new();
    let mut cursor = parent;
    while cursor != project && !cursor.exists() {
        missing.push(cursor.to_path_buf());
        cursor = cursor.parent().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "option path escaped project root",
            )
        })?;
    }
    for directory in missing.into_iter().rev() {
        fs::create_dir(&directory)?;
        created.push(directory);
    }
    Ok(())
}

fn remove_empty_directories(directories: &[PathBuf]) {
    for directory in directories.iter().rev() {
        let _ = fs::remove_dir(directory);
    }
}

fn rollback_changes(
    project: &Path,
    changes: &[FileChange],
    backups: &[Option<Vec<u8>>],
    committed: usize,
) -> io::Result<()> {
    let mut first_error = None;
    for index in (0..committed).rev() {
        let target = project.join(&changes[index].relative);
        let result = match &backups[index] {
            Some(contents) => crate::filesystem::atomic_write(&target, contents),
            None => fs::remove_file(&target),
        };
        if let Err(error) = result {
            first_error.get_or_insert(error);
        }
    }
    if let Some(error) = first_error {
        Err(error)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FeatureStatus, FileChange, ShaderConflictPolicy, apply_project_options_at,
        commit_changes_with, inspect_project_options_at, lila_debugger_path, lila_dll_copy_block,
        lila_link_block,
    };
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    struct Temporary(PathBuf);

    impl Temporary {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let id = NEXT.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir()
                .join(format!("weave-manage-options-{}-{id}", std::process::id()));
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

        assert!(error.to_string().contains("nested/effect.frag"));
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
}
