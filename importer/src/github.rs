use chrono::{Date, DateTime, FixedOffset, SecondsFormat};
use futures::future::join_all;
use github_client::{Client, Commit, CommitStatus};
use once_cell::sync::Lazy;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use tokio::fs;

static TOKEN: Lazy<String> = Lazy::new(|| std::env::var("GH_TOKEN").unwrap());
static OWNER: Lazy<String> = Lazy::new(|| std::env::var("GH_OWNER").unwrap());
static REPO: Lazy<String> = Lazy::new(|| std::env::var("GH_REPO").unwrap());
static COMMITS_SINCE: Lazy<String> = Lazy::new(|| std::env::var("GH_COMMITS_SINCE").unwrap());
static COMMITS_UNTIL: Lazy<String> = Lazy::new(|| std::env::var("GH_COMMITS_UNTIL").unwrap());
static CLIENT: Lazy<Client> =
    Lazy::new(|| Client::new((*OWNER).clone(), (*REPO).clone(), (*TOKEN).clone()).unwrap());

pub struct DaysBetween {
    curr: Date<FixedOffset>,
    until: Date<FixedOffset>,
}

impl Iterator for DaysBetween {
    type Item = Date<FixedOffset>;

    fn next(&mut self) -> Option<Date<FixedOffset>> {
        if self.curr <= self.until {
            let result = self.curr;
            self.curr = self.curr.succ();
            Some(result)
        } else {
            None
        }
    }
}

pub fn days_between(since: &str, until: &str) -> Result<DaysBetween, chrono::ParseError> {
    let since = DateTime::parse_from_rfc3339(since)?.date();
    let until = DateTime::parse_from_rfc3339(until)?.date();
    Ok(DaysBetween { curr: since, until })
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

pub async fn load_commits() -> Result<Vec<Commit>, String> {
    let days = days_between(&*COMMITS_SINCE, &*COMMITS_UNTIL).map_err(|err| {
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
            Box::pin(CLIENT.get_commits(start_of_day, end_of_day))
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
    read_or_fetch_and_write(PathBuf::from(path), || {
        Box::pin(CLIENT.get_statuses(git_ref))
    })
    .await
}
