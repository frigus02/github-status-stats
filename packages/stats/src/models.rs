use chrono::{DateTime, FixedOffset, Utc};
use influxdb_client::{FieldValue, Point, Timestamp};
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub enum BuildSource {
    Status,
    CheckRun,
}

#[derive(Debug)]
pub struct Build {
    pub name: String,
    pub source: BuildSource,
    pub successful: bool,
    pub duration_ms: i64,
    pub created_at: DateTime<FixedOffset>,
    pub commit_sha: String,
}

impl Build {
    pub fn into_point(self) -> Point {
        let mut tags = HashMap::new();
        tags.insert("name", self.name);
        tags.insert(
            "source",
            match self.source {
                BuildSource::Status => "status",
                BuildSource::CheckRun => "check_run",
            }
            .to_string(),
        );

        let mut fields = HashMap::new();
        fields.insert("commit", FieldValue::String(self.commit_sha));
        fields.insert("successful", FieldValue::Boolean(self.successful));
        fields.insert("duration_ms", FieldValue::Integer(self.duration_ms));

        Point {
            measurement: "build",
            tags,
            fields,
            timestamp: Timestamp::new(&self.created_at),
        }
    }
}

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

impl Hook {
    pub fn into_point(self) -> Point {
        let mut tags = HashMap::new();
        tags.insert(
            "type",
            match self.r#type {
                HookType::Status => "status",
                HookType::CheckRun => "check_run",
            }
            .to_string(),
        );

        let mut fields = HashMap::new();
        fields.insert("commit", FieldValue::String(self.commit_sha));

        Point {
            measurement: "import",
            tags,
            fields,
            timestamp: Timestamp::new(&self.time),
        }
    }
}

#[derive(Debug)]
pub struct Import {
    pub time: DateTime<Utc>,
    pub points: i64,
}

impl Import {
    pub fn into_point(self) -> Point {
        let tags = HashMap::new();

        let mut fields = HashMap::new();
        fields.insert("points", FieldValue::Integer(self.points));

        Point {
            measurement: "import",
            tags,
            fields,
            timestamp: Timestamp::new(&self.time),
        }
    }
}
