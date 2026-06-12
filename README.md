# Task Management API (Rust)

A backend API for a simple task-management workflow demonstrating **email-based
two-factor authentication**, **JWT auth**, **role-based access control**,
**task assignment**, and **per-user caching**.

Built with **Axum 0.7**, **SQLx (SQLite)**, **Argon2** password hashing,
**JWT (HS256)**, and an **in-memory per-user cache**.

---

## Tech stack

| Concern            | Choice                                            |
| ------------------ | ------------------------------------------------- |
| Language / edition | Rust (stable), edition 2021                       |
| Web framework      | Axum 0.7 + Tokio                                  |
| Database           | SQLite via SQLx 0.7 (file `data.db`)              |
| Migrations         | `sqlx::migrate!` (embedded, run on startup)       |
| Password hashing   | Argon2 (`argon2` crate)                           |
| Auth tokens        | JWT HS256 (`jsonwebtoken`)                        |
| 2FA codes          | 6-digit, SHA-256 hashed at rest, single-use, 5-min TTL |
| Caching            | In-memory per-user TTL cache (documented below)   |
| Serialization      | Serde / serde_json                                |

---

## Project layout

```
src/
  main.rs              # binary entrypoint: load config, init DB, serve
  lib.rs               # crate root: init_state() + build_app() (shared with tests)
  config.rs            # env-driven configuration
  error.rs             # AppError -> HTTP status mapping
  cache.rs             # in-memory per-user TasksCache (TTL + invalidation)
  state.rs             # AppState (db pool, cache, config)
  repo.rs              # all SQL queries (data-access layer)
  auth/
    password.rs        # Argon2 hash/verify
    otp.rs             # 6-digit code generation + SHA-256 hashing
    jwt.rs             # JWT encode/decode
    extractor.rs       # AuthUser extractor (validates Bearer token, RBAC helper)
  models/              # request/response DTOs + domain types + DB row types
    user.rs  task.rs  auth.rs  email_log.rs
  handlers/            # HTTP handlers grouped by area
    seed.rs  auth.rs  dev.rs  tasks.rs
  routes/router.rs     # route table
migrations/
  0001_init.sql        # users, tasks, login_challenges, email_logs
tests/
  workflow.rs          # 9 integration tests covering the full flow
scripts/
  validate.sh          # one-shot end-to-end curl walkthrough
```

---

## Setup & run

### 1. Prerequisites
- Rust toolchain (stable). No external database needed — SQLite file is created automatically.

### 2. Configure environment
```bash
cp .env.example .env
```
`.env` (all values have sensible defaults if unset):
```
DATABASE_URL=sqlite://data.db
JWT_SECRET=super-secret-dev-key-change-me
JWT_EXPIRY_SECONDS=3600
TWO_FACTOR_CODE_TTL_SECONDS=300     # 2FA codes expire after 5 minutes
TASKS_CACHE_TTL_SECONDS=60
PORT=8080
```

### 3. Build & run
```bash
cargo run
# -> listening on http://0.0.0.0:8080
```
Migrations run automatically on startup; no separate migration command is required.
(If you prefer the CLI: `cargo install sqlx-cli && sqlx migrate run`.)

### 4. Run the tests
```bash
cargo test
```
9 integration tests in `tests/workflow.rs`, each against a fresh throwaway SQLite DB.

### 5. One-command end-to-end validation
With the server running in one terminal:
```bash
./scripts/validate.sh
```

---

## Seeded users

`POST /seed/users` is idempotent and creates:

| Role  | Full name  | Email                   | Password         |
| ----- | ---------- | ----------------------- | ---------------- |
| admin | Admin      | admin@example.com       | `Admin@12345`    |
| staff | James Bond | jamesbond@example.com   | `JamesBond@12345`|

---

## API reference

| Method | Path                          | Auth        | Purpose                                            |
| ------ | ----------------------------- | ----------- | -------------------------------------------------- |
| POST   | `/seed/users`                 | none        | Create Admin + James Bond (idempotent)             |
| POST   | `/auth/login`                 | none        | Validate email/password, create 2FA challenge, "email" a code. Returns `login_challenge_id` (no JWT) |
| GET    | `/dev/email-logs/latest`      | none (dev)  | Latest "sent" email (the 2FA code). `?email=` filter |
| POST   | `/auth/verify-2fa`            | none        | Verify code, return JWT access token               |
| POST   | `/tasks`                      | Bearer, **admin** | Create a task                                |
| POST   | `/tasks/assign`               | Bearer, **admin** | Assign a task to a user; invalidates their cache |
| GET    | `/tasks/view-my-tasks`        | Bearer      | Tasks assigned to the caller, with cache metadata  |

### Business rules enforced
- Roles are `admin` or `staff`.
- Only admins can create or assign tasks (`POST /tasks`, `POST /tasks/assign` → `403` for staff).
- Staff only see tasks assigned to them.
- Login never returns a JWT directly; a 2FA code must be verified first.
- 2FA codes: 6 digits, **SHA-256 hashed at rest**, **single-use**, **5-minute expiry**.
  Incorrect → `401`; reused or expired → `400`.

---

## Validation workflow (curl)

