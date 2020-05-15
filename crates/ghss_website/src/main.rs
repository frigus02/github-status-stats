mod config;
mod cookie;
mod ctrlc;
mod github_hooks;
mod github_queries;
mod templates;
mod token;

use bytes::Bytes;
use config::with_config;
use ghss_store_client::{
    interval_aggregates_reply, query_client::QueryClient, store_client::StoreClient,
    total_aggregates_reply, AggregateFunction, BuildSource, Hook, IntervalAggregatesRequest,
    IntervalType, RecordHookRequest, TotalAggregatesRequest,
};
use ghss_tracing::register_new_tracing_root;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::convert::TryFrom;
use std::sync::Arc;
use templates::{
    with_templates, DashboardData, DashboardTemplate, IndexTemplate, RepositoryAccess,
};
use token::{optional_token, OptionalToken};
use tracing::{error, info, info_span};
use tracing_futures::Instrument;
use warp::{
    http::{Response, StatusCode, Uri},
    hyper::{self, header, service::Service, Body, Request},
    Filter,
};

type Config = Arc<config::Config>;
type Templates = Arc<templates::Templates<'static>>;

async fn index_route(
    token: OptionalToken,
    templates: Templates,
    config: Config,
) -> Result<impl warp::Reply, Infallible> {
    let reply: Box<dyn warp::Reply> = match token {
        OptionalToken::Some(user) => {
            let data = IndexTemplate::LoggedIn {
                user: user.name,
                repositories: user
                    .repositories
                    .into_iter()
                    .map(|repo| RepositoryAccess { name: repo.name })
                    .collect(),
                login_url: ghss_github::oauth::login_url(
                    &config.gh_client_id,
                    &config.gh_redirect_uri,
                    None,
                ),
            };
            let render = templates.render_index(&data);
            Box::new(warp::reply::html(render))
        }
        OptionalToken::Expired => {
            let login_url =
                ghss_github::oauth::login_url(&config.gh_client_id, &config.gh_redirect_uri, None);
            Box::new(warp::redirect::temporary(
                Uri::try_from(login_url).expect("Url to Uri"),
            ))
        }
        OptionalToken::None => {
            let data = IndexTemplate::Anonymous {
                login_url: ghss_github::oauth::login_url(
                    &config.gh_client_id,
                    &config.gh_redirect_uri,
                    None,
                ),
            };
            let render = templates.render_index(&data);
            Box::new(warp::reply::html(render))
        }
    };

    Ok(reply)
}

