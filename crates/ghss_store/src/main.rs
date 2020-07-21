mod config;
mod ctrlc;
mod db;
mod health;
mod query;
mod store;
mod telemetry_service;

use ghss_tracing::init_tracer;
use health::{HealthServer, HealthService};
use proto::query_server::QueryServer;
use proto::store_server::StoreServer;
use std::convert::From;
use telemetry_service::TelemetryServiceExt;
use tonic::{transport::Server, Code, Status};

pub(crate) mod proto {
    tonic::include_proto!("ghss.store");
}

impl From<db::Error> for Status {
    fn from(err: db::Error) -> Self {
        match err {
            db::Error::DBNotFound => Status::new(Code::FailedPrecondition, "DB not found"),
            db::Error::InvalidIdentifier(_)
            | db::Error::InvalidTimeRange
            | db::Error::EmptyColumns => Status::new(Code::InvalidArgument, format!("{:?}", err)),
            db::Error::SQLite(err) => Status::new(Code::Internal, format!("SQL error: {}", err)),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SQLiteStore {
    pub database_directory: String,
}

impl SQLiteStore {
    fn db_write(&self, repository_id: String) -> db::Result<db::write::DB> {
        db::write::DB::open(&self.database_directory, &repository_id)
    }

    fn db_read(&self, repository_id: String) -> db::Result<db::read::DB> {
        db::read::DB::open(&self.database_directory, &repository_id)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = config::load();

    init_tracer("store", config.otel_agent_endpoint.as_deref())?;

    let health_service = HealthService::default();
    let store = SQLiteStore {
        database_directory: config.database_directory,
    };

    Server::builder()
        .add_service(HealthServer::new(health_service).with_telemetry())
        .add_service(StoreServer::new(store.clone()).with_telemetry())
        .add_service(QueryServer::new(store).with_telemetry())
        .serve_with_shutdown(([0, 0, 0, 0], 50051).into(), async {
            ctrlc::ctrl_c().await;
        })
        .await?;

    Ok(())
}
