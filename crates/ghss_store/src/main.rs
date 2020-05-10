mod config;
mod db;
mod schema;

use tonic::{transport::Server, Code, Request, Response, Status};

use chrono::Utc;
use db::DB;
use ghss_tracing::{register_new_tracing_root, register_tracing_root};
use proto::store_server::{Store, StoreServer};
use proto::{ImportReply, ImportRequest, RecordHookReply, RecordHookRequest};
use std::convert::{From, TryInto};
use tracing::{info, info_span};

pub(crate) mod proto {
    tonic::include_proto!("store");
}

impl From<db::Error> for Status {
    fn from(err: db::Error) -> Self {
        match err {
            db::Error::SQLite(err) => Status::new(Code::Internal, format!("SQL error: {}", err)),
        }
    }
}

#[derive(Debug)]
struct SQLiteStore {
    database_directory: String,
}

impl SQLiteStore {
    fn open_db(&self, repository_id: String) -> db::Result<DB> {
        DB::open(format!("{}/{}.db", self.database_directory, repository_id))
    }
}

#[tonic::async_trait]
impl Store for SQLiteStore {
    async fn import(
        &self,
        request: Request<ImportRequest>,
    ) -> Result<Response<ImportReply>, Status> {
        info!("import");
        let request = request.into_inner();
        let mut db = self.open_db(request.repository_id)?;
        let trx = db.transaction()?;
        trx.upsert_builds(&request.builds)?;
        trx.upsert_commits(&request.commits)?;
        trx.insert_import(
            Utc::now().timestamp_millis(),
            (request.builds.len() + request.commits.len())
                .try_into()
                .map_err(|_| Status::new(Code::InvalidArgument, "Too many builds and commits"))?,
        )?;
        trx.commit()?;
        Ok(Response::new(ImportReply {}))
    }

    async fn record_hook(
        &self,
        request: Request<RecordHookRequest>,
    ) -> Result<Response<RecordHookReply>, Status> {
        info!("record hook");
        let request = request.into_inner();
        let mut db = self.open_db(request.repository_id)?;
        let trx = db.transaction()?;
        if let Some(hook) = request.hook {
            trx.insert_hook(&hook)?;
        } else {
            return Err(Status::new(
                Code::InvalidArgument,
                "Hook is a mandatory field",
            ));
        }
        if let Some(build) = request.build {
            trx.upsert_builds(&[build])?;
        }
        trx.commit()?;
        Ok(Response::new(RecordHookReply {}))
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
        .add_service(StoreServer::new(store))
        .serve(addr)
        .await?;

    Ok(())
}
