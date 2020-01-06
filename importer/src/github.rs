mod call;
mod datetime;
mod days_between;
mod page_links;

use chrono::{DateTime, FixedOffset, SecondsFormat};
use futures::future::join_all;
use once_cell::sync::Lazy;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use tokio::fs;

const BASE_URL: &str = "https://api.github.com";
const USER_AGENT: &str = concat!("github-status-stats/", env!("CARGO_PKG_VERSION"));

static TOKEN: Lazy<String> = Lazy::new(|| std::env::var("GH_TOKEN").unwrap());
static OWNER: Lazy<String> = Lazy::new(|| std::env::var("GH_OWNER").unwrap());
static REPO: Lazy<String> = Lazy::new(|| std::env::var("GH_REPO").unwrap());
static COMMITS_SINCE: Lazy<String> = Lazy::new(|| std::env::var("GH_COMMITS_SINCE").unwrap());
static COMMITS_UNTIL: Lazy<String> = Lazy::new(|| std::env::var("GH_COMMITS_UNTIL").unwrap());
static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static(USER_AGENT),
    );
    headers.insert(
        reqwest::header::AUTHORIZATION,
        format!("token {}", *TOKEN).parse().unwrap(),
    );

    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap()
});

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

async fn read_or_fetch_and_write<T, F>(path: PathBuf, fetch: F) -> Result<T, String>
where
    T: DeserializeOwned,
    T: Serialize,
    T: ?Sized,
    F: FnOnce() -> Pin<Box<dyn Future<Output = Result<T, Box<dyn std::error::Error>>>>>,
{
    let result: Result<T, String> = match fs::read(&path).await {
        Ok(contents) => serde_json::from_slice(&contents)
            .map_err(|err| format!("Error deserializing file: {:#?}", err)),
        Err(err) => {
            if err.kind() == tokio::io::ErrorKind::NotFound {
                let result = fetch()
                    .await
                    .map_err(|err| format!("Error fetching: {:#?}", err))?;

                let dirname = path.parent().ok_or("Error getting dirname")?;
                fs::create_dir_all(dirname)
                    .await
                    .map_err(|err| format!("Error creating directory: {:#?}", err))?;
                let serialized = serde_json::to_vec(&result)
                    .map_err(|err| format!("Error serializing result: {:#?}", err))?;
                fs::write(path, serialized)
                    .await
                    .map_err(|err| format!("Error writing file: {:#?}", err))?;

                Ok(result)
            } else {
                Err(format!("Error reading file: {:#?}", err))
            }
        }
    };

    result
}

async fn get_commits(
    since: String,
    until: String,
) -> Result<Vec<Commit>, Box<dyn std::error::Error>> {
    let raw_url = format!(
        "{base}/repos/{owner}/{repo}/commits",
        base = BASE_URL,
        owner = *OWNER,
        repo = *REPO
    );
    let url = reqwest::Url::parse_with_params(&raw_url, &[("since", since), ("until", until)])?;
    let commits = call::call_api_paged(&*CLIENT, url).await?;
    Ok(commits)
}

async fn get_statuses(git_ref: String) -> Result<Vec<CommitStatus>, Box<dyn std::error::Error>> {
    let raw_url = format!(
        "{base}/repos/{owner}/{repo}/commits/{git_ref}/statuses",
        base = BASE_URL,
        owner = *OWNER,
        repo = *REPO,
        git_ref = git_ref
    );
    let url = reqwest::Url::parse(&raw_url)?;
    let statuses = call::call_api_paged(&*CLIENT, url).await?;
    Ok(statuses)
}

pub async fn load_commits() -> Result<Vec<Commit>, String> {
    let days = days_between::days_between(&*COMMITS_SINCE, &*COMMITS_UNTIL).map_err(|err| {
        format!(
            "Error parsing GH_COMMITS_SINCE/GH_COMMITS_UNTIL: {:#?}",
            err
        )
    })?;
    let results = join_all(days.map(|day| {
        let start_of_day = day
            .and_hms_milli(0, 0, 0, 0)
            .to_rfc3339_opts(SecondsFormat::Millis, true);
        let end_of_day = day
            .and_hms_milli(23, 59, 59, 999)
            .to_rfc3339_opts(SecondsFormat::Millis, true);
        let path = format!(
            "data/{owner}/{repo}/commits/{date}.json",
            owner = *OWNER,
            repo = *REPO,
            date = start_of_day
                .replace(":", "")
                .replace("-", "")
                .replace(".", ""),
        );
        read_or_fetch_and_write(PathBuf::from(path), || {
            Box::pin(get_commits(start_of_day, end_of_day))
        })
    }))
    .await;
    let commits = results
        .into_iter()
        .collect::<Result<Vec<Vec<Commit>>, String>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<Commit>>();

    Ok(commits)
}

pub async fn load_statuses(git_ref: String) -> Result<Vec<CommitStatus>, String> {
    let path = format!(
        "data/{owner}/{repo}/statuses/{commit_sha}.json",
        owner = *OWNER,
        repo = *REPO,
        commit_sha = git_ref,
    );
    read_or_fetch_and_write(PathBuf::from(path), || Box::pin(get_statuses(git_ref))).await
}
