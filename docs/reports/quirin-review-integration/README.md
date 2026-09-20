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

This measurement precedes certificate-scope integration. The combined version
must also preserve the three-action Safe path by using retained scope evidence.

## Remaining review scope

The current base already implements bounded scalar helper calls. General owned
and borrowed helper parameters/results, error/resource summaries, buffer–socket
and FFI integration, typed C expression/alias expansion, runtime bounds, loop
summaries and persistent incremental caches remain separate follow-on work.
The annotation finding also requires route-specific treatment: the scalar-call
profile checks its annotations, while older executable profiles retain their
documented boundaries. This integration does not claim those features complete.

Full language conformance remains false. CT mapping, case-bound model proofs,
native observations and C projection coverage remain distinct evidence classes.
No provisional Unsafe finding is a validated upstream defect claim.
