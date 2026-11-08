//! Applies planned file changes with backups and rollback.

use super::ProjectOptionError;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// A project-relative file replacement prepared by an option planner.
#[derive(Clone)]
pub(super) struct FileChange {
    pub(super) relative: PathBuf,
    pub(super) contents: Vec<u8>,
}

/// Writes all planned files, keeping recovery data until the transaction ends.
pub(super) fn commit_changes(
    project: &Path,
    changes: &[FileChange],
) -> Result<(), ProjectOptionError> {
    commit_changes_with(project, changes, crate::filesystem::atomic_write)
}

/// Runs the transaction with a replaceable writer for failure testing.
pub(super) fn commit_changes_with(
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

/// Restores files already written, leaving recovery data if restoration fails.
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
