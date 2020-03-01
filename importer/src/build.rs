use github_client::{CheckRun, Client, CommitStatus, CommitStatusState, Repository};
use stats::Build;

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
        .map(|statuses| {
            let mut build: Build = statuses.into();
            build.commit_sha = commit_sha.to_owned();
            build
        })
        .collect()
}

fn check_runs_to_builds(check_runs: Vec<CheckRun>) -> Vec<Build> {
    check_runs
        .into_iter()
        .map(|check_run| check_run.into())
        .collect()
}

pub async fn get_most_recent_builds(
    client: &Client,
    repository: &Repository,
) -> Result<Vec<influxdb_client::Point>, BoxError> {
    let commit_shas = client
        .get_most_recent_commits(&repository.owner.login, &repository.name)
        .await?;
    get_builds(client, repository, commit_shas).await
}

pub async fn get_builds(
    client: &Client,
    repository: &Repository,
    commit_shas: Vec<String>,
) -> Result<Vec<influxdb_client::Point>, BoxError> {
    let mut points = Vec::new();
    for commit_sha in commit_shas {
        let statuses = client
            .get_statuses(&repository.owner.login, &repository.name, &commit_sha)
            .await?;
        let builds = statuses_to_builds(statuses, &commit_sha);
        for build in builds {
            points.push(build.into());
        }

        let check_runs = client
            .get_check_runs(&repository.owner.login, &repository.name, &commit_sha)
            .await?;
        let builds = check_runs_to_builds(check_runs);
        for build in builds {
            points.push(build.into());
        }
    }

    Ok(points)
}
