use crate::{
    utils::{calculate_aspect_ratio, thumbnail::get_urls_by_aspect},
    value_objects::AspectKind,
};

use serde::Deserialize;
use std::path::PathBuf;
use tempfile::TempDir;
use url::Host;

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone, Deserialize)]
pub struct Video {
    pub id: String,
    pub thumbnails: Vec<String>,
    pub original_url: String,
    pub width: Option<i64>,
    pub height: Option<i64>,
}

impl Video {
    #[must_use]
    pub fn thumbnail_urls<'a>(&'a self, service_host: Option<&Host<&str>>) -> Vec<String> {
        let aspect_ratio = calculate_aspect_ratio(self.width, self.height);
        let aspect_kind = AspectKind::get_nearest(aspect_ratio);

        let mut urls = get_urls_by_aspect(service_host, &self.id, aspect_kind);
        for url in self.thumbnails.iter() {
            urls.push(url.clone());
        }
        urls
    }
}

#[derive(Debug)]
pub struct MediaInFS {
    pub path: PathBuf,
    pub thumbnail_path: Option<PathBuf>,
    pub temp_dir: TempDir,
}

impl MediaInFS {
    #[inline]
    #[must_use]
    pub fn new(path: impl Into<PathBuf>, thumbnail_path: Option<PathBuf>, temp_dir: TempDir) -> Self {
        Self {
            path: path.into(),
            thumbnail_path,
            temp_dir,
        }
    }
}
