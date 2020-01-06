use actix_web::HttpRequest;
use bytes::Bytes;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Config {
    pub content_type: String,
    pub insecure_ssl: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Hook {
    pub r#type: String,
    pub id: i32,
    pub name: String,
    pub active: bool,
    pub events: Vec<String>,
    pub config: Config,
    pub updated_at: String,
    pub created_at: String,
    pub app_id: i32,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Repository {
    pub id: i32,
    pub node_id: String,
    pub name: String,
    pub full_name: String,
    pub private: bool,
}

#[derive(Debug)]
pub enum Payload {
    Ping(PingPayload),
    Status(StatusPayload),
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct PingPayload {
    pub zen: String,
    pub hook_id: i32,
    pub hook: Hook,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct StatusPayload {
    pub id: i32,
    pub sha: String,
    pub name: String,
    pub target_url: String,
    pub context: String,
    pub description: String,
    pub state: String,
    // commit
    // branches
    pub repository: Repository,
    // sender
    pub updated_at: String,
    pub created_at: String,
}

pub fn deserialize(req: HttpRequest, body: Bytes) -> Result<Payload, String> {
    let header_name = "X-GitHub-Event";
    let event = req
        .headers()
        .get(header_name)
        .ok_or(format!("Header {} missing", header_name))
        .and_then(|header| {
            header
                .to_str()
                .map_err(|err| format!("Header {} not readable: {}", header_name, err))
        })?;

    match event {
        "ping" => serde_json::from_slice::<PingPayload>(&body)
            .map(|data| Payload::Ping(data))
            .map_err(|err| format!("Failed to deserialize ping event: {}", err)),
        "status" => serde_json::from_slice::<StatusPayload>(&body)
            .map(|data| Payload::Status(data))
            .map_err(|err| format!("Failed to deserialize status event: {}", err)),
        _ => Err(format!("Unsupported event {}", event)),
    }
}
