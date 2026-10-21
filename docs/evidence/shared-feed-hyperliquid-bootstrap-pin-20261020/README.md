# Shared Hyperliquid bootstrap producer pin

Hlscreen pins both direct shared dependencies and all five resolved marketfeed
packages to producer `56b733dfdce0b865ad121d49c4d71d720b305733`.
All 52 Cargo checkout files match that Git revision; three Hyperliquid adapter
files changed from the prior pin. The initial pin-only candidate retained all
419 registry records. A fresh audit then found RUSTSEC-2026-0380 and
RUSTSEC-2026-0379 in Wasmtime 36.0.18. The final candidate raises Wasmtime to
36.0.19 and updates exactly 26 Wasmtime/Cranelift/Pulley patch packages and their
checksums, retaining 393 complete registry records. Dependency edges, kinds and
features match after those patch and Git identity substitutions. Generated
attribution carries the new package versions. This is source compatibility
evidence, not binary equivalence or complete producer input closure.

The upstream [36.0.19 release](https://github.com/bytecodealliance/wasmtime/releases/tag/v36.0.19)
fixes both compiler advisories. Existing extension tests execute a safe Wasm
module and reject unsafe manifests, bad hashes, wrong-symbol output and an
oversized module. Those tests check runtime compatibility and existing contracts;
they do not reproduce these compiler vulnerabilities or prove arbitrary Wasm safe.

[Validation](validation.json) records the qualification status, exact commands,
source bindings, rebuilt saved-prefix replay and preservation checks. Earlier
receipts retain their original pins and failures; this pin does not requalify
historical continuity, strict datasets or operational performance.

The producer adds Hyperliquid spot/perps to the private MFCJ3 disk/null bootstrap,
with native discovery/catalog and ordered-subscription binding. Direct session
construction enforces the planner's 200-native-subscription bound. The producer's
local qualification and independent review are source-scoped; its own hosted
qualification is recorded separately. Hlscreen consumes MFSF1 and rejects all
three causal headers before watermark access. Committed projection, production
actual-engine replay and daemon/CLI/live integration remain unfinished.

The registered durable-ingress workload remains failed and was not rerun.
cryptofeed owns acquisition, native identity/timing/quality and storage/replay;
rsibot owns context/features/models/ETF research and decisions; hlscreen owns
consumption and presentation. Source-specific hosted main/native/archive checks,
native continuity, strict datasets, calendar/hardware and operational admission
remain separate gates. No economics, activation, deployment or release follows.
