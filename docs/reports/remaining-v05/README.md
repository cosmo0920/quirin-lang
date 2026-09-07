# V05 — R06: exact versus bounded loop evidence (adversarial verification)

Date: 2026-09-07. Baseline: `6c2782f`. Branch: `claude/qcore-execution-audit-plan-24520f`.

**Verdict: verified — no defect found, no repair needed.** `Exact(n)` and
`AtMost(n)` loop evidence are semantically distinct in the resource-flow /
accounting model, and the Exact→AtMost mutation cannot retain a false exact
final-custody result. Five regression tests were added to pin the gaps that
existing coverage left open. No production code changed; no gate was weakened
and no expectation was deleted.

## Scope of the checked surface

| Component | Path |
|---|---|
| Evidence vocabulary and summary construction | `crates/qcore-core/src/resource_accounting.rs` (`LoopBoundEvidence`, `summarize_general_loop`) |
| Independent replay | `crates/qcore-core/src/resource_accounting.rs` (`check_loop_summary`, `loop_ledger_morphism`) |
| Flow arithmetic | `crates/qcore-core/src/resource_flow.rs` (`repeat_exact`, `bounded_zero_net_loop`, `bounded_loop`) |
| Aggregate closure replay | `crates/qcore-verification/src/research_extensions.rs` (`close_cycle17`) |
| Lean model | `proof/QCore/ResearchExtensions.lean` (`LoopCount`, `summarizeBoundedLoop`) |

`LoopBoundEvidence` is a supplied-premise model API consumed by the Cycle 17
research-extension closure. It is not the route by which C loops are analyzed:
`crates/qcore-c-gateway/src/legacy_c.rs` reaches loops through CFG fixed-point
abstract interpretation and never constructs a `LoopBoundEvidence`.

## Acceptance 1 — the matrix

Bodies (`memory` axis, unit footprint and unit cost):

- `positive-net` = `Flow::acquire` — net `+1` per iteration
- `negative-net` = `Flow::release` — net `-1` per iteration
- `zero-net-acquire-first` = acquire then release — net `0`
- `zero-net-release-first` = release then acquire — net `0`

| body | count | `Exact(n)` | `AtMost(n)` | early exit |
|---|---|---|---|---|
| positive-net | 0 | accepted, summary `Flow::zero()`, net 0 | accepted, `Flow::zero()` | only 0 iterations covered |
| positive-net | 1 | accepted, net `+1` | **`NonExactCustody`** | no shorter run covered |
| positive-net | 2 | accepted, net `+2` | **`NonExactCustody`** | no shorter run covered |
| negative-net | 0 | accepted, `Flow::zero()` | accepted, `Flow::zero()` | only 0 iterations covered |
| negative-net | 1 | accepted, net `-1`, required entry 1 | **`NonExactCustody`** | no shorter run covered |
| negative-net | 2 | accepted, net `-2`, required entry 2 | **`NonExactCustody`** | no shorter run covered |
| zero-net-acquire-first | 0/1/2 | accepted, net 0 | accepted, net 0, cost scaled by count | every actual ≤ count covered |
| zero-net-release-first | 0/1/2 | accepted, net 0 | accepted, net 0, cost scaled by count | every actual ≤ count covered |

Covering tests:

- `crates/qcore-core/tests/unit/resource_accounting.rs::exact_and_maximum_evidence_are_distinct_across_the_r06_matrix`
  (new) — the whole table, including the `is_covered_by` early-exit column. For
  nonzero-net bodies it asserts `observed.is_covered_by(exact.summary) == (actual == count)`
  for every `actual` in 0..=2, i.e. exact evidence admits no early exit.
- `…::maximum_counts_cannot_forge_exact_final_custody` (existing) — the nonzero-net
  rows for `AtMost`, `Terminating` and `Widened` at bounds 0..=2.
