use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use shuttle_secrets::SecretStore;

use crate::{
    error::{Error, Result},
    types::User,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub exp: i64,
}

pub fn create_jwt(user: &User, secret_store: &SecretStore) -> Result<String> {
    let exp = Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .expect("Failed generating timestamp")
        .timestamp();

    let claims = Claims {
        sub: user.id.to_string(),
        username: user.username.clone(),
        exp,
    };
    let header = Header::new(Algorithm::HS512);
    let secret = secret_store
        .get("JWT_ENCODING_SECRET")
        .expect("Failed to get JWT encoding secret");
    encode(
        &header,
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|_| Error::JWTTokenCreationError)
}

pub fn validate_jwt(jwt: &str, secret_store: &SecretStore) -> Result<Claims> {
    let secret = secret_store
        .get("JWT_ENCODING_SECRET")
        .expect("Failed to get JWT encoding secret");
    let decoded = decode::<Claims>(
        jwt,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS512),
    )
    .map_err(|_| Error::JWTValidationError)?;
    let current_timestamp = Utc::now().timestamp();

    if decoded.claims.exp < current_timestamp {
        return Err(Error::AuthExpired);
    }

    Ok(decoded.claims)
}
