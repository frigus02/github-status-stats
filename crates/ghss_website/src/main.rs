#[macro_use]
extern crate lazy_static;

mod cookie;
mod ctrlc;
mod github_hooks;
mod github_queries;
mod templates;
mod token;

use bytes::Bytes;
use ghss_models::{influxdb_name, influxdb_name_unsafe, influxdb_read_user_unsafe, Build};
use ghss_tracing::{error, info, info_span, Instrument};
use secstr::{SecStr, SecUtf8};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::Infallible;
use std::convert::TryFrom;
use templates::{DashboardData, DashboardTemplate, IndexTemplate, RepositoryAccess};
use warp::{
    http::{Response, StatusCode, Uri},
    hyper::{self, header, service::Service, Body, Request},
    Filter,
};

const COOKIE_NAME: &str = "token";
lazy_static! {
    static ref HOST: String = std::env::var("HOST").expect("env HOST");
    static ref GH_REDIRECT_URI: String = format!("{}/setup/authorized", *HOST);
    static ref GH_CLIENT_ID: String = std::env::var("GH_CLIENT_ID").expect("env GH_CLIENT_ID");
    static ref GH_CLIENT_SECRET: SecUtf8 =
        SecUtf8::from(std::env::var("GH_CLIENT_SECRET").expect("env GH_CLIENT_SECRET"));
    static ref GH_WEBHOOK_SECRET: SecStr =
        SecStr::from(std::env::var("GH_WEBHOOK_SECRET").expect("env GH_WEBHOOK_SECRET"));
    static ref INFLUXDB_BASE_URL: String =
        std::env::var("INFLUXDB_BASE_URL").expect("env INFLUXDB_BASE_URL");
    static ref INFLUXDB_ADMIN_USERNAME: String =
        std::env::var("INFLUXDB_ADMIN_USERNAME").expect("env INFLUXDB_ADMIN_USERNAME");
    static ref INFLUXDB_ADMIN_PASSWORD: SecUtf8 = SecUtf8::from(
        std::env::var("INFLUXDB_ADMIN_PASSWORD").expect("env INFLUXDB_ADMIN_PASSWORD")
    );
    static ref INFLUXDB_READ_PASSWORD: SecUtf8 =
        SecUtf8::from(std::env::var("INFLUXDB_READ_PASSWORD").expect("env INFLUXDB_READ_PASSWORD"));
    static ref TOKEN_SECRET: SecStr =
        SecStr::from(std::env::var("TOKEN_SECRET").expect("env TOKEN_SECRET"));
    static ref HONEYCOMB_API_KEY: SecUtf8 =
        SecUtf8::from(std::env::var("HONEYCOMB_API_KEY").expect("env HONEYCOMB_API_KEY"));
    static ref HONEYCOMB_DATASET: String =
        std::env::var("HONEYCOMB_DATASET").expect("env HONEYCOMB_DATASET");
}

async fn index_route(token: Option<token::User>) -> Result<impl warp::Reply, Infallible> {
    let data = match token {
        Some(user) => IndexTemplate::LoggedIn {
            user: user.name,
            repositories: user
                .repositories
                .into_iter()
                .map(|repo| RepositoryAccess { name: repo.name })
                .collect(),
            login_url: ghss_github::oauth::login_url(&*GH_CLIENT_ID, &GH_REDIRECT_URI, None),
        },
        None => IndexTemplate::Anonymous {
            login_url: ghss_github::oauth::login_url(&*GH_CLIENT_ID, &GH_REDIRECT_URI, None),
        },
    };

    let render = templates::render_index(&data);
    Ok(warp::reply::html(render))
}

