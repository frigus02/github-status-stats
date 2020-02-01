#[macro_use]
extern crate lazy_static;

mod filters;
mod github_hooks;
mod github_queries;
mod grafana_queries;
mod reverse_proxy;
mod templates;

use bytes::Bytes;
use filters::raw_request;
use log::info;
use reverse_proxy::ReverseProxy;
use secstr::{SecStr, SecUtf8};
use stats::{influxdb_name, Build};
use std::convert::Infallible;
use templates::{IndexTemplate, RepositoryAccess};
use warp::{
    http::{Request, Response, StatusCode},
    hyper::Body,
    Filter,
};

lazy_static! {
    static ref HOST: String = std::env::var("HOST").expect("env HOST");
    static ref GH_REDIRECT_URI: String = format!("{}/setup/authorized", *HOST);
    static ref GH_CLIENT_ID: String = std::env::var("GH_CLIENT_ID").expect("env GH_CLIENT_ID");
    static ref GH_CLIENT_SECRET: SecUtf8 =
        SecUtf8::from(std::env::var("GH_CLIENT_SECRET").expect("env GH_CLIENT_SECRET"));
    static ref GH_LOGIN_URL: String =
        github_client::oauth::login_url(&*GH_CLIENT_ID, &GH_REDIRECT_URI)
            .expect("construct GitHub login URL")
            .into_string();
    static ref GH_WEBHOOK_SECRET: SecStr =
        SecStr::from(std::env::var("GH_WEBHOOK_SECRET").expect("env GH_WEBHOOK_SECRET"));
    static ref INFLUXDB_BASE_URL: String =
        std::env::var("INFLUXDB_BASE_URL").expect("env INFLUXDB_BASE_URL");
    static ref INFLUXDB_ADMIN_USERNAME: String =
        std::env::var("INFLUXDB_ADMIN_USERNAME").expect("env INFLUXDB_ADMIN_USERNAME");
    static ref INFLUXDB_ADMIN_PASSWORD: SecUtf8 = SecUtf8::from(
        std::env::var("INFLUXDB_ADMIN_PASSWORD").expect("env INFLUXDB_ADMIN_PASSWORD")
    );
    static ref GRAFANA_BASE_URL: String =
        std::env::var("GRAFANA_BASE_URL").expect("env GRAFANA_BASE_URL");
    static ref GRAFANA_CLIENT: grafana_client::Client = grafana_client::Client::new(
        GRAFANA_BASE_URL.clone(),
        &std::env::var("GRAFANA_ADMIN_USERNAME").expect("env GRAFANA_ADMIN_USERNAME"),
        &std::env::var("GRAFANA_ADMIN_PASSWORD").expect("env GRAFANA_ADMIN_PASSWORD")
    )
    .unwrap();
    static ref GRAFANA_PROXY: ReverseProxy = ReverseProxy::new(&*GRAFANA_BASE_URL).unwrap();
}

fn new_error_res(status: StatusCode) -> Response<Body> {
    Response::builder()
        .status(status)
        .body(Body::from(status.to_string()))
        .unwrap()
}

async fn index_route(token: Option<String>) -> Result<impl warp::Reply, Infallible> {
    let data = match token {
        Some(token) => match grafana_queries::get_user(&token, &*GRAFANA_CLIENT).await {
            Ok(user) => IndexTemplate::LoggedIn {
                user: user.github.name,
                repositories: user
                    .repositories
                    .into_iter()
                    .map(|repo| RepositoryAccess {
                        full_name: repo.github.full_name,
                        grafana_org_id: repo.grafana.map(|org| org.id),
                    })
                    .collect(),
            },
            Err(err) => IndexTemplate::Error {
                message: err.to_string(),
            },
        },
        None => IndexTemplate::Anonymous {
            login_url: GH_LOGIN_URL.clone(),
        },
    };

    let render = templates::render_index(&data);
    Ok(warp::reply::html(render))
}

async fn dashboard_login_route(
    req: Request<Body>,
    token: Option<String>,
) -> Result<impl warp::Reply, Infallible> {
    let token = match token {
        Some(token) => token,
        None => return Ok(new_error_res(StatusCode::UNAUTHORIZED)),
    };

    let login = grafana_queries::sync_user(&token, &*GRAFANA_CLIENT)
        .await
        .map_err(|err| err.to_string());
    let res = match login {
        Ok(login) => GRAFANA_PROXY
            .call_with_auth(req, login)
            .await
            .map_err(|err| err.to_string()),
        Err(err) => Err(err),
    };
    match res {
        Ok(res) => Ok(res),
        Err(_) => Ok(new_error_res(StatusCode::INTERNAL_SERVER_ERROR)),
    }
}

