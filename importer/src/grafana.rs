use github_client::Repository;
use grafana_client::{
    Client, CreateDataSource, CreateOrUpdateDashboard, CreateOrganization, DataSource,
    DataSourceAccess, UpdateDataSource,
};
use log::info;
use stats::{grafana_org_name, influxdb_name};
use tokio::fs;

type BoxError = Box<dyn std::error::Error>;

fn assert_datasource_org(datasource: DataSource, org_id: i32) -> Result<DataSource, String> {
    if datasource.org_id == org_id {
        Ok(datasource)
    } else {
        Err(format!(
            "Datasource is not assigned to org {} (actual: {})",
            org_id, datasource.org_id
        ))
    }
}

async fn setup_datasource(
    client: &Client,
    datasource_name: &str,
    org_id: i32,
    repository: &Repository,
    influxdb_base_url: &str,
    influxdb_user: &str,
    influxdb_password: &str,
) -> Result<(), BoxError> {
    let datasource = match client.lookup_datasource(datasource_name).await? {
        Some(data_source) => data_source,
        None => {
            client
                .create_datasource(CreateDataSource {
                    name: datasource_name.to_owned(),
                    r#type: "influxdb".to_owned(),
                    access: DataSourceAccess::Proxy,
                    url: None,
                    password: None,
                    database: None,
                    user: None,
                    basic_auth: None,
                    basic_auth_user: None,
                    basic_auth_password: None,
                    with_credentials: None,
                    is_default: None,
                    json_data: None,
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
                UpdateDataSource {
                    name: datasource.name,
                    r#type: datasource.r#type,
                    access: datasource.access,
                    url: Some(influxdb_base_url.to_owned()),
                    password: None,
                    database: Some(influxdb_name(repository)),
                    user: Some(influxdb_user.to_owned()),
                    basic_auth: None,
                    basic_auth_user: None,
                    basic_auth_password: None,
                    with_credentials: None,
                    is_default: Some(true),
                    json_data: None,
                    secure_json_data: Some(
                        [("password".to_owned(), influxdb_password.to_owned())]
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

fn is_hidden_entry(entry: &tokio::fs::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map_or(false, |s| s.starts_with('.'))
}

async fn setup_dashboards(client: &Client, dashboards_path: &str) -> Result<(), BoxError> {
    let mut entries = fs::read_dir(dashboards_path).await?;
    while let Some(entry) = entries.next_entry().await? {
        if !is_hidden_entry(&entry) {
            info!("Creating dashboard {:?}", entry.path());
            let contents = fs::read(entry.path()).await?;
            let dashboard = serde_json::from_slice(&contents)?;
            client
                .create_or_update_dashboard(CreateOrUpdateDashboard {
                    dashboard,
                    user_id: None,
                    overwrite: Some(true),
                    message: None,
                    folder_id: None,
                    is_folder: None,
                })
                .await?;
        }
    }

    Ok(())
}

pub async fn setup(
    client: &Client,
    repository: &Repository,
    influxdb_base_url: &str,
    influxdb_user: &str,
    influxdb_password: &str,
    dashboards_path: &str,
) -> Result<(), BoxError> {
    info!("Grafana setup for {}", repository.full_name);

    let org_name = grafana_org_name(repository);
    let org_id = match client.lookup_organization(&org_name).await? {
        Some(org) => org.id,
        None => {
            client
                .create_organization(CreateOrganization { name: org_name })
                .await?
                .org_id
        }
    };
    client.switch_organization_context(org_id).await?;

    let datasource_name = "DB";
    setup_datasource(
        client,
        datasource_name,
        org_id,
        repository,
        influxdb_base_url,
        influxdb_user,
        influxdb_password,
    )
    .await?;

    setup_dashboards(client, dashboards_path).await?;

    Ok(())
}
