# Completion/Intent producer pin

The seven direct and 23 resolved marketfeed packages select published producer
`20f5b1fa16665ce93d0d2da6764e9ad0fd2bfca9`, whose 570 compiled inputs have recipe
`e5cd4371d81a253bccc4cc3d645519726fcfe168e0333a40c944b3fe525ec75d`.
The coordinated pin must preserve all 430 registry records and the 461-node,
1,315-edge dependency graph and features after producer-identity substitution.
HLS runtime code, tests and historical fixture bytes retain their existing
contracts.

The producer now permits bounded Completion/ReleaseIntent co-admission while
retaining each record's durable acknowledgement. Completion settlement survives
a later Intent failure; output handoff and application-write authorization still
require their existing barriers. These are producer-owned persistence changes,
not strategy decisions or a new HLS wire format. See the published
[producer capture evidence](https://github.com/rsitech-ai/cryptofeed-rs/blob/20f5b1fa16665ce93d0d2da6764e9ad0fd2bfca9/docs/evidence/owned-hyperliquid-capture-20261020/README.md).

This README precedes the candidate source freeze and local checks. Actual local
qualification, release/package inspection, security results and any later joint
run are separate derivative receipts naming their exact source and artifacts.
No prospective passing result or future artifact is asserted here. The prior
HLS1338/source69e3 qualification remains historical and does not qualify this pin.
Own hosted CI and native-package jobs must identify the eventual HLS commit;
no tagged release or deployment follows from local checks.

The unchanged historical 5479 synthetic fixture proves portable structural
consumption with 67 groups, 2 market events and 6 gaps. It is distinct from current
producer real-owner tests and the public joint14 sample. Source replay/native
comparison is producer work; HLS structural reading does not authenticate native
financial values. Native acquisition freshness, historical usable time, complete
continuity, strict datasets and extended operation remain unqualified.

Joint14's clean seal and final-file tail do not establish an availability time.
Timestamp/receipt differences use unsynchronized clocks, and comparisons with
previous public samples are uncontrolled. Original timing failures and the
registered 480-frame/s workload retain their own results. HLS presents shared
observations; Quants/rsibot owns context and strategy admission. No economic run,
activation, orders, merge or operational promotion is claimed.
