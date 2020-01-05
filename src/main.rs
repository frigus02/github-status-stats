mod github;
mod influxdb;
mod transform;

use chrono::{DateTime, FixedOffset};
use itertools::Itertools;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use std::convert::TryInto;

static RE_BUILD_CANCELED: Lazy<Option<Regex>> = Lazy::new(|| {
    std::env::var("BUILD_CANCELED_REGEX")
        .ok()
        .map(|var| Regex::new(&var).unwrap())
});

static TRANSFORM_STATUS_CONTEXT: Lazy<transform::Transform> =
    Lazy::new(|| transform::create_transform_from_env("STATUS_CONTEXT_TRANSFORM").unwrap());

struct Build {
    name: String,
    successful: bool,
    canceled: bool,
    duration_ms: i64,
    created_at: DateTime<FixedOffset>,
}

struct BuildAggregate {
    name: String,
    attempts: usize,
    first_attempt_successful: bool,
}

fn is_cancelled(status: &github::CommitStatus) -> bool {
    match &*RE_BUILD_CANCELED {
        Some(re) => re.is_match(&status.description),
        None => false,
    }
}

fn to_builds(mut statuses: Vec<github::CommitStatus>) -> Vec<Build> {
    let t = &*TRANSFORM_STATUS_CONTEXT;

    statuses.sort_by(|a, b| {
        a.created_at
            .timestamp_millis()
            .cmp(&b.created_at.timestamp_millis())
    });

    statuses
        .into_iter()
        .fold(
            Vec::<Vec<github::CommitStatus>>::new(),
            |mut groups, curr_status| {
                let index = groups
                    .iter()
                    .enumerate()
                    .find(|group| {
                        group.1.iter().all(|status| {
                            status.context == curr_status.context
                                && status.state == github::CommitStatusState::Pending
                        })
                    })
                    .map(|group| group.0);
                match index {
                    Some(index) => groups[index].push(curr_status),
                    None => groups.insert(0, vec![curr_status]),
                };
                groups
            },
        )
        .into_iter()
        .rev()
        .map(|group| {
            let mut iter = group.into_iter();
            let first = iter.next().unwrap();
            let first_millis = first.created_at.timestamp_millis();
            let created_at = first.created_at;
            let name = t.transform(first.context.clone());
            let last = iter.last().unwrap_or(first);
            let last_millis = last.created_at.timestamp_millis();
            Build {
                name,
                successful: last.state == github::CommitStatusState::Success,
                canceled: is_cancelled(&last),
                duration_ms: last_millis - first_millis,
                created_at,
            }
        })
        .collect()
}

fn accumulate_builds(sorted_builds: &[Build]) -> Vec<BuildAggregate> {
    sorted_builds
        .iter()
        .group_by(|build| &build.name)
        .into_iter()
        .map(|(key, mut group)| {
            let first_attempt_successful = group.next().unwrap().successful;
            BuildAggregate {
                name: key.clone(),
                attempts: 1 + group.count(),
                first_attempt_successful,
            }
        })
        .collect()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let commits = github::load_commits().await?;
    let commits_len = commits.len();
    let mut commits_curr: usize = 0;
    let mut influx_points = Vec::new();
    for commit in commits {
        commits_curr += 1;
        println!("Commit {}/{}", commits_curr, commits_len);

        let statuses = github::load_statuses(commit.sha.clone()).await?;
        let builds = to_builds(statuses);
        let acc_builds = accumulate_builds(&builds);

        for build in builds {
            let mut tags = HashMap::new();
            tags.insert("name", build.name);
            tags.insert("commit", commit.sha.clone());

            let mut fields = HashMap::new();
            fields.insert(
                "successful",
                influxdb::FieldValue::Boolean(build.successful),
            );
            fields.insert("canceled", influxdb::FieldValue::Boolean(build.canceled));
            fields.insert(
                "duration_ms",
                influxdb::FieldValue::Integer(build.duration_ms),
            );

            influx_points.push(influxdb::Point {
                measurement: "build",
                tags,
                fields,
                timestamp: influxdb::Timestamp::new(&build.created_at),
            });
        }

        for build in acc_builds {
            let mut tags = HashMap::new();
            tags.insert("name", build.name);
            tags.insert("commit", commit.sha.clone());

            let mut fields = HashMap::new();
            fields.insert(
                "attempts",
                influxdb::FieldValue::Integer(build.attempts.try_into().unwrap()),
            );
            fields.insert(
                "first_attempt_successful",
                influxdb::FieldValue::Boolean(build.first_attempt_successful),
            );

            influx_points.push(influxdb::Point {
                measurement: "build_per_commit",
                tags,
                fields,
                timestamp: influxdb::Timestamp::new(&commit.commit.committer.date),
            });
        }
    }

    influxdb::drop_measurement("build").await?;
    influxdb::drop_measurement("build_per_commit").await?;
    tokio::time::delay_for(std::time::Duration::from_secs(5)).await;
    influxdb::write(influx_points).await?;

    Ok(())
}
