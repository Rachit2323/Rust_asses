use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Todo => "todo",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::Done => "done",
        }
    }
}

impl std::str::FromStr for TaskStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "todo" => Ok(TaskStatus::Todo),
            "in_progress" => Ok(TaskStatus::InProgress),
            "done" => Ok(TaskStatus::Done),
            other => Err(format!("invalid status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    Medium,
    High,
}

impl Priority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Priority::Low => "low",
            Priority::Medium => "medium",
            Priority::High => "high",
        }
    }
}

impl std::str::FromStr for Priority {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "low" => Ok(Priority::Low),
            "medium" => Ok(Priority::Medium),
            "high" => Ok(Priority::High),
            other => Err(format!("invalid priority: {other}")),
        }
    }
}

/// Row as stored in / read from SQLite.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TaskRow {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: String,
    pub created_by_id: String,
    pub assigned_to_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Domain representation of a task. Mirrors the full `tasks` table; some
/// fields (audit timestamps, `created_by_id`) are persisted and read back
/// for completeness even though the current handlers don't surface them.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub priority: Priority,
    pub created_by_id: Uuid,
    pub assigned_to_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<TaskRow> for Task {
    type Error = anyhow::Error;

    fn try_from(row: TaskRow) -> Result<Self, Self::Error> {
        Ok(Task {
            id: Uuid::parse_str(&row.id)?,
            title: row.title,
            description: row.description,
            status: row.status.parse().map_err(|e: String| anyhow::anyhow!(e))?,
            priority: row
                .priority
                .parse()
                .map_err(|e: String| anyhow::anyhow!(e))?,
            created_by_id: Uuid::parse_str(&row.created_by_id)?,
            assigned_to_id: row
                .assigned_to_id
                .map(|s| Uuid::parse_str(&s))
                .transpose()?,
            created_at: DateTime::parse_from_rfc3339(&row.created_at)?.with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&row.updated_at)?.with_timezone(&Utc),
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AssignTaskRequest {
    pub task_id: Uuid,
    pub assigned_to_id: Uuid,
}

/// A task as represented in API responses. `assigned_to` is the assignee's
/// email so it matches the shape requested in the assignment spec.
#[derive(Debug, Serialize, Deserialize)]
pub struct TaskResponse {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: String,
    pub assigned_to: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ViewMyTasksUser {
    pub email: String,
    pub role: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ViewMyTasksSummary {
    pub total_assigned_tasks: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheInfo {
    pub hit: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ViewMyTasksResponse {
    pub user: ViewMyTasksUser,
    pub tasks: Vec<TaskResponse>,
    pub summary: ViewMyTasksSummary,
    pub cache: CacheInfo,
}
