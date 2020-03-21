use secstr::{SecStr, SecUtf8};
use std::convert::Infallible;
use std::sync::Arc;
use warp::Filter;

pub struct Config {
    pub host: String,
    pub cookie_name: &'static str,
    pub gh_redirect_uri: String,
    pub gh_client_id: String,
    pub gh_client_secret: SecUtf8,
    pub gh_webhook_secret: SecStr,
    pub influxdb_base_url: String,
    pub influxdb_admin_username: String,
    pub influxdb_admin_password: SecUtf8,
    pub influxdb_read_password: SecUtf8,
    pub token_secret: SecStr,
    pub honeycomb_api_key: SecUtf8,
    pub honeycomb_dataset: String,
}

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("env {}", name))
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
        influxdb_base_url: env("INFLUXDB_BASE_URL"),
        influxdb_admin_username: env("INFLUXDB_ADMIN_USERNAME"),
        influxdb_admin_password: SecUtf8::from(env("INFLUXDB_ADMIN_PASSWORD")),
        influxdb_read_password: SecUtf8::from(env("INFLUXDB_READ_PASSWORD")),
        token_secret: SecStr::from(env("TOKEN_SECRET")),
        honeycomb_api_key: SecUtf8::from(env("HONEYCOMB_API_KEY")),
        honeycomb_dataset: env("HONEYCOMB_DATASET"),
    }
}

pub fn with_config(
    config: Arc<Config>,
) -> impl Filter<Extract = (Arc<Config>,), Error = Infallible> + Clone {
    warp::any().map(move || config.clone())
}
