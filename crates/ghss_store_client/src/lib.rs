use ghss_github::{CheckRun, CheckRunConclusion};
use std::convert::TryInto;
pub use tonic::{transport::channel::Channel, Code, Response, Status};

tonic::include_proto!("ghss.store");

impl From<CheckRun> for Build {
    fn from(check_run: CheckRun) -> Self {
        Self {
            name: check_run.name,
            source: BuildSource::CheckRun as i32,
            commit: check_run.head_sha,
            successful: match &check_run.conclusion {
                Some(conclusion) => conclusion == &CheckRunConclusion::Success,
                None => false,
            },
            failed: match &check_run.conclusion {
                Some(conclusion) => {
                    conclusion == &CheckRunConclusion::Failure
                        || conclusion == &CheckRunConclusion::TimedOut
                }
                None => false,
            },
            duration_ms: match check_run.completed_at {
                Some(completed_at) => (completed_at.timestamp_millis()
                    - check_run.started_at.timestamp_millis())
                .try_into()
                .expect("duration should fit into u32"),
                None => 0,
            },
            timestamp: check_run.started_at.timestamp_millis(),
        }
    }
}
