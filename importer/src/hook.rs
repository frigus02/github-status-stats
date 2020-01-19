use chrono::{DateTime, FixedOffset};
use influxdb_client::{Client, FieldValue};

type BoxError = Box<dyn std::error::Error>;

pub async fn get_status_hook_commits_since(
    client: &Client<'_>,
    since: &DateTime<FixedOffset>,
) -> Result<Vec<String>, BoxError> {
    Ok(client
        .query(&format!(
            "SELECT DISTINCT(commit) FROM hook WHERE type = \"status\" AND time >= \"{}\"",
            since.to_rfc3339()
        ))
        .await?
        .results
        .pop()
        .and_then(|result| result.series)
        .and_then(|mut series| series.pop())
        .map(|series| series.values)
        .unwrap_or_else(Vec::new)
        .into_iter()
        .filter_map(|mut row| row.pop())
        .filter_map(|value| match value {
            FieldValue::String(value) => Some(value),
            _ => None,
        })
        .collect())
}
