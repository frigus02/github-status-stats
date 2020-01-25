#[macro_use]
extern crate lazy_static;

mod github_hooks;
mod grafana_auth;
mod reverse_proxy;

use bytes::Bytes;
use handlebars::Handlebars;
use log::{error, info};
use reverse_proxy::ReverseProxy;
use secstr::{SecStr, SecUtf8};
use serde::Serialize;
use stats::influxdb_name;
use std::convert::Infallible;
use warp::{
    http::{Request, StatusCode, Uri},
    hyper::Body,
    Filter,
};

lazy_static! {
    static ref HOST: String = std::env::var("HOST").unwrap();
    static ref GH_REDIRECT_URI: String = format!("{}/setup/authorized", *HOST);
    static ref GH_CLIENT_ID: String = std::env::var("GH_CLIENT_ID").unwrap();
    static ref GH_CLIENT_SECRET: SecUtf8 =
        SecUtf8::from(std::env::var("GH_CLIENT_SECRET").unwrap());
    static ref GH_LOGIN_URL: String =
        github_client::oauth::login_url(&*GH_CLIENT_ID, &GH_REDIRECT_URI)
            .unwrap()
            .into_string();
    static ref GH_WEBHOOK_SECRET: SecStr =
        SecStr::from(std::env::var("GH_WEBHOOK_SECRET").unwrap());
    static ref INFLUXDB_BASE_URL: String = std::env::var("INFLUXDB_BASE_URL").unwrap();
    static ref INFLUXDB_USERNAME: String = std::env::var("INFLUXDB_USERNAME").unwrap();
    static ref INFLUXDB_PASSWORD: SecUtf8 =
        SecUtf8::from(std::env::var("INFLUXDB_PASSWORD").unwrap());
    static ref GRAFANA_BASE_URL: String = std::env::var("GRAFANA_BASE_URL").unwrap();
    static ref GRAFANA_CLIENT: grafana_client::Client = grafana_client::Client::new(
        GRAFANA_BASE_URL.clone(),
        std::env::var("GRAFANA_ADMIN_USERNAME").unwrap(),
        std::env::var("GRAFANA_ADMIN_PASSWORD").unwrap()
    )
    .unwrap();
    static ref GRAFANA_PROXY: ReverseProxy = ReverseProxy::new(&*GRAFANA_BASE_URL).unwrap();
}

#[derive(Debug)]
struct HttpError;

impl warp::reject::Reject for HttpError {}

fn reject(err: Box<dyn std::error::Error>) -> warp::Rejection {
    error!("route error {}", err);
    warp::reject::custom(HttpError)
}

fn raw_query_option() -> impl Filter<Extract = (Option<String>,), Error = Infallible> + Copy {
    warp::query::raw()
        .map(Some)
        .or(warp::any().map(|| None))
        .unify()
}

fn raw_request() -> impl Filter<Extract = (Request<Body>,), Error = warp::Rejection> + Copy {
    warp::method()
        .and(warp::path::full())
        .and(raw_query_option())
        .and(warp::header::headers_cloned())
        .and(warp::body::bytes())
        .map(
            |method, path: warp::filters::path::FullPath, query: Option<String>, headers, body| {
                let mut req = Request::builder()
                    .method(method)
                    .uri(format!(
                        "{}{}",
                        path.as_str(),
                        query.map_or("".to_owned(), |q| format!("?{}", q))
                    ))
                    .body(warp::hyper::body::Body::from(body))
                    .expect("request builder");
                *req.headers_mut() = headers;
                req
            },
        )
}

#[derive(Serialize)]
struct TemplateData<'a> {
    user: Option<grafana_auth::GitHubUser>,
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
            {{#if user}}
                <h2>Hello {{user.name}}!</h2>
                <ul>
                    {{#each user.repos}}
                        <li>
                            <a href=\"/_/?orgId={{id}}\">{{full_name}}</a>
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
        let user = grafana_auth::get_github_user(&token)
            .await
            .map_err(reject)?;

        TemplateData {
            user: Some(user),
            login_url: &*GH_LOGIN_URL,
        }
    } else {
        TemplateData {
            user: None,
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
    req: Request<Body>,
    token: String,
) -> Result<impl warp::Reply, warp::Rejection> {
    let login = grafana_auth::sync_user(&token, &*GRAFANA_CLIENT)
        .await
        .map_err(reject)?;
    let res = GRAFANA_PROXY
        .call_with_auth(req, login)
        .await
        .map_err(reject)?;
    Ok(res)
}

async fn dashboard_route(req: Request<Body>) -> Result<impl warp::Reply, Infallible> {
    let res = GRAFANA_PROXY.call(req).await;
    Ok(res)
}

async fn setup_authorized_route(
    info: github_client::oauth::AuthCodeQuery,
) -> Result<impl warp::Reply, warp::Rejection> {
    let token = github_client::oauth::exchange_code(
        &*GH_CLIENT_ID,
        &*GH_CLIENT_SECRET.unsecure(),
        &*GH_REDIRECT_URI,
        info,
    )
    .await
    .map_err(reject)?;

    Ok(warp::reply::with_header(
        warp::redirect::temporary(Uri::from_static("/")),
        "set-cookie",
        format!(
            "token={}; Path=/; SameSite=Strict; Secure; HttpOnly",
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
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let index = warp::get()
        .and(warp::path::end())
        .and(warp::cookie::optional("token"))
        .and_then(index_route);

    let dashboard_login = warp::path!("_" / "login" / ..)
        .and(raw_request())
        .and(warp::cookie("token"))
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

    warp::serve(routes).run(([0, 0, 0, 0], 8888)).await;
    Ok(())
}
