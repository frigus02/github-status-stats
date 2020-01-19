mod apps;
mod call;
mod models;
mod page_links;

pub use models::*;

const BASE_URL: &str = "https://api.github.com";
pub const USER_AGENT: &str = concat!("github-status-stats/", env!("CARGO_PKG_VERSION"));

type BoxError = Box<dyn std::error::Error>;

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
        let lists: Vec<Vec<Installation>> =
            call::get_paged_preview(&self.client, url, call::MACHINE_MAN_PREVIEW).await?;
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
        let access_token = call::post_preview(&self.client, url, call::MACHINE_MAN_PREVIEW).await?;
        Ok(access_token)
    }

    pub async fn get_installation_repositories(&self) -> Result<Vec<Repository>, BoxError> {
        let raw_url = format!("{base}/installation/repositories", base = BASE_URL);
        let url = reqwest::Url::parse(&raw_url)?;
        let lists: Vec<RepositoryList> =
            call::get_paged_preview(&self.client, url, call::MACHINE_MAN_PREVIEW).await?;
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
    ) -> Result<Vec<Commit>, BoxError> {
        let raw_url = format!(
            "{base}/repos/{owner}/{repo}/commits",
            base = BASE_URL,
            owner = owner,
            repo = repo
        );
        let url = reqwest::Url::parse_with_params(&raw_url, &[("since", since)])?;
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

    pub async fn get_check_runs(
        &self,
        owner: &str,
        repo: &str,
        git_ref: &str,
    ) -> Result<Vec<CheckRun>, BoxError> {
        let raw_url = format!(
            "{base}/repos/{owner}/{repo}/commits/{git_ref}/check-runs",
            base = BASE_URL,
            owner = owner,
            repo = repo,
            git_ref = git_ref
        );
        let url = reqwest::Url::parse(&raw_url)?;
        let lists: Vec<CheckRunList> =
            call::get_paged_preview(&self.client, url, call::ANTIOPE_PREVIEW).await?;
        let check_runs = lists.into_iter().flat_map(|list| list.check_runs).collect();
        Ok(check_runs)
    }
}
