use super::super::web_utils::header_as_string;
use actix_web::HttpRequest;
use bytes::Bytes;
use github_client::{CheckRunEvent, GitHubAppAuthorizationEvent, PingEvent, StatusEvent};
use hmac::{Hmac, Mac};
use once_cell::sync::Lazy;
use secstr::SecStr;
use sha1::Sha1;

static WEBHOOK_SECRET: Lazy<SecStr> =
    Lazy::new(|| SecStr::from(std::env::var("GH_WEBHOOK_SECRET").unwrap()));

#[derive(Debug)]
pub enum Payload {
    CheckRun(Box<CheckRunEvent>),
    GitHubAppAuthorization(Box<GitHubAppAuthorizationEvent>),
    Ping(Box<PingEvent>),
    Status(Box<StatusEvent>),
}

fn validate_signature(req: &HttpRequest, body: &Bytes) -> Result<(), String> {
    let signature = SecStr::from(header_as_string(req, "X-Hub-Signature")?);
    let mut mac = Hmac::<Sha1>::new_varkey(&*WEBHOOK_SECRET.unsecure())
        .expect("HMAC can take key of any size");
    mac.input(body);
    let result = SecStr::from(format!("sha1={:x}", mac.result().code()));
    if result == signature {
        Ok(())
    } else {
        Err("Signature doesn't match".to_string())
    }
}

pub fn deserialize(req: HttpRequest, body: Bytes) -> Result<Payload, String> {
    validate_signature(&req, &body)?;

    let event = header_as_string(&req, "X-GitHub-Event")?;
    match event {
        "check_run" => serde_json::from_slice::<CheckRunEvent>(&body)
            .map(|data| Payload::CheckRun(Box::new(data)))
            .map_err(|err| format!("Failed to deserialize check_run event: {}", err)),
        "github_app_authorization" => serde_json::from_slice::<GitHubAppAuthorizationEvent>(&body)
            .map(|data| Payload::GitHubAppAuthorization(Box::new(data)))
            .map_err(|err| {
                format!(
                    "Failed to deserialize github_app_authorization event: {}",
                    err
                )
            }),
        "ping" => serde_json::from_slice::<PingEvent>(&body)
            .map(|data| Payload::Ping(Box::new(data)))
            .map_err(|err| format!("Failed to deserialize ping event: {}", err)),
        "status" => serde_json::from_slice::<StatusEvent>(&body)
            .map(|data| Payload::Status(Box::new(data)))
            .map_err(|err| format!("Failed to deserialize status event: {}", err)),
        _ => Err(format!("Unsupported event {}", event)),
    }
}
