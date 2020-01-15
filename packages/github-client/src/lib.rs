mod apps;
mod call;
mod datetime;
mod page_links;

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

const BASE_URL: &str = "https://api.github.com";
pub const USER_AGENT: &str = concat!("github-status-stats/", env!("CARGO_PKG_VERSION"));

type BoxError = Box<dyn std::error::Error>;

#[derive(Debug, Serialize, Deserialize)]
pub struct Account {
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AppWebhookEvent {
    CheckRun,
    CheckSuite,
    CommitComment,
    ContentReference,
    Create,
    Delete,
    Deployment,
    DeploymentStatus,
    Fork,
    Gollum,
    Issues,
    IssueComment,
    Label,
    Member,
    Membership,
    Milestone,
    OrgBlock,
    Organization,
    PageBuild,
    Project,
    ProjectCard,
    ProjectColumn,
    Public,
    PullRequest,
    PullRequestReview,
    PullRequestReviewComment,
    Push,
    Release,
    Repository,
    RepositoryDispatch,
    SecurityAdvisory,
    Status,
    Team,
    TeamAdd,
    Watch,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AppPermissionLevel {
    None,
    Read,
    Write,
}

impl Default for AppPermissionLevel {
    fn default() -> Self {
        AppPermissionLevel::None
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppPermissions {
    #[serde(default)]
    pub administration: AppPermissionLevel,
    #[serde(default)]
    pub blocking: AppPermissionLevel,
    #[serde(default)]
    pub checks: AppPermissionLevel,
    #[serde(default)]
    pub content_references: AppPermissionLevel,
    #[serde(default)]
    pub contents: AppPermissionLevel,
    #[serde(default)]
    pub deployments: AppPermissionLevel,
    #[serde(default)]
    pub emails: AppPermissionLevel,
    #[serde(default)]
    pub followers: AppPermissionLevel,
    #[serde(default)]
    pub gpg_keys: AppPermissionLevel,
    #[serde(default)]
    pub issues: AppPermissionLevel,
    #[serde(default)]
    pub keys: AppPermissionLevel,
    #[serde(default)]
    pub members: AppPermissionLevel,
    #[serde(default)]
    pub metadata: AppPermissionLevel,
    #[serde(default)]
    pub organization_administration: AppPermissionLevel,
    #[serde(default)]
    pub organization_hooks: AppPermissionLevel,
    #[serde(default)]
    pub organization_plan: AppPermissionLevel,
    #[serde(default)]
    pub organization_projects: AppPermissionLevel,
    #[serde(default)]
    pub organization_user_blocking: AppPermissionLevel,
    #[serde(default)]
    pub pages: AppPermissionLevel,
    #[serde(default)]
    pub plan: AppPermissionLevel,
    #[serde(default)]
    pub pull_requests: AppPermissionLevel,
    #[serde(default)]
    pub repository_hooks: AppPermissionLevel,
    #[serde(default)]
    pub repository_projects: AppPermissionLevel,
    #[serde(default)]
    pub single_file: AppPermissionLevel,
    #[serde(default)]
    pub starring: AppPermissionLevel,
    #[serde(default)]
    pub statuses: AppPermissionLevel,
    #[serde(default)]
    pub team_discussions: AppPermissionLevel,
    #[serde(default)]
    pub vulnerability_alerts: AppPermissionLevel,
    #[serde(default)]
    pub watching: AppPermissionLevel,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Installation {
    pub id: i32,
    pub account: Account,
    pub repository_selection: String,
    pub access_tokens_url: String,
    pub repositories_url: String,
    pub html_url: String,
    pub app_id: i32,
    pub app_slug: String,
    pub target_id: i32,
    pub target_type: String,
    pub permissions: AppPermissions,
    pub events: Vec<AppWebhookEvent>,
    #[serde(with = "datetime")]
    pub created_at: DateTime<FixedOffset>,
    #[serde(with = "datetime")]
    pub updated_at: DateTime<FixedOffset>,
    pub single_file_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstallationAccessToken {
    pub token: String,
    #[serde(with = "datetime")]
    pub expires_at: DateTime<FixedOffset>,
    pub permissions: AppPermissions,
    pub repository_selection: String,
}

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

#[derive(Debug, Serialize, Deserialize)]
pub struct License {
    pub key: String,
    pub name: String,
    pub spdx_id: String,
    pub url: String,
    pub node_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Repository {
    pub id: i32,
    pub node_id: String,
    pub name: String,
    pub full_name: String,
    pub private: bool,
    pub owner: Account,
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
    #[serde(with = "datetime")]
    pub created_at: DateTime<FixedOffset>,
    #[serde(with = "datetime")]
    pub updated_at: DateTime<FixedOffset>,
    #[serde(with = "datetime")]
    pub pushed_at: DateTime<FixedOffset>,
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
    pub license: Option<License>,
    pub forks: i32,
    pub open_issues: i32,
    pub watchers: i32,
    pub default_branch: String,
    // "permissions": { "admin": false, "push": false, "pull": false }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepositoryList {
    pub total_count: i32,
    pub repository_selection: String,
    pub repositories: Vec<Repository>,
}

pub struct Client {
    client: reqwest::Client,
}

impl Client {
    pub fn new(token: &str) -> Result<Client, BoxError> {
        Client::new_with_auth_header(format!("token {}", token))
    }

    pub fn new_app_auth(app_id: &str, private_key_pem: &str) -> Result<Client, BoxError> {
        let jwt = apps::generate_jwt(app_id, private_key_pem)?;
        Client::new_with_auth_header(format!("Bearer {}", jwt))
    }

    fn new_with_auth_header(auth_header: String) -> Result<Client, BoxError> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            reqwest::header::HeaderValue::from_static(USER_AGENT),
        );
        headers.insert(reqwest::header::AUTHORIZATION, auth_header.parse()?);

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Client { client })
    }

    pub async fn get_app_installations(&self) -> Result<Vec<Installation>, BoxError> {
        let raw_url = format!("{base}/app/installations", base = BASE_URL);
        let url = reqwest::Url::parse(&raw_url)?;
        let lists: Vec<Vec<Installation>> = call::get_paged_preview(&self.client, url).await?;
        let installations = lists.into_iter().flatten().collect();
        Ok(installations)
    }

    pub async fn create_app_installation_access_token(
        &self,
        installation_id: i32,
    ) -> Result<InstallationAccessToken, BoxError> {
        let raw_url = format!(
            "{base}/app/installations/{installation_id}/access_tokens",
            base = BASE_URL,
            installation_id = installation_id,
        );
        let url = reqwest::Url::parse(&raw_url)?;
        let access_token = call::post_preview(&self.client, url).await?;
        Ok(access_token)
    }

    pub async fn get_installation_repositories(&self) -> Result<Vec<Repository>, BoxError> {
        let raw_url = format!("{base}/installation/repositories", base = BASE_URL);
        let url = reqwest::Url::parse(&raw_url)?;
        let lists: Vec<RepositoryList> = call::get_paged_preview(&self.client, url).await?;
        let repositories = lists
            .into_iter()
            .flat_map(|list| list.repositories)
            .collect();
        Ok(repositories)
    }

    pub async fn get_user(&self) -> Result<User, BoxError> {
        let raw_url = format!("{base}/user", base = BASE_URL,);
        let url = reqwest::Url::parse(&raw_url)?;
        let user = call::get(&self.client, url).await?;
        Ok(user)
    }

    pub async fn get_commits(
        &self,
        owner: &str,
        repo: &str,
        since: &str,
        until: &str,
    ) -> Result<Vec<Commit>, BoxError> {
        let raw_url = format!(
            "{base}/repos/{owner}/{repo}/commits",
            base = BASE_URL,
            owner = owner,
            repo = repo
        );
        let url = reqwest::Url::parse_with_params(&raw_url, &[("since", since), ("until", until)])?;
        let lists: Vec<Vec<Commit>> = call::get_paged(&self.client, url).await?;
        let commits = lists.into_iter().flatten().collect();
        Ok(commits)
    }

    pub async fn get_statuses(
        &self,
        owner: &str,
        repo: &str,
        git_ref: &str,
    ) -> Result<Vec<CommitStatus>, BoxError> {
        let raw_url = format!(
            "{base}/repos/{owner}/{repo}/commits/{git_ref}/statuses",
            base = BASE_URL,
            owner = owner,
            repo = repo,
            git_ref = git_ref
        );
        let url = reqwest::Url::parse(&raw_url)?;
        let lists: Vec<Vec<CommitStatus>> = call::get_paged(&self.client, url).await?;
        let statuses = lists.into_iter().flatten().collect();
        Ok(statuses)
    }
}
