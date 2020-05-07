use tonic::{transport::Server, Request, Response, Status};

use proto::store_server::{Store, StoreServer};
use proto::{Build, Commit, Import, Hook, InsertReply};

pub mod proto {
    tonic::include_proto!("store");
}

#[derive(Debug, Default)]
pub struct SQLiteStore {}

#[tonic::async_trait]
impl Store for SQLiteStore {
    async fn insert_build(&self, request: Request<Build>) -> Result<Response<InsertReply>, Status> {
        Ok(Response::new(InsertReply {}))
    }

    async fn insert_commit(&self, request: Request<Commit>) -> Result<Response<InsertReply>, Status> {
        Ok(Response::new(InsertReply {}))
    }

    async fn insert_import(&self, request: Request<Import>) -> Result<Response<InsertReply>, Status> {
        Ok(Response::new(InsertReply {}))
    }

    async fn insert_hook(&self, request: Request<Hook>) -> Result<Response<InsertReply>, Status> {
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
