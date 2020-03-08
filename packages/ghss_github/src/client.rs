use super::apps;
use super::call;
use super::models::*;
use super::{BASE_URL, USER_AGENT};
use chrono::{DateTime, FixedOffset};
use std::collections::HashMap;

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

    pub async fn get_user_installations(&self) -> Result<Vec<Installation>, BoxError> {
        let raw_url = format!("{base}/user/installations", base = BASE_URL,);
        let url = reqwest::Url::parse(&raw_url)?;
        let lists: Vec<InstallationList> =
            call::get_paged_preview(&self.client, url, call::MACHINE_MAN_PREVIEW).await?;
        let installations = lists
            .into_iter()
            .flat_map(|list| list.installations)
            .collect();
        Ok(installations)
    }

    pub async fn get_user_installation_repositories(
        &self,
        installation_id: i32,
    ) -> Result<Vec<Repository>, BoxError> {
        let raw_url = format!(
            "{base}/user/installations/{installation_id}/repositories",
            base = BASE_URL,
            installation_id = installation_id
        );
        let url = reqwest::Url::parse(&raw_url)?;
        let lists: Vec<RepositoryList> =
            call::get_paged_preview(&self.client, url, call::MACHINE_MAN_PREVIEW).await?;
        let repositories = lists
            .into_iter()
            .flat_map(|list| list.repositories)
            .collect();
        Ok(repositories)
    }

    pub async fn get_most_recent_commits(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<MostRecentCommit>, BoxError> {
        let raw_url = format!("{base}/graphql", base = BASE_URL);
        let url = reqwest::Url::parse(&raw_url)?;
        let body = GraphQLQuery {
            query: "query ($owner: String!, $name: String!) {
                repository(owner: $owner, name: $name) {
                  defaultBranchRef {
                    target {
                      ... on Commit {
                        history(first: 50) {
                          nodes {
                            oid
                            committedDate
                          }
                        }
                      }
                    }
                  }
                }
            }",
            variables: Some(
                [
                    (
                        "owner".to_owned(),
                        serde_json::Value::String(owner.to_owned()),
                    ),
                    (
                        "name".to_owned(),
                        serde_json::Value::String(repo.to_owned()),
                    ),
                ]
                .iter()
                .cloned()
                .collect(),
            ),
        };
        let GraphQLResponse::<GetMostRecentCommits> { data, errors } =
            call::post(&self.client, url, &body).await?;

        Ok(data
            .ok_or_else(|| format!("no data. error: {:?}", errors))?
            .repository
            .default_branch_ref
            .target
            .history
            .nodes
            .into_iter()
            .map(|node| MostRecentCommit {
                sha: node.oid,
                committed_date: node.committed_date,
            })
            .collect())
    }

    pub async fn get_commit_dates(
        &self,
        owner: &str,
        repo: &str,
        commit_shas: &[String],
    ) -> Result<Vec<DateTime<FixedOffset>>, BoxError> {
        let raw_url = format!("{base}/graphql", base = BASE_URL);
        let url = reqwest::Url::parse(&raw_url)?;

        let args = (0..commit_shas.len())
            .map(|i| format!(", $commit{}: GitObjectID", i))
            .collect::<Vec<_>>()
            .join("");
        let objects = (0..commit_shas.len())
            .map(|i| format!("_{i}: object(oid: $commit{i}) {{ ...dateField }}", i = i))
            .collect::<Vec<_>>()
            .join("");
        let query = format!(
            "query ($owner: String!, $name: String!{}) {{
              repository(owner: $owner, name: $name) {{
                {}
              }}
            }}

            fragment dateField on Commit {{
              committedDate
            }}",
            args, objects
        );
        let mut variables = HashMap::new();
        variables.insert(
            "owner".to_owned(),
            serde_json::Value::String(owner.to_owned()),
        );
        variables.insert(
            "name".to_owned(),
            serde_json::Value::String(repo.to_owned()),
        );
        for (i, commit_sha) in commit_shas.iter().enumerate() {
            variables.insert(
                format!("commit{}", i),
                serde_json::Value::String(commit_sha.to_owned()),
            );
        }

        let body = GraphQLQuery {
            query: &query,
            variables: Some(variables),
        };
        let GraphQLResponse::<GetCommitDates> { data, errors } =
            call::post(&self.client, url, &body).await?;
        let date_nodes = data
            .ok_or_else(|| format!("no data. error: {:?}", errors))?
            .repository;
        Ok((0..commit_shas.len())
            .map(|i| {
                date_nodes
                    .get(&format!("_{}", i))
                    .map(|node| node.committed_date)
                    .ok_or_else(|| format!("no result for {}", i))
            })
            .collect::<Result<_, _>>()?)
    }

    pub async fn get_statuses(
        &self,
        owner: &str,
        repo: &str,
        git_ref: &str,
    ) -> Result<Vec<CommitStatus>, BoxError> {
        let mut url = reqwest::Url::parse(BASE_URL)?;
        url.path_segments_mut()
            .map_err(|_| "cannot be base")?
            .extend(&["repos", owner, repo, "commits", git_ref, "statuses"]);
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
        let mut url = reqwest::Url::parse(BASE_URL)?;
        url.path_segments_mut()
            .map_err(|_| "cannot be base")?
            .extend(&["repos", owner, repo, "commits", git_ref, "check-runs"]);
        let lists: Vec<CheckRunList> =
            call::get_paged_preview(&self.client, url, call::ANTIOPE_PREVIEW).await?;
        let check_runs = lists.into_iter().flat_map(|list| list.check_runs).collect();
        Ok(check_runs)
    }
}
