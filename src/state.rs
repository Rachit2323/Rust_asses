use std::sync::Arc;

use sqlx::SqlitePool;

use crate::{cache::TasksCache, config::Config};

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub cache: Arc<TasksCache>,
    pub config: Arc<Config>,
}
