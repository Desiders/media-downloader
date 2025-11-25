use std::io;
use tempfile::TempDir;
use tracing::{info, instrument};

use crate::{
    adapters::ffmpeg::download_thumbnail_to_path,
    entities::{MediaInFS, Thumbnail},
    interactors::Interactor,
};

#[derive(thiserror::Error, Debug)]
pub enum ErrorKind {
    #[error("Temp dir error: {0}")]
    TempDir(io::Error),
}

pub struct Download;

pub struct DownloadInput {
    thumbnail: Thumbnail,
}

impl DownloadInput {
    #[inline]
    #[must_use]
    pub const fn new(thumbnail: Thumbnail) -> Self {
        Self { thumbnail }
    }
}

impl Interactor<DownloadInput> for &Download {
    type Output = Option<MediaInFS>;
    type Err = ErrorKind;

    #[instrument(skip_all, fields(%thumbnail))]
    async fn execute(self, DownloadInput { thumbnail }: DownloadInput) -> Result<Self::Output, Self::Err> {
        let temp_dir = TempDir::new().map_err(Self::Err::TempDir)?;

        let temp_dir_path = temp_dir.path().to_path_buf();
        for thumbnail_url in thumbnail.thumbnail_urls() {
            if let Some(thumbnail_path) = download_thumbnail_to_path(thumbnail_url, &thumbnail.media_id, &temp_dir_path).await {
                info!("Thumbnail downloaded");
                return Ok(Some(MediaInFS::new(thumbnail_path, temp_dir)));
            }
        }
        Ok(None)
    }
}
