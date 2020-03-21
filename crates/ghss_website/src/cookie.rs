use chrono::{
    format::{DelayedFormat, StrftimeItems},
    DateTime, Duration, Utc,
};

fn expires(date: DateTime<Utc>) -> DelayedFormat<StrftimeItems<'static>> {
    date.format("%a, %d %b %Y %H:%M:%S GMT")
}

pub fn set(name: &str, value: &str) -> String {
    format!(
        "{}={}; Path=/; Expires={}; SameSite=Lax; Secure; HttpOnly",
        name,
        value,
        expires(Utc::now() + Duration::days(30))
    )
}

pub fn remove(name: &str) -> String {
    format!(
        "{}=; Path=/; Expires={}",
        name,
        expires(Utc::now() - Duration::days(1))
    )
}
