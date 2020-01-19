use chrono::{DateTime, FixedOffset, Utc};
use influxdb_client::{Client, FieldValue, Point, Timestamp};
use std::collections::HashMap;

type BoxError = Box<dyn std::error::Error>;

pub enum HookType {
    Status,
    CheckRun,
}

pub struct Hook {
    pub time: DateTime<Utc>,
    pub r#type: HookType,
    pub commit_sha: String,
    pub commits_since: DateTime<Utc>,
}

impl Hook {
    pub fn to_point(self) -> Point {
        let mut tags = HashMap::new();
        tags.insert(
            "type",
            match self.r#type {
                HookType::Status => "status",
                HookType::CheckRun => "check_run",
            }
            .to_string(),
        );
        tags.insert("commit", self.commit_sha);

        let mut fields = HashMap::new();
        fields.insert(
            "commits_since",
            FieldValue::Integer(self.commits_since.timestamp()),
        );

        Point {
            measurement: "import",
            tags,
            fields,
            timestamp: Timestamp::new(&self.time),
        }
    }
}

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
