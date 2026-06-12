//! End-to-end integration tests that exercise the full assignment workflow
//! against the real axum router backed by a throwaway SQLite database.

use assement::{build_app, config::Config, init_state};
use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt; // for `oneshot`

const ADMIN_EMAIL: &str = "admin@example.com";
const ADMIN_PASS: &str = "Admin@12345";
const JB_EMAIL: &str = "jamesbond@example.com";
const JB_PASS: &str = "JamesBond@12345";

/// Builds a fresh app backed by a unique temp SQLite file. `code_ttl` lets
/// individual tests force-expire 2FA codes (use a negative value).
async fn test_app(code_ttl: i64, cache_ttl: u64) -> Router {
    let db_path = std::env::temp_dir().join(format!("assement_test_{}.db", uuid::Uuid::new_v4()));
    let db_url = format!("sqlite://{}", db_path.display());

    let config = Config {
        database_url: db_url,
        jwt_secret: "test-secret".into(),
        jwt_expiry_seconds: 3600,
        two_factor_code_ttl_seconds: code_ttl,
        tasks_cache_ttl_seconds: cache_ttl,
        port: 0,
    };

    let state = init_state(config).await.expect("init state");
    build_app(state)
}

async fn send(app: &Router, req: Request<Body>) -> (StatusCode, Value) {
    let resp = app.clone().oneshot(req).await.expect("request");
    let status = resp.status();
    let bytes = resp.into_body().collect().await.expect("body").to_bytes();
    let value: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, value)
}

