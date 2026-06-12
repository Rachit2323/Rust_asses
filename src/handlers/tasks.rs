use axum::{extract::State, Json};
use sqlx::SqlitePool;

use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    models::task::{
        AssignTaskRequest, CacheInfo, CreateTaskRequest, Priority, Task, TaskResponse,
        ViewMyTasksResponse, ViewMyTasksSummary, ViewMyTasksUser,
    },
    repo,
    state::AppState,
};

async fn to_task_response(pool: &SqlitePool, task: &Task) -> AppResult<TaskResponse> {
    let assigned_to = match task.assigned_to_id {
        Some(id) => repo::get_user_by_id(pool, &id).await?.map(|u| u.email),
        None => None,
    };

    Ok(TaskResponse {
        id: task.id,
        title: task.title.clone(),
        description: task.description.clone(),
        status: task.status.as_str().to_string(),
        priority: task.priority.as_str().to_string(),
        assigned_to,
    })
}

/// `POST /tasks` - Admin only.
pub async fn create_task(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateTaskRequest>,
) -> AppResult<Json<TaskResponse>> {
    auth.require_admin()?;

    if req.title.trim().is_empty() {
        return Err(AppError::BadRequest("title must not be empty".into()));
    }

    let priority = req.priority.unwrap_or_else(|| "medium".to_string());
    priority
        .parse::<Priority>()
        .map_err(AppError::BadRequest)?;

    let description = req.description.unwrap_or_default();

    let task = repo::create_task(&state.db, &req.title, &description, &priority, &auth.user_id)
        .await?;

    Ok(Json(to_task_response(&state.db, &task).await?))
}

/// `POST /tasks/assign` - Admin only. Invalidates the assignee's
/// `view-my-tasks` cache so they immediately see the newly assigned task.
pub async fn assign_task(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<AssignTaskRequest>,
) -> AppResult<Json<TaskResponse>> {
    auth.require_admin()?;

    repo::get_user_by_id(&state.db, &req.assigned_to_id)
        .await?
        .ok_or_else(|| AppError::BadRequest("assigned_to_id does not refer to a known user".into()))?;

    let task = repo::assign_task(&state.db, &req.task_id, &req.assigned_to_id).await?;

    state.cache.invalidate(&req.assigned_to_id);

    Ok(Json(to_task_response(&state.db, &task).await?))
}

/// `GET /tasks/view-my-tasks`
///
/// Returns the tasks assigned to the authenticated user. Results are
/// cached per-user; the first call hits the database (`cache.hit = false`)
/// and subsequent calls within the TTL are served from the in-memory cache
/// (`cache.hit = true`) until invalidated by `POST /tasks/assign`.
pub async fn view_my_tasks(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<ViewMyTasksResponse>> {
    if let Some(cached) = state.cache.get(&auth.user_id) {
        let mut response: ViewMyTasksResponse = serde_json::from_str(&cached)
            .map_err(|e| AppError::Internal(format!("failed to deserialize cached response: {e}")))?;
        response.cache.hit = true;
        return Ok(Json(response));
    }

    let user = repo::get_user_by_id(&state.db, &auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let tasks = repo::get_tasks_assigned_to(&state.db, &auth.user_id).await?;

    let mut task_responses = Vec::with_capacity(tasks.len());
    for task in &tasks {
        task_responses.push(to_task_response(&state.db, task).await?);
    }

    let response = ViewMyTasksResponse {
        user: ViewMyTasksUser {
            email: user.email,
            role: user.role.as_str().to_string(),
        },
        summary: ViewMyTasksSummary {
            total_assigned_tasks: task_responses.len(),
        },
        tasks: task_responses,
        cache: CacheInfo { hit: false },
    };

    let serialized = serde_json::to_string(&response)
        .map_err(|e| AppError::Internal(format!("failed to serialize response for cache: {e}")))?;
    state.cache.set(auth.user_id, serialized);

    Ok(Json(response))
}
