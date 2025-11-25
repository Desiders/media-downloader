use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone)]
pub struct Video<'a> {
    pub id: &'a str,
    pub url: &'a str,
    pub filesize: Option<f64>,
    pub filesize_approx: Option<f64>,
    pub container: &'a str,
}

impl Video<'_> {
    #[inline]
    #[must_use]
    pub fn filesize_or_approx(&self) -> Option<f64> {
        self.filesize.or(self.filesize_approx)
    }

    #[inline]
    #[must_use]
    pub const fn extension(&self) -> &str {
        self.container
    }
}

impl Display for Video<'_> {
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
pub struct Audio<'a> {
    pub id: &'a str,
    pub url: &'a str,
    pub filesize: Option<f64>,
    pub filesize_approx: Option<f64>,
    pub codec: &'a str,
}

impl Audio<'_> {
    #[inline]
    #[must_use]
    pub fn filesize_or_approx(&self) -> Option<f64> {
        self.filesize.or(self.filesize_approx)
    }

    #[inline]
    #[must_use]
    pub const fn extension(&self) -> &str {
        self.codec
    }
}

impl Display for Audio<'_> {
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

pub struct Combined<'a>(pub Video<'a>, pub Audio<'a>);

impl Combined<'_> {
    #[inline]
    #[must_use]
    pub const fn id(&self) -> &str {
        self.0.id
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

impl Display for Combined<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "video: {}, audio: {}", self.0, self.1)
    }
}