async fn dashboard_route(
    owner: String,
    repo: String,
    token: OptionalToken,
    templates: Templates,
    config: Config,
) -> Result<Box<dyn warp::Reply>, Infallible> {
    let name = format!("{}/{}", owner, repo);
    let reply: Box<dyn warp::Reply> = match token {
        OptionalToken::Some(user) => {
            let data = match user.repositories.into_iter().find(|r| r.name == name) {
                Some(repo) => DashboardData::Data {
                    repository_id: repo.id,
                },
                None => DashboardData::Error {
                    message: "Not found".to_string(),
                },
            };
            let render = templates.render_dashboard(&DashboardTemplate {
                user: user.name,
                repository_name: name,
                data,
            });
            Box::new(warp::reply::html(render))
        }
        OptionalToken::Expired | OptionalToken::None => {
            let login_url = ghss_github::oauth::login_url(
                &config.gh_client_id,
                &config.gh_redirect_uri,
                Some(path_to_state(format!("/d/{}/{}", owner, repo))),
            );
            Box::new(warp::redirect::temporary(
                Uri::try_from(login_url).expect("Url to Uri"),
            ))
        }
    };
    Ok(reply)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ApiQueryAggregateFunction {
    Avg,
    Count,
}

impl From<ApiQueryAggregateFunction> for AggregateFunction {
    fn from(function: ApiQueryAggregateFunction) -> Self {
        match function {
            ApiQueryAggregateFunction::Avg => Self::Avg,
            ApiQueryAggregateFunction::Count => Self::Count,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ApiQueryAggregate {
    column: String,
    function: ApiQueryAggregateFunction,
}

impl From<ApiQueryAggregate> for ghss_store_client::Column {
    fn from(aggregate: ApiQueryAggregate) -> Self {
        Self {
            name: aggregate.column,
            aggregation: AggregateFunction::from(aggregate.function) as i32,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ApiQueryInterval {
    Sparse,
    Detailed,
}

impl From<ApiQueryInterval> for IntervalType {
    fn from(interval: ApiQueryInterval) -> Self {
        match interval {
            ApiQueryInterval::Sparse => Self::Sparse,
            ApiQueryInterval::Detailed => Self::Detailed,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ApiQueryParams {
    repository: i32,
    table: String,
    aggregates: Vec<ApiQueryAggregate>,
    from: i64,
    to: i64,
    group_by: Vec<String>,
    interval: Option<ApiQueryInterval>,
}

#[derive(Debug, Serialize)]
struct ApiQueryRow {
    aggregates: Vec<i64>,
    groups: Vec<String>,
    timestamp: Option<i64>,
}

impl From<interval_aggregates_reply::Row> for ApiQueryRow {
    fn from(row: interval_aggregates_reply::Row) -> Self {
        Self {
            aggregates: row.aggregates,
            groups: row.groups,
            timestamp: Some(row.timestamp),
        }
    }
}

impl From<total_aggregates_reply::Row> for ApiQueryRow {
    fn from(row: total_aggregates_reply::Row) -> Self {
        Self {
            aggregates: row.aggregates,
            groups: row.groups,
            timestamp: None,
        }
    }
}

#[derive(Debug, Serialize)]
struct ApiQueryResponse {
    pub rows: Vec<ApiQueryRow>,
}

async fn api_query_route(
    params: ApiQueryParams,
    token: OptionalToken,
    config: Config,
) -> Result<Box<dyn warp::Reply>, Infallible> {
    let reply: Box<dyn warp::Reply> = match token {
        OptionalToken::Some(user)
            if user.repositories.iter().any(|r| r.id == params.repository) =>
        {
            let res: Result<ApiQueryResponse, Box<dyn std::error::Error>> = async {
                let mut client = QueryClient::connect(config.store_url.clone()).await?;
                let rows = match params.interval {
                    Some(interval) => {
                        let request = ghss_tracing::tonic::request(IntervalAggregatesRequest {
                            repository_id: params.repository.to_string(),
                            table: params.table,
                            columns: params.aggregates.into_iter().map(|c| c.into()).collect(),
                            timestamp_from: params.from,
                            timestamp_to: params.to,
                            group_by_columns: params.group_by,
                            interval_type: IntervalType::from(interval) as i32,
                        });
                        let response = client.get_interval_aggregates(request).await?;
                        response
                            .into_inner()
                            .rows
                            .into_iter()
                            .map(|r| r.into())
                            .collect()
                    }
                    None => {
                        let request = ghss_tracing::tonic::request(TotalAggregatesRequest {
                            repository_id: params.repository.to_string(),
                            table: params.table,
                            columns: params.aggregates.into_iter().map(|c| c.into()).collect(),
                            timestamp_from: params.from,
                            timestamp_to: params.to,
                            group_by_columns: params.group_by,
                        });
                        let response = client.get_total_aggregates(request).await?;
                        response
                            .into_inner()
                            .rows
                            .into_iter()
                            .map(|r| r.into())
                            .collect()
                    }
                };
                Ok(ApiQueryResponse { rows })
            }
            .await;
            match res {
                Ok(res) => Box::new(warp::reply::json(&res)),
                Err(err) => {
                    error!(err = %err, "query failed");
                    Box::new(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        _ => Box::new(StatusCode::UNAUTHORIZED),
    };
    Ok(reply)
}

async fn setup_authorized_route(
    info: ghss_github::oauth::AuthCodeQuery,
    config: Config,
) -> Result<Box<dyn warp::Reply>, Infallible> {
    let token = async {
        let github_token = ghss_github::oauth::exchange_code(
            &config.gh_client_id,
            &config.gh_client_secret.unsecure(),
            &config.gh_redirect_uri,
            &info,
        )
        .await?;
        token::generate(&github_token.access_token, config.token_secret.unsecure()).await
    }
    .await;

    match token {
        Ok(token) => {
            let redirect_path = info.state.map_or("/".to_owned(), path_from_state);
            Ok(Box::new(
                Response::builder()
                    .status(StatusCode::TEMPORARY_REDIRECT)
                    .header("location", redirect_path)
                    .header("set-cookie", cookie::set(config.cookie_name, &token))
                    .body(Body::empty())
                    .unwrap(),
            ))
        }
        Err(_) => Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR)),
    }
}

async fn logout_route(config: Config) -> Result<impl warp::Reply, Infallible> {
    Ok(Response::builder()
        .status(StatusCode::TEMPORARY_REDIRECT)
        .header("location", "/")
        .header("set-cookie", cookie::remove(config.cookie_name))
        .body(Body::empty())
        .unwrap())
}

async fn hooks_route(
    signature: String,
    event: String,
    body: Bytes,
    config: Config,
) -> Result<impl warp::Reply, Infallible> {
    let res: Result<(), Box<dyn std::error::Error>> = async {
        let payload = github_hooks::deserialize(
            signature,
            event,
            body,
            &config.gh_webhook_secret.unsecure(),
        )?;

        info!("Hook: {:?}", payload);
        match payload {
            github_hooks::Payload::CheckRun(check_run) => {
                let mut client = StoreClient::connect(config.store_url.clone()).await?;
                let request = ghss_tracing::tonic::request(RecordHookRequest {
                    repository_id: check_run.repository.id.to_string(),
                    hook: Some(Hook {
                        r#type: BuildSource::CheckRun as i32,
                        commit: check_run.check_run.head_sha.clone(),
                        timestamp: check_run.check_run.started_at.timestamp_millis(),
                    }),
                    build: Some(check_run.check_run.into()),
                });
                let _response = client.record_hook(request).await?;
            }
            github_hooks::Payload::GitHubAppAuthorization(_auth) => {}
            github_hooks::Payload::Installation => {}
            github_hooks::Payload::InstallationRepositories => {}
            github_hooks::Payload::Ping(_ping) => {}
            github_hooks::Payload::Status(status) => {
                let mut client = StoreClient::connect(config.store_url.clone()).await?;
                let request = ghss_tracing::tonic::request(RecordHookRequest {
                    repository_id: status.repository.id.to_string(),
                    hook: Some(Hook {
                        r#type: BuildSource::Status as i32,
                        commit: status.sha,
                        timestamp: status.created_at.timestamp_millis(),
                    }),
                    build: None,
                });
                let _response = client.record_hook(request).await?;
            }
        };

        Ok(())
    }
    .await;

    match res {
        Ok(_) => Ok(StatusCode::OK),
        Err(err) => {
            error!(error = %err, "hook failed");
            Ok(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub fn path_to_state(path: String) -> String {
    base64::encode(path)
}

pub fn path_from_state(state: String) -> String {
    let result: Result<String, Box<dyn std::error::Error>> = base64::decode(state)
        .map_err(|err| err.into())
        .and_then(|bytes| Ok(Uri::try_from(bytes.as_slice())?))
        .and_then(|uri| {
            if uri.scheme().is_some() {
                return Err("only path allowed but found scheme".into());
            }
            if uri.authority().is_some() {
                return Err("only path allowed but found authority".into());
            }

            Ok(uri
                .path_and_query()
                .ok_or("path required but not found")?
                .to_string())
        });
    result.unwrap_or_else(|_| "/".to_owned())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::load();
    let config = Arc::new(config);
    let templates = templates::load();
    let templates = Arc::new(templates);

    ghss_tracing::setup(ghss_tracing::Config {
        honeycomb_api_key: config.honeycomb_api_key.unsecure().to_owned(),
        honeycomb_dataset: config.honeycomb_dataset.clone(),
        service_name: "website",
    });

    let index = warp::get()
        .and(warp::path::end())
        .and(optional_token(
            config.cookie_name,
            config.token_secret.unsecure().into(),
        ))
        .and(with_templates(templates.clone()))
        .and(with_config(config.clone()))
        .and_then(index_route);

    let favicon = warp::get()
        .and(warp::path!("favicon.ico"))
        .and(warp::fs::file("static/favicon.ico"));

    let static_files = warp::path!("static" / ..).and(warp::fs::dir("static"));

    let dashboard = warp::get()
        .and(warp::path!("d" / String / String))
        .and(optional_token(
            config.cookie_name,
            config.token_secret.unsecure().into(),
        ))
        .and(with_templates(templates.clone()))
        .and(with_config(config.clone()))
        .and_then(dashboard_route);

    let api_query = warp::get()
        .and(warp::path!("api" / "query"))
        .and(warp::query())
        .and(optional_token(
            config.cookie_name,
            config.token_secret.unsecure().into(),
        ))
        .and(with_config(config.clone()))
        .and_then(api_query_route);

    let setup_authorized = warp::get()
        .and(warp::path!("setup" / "authorized"))
        .and(warp::query::<ghss_github::oauth::AuthCodeQuery>())
        .and(with_config(config.clone()))
        .and_then(setup_authorized_route);

    let logout = warp::get()
        .and(warp::path!("logout"))
        .and(with_config(config.clone()))
        .and_then(logout_route);

    let hooks = warp::post()
        .and(warp::path!("hooks"))
        .and(warp::header("X-Hub-Signature"))
        .and(warp::header("X-GitHub-Event"))
        .and(warp::body::bytes())
        .and(with_config(config.clone()))
        .and_then(hooks_route);

    let routes = index
        .or(favicon)
        .or(static_files)
        .or(dashboard)
        .or(api_query)
        .or(setup_authorized)
        .or(hooks)
        .or(logout);

    let warp_svc = warp::service(routes);
    let make_svc = hyper::service::make_service_fn(move |_| {
        let warp_svc = warp_svc.clone();
        async move {
            let svc = hyper::service::service_fn(move |req: Request<Body>| {
                let mut warp_svc = warp_svc.clone();
                async move {
                    let span = info_span!(
                        "request",
                        method = req.method().as_str(),
                        path = req.uri().path(),
                        user_agent = req
                            .headers()
                            .get(header::USER_AGENT)
                            .map(|v| v.to_str().expect("user agent to string"))
                            .unwrap_or(""),
                        user_id = tracing::field::Empty,
                        status = tracing::field::Empty,
                        duration_ms = tracing::field::Empty,
                    );
                    {
                        // TODO: This seems weird. Need to understand why that's
                        // necessary or how to do it better.
                        let _guard = span.enter();
                        register_new_tracing_root();
                    }

                    let started = std::time::Instant::now();
                    let res = warp_svc.call(req).instrument(span.clone()).await;
                    let duration_ms = (std::time::Instant::now() - started).as_millis();

                    span.record("duration_ms", &duration_ms.to_string().as_str());

                    let _guard = span.enter();
                    match res.as_ref() {
                        Ok(res) => {
                            span.record("status", &res.status().as_str());
                            info!("request finished");
                        }
                        Err(err) => {
                            error!(error = %err, "request failed");
                        }
                    };

                    res
                }
            });
            Ok::<_, Infallible>(svc)
        }
    });

    hyper::Server::bind(&([0, 0, 0, 0], 8888).into())
        .serve(make_svc)
        .with_graceful_shutdown(async {
            ctrlc::ctrl_c().await;
        })
        .await?;

    Ok(())
}
