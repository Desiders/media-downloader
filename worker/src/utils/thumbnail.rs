use std::fmt::Display;

use crate::value_objects::AspectKind;

pub fn get_urls_by_aspect(service_domain: &str, id: impl Display, aspect_kind: AspectKind) -> Vec<String> {
    if service_domain.contains("youtube") || service_domain == "youtu.be" {
        let fragments = match aspect_kind {
            AspectKind::Vertical => vec!["oardefault"],
            AspectKind::Sd => vec!["sddefault", "0", "hqdefault"],
            AspectKind::Hd => vec!["maxresdefault", "hq720", "maxres2"],
            AspectKind::Other => vec![],
        };

        return fragments
            .into_iter()
            .chain(Some("frame0"))
            .map(|fragment| format!("https://i.ytimg.com/vi/{id}/{fragment}.jpg"))
            .collect();
    }

    vec![]
}
