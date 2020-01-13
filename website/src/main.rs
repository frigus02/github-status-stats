mod github;
mod web_utils;

use actix_web::cookie::{Cookie, SameSite};
use actix_web::{web, App, HttpMessage, HttpRequest, HttpResponse, HttpServer, Responder};
use bytes::Bytes;
use listenfd::ListenFd;
use typed_html::dom::DOMTree;
use typed_html::{html, text};

async fn index(req: HttpRequest) -> actix_web::Result<HttpResponse> {
    let user = if let Some(token) = req.cookie("token") {
        let client = github_client::Client::new(token.value())
            .map_err(|err| actix_web::error::ErrorBadRequest(err.to_string()))?;
        Some(
            client
                .get_user()
                .await
                .map_err(|err| actix_web::error::ErrorBadRequest(err.to_string()))?,
        )
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
                html!(<div><a href={github::auth::LOGIN_URL.as_str()}>"Login"</a></div>)
            } }
            </body>
        </html>
    );
    Ok(HttpResponse::Ok()
        .content_type("text/html")
        .body(doc.to_string()))
}

async fn setup_authorized(
    info: web::Query<github::auth::AuthCode>,
) -> actix_web::Result<HttpResponse> {
    let token = github::auth::exchange_code(info.into_inner())
        .await
        .map_err(|err| actix_web::error::ErrorBadRequest(err.to_string()))?;

    Ok(HttpResponse::TemporaryRedirect()
        .header("Location", "/")
        .cookie(
            Cookie::build("token", token.access_token)
                .path("/")
                .secure(true)
                .http_only(true)
                .same_site(SameSite::Strict)
                .finish(),
        )
        .finish())
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
