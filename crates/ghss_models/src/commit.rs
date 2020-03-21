use super::{Build, BuildSource};
use chrono::{DateTime, FixedOffset};
use ghss_influxdb::{FieldValue, Point, Timestamp};
use std::collections::HashMap;
use std::convert::TryInto;

#[derive(Debug)]
pub struct Commit {
    pub sha: String,
    pub build_name: String,
    pub build_source: BuildSource,
    pub builds: i64,
    pub builds_successful: i64,
    pub builds_failed: i64,
    pub committed_at: DateTime<FixedOffset>,
}

impl From<(DateTime<FixedOffset>, Vec<&Build>)> for Commit {
    fn from(params: (DateTime<FixedOffset>, Vec<&Build>)) -> Self {
        let (committed_at, builds) = params;
        let builds_len = builds.len().try_into().expect("convert build count");
        let builds_successful = builds
            .iter()
            .filter(|build| build.successful)
            .count()
            .try_into()
            .expect("convert successful build count");
        let builds_failed = builds
            .iter()
            .filter(|build| build.failed)
            .count()
            .try_into()
            .expect("convert failed build count");
        let first = builds.first().expect("first build");
        Self {
            sha: first.commit_sha.clone(),
            build_name: first.name.clone(),
            build_source: first.source.clone(),
            builds: builds_len,
            builds_successful,
            builds_failed,
            committed_at,
        }
    }
}

impl From<Commit> for Point {
    fn from(commit: Commit) -> Self {
        let mut tags = HashMap::new();
        tags.insert("build_name", commit.build_name);
        tags.insert("build_source", commit.build_source.to_tag_value());

        let mut fields = HashMap::new();
        fields.insert("commit", FieldValue::String(commit.sha));
        fields.insert("builds", FieldValue::Integer(commit.builds));
        fields.insert(
            "builds_successful",
            FieldValue::Integer(commit.builds_successful),
        );
        fields.insert("builds_failed", FieldValue::Integer(commit.builds_failed));

        Self {
            measurement: "commit",
            tags,
            fields,
            timestamp: Timestamp::new(&commit.committed_at),
        }
    }
}
