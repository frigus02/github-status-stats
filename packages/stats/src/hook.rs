use chrono::{DateTime, FixedOffset};
use influxdb_client::{FieldValue, Point, Timestamp};
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub enum HookType {
    Status,
    CheckRun,
}

#[derive(Debug)]
pub struct Hook {
    pub time: DateTime<FixedOffset>,
    pub r#type: HookType,
    pub commit_sha: String,
}

impl From<Hook> for Point {
    fn from(hook: Hook) -> Self {
        let mut tags = HashMap::new();
        tags.insert(
            "type",
            match hook.r#type {
                HookType::Status => "status",
                HookType::CheckRun => "check_run",
            }
            .to_string(),
        );

        let mut fields = HashMap::new();
        fields.insert("commit", FieldValue::String(hook.commit_sha));

        Self {
            measurement: "import",
            tags,
            fields,
            timestamp: Timestamp::new(&hook.time),
        }
    }
}
