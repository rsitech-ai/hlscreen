# Coordinated shared-feed integration

Hlscreen's seven direct and 23 resolved marketfeed dependencies use immutable
producer commit `84aa63534f79ae65161117effd624d66f0294f48`, compiled-input recipe
`0f332bdf2535607281fe89330e3457498e5d81ef91867d6a4b05d2f3e91d0221`.
The pin checker, full resolved graph and actual Cargo cache source bytes must
agree. The unchanged 430 registry packages and dependency features are checked
against the prior238 pin; no new wire schema or runtime capacity is introduced.

This directory carries portable qualification receipts. `qualification.json`
records actual local checks, source inventory, retained artifact, security and
package results. `joint12.json` records the separately registered simultaneous
growing-feed check. A receipt qualifies only the source, artifact and commands it
actually names; earlier successes do not qualify a later build. Missing receipts
or failed gates remain incomplete. Hosted Mac/Linux checks and the user's later
Razer run have separate source-specific outcomes.

The producer's [joint11 evidence](https://github.com/rsitech-ai/cryptofeed-rs/blob/84aa63534f79ae65161117effd624d66f0294f48/docs/evidence/owned-hyperliquid-capture-20261020/README.md)
reconciles7,708 records,1,509 projected items and1,699 released native events,
with a25.683136ms final-file source-to-target tail. It used the qualified
predecessor hlscreen artifact pinned to238. Those results do not substitute for
the newly pinned hlscreen artifact's own checks.

Native acquisition freshness remains **UNQUALIFIED**: perps timestamp/receipt
differences exceed eight seconds in joint11. Receipt timestamps mark application
read completion after the owner readiness gate; they do not measure kernel/TLS
arrival. Initial historical trade snapshots are separate from subsequent trades.
Source-to-target file tail, local health, a clean Seal and structural ACKs do not
establish historical availability, complete market continuity or a latency SLA.
The next acquisition diagnostic must preserve the existing owner barriers,
workload, caps, clocks and abort rules.

Quants' [draft integration PR98](https://github.com/s1korrrr/quants-lab/pull/98)
provides an independent reader and durable paired-cursor/application checkpoint.
Joint11 orderly restart checks reach7,708 groups/1,509 items and reopen sealed
idle without reapplying; all147 protected archive files retain their hashes and
recorded metadata. This is distinct from synthetic hard-crash/lost-ACK tests and
does not grant exactly-once external effects. Its own hosted run39561301014
executed zero steps due to account payment/spending restrictions. Account owner
action and an unchanged workflow rerun are required.

The native context owner remains Hummingbot22f in a separate detached checkout;
rsibot root gitlinks are unchanged. Actual context qualification is synthetic
spot-only. Real history, availability, full RESEARCH_V1 and perps context remain
unqualified. Original480 ingress and earlier watchdog/timing failures remain
retained. No orders, activation, deployment, merge, release or extended soak is
claimed by these infrastructure checks.
