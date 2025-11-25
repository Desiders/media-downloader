use serde::Deserialize;
use std::path::PathBuf;
use tempfile::TempDir;

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone, Deserialize)]
pub struct Video {
    pub id: String,
    pub url: String,
    pub width: Option<i64>,
    pub height: Option<i64>,
}

#[derive(Debug)]
pub struct MediaInFS {
    pub path: PathBuf,
    pub temp_dir: TempDir,
}

impl MediaInFS {
    #[inline]
    #[must_use]
    pub fn new(path: impl Into<PathBuf>, temp_dir: TempDir) -> Self {
        Self {
            path: path.into(),
            temp_dir,
        }
    }
}
