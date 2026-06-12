use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

/// Row as stored in / read from SQLite. This table acts as our local
/// "development email outbox" - in a real system this row would correspond
/// to an email sent to the user containing their 2FA code.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EmailLogRow {
    pub id: String,
    pub user_id: String,
    pub email: String,
    pub code: String,
    pub purpose: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct EmailLog {
    pub id: Uuid,
    #[allow(dead_code)]
    pub user_id: Uuid,
    pub email: String,
    pub code: String,
    pub purpose: String,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<EmailLogRow> for EmailLog {
    type Error = anyhow::Error;

    fn try_from(row: EmailLogRow) -> Result<Self, Self::Error> {
        Ok(EmailLog {
            id: Uuid::parse_str(&row.id)?,
            user_id: Uuid::parse_str(&row.user_id)?,
            email: row.email,
            code: row.code,
            purpose: row.purpose,
            created_at: DateTime::parse_from_rfc3339(&row.created_at)?.with_timezone(&Utc),
        })
    }
}

/// Development-only response shape for `GET /dev/email-logs/latest`.
#[derive(Debug, Serialize)]
pub struct EmailLogResponse {
    pub id: Uuid,
    pub to: String,
    pub purpose: String,
    pub code: String,
    pub created_at: DateTime<Utc>,
}

impl From<EmailLog> for EmailLogResponse {
    fn from(log: EmailLog) -> Self {
        EmailLogResponse {
            id: log.id,
            to: log.email,
            purpose: log.purpose,
            code: log.code,
            created_at: log.created_at,
        }
    }
}
