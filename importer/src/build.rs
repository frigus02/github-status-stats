use chrono::{DateTime, TimeZone};
use github_client::{CheckRun, Client, CommitStatus, CommitStatusState, Repository};
use log::info;
use stats::{build_from_check_run, build_from_statuses, Build};

type BoxError = Box<dyn std::error::Error>;

fn statuses_to_builds(mut statuses: Vec<CommitStatus>, commit_sha: &str) -> Vec<Build> {
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
        .map(|statuses| build_from_statuses(statuses, commit_sha.to_owned()))
        .collect()
}

fn check_runs_to_builds(check_runs: Vec<CheckRun>) -> Vec<Build> {
    check_runs.into_iter().map(build_from_check_run).collect()
}

pub async fn get_builds_since<Tz: TimeZone>(
    client: &Client,
    repository: &Repository,
    commits_since: &DateTime<Tz>,
) -> Result<Vec<influxdb_client::Point>, BoxError>
where
    Tz::Offset: std::fmt::Display,
{
    let commit_shas = client
        .get_commits(
            &repository.owner.login,
            &repository.name,
            &commits_since.to_rfc3339(),
        )
        .await?
        .into_iter()
        .map(|commit| commit.sha)
        .collect();
    get_builds(client, repository, commit_shas).await
}

pub async fn get_builds(
    client: &Client,
    repository: &Repository,
    commit_shas: Vec<String>,
) -> Result<Vec<influxdb_client::Point>, BoxError> {
    let mut points = Vec::new();
    let commits_len = commit_shas.len();
    let mut commits_curr: usize = 0;
    for commit_sha in commit_shas {
        commits_curr += 1;
        info!("Commit {}/{}", commits_curr, commits_len);

        let statuses = client
            .get_statuses(&repository.owner.login, &repository.name, &commit_sha)
            .await?;
        let builds = statuses_to_builds(statuses, &commit_sha);
        for build in builds {
            points.push(build.into_point());
        }

        let check_runs = client
            .get_check_runs(&repository.owner.login, &repository.name, &commit_sha)
            .await?;
        let builds = check_runs_to_builds(check_runs);
        for build in builds {
            points.push(build.into_point());
        }
    }

    Ok(points)
}
