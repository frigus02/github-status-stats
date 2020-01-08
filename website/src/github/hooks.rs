use actix_web::HttpRequest;
use bytes::Bytes;
use hmac::{Hmac, Mac};
use once_cell::sync::Lazy;
use secstr::SecStr;
use serde::Deserialize;
use sha1::Sha1;

static WEBHOOK_SECRET: Lazy<SecStr> =
    Lazy::new(|| SecStr::from(std::env::var("GH_WEBHOOK_SECRET").unwrap()));

#[derive(Debug)]
pub enum Payload {
    Ping(PingPayload),
    Status(StatusPayload),
    GitHubAppAuthorization(GitHubAppAuthorizationPayload),
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Config {
    pub content_type: String,
    pub insecure_ssl: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Hook {
    pub r#type: String,
    pub id: i32,
    pub name: String,
    pub active: bool,
    pub events: Vec<String>,
    pub config: Config,
    pub updated_at: String,
    pub created_at: String,
    pub app_id: i32,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
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
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Repository {
    pub id: i32,
    pub node_id: String,
    pub name: String,
    pub full_name: String,
    pub private: bool,
    pub owner: User,
    pub html_url: String,
    pub description: Option<String>,
    pub fork: bool,
    pub url: String,
    pub forks_url: String,
    pub keys_url: String,
    pub collaborators_url: String,
    pub teams_url: String,
    pub hooks_url: String,
    pub issue_events_url: String,
    pub events_url: String,
    pub assignees_url: String,
    pub branches_url: String,
    pub tags_url: String,
    pub blobs_url: String,
    pub git_tags_url: String,
    pub git_refs_url: String,
    pub trees_url: String,
    pub statuses_url: String,
    pub languages_url: String,
    pub stargazers_url: String,
    pub contributors_url: String,
    pub subscribers_url: String,
    pub subscription_url: String,
    pub commits_url: String,
    pub git_commits_url: String,
    pub comments_url: String,
    pub issue_comment_url: String,
    pub contents_url: String,
    pub compare_url: String,
    pub merges_url: String,
    pub archive_url: String,
    pub downloads_url: String,
    pub issues_url: String,
    pub pulls_url: String,
    pub milestones_url: String,
    pub notifications_url: String,
    pub labels_url: String,
    pub releases_url: String,
    pub deployments_url: String,
    pub created_at: String,
    pub updated_at: String,
    pub pushed_at: String,
    pub git_url: String,
    pub ssh_url: String,
    pub clone_url: String,
    pub svn_url: String,
    pub homepage: Option<String>,
    pub size: i32,
    pub stargazers_count: i32,
    pub watchers_count: i32,
    pub language: String,
    pub has_issues: bool,
    pub has_projects: bool,
    pub has_downloads: bool,
    pub has_wiki: bool,
    pub has_pages: bool,
    pub forks_count: i32,
    pub mirror_url: Option<String>,
    pub archived: bool,
    pub disabled: bool,
    pub open_issues_count: i32,
    pub license: Option<String>,
    pub forks: i32,
    pub open_issues: i32,
    pub watchers: i32,
    pub default_branch: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct PingPayload {
    pub zen: String,
    pub hook_id: i32,
    pub hook: Hook,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CommitCommitUser {
    pub name: String,
    pub email: String,
    pub date: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CommitCommitTree {
    pub sha: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CommitCommitVerification {
    pub verified: bool,
    pub reason: String,
    pub signature: String,
    pub payload: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CommitCommit {
    pub author: CommitCommitUser,
    pub committer: CommitCommitUser,
    pub message: String,
    pub tree: CommitCommitTree,
    pub url: String,
    pub comment_count: i32,
    pub verification: CommitCommitVerification,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Commit {
    pub sha: String,
    pub node_id: String,
    pub commit: CommitCommit,
    pub url: String,
    pub html_url: String,
    pub comments_url: String,
    pub author: User,
    pub committer: User,
    //"parents": []
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct BranchCommit {
    pub sha: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Branch {
    pub name: String,
    pub commit: BranchCommit,
    pub protected: bool,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct StatusPayload {
    pub id: i32,
    pub sha: String,
    pub name: String,
    pub target_url: Option<String>,
    pub context: String,
    pub description: Option<String>,
    pub state: String,
    pub commit: Commit,
    pub branches: Vec<Branch>,
    pub created_at: String,
    pub updated_at: String,
    pub repository: Repository,
    pub sender: User,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct GitHubAppAuthorizationPayload {
    pub action: String,
    pub sender: User,
}

fn header_as_string<'a>(req: &'a HttpRequest, header_name: &str) -> Result<&'a str, String> {
    req.headers()
        .get(header_name)
        .ok_or(format!("Header {} missing", header_name))
        .and_then(|header| {
            header
                .to_str()
                .map_err(|err| format!("Header {} not readable: {}", header_name, err))
        })
}

fn validate_signature(req: &HttpRequest, body: &Bytes) -> Result<(), String> {
    let signature = SecStr::from(header_as_string(req, "X-Hub-Signature")?);
    let mut mac = Hmac::<Sha1>::new_varkey(&*WEBHOOK_SECRET.unsecure())
        .expect("HMAC can take key of any size");
    mac.input(body);
    let result = SecStr::from(format!("sha1={:x}", mac.result().code()));
    if result == signature {
        Ok(())
    } else {
        Err(format!("Signature doesn't match"))
    }
}

pub fn deserialize(req: HttpRequest, body: Bytes) -> Result<Payload, String> {
    validate_signature(&req, &body)?;

    let event = header_as_string(&req, "X-GitHub-Event")?;
    match event {
        "ping" => serde_json::from_slice::<PingPayload>(&body)
            .map(|data| Payload::Ping(data))
            .map_err(|err| format!("Failed to deserialize ping event: {}", err)),
        "status" => serde_json::from_slice::<StatusPayload>(&body)
            .map(|data| Payload::Status(data))
            .map_err(|err| format!("Failed to deserialize status event: {}", err)),
        "github_app_authorization" => {
            serde_json::from_slice::<GitHubAppAuthorizationPayload>(&body)
                .map(|data| Payload::GitHubAppAuthorization(data))
                .map_err(|err| {
                    format!(
                        "Failed to deserialize github_app_authorization event: {}",
                        err
                    )
                })
        }
        _ => Err(format!("Unsupported event {}", event)),
    }
}
