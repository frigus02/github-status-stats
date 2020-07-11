pub struct Config {
    pub database_directory: String,
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
        database_directory: env("DATABASE_DIRECTORY"),
        otel_agent_endpoint: option_env("OTEL_AGENT_ENDPOINT"),
    }
}
