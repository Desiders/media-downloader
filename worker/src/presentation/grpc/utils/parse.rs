use tonic::Status;
use tracing::error;

pub fn required_field<T>(opt: Option<T>, field: &str) -> Result<T, Status> {
    opt.ok_or_else(|| {
        let msg = format!("{field} is required");
        error!("{msg}");
        Status::invalid_argument(msg)
    })
}
