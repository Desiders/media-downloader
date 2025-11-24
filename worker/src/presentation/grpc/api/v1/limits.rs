mod generated {
    tonic::include_proto!("worker.api.v1");
}
use froodi::async_impl::Container;
pub use generated::limits_service_server::LimitsServiceServer;
use generated::{GetCurrentLimitsRequest, GetCurrentLimitsResponse, limits_service_server::LimitsService};
use tonic::{Request, Response, Status, async_trait};

use crate::config::Limits;

#[derive(Debug, Clone)]
pub struct Service {}

#[async_trait]
impl LimitsService for Service {
    async fn get_current_limits(&self, request: Request<GetCurrentLimitsRequest>) -> Result<Response<GetCurrentLimitsResponse>, Status> {
        let container = request.extensions().get::<Container>().unwrap();
        let limits = container.get::<Limits>().await.unwrap();

        Ok(Response::new(GetCurrentLimitsResponse {
            max_file_size: limits.max_file_size,
        }))
    }
}

#[cfg(test)]
mod tests {
    use froodi::{DefaultScope::App, async_registry, instance, registry};

    use super::*;

    #[tokio::test]
    async fn test_get_current_limits() {
        let limits = Limits { max_file_size: 1024 };
        let container = Container::new(async_registry! {
            extend(
                registry! {
                    provide(App,instance(limits.clone())),
                }
            )
        });
        let service = Service {};

        let mut request = Request::new(GetCurrentLimitsRequest {});
        request.extensions_mut().insert(container);

        let response = service.get_current_limits(request).await.unwrap();

        assert_eq!(response.get_ref().max_file_size, limits.max_file_size);
    }
}
