use std::{io, sync::Arc};
use tempfile::TempDir;
use tracing::{info, instrument};

use crate::{
    adapters::ytdl::download_audio_to_path,
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
    #[error("URL parse error: {0}")]
    Url(#[from] url::ParseError),
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

pub struct DownloadInput {
    video: Video,
    format: format::Audio,
    cookie: Option<Cookie>,
}

impl DownloadInput {
    #[inline]
    #[must_use]
    pub const fn new(video: Video, format: format::Audio, cookie: Option<Cookie>) -> Self {
        Self { video, format, cookie }
    }
}

impl Interactor<DownloadInput> for &Download {
    type Output = MediaInFS;
    type Err = ErrorKind;

    #[instrument(skip_all, fields(%format))]
    async fn execute(self, DownloadInput { video, format, cookie }: DownloadInput) -> Result<Self::Output, Self::Err> {
        let extension = format.extension();
        let temp_dir = TempDir::new().map_err(Self::Err::TempDir)?;
        let file_path = temp_dir.path().join(format!("{}.{}", video.id, extension));

        if let Err(err) = download_audio_to_path(
            &self.yt_dlp_cfg.executable_path,
            &video.url,
            &self.yt_pot_provider_cfg.url,
            &format.id,
            extension,
            temp_dir.path(),
            DOWNLOAD_TIMEOUT,
            self.limits_cfg.max_file_size,
            cookie.as_ref(),
        )
        .await
        {
            return Err(Self::Err::Ytdlp(err));
        }

        info!("Audio downloaded");
        Ok(Self::Output::new(file_path, temp_dir))
    }
}
