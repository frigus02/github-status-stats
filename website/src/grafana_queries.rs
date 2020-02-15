use super::github_queries::{get_github_user, GitHubUser};
use futures::future::join_all;
use grafana_client::{Client, CreateOrganizationMembership, CreateUser, Organization, Role};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use stats::{grafana_org_name, grafana_user_login};

type BoxError = Box<dyn std::error::Error>;

struct GrafanaUser {
    id: i32,
    login: String,
}

pub struct MergedUser {
    pub github: github_client::User,
    pub repositories: Vec<MergedRepository>,
}

pub struct MergedRepository {
    pub github: github_client::Repository,
    pub grafana: Option<Organization>,
}

fn generate_random_password() -> String {
    thread_rng().sample_iter(&Alphanumeric).take(30).collect()
}

async fn ensure_grafana_user(
    client: &Client,
    login: String,
    name: Option<String>,
    email: Option<String>,
) -> Result<GrafanaUser, BoxError> {
    let user = client.lookup_user(&login).await?;
    let id = match user {
        Some(user) => user.id,
        None => {
            client
                .create_user(CreateUser {
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
    client: &Client,
    user: &GrafanaUser,
    repositories: Vec<MergedRepository>,
) -> Result<(), BoxError> {
    let mut orgs = client.get_organizations_for_user(user.id).await?;
    let repo_orgs: Vec<Organization> = repositories
        .into_iter()
        .filter_map(|repo| repo.grafana)
        .collect();

    for repo_org in repo_orgs {
        match orgs.iter().position(|org| org.org_id == repo_org.id) {
            Some(position) => {
                orgs.remove(position);
            }
            None => {
                client
                    .add_user_to_organization(
                        repo_org.id,
                        CreateOrganizationMembership {
                            login_or_email: user.login.clone(),
                            role: Role::Viewer,
                        },
                    )
                    .await?;
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

async fn get_repository_access_and_map_error(
    client: &Client,
    repository: github_client::Repository,
) -> Result<MergedRepository, String> {
    let org_name = grafana_org_name(&repository);
    let org = client
        .lookup_organization(&org_name)
        .await
        .map_err(|err| err.to_string())?;
    Ok(MergedRepository {
        github: repository,
        grafana: org,
    })
}

pub async fn get_user(github_token: &str, client: &Client) -> Result<MergedUser, BoxError> {
    let GitHubUser { user, repositories } = get_github_user(github_token).await?;
    let repositories = join_all(
        repositories
            .into_iter()
            .map(|repository| get_repository_access_and_map_error(client, repository)),
    )
    .await
    .into_iter()
    .collect::<Result<Vec<_>, _>>()?;
    Ok(MergedUser {
        github: user,
        repositories,
    })
}

pub async fn sync_user(github_token: &str, client: &Client) -> Result<String, BoxError> {
    let MergedUser {
        github: user,
        repositories,
    } = get_user(github_token, client).await?;
    let grafana_login = grafana_user_login(&user);
    let user = ensure_grafana_user(client, grafana_login, Some(user.name), user.email).await?;
    sync_repos_to_orgs(client, &user, repositories).await?;
    Ok(user.login)
}
