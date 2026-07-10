# Feature Definitions

The v1 feature set is a transparent screener surface, not a prediction engine.

- `spread_bps`: best-ask minus best-bid divided by mid price, in basis points.
- `tob_depth_usd`: top-of-book bid notional plus ask notional.
- `tob_imbalance`: top-of-book bid notional versus ask notional, bounded to `[-1, 1]`.
- `spread_shock_bps`: the largest recent spread expansion versus the local
  pre-shock top-of-book baseline.
- `spread_recovery_ms`: elapsed time from the detected spread shock to the
  first quote that recovered to the documented threshold.
- `signed_notional_flow_30s`: buy trades minus sell trades over the latest
  30-second exchange-time window.
- `bbo_ofi_proxy_30s`: a best-bid/best-ask queue-change proxy over the latest
  30-second window. It uses only public top-of-book sizes and prices.
- Return and volatility windows are computed from local trades whose exchange timestamps fall inside the requested decision-time window.
- Score fields are bounded heuristic ranks from `0` to `100`, not trade signals.

## Current Formula Definitions

- `spread_bps = (ask_px - bid_px) / ((bid_px + ask_px) / 2) * 10_000`
- `tob_depth_usd = bid_px * bid_sz + ask_px * ask_sz`
- `tob_imbalance = (bid_notional - ask_notional) / (bid_notional + ask_notional)`
- `ret_1m`, `ret_5m`, and `ret_1h` are timestamp-bounded trade returns for the last 1 minute, 5 minutes, and 1 hour.
- `rv_1m`, `rv_5m`, and `rv_1h` are population standard deviations over trade-to-trade returns inside each timestamp window, or `0` when fewer than three trades are available.
- `volume_z_1h` and `trade_count_z_1h` compare the latest candle with the prior candle baseline; they return `0` when there is not enough baseline variation.
- `liquidity_score = clamp(tob_depth_usd / 100, 0, 100)`
- `momentum_score = clamp(50 + selected_return * 100, 0, 100)`, where `selected_return` prefers `ret_5m`, then `ret_1m`, then `ret_1h`.
- `mean_reversion_score = clamp(50 - selected_return * 100, 0, 100)`, using the same selected return.

These scores are screen ordering aids only. They are not predictions, recommendations, or profitability claims.

## Market Composite

`hls tui` builds a chained market index from official 1m constituent candles;
it never averages incompatible raw asset prices. The index starts at `100`.
Constituent weights are proportional to the square root of finite positive
24-hour quote notional, capped at 10%, and renormalized within each available
minute. Missing constituents reduce the displayed liquidity-weight coverage
instead of producing synthetic flat candles.

Breadth is equal-weight and reported separately as advances, declines,
unchanged, and stale/missing symbols. Live quote volume uses summed public trade
notional when available and is labeled `ExactTrades`; historical candle volume
uses `volume_base * close` and is labeled `CloseApproximation`. Neither measure
is private fill volume or an execution signal.

## Research Microstructure Metrics

`FeatureSnapshot.microstructure_metrics` carries metric-level evidence with a
`support` value of `canonical`, `proxy`, or `unavailable`. The current runtime
does not emit any production-validated canonical metric. Implemented values are
bounded research proxies or explicit unavailable states.

Current metric contracts:

| Metric | Support when value exists | Formula / rule | Unit | Unavailable when |
| --- | --- | --- | --- | --- |
| `amihud_1m` | proxy | bounded `abs(return_1m) / dollar_volume_1m` over public trades | `return_per_usd` | fewer than two trades or non-positive public notional |
| `roll_effective_spread` | proxy | bounded adjacent public trade-price-change estimate | `price` | fewer than four trades or non-negative adjacent price-change covariance |
| `bipower_variation_5m` | proxy | trade-to-trade adjacent absolute-return products without canonical time-bar sampling | `decimal_variance` | fewer than three valid public trades |
| `bbo_ofi_proxy_30s` | proxy | best-level queue-change notional from public BBO updates | `usd_notional` | fewer than two BBO updates in the 30-second window |
| `signed_flow_toxicity_proxy_30s` | proxy | `abs(sum(signed_public_trade_notional_30s)) / sum(abs(public_trade_notional_30s))` | `ratio` | fewer than two trades or non-positive public notional in the 30-second window |
| `adverse_selection_toxicity_proxy` | proxy | ordinal `0/1/2` from public signed flow, BBO OFI proxy, resilience, and TOB depth | `ordinal` | missing signed flow, BBO OFI proxy, or TOB depth |

