use secstr::{SecStr, SecUtf8};

pub struct Config {
    pub host: String,
    pub cookie_name: &'static str,
    pub gh_redirect_uri: String,
    pub gh_client_id: String,
    pub gh_client_secret: SecUtf8,
    pub gh_webhook_secret: SecStr,
    pub store_url: String,
    pub token_secret: SecStr,
    pub otel_agent_endpoint: Option<String>,
}

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("env {}", name))
}

fn option_env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

pub fn load() -> Config {
    let host = env("HOST");
    let gh_redirect_uri = format!("{}/setup/authorized", host);
    Config {
        host,
        cookie_name: "token",
        gh_redirect_uri,
        gh_client_id: env("GH_CLIENT_ID"),
        gh_client_secret: SecUtf8::from(env("GH_CLIENT_SECRET")),
        gh_webhook_secret: SecStr::from(env("GH_WEBHOOK_SECRET")),
        store_url: env("STORE_URL"),
        token_secret: SecStr::from(env("TOKEN_SECRET")),
        otel_agent_endpoint: option_env("OTEL_AGENT_ENDPOINT"),
    }
}
