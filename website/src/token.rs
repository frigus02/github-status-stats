use super::github_queries::{get_github_user, GitHubUser};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

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

pub struct Repository {
    pub id: i32,
    pub name: String,
}

pub struct User {
    pub name: String,
    pub repositories: Vec<Repository>,
}

pub fn validate(token: &str, secret: &[u8]) -> Result<User, BoxError> {
    let token = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(secret),
        &Validation::new(Algorithm::HS256),
    )?;
    Ok(User {
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
