use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    tonic_prost_build::configure().compile_protos(
        &[
            "../proto/worker/test/echo.proto",
            "../proto/worker/api/version.proto",
            "../proto/worker/api/v1/limits.proto",
            "../proto/worker/api/v1/download.proto",
        ],
        &["../proto"],
    )?;
    Ok(())
}
