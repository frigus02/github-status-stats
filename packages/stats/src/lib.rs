mod models;

use github_client::{CheckRun, CheckRunConclusion, CommitStatus, CommitStatusState, Repository};
pub use models::*;

pub fn influxdb_name(repository: &Repository) -> String {
    format!("r{}", repository.id)
}

pub fn build_from_statuses(statuses: Vec<CommitStatus>, commit_sha: String) -> Build {
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
        commit_sha,
    }
}

pub fn build_from_check_run(check_run: CheckRun) -> Build {
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
        commit_sha: check_run.head_sha,
    }
}