async fn dashboard_route(
    owner: String,
    repo: String,
    token: Option<token::User>,
) -> Result<Box<dyn warp::Reply>, Infallible> {
    let name = format!("{}/{}", owner, repo);
    let reply: Box<dyn warp::Reply> = match token {
        Some(user) => {
            let data = match user.repositories.into_iter().find(|r| r.name == name) {
                Some(repo) => DashboardData::Data {
                    repository_id: repo.id,
                },
                None => DashboardData::Error {
                    message: "Not found".to_string(),
                },
            };

            let render = templates::render_dashboard(&DashboardTemplate {
                user: user.name,
                repository_name: name,
                data,
            });
            Box::new(warp::reply::html(render))
        }
        None => {
            let login_url = ghss_github::oauth::login_url(
                &*GH_CLIENT_ID,
                &GH_REDIRECT_URI,
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
struct ApiQueryParams {
    repository: i32,
    query: String,
}

#[derive(Debug, Serialize)]
struct ApiQueryResponse {
    pub tags: Option<HashMap<String, String>>,
    pub columns: Vec<String>,
    pub values: Vec<Vec<Option<ghss_influxdb::FieldValue>>>,
}

async fn api_query_route(
    params: ApiQueryParams,
    token: Option<token::User>,
) -> Result<Box<dyn warp::Reply>, Infallible> {
    let reply: Box<dyn warp::Reply> = match token {
        Some(user) if user.repositories.iter().any(|r| r.id == params.repository) => {
            let influxdb_db = influxdb_name_unsafe(params.repository);
            let res: Result<Vec<ApiQueryResponse>, Box<dyn std::error::Error>> = async {
                let client = ghss_influxdb::Client::new(
                    &*INFLUXDB_BASE_URL,
                    &influxdb_db,
                    &influxdb_read_user_unsafe(params.repository),
                    &*INFLUXDB_READ_PASSWORD.unsecure(),
                )?;
                let res = client
                    .query(&params.query)
                    .await?
                    .into_single_result()?
                    .into_series()?
                    .into_iter()
                    .map(|s| ApiQueryResponse {
                        tags: s.tags,
                        columns: s.columns,
                        values: s.values,
                    })
                    .collect();
                Ok(res)
            }
            .await;
            match res {
                Ok(res) => Box::new(warp::reply::json(&res)),
                Err(err) => {
                    error!(
                        influxdb.db = %influxdb_db,
                        influxdb.query = %params.query,
                        err = %err,
                        "influxdb query failed",
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
) -> Result<Box<dyn warp::Reply>, Infallible> {
    let token = async {
        let github_token = ghss_github::oauth::exchange_code(
            &*GH_CLIENT_ID,
            &*GH_CLIENT_SECRET.unsecure(),
            &*GH_REDIRECT_URI,
            &info,
        )
        .await?;
        token::generate(&github_token.access_token, TOKEN_SECRET.unsecure()).await
    }
    .await;

    match token {
        Ok(token) => {
            let redirect_path = info.state.map_or("/".to_owned(), path_from_state);
            Ok(Box::new(
                Response::builder()
                    .status(StatusCode::TEMPORARY_REDIRECT)
                    .header("location", redirect_path)
                    .header("set-cookie", cookie::set(COOKIE_NAME, &token))
                    .body(Body::empty())
                    .unwrap(),
            ))
        }
        Err(_) => Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR)),
    }
}

async fn logout_route() -> Result<impl warp::Reply, Infallible> {
    Ok(Response::builder()
        .status(StatusCode::TEMPORARY_REDIRECT)
        .header("location", "/")
        .header("set-cookie", cookie::remove(COOKIE_NAME))
        .body(Body::empty())
        .unwrap())
}

async fn hooks_route(
    signature: String,
    event: String,
    body: Bytes,
) -> Result<impl warp::Reply, Infallible> {
    let res: Result<(), Box<dyn std::error::Error>> = async {
        let payload =
            github_hooks::deserialize(signature, event, body, &*GH_WEBHOOK_SECRET.unsecure())?;

        info!("Hook: {:?}", payload);
        match payload {
            github_hooks::Payload::CheckRun(check_run) => {
                let influxdb_db = influxdb_name(&check_run.repository);
                let client = ghss_influxdb::Client::new(
                    &*INFLUXDB_BASE_URL,
                    &influxdb_db,
                    &*INFLUXDB_ADMIN_USERNAME,
                    &*INFLUXDB_ADMIN_PASSWORD.unsecure(),
                )?;
                client
                    .write(vec![
                        ghss_models::Hook {
                            time: check_run.check_run.started_at,
                            r#type: ghss_models::HookType::CheckRun,
                            commit_sha: check_run.check_run.head_sha.clone(),
                        }
                        .into(),
                        Build::from(check_run.check_run).into(),
                    ])
                    .await?
            }
            github_hooks::Payload::GitHubAppAuthorization(_auth) => {}
            github_hooks::Payload::Installation => {}
            github_hooks::Payload::InstallationRepositories => {}
            github_hooks::Payload::Ping(_ping) => {}
            github_hooks::Payload::Status(status) => {
                let influxdb_db = influxdb_name(&status.repository);
                let client = ghss_influxdb::Client::new(
                    &*INFLUXDB_BASE_URL,
                    &influxdb_db,
                    &*INFLUXDB_ADMIN_USERNAME,
                    &*INFLUXDB_ADMIN_PASSWORD.unsecure(),
                )?;
                client
                    .write(vec![ghss_models::Hook {
                        time: status.created_at,
                        r#type: ghss_models::HookType::Status,
                        commit_sha: status.sha,
                    }
                    .into()])
                    .await?
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

pub fn optional_token() -> impl Filter<Extract = (Option<token::User>,), Error = Infallible> + Clone
{
    warp::cookie::optional(COOKIE_NAME).map(|raw_token: Option<String>| {
        raw_token.and_then(|t| {
            let user = token::validate(&t, TOKEN_SECRET.unsecure());
            match user {
                Ok(user) => {
                    ghss_tracing::Span::current().record("user_id", &user.id.as_str());
                    Some(user)
                }
                Err(err) => {
                    error!(error = %err, "token validation failed");
                    None
                }
            }
        })
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ghss_tracing::setup(ghss_tracing::Config {
        honeycomb_api_key: HONEYCOMB_API_KEY.unsecure().to_owned(),
        honeycomb_dataset: HONEYCOMB_DATASET.clone(),
        service_name: "website".to_owned(),
    });

    let index = warp::get()
        .and(warp::path::end())
        .and(optional_token())
        .and_then(index_route);

    let favicon = warp::get()
        .and(warp::path!("favicon.ico"))
        .and(warp::fs::file("static/favicon.ico"));

    let static_files = warp::path!("static" / ..).and(warp::fs::dir("static"));

    let dashboard = warp::get()
        .and(warp::path!("d" / String / String))
        .and(optional_token())
        .and_then(dashboard_route);

    let api_query = warp::get()
        .and(warp::path!("api" / "query"))
        .and(warp::query())
        .and(optional_token())
        .and_then(api_query_route);

    let setup_authorized = warp::get()
        .and(warp::path!("setup" / "authorized"))
        .and(warp::query::<ghss_github::oauth::AuthCodeQuery>())
        .and_then(setup_authorized_route);

    let logout = warp::get()
        .and(warp::path!("logout"))
        .and_then(logout_route);

    let hooks = warp::post()
        .and(warp::path!("hooks"))
        .and(warp::header("X-Hub-Signature"))
        .and(warp::header("X-GitHub-Event"))
        .and(warp::body::bytes())
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
                        user_id = ghss_tracing::EmptyField,
                        status = ghss_tracing::EmptyField,
                        duration_ms = ghss_tracing::EmptyField,
                    );

                    let started = std::time::Instant::now();
                    let res = warp_svc.call(req).instrument(span.clone()).await;
                    let duration_ms = (std::time::Instant::now() - started).as_millis();

                    span.record("duration_ms", &duration_ms.to_string().as_str());

                    let _guard = span.enter();
                    match res.as_ref() {
                        Ok(res) => {
                            span.record("status", &res.status().as_str());
                        }
                        Err(err) => {
                            error!(error = %err,"request failed");
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
