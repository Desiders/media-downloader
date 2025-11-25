use crate::entities::Cookie;

use std::{
    io,
    os::fd::OwnedFd,
    path::Path,
    process::{Child, Command, Output, Stdio},
    time::Duration,
};
use tracing::{Level, event, instrument};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Download stream to a pipe.
/// This function forks a child process and executes `yt-dl` in it.
/// The child process redirects its stdout to the pipe.
/// # Errors
/// Returns [`io::Error`] if the spawn child process fails
/// # Returns
/// Returns the child process
#[instrument(skip_all)]
pub fn download_to_pipe(
    fd: OwnedFd,
    executable_path: impl AsRef<str>,
    url: impl AsRef<str>,
    pot_provider_api_url: impl AsRef<str>,
    format: impl AsRef<str>,
    max_filesize: u32,
    cookie: Option<&Cookie>,
) -> Result<Child, io::Error> {
    let max_filesize_str = max_filesize.to_string();

    let mut args = vec![
        "--js-runtimes",
        "deno:deno",
        "--ignore-config",
        "--no-colors",
        "--socket-timeout",
        "5",
        "--output",
        "-",
        "--no-playlist",
        "--no-mtime",
        "--no-write-comments",
        "--quiet",
        "--no-simulate",
        "--no-progress",
        "--no-check-formats",
        "--max-filesize",
        max_filesize_str.as_ref(),
        "-f",
        format.as_ref(),
    ];

    let extractor_arg = format!("youtubepot-bgutilhttp:base_url={}", pot_provider_api_url.as_ref());
    args.push("--extractor-args");
    args.push(&extractor_arg);

    let cookie_path = cookie.map(|c| c.path.to_string_lossy());
    if let Some(cookie_path) = cookie_path.as_deref() {
        event!(Level::TRACE, "Using cookies from: {}", cookie_path);

        args.push("--cookies");
        args.push(cookie_path);
    } else {
        event!(Level::TRACE, "No cookies provided");
    }

    args.push("--");
    args.push(url.as_ref());

    Command::new(executable_path.as_ref())
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(fd))
        .stderr(Stdio::inherit())
        .spawn()
}

#[instrument(skip_all)]
pub async fn download_video_to_path(
    executable_path: impl AsRef<str>,
    url: impl AsRef<str>,
    pot_provider_api_url: impl AsRef<str>,
    format_id: impl AsRef<str>,
    output_extension: impl AsRef<str>,
    output_dir_path: impl AsRef<Path>,
    timeout: u64,
    max_filesize: u32,
    cookie: Option<&Cookie>,
) -> Result<(), io::Error> {
    let output_dir_path = output_dir_path.as_ref().to_string_lossy();
    let max_filesize_str = max_filesize.to_string();

    let mut args = vec![
        "--js-runtimes",
        "deno:deno",
        "--no-update",
        "--ignore-config",
        "--no-colors",
        "--socket-timeout",
        "5",
        "--paths",
        output_dir_path.as_ref(),
        "--output",
        "%(id)s.%(ext)s",
        "--no-playlist",
        "--no-mtime",
        "--no-write-comments",
        "--quiet",
        "--no-simulate",
        "--no-progress",
        "--no-check-formats",
        "--embed-metadata",
        "--concurrent-fragments",
        "4",
        "--max-filesize",
        max_filesize_str.as_ref(),
        "-f",
        format_id.as_ref(),
        "--merge-output-format",
        output_extension.as_ref(),
    ];

    let extractor_arg = format!("youtubepot-bgutilhttp:base_url={}", pot_provider_api_url.as_ref());
    args.push("--extractor-args");
    args.push(&extractor_arg);

    let cookie_path = cookie.map(|c| c.path.to_string_lossy());
    if let Some(cookie_path) = cookie_path.as_deref() {
        event!(Level::TRACE, "Using cookies from: {}", cookie_path);

        args.push("--cookies");
        args.push(cookie_path);
    } else {
        event!(Level::TRACE, "No cookies provided");
    }

    args.push("--");
    args.push(url.as_ref());

    let child = tokio::process::Command::new(executable_path.as_ref())
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    match tokio::time::timeout(Duration::from_secs(timeout), child.wait_with_output()).await {
        Ok(Ok(Output { status, stderr, .. })) => {
            if status.success() {
                Ok(())
            } else {
                match status.code() {
                    Some(code) => Err(io::Error::other(format!(
                        "Youtube-dl exited with code {code} and message: {}",
                        String::from_utf8_lossy(&stderr),
                    ))),
                    None => Err(io::Error::other(format!(
                        "Youtube-dl exited with and message: {}",
                        String::from_utf8_lossy(&stderr),
                    ))),
                }
            }
        }
        Ok(Err(err)) => Err(err),
        Err(_) => Err(io::Error::new(io::ErrorKind::TimedOut, "Youtube-dl timed out")),
    }
}

