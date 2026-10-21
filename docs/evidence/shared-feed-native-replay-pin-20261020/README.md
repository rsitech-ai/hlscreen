# Shared-feed native replay producer pin

This slice pins both direct producer dependencies and all five transitive
marketfeed Git package identities to cryptofeed-rs
`95f00f51e076c83577228d95dc6605f3fb72df9c`. Registry package versions/checksums,
Wasmtime 36.0.19 and native HLS behavior remain the prior b1ecfba baseline.

The producer adds a single-owner actual-engine MFCJ3 disk replay reader.
Hlscreen continues to consume MFSF1 and explicitly rejects MFCJ1/2/3 before
watermark access. Committed projection, live migration and full native feature
preservation are separate required work. Passing compatibility checks cannot
establish causal consumption or repair the registered failed ingress workload.

Completion for this pin requires full local PR checks, exact dependency-graph
and shared-source comparison, a rebuilt CLI and exact saved-prefix replay,
full-lock audit/policy/attribution checks and independent evidence review. Retain
all 33 historical HLS evidence files, the two pilot files and the original
22-file frozen dataset with their recorded bytes and metadata. Publish this
source only as a draft update and inspect its own hosted/native artifact checks.
Rollback is the prior b1ecfba consumer pin; original captures are never rewritten.

The prior [bootstrap/security receipt](../shared-feed-hyperliquid-bootstrap-pin-20261020/README.md)
keeps its original source scope, including the minimal Wasmtime security patch.
Producer95 owns new local replay evidence; parent56b and HLSb1ec hosted success
do not qualify this consumer source. Rsibot owns strategy context/features,
model/ETF research and decisions; cryptofeed owns collection/storage/replay;
hlscreen owns presentation. No economics, activation, deployment, merge, release
or operational promotion follows from this pin.

Local qualification passed: all 10 checks, 567 Rust test executions (544 unique),
12 Python tests, full-lock audit/policy/attribution and byte-identical saved replay
pass. See [validation.json](validation.json) for exact commands, hashes and limits.
The rebuilt CLI is retained separately with mode 0555. Own-source hosted main,
native validations and artifact/archive inspection remain pending.

The first attempt terminated during compilation because the disk was full. Its
logs/result remain unqualified. The unchanged-source retry disables incremental
compiler caching after removal of disposable target/debug/incremental caches;
source, captures and evidence were preserved. No product policy or dependency
version was changed to obtain the pass.
