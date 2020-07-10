use crate::proto::{
    store_server::Store, HookedCommitsReply, HookedCommitsRequest, ImportReply, ImportRequest,
    RecordHookReply, RecordHookRequest,
};
use crate::SQLiteStore;
use tonic::{Code, Request, Response, Status};

#[tonic::async_trait]
impl Store for SQLiteStore {
    async fn import(
        &self,
        request: Request<ImportRequest>,
    ) -> Result<Response<ImportReply>, Status> {
        let request = request.into_inner();
        let mut db = self.db_write(request.repository_id)?;
        let trx = db.transaction()?;
        trx.upsert_builds(&request.builds)?;
        trx.upsert_commits(&request.commits)?;
        trx.insert_import(request.timestamp)?;
        trx.commit()?;
        Ok(Response::new(ImportReply {}))
    }

    async fn record_hook(
        &self,
        request: Request<RecordHookRequest>,
    ) -> Result<Response<RecordHookReply>, Status> {
        let request = request.into_inner();
        let mut db = self.db_write(request.repository_id)?;
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

    async fn get_hooked_commits_since_last_import(
        &self,
        request: Request<HookedCommitsRequest>,
    ) -> Result<Response<HookedCommitsReply>, Status> {
        let request = request.into_inner();
        let db = self.db_read(request.repository_id)?;
        let commits = db.get_hooked_commits_since_last_import(request.until)?;
        Ok(Response::new(HookedCommitsReply { commits }))
    }
}
