mod generated {
    tonic::include_proto!("worker.test");
}
pub use generated::echo_service_server::EchoServiceServer;
use generated::{EchoRequest, EchoResponse, echo_service_server::EchoService};
use tonic::{Request, Response, Status, async_trait};

#[derive(Debug, Clone)]
pub struct Service;

#[async_trait]
impl EchoService for Service {
    async fn echo(&self, request: Request<EchoRequest>) -> Result<Response<EchoResponse>, Status> {
        let reply = EchoResponse {
            message: request.into_inner().message,
        };
        Ok(Response::new(reply))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_echo_rpc() {
        let service = Service;
        let test_message = "Hello world!";
        let request = Request::new(EchoRequest {
            message: test_message.to_string(),
        });

        let response = service.echo(request).await.unwrap();

        assert_eq!(response.get_ref().message, test_message);
    }
}
