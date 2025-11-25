use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone)]
pub struct Video {
    pub id: String,
    pub url: String,
    pub filesize: Option<f64>,
    pub filesize_approx: Option<f64>,
    pub container: String,
}

impl Video {
    #[inline]
    #[must_use]
    pub fn filesize_or_approx(&self) -> Option<f64> {
        self.filesize.or(self.filesize_approx)
    }

    #[inline]
    #[must_use]
    pub const fn extension(&self) -> &str {
        self.container.as_str()
    }
}

impl Display for Video {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{id} {container} {filesize_kb:.2}KiB ({filesize_mb:.2}MiB)",
            id = self.id,
            container = self.container,
            filesize_kb = self.filesize_or_approx().map_or(0.0, |filesize| filesize / 1024.0),
            filesize_mb = self.filesize_or_approx().map_or(0.0, |filesize| filesize / 1024.0 / 1024.0),
        )
    }
}

#[derive(Debug, Clone)]
pub struct Audio {
    pub id: String,
    pub url: String,
    pub filesize: Option<f64>,
    pub filesize_approx: Option<f64>,
    pub codec: String,
}

impl Audio {
    #[inline]
    #[must_use]
    pub fn filesize_or_approx(&self) -> Option<f64> {
        self.filesize.or(self.filesize_approx)
    }

    #[inline]
    #[must_use]
    pub const fn extension(&self) -> &str {
        self.codec.as_str()
    }
}

impl Display for Audio {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{id} {codec} {filesize_kb:.2}KiB ({filesize_mb:.2}MiB)",
            id = self.id,
            codec = self.codec,
            filesize_kb = self.filesize_or_approx().map_or(0.0, |filesize| filesize / 1024.0),
            filesize_mb = self.filesize_or_approx().map_or(0.0, |filesize| filesize / 1024.0 / 1024.0),
        )
    }
}

pub struct Combined(pub Video, pub Audio);

impl Combined {
    #[inline]
    #[must_use]
    pub const fn id(&self) -> &str {
        self.0.id.as_str()
    }

    #[inline]
    #[must_use]
    pub const fn extension(&self) -> &str {
        self.0.extension()
    }

    #[inline]
    #[must_use]
    pub fn ids_are_equal(&self) -> bool {
        self.0.id == self.1.id
    }
}

impl Display for Combined {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "video: {}, audio: {}", self.0, self.1)
    }
}
