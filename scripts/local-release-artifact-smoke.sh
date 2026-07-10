#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
work_dir="${HLS_RELEASE_SMOKE_DIR:-$repo_root/target/local-release-smoke}"
bin="$repo_root/target/release/hls"

if [[ ! -x "$bin" ]]; then
  cargo build --release -p hls-cli --bin hls
fi

host_triple="$(rustc -vV | awk '/^host:/ {print $2}')"
package_name="hlscreen-${host_triple}"
stage_dir="$work_dir/$package_name"
archive="$work_dir/${package_name}.tar.gz"
checksum_file="$archive.sha256"
unpack_dir="$work_dir/unpack"
smoke_data_dir="$work_dir/smoke-data"

rm -rf "$work_dir"
mkdir -p "$stage_dir/bin" "$unpack_dir" "$smoke_data_dir"

cp "$bin" "$stage_dir/bin/hls"
cp "$repo_root/LICENSE" "$stage_dir/LICENSE"
cp "$repo_root/README.md" "$stage_dir/README.md"
cp "$repo_root/CHANGELOG.md" "$stage_dir/CHANGELOG.md"

(
  cd "$work_dir"
  tar -czf "$archive" "$package_name"
)

if command -v shasum >/dev/null 2>&1; then
  (
    cd "$work_dir"
    shasum -a 256 "$(basename "$archive")" > "$(basename "$checksum_file")"
    shasum -a 256 -c "$(basename "$checksum_file")"
  )
elif command -v sha256sum >/dev/null 2>&1; then
  (
    cd "$work_dir"
    sha256sum "$(basename "$archive")" > "$(basename "$checksum_file")"
    sha256sum -c "$(basename "$checksum_file")"
  )
else
  echo "sha256 checksum tool not found: install shasum or sha256sum" >&2
  exit 1
fi

tar -xzf "$archive" -C "$unpack_dir"
smoke_bin="$unpack_dir/$package_name/bin/hls"

"$smoke_bin" --help >/dev/null
"$smoke_bin" doctor --data-dir "$smoke_data_dir/doctor" >/dev/null
"$smoke_bin" live \
  --symbols @107 \
  --fixture-file "$repo_root/tests/fixtures/hyperliquid/ws_mock_live.ndjson" \
  --once \
  --data-dir "$smoke_data_dir/live" \
  >/dev/null

echo "release_artifact=$archive"
echo "release_checksum=$checksum_file"
echo "release_smoke=passed"