```bash
BASE=http://localhost:8080

# 1. Seed users
curl -s -X POST $BASE/seed/users

# 2. Admin login -> returns login_challenge_id (NOT a JWT)
curl -s -X POST $BASE/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"email":"admin@example.com","password":"Admin@12345"}'

# 3. Retrieve the 2FA code from the dev email log
curl -s "$BASE/dev/email-logs/latest?email=admin@example.com"

# 4. Verify 2FA -> Admin JWT
curl -s -X POST $BASE/auth/verify-2fa \
  -H 'Content-Type: application/json' \
  -d '{"login_challenge_id":"<CHALLENGE_ID>","code":"<CODE>"}'

# 5. Create 5 tasks (repeat with priorities high/medium/low/high/medium)
curl -s -X POST $BASE/tasks \
  -H "Authorization: Bearer <ADMIN_JWT>" \
  -H 'Content-Type: application/json' \
  -d '{"title":"Task 1","description":"Description for task 1","priority":"high"}'

# 6. Assign 3 tasks to James Bond (use his id from the seed response)
curl -s -X POST $BASE/tasks/assign \
  -H "Authorization: Bearer <ADMIN_JWT>" \
  -H 'Content-Type: application/json' \
  -d '{"task_id":"<TASK_ID>","assigned_to_id":"<JAMES_BOND_ID>"}'

# 7-8. James Bond login + verify (same two-step flow) -> James Bond JWT

# 9. James Bond tries to create a task -> 403 Forbidden
curl -s -o /dev/null -w '%{http_code}\n' -X POST $BASE/tasks \
  -H "Authorization: Bearer <JB_JWT>" \
  -H 'Content-Type: application/json' \
  -d '{"title":"nope","priority":"high"}'

# 10. View assigned tasks -> exactly 3, cache.hit=false
curl -s "$BASE/tasks/view-my-tasks" -H "Authorization: Bearer <JB_JWT>"

# 11. Same request again -> cache.hit=true
curl -s "$BASE/tasks/view-my-tasks" -H "Authorization: Bearer <JB_JWT>"
```

`./scripts/validate.sh` automates all of the above (extracting ids/codes/tokens for you).

---

## Final validation response

`GET /tasks/view-my-tasks` with the James Bond token. **First call** (served from DB):

```json
{
    "user": {
        "email": "jamesbond@example.com",
        "role": "staff"
    },
    "tasks": [
        {
            "id": "fdb009d5-c80c-4cd1-b852-94f131f6aa2f",
            "title": "Task 1",
            "description": "Description for task 1",
            "status": "todo",
            "priority": "high",
            "assigned_to": "jamesbond@example.com"
        },
        {
            "id": "9d3d0f30-3b3c-4a62-a300-502619372d29",
            "title": "Task 2",
            "description": "Description for task 2",
            "status": "todo",
            "priority": "medium",
            "assigned_to": "jamesbond@example.com"
        },
        {
            "id": "1a12f0d0-57a2-4708-a27d-93a1c44e25ab",
            "title": "Task 3",
            "description": "Description for task 3",
            "status": "todo",
            "priority": "low",
            "assigned_to": "jamesbond@example.com"
        }
    ],
    "summary": {
        "total_assigned_tasks": 3
    },
    "cache": {
        "hit": false
    }
}
```

**Second identical call** is served from cache — identical body except:

```json
    "cache": {
        "hit": true
    }
```

---

## Design notes

### Authentication & 2FA
1. `POST /auth/login` verifies the Argon2 password hash, then creates a
   `login_challenges` row holding only the **SHA-256 hash** of a random 6-digit
   code with a 5-minute expiry. The plaintext code is written to the
   `email_logs` table (our dev "outbox") and returned via
   `GET /dev/email-logs/latest`. The response is a `login_challenge_id` — **never a JWT**.
2. `POST /auth/verify-2fa` looks up the challenge, rejects it if `used`,
   `expired`, or the hash doesn't match, then marks it `used` (single-use) and
   issues a JWT (`sub`, `email`, `role`, `iat`, `exp`).
3. Protected routes use the `AuthUser` extractor, which parses
   `Authorization: Bearer <jwt>`, validates the signature/expiry, and exposes
   the caller's role for RBAC. `AuthUser::require_admin()` returns `403` for staff.

### Caching
- `GET /tasks/view-my-tasks` checks `TasksCache` (a `Mutex<HashMap<Uuid, (json, Instant)>>`)
  keyed by user id. On a miss it loads from the DB, serializes the response,
  stores it, and returns `cache.hit = false`. On a hit within the TTL it returns
  the stored body with `cache.hit = true`.
- `POST /tasks/assign` calls `cache.invalidate(assignee_id)` so a newly assigned
  task is immediately visible (the integration test
  `assigning_a_task_invalidates_the_cache` proves this).

> **Cache limitation (documented):** the cache is **in-memory and
> process-local**, the assignment's accepted alternative to Redis. It is not
> shared across multiple server instances and is cleared on restart. The
> `cache.rs` API surface (`get` / `set` / `invalidate`) is intentionally small
> so it can be swapped for a Redis-backed implementation without touching the
> handlers.

### Data model (`migrations/0001_init.sql`)
- **users**: `id, full_name, email (unique), password_hash, role, created_at, updated_at`
- **tasks**: `id, title, description, status, priority, created_by_id, assigned_to_id, created_at, updated_at`
- **login_challenges**: `id, user_id, code_hash, expires_at, used, created_at`
- **email_logs**: `id, user_id, email, code, purpose, created_at`

Ids are UUIDs (stored as TEXT); timestamps are RFC3339 TEXT. `status` and
`priority` are constrained via `CHECK` constraints.

---

## Testing

`cargo test` runs `tests/workflow.rs`:

- `seed_creates_admin_and_staff`
- `login_returns_challenge_not_jwt_then_verifies`
- `wrong_password_is_unauthorized`
- `incorrect_2fa_code_is_rejected`
- `reused_2fa_code_is_rejected`
- `expired_2fa_code_is_rejected`
- `full_workflow_with_roles_and_caching` — 5 tasks, assign 3, staff 403, exactly
  3 assigned, `cache.hit` false → true
- `assigning_a_task_invalidates_the_cache`
- `unauthenticated_requests_are_rejected`

All pass against fresh per-test SQLite databases.
