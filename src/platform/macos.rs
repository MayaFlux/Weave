use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub fn shader_source_root() -> Option<PathBuf> {
    for brew in brew_candidates() {
        for formula in ["mayaflux", "mayaflux-dev"] {
            if let Ok(output) = Command::new(&brew).args(["--prefix", formula]).output() {
                if output.status.success() {
                    if let Ok(prefix) = String::from_utf8(output.stdout) {
                        let path = PathBuf::from(prefix.trim()).join("share/MayaFlux/src/shaders");
                        if path.is_dir() {
                            return Some(path);
                        }
                    }
                }
            }
        }
    }
    None
}

fn brew_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path) = std::env::var_os("PATH") {
        candidates.extend(
            std::env::split_paths(&path)
                .map(|directory| directory.join("brew"))
                .filter(|candidate| !candidate.starts_with("/usr/local")),
        );
    }
    if let Ok(entries) = fs::read_dir("/opt") {
        candidates.extend(entries.flatten().map(|entry| entry.path().join("bin/brew")));
    }
    candidates.retain(|candidate| candidate.is_file());
    candidates
}
