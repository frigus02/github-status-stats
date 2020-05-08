mod db;
mod schema;

use tonic::{transport::Server, Request, Response, Status};

use proto::store_server::{Store, StoreServer};
use proto::{InsertBuildsRequest, InsertCommitsRequest, InsertImportsRequest, InsertHooksRequest, InsertReply};

pub(crate) mod proto {
    tonic::include_proto!("store");
}

#[derive(Debug, Default)]
struct SQLiteStore {}

#[tonic::async_trait]
impl Store for SQLiteStore {
    async fn insert_builds(&self, request: Request<InsertBuildsRequest>) -> Result<Response<InsertReply>, Status> {
        let request = request.into_inner();
        let conn = db::open(format!("dbs/{}.db", request.repository_id))?;
        Ok(Response::new(InsertReply {}))
    }

    async fn insert_commits(&self, request: Request<InsertCommitsRequest>) -> Result<Response<InsertReply>, Status> {
        Ok(Response::new(InsertReply {}))
    }

    async fn insert_imports(&self, request: Request<InsertImportsRequest>) -> Result<Response<InsertReply>, Status> {
        Ok(Response::new(InsertReply {}))
    }

    async fn insert_hooks(&self, request: Request<InsertHooksRequest>) -> Result<Response<InsertReply>, Status> {
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
