use chrono::{DateTime, FixedOffset, SecondsFormat};
use serde::de::{Deserializer, Error, Visitor};
use serde::ser::Serializer;
use std::fmt;

pub fn serialize<S>(datetime: &DateTime<FixedOffset>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&datetime.to_rfc3339_opts(SecondsFormat::Secs, true))
}

struct DateTimeVisitor;

impl<'de> Visitor<'de> for DateTimeVisitor {
    type Value = DateTime<FixedOffset>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an rfc3339 date and time string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        DateTime::parse_from_rfc3339(v).map_err(|err| E::custom(format!("{:#?}", err)))
    }
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<FixedOffset>, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_str(DateTimeVisitor)
}
