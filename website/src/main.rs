#[macro_use]
extern crate lazy_static;

mod ctrlc;
mod github_hooks;
mod github_queries;
mod templates;
mod token;

use bytes::Bytes;
use log::{error, info};
use secstr::{SecStr, SecUtf8};
use stats::{influxdb_name, Build};
use std::convert::Infallible;
use templates::{IndexTemplate, RepositoryAccess};
use warp::{
    http::{Response, StatusCode},
    hyper::Body,
    Filter,
};

const COOKIE_NAME: &str = "token";
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
    static ref TOKEN_SECRET: SecStr =
        SecStr::from(std::env::var("TOKEN_SECRET").expect("env TOKEN_SECRET"));
}

async fn index_route(token: Option<String>) -> Result<impl warp::Reply, Infallible> {
    let data = match token {
        Some(token) => match token::validate(&token, TOKEN_SECRET.unsecure()) {
            Ok(user) => IndexTemplate::LoggedIn {
                user: user.name,
                repositories: user
                    .repositories
                    .into_iter()
                    .map(|repo| RepositoryAccess { name: repo.name })
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

async fn dashboard_route(
    _owner: String,
    _repo: String,
    _token: Option<String>,
) -> Result<impl warp::Reply, Infallible> {
    Ok("")
}

async fn setup_authorized_route(
    info: github_client::oauth::AuthCodeQuery,
) -> Result<Box<dyn warp::Reply>, Infallible> {
    let token = async {
        let github_token = github_client::oauth::exchange_code(
            &*GH_CLIENT_ID,
            &*GH_CLIENT_SECRET.unsecure(),
            &*GH_REDIRECT_URI,
            info,
        )
        .await?;
        token::generate(&github_token.access_token, TOKEN_SECRET.unsecure()).await
    }
    .await;

    match token {
        Ok(token) => Ok(Box::new(
            Response::builder()
                .status(StatusCode::TEMPORARY_REDIRECT)
                .header("location", "/")
                .header(
                    "set-cookie",
                    format!(
                        "{}={}; Path=/; SameSite=Lax; Secure; HttpOnly",
                        COOKIE_NAME, token
                    ),
                )
                .body(Body::empty())
                .unwrap(),
        )),
        Err(_) => Ok(Box::new(StatusCode::INTERNAL_SERVER_ERROR)),
    }
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
                let client = influxdb_client::Client::new(
                    &*INFLUXDB_BASE_URL,
                    &influxdb_db,
                    &*INFLUXDB_ADMIN_USERNAME,
                    &*INFLUXDB_ADMIN_PASSWORD.unsecure(),
                )?;
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
                    .await?
            }
            github_hooks::Payload::GitHubAppAuthorization(_auth) => {}
            github_hooks::Payload::Installation => {}
            github_hooks::Payload::InstallationRepositories => {}
            github_hooks::Payload::Ping(_ping) => {}
            github_hooks::Payload::Status(status) => {
                let influxdb_db = influxdb_name(&status.repository);
                let client = influxdb_client::Client::new(
                    &*INFLUXDB_BASE_URL,
                    &influxdb_db,
                    &*INFLUXDB_ADMIN_USERNAME,
                    &*INFLUXDB_ADMIN_PASSWORD.unsecure(),
                )?;
                client
                    .write(vec![stats::Hook {
                        time: status.created_at,
                        r#type: stats::HookType::Status,
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
            error!("Hook error: {:?}", err);
            Ok(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let index = warp::get()
        .and(warp::path::end())
        .and(warp::cookie::optional(COOKIE_NAME))
        .and_then(index_route);

    let favicon = warp::get()
        .and(warp::path!("favicon.ico"))
        .and(warp::fs::file("static/favicon.ico"));

    let dashboard = warp::get()
        .and(warp::path!("d" / String / String))
        .and(warp::cookie::optional(COOKIE_NAME))
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
        .or(dashboard)
        .or(setup_authorized)
        .or(hooks)
        .with(warp::log("website"));

    let (_addr, server) =
        warp::serve(routes).bind_with_graceful_shutdown(([0, 0, 0, 0], 8888), async {
            ctrlc::ctrl_c().await;
        });

    server.await;

    Ok(())
}
