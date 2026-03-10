#!/usr/bin/env bash

set -euo pipefail

PID_FILE="${IRONCLAW_PID_FILE:-/tmp/ironclaw.pid}"
PORT="${IRONCLAW_PORT:-3000}"
STOP_TIMEOUT="${IRONCLAW_STOP_TIMEOUT:-15}"
FORCE=0

usage() {
  cat <<EOF
Usage:
  ./scripts/stop-ironclaw.sh [--force]

Behavior:
  - Stops the IronClaw process recorded in ${PID_FILE}
  - Falls back to the listener on port ${PORT} if the PID file is stale
  - Removes the PID file when the process is no longer running

Environment overrides:
  IRONCLAW_PID_FILE      PID file path (default: ${PID_FILE})
  IRONCLAW_PORT          Expected HTTP port (default: ${PORT})
  IRONCLAW_STOP_TIMEOUT  Seconds to wait after SIGTERM (default: ${STOP_TIMEOUT})

Examples:
  ./scripts/stop-ironclaw.sh
  ./scripts/stop-ironclaw.sh --force
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --force)
      FORCE=1
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

find_listener_pid() {
  if command -v lsof >/dev/null 2>&1; then
    lsof -t -nP -iTCP:${PORT} -sTCP:LISTEN 2>/dev/null | head -n 1 || true
  fi
}

pid=""
if [ -f "$PID_FILE" ]; then
  pid="$(tr -d '[:space:]' < "$PID_FILE")"
fi

if [ -n "$pid" ] && ! kill -0 "$pid" 2>/dev/null; then
  rm -f "$PID_FILE"
  pid=""
fi

if [ -z "$pid" ]; then
  pid="$(find_listener_pid)"
fi

if [ -z "$pid" ]; then
  echo "No IronClaw process found."
  rm -f "$PID_FILE"
  exit 0
fi

echo "Stopping IronClaw (PID ${pid})..."
kill "$pid" 2>/dev/null || true

stopped=0
for _ in $(seq 1 "$STOP_TIMEOUT"); do
  if ! kill -0 "$pid" 2>/dev/null; then
    stopped=1
    break
  fi
  sleep 1
done

if [ "$stopped" -ne 1 ] && [ "$FORCE" -eq 1 ]; then
  echo "Process still running, sending SIGKILL..."
  kill -9 "$pid" 2>/dev/null || true
  sleep 1
  if ! kill -0 "$pid" 2>/dev/null; then
    stopped=1
  fi
fi

if [ "$stopped" -eq 1 ] || ! kill -0 "$pid" 2>/dev/null; then
  rm -f "$PID_FILE"
  echo "IronClaw stopped."
  exit 0
fi

echo "IronClaw is still running (PID ${pid}). Use --force to send SIGKILL." >&2
exit 1
