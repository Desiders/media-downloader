mod generated {
    tonic::include_proto!("worker.api");
}
use froodi::async_impl::Container;
pub use generated::version_service_server::VersionServiceServer;
use generated::{GetCurrentVersionRequest, GetCurrentVersionResponse, version_service_server::VersionService};
use tonic::{Request, Response, Status, async_trait};

use crate::config::Version;

#[derive(Debug, Clone)]
pub struct Service;

#[async_trait]
impl VersionService for Service {
    async fn get_current_version(&self, request: Request<GetCurrentVersionRequest>) -> Result<Response<GetCurrentVersionResponse>, Status> {
        let container = request.extensions().get::<Container>().unwrap();
        let version = container.get::<Version>().await.unwrap();

        Ok(Response::new(GetCurrentVersionResponse {
            major: version.major,
            minor: version.minor,
            patch: version.patch,
        }))
    }
}

#[cfg(test)]
mod tests {
    use froodi::{DefaultScope::App, async_registry, instance, registry};

    use super::*;
    use crate::config::Version;

    #[tokio::test]
    async fn test_get_current_version() {
        let version = Version {
            major: 1,
            minor: 2,
            patch: 3,
        };
        let container = Container::new(async_registry! {
            extend(
                registry! {
                    provide(App, instance(version.clone())),
                }
            )
        });
        let service = Service {};

        let mut request = Request::new(GetCurrentVersionRequest {});
        request.extensions_mut().insert(container);

        let response = service.get_current_version(request).await.unwrap();

        assert_eq!(response.get_ref().major, version.major);
        assert_eq!(response.get_ref().minor, version.minor);
        assert_eq!(response.get_ref().patch, version.patch);
    }
}
