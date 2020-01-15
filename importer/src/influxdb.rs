use chrono::{DateTime, FixedOffset};
use once_cell::sync::Lazy;
use serde::Serialize;
use std::collections::HashMap;

const BASE_URL: &str = "http://localhost:8086";
const USER_AGENT: &str = concat!("github-status-stats/", env!("CARGO_PKG_VERSION"));
const DB: &str = "db0";
const USER: &str = "user";
const PASSWORD: &str = "password";

static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static(USER_AGENT),
    );
    let auth = format!("{}:{}", USER, PASSWORD);
    headers.insert(
        reqwest::header::AUTHORIZATION,
        format!("Basic {}", base64::encode(&auth)).parse().unwrap(),
    );

    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap()
});

#[derive(Debug)]
pub struct Timestamp {
    nanos: i64,
}

impl Timestamp {
    pub fn new(datetime: &DateTime<FixedOffset>) -> Timestamp {
        Timestamp {
            nanos: datetime.timestamp_nanos(),
        }
    }
}

#[derive(Debug)]
pub enum FieldValue {
    #[allow(dead_code)]
    String(String),
    #[allow(dead_code)]
    Float(f32),
    Integer(i64),
    Boolean(bool),
}

#[derive(Debug)]
pub struct Point {
    pub measurement: &'static str,
    pub tags: HashMap<&'static str, String>,
    pub fields: HashMap<&'static str, FieldValue>,
    pub timestamp: Timestamp,
}

impl Point {
    fn to_line(&self) -> String {
        let tags = self
            .tags
            .iter()
            .map(|(key, value)| format!(",{}={}", key, value))
            .collect::<Vec<String>>()
            .join("");
        let fields = self
            .fields
            .iter()
            .map(|(key, value)| {
                format!(
                    "{}={}",
                    key,
                    match value {
                        FieldValue::String(s) => s.clone(),
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
            self.measurement, tags, fields, self.timestamp.nanos
        )
    }
}

#[derive(Debug, Serialize)]
struct Query {
    q: String,
}

pub async fn write(points: Vec<Point>) -> Result<(), String> {
    let raw_url = format!("{base}/write", base = BASE_URL);
    let url = reqwest::Url::parse_with_params(&raw_url, &[("db", DB)])
        .map_err(|err| format!("Error parsing URL: {:#?}", err))?;
    let body = points
        .into_iter()
        .map(|point| point.to_line())
        .collect::<Vec<String>>()
        .join("\n");

    println!("Calling {:#?}", url);
    (*CLIENT)
        .post(url)
        .body(body)
        .send()
        .await
        .map_err(|err| format!("Error sending request: {:#?}", err))?
        .error_for_status()
        .map_err(|err| format!("Call to InfluxDB returned: {:#?}", err))?;

    Ok(())
}
