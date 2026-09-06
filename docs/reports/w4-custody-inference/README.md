# W4: interprocedural custody summary inference

Date: 2026-09-06. Status: W4 of the
[generalization plan](../../GENERALIZATION_PLAN.md) is executed for
**parameter/return custody with validation**. LC01's open boundary in
[`REEVALUATION_FOLLOWUP.md`](../../REEVALUATION_FOLLOWUP.md) is closed for that
scope. Cycle 19's raw units remain **unpromoted**; nothing here changes the
verdict contract, and the projection numbers below are coverage, not safety.

## What landed

`compiler/src/legacy_summary.rs` adds a bottom-up, SCC-ordered summary pass over
the per-function CFGs the unrestricted-C gateway already builds. For each
internal function it derives a `ResourceFunctionContract` — the *same* record a
`.qcore` module declares and a provider proposes — and publishes it as a
`qcore_schemas::contract::ResourceSummaryRecord` labelled
`ContractTrust::InferredValidated`. Declared contracts, inferred summaries, and
(future) provider proposals are now one data shape with three trust labels.

### Inference lattice

Parameter modes are ordered by what the caller must guarantee: `Borrowed`
(nothing) below `Owned` (a live non-null resource across the call), with
`Released` and `Consumed` describing effects rather than requirements. The
lattice the search walks is the **binding pattern** — which formals are seeded
with an entry resource — because `Owned`, `Consumed`, and `Released` prescribe
the same entry state and differ only in the exit disposition they claim.

Iteration starts from the *empty* binding, and this is a soundness requirement,
not a tuning choice. Seeding a formal with a live non-null resource also asserts
that it is **not null**, which refutes a null test in the body and deletes the
failure path a summary has to describe. An earlier draft of this pass did seed
every named formal; it inferred a non-null owned result for
`void *w(int c) { if (c) { void *p = q_alloc(); return p; } return 0; }`,
because binding `c` pruned the `return 0` path. That is a false-`Safe`
generator, and it is why a formal is bound only after the analysis reports it as
an unresolved resource identity — that is, only once the body has been observed
treating it as a resource it owns. The binding set only grows, so the loop
settles after at most one round per formal.

### Validation mechanism

Inference alone is worthless here. Once the binding pattern is stable, each bound
formal's mode is read off its exit disposition and the resulting contract is
checked against a **fresh** analyzer run seeded from that contract
(`observations_match_contract`, the same predicate `analyze_defined_function`
and the certificate recheck apply). A summary is published only if:

1. the body is fully modeled and defect-free — `Safe`, no diagnostics at all, so
   any unsupported construct, unresolved alias, leak, or double release rejects
   it;
2. at least one exit is reachable;
3. every reachable exit disposes of every formal exactly as the contract says,
   with no disagreement between exits;
4. no fresh resource is left attached to a caller-owned formal, because such a
   transfer is invisible at the call site;
5. the returned value matches the declared result mode at every exit, and a
   formal's own site is never returned as if it were fresh.

A candidate failing any condition yields no summary at all: the call stays
uncontracted and therefore `Unknown`. Parameter resources are exempt from
exit-leak reporting — discharging them is the caller's obligation — which is
exactly why condition (4) exists.

### Fixed-point rule for recursion

Components are processed callees-first (iterative Tarjan) and each component is
iterated to a fixed point seeded at "no summary". A member of a nontrivial
component, or a self-recursive function, always has a direct callee inside its
own component, so its body always contains an unmodeled call and never
validates; the least fixed point for a recursive component is empty and its
members stay `Unknown`.

The pass deliberately does **not** accept the greatest fixed point — an
assume-guarantee summary merely consistent with itself. `void bad(void *p) { bad(p); }`
is consistent with "releases `p`", but only for executions that never terminate,
and this gateway must not certify a non-terminating program as `Safe`. This
mirrors `whole_program.rs`, where a recursive QCore function must *declare* its
contract (`FunctionSemantics::recursive_contract`) rather than have one inferred.

### Declared contracts always win

An explicit `.qcore` contract governs every call site unconditionally. Inference
still runs for such a function, and a validated summary that disagrees with the
declaration raises `LegacyDiagnosticKind::SummaryConflict`, which keeps the
defining function `Unknown` and reports both contracts. The conflicting inferred
summary is not published as evidence. This is the LC01 lesson: never a silent
override in either direction.

### Certificate containment

`LegacySafetyCertificate` gained a `summaries: Vec<ResourceSummaryRecord>` field.
`check_legacy_safety_certificate` re-derives the summary set from the
preprocessed source and the declared contracts alone and rejects the certificate
if the two sets differ, or if any record is not `InferredValidated`. A
certificate that leans on a summary the recheck cannot reproduce fails
containment; a summary is never taken on the certificate's word.

### One analyzer fix this required

`ResourceReturnMode::None` with an assignment destination now *removes* the
destination's binding instead of leaving the previous one in place. Before W4
this mattered rarely; with internal calls modeled it is load-bearing —
`q = neutral_call(p)` must not leave `q` aliasing the resource it used to hold,
or the discarded resource would be covered by a stale alias.

