# CG29-EXECUTION-COVERAGE-AND-RECLAMATION: prediction

Frozen before any implementation code was written (wave 48, round 11; base
`main` `f6336c9d`). The BEFORE measurement was still running when this was
committed, so the baseline figures below are the brief's expectation, not yet
this card's measurement.

## What the card will build

1. **Execution coverage (W07)**, a new core module
   `qcore_core::execution_coverage`. Its input is a bounded, loop-free program
   fragment: atomic locations with initial values, and per thread a list of
   guarded instructions (atomic load, store, exchange or add RMW,
   compare-exchange, fence, non-atomic read and write). The module enumerates
   every candidate execution:
   - every thread path, with each read's value drawn from a per-location
     value domain;
   - every reads-from choice, every modification order and every S order;
   - synchronises-with derived from these.
   It checks each candidate with the unchanged `check_c11_execution`. The
   value domain is a static closure of the fragment's literals under its RMW
   arithmetic. It is justified without control flow, so out-of-thin-air
   cycles cannot escape it. The program-level verdict is:
   - `RaceFree` only when every candidate is checked and no consistent one
     races;
   - `Race`, with a witness execution, when some consistent one does;
   - `Unknown`, with a named reason, when a bound (threads, events, values,
     candidates) is exceeded, a loop is present or the fragment is
     malformed.
   The bounds are part of the evidence identity.
2. **Reclamation (W10)**, a new core module `qcore_core::reclamation`.
   - Hazard-pointer and epoch-based reclamation are protocol checkers over
     an explicit event trace.
   - A protocol contract maps helper operations to protocol roles, and is
     itself checked. Each helper boundary is `Declared`.
   - The store/load handshake the trace model assumes is either discharged
     by an execution-coverage `RaceFree` verdict on the handshake fragment,
     or recorded as `Declared`.
3. **Production caller.** The atomic-publication verdict
   (`qcore_verification::atomic_publication_source`, rule 11i) will run the
   extracted shape (publisher, guarded consumer) through the enumerator. A
   `Safe` verdict then carries an all-executions assumption with the bounds,
   in place of `SINGLE_GRAPH_ASSUMPTION`. An `Unknown` verdict keeps the
   single-graph assumption.
4. **Lean.**
   - `QCore.ExecutionCoverage` (new): enumeration completeness, value-domain
     soundness and verdict soundness, with shared fragment vectors.
   - `QCore.Reclamation` (new): the hazard-pointer and epoch invariants, and
     safety (no access to a reclaimed object in an accepted trace).
   - `QCore.WeakMemory` gains a spawn event and spawn edges in
     happens-before (additive), so initialisation happens before every
     thread, as in the Rust checker. The 20 litmus vectors stay unchanged.

## Census prediction

- **Program-level atomic or race claims.** None needed in the corpus, the
  system route or the tail-pause slice.
  - The only routed atomics are cmetrics' three wrappers
    (`cmt_atomic_load`, `cmt_atomic_store`, `cmt_atomic_compare_exchange`),
    each with a custody verdict of route G. Route G claims no ordering, so
    no consumer asks for race freedom over executions.
  - The system and the slice have 0 atomic leaves and 0 `atomic-route`
    parity rows.
- **Hazard-pointer and epoch reclamation.** No protocol in the Fluent Bit
  tree. Its only keyword hits are type names in the eBPF `vmlinux.h`
  headers. The property has **no corpus consumer**; its controls are
  synthetic.

## Measurement prediction

| Run | Prediction |
|---|---|
| Corpus BEFORE (`f6336c9d`) | **633 / 10 / 4,191**: `f0bbc0eb`'s 632 / 10 / 4,192 plus CG28's `cmt_atomic_compare_exchange` |
| Corpus AFTER | **identical to the BEFORE**: no move, no leaf change, the same `resource-diagnostics.json` digest |
| System and tail-pause slice | **identical** totals and functions, with no atomic leaf |
| Safe → non-Safe moves | none, so no correction |
| Added leaks or violations | none |
| Atomic-publication route | Its fixtures keep their statuses (`Safe` stays `Safe`, every `Unknown` control stays `Unknown`). The enumerator answers `RaceFree` for every shape the extractor accepts (release or seq_cst store, acquire or seq_cst load, guarded use). Only the `Safe` verdict's assumption list changes. The corpus arbiter does not run this route (0 functions). |

## Core-property prediction (reported apart from custody)

- **MP**, release/acquire with a guarded payload read: `RaceFree` over every
  candidate execution.
- **MP**, relaxed: `Race`.
- **SB handshake** (store, then load of the other location, each guarding a
  conflicting non-atomic access):
  - relaxed: `Race`, in the execution where both loads read the initial
    values;
  - with seq_cst fences, or all seq_cst: `RaceFree`.
- A bound exceeded, or a loop: `Unknown` with the bound named.
- **Reclamation.**
  - Accepted: a hazard-pointer trace that validates before the unlink and
    reclaims only after the hazard is cleared.
  - Refused: an access without a validated hazard (use after retire), a
    validation after the unlink, and a reclaim while a hazard holds the
    object.
  - Epoch: an advance with a thread still in the previous epoch is refused,
    and so is a reclaim less than two epochs after the retire.
