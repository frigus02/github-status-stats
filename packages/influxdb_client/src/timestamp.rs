use chrono::{DateTime, TimeZone};

#[derive(Debug)]
pub struct Timestamp {
    pub(crate) nanos: i64,
}

impl Timestamp {
    pub fn new<Tz: TimeZone>(datetime: &DateTime<Tz>) -> Timestamp {
        Timestamp {
            nanos: datetime.timestamp_nanos(),
        }
    }
}
