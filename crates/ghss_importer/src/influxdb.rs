use chrono::{DateTime, FixedOffset};
use ghss_store_client::{store_client::StoreClient, Channel};
use ghss_store_client::{Build, Commit, ImportRequest};
use serde::Deserialize;
use tracing::info;

type BoxError = Box<dyn std::error::Error>;

pub async fn import(
    client: &StoreClient<Channel>,
    repository_id: String,
    builds: Vec<Build>,
    commits: Vec<Commit>,
) -> Result<(), BoxError> {
    info!(points_count = builds.len() + commits.len(), "write points");
    let request = ghss_tracing::tonic::request(ImportRequest {
        repository_id,
        builds,
        commits,
    });
    let _response = client.import(request).await?;
    Ok(())
}

#[derive(Deserialize)]
struct ImportRow {
    time: DateTime<FixedOffset>,
}

pub async fn get_last_import(
    client: &StoreClient<Channel>,
) -> Result<Option<DateTime<FixedOffset>>, BoxError> {
    Ok(client
        .query("SELECT * FROM import ORDER BY time DESC LIMIT 1")
        .await?
        .into_single_result()?
        .into_single_series()?
        .and_then(|series| series.into_rows::<ImportRow>().next())
        .transpose()
        .map(|row| row.map(|row| row.time))?)
}

#[derive(Deserialize)]
struct HookRow {
    commit: String,
}

pub async fn get_commits_since_from_hooks(
    client: &StoreClient<Channel>,
    since: &DateTime<FixedOffset>,
) -> Result<Vec<String>, BoxError> {
    Ok(client
        .query(&format!(
            "SELECT DISTINCT(commit) AS commit FROM hook WHERE time >= '{}'",
            since.to_rfc3339()
        ))
        .await?
        .into_single_result()?
        .into_single_series()?
        .map_or_else(
            || Ok(Vec::new()),
            |series| {
                series
                    .into_rows::<HookRow>()
                    .map(|row| row.map(|row| row.commit))
                    .collect()
            },
        )?)
}