## What is deliberately not inferred

The inferred vocabulary is `Borrowed`/`Owned`/`Consumed`/`Released` parameters
and `None`/`Owned` results. Bodies needing anything else yield no summary:

| Shape | Why no summary | Result |
| --- | --- | --- |
| `Out` parameter (`*out = p`) | `*out` is outside the modeled place vocabulary, so the body is not fully modeled | call stays `Unknown` |
| `Stored` into a caller aggregate | condition (4) rejects it outright | call stays `Unknown` |
| `Reallocated` wrapper | custody depends on whether the result was null; the analyzer refuses to escape a possibly-null place, so no settled exit describes it | call stays `Unknown` |
| Formal returned unchanged | would mint a second custody for one resource | call stays `Unknown` |
| Formal released on one path, live on another | no single parameter mode holds | call stays `Unknown` |
| Possibly-null result returned without a null test in the callee | the analyzer refuses the escape | call stays `Unknown` |
| Recursion (self or mutual) | least fixed point is empty | call stays `Unknown` |
| Ambiguous declarator list (variadic, function-pointer parameter, K&R, unnamed tagged type, duplicate names) | attaching custody to the wrong argument would be unsound, so the parser refuses the whole function | call stays `Unknown` |

A separate precision limit, not a soundness one: the reduced importer has no
scalar type environment, so a formal used as the source of an ordinary
assignment (`ch->error_n = status`) is bound and summarized `Owned`. Call sites
passing a non-resource there then report "arguments do not match resource
contract" and stay `Unknown` — the same verdict as before W4, but no gain.

## Corpus measurement

Command, run against Fluent Bit `66910c10a4d7eafa810b84229cf2dcf7b0f26f97` (the
pinned revision) with its existing `build/compile_commands.json`:

```sh
cargo run --release -p qcore-compiler \
  --example cycle19_fluent_bit_libraries_audit -- ../fluent-bit
```

**Scope caveat.** This invocation covers **156 of the pinned 185 translation
units**. The Monkey / forced-Libevent leg (29 units: 1 Safe / 18 provisional
Unsafe / 10 Unknown in the pinned triage) needs a forced-libevent
`compile_commands.json` for the Monkey checkout, which does not exist in this
environment. That leg was **not re-measured**; its numbers below are quoted from
[`cycle19-resource-triage`](../cycle19-resource-triage/README.md), not
reproduced. The five measured slices reproduce the pinned table exactly at
baseline, which is what makes the comparison meaningful.

### Per verdict class

| Slice | Units | Safe before → after | Provisional Unsafe before → after | Unknown before → after |
| --- | ---: | ---: | ---: | ---: |
| CMetrics | 31 | 0 → 0 | 19 → 19 | 12 → 12 |
| CTraces | 17 | 1 → 1 | 2 → 2 | 13 → 13 |
| librdkafka | 94 | 0 → 0 | 53 → 53 | 38 → 38 |
| ChunkIO | 13 | 0 → **1** | 5 → 5 | 8 → **7** |
| Fluent Bit input chunk | 1 | 0 → 0 | 1 → 1 | 0 → 0 |
| **Measured total** | **156** | **1 → 2** | **80 → 80** | **71 → 70** |
| Monkey, forced Libevent (not re-measured) | 29 | 1 | 18 | 10 |
| Pinned aggregate | 185 | 2 → *2 + 1 measured* | 98 → *98 measured-unchanged* | 81 → *81 − 1 measured* |

Data-only units (4) and gateway failures (0) are unchanged. Unknown stays in the
denominator: the measured corpus is 2 Safe out of 152 non-data-only units, and
the 70 Unknown units are not evidence of anything.

### The one verdict movement

`chunkio/cio_error.c` moves Unknown → Safe. Its four functions
(`cio_error_get_str`, `cio_error_get`, `cio_error_set`, `cio_error_reset`)
perform no resource operation in the modeled vocabulary; the unit was Unknown
only because `cio_error_get_str`'s call to the internal `cio_error_get` was
uncontracted. `cio_error_get` now carries a validated `([Borrowed], None)`
summary, the call is modeled, and the unit is fully projected and defect-free.
The promotion went through the validated path and through
`check_legacy_safety_certificate`, which re-derived the same summaries.

This is a genuine but small and vacuous `Safe`: it says the four functions leave
no modeled resource obligation undischarged, over this translation unit's
projected scope, under the declared contract assumptions. It is not a claim
about ChunkIO.

### Diagnostic inventory

| Category | Before | After | Delta |
| --- | ---: | ---: | ---: |
| unsupported | 39,990 | 39,861 | −129 |
| unknown_alias | 7,958 | 7,942 | −16 |
| leak | 857 | 860 | **+3** |
| double_release | 35 | 35 | 0 |
| summary_conflict | — | 0 | 0 |

