use chrono::{DateTime, FixedOffset, TimeZone};
use github_client::{
    CheckRun, CheckRunConclusion, Client, CommitStatus, CommitStatusState, Repository,
};
use std::collections::HashMap;

type BoxError = Box<dyn std::error::Error>;

struct Build {
    name: String,
    successful: bool,
    duration_ms: i64,
    created_at: DateTime<FixedOffset>,
}

impl Build {
    fn from_statuses(statuses: Vec<CommitStatus>) -> Build {
        let mut iter = statuses.into_iter();
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
    }

    fn from_check_run(check_run: CheckRun) -> Build {
        Build {
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
        }
    }

    fn to_point(self, commit_sha: String) -> influxdb_client::Point {
        let mut tags = HashMap::new();
        tags.insert("name", self.name);
        tags.insert("commit", commit_sha);

        let mut fields = HashMap::new();
        fields.insert(
            "successful",
            influxdb_client::FieldValue::Boolean(self.successful),
        );
        fields.insert(
            "duration_ms",
            influxdb_client::FieldValue::Integer(self.duration_ms),
        );

        influxdb_client::Point {
            measurement: "build",
            tags,
            fields,
            timestamp: influxdb_client::Timestamp::new(&self.created_at),
        }
    }
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
        .map(Build::from_statuses)
        .collect()
}

fn check_runs_to_builds(check_runs: Vec<CheckRun>) -> Vec<Build> {
    check_runs.into_iter().map(Build::from_check_run).collect()
}

pub async fn get_builds<Tz: TimeZone>(
    client: &Client,
    repository: &Repository,
    commits_since: &DateTime<Tz>,
) -> Result<Vec<influxdb_client::Point>, BoxError>
where
    Tz::Offset: std::fmt::Display,
{
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
            points.push(build.to_point(commit.sha.clone()));
        }

        let check_runs = client
            .get_check_runs(&repository.owner.login, &repository.name, &commit.sha)
            .await?;
        let builds = check_runs_to_builds(check_runs);
        for build in builds {
            points.push(build.to_point(commit.sha.clone()));
        }
    }

    Ok(points)
}
