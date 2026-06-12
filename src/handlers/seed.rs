use axum::{extract::State, Json};

use crate::{
    auth::password::hash_password,
    error::AppResult,
    models::user::{SeedUsersResponse, UserResponse},
    repo,
    state::AppState,
};

/// Development convenience endpoint: creates the Admin and James Bond
/// accounts used throughout the validation workflow. Idempotent - if the
/// users already exist they are simply returned as-is.
///
/// Seeded credentials (documented in README.md):
///   admin@example.com     / Admin@12345    (role: admin)
///   jamesbond@example.com / JamesBond@12345 (role: staff)
pub async fn seed_users(State(state): State<AppState>) -> AppResult<Json<SeedUsersResponse>> {
    const SEEDS: &[(&str, &str, &str, &str)] = &[
        ("Admin", "admin@example.com", "Admin@12345", "admin"),
        (
            "James Bond",
            "jamesbond@example.com",
            "JamesBond@12345",
            "staff",
        ),
    ];

    let mut users = Vec::with_capacity(SEEDS.len());

    for (full_name, email, password, role) in SEEDS {
        let user = match repo::get_user_by_email(&state.db, email).await? {
            Some(existing) => existing,
            None => {
                let password_hash = hash_password(password)?;
                repo::create_user(&state.db, full_name, email, &password_hash, role).await?
            }
        };
        users.push(UserResponse::from(&user));
    }

    Ok(Json(SeedUsersResponse {
        message: "Admin and James Bond are ready. See README.md for seeded passwords.".into(),
        users,
    }))
}