Important caveats:

- `amihud_1m`, `roll_effective_spread`, and `bipower_variation_5m` are bounded
  public-trade research formulas. Their windowing and sampling have not been
  validated as canonical production estimates.
- `roll_effective_spread` is unavailable when the public trade-price changes do
  not show the negative serial covariance required by the Roll estimator.
- `bbo_ofi_proxy_30s` is still a best-level proxy. It is not full-book OFI.
- `signed_flow_toxicity_proxy_30s` is a bounded public trade-flow concentration
  proxy. It is not canonical toxicity, private fill quality, or adverse
  selection measured from account execution.
- `adverse_selection_toxicity_proxy` is an ordinal warning, not a toxicity
  model, fill model, or trading recommendation.
- The compact TUI `amihud` column prefers the public-trade Amihud-style proxy
  when available and falls back to the older spread/depth liquidity proxy.

## Liquidity Resilience and Tradeability

Liquidity resilience fields are derived from public `bbo` and `trades` events
only. They are designed to answer whether the visible top of book recovered
after a quoted-cost shock; they do not inspect hidden liquidity or full depth.

Current states:

- `resilience_state`: `unknown`, `normal`, `shock`, `recovering`, or `brittle`
- `tradeability_state`: `unknown`, `tradeable`, `costly`, `thin`, or `stale`
- `adverse_selection_proxy`: `unknown`, `normal`, `watch`, or `brittle`

Current rules:

- A spread shock requires both an absolute expansion of at least `25 bps` and a
  spread at least `2x` the local pre-shock baseline.
- Recovery is counted when the latest spread returns to the larger of `1.5x`
  baseline or baseline plus `10 bps`.
- A shock that remains unrecovered for more than `10 seconds` is labeled
  `brittle`.
- `tradeability_state = tradeable` requires fresh data, sufficient confidence,
  a normal resilience state, spread at or below `25 bps`, and at least `$5K` of
  top-of-book depth.
- `thin` is emitted for fresh rows with enough quote history but less than
  `$1K` of top-of-book depth.
- `unknown` is emitted when BBO history is insufficient; sparse windows are not
  silently treated as zero.

Important caveats:

- `bbo_ofi_proxy_30s` is a BBO-only order-flow imbalance proxy. It is not full
  order-book OFI and must not be interpreted as total depth pressure.
- `adverse_selection_proxy` is a screen warning from signed public trade flow,
  BBO OFI proxy, resilience state, and top-of-book depth. It is not a fill
  model, toxicity oracle, or execution recommendation.
- `tradeability_state` describes visible quoted cost and data quality only. It
  does not include fees, slippage beyond top-of-book, funding, market impact,
  account limits, or order placement feasibility.

## Fee-Aware Tradeability

Fee-aware tradeability is optional additive evidence. The default
`tradeability_state` remains public-data-only and does not change unless a
caller explicitly configures a local `FeeProfile` in the feature engine or
passes a local profile file to `hls screen --fee-profile-file` or bounded
`hls server --live --fee-profile-file`.

Current implemented contract:

- Fee configuration uses an explicit local profile name plus integer
  hundredths-of-basis-points for maker fee, taker fee, taker fill ratio,
  slippage buffer, and round-trip thresholds.
- `taker_fill_ratio_hundredths` is an explicit local assumption from `0` to
  `10000`; omitted profiles default to `10000` for backward-compatible
  all-taker economics.
- `blended_fee_bps = taker_fee_bps * taker_fill_ratio + maker_fee_bps * (1 - taker_fill_ratio)`.
- `expected_round_trip_cost_bps = spread_bps + 2 * blended_fee_bps + slippage_buffer_bps`.
- If the base public-data tradeability state is not `tradeable`, fee-aware
  evidence preserves that base state and records the base-state reason.