- `…::zero_net_maximum_covers_every_admitted_early_exit` (existing) — the zero-net
  `AtMost` rows and their early-exit coverage.
- `…::positive_and_negative_net_loops_have_checked_invariants` (existing) — exact
  positive/negative net invariants at count 3.
- `crates/qcore-core/tests/unit/resource_flow.rs::bounded_zero_net_loop_scales_only_total`
  (existing) — cost scaling and the nonzero-net refusal at the `Flow` layer.

Observation recorded, not a defect: `summarize_general_loop` short-circuits
`AtMost(0)`/`Terminating(0)`/`Widened(0)` to `Flow::zero()` for *any* body,
including a nonzero-net one, while `Flow::bounded_zero_net_loop(0)` refuses a
nonzero-net body outright. Both readings agree that a zero-iteration loop is the
empty flow, and `repeat_exact(0)` is `Flow::zero()` too, so the short-circuit
claims nothing extra. The Lean model short-circuits identically
(`summarizeBoundedLoop body (.atMost 0) = some (summarizeLoop body 0)`).

## Acceptance 2 — Exact→AtMost cannot retain a false exact result

| mutation | result |
|---|---|
| `Exact(n)` → `AtMost(n)`, nonzero net, n > 0 | **rejected**: `LoopSummaryError::NonExactCustody` from `check_loop_summary`; `CategoryError::InvalidLoopSummary` from `loop_ledger_morphism` |
| `Exact(0)` → `AtMost(0)`, any body | admitted — both denote `Flow::zero()`; there is no exact custody claim to retain |
| `Exact(n)` → `AtMost(n)`, zero-net body | admitted — and `repeat_exact(n)` is *equal* to `bounded_zero_net_loop(n)` for every zero-net body, with net 0 under both readings, so no exact final state survives the weakening |
| `AtMost(n)` → `Exact(n)`, zero-net body | admitted on the same ground; the summaries are identical |
| summary swapped for a shorter exact run under `AtMost(2)` | **rejected**: `LoopSummaryError::InvalidCertificate`, `CategoryError::InvalidLoopSummary` |

An `AtMost(n)` loop with nonzero net change is never constructible: it cannot
produce the exact final state `Exact(n)` would, because it produces no summary
at all. Unsupported nonzero-net upper bounds reject; alternatives are not
collapsed into an optimistic exact reading.

Covering tests:
`crates/qcore-core/tests/unit/resource_accounting.rs::evidence_mutation_never_retains_an_exact_final_custody_claim`
(new, all four rows over the full body × count matrix) and
`…::loop_ledger_morphisms_separate_exact_and_maximum_deltas` (new: `Exact(2)`
delta `+2` / `-2` versus a zero-net `AtMost(2)` with empty delta and cost 4, plus
the swapped-summary rejection).

## Acceptance 3 — independent certificate replay

Two replay paths were exercised, both on a mutated `Exact(2)` → `AtMost(2)`
certificate over a positive-net body:

1. `qcore_core::resource_accounting::check_loop_summary` — re-derives the summary
   from `(body, evidence)` and returns `NonExactCustody`. `loop_ledger_morphism`
   calls the same replay first and returns `InvalidLoopSummary`, so the ledger
   interpretation cannot be reached with a mutated certificate.
2. `qcore_verification::research_extensions::close_cycle17` — the aggregate
   Cycle 17 closure. The mutated certificate returns
   `Cycle17Error::Loop(LoopSummaryError::NonExactCustody)`; the unmutated one
   passes that step. New test:
   `compiler/tests/unit/research_extensions.rs::aggregate_closure_rejects_loop_evidence_mutated_from_exact_to_maximum`
   (Clang-free: the loop check precedes every source-dependent step).

## Acceptance 4 — gaps closed

Existing tests already covered the nonzero-net `AtMost` refusal, the zero-net
`AtMost` early-exit coverage, and the nonzero-net Exact→AtMost mutation. Five
minimal tests were added for the gaps:

