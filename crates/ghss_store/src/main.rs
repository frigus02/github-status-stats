mod db;
mod schema;

use tonic::{transport::Server, Code, Request, Response, Status};

use chrono::Utc;
use db::DB;
use proto::store_server::{Store, StoreServer};
use proto::{ImportReply, ImportRequest, RecordHookReply, RecordHookRequest};
use std::convert::{From, TryInto};

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

#[derive(Debug, Default)]
struct SQLiteStore {}

#[tonic::async_trait]
impl Store for SQLiteStore {
    async fn import(
        &self,
        request: Request<ImportRequest>,
    ) -> Result<Response<ImportReply>, Status> {
        let request = request.into_inner();
        let mut db = DB::open(format!("dbs/{}.db", request.repository_id))?;
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
        let request = request.into_inner();
        let mut db = DB::open(format!("dbs/{}.db", request.repository_id))?;
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
    let addr = "[::1]:50051".parse()?;
    let store = SQLiteStore::default();

    Server::builder()
        .add_service(StoreServer::new(store))
        .serve(addr)
        .await?;

    Ok(())
}
