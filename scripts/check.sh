#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

usage() {
  echo "Usage: scripts/check.sh [fast|pr|release]" >&2
}

if (( $# > 1 )); then
  usage
  exit 2
fi

mode="${1:-pr}"
case "$mode" in
  fast|pr|release) ;;
  *)
    usage
    exit 2
    ;;
esac

cd "$repo_root"

run_fast_checks() {
  cargo fmt --all -- --check
  cargo check --workspace --all-features --locked
  cargo test --workspace --all-features --locked
  git diff --check
}

run_pr_checks() {
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
  cargo test --workspace --all-features --locked
  cargo build --release --workspace --all-features --locked
  RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --locked
  python3 scripts/generate-screenshots.py --check
  scripts/check-release-packaging.sh
  git diff --check
}

require_exact_version() {
  local label="$1"
  local expected="$2"
  shift 2

  local actual
  if ! actual="$("$@" 2>/dev/null)"; then
    echo "$label is required at exactly $expected." >&2
    exit 1
  fi
  if [[ "$actual" != "$expected" ]]; then
    echo "$label must be $expected; found $actual." >&2
    exit 1
  fi
}

run_release_checks() {
  run_pr_checks

  require_exact_version "cargo-audit" "cargo-audit 0.22.2" cargo-audit --version
  cargo audit --deny warnings --ignore RUSTSEC-2024-0436

  require_exact_version "cargo-deny" "cargo-deny 0.20.2" cargo deny --version
  cargo deny check licenses sources

  scripts/check-third-party-licenses.sh

  uvx "zizmor@1.26.1" \
    --pedantic \
    --strict-collection \
    --color never \
    .github
}

case "$mode" in
  fast) run_fast_checks ;;
  pr) run_pr_checks ;;
  release) run_release_checks ;;
esac
