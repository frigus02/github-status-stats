mod config;
mod db;
mod query;
mod store;

use ghss_tracing::{register_new_tracing_root, register_tracing_root};
use proto::query_server::QueryServer;
use proto::store_server::StoreServer;
use std::convert::From;
use tonic::{transport::Server, Code, Status};
use tracing::info_span;

pub(crate) mod proto {
    tonic::include_proto!("store");
    tonic::include_proto!("query");
}

impl From<db::Error> for Status {
    fn from(err: db::Error) -> Self {
        match err {
            db::Error::DBNotFound => Status::new(Code::FailedPrecondition, "DB not found"),
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
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::load();

    ghss_tracing::setup(ghss_tracing::Config {
        honeycomb_api_key: config.honeycomb_api_key.unsecure().to_owned(),
        honeycomb_dataset: config.honeycomb_dataset.clone(),
        service_name: "store",
    });

    let addr = "[::1]:50051".parse()?;
    let store = SQLiteStore {
        database_directory: config.database_directory,
    };

    Server::builder()
        .trace_fn(|headers| {
            let span = info_span!("request");
            {
                // TODO: This seems weird. Need to understand why that's
                // necessary or how to do it better.
                let _guard = span.enter();
                match (
                    headers.get(ghss_tracing::HEADER_TRACE_ID),
                    headers.get(ghss_tracing::HEADER_PARENT_SPAN_ID),
                ) {
                    (Some(trace_id), Some(parent_span_id)) => {
                        register_tracing_root(
                            trace_id.to_str().unwrap(),
                            parent_span_id.to_str().unwrap(),
                        );
                    }
                    _ => register_new_tracing_root(),
                };
            }
            span
        })
        .add_service(StoreServer::new(store.clone()))
        .add_service(QueryServer::new(store))
        .serve(addr)
        .await?;

    Ok(())
}