#[instrument(skip_all)]
pub async fn download_audio_to_path(
    executable_path: impl AsRef<str>,
    url: impl AsRef<str>,
    pot_provider_api_url: impl AsRef<str>,
    format_id: impl AsRef<str>,
    output_extension: impl AsRef<str>,
    output_dir_path: impl AsRef<Path>,
    timeout: u64,
    max_filesize: u32,
    cookie: Option<&Cookie>,
) -> Result<(), io::Error> {
    let output_dir_path = output_dir_path.as_ref().to_string_lossy();
    let max_filesize_str = max_filesize.to_string();

    let mut args = vec![
        "--js-runtimes",
        "deno:deno",
        "--no-update",
        "--ignore-config",
        "--no-color",
        "--socket-timeout",
        "5",
        "--paths",
        output_dir_path.as_ref(),
        "--output",
        "%(id)s.%(ext)s",
        "--extract-audio",
        "--audio-format",
        output_extension.as_ref(),
        "--no-playlist",
        "--no-mtime",
        "--no-write-comments",
        "--quiet",
        "--no-simulate",
        "--no-progress",
        "--no-check-formats",
        "--embed-metadata",
        "--concurrent-fragments",
        "4",
        "--max-filesize",
        max_filesize_str.as_ref(),
        "-f",
        format_id.as_ref(),
    ];

    let extractor_arg = format!("youtubepot-bgutilhttp:base_url={}", pot_provider_api_url.as_ref());
    args.push("--extractor-args");
    args.push(&extractor_arg);
    args.push("--extractor-args");
    args.push("youtube:player_client=default,mweb,web_music,web_creator;player_skip=configs,initial_data");

    let cookie_path = cookie.map(|c| c.path.to_string_lossy());
    if let Some(cookie_path) = cookie_path.as_deref() {
        event!(Level::TRACE, "Using cookies from: {}", cookie_path);

        args.push("--cookies");
        args.push(cookie_path);
    } else {
        event!(Level::TRACE, "No cookies provided");
    }

    args.push("--");
    args.push(url.as_ref());

    let child = tokio::process::Command::new(executable_path.as_ref())
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    match tokio::time::timeout(Duration::from_secs(timeout), child.wait_with_output()).await {
        Ok(Ok(Output { status, stderr, .. })) => {
            if status.success() {
                Ok(())
            } else {
                match status.code() {
                    Some(code) => Err(io::Error::other(format!(
                        "Youtube-dl exited with code {code} and message: {}",
                        String::from_utf8_lossy(&stderr),
                    ))),
                    None => Err(io::Error::other(format!(
                        "Youtube-dl exited with and message: {}",
                        String::from_utf8_lossy(&stderr),
                    ))),
                }
            }
        }
        Ok(Err(err)) => Err(err),
        Err(_) => Err(io::Error::new(io::ErrorKind::TimedOut, "Youtube-dl timed out")),
    }
}
