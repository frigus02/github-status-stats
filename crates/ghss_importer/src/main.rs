mod build;
mod config;
mod store;

use build::{get_builds_from_commit_shas, get_most_recent_builds};
use config::Config;
use ghss_github::{Client, Repository};
use ghss_store_client::store_client::StoreClient;
use ghss_store_client::Code;
use opentelemetry::api::{
    Context, FutureExt, Key, StatusCode, TraceContextExt, TraceContextPropagator, Tracer,
};
use store::RepositoryImporter;

type BoxError = Box<dyn std::error::Error>;

async fn import_repository(
    gh_inst_client: &Client,
    store_client: &mut StoreClient<ghss_store_client::Channel>,
    repository: &Repository,
) -> Result<(), BoxError> {
    let cx = Context::current();
    let span = cx.span();

    let mut importer = RepositoryImporter::new(store_client, repository.id.to_string());

    let commits_since = importer.get_hooked_commits_since_last_import().await;
    match commits_since {
        Ok(commits_since) => {
            span.add_event(
                "log".into(),
                vec![Key::new("log.message").string("found last import; importing since then")],
            );
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
            span.add_event(
                "log".into(),
                vec![Key::new("log.message")
                    .string("first import; setup db and perform initial import")],
            );

            let (builds, commits) = get_most_recent_builds(&gh_inst_client, &repository).await?;
            importer.import(builds, commits).await?;
        }
        Err(status) => {
            span.add_event(
                "failed getting commits".into(),
                vec![Key::new("error.message").string(status.to_string())],
            );
        }
    }

    Ok(())
}

async fn import_installation(
    gh_app_client: &Client,
    store_client: &mut StoreClient<ghss_store_client::Channel>,
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
        import_repository(&gh_inst_client, store_client, &repository)
            .with_context(Context::current_with_span(span))
            .await?;
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
        import_installation(&gh_app_client, &mut store_client, installation.id)
            .with_context(Context::current_with_span(span))
            .await?;
    }

    Ok(())
}

fn init_tracer(agent_endpoint: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let provider = match agent_endpoint {
        Some(agent_endpoint) => {
            let exporter = opentelemetry_jaeger::Exporter::builder()
                .with_agent_endpoint(agent_endpoint.parse().unwrap())
                .with_process(opentelemetry_jaeger::Process {
                    service_name: "importer".to_string(),
                    tags: vec![],
                })
                .init()?;
            let batch = opentelemetry::sdk::BatchSpanProcessor::builder(
                exporter,
                tokio::spawn,
                tokio::time::interval,
            )
            .build();
            opentelemetry::sdk::Provider::builder()
                .with_batch_exporter(batch)
                .build()
        }
        None => {
            let exporter = opentelemetry::exporter::trace::stdout::Builder::default().init();
            opentelemetry::sdk::Provider::builder()
                .with_simple_exporter(exporter)
                .build()
        }
    };
    opentelemetry::global::set_provider(provider);

    let propagator = TraceContextPropagator::new();
    opentelemetry::global::set_http_text_propagator(propagator);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let config = config::load();

    init_tracer(config.otel_agent_endpoint.as_deref())?;

    let tracer = opentelemetry::global::tracer("importer");
    let span = tracer.start("import");
    let cx = Context::current_with_span(span);

    match import(config).with_context(cx.clone()).await {
        Ok(_) => Ok(()),
        Err(err) => {
            let span = cx.span();
            span.set_status(StatusCode::Internal, err.to_string());
            span.add_event(
                "import failed".into(),
                vec![Key::new("error.message").string(err.to_string())],
            );
            Err(format!(
                "Import {:032x} failed",
                span.span_context().trace_id().to_u128()
            )
            .into())
        }
    }
}
