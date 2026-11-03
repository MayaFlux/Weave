//! Compare a project with Weave's current templates and replace selected files.
use std::{
    fs, io,
    path::{Path, PathBuf},
};

use super::project::{ProjectOptions, project_template_files};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Difference {
    Missing,
    Changed,
}

#[derive(Clone, Debug)]
pub struct RefreshFile {
    pub relative: PathBuf,
    pub difference: Difference,
    original: Option<Vec<u8>>,
    replacement: Vec<u8>,
    executable: bool,
}

#[derive(Clone, Debug)]
pub struct RefreshPreview {
    pub project: PathBuf,
    pub files: Vec<RefreshFile>,
}

#[derive(Debug)]
pub struct RefreshResult {
    pub replaced: usize,
    pub backup: Option<PathBuf>,
}

pub fn preview(project: &Path) -> Result<RefreshPreview, String> {
    crate::filesystem::no_links(project)?;
    let project =
        fs::canonicalize(project).map_err(|error| format!("Could not open project: {error}"))?;
    if !project.is_dir()
        || (!project.join("CMakeLists.txt").is_file()
            && !project.join("cmake/mayaflux.cmake").is_file())
    {
        return Err(format!("Not a MayaFlux project: {}", project.display()));
    }
    crate::filesystem::no_links(&project)?;
    let cmake = match fs::read_to_string(project.join("CMakeLists.txt")) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(format!("Could not read CMakeLists.txt: {error}")),
    };
    if !cmake.contains("include(cmake/mayaflux)") && !project.join("cmake/mayaflux.cmake").is_file()
    {
        return Err(format!("Not a MayaFlux project: {}", project.display()));
    }
    let name = cmake
        .lines()
        .find_map(|line| {
            line.trim_start()
                .strip_prefix("project(")
                .and_then(|rest| rest.split_whitespace().next())
                .map(str::to_owned)
        })
        .or_else(|| {
            project
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .ok_or("Could not determine project name")?;
    super::project::validate_name(&name).map_err(|error| error.to_string())?;
    let lila_path = project.join("cmake/mayaflux.cmake");
    crate::filesystem::no_links(&lila_path)?;
    let with_lila = match fs::read_to_string(&lila_path) {
        Ok(contents) => contents.contains(super::project::lila_link_block()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => false,
        Err(error) => return Err(format!("Could not read {}: {error}", lila_path.display())),
    };
    let options = ProjectOptions {
        with_lila,
        with_vscode: true,
        copy_shader_sources: false,
    };
    let mut files = Vec::new();
    for (relative, replacement, executable) in project_template_files(&name, options) {
        if relative == Path::new("community.cmake") {
            continue;
        }
        let path = project.join(&relative);
        crate::filesystem::no_links(&path)?;
        let original = match fs::read(&path) {
            Ok(contents) => Some(contents),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(format!("Could not read {}: {error}", path.display())),
        };
        let replacement = replacement.into_bytes();
        if original.as_ref() == Some(&replacement) {
            continue;
        }
        let difference = if original.is_some() {
            Difference::Changed
        } else {
            Difference::Missing
        };
        files.push(RefreshFile {
            relative,
            difference,
            original,
            replacement,
            executable,
        });
    }
    Ok(RefreshPreview { project, files })
}

pub fn apply(preview: &RefreshPreview, selected: &[PathBuf]) -> Result<RefreshResult, String> {
    if selected.is_empty() {
        return Ok(RefreshResult {
            replaced: 0,
            backup: None,
        });
    }
    let mut changes = Vec::new();
    for relative in selected {
        crate::filesystem::validate_relative(relative)?;
        if changes
            .iter()
            .any(|file: &&RefreshFile| &file.relative == relative)
        {
            return Err(format!(
                "Duplicate refresh selection: {}",
                relative.display()
            ));
        }
        let file = preview
            .files
            .iter()
            .find(|file| &file.relative == relative)
            .ok_or_else(|| format!("File is not in the refresh preview: {}", relative.display()))?;
        changes.push(file);
    }
    crate::filesystem::no_links(&preview.project)?;
    for file in &changes {
        let path = preview.project.join(&file.relative);
        crate::filesystem::no_links(&path)?;
        let current = match fs::read(&path) {
            Ok(contents) => Some(contents),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(format!("Could not read {}: {error}", path.display())),
        };
        if current != file.original {
            return Err(format!(
                "{} changed since preview. Reopen Refresh Project Files.",
                file.relative.display()
            ));
        }
    }
    let backup = preview.project.join(".weave-refresh-backups").join(format!(
        "{}-{}",
        std::process::id(),
        crate::filesystem::unique_id()
    ));
    crate::filesystem::no_links(&backup)?;
    let mut completed: Vec<&RefreshFile> = Vec::new();
    for file in &changes {
        let target = preview.project.join(&file.relative);
        let saved = backup.join(&file.relative);
        let mut moved_original = false;
        let result: io::Result<()> = (|| {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            if file.original.is_some() {
                if let Some(parent) = saved.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::rename(&target, &saved)?;
                moved_original = true;
            }
            crate::filesystem::atomic_write(&target, &file.replacement)?;
            #[cfg(unix)]
            if file.executable {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&target, fs::Permissions::from_mode(0o755))?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            let mut restore_errors = Vec::new();
            if moved_original || file.original.is_none() {
                if let Err(error) = fs::remove_file(&target) {
                    if error.kind() != io::ErrorKind::NotFound {
                        restore_errors.push(error.to_string());
                    }
                }
            }
            if moved_original {
                if let Err(error) = fs::rename(&saved, &target) {
                    restore_errors.push(error.to_string());
                }
            }
            for done in completed.iter().rev() {
                let path = preview.project.join(&done.relative);
                if let Err(error) = fs::remove_file(&path) {
                    if error.kind() != io::ErrorKind::NotFound {
                        restore_errors.push(error.to_string());
                    }
                }
                if done.original.is_some() {
                    if let Err(error) = fs::rename(backup.join(&done.relative), &path) {
                        restore_errors.push(error.to_string());
                    }
                }
            }
            let mut message = format!("Could not refresh {}: {error}", file.relative.display());
            if !restore_errors.is_empty() {
                message.push_str(&format!(
                    ". Could not fully restore previous files: {}. Check {}",
                    restore_errors.join("; "),
                    backup.display()
                ));
            }
            return Err(message);
        }
        completed.push(file);
    }
    Ok(RefreshResult {
        replaced: completed.len(),
        backup: changes
            .iter()
            .any(|file| file.original.is_some())
            .then_some(backup),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operations::project::create_project;

    #[test]
    fn preview_shows_only_missing_and_changed_files_and_apply_respects_selection() {
        let root =
            std::env::temp_dir().join(format!("weave-refresh-{}", crate::filesystem::unique_id()));
        fs::create_dir(&root).unwrap();
        let project = create_project("RefreshDemo", &root, ProjectOptions::default())
            .unwrap()
            .path;
        fs::write(project.join("src/main.cpp"), "custom main").unwrap();
        fs::remove_file(project.join("CMakePresets.json")).unwrap();
        let before = preview(&project).unwrap();
        assert_eq!(before.files.len(), 2);
        assert!(
            before
                .files
                .iter()
                .any(|file| file.relative == Path::new("src/main.cpp")
                    && file.difference == Difference::Changed)
        );
        assert!(
            before
                .files
                .iter()
                .any(|file| file.relative == Path::new("CMakePresets.json")
                    && file.difference == Difference::Missing)
        );
        let result = apply(&before, &[PathBuf::from("src/main.cpp")]).unwrap();
        assert_eq!(result.replaced, 1);
        assert_eq!(
            fs::read(result.backup.unwrap().join("src/main.cpp")).unwrap(),
            b"custom main"
        );
        assert!(!project.join("CMakePresets.json").exists());
        let after = preview(&project).unwrap();
        assert_eq!(after.files.len(), 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn apply_rejects_files_changed_after_preview() {
        let root =
            std::env::temp_dir().join(format!("weave-refresh-{}", crate::filesystem::unique_id()));
        fs::create_dir(&root).unwrap();
        let project = create_project("RefreshDemo", &root, ProjectOptions::default())
            .unwrap()
            .path;
        fs::write(project.join("src/main.cpp"), "first edit").unwrap();
        let before = preview(&project).unwrap();
        fs::write(project.join("src/main.cpp"), "second edit").unwrap();
        assert!(apply(&before, &[PathBuf::from("src/main.cpp")]).is_err());
        assert_eq!(
            fs::read(project.join("src/main.cpp")).unwrap(),
            b"second edit"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn refresh_can_restore_a_missing_cmake_lists_file() {
        let root =
            std::env::temp_dir().join(format!("weave-refresh-{}", crate::filesystem::unique_id()));
        fs::create_dir(&root).unwrap();
        let project = create_project("RefreshDemo", &root, ProjectOptions::default())
            .unwrap()
            .path;
        fs::remove_file(project.join("CMakeLists.txt")).unwrap();
        let before = preview(&project).unwrap();
        let file = before
            .files
            .iter()
            .find(|file| file.relative == Path::new("CMakeLists.txt"))
            .unwrap();
        assert_eq!(file.difference, Difference::Missing);
        apply(&before, &[PathBuf::from("CMakeLists.txt")]).unwrap();
        assert!(
            fs::read_to_string(project.join("CMakeLists.txt"))
                .unwrap()
                .contains("project(RefreshDemo")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn preview_rejects_an_unrelated_cmake_project() {
        let root =
            std::env::temp_dir().join(format!("weave-refresh-{}", crate::filesystem::unique_id()));
        fs::create_dir(&root).unwrap();
        fs::write(
            root.join("CMakeLists.txt"),
            "project(Unrelated LANGUAGES CXX)\n",
        )
        .unwrap();
        assert!(preview(&root).is_err());
        assert!(!root.join(".weave-refresh-backups").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
