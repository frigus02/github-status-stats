mod build;
mod hook;
mod import;

pub use build::*;
use github_client::{Repository, User};
pub use hook::*;
pub use import::*;

pub fn influxdb_name(repository: &Repository) -> String {
    format!("r{}", repository.id)
}

pub fn influxdb_name_unsafe(id: i32) -> String {
    format!("r{}", id)
}

pub fn influxdb_read_user(repository: &Repository) -> String {
    format!("u{}", repository.id)
}

pub fn influxdb_read_user_unsafe(id: i32) -> String {
    format!("u{}", id)
}

pub fn grafana_org_name(repository: &Repository) -> String {
    repository.full_name.clone()
}

pub fn grafana_user_login(user: &User) -> String {
    format!("{}", user.id)
}
