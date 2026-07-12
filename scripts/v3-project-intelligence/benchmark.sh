#!/usr/bin/env bash
set -euo pipefail

project="${1:-.}"
runs="${2:-5}"

for ((run = 1; run <= runs; run++)); do
  cargo run --quiet --release -p vibehub-core --example project_intelligence_bench -- "$project"
done
