use axum::{extract::State, Json};
use chrono::{Duration, Utc};

use crate::{
    auth::{jwt, otp, password::verify_password},
    error::{AppError, AppResult},
    models::auth::{LoginRequest, LoginResponse, Verify2faRequest, Verify2faResponse},
    repo,
    state::AppState,
};

/// `POST /auth/login`
///
/// Validates email/password. On success this does NOT return a JWT -
/// instead it creates a 2FA challenge, generates a one-time code, "sends"
/// it via the development email log, and returns the `login_challenge_id`
/// that must be passed to `POST /auth/verify-2fa`.
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> AppResult<Json<LoginResponse>> {
    let user = repo::get_user_by_email(&state.db, &req.email)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if !verify_password(&req.password, &user.password_hash)? {
        return Err(AppError::Unauthorized);
    }

    let code = otp::generate_code();
    let code_hash = otp::hash_code(&code);
    let expires_at = Utc::now() + Duration::seconds(state.config.two_factor_code_ttl_seconds);

    let challenge = repo::create_login_challenge(&state.db, &user.id, &code_hash, expires_at).await?;

    // "Send" the code via the development email outbox.
    repo::create_email_log(&state.db, &user.id, &user.email, &code, "2fa_login").await?;

    Ok(Json(LoginResponse {
        login_challenge_id: challenge.id,
        message: "Verification code sent. Check GET /dev/email-logs/latest for the code.".into(),
    }))
}

/// `POST /auth/verify-2fa`
///
/// Verifies the one-time code for a login challenge and, on success,
/// issues a JWT access token. Codes expire after
/// `TWO_FACTOR_CODE_TTL_SECONDS` and can only be used once.
pub async fn verify_2fa(
    State(state): State<AppState>,
    Json(req): Json<Verify2faRequest>,
) -> AppResult<Json<Verify2faResponse>> {
    let challenge = repo::get_login_challenge(&state.db, &req.login_challenge_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("invalid login_challenge_id".into()))?;

    if challenge.used {
        return Err(AppError::BadRequest("verification code already used".into()));
    }

    if challenge.expires_at < Utc::now() {
        return Err(AppError::BadRequest("verification code expired".into()));
    }

    if otp::hash_code(&req.code) != challenge.code_hash {
        return Err(AppError::Unauthorized);
    }

    repo::mark_login_challenge_used(&state.db, &challenge.id).await?;

    let user = repo::get_user_by_id(&state.db, &challenge.user_id)
        .await?
        .ok_or_else(|| AppError::Internal("user for challenge not found".into()))?;

    let access_token = jwt::create_token(&user, &state.config.jwt_secret, state.config.jwt_expiry_seconds)?;

    Ok(Json(Verify2faResponse {
        access_token,
        token_type: "Bearer".into(),
        expires_in: state.config.jwt_expiry_seconds,
    }))
}
