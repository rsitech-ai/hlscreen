# Artifact child-output decoding repair

The previous hosted artifact run at `05ef784` reported success on all build
jobs, but its Windows validator logged a cp1252 reader-thread decoding exception
and then a success marker. That Windows packaged-validation result is not clean
or qualified. The original raw log and historical receipts remain preserved.

The validator now captures bytes and decodes both streams as strict UTF-8 in the
calling thread. Invalid output, nonzero exit and timeout prevent success. Four
real-child regressions cover both streams and failure paths. Independent tests
pass in both Python UTF-8 modes; the previous helper fails the forced-cp1252
regression in both modes. The tests run in the existing packaging check.

The corrected validator also passes against the actual downloaded hosted macOS
ARM archive: checksum and archive checks, extracted executable help/doctor,
offline live fixture and embedded dependency audit. This is local macOS evidence;
corrected hosted Windows validation requires a new run. See the exact scoped
[receipt](validation.json).

Rust source, shared-feed pins, dependencies, retained binary/replay and all 23
historical evidence files plus both original archives remain unchanged. This
repair does not qualify unfinished producer causal replay/live integration,
the failed ingress workload, strict datasets or operational promotion.
