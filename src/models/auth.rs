use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Returned by `POST /auth/login`. Deliberately does NOT contain a JWT -
/// the caller must complete 2FA via `POST /auth/verify-2fa`.
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub login_challenge_id: Uuid,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct Verify2faRequest {
    pub login_challenge_id: Uuid,
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct Verify2faResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

/// Row as stored in / read from SQLite.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LoginChallengeRow {
    pub id: String,
    pub user_id: String,
    pub code_hash: String,
    pub expires_at: String,
    pub used: i64,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct LoginChallenge {
    pub id: Uuid,
    pub user_id: Uuid,
    pub code_hash: String,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
}

impl TryFrom<LoginChallengeRow> for LoginChallenge {
    type Error = anyhow::Error;

    fn try_from(row: LoginChallengeRow) -> Result<Self, Self::Error> {
        Ok(LoginChallenge {
            id: Uuid::parse_str(&row.id)?,
            user_id: Uuid::parse_str(&row.user_id)?,
            code_hash: row.code_hash,
            expires_at: DateTime::parse_from_rfc3339(&row.expires_at)?.with_timezone(&Utc),
            used: row.used != 0,
            created_at: DateTime::parse_from_rfc3339(&row.created_at)?.with_timezone(&Utc),
        })
    }
}

/// JWT claims embedded in the access token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject - the user's id.
    pub sub: String,
    pub email: String,
    pub role: String,
    /// Expiry (unix timestamp seconds).
    pub exp: i64,
    /// Issued-at (unix timestamp seconds).
    pub iat: i64,
}
