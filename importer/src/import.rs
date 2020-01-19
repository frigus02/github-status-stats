use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use influxdb_client::{Client, FieldValue, Point, Timestamp};
use std::collections::HashMap;

type BoxError = Box<dyn std::error::Error>;

pub struct Import<Tz: TimeZone> {
    pub time: DateTime<Utc>,
    pub commits_since: DateTime<Tz>,
}

impl<Tz: TimeZone> Import<Tz> {
    pub fn to_point(self) -> Point {
        let tags = HashMap::new();

        let mut fields = HashMap::new();
        fields.insert(
            "commits_since",
            FieldValue::Integer(self.commits_since.timestamp()),
        );

        Point {
            measurement: "import",
            tags,
            fields,
            timestamp: Timestamp::new(&self.time),
        }
    }
}

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
