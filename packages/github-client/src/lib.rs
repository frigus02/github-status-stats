mod call;
mod datetime;
mod page_links;

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://api.github.com";
pub const USER_AGENT: &str = concat!("github-status-stats/", env!("CARGO_PKG_VERSION"));

type BoxError = Box<dyn std::error::Error>;

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub login: String,
    pub id: i32,
    pub node_id: String,
    pub avatar_url: String,
    pub gravatar_id: String,
    pub url: String,
    pub html_url: String,
    pub followers_url: String,
    pub following_url: String,
    pub gists_url: String,
    pub starred_url: String,
    pub subscriptions_url: String,
    pub organizations_url: String,
    pub repos_url: String,
    pub events_url: String,
    pub received_events_url: String,
    pub r#type: String,
    pub site_admin: bool,
    pub name: String,
    pub company: String,
    pub blog: String,
    pub location: Option<String>,
    pub email: Option<String>,
    pub hireable: Option<bool>,
    pub bio: Option<String>,
    pub public_repos: i32,
    pub public_gists: i32,
    pub followers: i32,
    pub following: i32,
    #[serde(with = "datetime")]
    pub created_at: DateTime<FixedOffset>,
    #[serde(with = "datetime")]
    pub updated_at: DateTime<FixedOffset>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserPlan {
    pub name: String,
    pub space: i32,
    pub private_repos: i32,
    pub collaborators: i32,
}

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
}

impl Client {
    pub fn new(token: &str) -> Result<Client, BoxError> {
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

        Ok(Client { client })
    }

    pub async fn get_user(&self) -> Result<User, BoxError> {
        let raw_url = format!("{base}/user", base = BASE_URL,);
        let url = reqwest::Url::parse(&raw_url)?;
        let user = call::call_api_single(&self.client, url).await?;
        Ok(user)
    }

    pub async fn get_commits(
        &self,
        owner: &str,
        repo: &str,
        since: String,
        until: String,
    ) -> Result<Vec<Commit>, BoxError> {
        let raw_url = format!(
            "{base}/repos/{owner}/{repo}/commits",
            base = BASE_URL,
            owner = owner,
            repo = repo
        );
        let url = reqwest::Url::parse_with_params(&raw_url, &[("since", since), ("until", until)])?;
        let commits = call::call_api_paged(&self.client, url).await?;
        Ok(commits)
    }

    pub async fn get_statuses(
        &self,
        owner: &str,
        repo: &str,
        git_ref: String,
    ) -> Result<Vec<CommitStatus>, BoxError> {
        let raw_url = format!(
            "{base}/repos/{owner}/{repo}/commits/{git_ref}/statuses",
            base = BASE_URL,
            owner = owner,
            repo = repo,
            git_ref = git_ref
        );
        let url = reqwest::Url::parse(&raw_url)?;
        let statuses = call::call_api_paged(&self.client, url).await?;
        Ok(statuses)
    }
}
