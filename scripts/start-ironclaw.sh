#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
PID_FILE="${IRONCLAW_PID_FILE:-/tmp/ironclaw.pid}"
LOG_FILE="${IRONCLAW_LOG_FILE:-/tmp/ironclaw.log}"
PORT="${IRONCLAW_PORT:-3000}"
START_TIMEOUT="${IRONCLAW_START_TIMEOUT:-90}"
FOREGROUND=0

usage() {
  cat <<EOF
Usage:
  ./scripts/start-ironclaw.sh [--foreground]

Behavior:
  - Uses the project .env from: ${ROOT_DIR}/.env
  - Prefers target/release/ironclaw if it exists
  - Falls back to cargo run when the release binary is missing
  - Clears conflicting shell env vars so the project .env wins

Environment overrides:
  IRONCLAW_PID_FILE       PID file path (default: ${PID_FILE})
  IRONCLAW_LOG_FILE       Log file path (default: ${LOG_FILE})
  IRONCLAW_PORT           Expected HTTP port (default: ${PORT})
  IRONCLAW_START_TIMEOUT  Seconds to wait for the port to open (default: ${START_TIMEOUT})

Examples:
  ./scripts/start-ironclaw.sh
  ./scripts/start-ironclaw.sh --foreground
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --foreground)
      FOREGROUND=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

cd "$ROOT_DIR"

if [ ! -f "${ROOT_DIR}/.env" ]; then
  echo "Missing project .env at ${ROOT_DIR}/.env" >&2
  exit 1
fi

if command -v lsof >/dev/null 2>&1; then
  existing_listener="$(lsof -nP -iTCP:${PORT} -sTCP:LISTEN 2>/dev/null || true)"
  if [ -n "$existing_listener" ]; then
    echo "Port ${PORT} is already in use:" >&2
    echo "$existing_listener" >&2
    echo "Stop the existing process first, then rerun this script." >&2
    exit 1
  fi
fi

env_clear=(
  env
  -u LLM_BACKEND
  -u LLM_BASE_URL
  -u LLM_API_KEY
  -u LLM_MODEL
  -u ANTHROPIC_BASE_URL
  -u ANTHROPIC_API_KEY
  -u ANTHROPIC_MODEL
  -u DATABASE_BACKEND
  -u DATABASE_URL
  -u DATABASE_POOL_SIZE
  -u DATABASE_SSLMODE
  -u CLI_ENABLED
  -u GATEWAY_AUTH_TOKEN
  -u TELEGRAM_BOT_TOKEN
)

if [ -x "${ROOT_DIR}/target/release/ironclaw" ]; then
  launch_cmd=("${env_clear[@]}" "${ROOT_DIR}/target/release/ironclaw" --no-onboard run)
  launch_mode="release binary"
else
  launch_cmd=("${env_clear[@]}" cargo run --bin ironclaw -- --no-onboard run)
  launch_mode="cargo run"
fi

if [ "$FOREGROUND" -eq 1 ]; then
  echo "Starting IronClaw in foreground using ${launch_mode}"
  exec "${launch_cmd[@]}"
fi

echo "Starting IronClaw in background using ${launch_mode}"
echo "Project .env: ${ROOT_DIR}/.env"
echo "Log file: ${LOG_FILE}"

: > "$LOG_FILE"
"${launch_cmd[@]}" >"$LOG_FILE" 2>&1 &
pid=$!
echo "$pid" > "$PID_FILE"

started=0
if command -v lsof >/dev/null 2>&1; then
  for _ in $(seq 1 "$START_TIMEOUT"); do
    if ! kill -0 "$pid" 2>/dev/null; then
      break
    fi
    if lsof -nP -iTCP:${PORT} -sTCP:LISTEN 2>/dev/null | grep -q LISTEN; then
      started=1
      break
    fi
    sleep 1
  done
else
  sleep 2
  if kill -0 "$pid" 2>/dev/null; then
    started=1
  fi
fi

if [ "$started" -eq 1 ]; then
  echo "IronClaw started."
  echo "PID: ${pid}"
  echo "URL: http://127.0.0.1:${PORT}/"
  echo "Log: ${LOG_FILE}"
  echo "PID file: ${PID_FILE}"
  echo "Stop: ./scripts/stop-ironclaw.sh"
  exit 0
fi

echo "IronClaw failed to start. Recent log:" >&2
tail -n 40 "$LOG_FILE" 2>/dev/null || true
exit 1
