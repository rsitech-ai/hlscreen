#!/usr/bin/env python3
"""Apply and verify reviewed cargo-dist 0.32.0 workflow hardening deltas."""

from __future__ import annotations

import argparse
import subprocess
from pathlib import Path


GENERATED_PERMISSIONS = '''name: Release
permissions:
  "contents": "write"
'''
HARDENED_PERMISSIONS = '''# cargo-dist 0.32.0 requires reviewed post-generation security fixes.
# scripts/harden-generated-release-workflow.py owns and verifies those deltas.
name: Release
permissions:
  "contents": "read"

concurrency:
  group: release-${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}
'''


def replace_once(contents: str, old: str, new: str, label: str) -> str:
    count = contents.count(old)
    if count == 1:
        return contents.replace(old, new, 1)
    if count == 0 and new in contents:
        return contents
    raise ValueError(f"release workflow has an unknown {label} shape")


def remove_once(contents: str, old: str, label: str) -> str:
    count = contents.count(old)
    if count == 1:
        return contents.replace(old, "", 1)
    if count == 0:
        return contents
    raise ValueError(f"release workflow has duplicate {label} blocks")


def harden(contents: str) -> str:
    contents = replace_once(
        contents,
        GENERATED_PERMISSIONS,
        HARDENED_PERMISSIONS,
        "top-level permissions",
    )
    contents = replace_once(
        contents,
        "  plan:\n    runs-on:",
        "  plan:\n    name: Plan release\n    runs-on:",
        "plan job name",
    )
    contents = replace_once(
        contents,
        '    runs-on: "ubuntu-22.04"\n    outputs:',
        '    runs-on: "ubuntu-22.04"\n    timeout-minutes: 20\n    outputs:',
        "plan timeout",
    )
    contents = replace_once(
        contents,
        '''      - name: Install dist
        # we specify bash to get pipefail; it guards against the `curl` command
        # failing. otherwise `sh` won't catch that `curl` returned non-0
        shell: bash
        run: "curl --proto '=https' --tlsv1.2 -LsSf https://github.com/axodotdev/cargo-dist/releases/download/v0.32.0/cargo-dist-installer.sh | sh"
''',
        '''      - name: Install dist
        shell: bash
        # cargo-dist 0.32.0 requires reviewed post-generation security fixes.
        run: cargo install cargo-dist --version 0.32.0 --locked
''',
        "plan dist installer",
    )
    contents = replace_once(
        contents,
        "      publishing: ${{ !github.event.pull_request }}\n"
        "    env:\n"
        "      GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}\n"
        "    steps:\n",
        "      publishing: ${{ !github.event.pull_request }}\n"
        "    env:\n"
        "      DIST_PUBLISHING: ${{ !github.event.pull_request }}\n"
        "      DIST_TAG: ${{ github.ref_name }}\n"
        "      GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}\n"
        "    steps:\n",
        "plan environment",
    )
    contents = replace_once(
        contents,
        '''      - id: plan
        run: |
          dist ${{ (!github.event.pull_request && format('host --steps=create --tag={0}', github.ref_name)) || 'plan' }} --output-format=json > plan-dist-manifest.json
          echo "dist ran successfully"
''',
        '''      - id: plan
        shell: bash
        run: |
          if [[ "$DIST_PUBLISHING" == "true" ]]; then
            dist host --steps=create --tag="$DIST_TAG" --output-format=json > plan-dist-manifest.json
          else
            dist plan --output-format=json > plan-dist-manifest.json
          fi
          echo "dist ran successfully"
''',
        "plan command",
    )
    contents = remove_once(
        contents,
        "    container: ${{ matrix.container && matrix.container.image || null }}\n",
        "dynamic container",
    )
    contents = remove_once(
        contents,
        '''      - name: Install Rust non-interactively if not already installed
        if: ${{ matrix.container }}
        run: |
          if ! command -v cargo > /dev/null 2>&1; then
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
            echo "$HOME/.cargo/bin" >> $GITHUB_PATH
          fi
''',
        "container Rust bootstrap",
    )
    contents = replace_once(
        contents,
        '''      - name: Install dist
        run: ${{ matrix.install_dist.run }}
''',
        '''      - name: Install dist (Unix)
        if: runner.os != 'Windows'
        shell: bash
        run: cargo install cargo-dist --version 0.32.0 --locked
      - name: Install dist (Windows)
        if: runner.os == 'Windows'
        shell: pwsh
        run: cargo install cargo-dist --version 0.32.0 --locked
''',
        "local dist installer",
    )
    contents = replace_once(
        contents,
        '''      - name: Install cargo-auditable
        run: ${{ matrix.install_cargo_auditable.run }}
      - name: Install dependencies
        run: |
          ${{ matrix.packages_install }}
''',
        '''      - name: Install cargo-auditable (Unix)
        if: runner.os != 'Windows'
        shell: bash
        run: cargo install cargo-auditable --version 0.7.5 --locked
      - name: Install cargo-auditable (Windows)
        if: runner.os == 'Windows'
        shell: pwsh
        run: cargo install cargo-auditable --version 0.7.5 --locked
      - name: Install cargo-audit (Unix)
        if: runner.os != 'Windows'
        shell: bash
        run: cargo install cargo-audit --version 0.22.2 --locked
      - name: Install cargo-audit (Windows)
        if: runner.os == 'Windows'
        shell: pwsh
        run: cargo install cargo-audit --version 0.22.2 --locked
''',
        "cargo-auditable installer",
    )
    contents = replace_once(
        contents,
        '''      - name: Build artifacts
        run: |
          # Actually do builds and make zips and whatnot
          dist build ${{ needs.plan.outputs.tag-flag }} --print=linkage --output-format=json ${{ matrix.dist_args }} > dist-manifest.json
''',
        '''      - name: Build artifacts
        env:
          DIST_ARGS: ${{ matrix.dist_args }}
          DIST_TAG_FLAG: ${{ needs.plan.outputs.tag-flag }}
        shell: bash
        run: |
          # Values come from the pinned dist plan and are expanded as data, not shell source.
          dist build $DIST_TAG_FLAG --print=linkage --output-format=json $DIST_ARGS > dist-manifest.json
''',
        "local artifact build",
    )
    contents = replace_once(
        contents,
        '''          cp dist-manifest.json "$BUILD_MANIFEST_NAME"
      - name: "Upload artifacts"
''',
        '''          cp dist-manifest.json "$BUILD_MANIFEST_NAME"
      - name: Validate packaged artifact (Unix)
        if: runner.os != 'Windows'
        env:
          DIST_TARGET: ${{ join(matrix.targets, '') }}
        shell: bash
        run: python3 scripts/verify-dist-local-artifact.py --artifact-dir target/distrib --target "$DIST_TARGET"
      - name: Validate packaged artifact (Windows)
        if: runner.os == 'Windows'
        env:
          DIST_TARGET: ${{ join(matrix.targets, '') }}
        shell: pwsh
        run: python scripts/verify-dist-local-artifact.py --artifact-dir target/distrib --target "$env:DIST_TARGET"
      - name: "Upload artifacts"
''',
        "local artifact validation",
    )
    contents = replace_once(
        contents,
        '    runs-on: ${{ matrix.runner }}\n    env:',
        '    runs-on: ${{ matrix.runner }}\n    timeout-minutes: 45\n    env:',
        "local artifact timeout",
    )
    contents = replace_once(
        contents,
        "  build-global-artifacts:\n    needs:",
        "  build-global-artifacts:\n    name: Build global artifacts\n    needs:",
        "global job name",
    )
    contents = replace_once(
        contents,
        '''  build-global-artifacts:
    name: Build global artifacts
    needs:
      - plan
      - build-local-artifacts
    runs-on: "ubuntu-22.04"
    env:
''',
        '''  build-global-artifacts:
    name: Build global artifacts
    needs:
      - plan
      - build-local-artifacts
    runs-on: "ubuntu-22.04"
    timeout-minutes: 30
    env:
''',
        "global artifact timeout",
    )
    contents = replace_once(
        contents,
        '''      - id: cargo-dist
        shell: bash
        run: |
          dist build ${{ needs.plan.outputs.tag-flag }} --output-format=json "--artifacts=global" > dist-manifest.json
''',
        '''      - id: cargo-dist
        env:
          DIST_TAG_FLAG: ${{ needs.plan.outputs.tag-flag }}
        shell: bash
        run: |
          dist build $DIST_TAG_FLAG --output-format=json "--artifacts=global" > dist-manifest.json
''',
        "global artifact build",
    )
    contents = replace_once(
        contents,
        "steps.cargo-cyclonedx.output.paths",
        "steps.cargo-cyclonedx.outputs.paths",
        "cargo-cyclonedx output",
    )
    contents = replace_once(
        contents,
        '''      - name: Install cargo-cyclonedx
        # we specify bash to get pipefail; it guards against the `curl` command
        # failing. otherwise `sh` won't catch that `curl` return non-0
        run: "curl --proto '=https' --tlsv1.2 -LsSf https://github.com/CycloneDX/cyclonedx-rust-cargo/releases/download/cargo-cyclonedx-0.5.5/cargo-cyclonedx-installer.sh | sh"
        shell: bash
''',
        '''      - name: Install cargo-cyclonedx
        run: cargo install cargo-cyclonedx --version 0.5.5 --locked
        shell: bash
''',
        "cargo-cyclonedx installer",
    )
    contents = replace_once(
        contents,
        "  host:\n    needs:",
        "  host:\n    name: Publish tag artifacts\n    needs:",
        "host job name",
    )
    contents = replace_once(
        contents,
        '    runs-on: "ubuntu-22.04"\n    outputs:\n      val:',
        '    runs-on: "ubuntu-22.04"\n    timeout-minutes: 15\n    outputs:\n      val:',
        "host timeout",
    )
    contents = replace_once(
        contents,
        '''    permissions:
      "attestations": "write"
      "contents": "write"
      "id-token": "write"
''',
        '''    permissions:
      "attestations": "write" # Create attestations for reviewed tag artifacts.
      "contents": "write" # Create the reviewed GitHub Release.
      "id-token": "write" # Mint the short-lived attestation identity token.
''',
        "host permissions",
    )
    contents = replace_once(
        contents,
        '''      - id: host
        shell: bash
        run: |
          dist host ${{ needs.plan.outputs.tag-flag }} --steps=upload --steps=release --output-format=json > dist-manifest.json
''',
        '''      - id: host
        env:
          DIST_TAG_FLAG: ${{ needs.plan.outputs.tag-flag }}
        shell: bash
        run: |
          dist host $DIST_TAG_FLAG --steps=upload --steps=release --output-format=json > dist-manifest.json
''',
        "host command",
    )
    contents = replace_once(
        contents,
        '''          RELEASE_COMMIT: "${{ github.sha }}"
        run: |
''',
        '''          RELEASE_COMMIT: "${{ github.sha }}"
          RELEASE_TAG: "${{ needs.plan.outputs.tag }}"
        run: |
''',
        "release tag environment",
    )
    contents = replace_once(
        contents,
        '          gh release create "${{ needs.plan.outputs.tag }}" --target "$RELEASE_COMMIT"',
        '          gh release create "$RELEASE_TAG" --target "$RELEASE_COMMIT"',
        "GitHub release command",
    )
    contents = replace_once(
        contents,
        "  announce:\n    needs:",
        "  announce:\n    name: Confirm release announcement\n    needs:",
        "announce job name",
    )
    contents = replace_once(
        contents,
        '''  announce:
    name: Confirm release announcement
    needs:
      - plan
      - host
    # use "always() && ..." to allow us to wait for all publish jobs while
    # still allowing individual publish jobs to skip themselves (for prereleases).
    # "host" however must run to completion, no skipping allowed!
    if: ${{ always() && needs.host.result == 'success' }}
    runs-on: "ubuntu-22.04"
    env:
''',
        '''  announce:
    name: Confirm release announcement
    needs:
      - plan
      - host
    # use "always() && ..." to allow us to wait for all publish jobs while
    # still allowing individual publish jobs to skip themselves (for prereleases).
    # "host" however must run to completion, no skipping allowed!
    if: ${{ always() && needs.host.result == 'success' }}
    runs-on: "ubuntu-22.04"
    timeout-minutes: 10
    env:
''',
        "announce timeout",
    )
    return contents


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--regenerate", action="store_true")
    parser.add_argument("--dist-bin", default="dist")
    parser.add_argument(
        "--workflow",
        type=Path,
        default=Path(".github/workflows/release.yml"),
    )
    args = parser.parse_args()
    if args.check and args.regenerate:
        parser.error("--check and --regenerate are mutually exclusive")

    if args.regenerate:
        config_path = Path("dist-workspace.toml")
        original_config = config_path.read_text(encoding="utf-8")
        dirty_line = 'allow-dirty = ["ci"]\n'
        if original_config.count(dirty_line) != 1:
            raise SystemExit("dist-workspace.toml has no exact CI allow-dirty contract")
        config_path.write_text(
            original_config.replace(dirty_line, "", 1),
            encoding="utf-8",
        )
        try:
            subprocess.run(
                [args.dist_bin, "generate", "--mode", "ci"],
                check=True,
            )
        finally:
            config_path.write_text(original_config, encoding="utf-8")

    original = args.workflow.read_text(encoding="utf-8")
    hardened = harden(original)
    if args.check:
        if original != hardened:
            raise SystemExit(
                "release workflow is not hardened; run "
                "python3 scripts/harden-generated-release-workflow.py"
            )
        print("release_workflow_hardening=passed")
        return 0

    args.workflow.write_text(hardened, encoding="utf-8")
    print(f"release_workflow_hardened={args.workflow}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
