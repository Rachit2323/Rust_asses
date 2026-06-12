pub mod auth;
pub mod cache;
pub mod config;
pub mod error;
pub mod handlers;
pub mod models;
pub mod repo;
pub mod routes;
pub mod state;

use std::{str::FromStr, sync::Arc};

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

use crate::{cache::TasksCache, config::Config, state::AppState};

/// Opens (creating if necessary) the SQLite pool described by the config and
/// runs all pending migrations. Shared by the binary and the test suite.
pub async fn init_state(config: Config) -> anyhow::Result<AppState> {
    let connect_options =
        SqliteConnectOptions::from_str(&config.database_url)?.create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(AppState {
        db: pool,
        cache: Arc::new(TasksCache::new(config.tasks_cache_ttl_seconds)),
        config: Arc::new(config),
    })
}

/// Builds the fully-wired axum router from application state.
pub fn build_app(state: AppState) -> axum::Router {
    routes::router::create_router(state)
}
