mod config;
mod ctrlc;
mod db;
mod health;
mod query;
mod store;

use health::{HealthServer, HealthService};
use opentelemetry::api::{Provider, TraceContextPropagator};
use proto::query_server::QueryServer;
use proto::store_server::StoreServer;
use std::convert::From;
use tonic::{transport::Server, Code, Status};
use tracing::info_span;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::layer::SubscriberExt;

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

fn init_tracer(agent_endpoint: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let provider = match agent_endpoint {
        Some(agent_endpoint) => {
            let exporter = opentelemetry_jaeger::Exporter::builder()
                .with_agent_endpoint(agent_endpoint.parse().unwrap())
                .with_process(opentelemetry_jaeger::Process {
                    service_name: "store".to_string(),
                    tags: vec![],
                })
                .init()?;
            let batch = opentelemetry::sdk::BatchSpanProcessor::builder(
                exporter,
                tokio::spawn,
                tokio::time::interval,
            )
            .build();
            opentelemetry::sdk::Provider::builder()
                .with_batch_exporter(batch)
                .build()
        }
        None => {
            let exporter = opentelemetry::exporter::trace::stdout::Builder::default().init();
            opentelemetry::sdk::Provider::builder()
                .with_simple_exporter(exporter)
                .build()
        }
    };

    let tracer = provider.get_tracer("store");
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    let subscriber = tracing_subscriber::Registry::default().with(telemetry);
    tracing::subscriber::set_global_default(subscriber)?;

    let propagator = TraceContextPropagator::new();
    opentelemetry::global::set_http_text_propagator(propagator);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::load();

    init_tracer(config.otel_agent_endpoint.as_deref())?;

    let health_service = HealthService::default();
    let store = SQLiteStore {
        database_directory: config.database_directory,
    };

    Server::builder()
        .trace_fn(|headers| {
            let cx = opentelemetry::global::get_http_text_propagator(|propagator| {
                propagator.extract(headers)
            });
            let span = info_span!("request");
            span.set_parent(&cx);
            span
        })
        .add_service(HealthServer::new(health_service))
        .add_service(StoreServer::new(store.clone()))
        .add_service(QueryServer::new(store))
        .serve_with_shutdown(([0, 0, 0, 0], 50051).into(), async {
            ctrlc::ctrl_c().await;
        })
        .await?;

    Ok(())
}
