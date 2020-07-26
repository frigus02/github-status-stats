mod config;
mod ctrlc;
mod github_hooks;
mod github_queries;
mod serve_file;
mod telemetry_middleware;
mod templates;
mod token;

use futures::{future::FutureExt as _, select};
use ghss_store_client::{
    AggregateFunction, BuildSource, Hook, IntervalAggregatesRequest, IntervalType, QueryClient,
    RecordHookRequest, StoreClient, TotalAggregatesRequest,
};
use ghss_tracing::{error_event, init_tracer, log_event};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serve_file::RouteExt;
use std::collections::HashMap;
use std::sync::Arc;
use telemetry_middleware::TelemetryMiddleware;
use templates::{DashboardData, DashboardTemplate, IndexTemplate, RepositoryAccess};
use tide::{
    http::{cookies::SameSite, Cookie, Url},
    Body, Redirect, Request, Response, StatusCode,
};
use token::OptionalToken;

#[derive(Clone)]
struct State {
    config: Arc<config::Config>,
    templates: Arc<templates::Templates<'static>>,
    store_client: StoreClient,
    query_client: QueryClient,
}

async fn handle_index(req: Request<State>) -> tide::Result<Response> {
    let state = req.state();
    let config = &state.config;
    let templates = &state.templates;
    let res: Response = match token::optional_token(
        &req,
        config.cookie_name,
        config.token_secret.unsecure().into(),
    ) {
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
            Response::builder(200)
                .body(templates.render_index(&data))
                .content_type(tide::http::mime::HTML)
                .build()
        }
        OptionalToken::Expired => {
            let login_url =
                ghss_github::oauth::login_url(&config.gh_client_id, &config.gh_redirect_uri, None);
            Redirect::temporary(login_url).into()
        }
        OptionalToken::None => {
            let data = IndexTemplate::Anonymous {
                login_url: ghss_github::oauth::login_url(
                    &config.gh_client_id,
                    &config.gh_redirect_uri,
                    None,
                ),
            };
            Response::builder(200)
                .body(templates.render_index(&data))
                .content_type(tide::http::mime::HTML)
                .build()
        }
    };

    Ok(res)
}

