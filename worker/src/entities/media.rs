use crate::{
    utils::{calculate_aspect_ratio, thumbnail::get_urls_by_aspect},
    value_objects::AspectKind,
};

use serde::Deserialize;
use std::path::PathBuf;
use tempfile::TempDir;
use url::{Host, Url};

#[derive(Debug, Clone, Deserialize)]
pub struct Thumbnail {
    pub url: Option<String>,
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone, Deserialize)]
pub struct Video {
    pub id: String,
    pub thumbnail: Option<String>,
    pub thumbnails: Option<Vec<Thumbnail>>,
    pub original_url: String,
    pub width: Option<i64>,
    pub height: Option<i64>,
}

impl Video {
    pub fn thumbnail_urls<'a>(&'a self, service_host: Option<&Host<&str>>) -> Vec<String> {
        let aspect_ratio = calculate_aspect_ratio(self.width, self.height);
        let aspect_kind = AspectKind::get_nearest(aspect_ratio);
        let mut thumbnail_urls = get_urls_by_aspect(service_host, &self.id, aspect_kind);

        if let Some(url) = &self.thumbnail {
            thumbnail_urls.push(url.clone());
        }
        for Thumbnail { url } in self.thumbnails.as_deref().unwrap_or_default() {
            if let Some(url) = url.clone() {
                thumbnail_urls.push(url);
            }
        }
        thumbnail_urls
    }

    pub fn domain(&self) -> Option<String> {
        match Url::parse(&self.original_url) {
            Ok(url) => match url.domain() {
                Some(domain) => Some(domain.to_owned()),
                None => None,
            },
            Err(_) => None,
        }
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
