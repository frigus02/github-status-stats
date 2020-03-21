use secstr::SecUtf8;

pub struct Config {
    pub gh_app_id: String,
    pub gh_private_key: SecUtf8,
    pub influxdb_base_url: String,
    pub influxdb_admin_username: String,
    pub influxdb_admin_password: SecUtf8,
    pub influxdb_read_password: SecUtf8,
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
        influxdb_base_url: env("INFLUXDB_BASE_URL"),
        influxdb_admin_username: env("INFLUXDB_ADMIN_USERNAME"),
        influxdb_admin_password: SecUtf8::from(env("INFLUXDB_ADMIN_PASSWORD")),
        influxdb_read_password: SecUtf8::from(env("INFLUXDB_READ_PASSWORD")),
        honeycomb_api_key: SecUtf8::from(env("HONEYCOMB_API_KEY")),
        honeycomb_dataset: env("HONEYCOMB_DATASET"),
    }
}
