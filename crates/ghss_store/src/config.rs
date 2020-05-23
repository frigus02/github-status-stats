use secstr::SecUtf8;

pub struct Config {
    pub database_directory: String,
    pub honeycomb_api_key: SecUtf8,
    pub honeycomb_dataset: String,
}

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("env {}", name))
}

pub fn load() -> Config {
    Config {
        database_directory: env("DATABASE_DIRECTORY"),
        honeycomb_api_key: SecUtf8::from(env("HONEYCOMB_API_KEY")),
        honeycomb_dataset: env("HONEYCOMB_DATASET"),
    }
}
