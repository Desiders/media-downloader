const ASPECT_VERTICAL: f64 = 9.0 / 16.0;
const ASPECT_SD: f64 = 4.0 / 3.0;
const ASPECT_HD: f64 = 16.0 / 9.0;

#[derive(Debug, Clone, Copy)]
pub enum AspectKind {
    Vertical,
    Sd,
    Hd,
    Other,
}

impl AspectKind {
    #[inline]
    #[must_use]
    pub const fn get_nearest(aspect_ratio: f64) -> Self {
        if aspect_is_equal(aspect_ratio, ASPECT_VERTICAL) {
            return Self::Vertical;
        }
        if aspect_is_equal(aspect_ratio, ASPECT_SD) {
            return Self::Sd;
        }
        if aspect_is_equal(aspect_ratio, ASPECT_HD) {
            return Self::Hd;
        }
        Self::Other
    }
}

#[inline]
#[must_use]
const fn aspect_is_equal(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.01
}
