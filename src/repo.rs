use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    error::AppError,
    models::{
        auth::{LoginChallenge, LoginChallengeRow},
        email_log::{EmailLog, EmailLogRow},
        task::{Task, TaskRow},
        user::{User, UserRow},
    },
};

fn now() -> String {
    Utc::now().to_rfc3339()
}

// ---------------------------------------------------------------------
// Users
// ---------------------------------------------------------------------

pub async fn create_user(
    pool: &SqlitePool,
    full_name: &str,
    email: &str,
    password_hash: &str,
    role: &str,
) -> Result<User, AppError> {
    let id = Uuid::new_v4().to_string();
    let timestamp = now();

    sqlx::query(
        "INSERT INTO users (id, full_name, email, password_hash, role, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(full_name)
    .bind(email)
    .bind(password_hash)
    .bind(role)
    .bind(&timestamp)
    .bind(&timestamp)
    .execute(pool)
    .await?;

    get_user_by_id(pool, &Uuid::parse_str(&id).unwrap())
        .await?
        .ok_or_else(|| AppError::Internal("failed to read back created user".into()))
}

pub async fn get_user_by_email(pool: &SqlitePool, email: &str) -> Result<Option<User>, AppError> {
    let row = sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE email = ?")
        .bind(email)
        .fetch_optional(pool)
        .await?;

    row.map(User::try_from)
        .transpose()
        .map_err(|e| AppError::Internal(e.to_string()))
}

pub async fn get_user_by_id(pool: &SqlitePool, id: &Uuid) -> Result<Option<User>, AppError> {
    let row = sqlx::query_as::<_, UserRow>("SELECT * FROM users WHERE id = ?")
        .bind(id.to_string())
        .fetch_optional(pool)
        .await?;

    row.map(User::try_from)
        .transpose()
        .map_err(|e| AppError::Internal(e.to_string()))
}

// ---------------------------------------------------------------------
// Login challenges (2FA)
// ---------------------------------------------------------------------

pub async fn create_login_challenge(
    pool: &SqlitePool,
    user_id: &Uuid,
    code_hash: &str,
    expires_at: DateTime<Utc>,
) -> Result<LoginChallenge, AppError> {
    let id = Uuid::new_v4().to_string();
    let timestamp = now();

    sqlx::query(
        "INSERT INTO login_challenges (id, user_id, code_hash, expires_at, used, created_at)
         VALUES (?, ?, ?, ?, 0, ?)",
    )
    .bind(&id)
    .bind(user_id.to_string())
    .bind(code_hash)
    .bind(expires_at.to_rfc3339())
    .bind(&timestamp)
    .execute(pool)
    .await?;

    get_login_challenge(pool, &Uuid::parse_str(&id).unwrap())
        .await?
        .ok_or_else(|| AppError::Internal("failed to read back created login challenge".into()))
}

pub async fn get_login_challenge(
    pool: &SqlitePool,
    id: &Uuid,
) -> Result<Option<LoginChallenge>, AppError> {
    let row = sqlx::query_as::<_, LoginChallengeRow>("SELECT * FROM login_challenges WHERE id = ?")
        .bind(id.to_string())
        .fetch_optional(pool)
        .await?;

    row.map(LoginChallenge::try_from)
        .transpose()
        .map_err(|e| AppError::Internal(e.to_string()))
}

pub async fn mark_login_challenge_used(pool: &SqlitePool, id: &Uuid) -> Result<(), AppError> {
    sqlx::query("UPDATE login_challenges SET used = 1 WHERE id = ?")
        .bind(id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

// ---------------------------------------------------------------------
// Email logs (development email outbox)
// ---------------------------------------------------------------------

pub async fn create_email_log(
    pool: &SqlitePool,
    user_id: &Uuid,
    email: &str,
    code: &str,
    purpose: &str,
) -> Result<(), AppError> {
    let id = Uuid::new_v4().to_string();
    let timestamp = now();

    sqlx::query(
        "INSERT INTO email_logs (id, user_id, email, code, purpose, created_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(user_id.to_string())
    .bind(email)
    .bind(code)
    .bind(purpose)
    .bind(&timestamp)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_latest_email_log(
    pool: &SqlitePool,
    email: Option<&str>,
) -> Result<Option<EmailLog>, AppError> {
    let row = if let Some(email) = email {
        sqlx::query_as::<_, EmailLogRow>(
            "SELECT * FROM email_logs WHERE email = ? ORDER BY created_at DESC, id DESC LIMIT 1",
        )
        .bind(email)
        .fetch_optional(pool)
        .await?
    } else {
        sqlx::query_as::<_, EmailLogRow>(
            "SELECT * FROM email_logs ORDER BY created_at DESC, id DESC LIMIT 1",
        )
        .fetch_optional(pool)
        .await?
    };

    row.map(EmailLog::try_from)
        .transpose()
        .map_err(|e| AppError::Internal(e.to_string()))
}

// ---------------------------------------------------------------------
// Tasks
// ---------------------------------------------------------------------

pub async fn create_task(
    pool: &SqlitePool,
    title: &str,
    description: &str,
    priority: &str,
    created_by_id: &Uuid,
) -> Result<Task, AppError> {
    let id = Uuid::new_v4().to_string();
    let timestamp = now();

    sqlx::query(
        "INSERT INTO tasks (id, title, description, status, priority, created_by_id, assigned_to_id, created_at, updated_at)
         VALUES (?, ?, ?, 'todo', ?, ?, NULL, ?, ?)",
    )
    .bind(&id)
    .bind(title)
    .bind(description)
    .bind(priority)
    .bind(created_by_id.to_string())
    .bind(&timestamp)
    .bind(&timestamp)
    .execute(pool)
    .await?;

    get_task(pool, &Uuid::parse_str(&id).unwrap())
        .await?
        .ok_or_else(|| AppError::Internal("failed to read back created task".into()))
}

pub async fn get_task(pool: &SqlitePool, id: &Uuid) -> Result<Option<Task>, AppError> {
    let row = sqlx::query_as::<_, TaskRow>("SELECT * FROM tasks WHERE id = ?")
        .bind(id.to_string())
        .fetch_optional(pool)
        .await?;

    row.map(Task::try_from)
        .transpose()
        .map_err(|e| AppError::Internal(e.to_string()))
}

pub async fn assign_task(
    pool: &SqlitePool,
    task_id: &Uuid,
    assigned_to_id: &Uuid,
) -> Result<Task, AppError> {
    let timestamp = now();

    let result = sqlx::query("UPDATE tasks SET assigned_to_id = ?, updated_at = ? WHERE id = ?")
        .bind(assigned_to_id.to_string())
        .bind(&timestamp)
        .bind(task_id.to_string())
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("task {task_id} not found")));
    }

    get_task(pool, task_id)
        .await?
        .ok_or_else(|| AppError::Internal("failed to read back assigned task".into()))
}

pub async fn get_tasks_assigned_to(
    pool: &SqlitePool,
    user_id: &Uuid,
) -> Result<Vec<Task>, AppError> {
    let rows = sqlx::query_as::<_, TaskRow>(
        "SELECT * FROM tasks WHERE assigned_to_id = ? ORDER BY priority = 'high' DESC, priority = 'medium' DESC, created_at ASC",
    )
    .bind(user_id.to_string())
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(Task::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::Internal(e.to_string()))
}
