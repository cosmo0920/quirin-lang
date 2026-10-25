# Cycle 5: Typed Errors Without Cleanup Packs

**Status: Done**

Evaluation profile: none (a language and model cycle: its acceptance is the standing gate's Rust tests and Lean audits; it claims no audit measurement over a pinned source set).

Cycle 5 implements Stage 5 of the prototype roadmap. It intentionally permits
cleanup-free catches only; sealed cleanup packs and `quant open` belong to
cycle 6.

## Phase 0: warm-up prerequisite

Before cycle 5 implementation begins:

- mark cycle 4 as done;
- move the cycle 4 result out of the repository root into `docs/cycles/`;
- update repository documentation to point at cycle 5 as the active cycle.

## Executable semantics

The Rust core provides:

- checked error declarations and finite qualitative effect rows;
- explicit normal and `error<E>` exit tags;
- path-sensitive exit maps retaining multiple post-ledgers for one error tag;
- `raise`, with no normal continuation and exact preservation of the current
  live-resource ledger;
- `?` propagation through exact per-exit call-summary ledger transitions;
- sequential composition that advances normal paths while retaining prior
  error exits;
- branch joining with equal-ledger validation on normal paths while preserving
  distinct error paths;
- catch clauses that remove handled throw effects and validate their normal
  joins;
- rejection of cleanup-free catches whenever a matching edge differs from the
  catch-entry ledger;
- separate checks for undeclared, effect-disallowed, propagated, and remaining
  unhandled errors.

## Lean verification obligations

The Lean core models the same path-sensitive exit-map semantics and proves:

1. `raise` has no normal exit, preserves its error ledger, and records its
   typed throw effect;
2. `?` propagation preserves every declared callee post-state;
3. sequencing preserves already-raised error exits;
4. a validated branch join gives every normal path one common ledger while
   retaining distinct error-path states;
5. cleanup-free catch recovery requires and preserves the entry baseline;
6. catching removes the handled error exit and handled throw effect;
7. validated catch joins have one normal ledger;
8. every accepted remaining error is declared and present in the effect row.

These obligations are checked by `lake build`; theorem statements containing
`sorry` or `admit` do not satisfy the cycle.

## Acceptance commands

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cd proof && lake build
```

Completion was verified with 51 passing Rust tests, warning-free strict Clippy,
and a successful Lean build containing no proof placeholders.
