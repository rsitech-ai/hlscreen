#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
mkdir -p "$repo_root/target"

HLS_REPO_ROOT="$repo_root" rustc \
  --edition=2024 \
  --test "$repo_root/tests/integration/release_packaging.rs" \
  -o "$repo_root/target/release_packaging_test"

HLS_REPO_ROOT="$repo_root" "$repo_root/target/release_packaging_test"
