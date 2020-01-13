mod github;

use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use bytes::Bytes;
use listenfd::ListenFd;
use once_cell::sync::Lazy;
use secstr::SecStr;
use serde::Deserialize;
use typed_html::dom::DOMTree;
use typed_html::html;
use url::Url;

static CLIENT_ID: Lazy<String> = Lazy::new(|| std::env::var("GH_CLIENT_ID").unwrap());
#[allow(dead_code)]
static CLIENT_SECRET: Lazy<SecStr> =
    Lazy::new(|| SecStr::from(std::env::var("GH_CLIENT_SECRET").unwrap()));

static LOGIN_URL: Lazy<Url> = Lazy::new(|| {
    Url::parse_with_params(
        "https://github.com/login/oauth/authorize",
        &[
            ("client_id", &*CLIENT_ID.as_str()),
            ("redirect_uri", "https://fceac1a3.ngrok.io/setup/authorized"),
        ],
    )
    .unwrap()
});

async fn index() -> impl Responder {
    let doc: DOMTree<String> = html!(
        <html>
            <head>
                <title>"Status Stats"</title>
            </head>
            <body>
                <a href={LOGIN_URL.as_str()}>"Login"</a>
            </body>
        </html>
    );
    HttpResponse::Ok().body(doc.to_string())
}

#[derive(Deserialize)]
struct AuthorizationInfo {
    code: String,
    state: Option<String>,
}

async fn setup_authorized(info: web::Query<AuthorizationInfo>) -> impl Responder {
    println!("Code: {}; State: {:?}", info.code, info.state);
    HttpResponse::Ok().body("Setup: Authorized")
}

async fn setup_installed() -> impl Responder {
    HttpResponse::Ok().body("Setup: Installed")
}

async fn hooks(req: HttpRequest, body: Bytes) -> impl Responder {
    match github::hooks::deserialize(req, body) {
        Ok(payload) => {
            println!("Hook: {:?}", payload);
            HttpResponse::Ok()
        }
        Err(err) => {
            println!("Error reading hook: {:?}", err);
            HttpResponse::BadRequest()
        }
    }
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let mut listenfd = ListenFd::from_env();
    let mut server = HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(index))
            .route("/setup/authorized", web::get().to(setup_authorized))
            .route("/setup/installed", web::get().to(setup_installed))
            .route("/hooks", web::post().to(hooks))
    });

    server = if let Some(l) = listenfd.take_tcp_listener(0).unwrap() {
        server.listen(l)?
    } else {
        server.bind("127.0.0.1:8888")?
    };

    server.run().await
}
