#!/usr/bin/env python3
"""Require all direct shared-feed packages and every transitive marketfeed crate
in Cargo.lock to resolve to one complete immutable source revision."""
import re
from pathlib import Path
import json
import subprocess
root = Path(__file__).resolve().parent.parent
metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--format-version", "1", "--locked"], cwd=root, text=True))
repository = "https://github.com/rsitech-ai/cryptofeed-rs"
packages = ["marketfeed-daemon", "marketfeed-adapter-api", "marketfeed-adapter-hyperliquid", "marketfeed-engine", "marketfeed-model", "marketfeed-recording"]
revisions = set()
for package in metadata["packages"]:
    if package["id"] not in metadata["workspace_members"]:
        continue
    for dependency in package["dependencies"]:
        if dependency["name"] not in packages:
            continue
        source = dependency.get("source") or ""
        match = re.fullmatch(re.escape("git+" + repository) + r"\?rev=([0-9a-f]{40})", source)
        if match is None:
            raise SystemExit(f"{dependency['name']} requires the shared repository and full immutable rev")
        revisions.add(match.group(1))
if len(revisions) != 1:
    raise SystemExit("shared decoder and recording revisions differ or are missing")
revision = revisions.pop()
expected = f"git+{repository}?rev={revision}#{revision}"
resolved = {p["name"] for p in metadata["packages"] if p["name"].startswith("marketfeed-")}
if not set(packages).issubset(resolved):
    raise SystemExit("shared packages missing from resolution")
for package in metadata["packages"]:
    if package["name"].startswith("marketfeed-") and package.get("source") != expected:
        raise SystemExit(f"{package['name']} resolves to a different shared revision")
print(f"shared_feed_revision={revision}")
