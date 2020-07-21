use chrono::{DateTime, FixedOffset};
use ghss_github::{
    CheckRun, Client, CommitStatus, CommitStatusState, MostRecentCommit, Repository,
};
use ghss_store_client::{Build, BuildSource, Commit};
use itertools::Itertools;
use std::convert::TryInto;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

fn statuses_to_build(statuses: Vec<CommitStatus>, commit: String) -> Build {
    let mut iter = statuses.into_iter();
    let first = iter.next().unwrap();
    let first_millis = first.created_at.timestamp_millis();
    let name = first.context.clone();
    let last = iter.last().unwrap_or(first);
    let last_millis = last.created_at.timestamp_millis();
    Build {
        name,
        source: BuildSource::Status as i32,
        commit,
        successful: last.state == CommitStatusState::Success,
        failed: last.state == CommitStatusState::Error || last.state == CommitStatusState::Failure,
        duration_ms: (last_millis - first_millis)
            .try_into()
            .expect("duration should fit into u32"),
        timestamp: first_millis,
    }
}

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
                .map(|statuses| statuses_to_build(statuses, commit_sha.to_owned()))
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

fn builds_to_commit(builds: Vec<&Build>, timestamp: DateTime<FixedOffset>) -> Commit {
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
    Commit {
        build_name: first.name.clone(),
        build_source: first.source,
        commit: first.commit.clone(),
        builds: builds_len,
        builds_successful,
        builds_failed,
        timestamp: timestamp.timestamp_millis(),
    }
}

fn builds_to_commits(builds: &[Build], committed_date: DateTime<FixedOffset>) -> Vec<Commit> {
    (&builds.iter().group_by(|build| build.name.clone()))
        .into_iter()
        .map(|(_, group)| builds_to_commit(group.collect(), committed_date))
        .collect()
}

pub async fn get_most_recent_builds(
    client: &Client,
    repository: &Repository,
) -> Result<(Vec<Build>, Vec<Commit>), BoxError> {
    let commit_shas = client
        .get_most_recent_commits(&repository.owner.login, &repository.name)
        .await?;
    get_builds(client, repository, commit_shas).await
}

pub async fn get_builds_from_commit_shas(
    client: &Client,
    repository: &Repository,
    commit_shas: Vec<String>,
) -> Result<(Vec<Build>, Vec<Commit>), BoxError> {
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
    recent_commits: Vec<MostRecentCommit>,
) -> Result<(Vec<Build>, Vec<Commit>), BoxError> {
    let mut builds = Vec::new();
    let mut commits = Vec::new();
    for commit in recent_commits {
        let statuses = client
            .get_statuses(&repository.owner.login, &repository.name, &commit.sha)
            .await?;
        let status_builds = statuses_to_builds(statuses, &commit.sha);
        commits.extend(builds_to_commits(&status_builds, commit.committed_date));
        builds.extend(status_builds);

        let check_runs = client
            .get_check_runs(&repository.owner.login, &repository.name, &commit.sha)
            .await?;
        let check_run_builds = check_runs_to_builds(check_runs);
        commits.extend(builds_to_commits(&check_run_builds, commit.committed_date));
        builds.extend(check_run_builds);
    }

    Ok((builds, commits))
}
