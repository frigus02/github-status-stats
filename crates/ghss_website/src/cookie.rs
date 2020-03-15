use chrono::{Duration, Utc};

pub fn set(name: &str, value: &str) -> String {
    format!("{}={}; Path=/; SameSite=Lax; Secure; HttpOnly", name, value)
}

pub fn remove(name: &str) -> String {
    format!(
        "{}=; Path=/; Expires={}",
        name,
        (Utc::now() - Duration::days(1)).format("%a, %d %b %Y %H:%M:%S GMT")
    )
}
