#[macro_use]
extern crate lazy_static;

mod github_hooks;

use bytes::Bytes;
use log::{error, info};
use secstr::{SecStr, SecUtf8};
use stats::influxdb_name;
use std::convert::Infallible;
use typed_html::{dom::DOMTree, html, text};
use warp::{http::StatusCode, http::Uri, Filter};

const REDIRECT_URI: &str = "https://d2921223.ngrok.io/setup/authorized";

lazy_static! {
    static ref GH_CLIENT_ID: String = std::env::var("GH_CLIENT_ID").unwrap();
    static ref GH_CLIENT_SECRET: SecUtf8 =
        SecUtf8::from(std::env::var("GH_CLIENT_SECRET").unwrap());
    static ref GH_LOGIN_URL: String =
        github_client::oauth::login_url(&*GH_CLIENT_ID, &REDIRECT_URI)
            .unwrap()
            .into_string();
    static ref GH_WEBHOOK_SECRET: SecStr =
        SecStr::from(std::env::var("GH_WEBHOOK_SECRET").unwrap());
    static ref INFLUXDB_BASE_URL: String = std::env::var("INFLUXDB_BASE_URL").unwrap();
    static ref INFLUXDB_USERNAME: String = std::env::var("INFLUXDB_USERNAME").unwrap();
    static ref INFLUXDB_PASSWORD: SecUtf8 =
        SecUtf8::from(std::env::var("INFLUXDB_PASSWORD").unwrap());
}

#[derive(Debug)]
struct HttpError;

impl warp::reject::Reject for HttpError {}

fn reject(err: Box<dyn std::error::Error>) -> warp::Rejection {
    error!("route error {}", err);
    warp::reject::custom(HttpError)
}

async fn index_route(token: Option<String>) -> Result<impl warp::Reply, warp::Rejection> {
    let user = if let Some(token) = token {
        let client = github_client::Client::new(&token).map_err(reject)?;
        Some(client.get_user().await.map_err(reject)?)
    } else {
        None
    };

    let doc: DOMTree<String> = html!(
        <html>
            <head>
                <title>"Status Stats"</title>
            </head>
            <body>
            { if let Some(user) = user {
                html!(<div><pre>{text!("{:?}", user)}</pre></div>)
            } else {
                html!(<div><a href={&*GH_LOGIN_URL}>"Login"</a></div>)
            } }
            </body>
        </html>
    );
    Ok(warp::reply::html(doc.to_string()))
}

async fn setup_authorized_route(
    info: github_client::oauth::AuthCodeQuery,
) -> Result<impl warp::Reply, warp::Rejection> {
    let token = github_client::oauth::exchange_code(
        &*GH_CLIENT_ID,
        &*GH_CLIENT_SECRET.unsecure(),
        REDIRECT_URI,
        info,
    )
    .await
    .map_err(reject)?;

    Ok(warp::reply::with_header(
        warp::redirect::temporary(Uri::from_static("/")),
        "set-cookie",
        format!(
            "token={}; secure; httponly; samesite=strict; path=/",
            token.access_token
        ),
    ))
}

async fn setup_installed_route() -> Result<impl warp::Reply, Infallible> {
    Ok("Setup: Installed")
}

async fn hooks_route(
    signature: String,
    event: String,
    body: Bytes,
) -> Result<impl warp::Reply, warp::Rejection> {
    let payload = github_hooks::deserialize(signature, event, body, &*GH_WEBHOOK_SECRET.unsecure())
        .map_err(reject)?;

    info!("Hook: {:?}", payload);
    match payload {
        github_hooks::Payload::CheckRun(check_run) => {
            let influxdb_db = influxdb_name(&check_run.repository);
            let client = influxdb_client::Client::new(
                &*INFLUXDB_BASE_URL,
                &influxdb_db,
                &*INFLUXDB_USERNAME,
                &*INFLUXDB_PASSWORD.unsecure(),
            )
            .map_err(reject)?;
            client
                .write(vec![
                    stats::Hook {
                        time: check_run.check_run.started_at,
                        r#type: stats::HookType::CheckRun,
                        commit_sha: check_run.check_run.head_sha.clone(),
                    }
                    .into_point(),
                    stats::build_from_check_run(check_run.check_run).into_point(),
                ])
                .await
                .map_err(reject)?;
        }
        github_hooks::Payload::GitHubAppAuthorization(_auth) => {}
        github_hooks::Payload::Ping(_ping) => {}
        github_hooks::Payload::Status(status) => {
            let influxdb_db = influxdb_name(&status.repository);
            let client = influxdb_client::Client::new(
                &*INFLUXDB_BASE_URL,
                &influxdb_db,
                &*INFLUXDB_USERNAME,
                &*INFLUXDB_PASSWORD.unsecure(),
            )
            .map_err(reject)?;
            client
                .write(vec![stats::Hook {
                    time: status.created_at,
                    r#type: stats::HookType::Status,
                    commit_sha: status.sha,
                }
                .into_point()])
                .await
                .map_err(reject)?;
        }
    };

    Ok(StatusCode::OK)
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let index = warp::get()
        .and(warp::cookie::optional("token"))
        .and_then(index_route);

    let setup_authorized = warp::get()
        .and(warp::path!("setup" / "authorized"))
        .and(warp::query::<github_client::oauth::AuthCodeQuery>())
        .and_then(setup_authorized_route);

    let setup_installed = warp::get()
        .and(warp::path!("setup" / "installed"))
        .and_then(setup_installed_route);

    let hooks = warp::post()
        .and(warp::path!("hooks"))
        .and(warp::cookie("X-Hub-Signature"))
        .and(warp::cookie("X-GitHub-Event"))
        .and(warp::body::bytes())
        .and_then(hooks_route);

    let routes = index
        .or(setup_authorized)
        .or(setup_installed)
        .or(hooks)
        .with(warp::log("website"));

    warp::serve(routes).run(([0, 0, 0, 0], 8888)).await
}
