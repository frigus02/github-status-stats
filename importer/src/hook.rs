use chrono::{DateTime, FixedOffset};
use influxdb_client::{Client, FieldValue};
use stats::HookType;

type BoxError = Box<dyn std::error::Error>;

pub async fn get_hook_types_since(
    client: &Client<'_>,
    since: &DateTime<FixedOffset>,
) -> Result<Vec<HookType>, BoxError> {
    Ok(client
        .query(&format!(
            "SELECT DISTINCT(type) FROM hook WHERE time >= \"{}\"",
            since.to_rfc3339()
        ))
        .await?
        .results
        .pop()
        .and_then(|result| result.series)
        .and_then(|mut series| series.pop())
        .map(|series| series.values)
        .unwrap_or_else(|| Vec::new())
        .into_iter()
        .filter_map(|mut row| row.pop())
        .filter_map(|value| match value {
            FieldValue::String(value) => match value.as_str() {
                "status" => Some(HookType::Status),
                "check_run" => Some(HookType::CheckRun),
                _ => None,
            },
            _ => None,
        })
        .collect())
}
