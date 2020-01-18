use jsonwebtoken::{encode, Algorithm, Header};
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

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();
    let claims = Claims {
        // Issued at time
        iat: now,
        // JWT expiration time (10 minute maximum)
        exp: now + (10 * 60),
        // GitHub App's identifier
        iss: app_id,
    };

    let token = encode(&header, &claims, private_key_pem.as_bytes())?;
    Ok(token)
}
