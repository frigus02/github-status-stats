mod db;
mod schema;

use tonic::{transport::Server, Request, Response, Status};

use db::DB;
use proto::store_server::{Store, StoreServer};
use proto::{
    InsertBuildsRequest, InsertCommitsRequest, InsertHooksRequest, InsertImportsRequest,
    InsertReply,
};
use std::convert::From;

pub(crate) mod proto {
    tonic::include_proto!("store");
}

impl From<db::Error> for Status {
    fn from(err: db::Error) -> Self {
        match err {
            db::Error::SQLite(err) => {
                Status::new(tonic::Code::Internal, format!("SQL error: {}", err))
            }
        }
    }
}

#[derive(Debug, Default)]
struct SQLiteStore {}

#[tonic::async_trait]
impl Store for SQLiteStore {
    async fn insert_builds(
        &self,
        request: Request<InsertBuildsRequest>,
    ) -> Result<Response<InsertReply>, Status> {
        let request = request.into_inner();
        let db = DB::open(format!("dbs/{}.db", request.repository_id))?;
        db.insert_builds(&request.builds)?;
        Ok(Response::new(InsertReply {}))
    }

    async fn insert_commits(
        &self,
        request: Request<InsertCommitsRequest>,
    ) -> Result<Response<InsertReply>, Status> {
        let request = request.into_inner();
        let db = DB::open(format!("dbs/{}.db", request.repository_id))?;
        db.insert_commits(&request.commits)?;
        Ok(Response::new(InsertReply {}))
    }

    async fn insert_imports(
        &self,
        request: Request<InsertImportsRequest>,
    ) -> Result<Response<InsertReply>, Status> {
        let request = request.into_inner();
        let db = DB::open(format!("dbs/{}.db", request.repository_id))?;
        db.insert_imports(&request.imports)?;
        Ok(Response::new(InsertReply {}))
    }

    async fn insert_hooks(
        &self,
        request: Request<InsertHooksRequest>,
    ) -> Result<Response<InsertReply>, Status> {
        let request = request.into_inner();
        let db = DB::open(format!("dbs/{}.db", request.repository_id))?;
        db.insert_hooks(&request.hooks)?;
        Ok(Response::new(InsertReply {}))
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
