use bytes::Bytes;
use futures_util::StreamExt as _;
use mpsc::{UnboundedSender, error::SendError, unbounded_channel};
use nix::{
    errno::Errno,
    fcntl::{FcntlArg::F_SETFD, FdFlag, fcntl},
    unistd::pipe,
};
use reqwest::Client;
use std::{fs::File, io, sync::Arc, time::Duration};
use tempfile::TempDir;
use tokio::{io::AsyncWriteExt as _, sync::mpsc, time::timeout};
use tracing::{Instrument as _, debug, debug_span, error, info, instrument, trace};
use url::Url;

use crate::{
    adapters::{
        ffmpeg::{download_thumbnail_to_path, merge_streams},
        ytdl::{download_to_pipe, download_video_to_path},
    },
    config,
    entities::{Cookie, MediaInFS, Video, format},
    interactors::Interactor,
    utils::format_error_report,
};

const DOWNLOAD_TIMEOUT: u64 = 360;
const RANGE_CHUNK_SIZE: i32 = 1024 * 1024 * 10;

#[derive(thiserror::Error, Debug)]
pub enum RangeErrorKind {
    #[error("Channel error: {0}")]
    Channel(#[from] SendError<Bytes>),
    #[error("Request error: {0}")]
    Reqwest(#[from] reqwest::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum ErrorKind {
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
    url: &'a Url,
    cookie: Option<&'a Cookie>,
    video: &'a Video,
    format: &'a format::Combined<'a>,
}

impl<'a> DownloadInput<'a> {
    #[inline]
    #[must_use]
    pub const fn new(url: &'a Url, cookie: Option<&'a Cookie>, video: &'a Video, format: &'a format::Combined<'a>) -> Self {
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
        let format_id = format.id();
        let host = url.host();
        let thumbnail_urls = video.thumbnail_urls(host.as_ref());
        let temp_dir = TempDir::new().map_err(Self::Err::TempDir)?;
        let file_path = temp_dir.path().join(format!("{}.{}", video.id, extension));

        if format.ids_are_equal() {
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
                            format_id,
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
            info!("Video downloaded");

            return Ok(Self::Output::new(file_path, thumbnail_path, temp_dir));
        }
        debug!("Formats are different");

        let (video_read_fd, video_write_fd) = pipe().map_err(Self::Err::Pipe)?;
        let (audio_read_fd, audio_write_fd) = pipe().map_err(Self::Err::Pipe)?;

        fcntl(&video_write_fd, F_SETFD(FdFlag::FD_CLOEXEC)).map_err(Self::Err::Pipe)?;
        fcntl(&audio_write_fd, F_SETFD(FdFlag::FD_CLOEXEC)).map_err(Self::Err::Pipe)?;

        let mut merge_child = merge_streams(&video_read_fd, &audio_read_fd, extension, &file_path, self.limits_cfg.max_file_size)
            .map_err(Self::Err::Ffmpeg)?;

        if let Some(filesize) = format.0.filesize_or_approx() {
            let (sender, mut receiver) = unbounded_channel();
            let url = format.0.url.to_owned();
            tokio::spawn(
                async move {
                    tokio::join!(
                        async move {
                            let _ = range_download_to_write(url, filesize, sender)
                                .await
                                .inspect_err(|err| error!("{}", format_error_report(&err)));
                        },
                        async move {
                            let mut writer = tokio::fs::File::from_std(File::from(video_write_fd));
                            while let Some(bytes) = receiver.recv().await {
                                if let Err(err) = writer.write(&bytes).await {
                                    match err.kind() {
                                        io::ErrorKind::BrokenPipe => break,
                                        _ => error!("{}", format_error_report(&err)),
                                    }
                                }
                            }
                        }
                    )
                }
                .instrument(debug_span!("video_range")),
            );
        } else {
            download_to_pipe(
                video_write_fd,
                self.yt_dlp_cfg.executable_path.as_ref(),
                &video.original_url,
                self.yt_pot_provider_cfg.url.as_ref(),
                format.0.id,
                self.limits_cfg.max_file_size,
                cookie,
            )
            .map_err(Self::Err::Ytdlp)?;
        }
        if let Some(filesize) = format.1.filesize_or_approx() {
            let (sender, mut receiver) = mpsc::unbounded_channel();

            let url = format.1.url.to_owned();
            tokio::spawn(
                async move {
                    tokio::join!(
                        async move {
                            let _ = range_download_to_write(url, filesize, sender)
                                .await
                                .inspect_err(|err| error!("{}", format_error_report(&err)));
                        },
                        async move {
                            let mut writer = tokio::fs::File::from_std(File::from(audio_write_fd));
                            while let Some(bytes) = receiver.recv().await {
                                if let Err(err) = writer.write(&bytes).await {
                                    match err.kind() {
                                        io::ErrorKind::BrokenPipe => break,
                                        _ => error!("{}", format_error_report(&err)),
                                    }
                                }
                            }
                        }
                    )
                }
                .instrument(debug_span!("audio_range")),
            );
        } else {
            download_to_pipe(
                audio_write_fd,
                self.yt_dlp_cfg.executable_path.as_ref(),
                &video.original_url,
                self.yt_pot_provider_cfg.url.as_ref(),
                format.1.id,
                self.limits_cfg.max_file_size,
                cookie,
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
                return Err(Self::Err::Ffmpeg(err));
            }
            Err(_) => {
                return Err(Self::Err::Ffmpeg(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "FFmpeg process timed out",
                )));
            }
        };
        if !exit_code.success() {
            return Err(Self::Err::Ffmpeg(io::Error::other(format!(
                "FFmpeg exited with status `{exit_code}`"
            ))));
        }

        info!("Video downloaded and merged");
        Ok(Self::Output::new(file_path, thumbnail_path, temp_dir))
    }
}

#[instrument(skip_all)]
async fn range_download_to_write(url: impl AsRef<str>, filesize: f64, sender: UnboundedSender<Bytes>) -> Result<(), RangeErrorKind> {
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
