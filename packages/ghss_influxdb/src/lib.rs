mod field_value;
mod point;
mod query;
mod timestamp;

pub use field_value::FieldValue;
use ghss_tracing::debug;
pub use point::Point;
use query::{Query, QueryResponse};
use reqwest::Url;

pub use timestamp::Timestamp;

pub const USER_AGENT: &str = concat!("github-status-stats/", env!("CARGO_PKG_VERSION"));

type BoxError = Box<dyn std::error::Error>;

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

    pub async fn query(&self, q: &str) -> Result<QueryResponse, BoxError> {
        let raw_url = format!("{base}/query", base = &self.base_url);
        let url = Url::parse_with_params(&raw_url, &[("db", &self.db)])?;
        debug!(request.method = "POST", request.url = %url, request.body.q = q, "influxdb request");
        let result = self
            .client
            .post(url)
            .form(&Query { q })
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(result)
    }

    pub async fn write(&self, points: Vec<Point>) -> Result<(), BoxError> {
        let raw_url = format!("{base}/write", base = &self.base_url);
        let url = Url::parse_with_params(&raw_url, &[("db", &self.db)])?;
        let body = points
            .into_iter()
            .map(|point| point.to_line())
            .collect::<Vec<String>>()
            .join("\n");
        debug!(request.method = "POST", request.url = %url, request.body = %body, "influxdb request");
        self.client
            .post(url)
            .body(body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}
