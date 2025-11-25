use serde::Deserialize;
use std::fmt::{self, Display, Formatter};

use crate::{
    utils::{calculate_aspect_ratio, thumbnail::get_urls_by_aspect},
    value_objects::AspectKind,
};

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone, Deserialize)]
pub struct Thumbnail {
    pub media_id: String,
    pub service_domain: String,
    pub thumbnails: Vec<String>,
    pub width: Option<i64>,
    pub height: Option<i64>,
}

impl Thumbnail {
    #[inline]
    #[must_use]
    pub const fn new(media_id: String, service_domain: String, thumbnails: Vec<String>, width: Option<i64>, height: Option<i64>) -> Self {
        Self {
            media_id,
            service_domain,
            thumbnails,
            width,
            height,
        }
    }

    #[must_use]
    pub fn thumbnail_urls(&self) -> Vec<String> {
        let aspect_ratio = calculate_aspect_ratio(self.width, self.height);
        let aspect_kind = AspectKind::get_nearest(aspect_ratio);

        let mut urls = get_urls_by_aspect(&self.service_domain, &self.media_id, aspect_kind);
        for url in self.thumbnails.iter() {
            urls.push(url.clone());
        }
        urls
    }
}

impl Display for Thumbnail {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{id} ({domain}) {width}x{height}",
            id = self.media_id,
            domain = self.service_domain,
            width = self.width.unwrap_or(0),
            height = self.height.unwrap_or(0),
        )
    }
}
