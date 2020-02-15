use chrono::{DateTime, FixedOffset, Utc};
use influxdb_client::Client;
use log::info;
use serde::Deserialize;
use stats::Import;

type BoxError = Box<dyn std::error::Error>;

pub async fn setup(
    client: &influxdb_client::Client<'_>,
    db: &str,
    user: &str,
    password: &str,
) -> Result<(), BoxError> {
    info!("Setup DB {} with user {}", db, user);
    client.query(&format!("CREATE DATABASE {}", db)).await?;
    client
        .query(&format!(
            "CREATE USER {} WITH PASSWORD '{}'",
            user, password
        ))
        .await?;
    client
        .query(&format!("GRANT READ ON {} TO {}", db, user))
        .await?;
    Ok(())
}

pub async fn import(
    client: &influxdb_client::Client<'_>,
    mut points: Vec<influxdb_client::Point>,
) -> Result<(), BoxError> {
    info!("Import {} points", points.len());
    points.push(
        Import {
            time: Utc::now(),
            points: points.len() as i64,
        }
        .into(),
    );
    client.write(points).await
}

#[derive(Deserialize)]
struct ImportRow {
    time: String,
}

pub async fn get_last_import(
    client: &Client<'_>,
) -> Result<Option<DateTime<FixedOffset>>, BoxError> {
    Ok(client
        .query("SELECT * FROM import ORDER BY time DESC LIMIT 1")
        .await?
        .into_single_result()?
        .into_single_series()
        .ok()
        .and_then(|series| series.into_rows::<ImportRow>().next())
        .and_then(|row| row.ok())
        .and_then(|row| DateTime::<FixedOffset>::parse_from_rfc3339(&row.time).ok()))
}

#[derive(Deserialize)]
struct HookRow {
    commit: String,
}

pub async fn get_status_hook_commits_since(
    client: &Client<'_>,
    since: &DateTime<FixedOffset>,
) -> Result<Vec<String>, BoxError> {
    Ok(client
        .query(&format!(
            "SELECT DISTINCT(commit) FROM hook WHERE type = \"status\" AND time >= \"{}\"",
            since.to_rfc3339()
        ))
        .await?
        .into_single_result()?
        .into_single_series()?
        .into_rows::<HookRow>()
        .filter_map(|row| row.ok().map(|row| row.commit))
        .collect())
}
