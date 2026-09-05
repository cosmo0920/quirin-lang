# Cycle 8: Executable Formal Model

**Status: Done**

Cycle 8 implements Stage 8 of the prototype roadmap. It integrates the earlier
semantic slices in a deliberately reduced, executable small-step machine shared
conceptually by Rust and Lean. It does not expand the unfinished surface
language or claim full QCore-C-to-C11 translation correctness.

Cycle 7.5 remains postponed. This cycle uses the already established abstract
resource and error semantics and requires no native Windows/MSVC work.

## Phase 0: warm-up prerequisite

Before Cycle 8 implementation begins:

- mark Cycle 7 as done;
- move the Cycle 7 record to `docs/cycles/`;
- update repository documentation to point at Cycle 8 as the active cycle;
- retain `docs/CYCLE_7_5.md` unchanged as a postponed portability plan.

## Executable Rust semantics

`compiler/src/executable_model.rs` provides:

- natural index values represented by a nonnegative integer type;
- erased, exactly-one, affine, and unrestricted usage grades;
- executable usage counting and upper/lower obligation checks;
- globally unique owned resource identities split between the untouched region
  baseline, the live local stack, and the released history;
- acquire and LIFO drop operations with explicit typed failures;
- declared user errors and typed safety failures rather than stuck states;
- one error region that seals its exact live stack into a cleanup pack;
- one-release-per-step cleanup in newest-first order;
- final observations containing either a natural index or a typed error with
  its sealed cleanup evidence;
- construction checks for duplicate baseline identities, usage bindings, and
  error declarations;
- executable tests for normal return, usage failure, partial initialization,
  typed raise, duplicate acquisition, invalid drop, LIFO cleanup, preservation
  of the identity partition, and absence of double release.

The model is intentionally a straight-line reduced core. Branches, borrows,
quantitative resource vectors, and emitted C remain in their already verified
specialized modules rather than being duplicated in the Stage 8 machine.

## Lean verification obligations

`QCore/ExecutableModel.lean` defines the corresponding natural indices, usage
grades, operations, typed errors, cleanup packs, configurations, executable
one-step function, and reflexive-transitive small-step relation. It proves:

1. progress: every well-formed state is final with a natural result or typed
   error, or it takes an executable step;
2. preservation: each step preserves the duplicate-free identity partition;
3. multi-step preservation by induction over the small-step relation;
4. no duplication of live resource identities;
5. no double release in the released-identity history;
6. the region cleanup theorem: cleanup restores the recorded baseline, empties
   the local stack, emits every packed release in order, and terminates with the
   same typed error and cleanup pack.

No theorem contains a proof placeholder or project-defined axiom.

## Complete verification instructions

From the repository root:

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
if rg -n '\b(sorry|admit)\b|^[[:space:]]*axiom\b' proof \
  --glob '*.lean'; then exit 1; fi
cd proof
lake build
lake env lean AuditCycle8.lean
```

The Rust suite continues to compile and execute the Cycle 7 emitted-C corpus on
non-MSVC hosts with the existing strict warnings and sanitizers. Native MSVC is
still excluded exactly as documented in `docs/CYCLE_7_5.md`.

## Verified completion result

Completion is verified by 77 passing Rust tests, including the eight reduced-core
model cases and four compiled-C translation-validation cases, warning-free
strict Clippy, a successful Lean build, a source audit with no proof
placeholders or project axioms, and a Cycle 8 theorem-dependency audit limited
to Lean's standard foundational dependencies.
