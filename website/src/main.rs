#[macro_use]
extern crate lazy_static;

mod github_hooks;
mod reverse_proxy;

use bytes::Bytes;
use handlebars::Handlebars;
use log::{error, info};
use reverse_proxy::ReverseProxy;
use secstr::{SecStr, SecUtf8};
use serde::Serialize;
use stats::influxdb_name;
use warp::{http, http::StatusCode, http::Uri, Filter};

const REDIRECT_URI: &str = "https://294c6b27.ngrok.io/setup/authorized";

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
    static ref GRAFANA_BASE_URL: String = std::env::var("GRAFANA_BASE_URL").unwrap();
    static ref GRAFANA_PROXY: ReverseProxy = ReverseProxy::new(&*GRAFANA_BASE_URL).unwrap();
}

#[derive(Debug)]
struct HttpError;

impl warp::reject::Reject for HttpError {}

fn reject(err: Box<dyn std::error::Error>) -> warp::Rejection {
    error!("route error {}", err);
    warp::reject::custom(HttpError)
}

#[derive(Serialize)]
struct LoggedInData {
    user: github_client::User,
    repos: Vec<github_client::Repository>,
}

#[derive(Serialize)]
struct TemplateData<'a> {
    data: Option<LoggedInData>,
    login_url: &'a str,
}

async fn index_route(token: Option<String>) -> Result<impl warp::Reply, warp::Rejection> {
    let template = "
        <!DOCTYPE html>
        <html>
        <head>
            <title>GitHub Status Stats</title>
        </head>
        <body>
            <h1>GitHub Status Stats</h1>
            {{#if data}}
                <h2>Hello {{data.user.name}}!</h2>
                <ul>
                    {{#each data.repos}}
                        <li>
                            <a href=\"/d/{{id}}\">{{full_name}}</a>
                        </li>
                    {{/each}}
                </ul>
                <a href=\"https://github.com/apps/status-stats\">Add repository</a>
            {{else}}
                <div><a href={{login_url}}>Login</a></div>
            {{/if}}
        </body>
        </html>
    ";

    let data = if let Some(token) = token {
        let client = github_client::Client::new(&token).map_err(reject)?;
        let user = client.get_user().await.map_err(reject)?;
        let installations = client.get_user_installations().await.map_err(reject)?;
        let mut repos = Vec::new();
        for installation in installations {
            let mut r = client
                .get_user_installation_repositories(installation.id)
                .await
                .map_err(reject)?;
            repos.append(&mut r);
        }

        TemplateData {
            data: Some(LoggedInData { user, repos }),
            login_url: &*GH_LOGIN_URL,
        }
    } else {
        TemplateData {
            data: None,
            login_url: &*GH_LOGIN_URL,
        }
    };

    let mut hb = Handlebars::new();
    hb.register_template_string("template.html", template)
        .unwrap();
    let render = hb
        .render("template.html", &data)
        .unwrap_or_else(|err| err.to_string());
    Ok(warp::reply::html(render))
}

async fn dashboard_login_route(
    method: http::Method,
    path: warp::filters::path::FullPath,
    headers: http::HeaderMap,
    body: bytes::Bytes,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut req = http::Request::builder()
        .method(method)
        .uri(path.as_str())
        .body(warp::hyper::body::Body::from(body))
        .expect("request builder");
    *req.headers_mut() = headers;

    let auth = reverse_proxy::Auth {
        login: "admin".to_owned(),
        name: "Admin".to_owned(),
        email: "admin@example.com".to_owned(),
    };

    let res = GRAFANA_PROXY.call(req, Some(auth)).await.map_err(reject)?;

    Ok(res)
}

async fn dashboard_route(
    method: http::Method,
    path: warp::filters::path::FullPath,
    headers: http::HeaderMap,
    body: bytes::Bytes,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut req = http::Request::builder()
        .method(method)
        .uri(path.as_str())
        .body(warp::hyper::body::Body::from(body))
        .expect("request builder");
    *req.headers_mut() = headers;

    let res = GRAFANA_PROXY.call(req, None).await.map_err(reject)?;

    Ok(res)
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
        .and(warp::path::end())
        .and(warp::cookie::optional("token"))
        .and_then(index_route);

    let request_data = warp::method()
        .and(warp::path::full())
        .and(warp::header::headers_cloned())
        .and(warp::body::bytes());
    let dashboard_login = warp::path!("_" / "login" / ..)
        .and(request_data)
        .and_then(dashboard_login_route);
    let dashboard = warp::path!("_" / ..)
        .and(request_data)
        .and_then(dashboard_route);

    let setup_authorized = warp::get()
        .and(warp::path!("setup" / "authorized"))
        .and(warp::query::<github_client::oauth::AuthCodeQuery>())
        .and_then(setup_authorized_route);

    let hooks = warp::post()
        .and(warp::path!("hooks"))
        .and(warp::cookie("X-Hub-Signature"))
        .and(warp::cookie("X-GitHub-Event"))
        .and(warp::body::bytes())
        .and_then(hooks_route);

    let routes = index
        .or(dashboard_login)
        .or(dashboard)
        .or(setup_authorized)
        .or(hooks)
        .with(warp::log("website"));

    warp::serve(routes).run(([0, 0, 0, 0], 8888)).await
}
