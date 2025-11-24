#[allow(clippy::cast_precision_loss)]
pub const fn calculate_aspect_ratio(width: Option<i64>, height: Option<i64>) -> f64 {
    match (width, height) {
        (Some(width), Some(height)) if height > 0 => width as f64 / height as f64,
        _ => 0.0,
    }
}
