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
