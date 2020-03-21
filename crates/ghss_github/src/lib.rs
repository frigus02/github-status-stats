mod apps;
mod call;
mod client;
mod models;
pub mod oauth;
mod page_links;

pub use client::Client;
pub use models::*;

pub const BASE_URL: &str = "https://api.github.com";
pub const USER_AGENT: &str = concat!("github-status-stats/", env!("CARGO_PKG_VERSION"));
