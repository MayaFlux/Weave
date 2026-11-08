//! Inspects and applies optional features to an existing MayaFlux project.
//! Edits are planned before a single transaction changes project files.

mod lila;
mod shaders;
#[cfg(test)]
mod tests;
mod transaction;

use super::project::read_shader_sources_at;
#[cfg(test)]
use lila::enable_lila;
use lila::inspect_lila;
use shaders::inspect_shaders;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
#[cfg(test)]
use transaction::commit_changes_with;
use transaction::{FileChange, commit_changes};

/// Whether an optional feature is present, addable, or needs manual review.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FeatureStatus {
    Enabled,
    Available,
    Unknown(String),
}

/// Feature states and actions available for a project.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectOptionInspection {
    pub lila: FeatureStatus,
    pub shader_sources: FeatureStatus,
    pub can_add_lila: bool,
    pub can_copy_shader_sources: bool,
}

/// How existing project shaders are handled when installed sources differ.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShaderConflictPolicy {
    Reject,
    Skip,
    Replace,
}

/// A project edit failure or a set of shader files needing a user choice.
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

/// File edits and status messages prepared before any project file is changed.
#[derive(Default)]
struct OptionPlan {
    changes: Vec<FileChange>,
    messages: Vec<String>,
}

/// Inspects options using shader sources from the current MayaFlux installation.
pub fn inspect_project_options(
    project_path: &Path,
) -> Result<ProjectOptionInspection, ProjectOptionError> {
    let shader_root = crate::platform::shader_source_root().ok();
    inspect_project_options_at(project_path, shader_root.as_deref())
}

/// Inspects options using an explicit shader source location.
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

/// Applies selected options using the current MayaFlux installation.
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

/// Plans selected edits and applies them together with rollback on failure.
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
    let mut plan = OptionPlan::default();
    if with_lila {
        let lila = lila::plan(&project)?;
        plan.changes.extend(lila.changes);
        plan.messages.extend(lila.messages);
    }
    if copy_shader_sources {
        let shaders = shaders::plan(&project, shader_root, conflict_policy)?;
        plan.changes.extend(shaders.changes);
        plan.messages.extend(shaders.messages);
    }

    if !plan.changes.is_empty() {
        commit_changes(&project, &plan.changes)?;
    }
    Ok(plan.messages)
}

/// Resolves a supported project without following linked project paths.
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
