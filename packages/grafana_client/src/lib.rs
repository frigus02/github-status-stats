mod client;
mod models;

pub use client::Client;
pub use models::*;

pub const USER_AGENT: &str = concat!("github-status-stats/", env!("CARGO_PKG_VERSION"));
