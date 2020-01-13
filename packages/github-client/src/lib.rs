mod call;
mod datetime;
mod page_links;

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://api.github.com";
const USER_AGENT: &str = concat!("github-status-stats/", env!("CARGO_PKG_VERSION"));

type BoxError = Box<dyn std::error::Error>;

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitPerson {
    pub name: String,
    pub email: String,
    #[serde(with = "datetime")]
    pub date: DateTime<FixedOffset>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitCommit {
    pub author: CommitPerson,
    pub committer: CommitPerson,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Commit {
    pub sha: String,
    pub commit: CommitCommit,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CommitStatusState {
    Pending,
    Error,
    Failure,
    Success,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitStatus {
    pub state: CommitStatusState,
    pub description: String,
    pub context: String,
    #[serde(with = "datetime")]
    pub created_at: DateTime<FixedOffset>,
    #[serde(with = "datetime")]
    pub updated_at: DateTime<FixedOffset>,
}

pub struct Client {
    client: reqwest::Client,
    owner: String,
    repo: String,
}

impl Client {
    pub fn new(owner: String, repo: String, token: String) -> Result<Client, BoxError> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            reqwest::header::HeaderValue::from_static(USER_AGENT),
        );
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("token {}", token).parse().unwrap(),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Client {
            client,
            repo,
            owner,
        })
    }

    pub async fn get_commits(&self, since: String, until: String) -> Result<Vec<Commit>, BoxError> {
        let raw_url = format!(
            "{base}/repos/{owner}/{repo}/commits",
            base = BASE_URL,
            owner = &self.owner,
            repo = &self.repo
        );
        let url = reqwest::Url::parse_with_params(&raw_url, &[("since", since), ("until", until)])?;
        let commits = call::call_api_paged(&self.client, url).await?;
        Ok(commits)
    }

    pub async fn get_statuses(&self, git_ref: String) -> Result<Vec<CommitStatus>, BoxError> {
        let raw_url = format!(
            "{base}/repos/{owner}/{repo}/commits/{git_ref}/statuses",
            base = BASE_URL,
            owner = &self.owner,
            repo = &self.repo,
            git_ref = git_ref
        );
        let url = reqwest::Url::parse(&raw_url)?;
        let statuses = call::call_api_paged(&self.client, url).await?;
        Ok(statuses)
    }
}
