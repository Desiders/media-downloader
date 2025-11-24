use bytes::Bytes;
use futures_util::StreamExt as _;
use mpsc::{UnboundedReceiver, UnboundedSender, error::SendError, unbounded_channel};
use nix::{
    errno::Errno,
    fcntl::{FcntlArg::F_SETFD, FdFlag, fcntl},
    unistd::pipe,
};
use reqwest::Client;
use std::{fs::File, io, sync::Arc, time::Duration};
use tempfile::TempDir;
use tokio::{io::AsyncWriteExt as _, sync::mpsc, time::timeout};
use tracing::{Instrument as _, debug, error, info, info_span, instrument, trace};
use url::Url;

use crate::{
    adapters::{
        ffmpeg::{download_thumbnail_to_path, merge_streams},
        ytdl::{download_to_pipe, download_video_to_path},
    },
    config,
    entities::{Cookie, VideoAndFormat, VideoInFS},
    interactors::Interactor,
    utils::format_error_report,
};

const DOWNLOAD_TIMEOUT: u64 = 360;
const RANGE_CHUNK_SIZE: i32 = 1024 * 1024 * 10;

#[derive(thiserror::Error, Debug)]
pub enum RangeDownloadErrorKind {
    #[error("Channel error: {0}")]
    Channel(#[from] SendError<Bytes>),
    #[error("Request error: {0}")]
    Reqwest(#[from] reqwest::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum DownloadErrorKind {
    #[error("Ytdlp error: {0}")]
    Ytdlp(io::Error),
    #[error("Ffmpeg error: {0}")]
    Ffmpeg(io::Error),
    #[error("Pipe error: {0}")]
    Pipe(Errno),
    #[error("Temp dir error: {0}")]
    TempDir(io::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum DownloadPlaylistErrorKind {
    #[error("Channel error: {0}")]
    Channel(#[from] SendError<(usize, Result<VideoInFS, DownloadErrorKind>)>),
    #[error("Ytdlp error: {0}")]
    Ytdlp(io::Error),
    #[error("Ffmpeg error: {0}")]
    Ffmpeg(io::Error),
    #[error("Pipe error: {0}")]
    Pipe(Errno),
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
    pub video_and_format: VideoAndFormat<'a>,
}

impl<'a> DownloadInput<'a> {
    #[inline]
    #[must_use]
    pub const fn new(url: &'a Url, cookie: Option<Cookie>, video_and_format: VideoAndFormat<'a>) -> Self {
        Self {
            url,
            cookie,
            video_and_format,
        }
    }
}

impl Interactor<DownloadInput<'_>> for &Download {
    type Output = VideoInFS;
    type Err = DownloadErrorKind;

    #[instrument(skip_all, fields(extension = format.get_extension(), %format))]
    async fn execute(
        self,
        DownloadInput {
            video_and_format: VideoAndFormat { video, format },
            cookie,
            url,
        }: DownloadInput<'_>,
    ) -> Result<Self::Output, Self::Err> {
        let extension = format.get_extension();
        let temp_dir = TempDir::new().map_err(Self::Err::TempDir)?;
        let file_path = temp_dir.path().join(format!("{video_id}.{extension}", video_id = video.id));
        let host = url.host();
        let thumbnail_urls = video.thumbnail_urls(host.as_ref());

        if format.format_ids_are_equal() {
            debug!("Formats are the same");

            let (download_res, thumbnail_path) = tokio::join!(
                {
                    let url = video.original_url.clone();
                    let yt_dlp_executable_path = self.yt_dlp_cfg.executable_path.clone();
                    let yt_pot_provider_url = self.yt_pot_provider_cfg.url.clone();
                    let temp_dir_path = temp_dir.path().to_path_buf();
                    async move {
                        download_video_to_path(
                            yt_dlp_executable_path,
                            url,
                            yt_pot_provider_url,
                            format.video_format.id,
                            extension,
                            temp_dir_path,
                            DOWNLOAD_TIMEOUT,
                            self.limits_cfg.max_file_size,
                            cookie.as_ref(),
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
            info!("Video downloaded");

            return Ok(VideoInFS::new(file_path, thumbnail_path, temp_dir));
        }

        todo!()
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
    pub videos_and_formats: Vec<VideoAndFormat<'a>>,
    pub sender: UnboundedSender<(usize, Result<VideoInFS, DownloadErrorKind>)>,
}

impl<'a> DownloadPlaylistInput<'a> {
    #[inline]
    #[must_use]
    pub fn new(
        url: &'a Url,
        cookie: Option<Cookie>,
        videos_and_formats: Vec<VideoAndFormat<'a>>,
    ) -> (Self, UnboundedReceiver<(usize, Result<VideoInFS, DownloadErrorKind>)>) {
        let (sender, receiver) = unbounded_channel();
        (
            Self {
                url,
                cookie,
                videos_and_formats,
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
            videos_and_formats,
            sender,
        }: DownloadPlaylistInput<'_>,
    ) -> Result<Self::Output, Self::Err> {
        let host = url.host();

        for (index, VideoAndFormat { video, format }) in videos_and_formats.iter().enumerate() {
            let extension = format.get_extension();

            let span = info_span!("iter", extension, %format).entered();
            let temp_dir = TempDir::new().map_err(Self::Err::TempDir)?;
            let file_path = temp_dir.path().join(format!("{video_id}.{extension}", video_id = video.id));
            let thumbnail_urls = video.thumbnail_urls(host.as_ref());

            if format.format_ids_are_equal() {
                debug!("Formats are the same");

                let span = span.exit();
                let (download_res, thumbnail_path) = tokio::join!(
                    {
                        let url = video.original_url.clone();
                        let yt_dlp_executable_path = self.yt_dlp_cfg.executable_path.clone();
                        let yt_pot_provider_url = self.yt_pot_provider_cfg.url.clone();
                        let temp_dir_path = temp_dir.path().to_path_buf();
                        let cookie = cookie.as_ref();
                        async move {
                            download_video_to_path(
                                yt_dlp_executable_path,
                                url,
                                yt_pot_provider_url,
                                format.video_format.id,
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
                    let _guard = span.enter();
                    sender.send((index, Err(DownloadErrorKind::Ytdlp(err))))?;
                    continue;
                }

                let _guard = span.enter();
                info!("Video downloaded");
                sender.send((index, Ok(VideoInFS::new(file_path, thumbnail_path, temp_dir))))?;
                continue;
            }

            debug!("Formats are different");

            let (video_read_fd, video_write_fd) = pipe().map_err(Self::Err::Pipe)?;
            let (audio_read_fd, audio_write_fd) = pipe().map_err(Self::Err::Pipe)?;

            fcntl(&video_write_fd, F_SETFD(FdFlag::FD_CLOEXEC)).map_err(Self::Err::Pipe)?;
            fcntl(&audio_write_fd, F_SETFD(FdFlag::FD_CLOEXEC)).map_err(Self::Err::Pipe)?;

            let mut merge_child = merge_streams(&video_read_fd, &audio_read_fd, extension, &file_path, self.limits_cfg.max_file_size)
                .map_err(Self::Err::Ffmpeg)?;

            let span = span.exit();
            if let Some(filesize) = format.video_format.filesize_or_approx() {
                let (sender, mut receiver) = mpsc::unbounded_channel();

                let url = format.video_format.url.to_owned();
                tokio::spawn(
                    async move {
                        tokio::join!(
                            async move {
                                let _ = range_download_to_write(url, filesize, sender)
                                    .await
                                    .map_err(|err| error!("{}", format_error_report(&err)));
                            },
                            async move {
                                let mut writer = tokio::fs::File::from_std(File::from(video_write_fd));
                                while let Some(bytes) = receiver.recv().await {
                                    let _ = writer.write(&bytes).await.map_err(|err| error!("{}", format_error_report(&err)));
                                }
                            }
                        )
                    }
                    .instrument(span.clone()),
                );
            } else {
                let _guard = span.enter();
                download_to_pipe(
                    video_write_fd,
                    self.yt_dlp_cfg.executable_path.as_ref(),
                    &video.original_url,
                    self.yt_pot_provider_cfg.url.as_ref(),
                    format.video_format.id,
                    self.limits_cfg.max_file_size,
                    cookie.as_ref(),
                )
                .map_err(Self::Err::Ytdlp)?;
            }
            if let Some(filesize) = format.audio_format.filesize_or_approx() {
                let (sender, mut receiver) = mpsc::unbounded_channel();

                let url = format.audio_format.url.to_owned();
                tokio::spawn(
                    async move {
                        tokio::join!(
                            async move {
                                let _ = range_download_to_write(url, filesize, sender)
                                    .await
                                    .map_err(|err| error!("{}", format_error_report(&err)));
                            },
                            async move {
                                let mut writer = tokio::fs::File::from_std(File::from(audio_write_fd));
                                while let Some(bytes) = receiver.recv().await {
                                    let _ = writer.write(&bytes).await.map_err(|err| error!("{}", format_error_report(&err)));
                                }
                            }
                        )
                    }
                    .instrument(span.clone()),
                );
            } else {
                let _guard = span.enter();
                download_to_pipe(
                    audio_write_fd,
                    self.yt_dlp_cfg.executable_path.as_ref(),
                    &video.original_url,
                    self.yt_pot_provider_cfg.url.as_ref(),
                    format.audio_format.id,
                    self.limits_cfg.max_file_size,
                    cookie.as_ref(),
                )
                .map_err(Self::Err::Ytdlp)?;
            }

            let mut thumbnail_path = None;
            for thumbnail_url in thumbnail_urls {
                if let Some(path) = download_thumbnail_to_path(thumbnail_url, &video.id, temp_dir.path()).await {
                    info!("Thumbnail downloaded");
                    thumbnail_path = Some(path);
                    break;
                }
            }

            let exit_code = match timeout(Duration::from_secs(DOWNLOAD_TIMEOUT), merge_child.wait()).await {
                Ok(Ok(exit_code)) => exit_code,
                Ok(Err(err)) => {
                    let _guard = span.enter();
                    sender.send((index, Err(DownloadErrorKind::Ffmpeg(err))))?;
                    continue;
                }
                Err(_) => {
                    let _guard = span.enter();
                    sender.send((
                        index,
                        Err(DownloadErrorKind::Ffmpeg(io::Error::new(
                            io::ErrorKind::TimedOut,
                            "FFmpeg process timed out",
                        ))),
                    ))?;
                    continue;
                }
            };

            let _guard = span.enter();
            if !exit_code.success() {
                sender.send((
                    index,
                    Err(DownloadErrorKind::Ffmpeg(io::Error::other(format!(
                        "FFmpeg exited with status `{exit_code}`"
                    )))),
                ))?;
                continue;
            }

            info!("Video downloaded and merged");

            sender.send((index, Ok(VideoInFS::new(file_path, thumbnail_path, temp_dir))))?;
        }

        Ok(())
    }
}

#[instrument(skip_all)]
async fn range_download_to_write(
    url: impl AsRef<str>,
    filesize: f64,
    sender: UnboundedSender<Bytes>,
) -> Result<(), RangeDownloadErrorKind> {
    let client = Client::new();
    let url = url.as_ref();

    let mut start = 0;
    let mut end = RANGE_CHUNK_SIZE;

    loop {
        trace!(start, end, "Download chunk");

        #[allow(clippy::cast_possible_truncation)]
        if end >= filesize as i32 {
            let mut stream = client
                .get(url)
                .header("Range", format!("bytes={start}-"))
                .send()
                .await?
                .bytes_stream();

            while let Some(chunk_res) = stream.next().await {
                let chunk = chunk_res?;
                sender.send(chunk)?;
            }

            break;
        }

        let mut stream = client
            .get(url)
            .header("Range", format!("bytes={start}-{end}"))
            .send()
            .await?
            .bytes_stream();

        while let Some(chunk_res) = stream.next().await {
            let chunk = chunk_res?;
            sender.send(chunk)?;
        }

        start = end + 1;
        end += RANGE_CHUNK_SIZE;
    }

    Ok(())
}
