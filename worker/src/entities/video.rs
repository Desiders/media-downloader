use super::{
    PreferredLanguages,
    combined_format::{Format, Formats},
    format::{Any, Audios, Kind},
};
use crate::{
    utils::{calculate_aspect_ratio, thumbnail::get_urls_by_aspect},
    value_objects::AspectKind,
};

#[derive(thiserror::Error, Debug)]
#[error("Video format not found")]
pub struct VideoFormatNotFound;

use serde::Deserialize;
use std::{
    ops::{Deref, DerefMut},
    path::PathBuf,
    vec,
};
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
    pub display_id: Option<String>,
    pub title: Option<String>,
    pub uploader: Option<String>,
    pub thumbnail: Option<String>,
    pub thumbnails: Option<Vec<Thumbnail>>,
    pub original_url: String,
    pub duration: Option<f64>,
    pub width: Option<i64>,
    pub height: Option<i64>,

    #[serde(flatten)]
    format: Option<Any>,
    #[serde(default)]
    formats: Vec<Any>,
}

impl Video {
    pub fn get_combined_formats(&self) -> Formats<'_> {
        let mut format_kinds = vec![];

        for format in &self.formats {
            let Ok(format) = format.kind(self.duration) else {
                continue;
            };

            format_kinds.push(format);
        }
        if let Some(format) = &self.format {
            if let Ok(format) = format.kind(self.duration) {
                format_kinds.push(format);
            }
        }

        Formats::from(format_kinds)
    }

    pub fn get_audio_formats(&self) -> Audios<'_> {
        let mut formats = vec![];

        for format in &self.formats {
            let Ok(format) = format.kind(self.duration) else {
                continue;
            };

            if let Kind::Audio(format) = format {
                formats.push(format);
            }
        }
        if let Some(format) = &self.format {
            if let Ok(Kind::Audio(format)) = format.kind(self.duration) {
                formats.push(format);
            }
        }

        Audios::from(formats)
    }

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

#[derive(Debug, Default, Clone, Deserialize)]
pub struct VideosInYT(Vec<Video>);

impl VideosInYT {
    pub fn new(videos: impl Into<Vec<Video>>) -> Self {
        Self(videos.into())
    }

    pub const fn len(&self) -> usize {
        self.0.len()
    }
}

impl IntoIterator for VideosInYT {
    type Item = Video;
    type IntoIter = vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Extend<Video> for VideosInYT {
    fn extend<T: IntoIterator<Item = Video>>(&mut self, iter: T) {
        self.0.extend(iter);
    }
}

impl Deref for VideosInYT {
    type Target = Vec<Video>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for VideosInYT {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct VideoAndFormat<'a> {
    pub video: &'a Video,
    pub format: Format<'a>,
}

impl<'a> VideoAndFormat<'a> {
    pub fn new_with_select_format(
        video: &'a Video,
        max_file_size: u32,
        PreferredLanguages { languages }: &PreferredLanguages,
    ) -> Result<Self, VideoFormatNotFound> {
        let mut formats = video.get_combined_formats();
        formats.sort(max_file_size, languages);

        let Some(format) = formats.first().cloned() else {
            return Err(VideoFormatNotFound);
        };

        Ok(Self { video, format })
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct VideoInFS {
    pub path: PathBuf,
    pub thumbnail_path: Option<PathBuf>,
    pub temp_dir: TempDir,
}

impl VideoInFS {
    pub fn new(path: impl Into<PathBuf>, thumbnail_path: Option<PathBuf>, temp_dir: TempDir) -> Self {
        Self {
            path: path.into(),
            thumbnail_path,
            temp_dir,
        }
    }
}