| test | file | gap |
|---|---|---|
| `exact_and_maximum_evidence_are_distinct_across_the_r06_matrix` | `crates/qcore-core/tests/unit/resource_accounting.rs` | zero-net bodies under **Exact**; the "exact admits no early exit" direction |
| `evidence_mutation_never_retains_an_exact_final_custody_claim` | same | mutation on zero-net bodies and at count 0; the reverse AtMost→Exact mutation |
| `loop_ledger_morphisms_separate_exact_and_maximum_deltas` | same | ledger deltas for exact positive/negative loops versus a bounded zero-net loop |
| `bounded_loop_alias_is_exact_repetition_not_a_maximum_count` | `crates/qcore-core/tests/unit/resource_flow.rs` | the historical `bounded_loop` alias had no test pinning it to exact repetition, despite its maximum-sounding name |
| `aggregate_closure_rejects_loop_evidence_mutated_from_exact_to_maximum` | `compiler/tests/unit/research_extensions.rs` | aggregate-closure replay of a mutated loop certificate |

## Source-level reachability

Four probe programs were run through `cargo run -p qcorec -- check`
(`while` bodies with a balanced body, a nonzero-net body, and a loop inside a
live-handle scope, plus a loop-free control):

| probe | result |
|---|---|
| loop-free control (`examples/cycle12.qc` shape) | `checked 1 function(s); entry main`, exit 0 |
| `while` around a check, outer live handle | `Q500: structured control flow is not in the Cycle 12 executable subset`, exit 1 |
| `while` with a balanced acquire/drop body | same `Q500`, exit 1 |
| `while` with a nonzero-net (leaking) body | same `Q500`, exit 1 |

The front end parses `while` structurally (`StatementKind::While`) but the source
compiler refuses it. The Exact/AtMost distinction is therefore **not reachable
from the source route at all**: no source loop is admitted, so none is summarized
under either evidence. This is fail-closed and consistent with invariant 1;
loops in source remain unimplemented, not optimistically approximated.

## Lean parity

No semantic change was made, so no Lean change was required. The existing model
in `proof/QCore/ResearchExtensions.lean` already states the checked properties
and matches the Rust behavior cell for cell:

- `exact_count_retains_exact_net` — exact evidence keeps `n * body.net`
- `positive_maximum_nonzero_net_rejected` — the mutation target is refused for a
  nonzero-net body at every positive bound
- `accepted_maximum_has_zero_net` — every admitted maximum has zero net
- `maximum_total_covers_early_exit` — `actual ≤ bound → actual * total ≤ summary.total`
- `summarizeBoundedLoop body (.atMost 0)` matches the Rust zero-count short-circuit

`proof/QCore/Flow.lean` carries the matching `boundedZeroNetLoop` theorems
(`bounded_loop_has_zero_net`, `bounded_loop_preserves_peak`,
`bounded_loop_scales_total`).

## Standing per-change gate

| gate | result |
|---|---|
| `git -c protocol.file.allow=always submodule update --init --recursive` | ok (libevent at `d82464a277d0f42703702c4dfd9af6af38595a83`) |
| `cargo test --workspace` | **824 passed**, 0 failed (819 before; +5 new) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0, `failed_expectations: 0` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `lake build` (proof, elan) | 62 jobs, success |
| `python3 scripts/check_lean_audits.py` | Passed 20 Lean audits |
| `python3 -m unittest discover -s scripts/tests` | 43 tests, OK |
| `cargo test` in `providers/` | 45 passed, 0 failed |

## What this does not establish

The summary API carries a supplied count premise; it does not derive source
control flow, iteration counts or allocation identity. Scalar ledger balance is
not an identity discharge. Arbitrary nonzero-net upper-bound inference remains
deliberately unimplemented and out of scope. No raw Cycle 19 unit verdict, CT
mapping or contract-corpus pin changed.
