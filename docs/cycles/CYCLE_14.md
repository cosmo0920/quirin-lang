# Cycle 14: Concurrency and Resource-Race Verification

**Status: Done**

Evaluation profile: none (a language and model cycle: its acceptance is the standing gate's Rust tests and Lean audits; it claims no audit measurement over a pinned source set).

Cycle 14 implements the specification's restricted first-order concurrency
core as an explicit checked event IR. It does not infer concurrency safety from
ordinary usage grades or resource-count vectors.

## Rust implementation

`compiler/src/concurrency.rs` provides:

- identity-indexed half-open resource footprints and conserved access shares;
- read, write, atomic-read, and atomic-write access effects;
- scoped spawn/join handles with quiescent scope exit;
- vector clocks for sequenced-before, spawn/join, mutex handoff, and explicit
  synchronized custody transfer;
- linear mutex capabilities, same-task unlock, and increasing lock ranks;
- join-while-locked rejection;
- unique destruction custody; and
- diagnostics identifying both events in a conflicting unordered access.

The checker deliberately does not treat event-list order as cross-thread
happens-before. Same-mutex release/acquire orders critical sections; different
mutex identities do not. The tests cover shared readers, races, same- and
different-mutex accesses, lock duplication/order, join quiescence, synchronized
custody, and atomic-to-atomic access.

Detached tasks, condition variables, dynamic task collections, general
lock-free algorithms, atomic reclamation, and a native threads backend remain
outside this bounded core and are not claimed by the certificate.

## Lean verification

`proof/QCore/Concurrency.lean` models the accepted checker boundary and proves
that accepted accesses are authorized and conflict-ordered, mutex capabilities
and destruction custody are unique, and accepted scopes are quiescent.
`proof/AuditCycle14.lean` audits each theorem.

## Verification

```sh
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
if rg -n '\b(sorry|admit)\b|^[[:space:]]*axiom\b' proof --glob '*.lean'; then exit 1; fi
cd proof
lake build
lake env lean AuditCycle14.lean
```

Cycle 15 is the next implementation milestone.