The +3 leaks are all in `cmetrics/cmt_encode_opentelemetry.c`, which was already
provisional Unsafe (43 → 46 leaks) and stays so. Newly modeled calls surface
additional **provisional** leak allegations inside already-Unsafe units; per
invariant 2 these are not confirmed upstream defects, and no unit entered the
Unsafe class from Safe or Unknown. Zero summary conflicts across the corpus:
no declared contract disagreed with a validated inference.

## Regression suite

`compiler/tests/unit/legacy_summary.rs` pins the LC01 repair list. The cardinal
expectation of every case is negative — nothing becomes `Safe` unless every
obligation is discharged:

| Case | Pinned outcome |
| --- | --- |
| LC01 allocation wrapper, result discarded | **Unsafe with a leak** in the caller (was a false `Safe` at the finding, `Unknown` after the 2026-09-05 repair) |
| LC01 allocation wrapper, result released | Safe; `make` summarized `() -> Owned{nonnull}` |
| Caller discard (result never bound) | Unknown: discarded owned result |
| Caller release through a wrapper | Safe; wrapper summarized `Released{nonnull}` |
| Caller release, then release again | Unsafe, double release |
| Nested wrappers | composed bottom-up; leak reported, discharge accepted |
| Parameter consumption | Safe; `Consumed{nonnull}`; release afterwards is release-after-escape |
| Borrowed (non-resource) parameter | Safe; `Borrowed`, no custody obligation |
| Observed parameter (`q_mutate`) | `Owned{nonnull}`, not `Borrowed`; calling after release stays Unknown |
| Direct recursion | Unknown |
| Self-recursive allocation wrapper | Unknown |
| Mutual recursion | Unknown for every member |
| Out-parameter wrapper | no summary; call stays uncontracted |
| Realloc wrapper | no summary; the inline reallocation still resolves both outcomes |
| Unsupported body / leaking callee / conditional release / caller-aggregate store / formal returned unchanged | no summary; never `Safe` |
| Nullable-returning wrapper | `Owned{nullable}`; caller without a null check stays Unknown |
| Declared contract agreeing with inference | preserved, no conflict |
| Declared contract disagreeing with inference | `SummaryConflict`, unit Unknown, summary not published |

`compiler/tests/unit/legacy_c.rs`'s
`internal_definitions_never_receive_an_automatic_neutral_contract` replaces the
superseded `lc01_internal_calls_require_explicit_custody_contracts`: being
defined in the translation unit still buys a function nothing, and each of its
four bodies defeats validation for a different reason.

The follow-up probe's `LC01_returned_owner_lost_at_internal_call` expectation now
observes `Unsafe` with a `Leak` diagnostic in `main` — the finding's exact
counterexample — rather than the `Unknown` recorded by the 2026-09-05 repair.

## Lean parity

`proof/QCore/CustodySummary.lean` models the validation relation
(`validate`, `exitMatches`, `parameterMatches`, `resultMatches`) and proves what
a validated summary implies: an unclear analysis never validates, a body with no
reachable exit never validates, a validated summary matches *every* reachable
exit, nothing is attached to a formal, a neutral result forces every exit to
return nothing, an owned result never hands back a formal, a non-null owned
result never returns nothing, a released formal is released at every exit, and a
borrowed formal binds nothing. Two concrete theorems pin LC01: the wrapper does
not validate as neutral, and does validate as an owned return.

`proof/AuditW4.lean` prints the axiom surface: every theorem depends only on
`propext` and `Quot.sound`. No project axioms, no placeholders.

## Gate

```
git -c protocol.file.allow=always submodule update --init --recursive   # ok
cargo test --workspace                                                  # ok
cargo fmt --all -- --check                                              # ok
cargo clippy --all-targets -- -D warnings                               # ok
cargo run -p qcore-compiler --example reevaluation_probe                # exit 0
cargo run -p qcore-compiler --example reevaluation_followup             # exit 0
cd proof && lake build                                                  # ok
cd proof && lake env lean AuditW4.lean                                  # ok
```

## What W4 still leaves open

- **Crate relocation.** The plan places this pass in `qcore-c-gateway`
  (`legacy_c.rs`, `clang.rs`, `typed_c_ast.rs`). It landed in the
  `qcore-compiler` grab-bag alongside the analyzer it is inseparable from,
  following W1's own two-step precedent (loader first, relocation second). The
  extraction is unstarted.
- **The unsupported modes** in the table above: `Out`, `Stored`, `Removed`, and
  `Reallocated` inference, plus possibly-null returns without a callee-side null
  test. Each needs analyzer work (modeled `*out` places, path-sensitive
  reallocation), not summary work.
- **Recursive components** stay `Unknown` by construction. Accepting a
  greatest-fixed-point summary would need a termination argument this project
  does not have.
- **Scalar formals** are bound whenever they source an ordinary assignment, which
  costs precision at call sites (see above). A typed importer would fix it.
- The Juliet temporal-safety held-out corpus (plan W4 task 4's stretch item) is
  not wired up.
