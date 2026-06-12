use axum::{async_trait, extract::FromRequestParts, http::request::Parts};
use uuid::Uuid;

use crate::{auth::jwt::decode_token, error::AppError, models::user::UserRole, state::AppState};

/// Extracts and validates the JWT from the `Authorization: Bearer <token>`
/// header. Use this as a handler argument to require authentication; the
/// resulting `AuthUser` carries the caller's identity and role so handlers
/// can enforce role-based access control.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    /// Caller's email, carried from the JWT claims for logging / auditing.
    #[allow(dead_code)]
    pub email: String,
    pub role: UserRole,
}

impl AuthUser {
    pub fn require_admin(&self) -> Result<(), AppError> {
        if self.role == UserRole::Admin {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }
}

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(AppError::Unauthorized)?;

        let token = header
            .strip_prefix("Bearer ")
            .ok_or(AppError::Unauthorized)?;

        let claims = decode_token(token, &state.config.jwt_secret)?;

        let user_id = Uuid::parse_str(&claims.sub).map_err(|_| AppError::Unauthorized)?;
        let role: UserRole = claims.role.parse().map_err(|_| AppError::Unauthorized)?;

        Ok(AuthUser {
            user_id,
            email: claims.email,
            role,
        })
    }
}
