#!/usr/bin/env zsh
set -euo pipefail

# Capture sanitized 1600×1000 release screenshots from the current Tauri dev app.
# Requires macOS Accessibility and Screen Recording for the capturing process.
# If those permissions are denied, use the DEV fixture instead:
#   npm run dev
#   open http://localhost:1420/?fixture=release#agent-profiles
#   sips -z 1000 1600 <png>

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT_DIR"

TAG="${1:-${RELEASE_TAG:-}}"
if [[ -z "$TAG" ]]; then
  TAG="v$(node -p 'require("./package.json").version')"
fi
VERSION="${TAG#v}"
OUT_DIR="$ROOT_DIR/assets/releases/v$VERSION"
mkdir -p "$OUT_DIR"

REAL_HOME="${HOME}"
DEMO_HOME="/Users/Shared/Demo/Home"
MOCK_PORT=18765
WINDOW_WIDTH=1600
WINDOW_HEIGHT=1000

step() {
  printf '\n[%s] %s\n' "$1" "$2"
}

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing command: $1" >&2
    exit 1
  fi
}

require_cmd osascript
require_cmd screencapture
require_cmd sips
require_cmd python3
require_cmd npm

cleanup() {
  if [[ -n "${MOCK_PID:-}" ]]; then
    kill "$MOCK_PID" 2>/dev/null || true
  fi
  if [[ -n "${TAURI_PID:-}" ]]; then
    kill "$TAURI_PID" 2>/dev/null || true
    wait "$TAURI_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

step "1/6" "Seeding demo HOME at $DEMO_HOME"
mkdir -p \
  "$DEMO_HOME/.config/opencode" \
  "$DEMO_HOME/.claude/vibehub-profiles" \
  "$DEMO_HOME/Library/Application Support/VibeHub" \
  "/Users/Shared/Demo/Workspace"

python3 - "$DEMO_HOME" <<'PY'
import json, sys, datetime
home = sys.argv[1]
opencode = {
  "$schema": "https://opencode.ai/config.json",
  "model": "openai/gpt-5.4",
  "small_model": "openai/gpt-5.4-mini",
  "provider": {
    "openai": {
      "name": "OpenAI",
      "options": {
        "baseURL": "http://127.0.0.1:18765/v1",
        "apiKey": "sk-demo-not-a-real-key"
      },
      "models": {
        "gpt-5.4": {
          "name": "GPT-5.4",
          "reasoning": True,
          "variants": {"low": {}, "medium": {}, "high": {}, "xhigh": {}}
        },
        "gpt-5.4-mini": {
          "name": "GPT-5.4 mini",
          "reasoning": True,
          "variants": {"low": {}, "medium": {}, "high": {}}
        }
      }
    }
  }
}
with open(f"{home}/.config/opencode/opencode.json", "w", encoding="utf-8") as fh:
    json.dump(opencode, fh, indent=2)
    fh.write("\n")

settings = {
    "model": "claude-sonnet-4",
    "env": {
        "ANTHROPIC_AUTH_TOKEN": "sk-demo-not-a-real-key",
        "ANTHROPIC_BASE_URL": "https://api.anthropic.com"
    }
}
with open(f"{home}/.claude/vibehub-profiles/VibeHub Daily.settings.json", "w", encoding="utf-8") as fh:
    json.dump(settings, fh, indent=2)
    fh.write("\n")

config = {
  "workspaces": [{
    "id": "ws-demo",
    "name": "产品工作区",
    "path": "/Users/Shared/Demo/Workspace",
    "auto_scan": False,
    "created_at": "2026-05-01T00:00:00Z"
  }],
  "tags": [{
    "id": "tag-claude-daily",
    "name": "Claude Daily",
    "color": "#D97757",
    "category": "cli",
    "config": {
      "executable": "claude",
      "args": ["--settings", "/Users/Shared/Demo/Home/.claude/vibehub-profiles/VibeHub Daily.settings.json"],
      "terminal": "Terminal"
    }
  }],
  "projects": [],
  "theme": "light",
  "recent_projects": []
}
with open(f"{home}/Library/Application Support/VibeHub/config.json", "w", encoding="utf-8") as fh:
    json.dump(config, fh, indent=2)
    fh.write("\n")
PY

step "2/6" "Starting mock /v1/models server on :$MOCK_PORT"
if lsof -nP -t -iTCP:"$MOCK_PORT" -sTCP:LISTEN >/dev/null 2>&1; then
  kill $(lsof -nP -t -iTCP:"$MOCK_PORT" -sTCP:LISTEN) 2>/dev/null || true
  sleep 0.3
fi
python3 - "$MOCK_PORT" <<'PY' &
import json, sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

port = int(sys.argv[1])
payload = json.dumps({
    "object": "list",
    "data": [
        {"id": "gpt-5.4", "object": "model", "owned_by": "openai"},
        {"id": "gpt-5.4-mini", "object": "model", "owned_by": "openai"},
        {"id": "gpt-4.1", "object": "model", "owned_by": "openai"},
        {"id": "gpt-4o", "object": "model", "owned_by": "openai"},
        {"id": "o4-mini", "object": "model", "owned_by": "openai"},
    ],
}).encode()

class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path.rstrip("/").endswith("/models"):
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)
            return
        self.send_error(404)

    def log_message(self, *_args):
        return

class ReuseServer(ThreadingHTTPServer):
    allow_reuse_address = True

