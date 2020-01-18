use chrono::{DateTime, Duration, FixedOffset, Utc};
use github_client::{
    CheckRun, CheckRunConclusion, Client, CommitStatus, CommitStatusState, Repository,
};
use once_cell::sync::Lazy;
use std::collections::HashMap;

type BoxError = Box<dyn std::error::Error>;

static GH_APP_ID: Lazy<String> = Lazy::new(|| std::env::var("GH_APP_ID").unwrap());
static GH_PRIVATE_KEY: Lazy<String> = Lazy::new(|| std::env::var("GH_PRIVATE_KEY").unwrap());

static INFLUXDB_BASE_URL: Lazy<String> = Lazy::new(|| std::env::var("INFLUXDB_BASE_URL").unwrap());
static INFLUXDB_USERNAME: Lazy<String> = Lazy::new(|| std::env::var("INFLUXDB_USERNAME").unwrap());
static INFLUXDB_PASSWORD: Lazy<String> = Lazy::new(|| std::env::var("INFLUXDB_PASSWORD").unwrap());

struct Build {
    name: String,
    successful: bool,
    duration_ms: i64,
    created_at: DateTime<FixedOffset>,
}

fn statuses_to_builds(mut statuses: Vec<CommitStatus>) -> Vec<Build> {
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

fn check_runs_to_builds(check_runs: Vec<CheckRun>) -> Vec<Build> {
    check_runs
        .into_iter()
        .map(|check_run| Build {
            name: check_run.name,
            successful: match check_run.conclusion {
                Some(conclusion) => conclusion == CheckRunConclusion::Success,
                None => false,
            },
            duration_ms: match check_run.completed_at {
                Some(completed_at) => {
                    check_run.started_at.timestamp_millis() - completed_at.timestamp_millis()
                }
                None => 0,
            },
            created_at: check_run.started_at,
        })
        .collect()
}

fn new_build_point(build: Build, commit_sha: String) -> influxdb_client::Point {
    let mut tags = HashMap::new();
    tags.insert("name", build.name);
    tags.insert("commit", commit_sha);

    let mut fields = HashMap::new();
    fields.insert(
        "successful",
        influxdb_client::FieldValue::Boolean(build.successful),
    );
    fields.insert(
        "duration_ms",
        influxdb_client::FieldValue::Integer(build.duration_ms),
    );

    influxdb_client::Point {
        measurement: "build",
        tags,
        fields,
        timestamp: influxdb_client::Timestamp::new(&build.created_at),
    }
}

async fn get_builds(
    client: &Client,
    repository: Repository,
    commits_since: DateTime<Utc>,
) -> Result<Vec<influxdb_client::Point>, BoxError> {
    let mut points = Vec::new();
    let commits = client
        .get_commits(
            &repository.owner.login,
            &repository.name,
            &commits_since.to_rfc3339(),
        )
        .await?;
    let commits_len = commits.len();
    let mut commits_curr: usize = 0;
    for commit in commits {
        commits_curr += 1;
        println!("Commit {}/{}", commits_curr, commits_len);

        let statuses = client
            .get_statuses(&repository.owner.login, &repository.name, &commit.sha)
            .await?;
        let builds = statuses_to_builds(statuses);
        for build in builds {
            points.push(new_build_point(build, commit.sha.clone()));
        }

        let check_runs = client
            .get_check_runs(&repository.owner.login, &repository.name, &commit.sha)
            .await?;
        let builds = check_runs_to_builds(check_runs);
        for build in builds {
            points.push(new_build_point(build, commit.sha.clone()));
        }
    }

    Ok(points)
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let gh_app_client = Client::new_app_auth(&*GH_APP_ID, &*GH_PRIVATE_KEY)?;
    let installations = gh_app_client.get_app_installations().await?;
    for installation in installations {
        println!("Installation {}", installation.id);
        let token = gh_app_client
            .create_app_installation_access_token(installation.id)
            .await?;
        let gh_inst_client = Client::new(&token.token)?;
        let repositories = gh_inst_client.get_installation_repositories().await?;
        for repository in repositories {
            println!("Repository {}", repository.full_name);

            let influxdb_db = format!("r{}", repository.id);
            let influxdb_client = influxdb_client::Client::new(
                &*INFLUXDB_BASE_URL,
                &influxdb_db,
                &*INFLUXDB_USERNAME,
                &*INFLUXDB_PASSWORD,
            )?;

            let is_new = influxdb_client
                .query("SHOW MEASUREMENTS")
                .await?
                .results
                .first()
                .ok_or_else(|| {
                    format!(
                        "InfluxDB returned no result for SHOW MEASUREMENTS query for repository {}",
                        repository.id
                    )
                })?
                .series
                .is_none();
            if is_new {
                influxdb_client
                    .query(&format!("CREATE DATABASE {}", influxdb_db))
                    .await?;
            }

            let commits_since = if is_new {
                Utc::now() - Duration::weeks(1)
            } else {
                Utc::now() - Duration::hours(2)
            };

            let points = get_builds(&gh_inst_client, repository, commits_since).await?;
            if !points.is_empty() {
                influxdb_client.write(points).await?;
            }
        }
    }

    Ok(())
}
