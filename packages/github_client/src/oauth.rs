use super::USER_AGENT;
use reqwest::{Client, Url};
use serde::Deserialize;

type BoxError = Box<dyn std::error::Error>;

#[derive(Deserialize)]
pub struct AuthCodeQuery {
    pub code: String,
    pub state: Option<String>,
}

impl std::fmt::Debug for AuthCodeQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("AuthCodeQuery")
            .field("code", &"***")
            .field("state", &self.state)
            .finish()
    }
}

#[derive(Deserialize)]
pub struct AuthToken {
    pub access_token: String,
    pub token_type: String,
}

impl std::fmt::Debug for AuthToken {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("AuthToken")
            .field("access_token", &"***")
            .field("token_type", &self.token_type)
            .finish()
    }
}

pub fn login_url(client_id: &str, redirect_uri: &str) -> Result<Url, BoxError> {
    Ok(Url::parse_with_params(
        "https://github.com/login/oauth/authorize",
        &[("client_id", client_id), ("redirect_uri", redirect_uri)],
    )?)
}

pub async fn exchange_code(
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    code: AuthCodeQuery,
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