- If the base state is `tradeable`, the fee-aware state stays `tradeable` only
  when expected round-trip cost is at or below the profile's tradeable
  threshold; otherwise it becomes `costly`.

Screen fields exposed when fee evidence exists:

- `fee_tradeability_state`
- `fee_expected_round_trip_cost_bps`
- `fee_profile`
- `maker_fee_bps`
- `taker_fee_bps`
- `taker_fill_ratio`

Important caveats:

- Fee-aware output does not query private account fee tiers, wallet state, user
  fills, or account limits.
- It is not a fill model, profitability model, routing model, or execution
  feasibility check.
- `taker_fill_ratio` is not inferred from realized fills; it is a local
  operator-supplied assumption used only to make screening costs explicit.
- Current CLI support is limited to explicit local JSON/TOML profile files for
  `hls screen` and read-only `hls live` filtering/rendering. Account fee-tier
  lookup and fill/execution modeling remain out of scope.

## Data Confidence

The microstructure workstation uses `DataConfidenceSnapshot` as the shared
contract for row-level data quality. It starts at `100` and degrades when the
pipeline observes evidence that a row should not be fully trusted.

Confidence levels:

- `high`: score `90..100`
- `medium`: score `70..89`
- `low`: score `30..69`
- `untrusted`: score below `30`

Current foundation reason codes:

- `reconnect_gap`: a reconnect or missed interval affected the row.
- `stale_quote`: quote freshness is outside the accepted window.
- `sparse_trades`: there are not enough trades for a windowed calculation.
- `duplicate_events`: duplicate events were observed or deduped.
- `parser_drops`: parser failures affected available evidence.
- `writer_backlog`: local recording pressure could affect durability.
- `incomplete_window`: one or more feature windows are not valid yet.

The terminal market board keeps this confidence state visible next to the row
score and in each row's detail card. A low-confidence row must not
silently look equivalent to a fully trusted row.

## Score Breakdowns

The microstructure score contract stores named components rather than a single
opaque number. `ScoreBreakdown` records:

- raw component total, clamped to `0..100`
- confidence-adjusted total, also clamped to `0..100`
- confidence score used for adjustment
- named components such as liquidity, momentum, spread cost, signed flow,
  resilience, metadata, or custom components
- per-component raw values, normalized values, weights, signed contributions,
  direction, and evidence windows
- unavailable evidence names when a public row cannot support a component

The screen DSL exposes `score_total`, `score_raw_total`,
`score_confidence_penalty`, and `score_component.<name>`. `hls explain` renders
the same model as the TUI why-ranked pane. Score breakdowns remain screen
heuristics; they are not orders, trade recommendations, execution simulations,
or performance proof.

## Metrics Contract

Metric definitions use `hls_`-prefixed names and low-cardinality labels. Labels
such as `symbol`, `run_id`, account, wallet, address, transaction hash, and
trade id are rejected by the foundation contract. Symbol-level detail belongs in
TUI rows, structured logs, replay artifacts, or top-N summaries, not in every
histogram label.

## Screening Rules

`hls-screen` supports a small deterministic rule language over `FeatureSnapshot` fields:

- Boolean operators: `and`, `or`
- Comparisons: `>`, `>=`, `<`, `<=`, `==`, `!=`
- Literals: numbers, strings, booleans
- Function: `abs(field)` for numeric fields
- Sort syntax: `field:asc`, `field:desc`, `abs(field):asc`, `abs(field):desc`

Built-in presets:

- `liquid_momentum`
- `volume_anomaly`
- `tight_spread_movers`
- `mean_reversion_watch`
- `thin_books`
- `liquidity_resilience`
- `brittle_tradeability`
- `flow_pressure`

Missing numeric values do not match numeric comparisons. Invalid expressions are rejected and do not replace the active screen session.

## Alerts And Analytics Boundaries

Alerts are local, read-only replay/live annotations over public `FeatureSnapshot`
rows. `AlertPlaybook` rules can emit `AlertEvent` records with trigger reason,
confidence level, confidence score, source interval, severity, and cooldown
state. Alert actions must be `local_only`; exchange actions, orders, wallet
operations, private account data, and external delivery are rejected or left out
of the current model.

