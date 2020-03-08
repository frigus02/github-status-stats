use chrono::{DateTime, Utc};
use ghss_influxdb::{FieldValue, Point, Timestamp};
use std::collections::HashMap;

#[derive(Debug)]
pub struct Import {
    pub time: DateTime<Utc>,
    pub points: i64,
}

impl From<Import> for Point {
    fn from(import: Import) -> Self {
        let tags = HashMap::new();

        let mut fields = HashMap::new();
        fields.insert("points", FieldValue::Integer(import.points));

        Self {
            measurement: "import",
            tags,
            fields,
            timestamp: Timestamp::new(&import.time),
        }
    }
}
