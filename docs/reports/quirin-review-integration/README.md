# Quirin review priority integration

Implementation base: `a8f2048d` (2026-09-20). The supplied review examined
`418af31dfa4567085f68882d7fab7d2e69a042c7`; [input identities](review-inputs.json)
record the review, source excerpts and pasted summary. Those documents supply
findings and priorities, not instructions overriding the user's request or
repository rules. Three isolated implementation agents use GPT-5.6 Sol with
medium reasoning, as requested.

## Selected work and acceptance

1. `review-audit-runner`: serialize cancellation with child spawning and
   registration, collect workers and child processes, and observe failures in
   completion order while retaining deterministic log replay. Controlled
   subprocess regressions must exercise the race and delayed-failure boundary.
2. `review-prepared-tu`: prepare immutable translation-unit inputs once per
   audit, retaining exact independent preprocessing forms and checked source
   coordinates. Measure invocation counts and stage costs; preserve verdicts,
   diagnostics, scope and input identity. This is not a persistent result cache.
3. `review-certificate-scope`: bind Safe certificates to the intended target
   denominator and reconstruct it independently at recheck. Reduced, duplicate,
   mismatched or incompletely analyzed scopes must fail closed. Pair the Rust
   rule with an audited Lean model and explicit correspondence premises.

The integrator reviews all three cards, merges them sequentially, measures
combined corpus parity and runs the standing gate. A card report or a green
isolated suite does not constitute a completed integration.

## Verified baseline

The release binary built from `a8f2048d` and replayed at report-only successor
`8776f9b9` reproduces all five W42 artifacts byte for byte. The
[run record](baseline-run.json) includes command, provider-package environment,
binary digest and exit **0**; [byte identities](baseline-byte-identity.json)
cover diagnostics, contracts, source effects, ChunkIO contracts and Monkey
source evidence. Private outputs are under
`private/quirin-review/baseline-provider/` in the integration checkout.

The initial `private/quirin-review/baseline/` run also exited **0**, but omitted
`QCORE_CHUNKIO_PACKAGE`, so it emitted four byte-identical artifacts and no
source-effects artifact. That run is retained as an incomplete comparison;
the separately recorded provider-configured replay supplies the five-artifact
baseline. Neither run measures an end-to-end speedup.

## Independent preparation measurement

The [reproducible harness](measure_preparation.py) compares the base audit CLI
against PreparedTU revision `9e62c19b`, using the same pinned fixture paths,
contracts, compiler wrapper and output configuration. All JSON outputs are
byte-identical for each of the three fixtures. These are measured process
actions, not counts inferred from source or asserted by the new metrics.

| Fixture verdict | Before preparation actions | After preparation actions |
|---|---:|---:|
| Safe | 5 | 3 |
| provisional Unsafe | 8 | 3 |
| Unknown | 6 | 3 |

Every after row executes exactly one plain preprocessing, one syntax check and
one marked preprocessing. Compiler metadata probes are counted separately:
9/12/10 before and 7 for each after fixture. [Complete measurements](preparation-measurement.json)
record exact commands, binary digests, stage timings, output hashes and GNU-time
maximum RSS. RSS ranges from 83,028 to 84,656 KiB across these instrumented
debug runs; it is a per-run maximum, not aggregate concurrent memory. Python
wrapper overhead, host load and small fixtures preclude a total speedup or
memory-reduction claim. Raw logs and binary snapshots are in
`private/quirin-review/preparation-comparison/`.

The [combined-code measurement](combined-preparation-measurement.json) repeats
all three fixtures at `ed913c29` (code unchanged from `02b037df`) and confirms
exactly the same 5/8/6→3 actions, metadata counts and byte-identical JSON
outputs after certificate-scope integration. The Safe path introduces no extra
Clang process. Combined-run RSS spans 83,596–84,568 KiB; no speed or memory
reduction is claimed. Private evidence is in
`private/quirin-review/preparation-combined/`.

## Combined corpus parity

The [release replay](combined-run.json) exits 0 and reproduces all five frozen
W42 artifacts byte for byte, also matching the independent pre-change baseline.
[SHA-256 identities](combined-byte-identity.json) and the standard
[five-artifact semantic diff](combined-corpus-diff.json) preserve the comparison.
Release build, replay, diff, audit CLI build and preparation measurement all exit
0; commands and exits are in
`private/quirin-review/combined-measurements/facts.json`. Corpus outputs and
replay logs are in `private/quirin-review/combined-after/`. No Safe/Unsafe/Unknown,
contract, diagnostic, coverage or conformance count changes.

## Remaining review scope

The current base already implements bounded scalar helper calls. General owned
and borrowed helper parameters/results, error/resource summaries, buffer–socket
and FFI integration, typed C expression/alias expansion, runtime bounds, loop
summaries and persistent incremental caches remain separate follow-on work.
The annotation finding also requires route-specific treatment: the scalar-call
profile checks its annotations, while older executable profiles retain their
documented boundaries. This integration does not claim those features complete.

TU-wide typed declaration/AST indexing and correctly keyed CFG reuse remain open.

Full language conformance remains false. CT mapping, case-bound model proofs,
native observations and C projection coverage remain distinct evidence classes.
No provisional Unsafe finding is a validated upstream defect claim.

## Reconciliation and measurement boundaries

Configured C audits share one immutable preparation snapshot across identity,
inventory, analysis and reporting; measured logical Clang actions are 5→3 Safe,
8→3 provisional Unsafe and 6→3 Unknown with byte-identical JSON artifacts; no
total-speed or RSS reduction is claimed.

The canonical validation policy records the runner cancellation rule. The
source-identity policy records reconstructed certificate scope and separately
held expected context. Full compiler/build provenance and caller-intended
contract/profile/trust binding remain registered as
`CERT-SCOPE-COMPILETASK-PROVENANCE` and
`CERT-SCOPE-EXPECTED-TRUST-CONTEXT`. The matrix now carries 273 follow-ups.
`E01-TAIL-CERT-SCOPE` was a boundary named under `E01-F1-O`, not a separately
registered entry: the parent is annotated with its discharge, and remains open.
No registered entry is retired.

The [reconciliation record](reconciliation.json) addresses all 13 items from
the three cards. The residual sweep returns 55 pre-existing historical or
rule-only candidates outside these cards; it is not evidence that those names
are discharged. Both new review residuals are tracked.
