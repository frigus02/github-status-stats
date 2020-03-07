use chrono::{DateTime, FixedOffset};
use ghss_github::{
    CheckRun, Client, CommitStatus, CommitStatusState, MostRecentCommit, Repository,
};
use ghss_influxdb::Point;
use ghss_models::{Build, Commit};
use itertools::Itertools;

type BoxError = Box<dyn std::error::Error>;

fn statuses_to_builds(mut statuses: Vec<CommitStatus>, commit_sha: &str) -> Vec<Build> {
    statuses.sort_by(|a, b| {
        a.created_at
            .timestamp_millis()
            .cmp(&b.created_at.timestamp_millis())
    });

    statuses
        .into_iter()
        .group_by(|status| status.context.clone())
        .into_iter()
        .flat_map(|group| {
            let (_, statuses) = group;
            statuses
                .batching(|it| match it.next() {
                    None => None,
                    Some(x) => {
                        let mut result: Vec<CommitStatus> = vec![x];
                        while result.last().unwrap().state == CommitStatusState::Pending {
                            match it.next() {
                                Some(x) => result.push(x),
                                None => break,
                            };
                        }
                        Some(result)
                    }
                })
                .map(|statuses| (commit_sha.to_owned(), statuses).into())
                .collect_vec()
        })
        .collect()
}

fn check_runs_to_builds(check_runs: Vec<CheckRun>) -> Vec<Build> {
    check_runs
        .into_iter()
        .map(|check_run| check_run.into())
        .collect()
}

fn builds_to_points(builds: Vec<Build>, committed_date: DateTime<FixedOffset>) -> Vec<Point> {
    let mut points = Vec::new();
    for (_, group) in &builds.iter().group_by(|build| build.name.clone()) {
        points.push(Commit::from((committed_date, group.collect())).into());
    }

    for build in builds {
        points.push(build.into());
    }

    points
}

pub async fn get_most_recent_builds(
    client: &Client,
    repository: &Repository,
) -> Result<Vec<ghss_influxdb::Point>, BoxError> {
    let commit_shas = client
        .get_most_recent_commits(&repository.owner.login, &repository.name)
        .await?;
    get_builds(client, repository, commit_shas).await
}

pub async fn get_builds_from_commit_shas(
    client: &Client,
    repository: &Repository,
    commit_shas: Vec<String>,
) -> Result<Vec<ghss_influxdb::Point>, BoxError> {
    let commit_dates = client
        .get_commit_dates(&repository.owner.login, &repository.name, &commit_shas)
        .await?;
    let commits = commit_shas
        .into_iter()
        .zip(commit_dates.into_iter())
        .map(|(sha, committed_date)| MostRecentCommit {
            sha,
            committed_date,
        })
        .collect();
    get_builds(client, repository, commits).await
}

async fn get_builds(
    client: &Client,
    repository: &Repository,
    commits: Vec<MostRecentCommit>,
) -> Result<Vec<ghss_influxdb::Point>, BoxError> {
    let mut points = Vec::new();
    for commit in commits {
        let statuses = client
            .get_statuses(&repository.owner.login, &repository.name, &commit.sha)
            .await?;
        let builds = statuses_to_builds(statuses, &commit.sha);
        points.extend(builds_to_points(builds, commit.committed_date));

        let check_runs = client
            .get_check_runs(&repository.owner.login, &repository.name, &commit.sha)
            .await?;
        let builds = check_runs_to_builds(check_runs);
        points.extend(builds_to_points(builds, commit.committed_date));
    }

    Ok(points)
}