async fn dashboard_route(req: Request<Body>) -> Result<impl warp::Reply, Infallible> {
    let res = GRAFANA_PROXY.call(req).await;
    Ok(res)
}

async fn setup_authorized_route(
    info: github_client::oauth::AuthCodeQuery,
) -> Result<impl warp::Reply, Infallible> {
    let token = github_client::oauth::exchange_code(
        &*GH_CLIENT_ID,
        &*GH_CLIENT_SECRET.unsecure(),
        &*GH_REDIRECT_URI,
        info,
    )
    .await;
    match token {
        Ok(token) => Ok(Response::builder()
            .status(StatusCode::TEMPORARY_REDIRECT)
            .header("location", "/")
            .header(
                "set-cookie",
                format!(
                    "token={}; Path=/; SameSite=Strict; Secure; HttpOnly",
                    token.access_token
                ),
            )
            .body(Body::empty())
            .unwrap()),
        Err(_) => Ok(new_error_res(StatusCode::INTERNAL_SERVER_ERROR)),
    }
}

async fn hooks_route(
    signature: String,
    event: String,
    body: Bytes,
) -> Result<impl warp::Reply, Infallible> {
    let payload = github_hooks::deserialize(signature, event, body, &*GH_WEBHOOK_SECRET.unsecure())
        .map_err(|err| err.to_string());

    info!("Hook: {:?}", payload);
    let res = match payload {
        Ok(github_hooks::Payload::CheckRun(check_run)) => {
            let influxdb_db = influxdb_name(&check_run.repository);
            let client = influxdb_client::Client::new(
                &*INFLUXDB_BASE_URL,
                &influxdb_db,
                &*INFLUXDB_ADMIN_USERNAME,
                &*INFLUXDB_ADMIN_PASSWORD.unsecure(),
            )
            .unwrap();
            client
                .write(vec![
                    stats::Hook {
                        time: check_run.check_run.started_at,
                        r#type: stats::HookType::CheckRun,
                        commit_sha: check_run.check_run.head_sha.clone(),
                    }
                    .into(),
                    Build::from(check_run.check_run).into(),
                ])
                .await
                .map_err(|err| err.to_string())
        }
        Ok(github_hooks::Payload::GitHubAppAuthorization(_auth)) => Ok(()),
        Ok(github_hooks::Payload::Installation) => Ok(()),
        Ok(github_hooks::Payload::InstallationRepositories) => Ok(()),
        Ok(github_hooks::Payload::Ping(_ping)) => Ok(()),
        Ok(github_hooks::Payload::Status(status)) => {
            let influxdb_db = influxdb_name(&status.repository);
            let client = influxdb_client::Client::new(
                &*INFLUXDB_BASE_URL,
                &influxdb_db,
                &*INFLUXDB_ADMIN_USERNAME,
                &*INFLUXDB_ADMIN_PASSWORD.unsecure(),
            )
            .unwrap();
            client
                .write(vec![stats::Hook {
                    time: status.created_at,
                    r#type: stats::HookType::Status,
                    commit_sha: status.sha,
                }
                .into()])
                .await
                .map_err(|err| err.to_string())
        }
        Err(err) => Err(err),
    };

    match res {
        Ok(_) => Ok(StatusCode::OK),
        Err(_) => Ok(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let index = warp::get()
        .and(warp::path::end())
        .and(warp::cookie::optional("token"))
        .and_then(index_route);

    let favicon = warp::get()
        .and(warp::path!("favicon.ico"))
        .and(warp::fs::file("static/favicon.ico"));

    let dashboard_login = warp::path!("_" / "login" / ..)
        .and(raw_request())
        .and(warp::cookie::optional("token"))
        .and_then(dashboard_login_route);
    let dashboard = warp::path!("_" / ..)
        .and(raw_request())
        .and_then(dashboard_route);

    let setup_authorized = warp::get()
        .and(warp::path!("setup" / "authorized"))
        .and(warp::query::<github_client::oauth::AuthCodeQuery>())
        .and_then(setup_authorized_route);

    let hooks = warp::post()
        .and(warp::path!("hooks"))
        .and(warp::header("X-Hub-Signature"))
        .and(warp::header("X-GitHub-Event"))
        .and(warp::body::bytes())
        .and_then(hooks_route);

    let routes = index
        .or(favicon)
        .or(dashboard_login)
        .or(dashboard)
        .or(setup_authorized)
        .or(hooks)
        .with(warp::log("website"));

    warp::serve(routes).run(([0, 0, 0, 0], 8888)).await;
    Ok(())
}
