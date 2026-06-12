use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use uuid::Uuid;

use crate::{
    error::AppError,
    models::{auth::Claims, user::User},
};

/// Creates a signed JWT access token for the given user.
pub fn create_token(user: &User, secret: &str, expiry_seconds: i64) -> Result<String, AppError> {
    let now = Utc::now();
    let claims = Claims {
        sub: user.id.to_string(),
        email: user.email.clone(),
        role: user.role.as_str().to_string(),
        iat: now.timestamp(),
        exp: (now + chrono::Duration::seconds(expiry_seconds)).timestamp(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("failed to create token: {e}")))
}

/// Decodes and validates a JWT access token, returning its claims.
pub fn decode_token(token: &str, secret: &str) -> Result<Claims, AppError> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|_| AppError::Unauthorized)
}

#[allow(dead_code)]
pub fn parse_user_id(claims: &Claims) -> Result<Uuid, AppError> {
    Uuid::parse_str(&claims.sub).map_err(|_| AppError::Unauthorized)
}
