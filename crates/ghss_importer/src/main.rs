mod build;
mod config;
mod store;

use build::{get_builds_from_commit_shas, get_most_recent_builds};
use config::Config;
use ghss_github::{Client, Repository};
use ghss_store_client::StoreClient;
use ghss_store_client::Code;
use ghss_tracing::{init_tracer, log_event};
use opentelemetry::api::{Context, FutureExt, Key, StatusCode, TraceContextExt, Tracer};
use store::RepositoryImporter;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

async fn import_repository(
    gh_inst_client: &Client,
    store_client: &mut StoreClient,
    repository: &Repository,
) -> Result<(), BoxError> {
    let mut importer = RepositoryImporter::new(store_client, repository.id.to_string());

    let commits_since = importer.get_hooked_commits_since_last_import().await;
    match commits_since {
        Ok(commits_since) => {
            log_event("found last import; importing since then".into());
            let commit_shas: Vec<String> = commits_since
                .into_inner()
                .commits
                .into_iter()
                .map(|commit| commit.commit)
                .collect();
            if !commit_shas.is_empty() {
                let (builds, commits) =
                    get_builds_from_commit_shas(gh_inst_client, repository, commit_shas).await?;
                importer.import(builds, commits).await?;
            }
        }
        Err(status) if status.code() == Code::FailedPrecondition => {
            log_event("first import; setup db and perform initial import".into());
            let (builds, commits) = get_most_recent_builds(&gh_inst_client, &repository).await?;
            importer.import(builds, commits).await?;
        }
        Err(status) => {
            return Err(status.into());
        }
    }

    Ok(())
}

async fn import_installation(
    gh_app_client: &Client,
    store_client: &mut StoreClient,
    installation_id: i32,
) -> Result<(), BoxError> {
    let tracer = opentelemetry::global::tracer("importer");
    let token = gh_app_client
        .create_app_installation_access_token(installation_id)
        .await?;
    let gh_inst_client = Client::new(&token.token)?;
    let repositories = gh_inst_client.get_installation_repositories().await?;
    for repository in repositories {
        let span = tracer
            .span_builder("repository")
            .with_attributes(vec![
                Key::new("repository.id").i64(repository.id.into()),
                Key::new("repository.full_name").string(repository.full_name.clone()),
            ])
            .start(&tracer);
        let cx = Context::current_with_span(span);
        let res = import_repository(&gh_inst_client, store_client, &repository)
            .with_context(cx.clone())
            .await;
        if let Err(err) = res {
            let span = cx.span();
            span.set_status(StatusCode::Internal, err.to_string());
            span.set_attribute(Key::new("error").string(err.to_string()));
        }
    }
    Ok(())
}

async fn import(config: Config) -> Result<(), BoxError> {
    let tracer = opentelemetry::global::tracer("importer");
    let mut store_client = StoreClient::connect(config.store_url).await?;
    let gh_app_client = Client::new_app_auth(&config.gh_app_id, &config.gh_private_key.unsecure())?;
    let installations = gh_app_client.get_app_installations().await?;
    for installation in installations {
        let span = tracer
            .span_builder("installation")
            .with_attributes(vec![Key::new("installation.id").i64(installation.id.into())])
            .start(&tracer);
        let cx = Context::current_with_span(span);
        let res = import_installation(&gh_app_client, &mut store_client, installation.id)
            .with_context(cx.clone())
            .await;
        if let Err(err) = res {
            let span = cx.span();
            span.set_status(StatusCode::Internal, err.to_string());
            span.set_attribute(Key::new("error").string(err.to_string()));
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let config = config::load();

    init_tracer("importer", config.otel_agent_endpoint.as_deref())?;

    let tracer = opentelemetry::global::tracer("importer");
    let span = tracer.start("import");
    let cx = Context::current_with_span(span);

    match import(config).with_context(cx.clone()).await {
        Ok(_) => Ok(()),
        Err(err) => {
            let span = cx.span();
            span.set_status(StatusCode::Internal, err.to_string());
            span.set_attribute(Key::new("error").string(err.to_string()));
            Err(format!(
                "Import {:032x} failed",
                span.span_context().trace_id().to_u128()
            )
            .into())
        }
    }
}
