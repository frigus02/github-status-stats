use futures::future::join_all;
use ghss_github::{Client, Installation, Repository, User};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub struct GitHubUser {
    pub user: User,
    pub repositories: Vec<Repository>,
}

async fn get_repositories_and_map_error(
    client: &Client,
    installation: &Installation,
) -> Result<Vec<Repository>, String> {
    client
        .get_user_installation_repositories(installation.id)
        .await
        .map_err(|err| err.to_string())
}

pub async fn get_github_user(token: &str) -> Result<GitHubUser, BoxError> {
    let client = Client::new(token)?;
    let user = client.get_user().await?;
    let installations = client.get_user_installations().await?;
    let repositories = join_all(
        installations
            .iter()
            .map(|installation| get_repositories_and_map_error(&client, installation)),
    )
    .await
    .into_iter()
    .collect::<Result<Vec<_>, _>>()?
    .into_iter()
    .flatten()
    .collect::<Vec<Repository>>();

    Ok(GitHubUser { user, repositories })
}
