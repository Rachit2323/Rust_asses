use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    Staff,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserRole::Admin => "admin",
            UserRole::Staff => "staff",
        }
    }
}

impl std::str::FromStr for UserRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "admin" => Ok(UserRole::Admin),
            "staff" => Ok(UserRole::Staff),
            other => Err(format!("invalid role: {other}")),
        }
    }
}

/// Row as stored in / read from SQLite. Ids and timestamps are stored as
/// TEXT columns and converted to richer types in [`User`].
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserRow {
    pub id: String,
    pub full_name: String,
    pub email: String,
    pub password_hash: String,
    pub role: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Domain representation of a user. Mirrors the full `users` table; audit
/// timestamps are persisted and read back for completeness.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct User {
    pub id: Uuid,
    pub full_name: String,
    pub email: String,
    pub password_hash: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<UserRow> for User {
    type Error = anyhow::Error;

    fn try_from(row: UserRow) -> Result<Self, Self::Error> {
        Ok(User {
            id: Uuid::parse_str(&row.id)?,
            full_name: row.full_name,
            email: row.email,
            password_hash: row.password_hash,
            role: row.role.parse().map_err(|e: String| anyhow::anyhow!(e))?,
            created_at: DateTime::parse_from_rfc3339(&row.created_at)?.with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&row.updated_at)?.with_timezone(&Utc),
        })
    }
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub full_name: String,
    pub email: String,
    pub role: String,
}

impl From<&User> for UserResponse {
    fn from(user: &User) -> Self {
        UserResponse {
            id: user.id,
            full_name: user.full_name.clone(),
            email: user.email.clone(),
            role: user.role.as_str().to_string(),
        }
    }
}

/// Request body shape for creating an arbitrary user. The validation flow
/// uses the fixed `POST /seed/users` accounts, so this DTO is provided for
/// completeness / future extension.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct CreateUserRequest {
    pub full_name: String,
    pub email: String,
    pub password: String,
    pub role: String,
}

#[derive(Debug, Serialize)]
pub struct SeedUsersResponse {
    pub message: String,
    pub users: Vec<UserResponse>,
}
