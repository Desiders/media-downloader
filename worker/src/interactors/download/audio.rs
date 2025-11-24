use mpsc::{UnboundedReceiver, UnboundedSender, error::SendError, unbounded_channel};
use std::{io, sync::Arc};
use tempfile::TempDir;
use tokio::sync::mpsc;
use tracing::{info, info_span, instrument};
use url::Url;

use crate::{
    adapters::{ffmpeg::download_thumbnail_to_path, ytdl::download_audio_to_path},
    config,
    entities::{AudioAndFormat, AudioInFS, Cookie},
    interactors::Interactor,
};

const DOWNLOAD_TIMEOUT: u64 = 360;

#[derive(thiserror::Error, Debug)]
pub enum DownloadErrorKind {
    #[error("Ytdlp error: {0}")]
    Ytdlp(io::Error),
    #[error("Temp dir error: {0}")]
    TempDir(io::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum DownloadPlaylistErrorKind {
    #[error("Channel error: {0}")]
    Channel(#[from] SendError<(usize, Result<AudioInFS, DownloadErrorKind>)>),
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
    pub url: &'a Url,
    pub cookie: Option<Cookie>,
    pub audio_and_format: AudioAndFormat<'a>,
}

impl<'a> DownloadInput<'a> {
    #[inline]
    #[must_use]
    pub const fn new(url: &'a Url, cookie: Option<Cookie>, audio_and_format: AudioAndFormat<'a>) -> Self {
        Self {
            url,
            cookie,
            audio_and_format,
        }
    }
}

impl Interactor<DownloadInput<'_>> for &Download {
    type Output = AudioInFS;
    type Err = DownloadErrorKind;

    #[instrument(skip_all, fields(extension = format.codec.get_extension(), %format))]
    async fn execute(
        self,
        DownloadInput {
            audio_and_format: AudioAndFormat { video, format },
            cookie,
            url,
        }: DownloadInput<'_>,
    ) -> Result<Self::Output, Self::Err> {
        let extension = format.codec.get_extension();
        let temp_dir = TempDir::new().map_err(Self::Err::TempDir)?;
        let file_path = temp_dir.path().join(format!("{video_id}.{extension}", video_id = video.id));
        let host = url.host();
        let thumbnail_urls = video.thumbnail_urls(host.as_ref());

        let (download_res, thumbnail_path) = tokio::join!(
            {
                let url = video.original_url.clone();
                let yt_dlp_executable_path = self.yt_dlp_cfg.executable_path.clone();
                let yt_pot_provider_url = self.yt_pot_provider_cfg.url.clone();
                let temp_dir_path = temp_dir.path().to_path_buf();
                let cookie = cookie.as_ref();
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

        Ok(AudioInFS::new(file_path, thumbnail_path, temp_dir))
    }
}

pub struct DownloadPlaylist {
    yt_dlp_cfg: Arc<config::YtDlp>,
    limits_cfg: Arc<config::Limits>,
    yt_pot_provider_cfg: Arc<config::YtPotProvider>,
}

impl DownloadPlaylist {
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

pub struct DownloadPlaylistInput<'a> {
    pub url: &'a Url,
    pub cookie: Option<Cookie>,
    pub audios_and_formats: Vec<AudioAndFormat<'a>>,
    pub sender: UnboundedSender<(usize, Result<AudioInFS, DownloadErrorKind>)>,
}

impl<'a> DownloadPlaylistInput<'a> {
    #[inline]
    #[must_use]
    pub fn new(
        url: &'a Url,
        cookie: Option<Cookie>,
        audios_and_formats: Vec<AudioAndFormat<'a>>,
    ) -> (Self, UnboundedReceiver<(usize, Result<AudioInFS, DownloadErrorKind>)>) {
        let (sender, receiver) = unbounded_channel();
        (
            Self {
                url,
                cookie,
                audios_and_formats,
                sender,
            },
            receiver,
        )
    }
}

impl Interactor<DownloadPlaylistInput<'_>> for &DownloadPlaylist {
    type Output = ();
    type Err = DownloadPlaylistErrorKind;

    #[instrument(skip_all)]
    async fn execute(
        self,
        DownloadPlaylistInput {
            url,
            cookie,
            audios_and_formats,
            sender,
        }: DownloadPlaylistInput<'_>,
    ) -> Result<Self::Output, Self::Err> {
        let host = url.host();

        for (index, AudioAndFormat { video, format }) in audios_and_formats.into_iter().enumerate() {
            let extension = format.codec.get_extension();

            let span = info_span!("iter", extension, %format).entered();
            let temp_dir = TempDir::new().map_err(Self::Err::TempDir)?;
            let file_path = temp_dir.path().join(format!("{video_id}.{extension}", video_id = video.id));
            let thumbnail_urls = video.thumbnail_urls(host.as_ref());

            let span = span.exit();
            let (download_res, thumbnail_path) = tokio::join!(
                {
                    let url = video.original_url.clone();
                    let yt_dlp_executable_path = self.yt_dlp_cfg.executable_path.clone();
                    let yt_pot_provider_url = self.yt_pot_provider_cfg.url.clone();
                    let temp_dir_path = temp_dir.path().to_path_buf();
                    let cookie = cookie.as_ref();
                    async move {
                        let res = download_audio_to_path(
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
                        .await;
                        res
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
                let _guard = span.enter();
                sender.send((index, Err(DownloadErrorKind::Ytdlp(err))))?;
                continue;
            }

            let _guard = span.enter();
            info!("Audio downloaded");
            sender.send((index, Ok(AudioInFS::new(file_path, thumbnail_path, temp_dir))))?;
        }

        Ok(())
    }
}
