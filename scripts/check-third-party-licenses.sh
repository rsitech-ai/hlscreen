#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
generated="$(mktemp "${TMPDIR:-/tmp}/hlscreen-third-party-licenses.XXXXXX")"
trap 'rm -f "$generated"' EXIT

cd "$repo_root"
if [[ "$(cargo about --version)" != "cargo-about 0.9.1" ]]; then
  echo "cargo-about 0.9.1 is required." >&2
  echo "Install it with:" >&2
  echo "  cargo install cargo-about --version 0.9.1 --locked --features cli" >&2
  exit 1
fi

cargo fetch --locked
cargo about generate \
  --workspace \
  --all-features \
  --locked \
  --offline \
  --fail \
  --config about.toml \
  --output-file "$generated" \
  about.hbs

if ! cmp -s THIRD_PARTY_LICENSES.txt "$generated"; then
  echo "THIRD_PARTY_LICENSES.txt is stale." >&2
  echo "Regenerate it with cargo-about 0.9.1:" >&2
  echo "  cargo about generate --workspace --all-features --locked --offline --fail --config about.toml --output-file THIRD_PARTY_LICENSES.txt about.hbs" >&2
  diff -u THIRD_PARTY_LICENSES.txt "$generated" || true
  exit 1
fi

python3 scripts/check-third-party-notices.py

echo "third_party_licenses=verified"
