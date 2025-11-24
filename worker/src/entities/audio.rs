use std::path::PathBuf;
use tempfile::TempDir;

use super::{PreferredLanguages, Video, format::Audio};

#[derive(thiserror::Error, Debug)]
#[error("Audio format not found")]
pub struct AudioFormatNotFound;

pub struct AudioAndFormat<'a> {
    pub video: &'a Video,
    pub format: Audio<'a>,
}

impl<'a> AudioAndFormat<'a> {
    pub fn new_with_select_format(
        video: &'a Video,
        max_file_size: u32,
        PreferredLanguages { languages }: &PreferredLanguages,
    ) -> Result<Self, AudioFormatNotFound> {
        let mut formats = video.get_audio_formats();
        formats.sort(max_file_size, languages);

        let Some(format) = formats.first().cloned() else {
            return Err(AudioFormatNotFound);
        };

        Ok(Self { video, format })
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub struct AudioInFS {
    pub path: PathBuf,
    pub thumbnail_path: Option<PathBuf>,
    pub temp_dir: TempDir,
}

impl AudioInFS {
    pub fn new(path: impl Into<PathBuf>, thumbnail_path: Option<PathBuf>, temp_dir: TempDir) -> Self {
        Self {
            path: path.into(),
            thumbnail_path,
            temp_dir,
        }
    }
}
