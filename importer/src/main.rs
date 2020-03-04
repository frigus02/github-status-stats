#[macro_use]
extern crate lazy_static;

mod build;
mod influxdb;

use build::{get_builds, get_most_recent_builds};
use github_client::Client;
use influxdb::{get_last_import, get_status_hook_commits_since, import};
use secstr::SecUtf8;
use stats::{influxdb_name, influxdb_read_user};
use tracing::{info, info_span};

type BoxError = Box<dyn std::error::Error>;

lazy_static! {
    static ref GH_APP_ID: String = std::env::var("GH_APP_ID").unwrap();
    static ref GH_PRIVATE_KEY: SecUtf8 = SecUtf8::from(std::env::var("GH_PRIVATE_KEY").unwrap());
    static ref INFLUXDB_BASE_URL: String = std::env::var("INFLUXDB_BASE_URL").unwrap();
    static ref INFLUXDB_ADMIN_USERNAME: String = std::env::var("INFLUXDB_ADMIN_USERNAME").unwrap();
    static ref INFLUXDB_ADMIN_PASSWORD: SecUtf8 =
        SecUtf8::from(std::env::var("INFLUXDB_ADMIN_PASSWORD").unwrap());
    static ref INFLUXDB_READ_PASSWORD: SecUtf8 =
        SecUtf8::from(std::env::var("INFLUXDB_READ_PASSWORD").unwrap());
    static ref HONEYCOMB_API_KEY: SecUtf8 =
        SecUtf8::from(std::env::var("HONEYCOMB_API_KEY").expect("env HONEYCOMB_API_KEY"));
    static ref HONEYCOMB_DATASET: String =
        std::env::var("HONEYCOMB_DATASET").expect("env HONEYCOMB_DATASET");
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    tracing::setup(tracing::Config {
        honeycomb_api_key: HONEYCOMB_API_KEY.unsecure().to_owned(),
        honeycomb_dataset: HONEYCOMB_DATASET.clone(),
        service_name: "importer".to_owned(),
    });

    let span = info_span!("import", import_id = %tracing::uuid());
    let _guard = span.enter();

    let gh_app_client = Client::new_app_auth(&*GH_APP_ID, &*GH_PRIVATE_KEY.unsecure())?;

    let installations = gh_app_client.get_app_installations().await?;
    for installation in installations {
        let span = info_span!("installation", installation.id);
        let _guard = span.enter();

        let token = gh_app_client
            .create_app_installation_access_token(installation.id)
            .await?;
        let gh_inst_client = Client::new(&token.token)?;
        let repositories = gh_inst_client.get_installation_repositories().await?;
        for repository in repositories {
            let span = info_span!("repository", repository.id);
            let _guard = span.enter();

            info!(%repository.full_name, "start importing repository");

            let influxdb_db = influxdb_name(&repository);
            let influxdb_client = influxdb_client::Client::new(
                &*INFLUXDB_BASE_URL,
                &influxdb_db,
                &*INFLUXDB_ADMIN_USERNAME,
                &*INFLUXDB_ADMIN_PASSWORD.unsecure(),
            )?;
            let influxdb_read_user = influxdb_read_user(&repository);

            let last_import = get_last_import(&influxdb_client).await?;
            if let Some(last_import) = last_import {
                info!(
                    repository.last_import = %last_import,
                    "found last import; importing since then"
                );

                let commit_shas =
                    get_status_hook_commits_since(&influxdb_client, &last_import).await?;
                if !commit_shas.is_empty() {
                    let points = get_builds(&gh_inst_client, &repository, commit_shas).await?;
                    import(&influxdb_client, points).await?;
                }
            } else {
                info!("first import; setup db and perform initial import");

                influxdb::setup(
                    &influxdb_client,
                    &influxdb_db,
                    &influxdb_read_user,
                    &*INFLUXDB_READ_PASSWORD.unsecure(),
                )
                .await?;
                let points = get_most_recent_builds(&gh_inst_client, &repository).await?;
                import(&influxdb_client, points).await?;
            }
        }
    }

    tracing::flush().await;

    Ok(())
}
