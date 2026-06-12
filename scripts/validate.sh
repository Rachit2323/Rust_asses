#!/usr/bin/env bash
#
# End-to-end validation of the Task Management API.
# Requires the server to be running (default http://localhost:8080) and
# `python3` available for JSON parsing.
#
# Usage: ./scripts/validate.sh
set -euo pipefail

BASE="${BASE:-http://localhost:8080}"

jqp() { python3 -c "import sys,json;print(json.load(sys.stdin)$1)"; }

hr() { printf '\n========== %s ==========\n' "$1"; }

hr "1. Seed Admin + James Bond"
curl -s -X POST "$BASE/seed/users" | python3 -m json.tool

login_and_verify() {
  # $1 = email, $2 = password -> echoes the JWT access token
  local email="$1" password="$2" challenge code token
  challenge=$(curl -s -X POST "$BASE/auth/login" \
    -H 'Content-Type: application/json' \
    -d "{\"email\":\"$email\",\"password\":\"$password\"}" | jqp '["login_challenge_id"]')
  code=$(curl -s "$BASE/dev/email-logs/latest?email=$email" | jqp '["code"]')
  token=$(curl -s -X POST "$BASE/auth/verify-2fa" \
    -H 'Content-Type: application/json' \
    -d "{\"login_challenge_id\":\"$challenge\",\"code\":\"$code\"}" | jqp '["access_token"]')
  echo "$token"
}

hr "2-4. Admin login + 2FA -> JWT"
ADMIN_TOKEN=$(login_and_verify "admin@example.com" "Admin@12345")
echo "Admin JWT acquired (len ${#ADMIN_TOKEN})"

hr "5. Create exactly 5 tasks as Admin"
TASK_IDS=()
PRIORITIES=(high medium low high medium)
for i in 1 2 3 4 5; do
  prio="${PRIORITIES[$((i-1))]}"
  tid=$(curl -s -X POST "$BASE/tasks" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H 'Content-Type: application/json' \
    -d "{\"title\":\"Task $i\",\"description\":\"Description for task $i\",\"priority\":\"$prio\"}" \
    | jqp '["id"]')
  TASK_IDS+=("$tid")
  echo "  created Task $i ($prio) -> $tid"
done

hr "6. Get James Bond's user id"
JB_ID=$(curl -s -X POST "$BASE/seed/users" | jqp '["users"][1]["id"]')
echo "  James Bond id: $JB_ID"

hr "7. Assign exactly 3 tasks (high, medium, low) to James Bond"
# Assign tasks 1 (high), 2 (medium), 3 (low)
for idx in 0 1 2; do
  curl -s -X POST "$BASE/tasks/assign" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H 'Content-Type: application/json' \
    -d "{\"task_id\":\"${TASK_IDS[$idx]}\",\"assigned_to_id\":\"$JB_ID\"}" >/dev/null
  echo "  assigned ${TASK_IDS[$idx]} -> James Bond"
done

hr "8-9. James Bond login + 2FA -> JWT"
JB_TOKEN=$(login_and_verify "jamesbond@example.com" "JamesBond@12345")
echo "James Bond JWT acquired (len ${#JB_TOKEN})"

hr "10. James Bond attempts to create a task (expect 403 Forbidden)"
code=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE/tasks" \
  -H "Authorization: Bearer $JB_TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"title":"hack","priority":"high"}')
echo "  HTTP status: $code  (expected 403)"

hr "11. James Bond GET /tasks/view-my-tasks (cache.hit = false)"
curl -s "$BASE/tasks/view-my-tasks" -H "Authorization: Bearer $JB_TOKEN" | python3 -m json.tool

hr "12. James Bond GET /tasks/view-my-tasks AGAIN (cache.hit = true)"
curl -s "$BASE/tasks/view-my-tasks" -H "Authorization: Bearer $JB_TOKEN" | python3 -m json.tool

printf '\nValidation complete.\n'
