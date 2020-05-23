mod build;
mod config;
mod store;

use build::{get_builds_from_commit_shas, get_most_recent_builds};
use config::Config;
use ghss_github::Client;
use ghss_store_client::store_client::StoreClient;
use ghss_store_client::Code;
use ghss_tracing::register_new_tracing_root;
use store::RepositoryImporter;
use tracing::{error, info, info_span};

type BoxError = Box<dyn std::error::Error>;

#[allow(clippy::cognitive_complexity)]
async fn run(config: Config) -> Result<(), BoxError> {
    let gh_app_client = Client::new_app_auth(&config.gh_app_id, &config.gh_private_key.unsecure())?;
    let mut store_client = StoreClient::connect(config.store_url).await?;

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

            let mut importer =
                RepositoryImporter::new(&mut store_client, repository.id.to_string());

            let commits_since = importer.get_hooked_commits_since_last_import().await;
            match commits_since {
                Ok(commits_since) => {
                    info!("found last import; importing since then");
                    let commit_shas: Vec<String> = commits_since
                        .into_inner()
                        .commits
                        .into_iter()
                        .map(|commit| commit.commit)
                        .collect();
                    if !commit_shas.is_empty() {
                        let (builds, commits) =
                            get_builds_from_commit_shas(&gh_inst_client, &repository, commit_shas)
                                .await?;
                        importer.import(builds, commits).await?;
                    }
                }
                Err(status) if status.code() == Code::FailedPrecondition => {
                    info!("first import; setup db and perform initial import");

                    let (builds, commits) =
                        get_most_recent_builds(&gh_inst_client, &repository).await?;
                    importer.import(builds, commits).await?;
                }
                Err(status) => {
                    error!(error = %status, "failed getting commits");
                }
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let config = config::load();

    ghss_tracing::setup(ghss_tracing::Config {
        honeycomb_api_key: config.honeycomb_api_key.unsecure().to_owned(),
        honeycomb_dataset: config.honeycomb_dataset.clone(),
        service_name: "importer",
    });

    let res = async {
        let span = info_span!("import");
        let _guard = span.enter();
        register_new_tracing_root();

        match run(config).await {
            Ok(_) => Ok(()),
            Err(err) => {
                error!(error = %err, "import failed");
                Err(format!(
                    "Import {} failed",
                    span.id().map_or(0, |id| id.into_u64())
                ))
            }
        }
    }
    .await;

    ghss_tracing::flush().await;

    res
}
