use std::{io, sync::Arc};
use tempfile::TempDir;
use tracing::{info, instrument};
use url::Url;

use crate::{
    adapters::{ffmpeg::download_thumbnail_to_path, ytdl::download_audio_to_path},
    config,
    entities::{Cookie, MediaInFS, Video, format},
    interactors::Interactor,
};

const DOWNLOAD_TIMEOUT: u64 = 360;

#[derive(thiserror::Error, Debug)]
pub enum ErrorKind {
    #[error("Ytdlp error: {0}")]
    Ytdlp(io::Error),
    #[error("Temp dir error: {0}")]
    TempDir(io::Error),
}

pub struct Download {
    yt_dlp_cfg: Arc<config::YtDlp>,
    limits_cfg: Arc<config::Limits>,
    yt_pot_provider_cfg: Arc<config::YtPotProvider>,
}

impl Download {
    #[inline]
    #[must_use]
    pub const fn new(
        yt_dlp_cfg: Arc<config::YtDlp>,
        limits_cfg: Arc<config::Limits>,
        yt_pot_provider_cfg: Arc<config::YtPotProvider>,
    ) -> Self {
        Self {
            yt_dlp_cfg,
            limits_cfg,
            yt_pot_provider_cfg,
        }
    }
}

pub struct DownloadInput<'a> {
    url: &'a Url,
    cookie: Option<&'a Cookie>,
    video: &'a Video,
    format: format::Audio<'a>,
}

impl<'a> DownloadInput<'a> {
    #[inline]
    #[must_use]
    pub const fn new(url: &'a Url, cookie: Option<&'a Cookie>, video: &'a Video, format: format::Audio<'a>) -> Self {
        Self {
            url,
            cookie,
            video,
            format,
        }
    }
}

impl Interactor<DownloadInput<'_>> for &Download {
    type Output = MediaInFS;
    type Err = ErrorKind;

    #[instrument(skip_all, fields(%format))]
    async fn execute(
        self,
        DownloadInput {
            url,
            cookie,
            video,
            format,
        }: DownloadInput<'_>,
    ) -> Result<Self::Output, Self::Err> {
        let extension = format.extension();
        let host = url.host();
        let thumbnail_urls = video.thumbnail_urls(host.as_ref());
        let temp_dir = TempDir::new().map_err(Self::Err::TempDir)?;
        let file_path = temp_dir.path().join(format!("{}.{}", video.id, extension));

        let (download_res, thumbnail_path) = tokio::join!(
            {
                let url = video.original_url.clone();
                let yt_dlp_executable_path = self.yt_dlp_cfg.executable_path.clone();
                let yt_pot_provider_url = self.yt_pot_provider_cfg.url.clone();
                let temp_dir_path = temp_dir.path().to_path_buf();
                async move {
                    download_audio_to_path(
                        yt_dlp_executable_path,
                        url,
                        yt_pot_provider_url,
                        format.id,
                        extension,
                        temp_dir_path,
                        DOWNLOAD_TIMEOUT,
                        self.limits_cfg.max_file_size,
                        cookie,
                    )
                    .await
                }
            },
            {
                let temp_dir_path = temp_dir.path().to_path_buf();
                async move {
                    for thumbnail_url in thumbnail_urls {
                        if let Some(thumbnail_path) = download_thumbnail_to_path(thumbnail_url, &video.id, &temp_dir_path).await {
                            info!("Thumbnail downloaded");
                            return Some(thumbnail_path);
                        }
                    }
                    None
                }
            }
        );
        if let Err(err) = download_res {
            return Err(Self::Err::Ytdlp(err));
        }

        info!("Audio downloaded");
        Ok(Self::Output::new(file_path, thumbnail_path, temp_dir))
    }
}
