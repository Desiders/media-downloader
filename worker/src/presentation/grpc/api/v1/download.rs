mod generated {
    tonic::include_proto!("worker.api.v1");
}
pub use generated::download_service_server::DownloadServiceServer;
use generated::{
    AudioFormat, DownloadAudioRequest, DownloadAudioResponse, DownloadThumbnailRequest, DownloadThumbnailResponse, DownloadVideoRequest,
    DownloadVideoResponse, FileHeader, Video, VideoFormat, download_audio_response, download_service_server::DownloadService,
    download_thumbnail_response, download_video_response,
};
use tokio::io::AsyncReadExt as _;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, async_trait};
use tracing::error;

use crate::{
    entities::{self, MediaInFS, Thumbnail, format::Combined},
    impl_from_format,
    interactors::{
        Interactor as _,
        download::{audio, thumbnail, video},
    },
    presentation::grpc::{
        api::v1::download::generated::FileChunk,
        utils::{di_container, parse::required_field},
    },
};

const CHUNK_SIZE_BYTES: u64 = 64 * 1024;
const CHANNEL_BUFFER_SIZE: usize = 512;

trait StreamResponse: Send + 'static {
    type Message;

    fn with_header(filesize: u64) -> Self;
    fn with_chunk(content: Vec<u8>) -> Self;
}

async fn create_file_stream<R>(MediaInFS { path, temp_dir }: MediaInFS) -> Result<ReceiverStream<Result<R, Status>>, Status>
where
    R: StreamResponse,
{
    let mut file = tokio::fs::File::open(path)
        .await
        .inspect_err(|err| error!("Failed to open file: {err}"))
        .map_err(|err| Status::internal(format!("Failed to open downloaded file: {err}")))?;
    let (tx, rx) = tokio::sync::mpsc::channel(CHANNEL_BUFFER_SIZE);

    let metadata = file
        .metadata()
        .await
        .inspect_err(|err| error!("Failed to get file metadata: {err}"))
        .map_err(|err| Status::internal(format!("Failed to get file metadata: {err}")))?;

    tx.send(Ok(R::with_header(metadata.len()))).await.unwrap();

    tokio::spawn(async move {
        let mut buf = vec![0u8; CHUNK_SIZE_BYTES as usize];
        loop {
            let n = match file.read(&mut buf).await {
                Ok(0) => break,
                Ok(val) => val,
                Err(err) => {
                    error!("Failed to read chunk: {err}");
                    let _ = tx.send(Err(Status::internal(format!("Failed to read chunk: {err}")))).await;
                    break;
                }
            };
            let mut chunk = Vec::with_capacity(n);
            chunk.extend_from_slice(&buf[..n]);

            if tx.send(Ok(R::with_chunk(chunk))).await.is_err() {
                error!("Client disconnected during transfer");
                break;
            }
        }
        drop(temp_dir);
    });

    Ok(ReceiverStream::new(rx))
}

#[derive(Debug, Clone)]
pub struct Service;

#[async_trait]
impl DownloadService for Service {
    type DownloadAudioStream = ReceiverStream<Result<DownloadAudioResponse, Status>>;
    type DownloadVideoStream = ReceiverStream<Result<DownloadVideoResponse, Status>>;
    type DownloadThumbnailStream = ReceiverStream<Result<DownloadThumbnailResponse, Status>>;

    async fn download_audio(&self, request: Request<DownloadAudioRequest>) -> Result<Response<Self::DownloadAudioStream>, Status> {
        let container = di_container::get(&request)?;
        let interactor = container
            .get::<audio::Download>()
            .await
            .inspect_err(|err| error!("Failed to get interactor: {err}"))
            .map_err(|err| Status::internal(err.to_string()))?;
        let request = request.into_inner();

        let video = required_field(request.video, "Video")?.into();
        let format = required_field(request.format, "Format")?.into();

        let media = interactor
            .execute(audio::DownloadInput::new(video, format, None))
            .await
            .inspect_err(|err| error!("Failed to download audio: {err}"))
            .map_err(|err| Status::internal(format!("Failed to download audio: {err}")))?;

        create_file_stream(media).await.map(Response::new)
    }

    async fn download_video(&self, request: Request<DownloadVideoRequest>) -> Result<Response<Self::DownloadVideoStream>, Status> {
        let container = di_container::get(&request)?;
        let interactor = container
            .get::<video::Download>()
            .await
            .inspect_err(|err| error!("Failed to get interactor: {err}"))
            .map_err(|err| Status::internal(err.to_string()))?;
        let request = request.into_inner();

        let video = required_field(request.video, "Video")?.into();
        let format = {
            let format = required_field(request.format, "Format")?;
            let video = required_field(format.video, "Video format")?.into();
            let audio = required_field(format.audio, "Audio format")?.into();
            Combined(video, audio)
        };

        let media = interactor
            .execute(video::DownloadInput::new(video, format, None))
            .await
            .inspect_err(|err| error!("Failed to download video: {err}"))
            .map_err(|err| Status::internal(format!("Failed to download video: {err}")))?;

        create_file_stream(media).await.map(Response::new)
    }

    async fn download_thumbnail(
        &self,
        request: Request<DownloadThumbnailRequest>,
    ) -> Result<Response<Self::DownloadThumbnailStream>, Status> {
        let container = di_container::get(&request)?;
        let interactor = container
            .get::<thumbnail::Download>()
            .await
            .inspect_err(|err| error!("Failed to get interactor: {err}"))
            .map_err(|err| Status::internal(err.to_string()))?;
        let request = request.into_inner();

        let media = interactor
            .execute(thumbnail::DownloadInput::new(Thumbnail::new(
                request.media_id,
                request.service_domain,
                request.thumbnails,
                request.width,
                request.height,
            )))
            .await
            .inspect_err(|err| error!("Failed to download thumbnail: {err}"))
            .map_err(|err| Status::internal(format!("Failed to download thumbnail: {err}")))?
            .ok_or_else(|| Status::not_found("Available thumbnail is not found"))?;

        create_file_stream(media).await.map(Response::new)
    }
}

macro_rules! impl_stream_response {
    ($response_type:ty, $message_module:path) => {
        impl StreamResponse for $response_type {
            type Message = $message_module;

            fn with_header(filesize: u64) -> Self {
                use $message_module as Message;
                Self {
                    message: Some(Message::Header(FileHeader { filesize })),
                }
            }

            fn with_chunk(content: Vec<u8>) -> Self {
                use $message_module as Message;
                Self {
                    message: Some(Message::Chunk(FileChunk { content })),
                }
            }
        }
    };
}

impl_stream_response!(DownloadAudioResponse, download_audio_response::Message);
impl_stream_response!(DownloadVideoResponse, download_video_response::Message);
impl_stream_response!(DownloadThumbnailResponse, download_thumbnail_response::Message);

impl_from_format!(VideoFormat => entities::format::Video {
    id, url, filesize, filesize_approx, container
});

impl_from_format!(AudioFormat => entities::format::Audio {
    id, url, filesize, filesize_approx, codec
});

impl_from_format!(Video => entities::Video {
    id, url, width, height
});
