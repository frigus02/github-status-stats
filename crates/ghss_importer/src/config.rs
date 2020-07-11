use secstr::SecUtf8;

pub struct Config {
    pub gh_app_id: String,
    pub gh_private_key: SecUtf8,
    pub store_url: String,
    pub otel_agent_endpoint: Option<String>,
}

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("env {}", name))
}

fn option_env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

pub fn load() -> Config {
    Config {
        gh_app_id: env("GH_APP_ID"),
        gh_private_key: SecUtf8::from(env("GH_PRIVATE_KEY")),
        store_url: env("STORE_URL"),
        otel_agent_endpoint: option_env("OTEL_AGENT_ENDPOINT"),
    }
}
