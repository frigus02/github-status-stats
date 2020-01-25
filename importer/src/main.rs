#[macro_use]
extern crate lazy_static;

mod build;
mod hook;
mod import;

use build::{get_builds, get_builds_since};
use chrono::{Duration, Utc};
use github_client::{Client, Repository};
use hook::get_status_hook_commits_since;
use import::get_last_import;
use log::info;
use secstr::SecUtf8;
use stats::{grafana_org_name, influxdb_name, Import};

type BoxError = Box<dyn std::error::Error>;

lazy_static! {
    static ref GH_APP_ID: String = std::env::var("GH_APP_ID").unwrap();
    static ref GH_PRIVATE_KEY: SecUtf8 = SecUtf8::from(std::env::var("GH_PRIVATE_KEY").unwrap());
    static ref INFLUXDB_BASE_URL: String = std::env::var("INFLUXDB_BASE_URL").unwrap();
    static ref INFLUXDB_USERNAME: String = std::env::var("INFLUXDB_USERNAME").unwrap();
    static ref INFLUXDB_PASSWORD: SecUtf8 =
        SecUtf8::from(std::env::var("INFLUXDB_PASSWORD").unwrap());
    static ref GRAFANA_BASE_URL: String = std::env::var("GRAFANA_BASE_URL").unwrap();
    static ref GRAFANA_ADMIN_USERNAME: String = std::env::var("GRAFANA_ADMIN_USERNAME").unwrap();
    static ref GRAFANA_ADMIN_PASSWORD: SecUtf8 =
        SecUtf8::from(std::env::var("GRAFANA_ADMIN_PASSWORD").unwrap());
}

async fn import(
    influxdb_client: &influxdb_client::Client<'_>,
    mut points: Vec<influxdb_client::Point>,
) -> Result<(), BoxError> {
    info!("Import {} points", points.len());
    points.push(
        Import {
            time: Utc::now(),
            points: points.len() as i64,
        }
        .into_point(),
    );
    influxdb_client.write(points).await
}

fn assert_datasource_org(
    datasource: grafana_client::DataSource,
    org_id: i32,
) -> Result<grafana_client::DataSource, String> {
    if datasource.org_id == org_id {
        Ok(datasource)
    } else {
        Err(format!(
            "Datasource is not assigned to org {} (actual: {})",
            org_id, datasource.org_id
        ))
    }
}

async fn setup_grafana(
    client: &grafana_client::Client,
    repository: &Repository,
) -> Result<(), BoxError> {
    let org_name = grafana_org_name(repository);
    let org_id = match client.lookup_organization(&org_name).await? {
        Some(org) => org.id,
        None => {
            client
                .create_organization(grafana_client::CreateOrganization { name: org_name })
                .await?
                .org_id
        }
    };
    client.switch_organization_context(org_id).await?;

    let datasource_name = "DB".to_owned();
    let datasource = match client.lookup_datasource(&datasource_name).await? {
        Some(data_source) => data_source,
        None => {
            client
                .create_datasource(grafana_client::CreateDataSource {
                    name: datasource_name,
                    r#type: "influxdb".to_owned(),
                    access: grafana_client::DataSourceAccess::Proxy,
                    url: None,
                    password: None,
                    database: None,
                    user: None,
                    basic_auth: None,
                    basic_auth_user: None,
                    basic_auth_password: None,
                    with_credentials: None,
                    is_default: None,
                    secure_json_data: None,
                })
                .await?
                .datasource
        }
    };
    let datasource = assert_datasource_org(datasource, org_id)?;
    if datasource.url.is_empty() {
        client
            .update_datasource(
                datasource.id,
                grafana_client::UpdateDataSource {
                    name: datasource.name,
                    r#type: datasource.r#type,
                    access: datasource.access,
                    url: Some(INFLUXDB_BASE_URL.clone()),
                    password: None,
                    database: Some(influxdb_name(repository)),
                    user: Some(INFLUXDB_USERNAME.clone()),
                    basic_auth: None,
                    basic_auth_user: None,
                    basic_auth_password: None,
                    with_credentials: None,
                    is_default: Some(true),
                    secure_json_data: Some(
                        [(
                            "password".to_owned(),
                            INFLUXDB_PASSWORD.unsecure().to_owned(),
                        )]
                        .iter()
                        .cloned()
                        .collect(),
                    ),
                    version: Some(datasource.version),
                },
            )
            .await?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    env_logger::init();

    let gh_app_client = Client::new_app_auth(&*GH_APP_ID, &*GH_PRIVATE_KEY.unsecure())?;

    let grafana_client = grafana_client::Client::new(
        GRAFANA_BASE_URL.clone(),
        &*GRAFANA_ADMIN_USERNAME,
        &*GRAFANA_ADMIN_PASSWORD.unsecure(),
    )?;

    let installations = gh_app_client.get_app_installations().await?;
    for installation in installations {
        info!("Installation {}", installation.id);
        let token = gh_app_client
            .create_app_installation_access_token(installation.id)
            .await?;
        let gh_inst_client = Client::new(&token.token)?;
        let repositories = gh_inst_client.get_installation_repositories().await?;
        for repository in repositories {
            info!("Repository {}", repository.full_name);

            let influxdb_db = influxdb_name(&repository);
            let influxdb_client = influxdb_client::Client::new(
                &*INFLUXDB_BASE_URL,
                &influxdb_db,
                &*INFLUXDB_USERNAME,
                &*INFLUXDB_PASSWORD.unsecure(),
            )?;

            let last_import = get_last_import(&influxdb_client).await?;
            if let Some(last_import) = last_import {
                let commit_shas =
                    get_status_hook_commits_since(&influxdb_client, &last_import).await?;
                if !commit_shas.is_empty() {
                    let points = get_builds(&gh_inst_client, &repository, commit_shas).await?;
                    import(&influxdb_client, points).await?;
                }
            } else {
                setup_grafana(&grafana_client, &repository).await?;
                influxdb_client
                    .query(&format!("CREATE DATABASE {}", influxdb_db))
                    .await?;
                let points = get_builds_since(
                    &gh_inst_client,
                    &repository,
                    &(Utc::now() - Duration::weeks(1)),
                )
                .await?;
                import(&influxdb_client, points).await?;
            }
        }
    }

    Ok(())
}
