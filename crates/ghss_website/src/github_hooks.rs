use ghss_github::{CheckRunEvent, GitHubAppAuthorizationEvent, PingEvent, StatusEvent};
use hmac::{Hmac, Mac, NewMac};
use secstr::SecStr;
use sha1::Sha1;

type BoxError = Box<dyn std::error::Error>;

#[derive(Debug)]
pub enum Payload {
    CheckRun(Box<CheckRunEvent>),
    GitHubAppAuthorization(Box<GitHubAppAuthorizationEvent>),
    Installation,
    InstallationRepositories,
    Ping(Box<PingEvent>),
    Status(Box<StatusEvent>),
}

fn validate_signature(
    signature: impl Into<Vec<u8>>,
    body: &[u8],
    secret: &[u8],
) -> Result<(), BoxError> {
    let mut mac = Hmac::<Sha1>::new_varkey(secret).expect("HMAC can take key of any size");
    mac.update(body);
    let result = SecStr::from(format!("sha1={:x}", mac.finalize().into_bytes()));
    if result == SecStr::from(signature) {
        Ok(())
    } else {
        Err(From::from("Signature doesn't match".to_string()))
    }
}

pub fn deserialize(
    signature: impl Into<Vec<u8>>,
    event: &str,
    body: &[u8],
    secret: &[u8],
) -> Result<Payload, BoxError> {
    validate_signature(signature, body, secret)?;

    match event {
        "check_run" => Ok(serde_json::from_slice::<CheckRunEvent>(body)
            .map(|data| Payload::CheckRun(Box::new(data)))?),
        "github_app_authorization" => {
            Ok(serde_json::from_slice::<GitHubAppAuthorizationEvent>(body)
                .map(|data| Payload::GitHubAppAuthorization(Box::new(data)))?)
        }
        "ping" => {
            Ok(serde_json::from_slice::<PingEvent>(body)
                .map(|data| Payload::Ping(Box::new(data)))?)
        }
        "status" => Ok(serde_json::from_slice::<StatusEvent>(body)
            .map(|data| Payload::Status(Box::new(data)))?),
        "installation" => Ok(Payload::Installation),
        // integration_installation is deprecated; replaced by installation
        "integration_installation" => Ok(Payload::Installation),
        "installation_repositories" => Ok(Payload::InstallationRepositories),
        // integration_installation_repositories is deprecated; replaced by installation_repositories
        "integration_installation_repositories" => Ok(Payload::InstallationRepositories),
        _ => Err(From::from(format!("Unsupported event {}", event))),
    }
}
