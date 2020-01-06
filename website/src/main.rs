mod github;

use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use bytes::Bytes;
use listenfd::ListenFd;

async fn index() -> impl Responder {
    HttpResponse::Ok().body("INDEX")
}

async fn setup_authorized() -> impl Responder {
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
