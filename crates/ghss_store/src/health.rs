use health_check_response::ServingStatus;
use health_server::Health;
pub use health_server::HealthServer;
use std::pin::Pin;
use tokio::stream::Stream;
use tonic::{Request, Response, Status};

tonic::include_proto!("grpc.health.v1");

#[derive(Debug, Default)]
pub struct HealthService {}

#[tonic::async_trait]
impl Health for HealthService {
    async fn check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        Ok(Response::new(HealthCheckResponse {
            status: ServingStatus::Serving as i32,
        }))
    }

    type WatchStream =
        Pin<Box<dyn Stream<Item = Result<HealthCheckResponse, Status>> + Send + Sync + 'static>>;

    async fn watch(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<Self::WatchStream>, Status> {
        unimplemented!()
    }
}
