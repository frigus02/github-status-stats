use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

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
#[serde(rename_all = "snake_case")]
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
#[serde(rename_all = "snake_case")]
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
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
    pub single_file_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstallationList {
    pub total_count: i32,
    pub installations: Vec<Installation>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstallationAccessToken {
    pub token: String,
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
    pub created_at: DateTime<FixedOffset>,
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
pub struct Commit {
    pub sha: String,
    pub node_id: String,
    pub commit: CommitCommit,
    pub url: String,
    pub html_url: String,
    pub comments_url: String,
    pub author: Account,
    pub committer: Account,
    //"parents": []
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
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
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct License {
    pub key: String,
    pub name: String,
    pub spdx_id: String,
    pub url: Option<String>,
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
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
    pub pushed_at: DateTime<FixedOffset>,
    pub git_url: String,
    pub ssh_url: String,
    pub clone_url: String,
    pub svn_url: String,
    pub homepage: Option<String>,
    pub size: i32,
    pub stargazers_count: i32,
    pub watchers_count: i32,
    pub language: Option<String>,
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
    pub repository_selection: Option<String>,
    pub repositories: Vec<Repository>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckRunOutput {
    pub title: Option<String>,
    pub summary: Option<String>,
    pub text: Option<String>,
    pub annotations_count: i32,
    pub annotations_url: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CheckRunStatus {
    Queued,
    InProgress,
    Completed,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CheckRunConclusion {
    Success,
    Failure,
    Neutral,
    Cancelled,
    TimedOut,
    ActionRequired,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckRun {
    pub id: i32,
    pub head_sha: String,
    pub node_id: String,
    pub external_id: String,
    pub url: String,
    pub html_url: String,
    pub details_url: String,
    pub status: CheckRunStatus,
    pub conclusion: Option<CheckRunConclusion>,
    pub started_at: DateTime<FixedOffset>,
    pub completed_at: Option<DateTime<FixedOffset>>,
    pub output: CheckRunOutput,
    pub name: String,
    // "check_suite": { "id": 5 },
    // "app": { ... },
    // "pull_requests": [ { ... } ]
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckRunList {
    pub total_count: i32,
    pub check_runs: Vec<CheckRun>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CheckRunEventAction {
    Created,
    Completed,
    Rerequested,
    RequestedAction,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CheckRunEvent {
    pub action: CheckRunEventAction,
    pub check_run: CheckRun,
    pub repository: Repository,
    pub sender: Account,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HookConfig {
    pub content_type: String,
    pub insecure_ssl: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Hook {
    pub r#type: String,
    pub id: i32,
    pub name: String,
    pub active: bool,
    pub events: Vec<String>,
    pub config: HookConfig,
    pub updated_at: DateTime<FixedOffset>,
    pub created_at: DateTime<FixedOffset>,
    pub app_id: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PingEvent {
    pub zen: String,
    pub hook_id: i32,
    pub hook: Hook,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitCommitUser {
    pub name: String,
    pub email: String,
    pub date: DateTime<FixedOffset>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitCommitTree {
    pub sha: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitCommitVerification {
    pub verified: bool,
    pub reason: String,
    pub signature: Option<String>,
    pub payload: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitCommit {
    pub author: CommitCommitUser,
    pub committer: CommitCommitUser,
    pub message: String,
    pub tree: CommitCommitTree,
    pub url: String,
    pub comment_count: i32,
    pub verification: CommitCommitVerification,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BranchCommit {
    pub sha: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Branch {
    pub name: String,
    pub commit: BranchCommit,
    pub protected: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusEvent {
    pub id: i32,
    pub sha: String,
    pub name: String,
    pub target_url: Option<String>,
    pub context: String,
    pub description: Option<String>,
    pub state: String,
    pub commit: Commit,
    pub branches: Vec<Branch>,
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
    pub repository: Repository,
    pub sender: Account,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubAppAuthorizationEvent {
    pub action: String,
    pub sender: Account,
}
