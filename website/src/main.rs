#[macro_use]
extern crate lazy_static;

mod github_hooks;
mod grafana_auth;
mod reverse_proxy;

use bytes::Bytes;
use handlebars::Handlebars;
use log::info;
use reverse_proxy::ReverseProxy;
use secstr::{SecStr, SecUtf8};
use serde::Serialize;
use stats::{influxdb_name, Build};
use std::convert::Infallible;
use warp::{
    http::{Request, Response, StatusCode},
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
    static ref INFLUXDB_ADMIN_USERNAME: String = std::env::var("INFLUXDB_ADMIN_USERNAME").unwrap();
    static ref INFLUXDB_ADMIN_PASSWORD: SecUtf8 =
        SecUtf8::from(std::env::var("INFLUXDB_ADMIN_PASSWORD").unwrap());
    static ref GRAFANA_BASE_URL: String = std::env::var("GRAFANA_BASE_URL").unwrap();
    static ref GRAFANA_CLIENT: grafana_client::Client = grafana_client::Client::new(
        GRAFANA_BASE_URL.clone(),
        &std::env::var("GRAFANA_ADMIN_USERNAME").unwrap(),
        &std::env::var("GRAFANA_ADMIN_PASSWORD").unwrap()
    )
    .unwrap();
    static ref GRAFANA_PROXY: ReverseProxy = ReverseProxy::new(&*GRAFANA_BASE_URL).unwrap();
}

fn raw_query_option() -> impl Filter<Extract = (Option<String>,), Error = Infallible> + Clone {
    warp::query::raw()
        .map(Some)
        .or(warp::any().map(|| None))
        .unify()
}

fn raw_request() -> impl Filter<Extract = (Request<Body>,), Error = warp::Rejection> + Clone {
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

fn new_error_res(status: StatusCode) -> Response<Body> {
    Response::builder()
        .status(status)
        .body(Body::from(status.to_string()))
        .unwrap()
}

#[derive(Serialize)]
struct TemplateData<'a> {
    user: Option<grafana_auth::GitHubUser>,
    login_url: &'a str,
}

async fn index_route(token: Option<String>) -> Result<impl warp::Reply, Infallible> {
    // TODO: Resolve repository id (== org name) to org id by querying grafana
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
                    {{#each user.repositories}}
                        <li>
                            <a href=\"/_/\">{{full_name}}</a>
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
        grafana_auth::get_github_user(&token)
            .await
            .map(|user| TemplateData {
                user: Some(user),
                login_url: &*GH_LOGIN_URL,
            })
    } else {
        Ok(TemplateData {
            user: None,
            login_url: &*GH_LOGIN_URL,
        })
    };

    match data {
        Ok(data) => {
            let mut hb = Handlebars::new();
            hb.register_template_string("template.html", template)
                .unwrap();
            let render = hb
                .render("template.html", &data)
                .unwrap_or_else(|err| err.to_string());
            Ok(warp::reply::with_status(
                warp::reply::html(render),
                StatusCode::OK,
            ))
        }
        Err(err) => Ok(warp::reply::with_status(
            warp::reply::html(err.to_string()),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

async fn dashboard_login_route(
    req: Request<Body>,
    token: Option<String>,
) -> Result<impl warp::Reply, Infallible> {
    let token = match token {
        Some(token) => token,
        None => return Ok(new_error_res(StatusCode::UNAUTHORIZED)),
    };

    let login = grafana_auth::sync_user(&token, &*GRAFANA_CLIENT)
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
