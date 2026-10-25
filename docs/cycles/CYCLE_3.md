# Cycle 3: Ownership Without Errors

**Status: Done**

Evaluation profile: none (a language and model cycle: its acceptance is the standing gate's Rust tests and Lean audits; it claims no audit measurement over a pinned source set).

Cycle 3 implements Stage 3 of the prototype roadmap. A cycle is complete only
when its executable Rust behavior and corresponding Lean model both pass their
builds.

## Phase 0: warm-up prerequisite

Before cycle 3 implementation begins:

- mark cycle 2 as done;
- move the cycle 2 result out of the repository root into `docs/cycles/`;
- update repository documentation to point at cycle 3 as the active cycle.

## Executable semantics

The Rust core must provide:

- exclusive `own` bindings for tracked resource identities;
- ownership moves that invalidate the source without changing identity or
  aggregate footprint;
- explicit drops and transfers out of the current function;
- lexical shared and mutable borrows, with mutable-borrow exclusivity;
- a resource identity ledger with resource type, owner, exact multidimensional
  footprint, and acquisition order;
- pointwise equality between the stored aggregate live vector and the sum of
  ledger footprints after every accepted transition;
- branch joining only when both ownership states agree;
- normal-exit rejection when a local resource remains live.

Quantity arithmetic overflow and any ledger/live-vector inconsistency must be
explicit errors.

## Lean verification obligations

The Lean core must model the same ledger interpretation and prove:

1. fresh acquisition preserves unique resource identities;
2. ownership moves preserve resource identities and aggregate live quantity;
3. release removes the selected identity and accounts for exactly its
   footprint;
4. a shared loan blocks move/drop of its resource;
5. a mutable loan is exclusive and blocks move/drop of its resource;
6. an empty normal-exit ledger has the zero aggregate live vector.

These obligations are checked by `lake build`; theorem statements containing
`sorry` do not satisfy the cycle.

## Acceptance commands

```sh
cargo test --workspace
cd proof && lake build
```

Completion was verified with 33 passing Rust tests, warning-free strict Clippy,
and a successful Lean build containing no `sorry` or `admit` placeholders.
