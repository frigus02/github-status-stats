use chrono::{DateTime, FixedOffset};
use influxdb_client::{Client, FieldValue};

type BoxError = Box<dyn std::error::Error>;

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
