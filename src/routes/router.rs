use axum::{
    routing::{get, post},
    Router,
};

use crate::{handlers, state::AppState};

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/seed/users", post(handlers::seed::seed_users))
        .route("/auth/login", post(handlers::auth::login))
        .route("/auth/verify-2fa", post(handlers::auth::verify_2fa))
        .route("/dev/email-logs/latest", get(handlers::dev::latest_email_log))
        .route("/tasks", post(handlers::tasks::create_task))
        .route("/tasks/assign", post(handlers::tasks::assign_task))
        .route("/tasks/view-my-tasks", get(handlers::tasks::view_my_tasks))
        .with_state(state)
}
