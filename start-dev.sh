#!/usr/bin/env zsh
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT_DIR"

MODE="dev"
INSTALL_DEPS=1

for arg in "$@"; do
  case "$arg" in
    --check)
      MODE="check"
      ;;
    --build)
      MODE="build"
      ;;
    --no-install)
      INSTALL_DEPS=0
      ;;
    -h|--help)
      cat <<'EOF'
VibeHub macOS development helper

Usage:
  ./start-dev.sh              Check macOS env, install deps if needed, start Tauri dev
  ./start-dev.sh --check      Check macOS env only
  ./start-dev.sh --build      Check env, install deps if needed, run full Tauri build
  ./start-dev.sh --no-install Skip npm ci when node_modules is missing
EOF
      exit 0
      ;;
    *)
      echo "Unknown argument: $arg" >&2
      exit 2
      ;;
  esac
done

step() {
  printf '\n[%s] %s\n' "$1" "$2"
}

require_cmd() {
  local name="$1"
  local hint="$2"
  if ! command -v "$name" >/dev/null 2>&1; then
    echo "Missing command: $name" >&2
    echo "$hint" >&2
    exit 1
  fi
}

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "This helper is for macOS. Use npm run tauri dev on other platforms." >&2
  exit 1
fi

if [[ -x /opt/homebrew/opt/node@20/bin/node ]]; then
  export PATH="/opt/homebrew/opt/node@20/bin:/opt/homebrew/bin:$PATH"
elif [[ -x /opt/homebrew/bin/brew ]]; then
  export PATH="/opt/homebrew/bin:$PATH"
fi

step "1/5" "Checking Homebrew and Xcode Command Line Tools"
require_cmd brew "Install Homebrew from https://brew.sh/"
if ! xcode-select -p >/dev/null 2>&1; then
  echo "Missing Xcode Command Line Tools. Run: xcode-select --install" >&2
  exit 1
fi
echo "Homebrew: $(brew --version | head -n 1)"
echo "Xcode CLT: $(xcode-select -p)"

step "2/5" "Checking Node.js and npm"
require_cmd node "Install with: brew install node@20"
require_cmd npm "Install with: brew install node@20"
NODE_MAJOR="$(node -e 'process.stdout.write(String(process.versions.node.split(".")[0]))')"
if (( NODE_MAJOR < 20 )); then
  echo "Node.js 20+ is required. Current: $(node --version)" >&2
  exit 1
fi
echo "node: $(command -v node) ($(node --version))"
echo "npm:  $(command -v npm) ($(npm --version))"

step "3/5" "Checking Rust toolchain"
require_cmd cargo "Install with: brew install rust"
require_cmd rustc "Install with: brew install rust"
echo "cargo: $(command -v cargo) ($(cargo --version))"
echo "rustc: $(command -v rustc) ($(rustc --version))"

step "4/5" "Checking frontend dependencies"
if [[ ! -d node_modules ]]; then
  if (( INSTALL_DEPS == 0 )); then
    echo "node_modules is missing and --no-install was passed." >&2
    exit 1
  fi
  npm ci
else
  echo "node_modules present"
fi

step "5/5" "Running requested mode"
case "$MODE" in
  check)
    npm run build
    (cd src-tauri && cargo fmt --all -- --check)
    cargo check --manifest-path src-tauri/Cargo.toml --locked
    ;;
  build)
    npm run tauri build
    ;;
  dev)
    npm run tauri dev
    ;;
esac
