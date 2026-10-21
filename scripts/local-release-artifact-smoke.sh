#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
work_dir="${HLS_RELEASE_SMOKE_DIR:-$repo_root/target/local-release-smoke}"
build_dir="${CARGO_TARGET_DIR:-$repo_root/target}"
case "$build_dir" in
  /*) ;;
  *) build_dir="$repo_root/$build_dir" ;;
esac
host_triple="$(rustc -vV | awk '/^host:/ {print $2}')"
bin="$build_dir/$host_triple/release/hls"

# Check Cargo's current source fingerprints even when an older binary exists.
cargo build --locked --release --target "$host_triple" --target-dir "$build_dir" -p hls-cli --bin hls

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
cp "$repo_root/NOTICE" "$stage_dir/NOTICE"
cp "$repo_root/THIRD_PARTY_LICENSES.txt" "$stage_dir/THIRD_PARTY_LICENSES.txt"
cp "$repo_root/THIRD_PARTY_NOTICES.md" "$stage_dir/THIRD_PARTY_NOTICES.md"
cp "$repo_root/README.md" "$stage_dir/README.md"
cp "$repo_root/CHANGELOG.md" "$stage_dir/CHANGELOG.md"

(
  cd "$work_dir"
  COPYFILE_DISABLE=1 tar -czf "$archive" "$package_name"
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
test -s "$unpack_dir/$package_name/LICENSE"
test -s "$unpack_dir/$package_name/NOTICE"
test -s "$unpack_dir/$package_name/THIRD_PARTY_LICENSES.txt"
test -s "$unpack_dir/$package_name/THIRD_PARTY_NOTICES.md"
smoke_bin="$unpack_dir/$package_name/bin/hls"

"$smoke_bin" --help >/dev/null
"$smoke_bin" doctor --data-dir "$smoke_data_dir/doctor" >/dev/null
"$smoke_bin" live \
  --symbols @107 \
  --fixture-file "$repo_root/tests/fixtures/hyperliquid/ws_mock_live.ndjson" \
  --once \
  --data-dir "$smoke_data_dir/live" \
  >/dev/null

python3 "$repo_root/scripts/verify-mfop5-smoke.py" \
  --binary "$smoke_bin" \
  --data-dir "$smoke_data_dir/mfop5"

echo "release_artifact=$archive"
echo "release_checksum=$checksum_file"
echo "release_smoke=passed"
