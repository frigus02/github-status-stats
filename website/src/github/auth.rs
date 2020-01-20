use reqwest::{Client, Url};
use secstr::SecUtf8;
use serde::Deserialize;
use std::error::Error;

const REDIRECT_URI: &str = "https://d2921223.ngrok.io/setup/authorized";

lazy_static! {
    static ref CLIENT_ID: String = std::env::var("GH_CLIENT_ID").unwrap();
    static ref CLIENT_SECRET: SecUtf8 = SecUtf8::from(std::env::var("GH_CLIENT_SECRET").unwrap());
    pub static ref LOGIN_URL: Url = Url::parse_with_params(
        "https://github.com/login/oauth/authorize",
        &[
            ("client_id", &*CLIENT_ID.as_str()),
            ("redirect_uri", REDIRECT_URI),
        ],
    )
    .unwrap();
}

#[derive(Deserialize)]
pub struct AuthCode {
    code: String,
    state: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AuthToken {
    pub access_token: String,
    pub token_type: String,
}

pub async fn exchange_code(code: AuthCode) -> Result<AuthToken, Box<dyn Error>> {
    let url = Url::parse("https://github.com/login/oauth/access_token")?;
    let res = Client::new()
        .post(url)
        .header(reqwest::header::USER_AGENT, github_client::USER_AGENT)
        .header(reqwest::header::ACCEPT, "application/json")
        .form(&[
            ("client_id", &*CLIENT_ID.as_str()),
            ("client_secret", &*CLIENT_SECRET.unsecure()),
            ("code", code.code.as_str()),
            ("redirect_uri", REDIRECT_URI),
            ("state", code.state.as_ref().map_or("", |x| x.as_str())),
        ])
        .send()
        .await?
        .error_for_status()?;
    let data: AuthToken = res.json().await?;

    Ok(data)
}
