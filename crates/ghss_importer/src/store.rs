use chrono::{DateTime, Utc};
use ghss_store_client::StoreClient;
use ghss_store_client::{
    Build, Commit, HookedCommitsReply, HookedCommitsRequest, ImportRequest, Response, Status,
};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub struct RepositoryImporter<'client> {
    client: &'client mut StoreClient,
    repository_id: String,
    timestamp: DateTime<Utc>,
}

impl<'client> RepositoryImporter<'client> {
    pub fn new(client: &'client mut StoreClient, repository_id: String) -> Self {
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
        let _response = self
            .client
            .import(ImportRequest {
                repository_id: self.repository_id.clone(),
                builds,
                commits,
                timestamp: self.timestamp.timestamp_millis(),
            })
            .await?;
        Ok(())
    }

    pub async fn get_hooked_commits_since_last_import(
        &mut self,
    ) -> Result<Response<HookedCommitsReply>, Status> {
        self.client
            .get_hooked_commits_since_last_import(HookedCommitsRequest {
                repository_id: self.repository_id.clone(),
                until: self.timestamp.timestamp_millis(),
            })
            .await
    }
}
