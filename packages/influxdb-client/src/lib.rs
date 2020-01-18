use chrono::{DateTime, FixedOffset};
use reqwest::Url;
use std::collections::HashMap;

pub const USER_AGENT: &str = concat!("github-status-stats/", env!("CARGO_PKG_VERSION"));

type BoxError = Box<dyn std::error::Error>;

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

pub struct Client<'a> {
    client: reqwest::Client,
    base_url: &'a str,
    db: &'a str,
}

impl Client<'_> {
    pub fn new<'a>(
        base_url: &'a str,
        db: &'a str,
        username: &str,
        password: &str,
    ) -> Result<Client<'a>, BoxError> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            reqwest::header::HeaderValue::from_static(USER_AGENT),
        );
        let auth = format!("{}:{}", username, password);
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Basic {}", base64::encode(&auth)).parse()?,
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Client {
            client,
            base_url,
            db,
        })
    }

    pub async fn write(&self, points: Vec<Point>) -> Result<(), BoxError> {
        let raw_url = format!("{base}/write", base = &self.base_url);
        let url = Url::parse_with_params(&raw_url, &[("db", &self.db)])?;
        let body = points
            .into_iter()
            .map(|point| point.to_line())
            .collect::<Vec<String>>()
            .join("\n");
        &self
            .client
            .post(url)
            .body(body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}
