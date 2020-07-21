use super::USER_AGENT;
use reqwest::{Client, Url};
use serde::Deserialize;

type BoxError = Box<dyn std::error::Error + Sync + Send>;

#[derive(Deserialize)]
pub struct AuthCodeQuery {
    pub code: String,
    pub state: Option<String>,
}

#[derive(Deserialize)]
pub struct AuthToken {
    pub access_token: String,
    pub token_type: String,
}

pub fn login_url(client_id: &str, redirect_uri: &str, state: Option<String>) -> String {
    Url::parse_with_params(
        "https://github.com/login/oauth/authorize",
        &[
            ("client_id", client_id),
            ("redirect_uri", redirect_uri),
            ("state", state.as_ref().unwrap_or(&"".to_owned())),
        ],
    )
    .expect("cannot parse GitHub base url")
    .into_string()
}

pub async fn exchange_code(
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    code: &AuthCodeQuery,
) -> Result<AuthToken, BoxError> {
    let url = Url::parse("https://github.com/login/oauth/access_token")?;
    let res = Client::new()
        .post(url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .header(reqwest::header::ACCEPT, "application/json")
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", code.code.as_str()),
            ("redirect_uri", redirect_uri),
            ("state", code.state.as_ref().map_or("", |x| x.as_str())),
        ])
        .send()
        .await?
        .error_for_status()?;
    let data: AuthToken = res.json().await?;

    Ok(data)
}
