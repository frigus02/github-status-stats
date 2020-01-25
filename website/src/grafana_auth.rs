use futures::future::join_all;
use github_client::Repository;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::Serialize;

type BoxError = Box<dyn std::error::Error>;

#[derive(Serialize)]
pub struct GitHubUser {
    pub id: i32,
    pub name: String,
    pub email: Option<String>,
    pub repositories: Vec<github_client::Repository>,
}

struct GrafanaUser {
    id: i32,
    login: String,
}

async fn get_repositories_and_map_error(
    client: &github_client::Client,
    installation: &github_client::Installation,
) -> Result<Vec<github_client::Repository>, String> {
    client
        .get_user_installation_repositories(installation.id)
        .await
        .map_err(|err| format!("{}", err))
}

pub async fn get_github_user(token: &str) -> Result<GitHubUser, BoxError> {
    let client = github_client::Client::new(token)?;
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

    Ok(GitHubUser {
        id: user.id,
        name: user.name,
        email: user.email,
        repositories,
    })
}

fn generate_random_password() -> String {
    thread_rng().sample_iter(&Alphanumeric).take(30).collect()
}

async fn ensure_grafana_user(
    client: &grafana_client::Client,
    github_user_id: i32,
    name: Option<String>,
    email: Option<String>,
) -> Result<GrafanaUser, BoxError> {
    let login = format!("{}", github_user_id);
    let user = client.lookup_user(&login).await?;
    let id = match user {
        Some(user) => user.id,
        None => {
            client
                .create_user(grafana_client::CreateUser {
                    login: login.clone(),
                    name,
                    email,
                    password: generate_random_password(),
                })
                .await?
                .id
        }
    };
    Ok(GrafanaUser { id, login })
}

async fn sync_repos_to_orgs(
    client: &grafana_client::Client,
    user: &GrafanaUser,
    repositories: Vec<github_client::Repository>,
) -> Result<(), BoxError> {
    let mut orgs = client.get_organizations_for_user(user.id).await?;

    for repo in repositories {
        let org_name = format!("{}", repo.id);
        match orgs.iter().position(|org| org.name == org_name) {
            Some(position) => {
                orgs.remove(position);
            }
            None => {
                let org = client.lookup_organization(&org_name).await?;
                if let Some(org) = org {
                    client
                        .add_user_to_organization(
                            org.id,
                            grafana_client::CreateOrganizationMembership {
                                login_or_email: user.login.clone(),
                                role: grafana_client::Role::Viewer,
                            },
                        )
                        .await?;
                }
            }
        }
    }

    for org in orgs {
        client
            .remove_user_from_organization(org.org_id, user.id)
            .await?;
    }

    Ok(())
}

pub async fn sync_user(
    github_token: &str,
    client: &grafana_client::Client,
) -> Result<String, BoxError> {
    let permissions = get_github_user(github_token).await?;
    let user = ensure_grafana_user(
        client,
        permissions.id,
        Some(permissions.name),
        permissions.email,
    )
    .await?;
    sync_repos_to_orgs(client, &user, permissions.repositories).await?;
    Ok(user.login)
}