Current implemented alert condition grammar:

- `spread_shock_and_low_confidence`: emits when `spread_shock_bps` is at or
  above the configured threshold and `confidence.score` is at or below the
  configured maximum.
- `field_threshold`: emits when one typed numeric field crosses a configured
  threshold. Supported fields are `confidence_score`, `spread_bps`,
  `spread_shock_bps`, `tob_depth_usd`, `tob_imbalance`,
  `signed_notional_flow_30s`, `bbo_ofi_proxy_30s`, `rv_1m`, `rv_5m`, and
  `day_ntl_vlm`. Supported operators are `gt`, `gte`, `lt`, `lte`, and `eq`.
- `all`: emits when every child condition emits; empty condition lists fail
  validation.
- `any`: emits when at least one child condition emits and reports the first
  matching child reason; empty condition lists fail validation.
- `not`: emits when its child condition does not emit. The reason is
  deliberately conservative because missing public evidence can make the child
  condition false.

The local alert evaluator suppresses repeated events for the same playbook,
rule, and symbol while the rule cooldown is active. Suppressed attempts are
reported separately as `SuppressedAlert` records so replay output can explain
why an event was not emitted.
When `--alert-history-file` is supplied, emitted timestamps in that local JSONL
history seed cooldown state on the next CLI invocation. Without a history file,
cooldown state is intentionally limited to the current process invocation.

`hls alerts` evaluates either the built-in local playbook or a user-supplied
JSON/TOML playbook file over replayed rows or deterministic fixture rows, can
print JSON for scripting, and can append emitted and suppressed evidence to
local JSONL with `--alert-history-file`.
`hls alerts --history-file <path>` lists recent local history records with
optional `--symbol`, `--limit`, and `--json` output. These paths are local-only:
they do not send exchange actions, query private account data, use wallet
state, or deliver external notifications. File-backed playbooks can use the
fixed spread-shock condition or the typed threshold/boolean grammar above.

Live evaluation and Ratatui alert-history panes are not wired yet. Current alert
behavior is an explicit standalone local command, not a scheduler or operational
alert engine.

## Historical Analog Search

`hls analog` searches local normalized replay windows for `FeatureSnapshot`
states similar to a selected symbol's latest replayed state. It can either scan
a normalized local run directly or write/read a schema-versioned local JSON
index with `--write-index <path>` and `--index-file <path>`. The implementation
is intentionally local: it does not require a hosted data lake, private account
data, or network calls while searching a recorded run or local index.

Current comparable dimensions:

- `spread_bps`
- `tob_imbalance`
- `signed_notional_flow_30s`
- `bbo_ofi_proxy_30s`
- `rv_5m`
- `liquidity_score`
- `momentum_score`

Each match includes a normalized distance and the largest contributing feature
differences. Candidates require at least three comparable dimensions. If local
history is too sparse or the caller asks for more valid candidates than exist,
the report returns `insufficient_evidence` and an empty `matches` array instead
of fabricating analogs.

Analog output is research context only. It is not a prediction, recommendation,
execution simulation, or profitability claim.

Still planned beyond the current `specs/006-alerts-and-analytics` local
surface:

- Live evaluation, external delivery, daemon scheduling, and TUI alert history
  panes. The current local playbook grammar supports fixed spread-shock
  rules, typed `field_threshold`, `all`/`any`/`not` boolean composition, and
  explicit local JSONL evidence history with CLI listing.
- Larger database/service-backed historical analog indexes beyond the current
  local JSON index.
- Richer adverse-selection/toxicity analytics beyond the current public-data
  ordinal proxy.
- Account/fill-model-aware fee economics. The current implementation supports
  optional library-configured fee profiles, `hls screen --fee-profile-file`,
  and bounded `hls server --live --fee-profile-file`, but no private account
  fee-tier lookup.
- Broader plugin runtime surfaces. The current extension layer supports bounded
  standalone CLI row-annotation execution; live integration, plugin discovery,
  TUI panels, and score/health annotation execution remain future work.
