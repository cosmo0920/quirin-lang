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

**Status: completed.** See [`cycles/CYCLE_12.md`](cycles/CYCLE_12.md).

Lower checked source programs—not only Rust IR fixtures—to cleanup-explicit CFG
and C11. Add module/file handling, a stable CLI, diagnostics, and emitted-source
mapping. Require source/reference/C differential tests for values, errors,
resource traces, ownership transfers, and quantitative bounds.

## Cycle 13: Translation validation and analysis hardening

**Status: completed.** See [`cycles/CYCLE_13.md`](cycles/CYCLE_13.md).

Add property testing, pass-boundary CFG validation, larger sanitizer corpora,
and a translation-validation checker. Extend legacy-C analysis with a
production C parser, interprocedural summaries, structs/fields, arrays, and
documented widening, preserving `Unknown` for everything unmodeled.

## Cycle 14: Concurrency and resource-race verification

**Status: completed.** See [`cycles/CYCLE_14.md`](cycles/CYCLE_14.md).

Specify threads, shared ownership, atomics, mutex capabilities,
happens-before, lock order, and resource access footprints. Implement race and
deadlock diagnostics for a restricted concurrent core. Prove that accepted
programs have no conflicting unordered resource accesses and that lock
capabilities cannot be duplicated. This is the proving specification for
multi-threaded resource races absent from Cycles 0–9.

## Cycle 15: Advanced control and cleanup

**Status: completed.** See [`cycles/CYCLE_15.md`](cycles/CYCLE_15.md).

Define asynchronous cancellation, fallible destructors and double-error
semantics, higher-order ownership capture, and restrictions for resumable or
multi-shot effects. Implement only fragments with executable tests and
mechanized cleanup/linearity preservation.

## Cycle 16: Full metatheory and research closure

**Status: completed.** See [`cycles/CYCLE_16.md`](cycles/CYCLE_16.md).

Unify the reduced proofs into preservation and progress for the implemented
combined language, resource safety across all implemented ledgers, and
end-to-end correctness for the supported C11 target. Keep principal inference,
general nonzero-net loops, unrestricted legacy C, and any categorical
unification that remains open explicitly labeled as research.

## Cycle 17: Research extensions and production closure

**Status: planned.** See [`cycles/CYCLE_17.md`](cycles/CYCLE_17.md).

Promote the six explicit Cycle 16 research-boundary items into one roadmap
milestone: principal inference, general nonzero-net loops, unrestricted
legacy-C analysis, categorical unification, full C11 weak-memory semantics,
and a native concurrent backend. Execute them as ordered workstreams under one
completion gate. Cycle 17 is complete only when each workstream has executable
validation, corresponding Lean definitions and theorems, and an end-to-end
composition result; a bounded prototype or an `Unknown` fallback does not by
itself close the promoted item.

## Cycle 18: Libevent event-driven concurrency verification

**Status: planned.** See [`cycles/CYCLE_18.md`](cycles/CYCLE_18.md).

Define and verify a Libevent-style reactor profile on top of the Cycle 17
native concurrency and C11 memory model. Model event bases, event generations,
pending and active queues, callback custody, priorities, persistent events,
cancellation/finalization, cross-thread notification, and the supported
bufferevent locking modes. Add an annotated legacy-C importer and a pinned
Libevent adapter, then prove lifecycle safety, callback authorization,
same-base dispatch exclusion, cross-base race freedom, quiescent teardown, and
source/model/runtime trace refinement. Backend-specific tests must cover the
available select, poll, epoll, kqueue, and Windows IOCP profiles without
treating priority, readiness, or wall-clock timing as synchronization evidence.

Later cycles may be split into consecutive numbers if their executable and
proof scope is too large, but none may be skipped without updating this
dependency order and its acceptance criteria.
