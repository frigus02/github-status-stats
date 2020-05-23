use secstr::SecUtf8;

pub struct Config {
    pub gh_app_id: String,
    pub gh_private_key: SecUtf8,
    pub store_url: String,
    pub honeycomb_api_key: SecUtf8,
    pub honeycomb_dataset: String,
}

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("env {}", name))
}

pub fn load() -> Config {
    Config {
        gh_app_id: env("GH_APP_ID"),
        gh_private_key: SecUtf8::from(env("GH_PRIVATE_KEY")),
        store_url: env("STORE_URL"),
        honeycomb_api_key: SecUtf8::from(env("HONEYCOMB_API_KEY")),
        honeycomb_dataset: env("HONEYCOMB_DATASET"),
    }
}
