use super::{FieldValue, Timestamp};
use std::collections::HashMap;

#[derive(Debug)]
pub struct Point {
    pub measurement: &'static str,
    pub tags: HashMap<&'static str, String>,
    pub fields: HashMap<&'static str, FieldValue>,
    pub timestamp: Timestamp,
}

impl Point {
    pub(crate) fn to_line(&self) -> String {
        let tags = self
            .tags
            .iter()
            .map(|(key, value)| {
                format!(
                    ",{}={}",
                    escape_tags_or_field_key(key),
                    escape_tags_or_field_key(value)
                )
            })
            .collect::<Vec<String>>()
            .join("");
        let fields = self
            .fields
            .iter()
            .map(|(key, value)| {
                format!(
                    "{}={}",
                    escape_tags_or_field_key(key),
                    match value {
                        FieldValue::String(s) => format!("\"{}\"", escape_string_field_value(s)),
                        FieldValue::Float(f) => f.to_string(),
                        FieldValue::Integer(i) => i.to_string(),
                        FieldValue::Boolean(b) => b.to_string(),
                    }
                )
            })
            .collect::<Vec<String>>()
            .join(",");
        format!(
            "{}{} {} {}",
            escape_measurement(self.measurement),
            tags,
            fields,
            self.timestamp.nanos
        )
    }
}

fn escape_string_field_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn escape_tags_or_field_key(value: &str) -> String {
    value
        .replace(',', "\\,")
        .replace('=', "\\=")
        .replace(' ', "\\ ")
}

fn escape_measurement(value: &str) -> String {
    value.replace(',', "\\,").replace(' ', "\\ ")
}