ReuseServer(("127.0.0.1", port), Handler).serve_forever()
PY
MOCK_PID=$!
sleep 0.3

step "3/6" "Starting Tauri dev with demo HOME"
export CARGO_HOME="${CARGO_HOME:-$REAL_HOME/.cargo}"
export RUSTUP_HOME="${RUSTUP_HOME:-$REAL_HOME/.rustup}"
export npm_config_cache="${npm_config_cache:-$REAL_HOME/.npm}"
export CARGO_TERM_COLOR=never
export HOME="$DEMO_HOME"
export LANG="zh_CN.UTF-8"
export LC_ALL="zh_CN.UTF-8"
export VITE_START_PAGE="agent-profiles"

: >/tmp/vibehub-release-capture.log
npm run tauri dev >/tmp/vibehub-release-capture.log 2>&1 &
TAURI_PID=$!

debug_pid() {
  ps -axo pid=,command= | awk '$2 ~ /\/debug\/vibehub$/ { print $1 }' | tail -n 1
}

wait_for_window() {
  local tries=0
  while (( tries < 600 )); do
    if [[ -n "$(debug_pid)" ]]; then
      return 0
    fi
    if ! kill -0 "$TAURI_PID" 2>/dev/null; then
      echo "Tauri dev exited early. Last log:" >&2
      tail -n 80 /tmp/vibehub-release-capture.log >&2 || true
      return 1
    fi
    sleep 1
    tries=$((tries + 1))
  done
  echo "VibeHub window did not appear. Last log:" >&2
  tail -n 80 /tmp/vibehub-release-capture.log >&2 || true
  return 1
}

step "4/6" "Waiting for VibeHub window"
wait_for_window
sleep 4

resize_window() {
  local pid
  pid="$(debug_pid)"
  osascript <<APPLESCRIPT
tell application "System Events"
  tell (first process whose unix id is $pid)
    set frontmost to true
    delay 0.4
    try
      set position of window 1 to {80, 60}
      set size of window 1 to {$WINDOW_WIDTH, $WINDOW_HEIGHT}
    end try
  end tell
end tell
APPLESCRIPT
}

click_named() {
  local name="$1"
  local pid
  pid="$(debug_pid)"
  osascript <<APPLESCRIPT
tell application "System Events"
  tell (first process whose unix id is $pid)
    set frontmost to true
    delay 0.2
    set targets to entire contents of window 1
    repeat with el in targets
      try
        if (name of el is "$name") or (value of el is "$name") or (description of el is "$name") then
          click el
          return
        end if
      end try
    end repeat
    error "Did not find UI element named $name"
  end tell
end tell
APPLESCRIPT
}

press_escape() {
  osascript -e 'tell application "System Events" to key code 53'
}

capture_window() {
  local dest="$1"
  local pid
  pid="$(debug_pid)"
  resize_window
  sleep 0.4
  python3 - "$dest" "$WINDOW_WIDTH" "$WINDOW_HEIGHT" "$pid" <<'PY'
import subprocess, sys, tempfile, os
dest, width, height, pid = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
bounds = subprocess.check_output([
    "osascript", "-e",
    f'tell application "System Events" to tell (first process whose unix id is {pid}) to get {{position, size}} of window 1'
], text=True).strip()
nums = [int(value) for value in bounds.replace("{", " ").replace("}", " ").replace(",", " ").split() if value.lstrip("-").isdigit()]
x, y, w, h = nums[:4]
raw = tempfile.mktemp(suffix=".png")
subprocess.check_call(["screencapture", "-x", "-R", f"{x},{y},{w},{h}", raw])
subprocess.check_call(["sips", "-z", height, width, raw, "--out", dest], stdout=subprocess.DEVNULL)
os.remove(raw)
print(dest)
PY
}

step "5/6" "Capturing Agent 配置 and dialogs"
sleep 2
capture_window "$OUT_DIR/agent-profiles.png"

click_named "编辑 Provider" || click_named "Edit Provider" || echo "Could not open Provider editor" >&2
sleep 1
capture_window "$OUT_DIR/agent-provider-editor.png"
press_escape || true
sleep 0.6

click_named "检测模型" || click_named "Detect models" || echo "Could not open model import" >&2
sleep 2
click_named "全选未导入" || click_named "Select unimported" || true
sleep 0.4
capture_window "$OUT_DIR/agent-import-models.png"
press_escape || true
sleep 0.6

click_named "Claude Daily" || echo "Could not open tag editor" >&2
sleep 1
capture_window "$OUT_DIR/tag-launch.png"
press_escape || true
sleep 0.4

step "6/6" "Copying Agent 配置 shot into README visual tour"
cp "$OUT_DIR/agent-profiles.png" "$ROOT_DIR/assets/readme/agent-profiles.png"

python3 - "$OUT_DIR" <<'PY'
from pathlib import Path
import struct, sys
out = Path(sys.argv[1])
expected = {
    "agent-profiles.png",
    "agent-provider-editor.png",
    "agent-import-models.png",
    "tag-launch.png",
}
missing = [name for name in expected if not (out / name).is_file()]
if missing:
    raise SystemExit(f"missing screenshots: {missing}")
for path in out.glob("*.png"):
    data = path.read_bytes()
    width, height = struct.unpack(">II", data[16:24])
    print(f"{path.name}: {width}x{height}")
    if (width, height) != (1600, 1000):
        raise SystemExit(f"{path} must be 1600x1000, got {width}x{height}")
PY

echo
echo "Wrote screenshots to $OUT_DIR"
