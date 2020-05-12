mod build;
mod hook;

pub use build::*;
use ghss_github::Repository;
pub use hook::*;

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