fn post(path: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn post_auth(path: &str, token: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn get(path: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(path)
        .body(Body::empty())
        .unwrap()
}

fn get_auth(path: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(path)
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

/// Seeds users and returns their ids: (admin_id, james_bond_id).
async fn seed(app: &Router) -> (String, String) {
    let (status, body) = send(app, post("/seed/users", json!({}))).await;
    assert_eq!(status, StatusCode::OK);
    let users = body["users"].as_array().unwrap();
    (
        users[0]["id"].as_str().unwrap().to_string(),
        users[1]["id"].as_str().unwrap().to_string(),
    )
}

/// Starts a login and returns the `login_challenge_id`.
async fn start_login(app: &Router, email: &str, password: &str) -> String {
    let (status, body) = send(
        app,
        post("/auth/login", json!({ "email": email, "password": password })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "login failed: {body}");
    // First step must NOT hand out a JWT.
    assert!(
        body.get("access_token").is_none(),
        "login must not return a JWT directly"
    );
    body["login_challenge_id"].as_str().unwrap().to_string()
}

/// Reads the latest 2FA code "emailed" to a user.
async fn latest_code(app: &Router, email: &str) -> String {
    let (status, body) = send(app, get(&format!("/dev/email-logs/latest?email={email}"))).await;
    assert_eq!(status, StatusCode::OK);
    body["code"].as_str().unwrap().to_string()
}

/// Full login + 2FA, returning the JWT access token.
async fn login(app: &Router, email: &str, password: &str) -> String {
    let challenge = start_login(app, email, password).await;
    let code = latest_code(app, email).await;
    let (status, body) = send(
        app,
        post(
            "/auth/verify-2fa",
            json!({ "login_challenge_id": challenge, "code": code }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "verify-2fa failed: {body}");
    body["access_token"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn seed_creates_admin_and_staff() {
    let app = test_app(300, 60).await;
    let (status, body) = send(&app, post("/seed/users", json!({}))).await;
    assert_eq!(status, StatusCode::OK);
    let users = body["users"].as_array().unwrap();
    assert_eq!(users.len(), 2);
    assert_eq!(users[0]["role"], "admin");
    assert_eq!(users[1]["role"], "staff");
}

#[tokio::test]
async fn login_returns_challenge_not_jwt_then_verifies() {
    let app = test_app(300, 60).await;
    seed(&app).await;

    let challenge = start_login(&app, ADMIN_EMAIL, ADMIN_PASS).await;
    let code = latest_code(&app, ADMIN_EMAIL).await;

    let (status, body) = send(
        &app,
        post(
            "/auth/verify-2fa",
            json!({ "login_challenge_id": challenge, "code": code }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["access_token"].as_str().unwrap().len() > 20);
}

#[tokio::test]
async fn wrong_password_is_unauthorized() {
    let app = test_app(300, 60).await;
    seed(&app).await;
    let (status, _) = send(
        &app,
        post(
            "/auth/login",
            json!({ "email": ADMIN_EMAIL, "password": "wrong" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn incorrect_2fa_code_is_rejected() {
    let app = test_app(300, 60).await;
    seed(&app).await;
    let challenge = start_login(&app, ADMIN_EMAIL, ADMIN_PASS).await;

    let (status, _) = send(
        &app,
        post(
            "/auth/verify-2fa",
            json!({ "login_challenge_id": challenge, "code": "000000" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn reused_2fa_code_is_rejected() {
    let app = test_app(300, 60).await;
    seed(&app).await;
    let challenge = start_login(&app, ADMIN_EMAIL, ADMIN_PASS).await;
    let code = latest_code(&app, ADMIN_EMAIL).await;

    // First use succeeds.
    let (status, _) = send(
        &app,
        post(
            "/auth/verify-2fa",
            json!({ "login_challenge_id": challenge, "code": code }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Second use of the same challenge/code is rejected.
    let (status, _) = send(
        &app,
        post(
            "/auth/verify-2fa",
            json!({ "login_challenge_id": challenge, "code": code }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn expired_2fa_code_is_rejected() {
    // Negative TTL => the challenge is already expired when created.
    let app = test_app(-1, 60).await;
    seed(&app).await;
    let challenge = start_login(&app, ADMIN_EMAIL, ADMIN_PASS).await;
    let code = latest_code(&app, ADMIN_EMAIL).await;

    let (status, _) = send(
        &app,
        post(
            "/auth/verify-2fa",
            json!({ "login_challenge_id": challenge, "code": code }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn full_workflow_with_roles_and_caching() {
    let app = test_app(300, 60).await;
    let (_admin_id, jb_id) = seed(&app).await;

    let admin_token = login(&app, ADMIN_EMAIL, ADMIN_PASS).await;

    // Create exactly 5 tasks.
    let priorities = ["high", "medium", "low", "high", "medium"];
    let mut task_ids = Vec::new();
    for (i, prio) in priorities.iter().enumerate() {
        let (status, body) = send(
            &app,
            post_auth(
                "/tasks",
                &admin_token,
                json!({ "title": format!("Task {}", i + 1), "priority": prio }),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        task_ids.push(body["id"].as_str().unwrap().to_string());
    }
    assert_eq!(task_ids.len(), 5);

    // Assign exactly 3 to James Bond (high, medium, low).
    for id in task_ids.iter().take(3) {
        let (status, _) = send(
            &app,
            post_auth(
                "/tasks/assign",
                &admin_token,
                json!({ "task_id": id, "assigned_to_id": jb_id }),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    let jb_token = login(&app, JB_EMAIL, JB_PASS).await;

    // James Bond cannot create tasks -> 403.
    let (status, _) = send(
        &app,
        post_auth(
            "/tasks",
            &jb_token,
            json!({ "title": "nope", "priority": "high" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // First view -> from DB, cache.hit = false, exactly 3 tasks.
    let (status, body) = send(&app, get_auth("/tasks/view-my-tasks", &jb_token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["user"]["email"], JB_EMAIL);
    assert_eq!(body["user"]["role"], "staff");
    assert_eq!(body["summary"]["total_assigned_tasks"], 3);
    assert_eq!(body["tasks"].as_array().unwrap().len(), 3);
    assert_eq!(body["cache"]["hit"], false);
    for t in body["tasks"].as_array().unwrap() {
        assert_eq!(t["assigned_to"], JB_EMAIL);
    }

    // Second identical view -> from cache, cache.hit = true.
    let (status, body) = send(&app, get_auth("/tasks/view-my-tasks", &jb_token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["cache"]["hit"], true);
    assert_eq!(body["summary"]["total_assigned_tasks"], 3);
}

#[tokio::test]
async fn assigning_a_task_invalidates_the_cache() {
    let app = test_app(300, 60).await;
    let (_admin_id, jb_id) = seed(&app).await;
    let admin_token = login(&app, ADMIN_EMAIL, ADMIN_PASS).await;

    // Create + assign 2 tasks first.
    let mut ids = Vec::new();
    for prio in ["high", "medium", "low"] {
        let (_s, body) = send(
            &app,
            post_auth(
                "/tasks",
                &admin_token,
                json!({ "title": "t", "priority": prio }),
            ),
        )
        .await;
        ids.push(body["id"].as_str().unwrap().to_string());
    }
    for id in ids.iter().take(2) {
        send(
            &app,
            post_auth(
                "/tasks/assign",
                &admin_token,
                json!({ "task_id": id, "assigned_to_id": jb_id }),
            ),
        )
        .await;
    }

    let jb_token = login(&app, JB_EMAIL, JB_PASS).await;

    // Warm the cache (2 tasks, hit=false).
    let (_s, body) = send(&app, get_auth("/tasks/view-my-tasks", &jb_token)).await;
    assert_eq!(body["summary"]["total_assigned_tasks"], 2);
    assert_eq!(body["cache"]["hit"], false);

    // Confirm it's cached.
    let (_s, body) = send(&app, get_auth("/tasks/view-my-tasks", &jb_token)).await;
    assert_eq!(body["cache"]["hit"], true);

    // Assign a 3rd task -> must invalidate James Bond's cache.
    send(
        &app,
        post_auth(
            "/tasks/assign",
            &admin_token,
            json!({ "task_id": ids[2], "assigned_to_id": jb_id }),
        ),
    )
    .await;

    // Next read is a fresh DB load (hit=false) and reflects 3 tasks.
    let (_s, body) = send(&app, get_auth("/tasks/view-my-tasks", &jb_token)).await;
    assert_eq!(body["cache"]["hit"], false);
    assert_eq!(body["summary"]["total_assigned_tasks"], 3);
}

#[tokio::test]
async fn unauthenticated_requests_are_rejected() {
    let app = test_app(300, 60).await;
    seed(&app).await;
    let (status, _) = send(&app, get("/tasks/view-my-tasks")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
