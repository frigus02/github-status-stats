use super::github_queries::{get_github_user, GitHubUser};
use ghss_tracing::error_event;
use jsonwebtoken::{
    decode, encode, errors::Error as TokenError, Algorithm, DecodingKey, EncodingKey, Header,
    Validation,
};
use opentelemetry::api::{Context, Key, TraceContextExt};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::time::SystemTime;
use warp::Filter;

type BoxError = Box<dyn std::error::Error>;

#[derive(Debug, Serialize, Deserialize)]
struct RepositoryClaim {
    #[serde(rename = "i")]
    id: i32,
    #[serde(rename = "n")]
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    exp: u64,
    sub: String,
    #[serde(rename = "c_n")]
    name: String,
    #[serde(rename = "c_r")]
    repositories: Vec<RepositoryClaim>,
}

pub async fn generate(github_token: &str, secret: &[u8]) -> Result<String, BoxError> {
    let GitHubUser { user, repositories } = get_github_user(github_token).await?;

    let header = Header::new(Algorithm::HS256);

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();
    let claims = Claims {
        // Expiration time (24 hours)
        exp: now + (24 * 60 * 60),
        // User data
        sub: user.id.to_string(),
        name: user.name,
        repositories: repositories
            .into_iter()
            .map(|repository| RepositoryClaim {
                id: repository.id,
                name: repository.full_name,
            })
            .collect(),
    };

    let token = encode(&header, &claims, &EncodingKey::from_secret(secret))?;
    Ok(token)
}

#[derive(Debug)]
pub struct Repository {
    pub id: i32,
    pub name: String,
}

#[derive(Debug)]
pub struct User {
    pub id: String,
    pub name: String,
    pub repositories: Vec<Repository>,
}

fn validate(token: &str, secret: &[u8]) -> Result<User, TokenError> {
    let token = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(secret),
        &Validation::new(Algorithm::HS256),
    )?;
    Ok(User {
        id: token.claims.sub,
        name: token.claims.name,
        repositories: token
            .claims
            .repositories
            .into_iter()
            .map(|r| Repository {
                id: r.id,
                name: r.name,
            })
            .collect(),
    })
}

pub enum OptionalToken {
    Some(User),
    Expired,
    None,
}

pub fn optional_token(
    cookie_name: &'static str,
    token_secret: Vec<u8>,
) -> impl Filter<Extract = (OptionalToken,), Error = Infallible> + Clone {
    warp::cookie::optional(cookie_name).map(move |raw_token: Option<String>| match raw_token {
        Some(raw_token) => {
            let user = validate(&raw_token, token_secret.as_slice());
            let cx = Context::current();
            match user {
                Ok(user) => {
                    cx.span()
                        .set_attribute(Key::new("enduser.id").string(user.id.as_str()));
                    OptionalToken::Some(user)
                }
                Err(err) if err.to_string() == "ExpiredSignature" => OptionalToken::Expired,
                Err(err) => {
                    error_event(format!("token validation failed: {:?}", err));
                    OptionalToken::None
                }
            }
        }
        None => OptionalToken::None,
    })
}
