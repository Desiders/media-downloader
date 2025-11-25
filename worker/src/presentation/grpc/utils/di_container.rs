use froodi::async_impl::Container;
use tonic::{Request, Status};
use tracing::error;

pub fn get<T>(request: &Request<T>) -> Result<&Container, Status> {
    request
        .extensions()
        .get::<Container>()
        .ok_or_else(|| Status::internal("DI container is not available"))
        .inspect_err(|err| error!("Failed to get DI container: {err}"))
}
