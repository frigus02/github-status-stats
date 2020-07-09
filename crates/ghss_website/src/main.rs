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
    query_client::QueryClient, store_client::StoreClient, AggregateFunction, BuildSource, Hook,
    IntervalAggregatesRequest, IntervalType, RecordHookRequest, TotalAggregatesRequest,
};
use opentelemetry::api::{
    Context, FutureExt, Key, SpanKind, TraceContextExt, TraceContextPropagator, Tracer,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::Infallible;
use std::convert::TryFrom;
use std::sync::Arc;
use templates::{
    with_templates, DashboardData, DashboardTemplate, IndexTemplate, RepositoryAccess,
};
use token::{optional_token, OptionalToken};
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

#[derive(Debug)]
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

#[derive(Debug)]
struct ApiQueryColumn {
    name: String,
    agg_func: ApiQueryAggregateFunction,
}

impl From<ApiQueryColumn> for ghss_store_client::Column {
    fn from(aggregate: ApiQueryColumn) -> Self {
        Self {
            name: aggregate.name,
            agg_func: AggregateFunction::from(aggregate.agg_func) as i32,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ApiQueryIntervalType {
    Sparse,
    Detailed,
}

impl From<ApiQueryIntervalType> for IntervalType {
    fn from(interval: ApiQueryIntervalType) -> Self {
        match interval {
            ApiQueryIntervalType::Sparse => Self::Sparse,
            ApiQueryIntervalType::Detailed => Self::Detailed,
        }
    }
}

struct VecApiQueryAggregateVisitor;

impl<'de> serde::de::Visitor<'de> for VecApiQueryAggregateVisitor {
    type Value = Vec<ApiQueryColumn>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a comma separated list of aggregate functions and columns")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let re = Regex::new(r"(avg|count)\(([A-Za-z0-9_]+)\)").unwrap();
        v.split(',')
            .filter(|part| !part.is_empty())
            .map(|part| {
                re.captures(part)
                    .ok_or_else(|| E::custom("invalid aggregate value"))
                    .map(|cap| ApiQueryColumn {
                        name: cap[2].into(),
                        agg_func: match &cap[1] {
                            "avg" => ApiQueryAggregateFunction::Avg,
                            "count" => ApiQueryAggregateFunction::Count,
                            _ => panic!("invalid aggregate function"),
                        },
                    })
            })
            .collect()
    }
}

fn deserialize_api_query_aggregates<'de, D>(
    deserializer: D,
) -> Result<Vec<ApiQueryColumn>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    deserializer.deserialize_str(VecApiQueryAggregateVisitor)
}

struct OptionVecStringVisitor;

impl<'de> serde::de::Visitor<'de> for OptionVecStringVisitor {
    type Value = Option<Vec<String>>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("null or a comma separated list of strings")
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        deserializer.deserialize_str(self)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Some(
            v.split(',')
                .filter(|part| !part.is_empty())
                .map(|part| part.into())
                .collect(),
        ))
    }
}

fn deserialize_option_strings<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    deserializer.deserialize_option(OptionVecStringVisitor)
}

#[derive(Debug, Deserialize)]
struct ApiQueryParams {
    repository: i32,
    table: String,
    #[serde(deserialize_with = "deserialize_api_query_aggregates")]
    columns: Vec<ApiQueryColumn>,
    since: i64,
    until: i64,
    #[serde(default, deserialize_with = "deserialize_option_strings")]
    group_by: Option<Vec<String>>,
    interval: Option<ApiQueryIntervalType>,
}

#[derive(Debug, Serialize)]
struct ApiQueryResponseSeries {
    tags: Vec<String>,
    values: Vec<Option<Vec<f64>>>,
}

#[derive(Debug, Serialize)]
struct ApiQueryResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamps: Option<Vec<i64>>,
    series: Vec<ApiQueryResponseSeries>,
}

