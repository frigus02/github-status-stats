use chrono::{DateTime, FixedOffset, Utc};
use influxdb_client::{Client, FieldValue};
use log::info;
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
        .into_point(),
    );
    client.write(points).await
}

pub async fn get_last_import(
    client: &Client<'_>,
) -> Result<Option<DateTime<FixedOffset>>, BoxError> {
    Ok(client
        .query("SELECT * FROM import ORDER BY time DESC LIMIT 1")
        .await?
        .results
        .pop()
        .ok_or("InfluxDB returned no result")?
        .series
        .and_then(|mut series| series.pop())
        .and_then(|mut series| series.values.pop().map(|row| (row, series.index("time"))))
        .and_then(|(row, time_index)| time_index.map(|time_index| (row, time_index)))
        .map(|(mut row, time_index)| row.remove(time_index))
        .and_then(|time| match time {
            FieldValue::String(time) => DateTime::<FixedOffset>::parse_from_rfc3339(&time).ok(),
            _ => None,
        }))
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
        .results
        .pop()
        .and_then(|result| result.series)
        .and_then(|mut series| series.pop())
        .map(|series| series.values)
        .unwrap_or_else(Vec::new)
        .into_iter()
        .filter_map(|mut row| row.pop())
        .filter_map(|value| match value {
            FieldValue::String(value) => Some(value),
            _ => None,
        })
        .collect())
}
