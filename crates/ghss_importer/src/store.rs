use chrono::{DateTime, Utc};
use ghss_store_client::{store_client::StoreClient, Channel};
use ghss_store_client::{
    Build, Commit, HookedCommitsReply, HookedCommitsRequest, ImportRequest, Response, Status,
};
use tracing::info;

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
        info!(points_count = builds.len() + commits.len(), "write points");
        let request = ghss_tracing::tonic::request(ImportRequest {
            repository_id: self.repository_id.clone(),
            builds,
            commits,
            timestamp: self.timestamp.timestamp_millis(),
        });
        let _response = self.client.import(request).await?;
        Ok(())
    }

    pub async fn get_hooked_commits_since_last_import(
        &mut self,
    ) -> Result<Response<HookedCommitsReply>, Status> {
        self.client
            .get_hooked_commits_since_last_import(ghss_tracing::tonic::request(
                HookedCommitsRequest {
                    repository_id: self.repository_id.clone(),
                    until: self.timestamp.timestamp_millis(),
                },
            ))
            .await
    }
}
