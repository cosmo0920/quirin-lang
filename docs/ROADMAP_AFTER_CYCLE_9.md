# Roadmap after Cycle 9

Cycle 9 completes the original numbered prototype roadmap, but not the full
language described by the normative specification. Each future cycle requires
executable Rust tests, matching Lean definitions and safety theorems, strict
Clippy, a placeholder-free Lean build, and a theorem-dependency audit.

Cycle 7.5 was completed separately as a native Windows/MSVC portability
validation effort. It is not folded into the milestones below.

## Cycle 10: Complete source front end

**Status: completed.** See [`cycles/CYCLE_10.md`](cycles/CYCLE_10.md).

Implement the remaining function, statement, ownership, borrow, effect,
resource-flow, error-region, and cleanup syntax; name resolution; source spans;
and elaboration into one typed source IR. Require grammar golden/round-trip
tests and source diagnostics for every existing semantic pass.

## Cycle 11: Whole-program checking and summaries

**Status: completed.** See [`cycles/CYCLE_11.md`](cycles/CYCLE_11.md).

Connect usage, ownership, borrows, resource vectors, typed errors, regions, and
function summaries in one pipeline. Add call-graph fixed points,
recursive-summary rules, and cross-function identity transfer. Prove
pass-interface invariants and acceptance preservation into checked CFG.

## Cycle 12: End-to-end QCore-C compiler and CLI

**Status: completed.** See [`../CYCLE_12.md`](../CYCLE_12.md).

Lower checked source programs—not only Rust IR fixtures—to cleanup-explicit CFG
and C11. Add module/file handling, a stable CLI, diagnostics, and emitted-source
mapping. Require source/reference/C differential tests for values, errors,
resource traces, ownership transfers, and quantitative bounds.

## Cycle 13: Translation validation and analysis hardening

**Status: completed.** See [`../CYCLE_13.md`](../CYCLE_13.md).

Add property testing, pass-boundary CFG validation, larger sanitizer corpora,
and a translation-validation checker. Extend legacy-C analysis with a
production C parser, interprocedural summaries, structs/fields, arrays, and
documented widening, preserving `Unknown` for everything unmodeled.

## Cycle 14: Concurrency and resource-race verification

Specify threads, shared ownership, atomics, mutex capabilities,
happens-before, lock order, and resource access footprints. Implement race and
deadlock diagnostics for a restricted concurrent core. Prove that accepted
programs have no conflicting unordered resource accesses and that lock
capabilities cannot be duplicated. This is the proving specification for
multi-threaded resource races absent from Cycles 0–9.

## Cycle 15: Advanced control and cleanup

Define asynchronous cancellation, fallible destructors and double-error
semantics, higher-order ownership capture, and restrictions for resumable or
multi-shot effects. Implement only fragments with executable tests and
mechanized cleanup/linearity preservation.

## Cycle 16: Full metatheory and research closure

Unify the reduced proofs into preservation and progress for the implemented
combined language, resource safety across all implemented ledgers, and
end-to-end correctness for the supported C11 target. Keep principal inference,
general nonzero-net loops, unrestricted legacy C, and any categorical
unification that remains open explicitly labeled as research.

Later cycles may be split into consecutive numbers if their executable and
proof scope is too large, but none may be skipped without updating this
dependency order and its acceptance criteria.
