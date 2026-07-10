# Feature Specification: Alerts And Analytics

**Feature Branch**: `006-alerts-and-analytics`

**Created**: 2026-07-08

**Status**: Draft

**Input**: Add bounded local alert evaluation, historical analog lookup, plugin runtime execution, local fee-assumption tradeability, and research microstructure metric proxies while preserving read-only safety.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Define Read-Only Alert Playbooks (Priority: P1)

An operator can define local alerts over public market-data conditions without creating trading actions.

**Why this priority**: Alerts are the next useful layer after screening, but must not become automated execution.

**Independent Test**: Run replay fixtures through alert rules and verify expected events, cooldowns, and read-only outputs.

**Acceptance Scenarios**:

1. **Given** a spread shock plus low confidence, **When** a playbook evaluates it, **Then** it emits a local alert event with caveats and no action route.
2. **Given** an alert is in cooldown, **When** the condition repeats, **Then** duplicate noise is suppressed.

---

### User Story 2 - Search Historical Analogs (Priority: P2)

A researcher can search local recorded history for intervals similar to the selected live or replay state.

**Why this priority**: Analog search turns recordings into research memory.

**Independent Test**: Run fixed historical fixtures and verify top analog matches are deterministic and explain which features matched.

**Acceptance Scenarios**:

1. **Given** a selected symbol state, **When** analog search runs, **Then** results include similar historical windows with distance/explanation values.
2. **Given** insufficient historical data, **When** analog search runs, **Then** it reports insufficient evidence instead of fabricating matches.

---

### User Story 3 - Extend Metrics And Fee-Aware Tradeability (Priority: P2)

A researcher can inspect bounded public-data metric formulas with explicit proxy and unavailable states.

**Why this priority**: The pasted roadmap calls out Amihud, Roll, bipower variation, adverse-selection/toxicity, and fee-aware tradeability.

**Independent Test**: Run deterministic fixtures with known values and verify metric formulas, unsupported-data labels, and fee-sensitive tradeability classifications.

**Acceptance Scenarios**:

1. **Given** enough trade/quote data, **When** metrics are computed, **Then** research proxies match documented formulas within tolerance and remain labeled as proxies.
2. **Given** data is insufficient, **When** the UI renders a metric, **Then** it shows unavailable status rather than a fake value.

---

### User Story 4 - Execute Read-Only Plugins (Priority: P3)

A developer can run bounded read-only plugins against row-scoped inputs.

**Why this priority**: The extension contract exists, but runtime execution is not implemented.

**Independent Test**: Run a fixture plugin with allowed permissions and verify rejected permissions fail before execution.

**Acceptance Scenarios**:

1. **Given** a plugin requests no unsafe capabilities, **When** it runs on fixture rows, **Then** output annotations are bounded and deterministic.
2. **Given** a plugin requests network, filesystem, private data, mutation, or execution, **When** validation runs, **Then** it is rejected before runtime.

### Edge Cases

- Alert rules produce too many events during volatile periods.
- Historical data windows are sparse or confidence-degraded.
- Fee config is missing or stale.
- A metric cannot be computed from public BBO/trade data.
- A plugin hangs, panics, emits oversized output, or requests forbidden capabilities.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Alert playbooks MUST be local, read-only, replayable, and incapable of placing orders or sending exchange actions.
- **FR-002**: Alert events MUST include trigger reason, confidence state, cooldown status, and source interval.
- **FR-003**: Historical analog search MUST run on local recorded/replay data and explain match drivers.
- **FR-004**: Research metrics MUST use documented formulas, remain labeled as proxies until production validation exists, and expose unavailable states when data is insufficient.
- **FR-005**: Fee-aware tradeability MUST use explicit fee configuration and must not require private account data by default.
- **FR-006**: Plugin runtime execution MUST enforce no network, no filesystem, no private account data, no mutation, bounded time, and bounded output by default.
- **FR-007**: Every alert, analog, metric, plugin output, or fee-aware classification MUST be replay-testable.

### Key Entities *(include if feature involves data)*

- **Alert Playbook**: Versioned rule set, cooldowns, severity, and read-only output routing.
- **Alert Event**: Triggered event with evidence, confidence, source window, and cooldown context.
- **Analog Query**: Selected state vector, search scope, distance metric, and result explanations.
- **Metric Definition**: Formula, data requirements, units, validity conditions, and proxy/unavailable semantics.
- **Plugin Runtime Invocation**: Sandboxed execution request, permissions, timeout, input, and bounded output.
- **Fee Profile**: Explicit maker/taker/funding/slippage assumptions used for tradeability classification.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Replay fixtures produce deterministic alert events with zero exchange-action outputs.
- **SC-002**: Analog search fixtures return expected top matches and insufficient-data states.
- **SC-003**: Metric fixtures validate research formulas and proxy/unavailable labels within documented tolerances.
- **SC-004**: Unsafe plugin permission fixtures are rejected 100% of the time before execution.

## Assumptions

- Managed alert delivery, hosted collaboration, and live trading automation remain out of scope.
- Fee profiles are explicit local config, not private account queries.
- Plugin runtime may use WASM or another sandbox only after a dependency review.
