mod influxdb;

use chrono::{DateTime, FixedOffset};
use github_client::{Client, CommitStatus, CommitStatusState};
use once_cell::sync::Lazy;
use std::collections::HashMap;

static APP_ID: Lazy<String> = Lazy::new(|| std::env::var("GH_APP_ID").unwrap());
static PRIVATE_KEY: Lazy<String> = Lazy::new(|| std::env::var("GH_PRIVATE_KEY").unwrap());
static COMMITS_SINCE: Lazy<String> = Lazy::new(|| std::env::var("GH_COMMITS_SINCE").unwrap());
static COMMITS_UNTIL: Lazy<String> = Lazy::new(|| std::env::var("GH_COMMITS_UNTIL").unwrap());

struct Build {
    name: String,
    successful: bool,
    duration_ms: i64,
    created_at: DateTime<FixedOffset>,
}

fn to_builds(mut statuses: Vec<CommitStatus>) -> Vec<Build> {
    statuses.sort_by(|a, b| {
        a.created_at
            .timestamp_millis()
            .cmp(&b.created_at.timestamp_millis())
    });

    statuses
        .into_iter()
        .fold(
            Vec::<Vec<CommitStatus>>::new(),
            |mut groups, curr_status| {
                let index = groups
                    .iter()
                    .enumerate()
                    .find(|group| {
                        group.1.iter().all(|status| {
                            status.context == curr_status.context
                                && status.state == CommitStatusState::Pending
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
            let name = first.context.clone();
            let last = iter.last().unwrap_or(first);
            let last_millis = last.created_at.timestamp_millis();
            Build {
                name,
                successful: last.state == CommitStatusState::Success,
                duration_ms: last_millis - first_millis,
                created_at,
            }
        })
        .collect()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new_app_auth(&*APP_ID, &*PRIVATE_KEY)?;
    let installations = client.get_app_installations().await?;
    for installation in installations {
        println!("Installation {}", installation.id);
        let token = client
            .create_app_installation_access_token(installation.id)
            .await?;
        let client = Client::new(&token.token)?;
        let repositories = client.get_installation_repositories().await?;
        for repository in repositories {
            println!("Repository {}", repository.full_name);
            let commits = client
                .get_commits(
                    &repository.owner.login,
                    &repository.name,
                    &*COMMITS_SINCE,
                    &*COMMITS_UNTIL,
                )
                .await?;
            let commits_len = commits.len();
            let mut commits_curr: usize = 0;
            let mut influx_points = Vec::new();
            for commit in commits {
                commits_curr += 1;
                println!("Commit {}/{}", commits_curr, commits_len);

                let statuses = client
                    .get_statuses(&repository.owner.login, &repository.name, &commit.sha)
                    .await?;
                let builds = to_builds(statuses);

                for build in builds {
                    let mut tags = HashMap::new();
                    tags.insert("name", build.name);
                    tags.insert("commit", commit.sha.clone());

                    let mut fields = HashMap::new();
                    fields.insert(
                        "successful",
                        influxdb::FieldValue::Boolean(build.successful),
                    );
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
            }

            influxdb::write(influx_points).await?;
        }
    }

    Ok(())
}
