//! Inspects installed and project shader sources.

use super::{FeatureStatus, FileChange, OptionPlan, ProjectOptionError, ShaderConflictPolicy};
use crate::operations::project::{SHADER_EXTENSIONS, read_shader_sources_at};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Compares project shaders with the installed MayaFlux sources.
pub(super) fn inspect_shaders(project: &Path, shader_root: Option<&Path>) -> FeatureStatus {
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

/// Reads supported shader files without traversing linked paths.
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

/// Plans shader copies according to the requested conflict policy.
pub(super) fn plan(
    project: &Path,
    shader_root: Option<&Path>,
    conflict_policy: ShaderConflictPolicy,
) -> Result<OptionPlan, ProjectOptionError> {
    let mut plan = OptionPlan::default();
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
        plan.messages.push(format!(
            "Copied {} MayaFlux shader source file(s) into data/shaders.",
            shader_changes.len()
        ));
        plan.changes.extend(shader_changes);
    } else if conflicts.is_empty() {
        plan.messages
            .push("Installed MayaFlux shader sources are already present.".to_owned());
    } else {
        plan.messages
            .push("No non-conflicting MayaFlux shader files needed copying.".to_owned());
    }
    if conflict_policy == ShaderConflictPolicy::Skip && !conflicts.is_empty() {
        plan.messages.push(format!(
            "Skipped {} shader file(s) that already contain project content.",
            conflicts.len()
        ));
    }
    Ok(plan)
}
