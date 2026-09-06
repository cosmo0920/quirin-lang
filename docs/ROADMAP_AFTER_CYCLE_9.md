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

**Status: completed.** See [`cycles/CYCLE_17.md`](cycles/CYCLE_17.md).

Promote the six explicit Cycle 16 research-boundary items into one roadmap
milestone: principal inference, general nonzero-net loops, unrestricted
legacy-C analysis, categorical unification, full C11 weak-memory semantics,
and a native concurrent backend. Execute them as ordered workstreams under one
completion gate. Cycle 17 is complete only when each workstream has executable
validation, corresponding Lean definitions and theorems, and an end-to-end
composition result; a bounded prototype or an `Unknown` fallback does not by
itself close the promoted item.

## Cycle 18: Libevent event-driven concurrency verification

**Status: bounded implementation and native platform matrix complete; aggregate acceptance pending.** See [`cycles/CYCLE_18.md`](cycles/CYCLE_18.md).

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

## Cycle 19: Fluent Bit resource and event integration

**Status: in progress.** See [`cycles/CYCLE_19.md`](cycles/CYCLE_19.md).

Build on Cycle 17 custody and Cycle 18 event semantics. The remaining work is
19B source-to-identity mapping for aggregates, lists, queues, refcounts and
arenas; 19C remaining wrapper lifecycle paths, native constructor/caller/ring
bindings and full sync/OS guarantees for the now source-bound nested teardown
chain; and 19D complete wrapper/map custody, remaining wrapper execution bindings,
failure-path binding and full event-program composition. 19E audits their
composition. Recorded callback executions and projected C functions do not
close these static safety gates. Cycle 18 and 19 share the same observation
rather than counting it as independent evidence.

The [nested teardown continuation](reports/cycle19-nested-cleanup/README.md)
checks both traversal loops and parent destructors, composes memory-backed model
identities, and preserves residual filesystem/counter obligations through parent
release. All 400 workspace tests pass with unchanged pinned aggregate verdicts;
these conditional cases do not close the remaining native and event gates.

## Cycle 20: Verifier plugin ecosystem

**Status: active; foundation implemented, semantic dispatch and provider parity
remain open.** See [`cycles/CYCLE_20.md`](cycles/CYCLE_20.md).

Extract the Cycle 19 semantics behind versioned providers. The core owns
obligations, identity, independent evidence checking and composition. Evidence
schema v2 must carry replayable semantic witnesses; protocol-v1 assertions
remain insufficient for Safe or verified Unsafe. Provider extraction follows
Libevent, ChunkIO, CMetrics/CTraces, librdkafka, then Fluent Bit composition,
with archived pinned parity before each existing route is removed. Packaging,
OS isolation, CLI lifecycle and formal composition remain acceptance work.

## Generalization across and beyond Cycles 19–20

The [checker generalization plan](GENERALIZATION_PLAN.md) is the original
2026-09-06 proposal preserved verbatim. Its line counts and architecture review
are a dated snapshot, not a continuously rewritten implementation record. The
workstreams below extend the dependency order without declaring either cycle
complete or assigning speculative cycle numbers. The
[reevaluation gates](REEVALUATION_SPEC.md#9-ordered-implementation-and-acceptance-gates) remain
binding: G1 containment precedes provider expansion; G4 parity precedes deletion.

| Workstream | Dependency and acceptance | Current execution record |
| --- | --- | --- |
| W1: contracts as data | Total-or-reject native contract loader, explicit trusted inputs, pinned verdict/diagnostic parity | [Cycle 19 resource loader](reports/cycle19-contract-loader/README.md): audit table migrated; richer observation/lifecycle predicates remain separate interpreters |
| W2: profiles as data | Versioned source/build/profile manifests; schema review before mass migration; parity before removing tables | Planned; library enums and source/profile tables remain |
| W3: obligations and dispatch | Core-derived inventories, v2 semantic witness design/review and replay, fail-closed source dispatch | Planned; VP05 and semantic-provider acceptance remain open |
| W4: validated custody inference | Depends on W1; SCC/fixed-point parameter and return summaries, independent certificate rederivation, LC01 regressions | [Bounded chunk dependency check](reports/cycle19-chunk-helper/README.md) validates constant-field helper bodies; general SCC/fixed-point custody inference and LC01 remain open |
| W5: configured audit and provider extraction | Depends on W1–W3, uses W4 as available; reusable orchestration, second-codebase demonstration and per-provider parity before deletion | Planned; Cycle 19 audit route retained |
| W6: generic CT corpus | Continuous; source-level CT01–CT50 tests consume W3/W4 results; unmapped and Unknown cases remain visible | Planned; loader regressions do not mark CT cases complete |
| W7: hygiene | Continuous; diagnostic namespaces, generic model names, roadmap and Lean CI | Roadmap updated; diagnostic/model renames and Lean CI remain open |

Continue Cycle 19 using these generic interfaces as they become available;
W2/W3 may develop independently of W1. The next custody work consumes loaded
contracts rather than adding new Rust production tables. Source identity,
synchronization evidence, wrapper failure paths and nested teardown still need
Cycle 19's own checks; moving contracts into files proves none of those facts.
Keep every verdict's artifact, property, profile, scope, bounds, assumptions and
evidence version explicit. Report verdict-class deltas separately from parser
projection, keep Unknown in the denominator, and archive every parity checkpoint
under `docs/reports/`.

Later cycles may be split into consecutive numbers if their executable and
proof scope is too large, but none may be skipped without updating this
dependency order and its acceptance criteria.