async fn api_query_route(
    params: ApiQueryParams,
    token: OptionalToken,
    mut client: QueryClient<ghss_store_client::Channel>,
) -> Result<Box<dyn warp::Reply>, Infallible> {
    let reply: Box<dyn warp::Reply> = match token {
        OptionalToken::Some(user)
            if user.repositories.iter().any(|r| r.id == params.repository) =>
        {
            let res: Result<ApiQueryResponse, Box<dyn std::error::Error>> = async {
                let response = match params.interval {
                    Some(interval) => {
                        let request_cx =
                            ghss_store_client::request_context("get_interval_aggregates");
                        let request = ghss_store_client::request(
                            IntervalAggregatesRequest {
                                repository_id: params.repository.to_string(),
                                table: params.table,
                                columns: params.columns.into_iter().map(|c| c.into()).collect(),
                                since: params.since,
                                until: params.until,
                                group_by: params.group_by.unwrap_or_default(),
                                interval: IntervalType::from(interval) as i32,
                            },
                            &request_cx,
                        );
                        let response = client
                            .get_interval_aggregates(request)
                            .with_context(request_cx)
                            .await?
                            .into_inner();
                        let mut timestamps = Vec::new();
                        let mut series = HashMap::new();
                        for row in response.rows {
                            if timestamps.last() != Some(&row.timestamp) {
                                timestamps.push(row.timestamp);
                            }

                            let values: &mut Vec<Option<Vec<f64>>> =
                                series.entry(row.groups).or_default();
                            values.resize(timestamps.len() - 1, None);
                            values.push(Some(row.values));
                        }
                        for values in series.values_mut() {
                            values.resize(timestamps.len(), None);
                        }

                        ApiQueryResponse {
                            timestamps: Some(timestamps),
                            series: series
                                .into_iter()
                                .map(|(tags, values)| ApiQueryResponseSeries { tags, values })
                                .collect(),
                        }
                    }
                    None => {
                        let request_cx = ghss_store_client::request_context("get_total_aggregates");
                        let request = ghss_store_client::request(
                            TotalAggregatesRequest {
                                repository_id: params.repository.to_string(),
                                table: params.table,
                                columns: params.columns.into_iter().map(|c| c.into()).collect(),
                                since: params.since,
                                until: params.until,
                                group_by: params.group_by.unwrap_or_default(),
                            },
                            &request_cx,
                        );
                        let response = client
                            .get_total_aggregates(request)
                            .with_context(request_cx)
                            .await?
                            .into_inner();
                        let mut series = HashMap::new();
                        for row in response.rows {
                            series.insert(row.groups, vec![Some(row.values)]);
                        }

                        ApiQueryResponse {
                            timestamps: None,
                            series: series
                                .into_iter()
                                .map(|(tags, values)| ApiQueryResponseSeries { tags, values })
                                .collect(),
                        }
                    }
                };
                Ok(response)
            }
            .await;
            match res {
                Ok(res) => Box::new(warp::reply::json(&res)),
                Err(err) => {
                    let cx = Context::current();
                    cx.span().add_event(
                        "query failed".into(),
                        vec![Key::new("error.message").string(err.to_string())],
                    );
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
    mut client: StoreClient<ghss_store_client::Channel>,
) -> Result<impl warp::Reply, Infallible> {
    let cx = Context::current();
    let res: Result<(), Box<dyn std::error::Error>> = async {
        let payload = github_hooks::deserialize(
            signature,
            event,
            body,
            &config.gh_webhook_secret.unsecure(),
        )?;

        cx.span().add_event(
            "hook".into(),
            vec![Key::new("payload").string(format!("{:?}", payload))],
        );
        match payload {
            github_hooks::Payload::CheckRun(check_run) => {
                let request_cx = ghss_store_client::request_context("record_hook");
                let request = ghss_store_client::request(
                    RecordHookRequest {
                        repository_id: check_run.repository.id.to_string(),
                        hook: Some(Hook {
                            r#type: BuildSource::CheckRun as i32,
                            commit: check_run.check_run.head_sha.clone(),
                            timestamp: check_run.check_run.started_at.timestamp_millis(),
                        }),
                        build: Some(check_run.check_run.into()),
                    },
                    &request_cx,
                );
                let _response = client.record_hook(request).with_context(request_cx).await?;
            }
            github_hooks::Payload::GitHubAppAuthorization(_auth) => {}
            github_hooks::Payload::Installation => {}
            github_hooks::Payload::InstallationRepositories => {}
            github_hooks::Payload::Ping(_ping) => {}
            github_hooks::Payload::Status(status) => {
                let request_cx = ghss_store_client::request_context("record_hook");
                let request = ghss_store_client::request(
                    RecordHookRequest {
                        repository_id: status.repository.id.to_string(),
                        hook: Some(Hook {
                            r#type: BuildSource::Status as i32,
                            commit: status.sha,
                            timestamp: status.created_at.timestamp_millis(),
                        }),
                        build: None,
                    },
                    &request_cx,
                );
                let _response = client.record_hook(request).with_context(request_cx).await?;
            }
        };

        Ok(())
    }
    .await;

    match res {
        Ok(_) => Ok(StatusCode::OK),
        Err(err) => {
            cx.span().add_event(
                "hook failed".into(),
                vec![Key::new("error.message").string(err.to_string())],
            );
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

pub fn with_store_client(
    client: StoreClient<ghss_store_client::Channel>,
) -> impl Filter<Extract = (StoreClient<ghss_store_client::Channel>,), Error = Infallible> + Clone {
    warp::any().map(move || client.clone())
}

pub fn with_query_client(
    client: QueryClient<ghss_store_client::Channel>,
) -> impl Filter<Extract = (QueryClient<ghss_store_client::Channel>,), Error = Infallible> + Clone {
    warp::any().map(move || client.clone())
}

fn init_tracer(agent_endpoint: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let provider = match agent_endpoint {
        Some(agent_endpoint) => {
            let exporter = opentelemetry_jaeger::Exporter::builder()
                .with_agent_endpoint(agent_endpoint.parse().unwrap())
                .with_process(opentelemetry_jaeger::Process {
                    service_name: "website".to_string(),
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
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::load();
    let config = Arc::new(config);
    let templates = templates::load();
    let templates = Arc::new(templates);

    let store_client = StoreClient::connect(config.store_url.clone()).await?;
    let query_client = QueryClient::connect(config.store_url.clone()).await?;

    init_tracer(config.otel_agent_endpoint.as_deref())?;

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
        .and(with_query_client(query_client))
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
        .and(with_store_client(store_client))
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
                    let tracer = opentelemetry::global::tracer("website");
                    let span = tracer
                        .span_builder("request")
                        .with_kind(SpanKind::Server)
                        .with_attributes(vec![
                            Key::new("method").string(req.method().as_str()),
                            Key::new("path").string(req.uri().path()),
                            Key::new("user_agent").string(
                                req.headers()
                                    .get(header::USER_AGENT)
                                    .map(|v| v.to_str().expect("user agent should be a string"))
                                    .unwrap_or(""),
                            ),
                        ])
                        .start(&tracer);
                    let cx = Context::current_with_span(span);

                    //let started = std::time::Instant::now();
                    let res = warp_svc.call(req).with_context(cx.clone()).await;
                    //let duration_ms = (std::time::Instant::now() - started).as_millis();
                    //span.record("duration_ms", &duration_ms.to_string().as_str());

                    match res.as_ref() {
                        Ok(res) => {
                            cx.span().set_attribute(
                                Key::new("status").u64(res.status().as_u16().into()),
                            );
                        }
                        Err(err) => {
                            let span = cx.span();
                            span.set_status(
                                opentelemetry::api::StatusCode::Internal,
                                err.to_string(),
                            );
                            span.add_event(
                                "request failed".into(),
                                vec![Key::new("error.message").string(err.to_string())],
                            );
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
