use chrono::{DateTime, Utc};
use ghss_store_client::{store_client::StoreClient, Channel};
use ghss_store_client::{
    Build, Commit, HookedCommitsReply, HookedCommitsRequest, ImportRequest, Response, Status,
};
use opentelemetry::api::{FutureExt, Key, TraceContextExt};

type BoxError = Box<dyn std::error::Error>;

pub struct RepositoryImporter<'client> {
    client: &'client mut StoreClient<Channel>,
    repository_id: String,
    timestamp: DateTime<Utc>,
}

impl<'client> RepositoryImporter<'client> {
    pub fn new(client: &'client mut StoreClient<Channel>, repository_id: String) -> Self {
        Self {
            client,
            repository_id,
            timestamp: Utc::now(),
        }
    }

    pub async fn import(
        &mut self,
        builds: Vec<Build>,
        commits: Vec<Commit>,
    ) -> Result<(), BoxError> {
        let request_cx = ghss_store_client::request_context("ghss.store.Store/Import");
        request_cx
            .span()
            .set_attribute(Key::new("import.builds").u64(builds.len() as u64));
        request_cx
            .span()
            .set_attribute(Key::new("import.commits").u64(commits.len() as u64));
        let request = ghss_store_client::request(
            ImportRequest {
                repository_id: self.repository_id.clone(),
                builds,
                commits,
                timestamp: self.timestamp.timestamp_millis(),
            },
            &request_cx,
        );
        let _response = self.client.import(request).with_context(request_cx).await?;
        Ok(())
    }

    pub async fn get_hooked_commits_since_last_import(
        &mut self,
    ) -> Result<Response<HookedCommitsReply>, Status> {
        let request_cx = ghss_store_client::request_context("ghss.store.Store/GetHookedCommitsSinceLastImport");
        let request = ghss_store_client::request(
            HookedCommitsRequest {
                repository_id: self.repository_id.clone(),
                until: self.timestamp.timestamp_millis(),
            },
            &request_cx,
        );
        self.client
            .get_hooked_commits_since_last_import(request)
            .with_context(request_cx)
            .await
    }
}
