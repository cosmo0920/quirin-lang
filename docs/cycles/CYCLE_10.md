# Cycle 10: Complete Source Front End

**Status: Done**

Evaluation profile: none (a language and model cycle: its acceptance is the standing gate's Rust tests and Lean audits; it claims no audit measurement over a pinned source set).

Cycle 10 implements the source-front-end prerequisite in the post-Cycle-9
roadmap. It preserves the checked IR APIs from Cycles 1–9 while adding one
canonical, source-located boundary for later whole-program integration.

The consolidated revision 0.2 specification in `../obsolete/QCORE_SPEC_r0_2.md`
already incorporates the checked-concurrency amendment. Cycle 10 recognizes
its new surface forms for forward compatibility, but it does not claim the
concurrency safety results assigned to the later C0–C7 concurrency roadmap.

## Rust implementation

`compiler/src/frontend.rs` provides:

- structural parsing for every revision 0.1 top-level declaration family;
- structural parsing for let/index bindings, assignment, expressions, move,
  drop, discharge, guard, branches, matches, loops, returns, raises, loop
  control, defer, error regions/catches, and unsafe blocks;
- forward-compatible parsing of `concurrency checked`, footprints, scoped
  tasks, spawn/join expressions, and structured mutex guards;
- exact byte spans for the lexical stream, declarations, parameters, blocks,
  and statements;
- explicit type classification for atomic, named, tuple, array, slice,
  pointer, owned, borrowed, quantitative, access-quantitative, result,
  function, task, mutex, and never types;
- collection of global and lexical bindings into stable symbol identities;
- duplicate-global, duplicate-parameter, duplicate-local, and unknown expression/place
  diagnostics at original source spans;
- one `TypedSourceProgram` containing resolved declarations, typed bindings,
  source spans, nested typed statements, and their semantic-pass ownership;
- a common diagnostic adapter for name/type, index, usage, ownership,
  resource-flow, typed-error, error-region, cleanup-lowering, C-emission, and
  future concurrency passes;
- an exact-source printer and lexical-structure round-trip predicate.

The Cycle 10 boundary intentionally does not run the existing semantic engines
as one whole-program fixed point. That connection, recursive summaries, and
cross-function identity transfer are Cycle 11.

## Rust acceptance corpus

The golden source covers all declaration families, generic/index parameters,
contracts and effects, ownership/borrow types, control flow, typed errors,
error-region cleanup, and forward concurrency syntax. Tests also cover exact
printing, lexical round trips, statement classification, type classification,
resolved symbols, pass attribution, malformed delimiters/declarations, and
source-located duplicate/unknown-name failures.

## Lean verification

`proof/QCore/Frontend.lean` defines the reduced formal front-end contract:
source spans, declarations and references, program well-formedness, typed
programs, and elaboration. It proves that successful elaboration:

1. implies the source program is well formed;
2. preserves the complete source program, declaration count, and spans;
3. preserves unique declaration names;
4. resolves every recorded source reference;
5. produces valid source spans;
6. assigns one symbol per declaration with no duplicate symbol; and
7. rejects every program that violates the front-end invariant.

No theorem contains a proof placeholder or project-defined axiom.

## Complete verification instructions

From the repository root:

```sh
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
if rg -n '\b(sorry|admit)\b|^[[:space:]]*axiom\b' proof \
  --glob '*.lean'; then exit 1; fi
cd proof
lake build
lake env lean AuditCycle10.lean
```

The C translation-validation tests continue to use the host C compiler through
the Rust `cc` crate. The unrelated `docs/cmake-3.20.0-linux-x86_64/` workspace
archive is neither required nor used.

## Verified completion result

Cycle 10 completed with 97 passing Rust tests, strict formatting and Clippy,
a placeholder-free Lean build, and a standard-foundations theorem audit.
