# Quickstart: Production Release And Packaging

## Local Validation

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --release --workspace --all-features
scripts/check-release-packaging.sh
git diff --check
```

## Release-Candidate Proof

1. Build the reviewed source revision.
2. Create and verify the local archive and checksum.
3. Run `hls --help`, `hls doctor`, and the bounded fixture smoke from the unpacked archive.
4. Review the generated release plan and CI result.
5. Do not claim a published release until a reviewed `v*` run has produced and exposed the expected artifacts and checksums.

## Expected Outcome

Local packaging mechanics pass, while public binary installation remains
explicitly pending until the tag workflow and release page are verified.
