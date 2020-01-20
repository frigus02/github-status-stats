use bytes::Bytes;
use github_client::{CheckRunEvent, GitHubAppAuthorizationEvent, PingEvent, StatusEvent};
use hmac::{Hmac, Mac};
use secstr::SecStr;
use sha1::Sha1;

type BoxError = Box<dyn std::error::Error>;

lazy_static! {
    static ref WEBHOOK_SECRET: SecStr = SecStr::from(std::env::var("GH_WEBHOOK_SECRET").unwrap());
}

#[derive(Debug)]
pub enum Payload {
    CheckRun(Box<CheckRunEvent>),
    GitHubAppAuthorization(Box<GitHubAppAuthorizationEvent>),
    Ping(Box<PingEvent>),
    Status(Box<StatusEvent>),
}

fn validate_signature(signature: String, body: &Bytes) -> Result<(), BoxError> {
    let mut mac = Hmac::<Sha1>::new_varkey(&*WEBHOOK_SECRET.unsecure())
        .expect("HMAC can take key of any size");
    mac.input(body);
    let result = SecStr::from(format!("sha1={:x}", mac.result().code()));
    if result == SecStr::from(signature) {
        Ok(())
    } else {
        Err(From::from("Signature doesn't match".to_string()))
    }
}

pub fn deserialize(signature: String, event: String, body: Bytes) -> Result<Payload, BoxError> {
    validate_signature(signature, &body)?;

    match event.as_str() {
        "check_run" => Ok(serde_json::from_slice::<CheckRunEvent>(&body)
            .map(|data| Payload::CheckRun(Box::new(data)))?),
        "github_app_authorization" => {
            Ok(serde_json::from_slice::<GitHubAppAuthorizationEvent>(&body)
                .map(|data| Payload::GitHubAppAuthorization(Box::new(data)))?)
        }
        "ping" => {
            Ok(serde_json::from_slice::<PingEvent>(&body)
                .map(|data| Payload::Ping(Box::new(data)))?)
        }
        "status" => Ok(serde_json::from_slice::<StatusEvent>(&body)
            .map(|data| Payload::Status(Box::new(data)))?),
        _ => Err(From::from(format!("Unsupported event {}", event))),
    }
}
