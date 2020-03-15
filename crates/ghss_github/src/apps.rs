use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::time::SystemTime;

#[derive(Debug, Serialize, Deserialize)]
struct Claims<'a> {
    iat: u64,
    exp: u64,
    iss: &'a str,
}

pub fn generate_jwt(app_id: &str, private_key_pem: &str) -> Result<String, Box<dyn Error>> {
    let header = Header::new(Algorithm::RS256);

    // To guard against time difference between this machine and the GitHub
    // server, issue token 30 seconds in the past.
    let offset_from_now = 30;

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs()
        - offset_from_now;
    let claims = Claims {
        // Issued at time
        iat: now,
        // JWT expiration time (10 minute maximum)
        exp: now + (10 * 60),
        // GitHub App's identifier
        iss: app_id,
    };

    let token = encode(
        &header,
        &claims,
        &EncodingKey::from_rsa_pem(private_key_pem.as_bytes())?,
    )?;
    Ok(token)
}
