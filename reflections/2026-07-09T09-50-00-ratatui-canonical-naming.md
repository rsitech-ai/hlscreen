## Task
- **ID/Title:** Canonical Ratatui workstation naming
- **Date:** 2026-07-09
- **Scope:** multi-file

## Plan and Risks
- **Planned approach:** Rename the transitional `interactive_tui` test surface to `workstation_interaction`, update test names and validation docs, and preserve existing runtime behavior.
- **Top failure hypotheses:** Cargo test target references could drift; documentation could still point to the old test file; a naming-only slice could accidentally alter renderer behavior.
- **Success criteria:** No current operational docs or test targets reference `interactive_tui` or `interactive_renderer`; focused TUI tests pass under the new target name; full formatting/tests/clippy stay green. Historical memory/reflection logs may retain old command evidence.

## Candidate Attempts
| Candidate | Summary | Outcome | Signals | Why selected / rejected |
|---|---|---|---|---|
| A | Remove the legacy deterministic text renderer. | Rejected | `screen` and non-TTY fixture outputs still use it. | Too risky and not required to make `hls live --tui` canonical. |
| B | Rename transitional test/docs terminology only. | Selected | Keeps behavior stable while removing ambiguity around the canonical TUI. | Directly addresses the user's "just this TUI" wording without collateral breakage. |

## Reflection
- **Failure modes observed:** A broad search still found the obsolete substring in historical memory/reflection entries and in a historical branch name recorded by operational docs.
- **Root cause:** The test file had been renamed after earlier plan/memory entries copied its old target name, and the old branch name embedded the same `interactive_tui` substring.
- **Fix that resolved it:** Renamed the active test target to `workstation_interaction`, renamed `interactive_renderer_*` tests to `workstation_renderer_*`, updated current plan/memory/agent lesson references, and removed the obsolete branch-name literal from active operational docs. Historical logs were left intact.
- **What improved score/quality:** The source tree now has one canonical named live TUI surface: the Ratatui workstation. Future validation commands point at the workstation interaction tests instead of suggesting a second or transitional TUI.
- **Useful command-level evidence:** `rg -n "interactive_tui|interactive_renderer" README.md PLAN.md MEMORY.md TODO.md docs crates scripts specs --glob '!memory/**' --glob '!reflections/**'`; `cargo fmt --check`; `cargo test -p hls-tui --test workstation_interaction --test ratatui_cockpit`; `cargo test -p hls-cli --test live_mock`; `cargo test --workspace --all-features`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo build --workspace --all-features`; `git diff --check`.
- **Branch comparison insight (if multiple attempts):** Not applicable.

## Reusable Lesson
- **Pattern that worked:** Separate active operational docs from historical audit logs when removing obsolete naming.
- **Pattern to avoid:** Leaving transitional names in tests after the product surface becomes canonical.
- **Where to apply next:** Any future TUI docs/specs should call the surface Ratatui workstation or live cockpit consistently.

## Decision
- **Final chosen approach:** Behavior-preserving test target rename plus active docs update.
- **Commit/rollback decision:** Commit and push after green focused and full workspace gates.
- **Next step / follow-up:** Continue visual/runtime workstation improvements; this slice only canonicalizes naming.
