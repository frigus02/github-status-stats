mod build;
mod hook;
mod import;

use build::{get_builds, get_builds_since};
use chrono::{Duration, Utc};
use github_client::Client;
use hook::get_status_hook_commits_since;
use import::get_last_import;
use once_cell::sync::Lazy;
use stats::{influxdb_name, Import};

type BoxError = Box<dyn std::error::Error>;

static GH_APP_ID: Lazy<String> = Lazy::new(|| std::env::var("GH_APP_ID").unwrap());
static GH_PRIVATE_KEY: Lazy<String> = Lazy::new(|| std::env::var("GH_PRIVATE_KEY").unwrap());

static INFLUXDB_BASE_URL: Lazy<String> = Lazy::new(|| std::env::var("INFLUXDB_BASE_URL").unwrap());
static INFLUXDB_USERNAME: Lazy<String> = Lazy::new(|| std::env::var("INFLUXDB_USERNAME").unwrap());
static INFLUXDB_PASSWORD: Lazy<String> = Lazy::new(|| std::env::var("INFLUXDB_PASSWORD").unwrap());

async fn import(
    influxdb_client: &influxdb_client::Client<'_>,
    mut points: Vec<influxdb_client::Point>,
) -> Result<(), BoxError> {
    points.push(
        Import {
            time: Utc::now(),
            points: points.len() as i64,
        }
        .into_point(),
    );
    influxdb_client.write(points).await
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let gh_app_client = Client::new_app_auth(&*GH_APP_ID, &*GH_PRIVATE_KEY)?;
    let installations = gh_app_client.get_app_installations().await?;
    for installation in installations {
        println!("Installation {}", installation.id);
        let token = gh_app_client
            .create_app_installation_access_token(installation.id)
            .await?;
        let gh_inst_client = Client::new(&token.token)?;
        let repositories = gh_inst_client.get_installation_repositories().await?;
        for repository in repositories {
            println!("Repository {}", repository.full_name);

            let influxdb_db = influxdb_name(&repository);
            let influxdb_client = influxdb_client::Client::new(
                &*INFLUXDB_BASE_URL,
                &influxdb_db,
                &*INFLUXDB_USERNAME,
                &*INFLUXDB_PASSWORD,
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