async fn handle_dashboard(req: Request<State>) -> tide::Result<Response> {
    let state = req.state();
    let config = &state.config;
    let templates = &state.templates;
    let owner: String = req.param("owner")?;
    let repo: String = req.param("repo")?;
    let name = format!("{}/{}", owner, repo);
    let res: Response = match token::optional_token(
        &req,
        config.cookie_name,
        config.token_secret.unsecure().into(),
    ) {
        OptionalToken::Some(user) => {
            let data = match user.repositories.into_iter().find(|r| r.name == name) {
                Some(repo) => DashboardData::Data {
                    repository_id: repo.id,
                },
                None => DashboardData::Error {
                    message: "Not found".to_string(),
                },
            };
            let mut res: Response = templates
                .render_dashboard(&DashboardTemplate {
                    user: user.name,
                    repository_name: name,
                    data,
                })
                .into();
            res.set_content_type(tide::http::mime::HTML);
            res
        }
        OptionalToken::Expired | OptionalToken::None => {
            let login_url = ghss_github::oauth::login_url(
                &config.gh_client_id,
                &config.gh_redirect_uri,
                Some(path_to_state(format!("/d/{}/{}", owner, repo))),
            );
            Redirect::temporary(login_url).into()
        }
    };
    Ok(res)
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

async fn handle_api_query(req: Request<State>) -> tide::Result<Response> {
    let state = req.state();
    let config = &state.config;
    let mut client = state.query_client.clone();
    let params: ApiQueryParams = req.query()?;
    let res: Response = match token::optional_token(
        &req,
        config.cookie_name,
        config.token_secret.unsecure().into(),
    ) {
        OptionalToken::Some(user)
            if user.repositories.iter().any(|r| r.id == params.repository) =>
        {
            let res: Result<ApiQueryResponse, Box<dyn std::error::Error>> = async {
                let response = match params.interval {
                    Some(interval) => {
                        let response = client
                            .get_interval_aggregates(IntervalAggregatesRequest {
                                repository_id: params.repository.to_string(),
                                table: params.table,
                                columns: params.columns.into_iter().map(|c| c.into()).collect(),
                                since: params.since,
                                until: params.until,
                                group_by: params.group_by.unwrap_or_default(),
                                interval: IntervalType::from(interval) as i32,
                            })
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
                        let response = client
                            .get_total_aggregates(TotalAggregatesRequest {
                                repository_id: params.repository.to_string(),
                                table: params.table,
                                columns: params.columns.into_iter().map(|c| c.into()).collect(),
                                since: params.since,
                                until: params.until,
                                group_by: params.group_by.unwrap_or_default(),
                            })
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
                Ok(res) => Body::from_json(&res)?.into(),
                Err(err) => {
                    error_event(format!("query failed: {:?}", err));
                    Response::from_res(StatusCode::InternalServerError)
                }
            }
        }
        _ => Response::from_res(StatusCode::Unauthorized),
    };
    Ok(res)
}

async fn handle_setup_authorized(req: Request<State>) -> tide::Result<Response> {
    let state = req.state();
    let config = &state.config;
    let info: ghss_github::oauth::AuthCodeQuery = req.query()?;
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

    let res = match token {
        Ok(token) => {
            let redirect_path = info.state.map_or("/".to_owned(), path_from_state);
            let mut res: Response = Redirect::temporary(redirect_path).into();
            res.insert_cookie(
                Cookie::build(config.cookie_name, token)
                    .path("/")
                    .max_age(time::Duration::days(30))
                    .same_site(SameSite::Lax)
                    .secure(true)
                    .http_only(true)
                    .finish(),
            );
            res
        }
        Err(_) => Response::from_res(StatusCode::InternalServerError),
    };
    Ok(res)
}

async fn handle_logout(req: Request<State>) -> tide::Result<Response> {
    let state = req.state();
    let config = &state.config;
    let mut res: Response = Redirect::temporary("/").into();
    res.remove_cookie(Cookie::new(config.cookie_name, ""));
    Ok(res)
}

async fn handle_hooks(mut req: Request<State>) -> tide::Result<Response> {
    let body = req.body_bytes().await?;
    let state = req.state();
    let config = &state.config;
    let mut client = state.store_client.clone();
    let signature = req.header("X-Hub-Signature").ok_or_else(|| {
        tide::Error::from_str(StatusCode::BadRequest, "X-Hub-Signature header required")
    })?;
    let event = req.header("X-GitHub-Event").ok_or_else(|| {
        tide::Error::from_str(StatusCode::BadRequest, "X-GitHub-Event header required")
    })?;
    let res: Result<(), Box<dyn std::error::Error>> = async {
        let payload = github_hooks::deserialize(
            signature.as_str(),
            event.as_str(),
            &body,
            &config.gh_webhook_secret.unsecure(),
        )?;

        log_event(format!("hook payload: {:?}", payload));

        match payload {
            github_hooks::Payload::CheckRun(check_run) => {
                let _response = client
                    .record_hook(RecordHookRequest {
                        repository_id: check_run.repository.id.to_string(),
                        hook: Some(Hook {
                            r#type: BuildSource::CheckRun as i32,
                            commit: check_run.check_run.head_sha.clone(),
                            timestamp: check_run.check_run.started_at.timestamp_millis(),
                        }),
                        build: Some(check_run.check_run.into()),
                    })
                    .await?;
            }
            github_hooks::Payload::GitHubAppAuthorization(_auth) => {}
            github_hooks::Payload::Installation => {}
            github_hooks::Payload::InstallationRepositories => {}
            github_hooks::Payload::Ping(_ping) => {}
            github_hooks::Payload::Status(status) => {
                let _response = client
                    .record_hook(RecordHookRequest {
                        repository_id: status.repository.id.to_string(),
                        hook: Some(Hook {
                            r#type: BuildSource::Status as i32,
                            commit: status.sha,
                            timestamp: status.created_at.timestamp_millis(),
                        }),
                        build: None,
                    })
                    .await?;
            }
        };

        Ok(())
    }
    .await;

    let res = match res {
        Ok(_) => Response::from_res(StatusCode::Ok),
        Err(err) => {
            error_event(format!("hook failed: {:?}", err));
            Response::from_res(StatusCode::InternalServerError)
        }
    };
    Ok(res)
}

pub fn path_to_state(path: String) -> String {
    base64::encode(path)
}

pub fn path_from_state(state: String) -> String {
    let result: Result<String, Box<dyn std::error::Error>> = base64::decode(state)
        .map_err(|err| err.into())
        .and_then(|bytes| Ok(Url::parse(std::str::from_utf8(&bytes)?)?))
        .and_then(|url| {
            if !url.scheme().is_empty() {
                return Err("only path allowed but found scheme".into());
            }
            if url.has_authority() {
                return Err("only path allowed but found authority".into());
            }

            Ok(url.path().to_string())
        });
    result.unwrap_or_else(|_| "/".to_owned())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = config::load();
    let templates = templates::load();

    let store_client = StoreClient::connect(config.store_url.clone()).await?;
    let query_client = QueryClient::connect(config.store_url.clone()).await?;

    init_tracer("website", config.otel_agent_endpoint.as_deref())?;

    let state = State {
        config: Arc::new(config),
        templates: Arc::new(templates),
        store_client,
        query_client,
    };

    let mut app = tide::with_state(state);
    app.at("/").get(handle_index);
    app.at("/favicon.ico").serve_file("static/favicon.ico");
    app.at("/static").serve_dir("static")?;
    app.at("/d/:owner/:repo").get(handle_dashboard);
    app.at("/api/query").get(handle_api_query);
    app.at("/setup/authorized").get(handle_setup_authorized);
    app.at("/logout").get(handle_logout);
    app.at("/hooks").post(handle_hooks);

    app.middleware(TelemetryMiddleware {});

    let res = select! {
        res = app.listen("0.0.0.0:8888").fuse() => res,
        () = ctrlc::ctrl_c().fuse() => {
            println!("Received Ctrl+C. Shutting down...");
            Ok(())
        },
    };
    res?;
    Ok(())
}
