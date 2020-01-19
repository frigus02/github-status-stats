use chrono::{DateTime, FixedOffset};
use influxdb_client::{Client, FieldValue};

type BoxError = Box<dyn std::error::Error>;

pub async fn get_last_import(
    client: &Client<'_>,
) -> Result<Option<DateTime<FixedOffset>>, BoxError> {
    Ok(client
        .query("SELECT time FROM import ORDER BY time DESC LIMIT 1")
        .await?
        .results
        .pop()
        .ok_or("InfluxDB returned no result")?
        .series
        .and_then(|mut series| series.pop())
        .and_then(|mut series| series.values.pop())
        .and_then(|mut row| row.pop())
        .and_then(|time| match time {
            FieldValue::String(time) => DateTime::<FixedOffset>::parse_from_rfc3339(&time).ok(),
            _ => None,
        }))
}
