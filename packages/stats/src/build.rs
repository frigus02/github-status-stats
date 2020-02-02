use chrono::{DateTime, FixedOffset};
use github_client::{CheckRun, CheckRunConclusion, CommitStatus, CommitStatusState};
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

impl From<Vec<CommitStatus>> for Build {
    fn from(statuses: Vec<CommitStatus>) -> Self {
        let mut iter = statuses.into_iter();
        let first = iter.next().unwrap();
        let first_millis = first.created_at.timestamp_millis();
        let created_at = first.created_at;
        let name = first.context.clone();
        let last = iter.last().unwrap_or(first);
        let last_millis = last.created_at.timestamp_millis();
        Self {
            name,
            source: BuildSource::Status,
            successful: last.state == CommitStatusState::Success,
            duration_ms: last_millis - first_millis,
            created_at,
            commit_sha: "".to_owned(),
        }
    }
}

impl From<CheckRun> for Build {
    fn from(check_run: CheckRun) -> Self {
        Self {
            name: check_run.name,
            source: BuildSource::CheckRun,
            successful: match check_run.conclusion {
                Some(conclusion) => conclusion == CheckRunConclusion::Success,
                None => false,
            },
            duration_ms: match check_run.completed_at {
                Some(completed_at) => {
                    completed_at.timestamp_millis() - check_run.started_at.timestamp_millis()
                }
                None => 0,
            },
            created_at: check_run.started_at,
            commit_sha: check_run.head_sha,
        }
    }
}

impl From<Build> for Point {
    fn from(build: Build) -> Self {
        let mut tags = HashMap::new();
        tags.insert("name", build.name);
        tags.insert(
            "source",
            match build.source {
                BuildSource::Status => "status",
                BuildSource::CheckRun => "check_run",
            }
            .to_string(),
        );

        let mut fields = HashMap::new();
        fields.insert("commit", FieldValue::String(build.commit_sha));
        fields.insert(
            "successful",
            FieldValue::Integer(if build.successful { 1 } else { 0 }),
        );
        fields.insert("duration_ms", FieldValue::Integer(build.duration_ms));

        Self {
            measurement: "build",
            tags,
            fields,
            timestamp: Timestamp::new(&build.created_at),
        }
    }
}
