# Cycle 11: Whole-Program Checking and Summaries

**Status: Done**

Evaluation profile: none (a language and model cycle: its acceptance is the standing gate's Rust tests and Lean audits; it claims no audit measurement over a pinned source set).

Cycle 11 connects the Cycle 10 typed source boundary to the sequential semantic
engines from Cycles 2–7. It checks every source function as one transaction,
computes interprocedural summaries, and produces a checked cleanup-explicit CFG.
Source-to-C file compilation and the CLI remain Cycle 12.

## Rust implementation

`compiler/src/whole_program.rs` provides:

- exactly one semantic bundle for every source function symbol;
- source-span validation for every interprocedural call;
- one pipeline over usage, ownership and lexical borrows, resource flow,
  typed errors, error regions, cleanup lowering, and CFG validation;
- ordered resource-flow segments around calls and composition of callee flows;
- propagation of qualitative effects through the call graph;
- call-graph cycle detection and explicit contracts for every recursive SCC;
- a post-fixed-point rule requiring one recursive unfolding to be covered by
  its declared flow, effects, and parameter ownership summary;
- source-type/summary arity checks and mandatory borrowed modes for borrow
  parameters;
- cross-function identity transfer for borrowed, consumed, and returned owned
  arguments, rejecting missing, duplicated, or owned-aliased identities;
- declared resource-bound and caller-effect-row checks on completed summaries;
- translation validation from each accepted lowering input to its checked CFG;
  and
- source-located diagnostics at the owning function or call span.

The public entry point is `check_whole_program`. Its result retains the typed
source program, all checked pass results, resolved summaries, recursive-function
set, and checked CFGs.

## Lean verification

`proof/QCore/WholeProgram.lean` defines the reduced whole-program boundary,
summary rounds, recursion rule, and identity consumption. It proves that:

1. successful pass checking establishes every pass-interface invariant;
2. checked pass input and whole-program source are preserved;
3. every accepted function has a checked CFG;
4. source and CFG identity domains agree;
5. recursive functions have an explicit declared summary;
6. cross-function consumption cannot mint an identity;
7. consumption preserves identity uniqueness; and
8. recursive summary rounds select the declared contract.

`proof/AuditCycle11.lean` audits the foundational dependencies of all Cycle 11
theorems. No theorem contains a proof placeholder or project-defined axiom.

## Verification

From the repository root:

```sh
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
if rg -n '\b(sorry|admit)\b|^[[:space:]]*axiom\b' proof \
  --glob '*.lean'; then exit 1; fi
cd proof
lake build
lake env lean AuditCycle11.lean
```

Cycle 11 is verified by 106 passing Rust tests (97 existing plus nine Cycle 11
integration tests), strict formatting and Clippy, a placeholder-free Lean
build, and the Cycle 11 theorem-dependency audit.

Cycle 12 is the next implementation milestone.
