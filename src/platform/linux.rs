use std::path::{Path, PathBuf};

pub fn shader_source_root() -> Option<PathBuf> {
    ["/usr", "/usr/local"]
        .into_iter()
        .map(|prefix| Path::new(prefix).join("share/MayaFlux/src/shaders"))
        .find(|path| path.is_dir())
}
