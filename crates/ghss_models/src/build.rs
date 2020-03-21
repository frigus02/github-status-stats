use chrono::{DateTime, FixedOffset};
use ghss_github::{CheckRun, CheckRunConclusion, CommitStatus, CommitStatusState};
use ghss_influxdb::{FieldValue, Point, Timestamp};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum BuildSource {
    Status,
    CheckRun,
}

impl BuildSource {
    pub(crate) fn to_tag_value(&self) -> String {
        match self {
            BuildSource::Status => "status",
            BuildSource::CheckRun => "check_run",
        }
        .to_owned()
    }
}

#[derive(Debug)]
pub struct Build {
    pub name: String,
    pub source: BuildSource,
    pub successful: bool,
    pub failed: bool,
    pub duration_ms: i64,
    pub created_at: DateTime<FixedOffset>,
    pub commit_sha: String,
}

impl From<(String, Vec<CommitStatus>)> for Build {
    fn from(params: (String, Vec<CommitStatus>)) -> Self {
        let (commit_sha, statuses) = params;
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
            failed: last.state == CommitStatusState::Error
                || last.state == CommitStatusState::Failure,
            duration_ms: last_millis - first_millis,
            created_at,
            commit_sha,
        }
    }
}

impl From<CheckRun> for Build {
    fn from(check_run: CheckRun) -> Self {
        Self {
            name: check_run.name,
            source: BuildSource::CheckRun,
            successful: match &check_run.conclusion {
                Some(conclusion) => conclusion == &CheckRunConclusion::Success,
                None => false,
            },
            failed: match &check_run.conclusion {
                Some(conclusion) => {
                    conclusion == &CheckRunConclusion::Failure
                        || conclusion == &CheckRunConclusion::TimedOut
                }
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
        tags.insert("source", build.source.to_tag_value());

        let mut fields = HashMap::new();
        fields.insert("commit", FieldValue::String(build.commit_sha));
        fields.insert(
            "successful",
            FieldValue::Integer(if build.successful { 1 } else { 0 }),
        );
        fields.insert(
            "failed",
            FieldValue::Integer(if build.failed { 1 } else { 0 }),
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
