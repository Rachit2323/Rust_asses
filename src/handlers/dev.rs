use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

use crate::{
    error::{AppError, AppResult},
    models::email_log::EmailLogResponse,
    repo,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct LatestEmailLogQuery {
    /// Optional email filter, e.g. `?email=admin@example.com`. If omitted,
    /// the most recently sent email across all users is returned.
    pub email: Option<String>,
}

/// `GET /dev/email-logs/latest`
///
/// Development-only endpoint that exposes the most recently "sent" email
/// (i.e. 2FA verification code) so the validation workflow can be driven
/// entirely via curl/Postman without a real mailbox.
pub async fn latest_email_log(
    State(state): State<AppState>,
    Query(params): Query<LatestEmailLogQuery>,
) -> AppResult<Json<EmailLogResponse>> {
    let log = repo::get_latest_email_log(&state.db, params.email.as_deref())
        .await?
        .ok_or_else(|| AppError::NotFound("no email logs found".into()))?;

    Ok(Json(log.into()))
}
