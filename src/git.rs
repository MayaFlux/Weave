use std::path::Path;
use std::process::Command;

pub fn initialize_repository(path: &Path, label: &str) -> Option<String> {
    initialize_repository_with(path, label, |directory, arguments| {
        let output = Command::new("git")
            .arg("-C")
            .arg(directory)
            .args(arguments)
            .output()?;
        Ok(GitCommandResult {
            succeeded: output.status.success(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        })
    })
}

struct GitCommandResult {
    succeeded: bool,
    stderr: String,
}

fn initialize_repository_with(
    path: &Path,
    label: &str,
    mut run: impl FnMut(&Path, &[&str]) -> std::io::Result<GitCommandResult>,
) -> Option<String> {
    let init = match run(path, &["init", "-q", "-b", "main"]) {
        Ok(result) => result,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(error) => {
            return Some(format!(
                "Git could not initialize the {label} repository: {error}"
            ));
        }
    };
    if !init.succeeded {
        return Some(format!(
            "Git could not initialize the {label} repository: {}",
            init.stderr
        ));
    }

    let add = match run(path, &["add", "--all"]) {
        Ok(result) => result,
        Err(error) => return Some(format!("Git could not stage the {label} files: {error}")),
    };
    if !add.succeeded {
        return Some(format!(
            "Git could not stage the {label} files: {}",
            add.stderr
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn initializes_main_branch_then_stages_generated_files() {
        let requests = RefCell::new(Vec::new());
        let warning =
            initialize_repository_with(Path::new("fixture/project"), "project", |path, args| {
                requests.borrow_mut().push((
                    path.to_path_buf(),
                    args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>(),
                ));
                Ok(GitCommandResult {
                    succeeded: true,
                    stderr: String::new(),
                })
            });
        assert!(warning.is_none());
        assert_eq!(
            requests.into_inner(),
            vec![
                (
                    Path::new("fixture/project").to_path_buf(),
                    vec![
                        "init".to_owned(),
                        "-q".to_owned(),
                        "-b".to_owned(),
                        "main".to_owned()
                    ]
                ),
                (
                    Path::new("fixture/project").to_path_buf(),
                    vec!["add".to_owned(), "--all".to_owned()]
                ),
            ]
        );
    }

    #[test]
    fn missing_git_is_optional_and_does_not_attempt_staging() {
        let mut calls = 0;
        let result = initialize_repository_with(Path::new("fixture/project"), "project", |_, _| {
            calls += 1;
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "git not found",
            ))
        });
        assert!(result.is_none());
        assert_eq!(calls, 1);
    }

    #[test]
    fn init_failure_reports_a_warning_and_does_not_stage() {
        let mut calls = 0;
        let warning =
            initialize_repository_with(Path::new("fixture/module"), "community module", |_, _| {
                calls += 1;
                Ok(GitCommandResult {
                    succeeded: false,
                    stderr: "controlled init error".to_owned(),
                })
            })
            .unwrap();
        assert_eq!(calls, 1);
        assert!(warning.contains("community module repository"));
        assert!(warning.contains("controlled init error"));
    }

    #[test]
    fn staging_failure_reports_warning_after_repository_initialization() {
        let mut calls = 0;
        let warning =
            initialize_repository_with(Path::new("fixture/project"), "project", |_, _| {
                calls += 1;
                Ok(GitCommandResult {
                    succeeded: calls == 1,
                    stderr: if calls == 1 {
                        String::new()
                    } else {
                        "controlled staging error".to_owned()
                    },
                })
            })
            .unwrap();
        assert_eq!(calls, 2);
        assert!(warning.contains("stage the project files"));
        assert!(warning.contains("controlled staging error"));
    }
}
