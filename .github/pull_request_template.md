## Summary

-

## Validation

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] `cargo test --workspace --all-features`
- [ ] `cargo build --workspace --all-features`
- [ ] `git diff --check`

## Read-Only Safety

- [ ] No wallet, private-key, signing, order, cancel, withdrawal, leverage, or exchange-action surface was added.
- [ ] No scores, presets, screenshots, or docs imply trading recommendations or profitability.
- [ ] Public API assumptions are documented or covered by tests.

## Screenshots / Docs

- [ ] README and docs were updated if behavior changed.
- [ ] Screenshots were regenerated with `python3 scripts/generate-screenshots.py` if CLI output changed.
