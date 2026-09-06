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
Five [native memory teardown executions](reports/cycle19-native-teardown/README.md)
now bind real constructor allocations and reciprocal ring snapshots to exact
release order under sanitizers. General caller and failure-path provenance and
the filesystem/event gates remain open. A
[controlled constructor failure matrix](reports/cycle19-constructor-failures/README.md)
adds nine failures and one control with complete observed rollback/teardown
accounting; returned null fields and error-output conventions remain explicit.
The [unsupported close classification](reports/cycle19-unsupported-close/README.md)
checks the remaining backend discriminant class symbolically. It retains an
explicit `Unknown` after outer chunk release because backend custody and native
constructor reachability are unresolved.
The [parent composition](reports/cycle19-nested-unsupported/README.md) now
preserves exact opaque stream/chunk/backend-kind identities through parent free,
alongside typed filesystem and up-counter residuals.
[Four native unsupported-type constructor observations](reports/cycle19-unsupported-constructors/README.md)
record null chunk returns, unchanged errors and complete rollback before empty
stream teardown. They narrow the observed caller boundary without making the
conditional unsupported-close path a reachable violation.
[Real Linux cleanup observations](reports/cycle19-file-os/README.md) bind twelve
helper calls to descriptor/mapping probes and pathname inode witnesses. This
adds actual OS outcomes to the controlled observations; enclosing filesystem
teardown, full sync/durability and general caller guarantees remain open.
[Six native filesystem tree observations](reports/cycle19-file-tree/README.md)
now bind one-stream/one-file constructors and parent teardown, including exact
OS residual identities after injected unmap/close failures and cleanup after
sync failure. These bounded cases account for 66 allocation generations without
closing general filesystem or static custody gates.

[Twelve filesystem allocation-failure/control processes](reports/cycle19-file-constructor-failures/README.md)
add 81 acquired generations, 73 recorded releases and eight outstanding prefix
generations. Two null-argument UBSan terminations preserve interrupted execution;
only ten returning cases complete teardown and leak checking. Next constructor
work includes other options, OS failures and multiple-failure combinations;
source-wide caller/custody and wrapper/event gates remain open.

[Eight constructor OS-failure processes](reports/cycle19-file-os-constructors/README.md)
now bind size-lookup fallback, POSIX resize errors, mapping failures and rollback
close to original OS identities. All 88 allocation generations are released;
failed rollback close retains a descriptor after null chunk return and parent
release. Next bounded paths include constructor ACL/existing-file cases and
residency/transaction transitions. General caller, OS and aggregate custody
obligations remain open.

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
| W0: workspace layering | Schema surface + CLI extracted with unchanged behavior; downward-only dependency guard | [W0 schema-crate record](reports/w0-schema-crate/README.md): `qcore-schemas` and `qcorec` extracted 2026-09-06; providers stay compile-time crates by recorded decision |
| W1: contracts as data | Total-or-reject native contract loader, explicit trusted inputs, pinned verdict/diagnostic parity | [Cycle 19 resource loader](reports/cycle19-contract-loader/README.md): audit table migrated; [W1 relocation](reports/w1-contract-crate/README.md): loader/observation extracted to `crates/qcore-contract` over new `qcore-core`/`qcore-frontend` crates (2026-09-06); [W1 literal sites](reports/w1-literal-sites/README.md): leftover test/example literals migrated to loader-fed `.qcore` fixtures (2026-09-06); [W1 clause vocabulary](reports/w1-clause-vocabulary/README.md): interpreter boundary documented and pinned corpus-wide, ABI scalar table completed (2026-09-06) — **executed**; interpreter unification is W5-adjacent and error sentinels need target-interface work |
| W2: profiles as data | Versioned source/build/profile manifests; schema review before mass migration; parity before removing tables | Bounded migration executed: schema/loader, library/source/mechanism/operation and Libevent tables; full 185-TU parity archived in [W2 report](reports/w2-profile-manifests/README.md). H6 first-party registration also loads data ([record](reports/w2-provider-discovery/README.md)); typed imports and Boolean routing also load data (bounded reports below); [configured orchestration](reports/w5-configured-audit/README.md) also extracted; general semantic source binding and provider consolidation remain |
| W3: obligations and dispatch | Core-derived inventories, v2 semantic witness design/review and replay, fail-closed source dispatch | [Bounded W3 replay/dispatch](reports/w3-evidence-v2/README.md): source scopes dispatch, observed-agreement v2 replay and schemas-only process conformance run; [host extraction](reports/w3-verifier-host/README.md) moves registry/process/firewall/composition to qcore-verifier-host; lifecycle source effects, production profile import and trusted-kernel consolidation remain open; no first-party promotion |
| W4: validated custody inference | Depends on W1; SCC/fixed-point parameter and return summaries, independent certificate rederivation, LC01 regressions | Executed for parameter/return custody with validation ([record](reports/w4-custody-inference/README.md)): SCC-ordered inference validated by re-analysis, certificate re-derivation, LC01 regression suite, Lean relation. [Bounded extension](reports/w4-custody-extensions/README.md) adds direct `Out`, `Stored`, `Reallocated` and nullable returns; [Gateway relocation](reports/w4-gateway-crate/README.md) executed with exact 185-unit parity; [Direct Removed wrappers](reports/w4-removed-custody/README.md) now validate contained entry/detached exit; [Direct Out forwarding](reports/w4-out-forwarding/README.md) adds original-slot validation and conservative mutation/shadow/alias guards; [Indirect-access exclusion](reports/w4-indirect-access/README.md) repairs optimistic erased reads; typed access evidence, general removal/out-slot aliases and recursive components remain open |
| W5: configured audit and provider extraction | Depends on W1–W3, uses W4 as available; reusable orchestration, second-codebase demonstration and per-provider parity before deletion | [Configured resource audit](reports/w5-configured-audit/README.md) extracted into provider-free L3 with a data-only second-codebase demonstration and exact 185-TU parity; [Kernel/contract consolidation](reports/w5-chunk-summary-boundary/README.md), [frontend relocation](reports/w5-whole-program-frontend/README.md) and [reproducible Unknown-only provider executables](reports/w5-provider-packages/README.md) are implemented; specialized semantic extraction, registry-based corpus composition, and route deletion remain; [explicit CLI lifecycle](reports/w5-cli-packages/README.md) and [generic reactor split](reports/w5-generic-reactor/README.md) are implemented; callback wire schema remains open |
| W6: generic CT corpus | Continuous; source-level CT01–CT50 tests consume W3/W4 results; unmapped and Unknown cases remain visible | [W6 harness record](reports/w6-ct-harness/README.md): manifest-keyed harness executes mapped cases through the real source pipeline and fails closed on manifest/fixture disagreement; CT02/CT12/CT16 mapped as sequential-core fixtures (2026-09-06). [Batch 2](reports/w6-ct-batch-2/README.md): CT19 (error edge with an outstanding out-of-frame obligation) and CT37 (allocation retained in the completion record) mapped, plus the full triage of all 47 then-unmapped cases — 2 decidable-now, 8 blocked on W3/W4, 33 blocked on unimplemented syntax, 4 ambiguous (2026-09-06); 45 cases remain unmapped, counted by a pinned test. [Frontend gates](reports/w6-frontend-gates/README.md): unchecked source clauses reject with Q021 and guard/scope binders resolve body-locally; semantic contract enforcement and concurrency lowering remain open |
| W7: hygiene | Continuous; diagnostic namespaces, generic model names, roadmap and Lean CI | Roadmap updated; [Lean CI hygiene](reports/w7-lean-ci/README.md) implemented and locally checked (hosted run pending push); [generic model names](reports/w7-generic-lean-names/README.md) implemented with compatibility aliases; Libevent L00–L12 provider-local diagnostics are migrated with W3 |

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

## Existing-file profile observations (2026-09-06)

[Seven native existing-file profiles](reports/cycle19-file-existing/README.md)
release all 70 allocation generations: seven temporary, 20 rollback and 43
parent-teardown releases. Original inode/bytes, read-only mapping permissions,
malformed-magic/short-header/empty-read-only errors and a descriptor residual
after suppressed chmod plus rollback-close failures stay distinct. A 17-clause
policy, strict trace mutations and four Lean theorems check this bounded evidence.
General profiles, residency/transaction paths, caller provenance, full OS and
sync guarantees, aggregate source custody, wrapper lifecycle and full event
composition remain open. No raw unit is promoted.

## Native residency and transaction observations (2026-09-06)

[Five native sequences](reports/cycle19-file-transitions/README.md) release
all 50 allocation generations and seven descriptor/mapping epochs. The adapter
checks 26 operation results, ring/counter transitions and separate logical/header
lengths. Rollback restores logical length without restoring the mapped header;
down/up reloads appended bytes. Locked begin returns retry while direct write
succeeds. A 16-clause runtime policy and six executable Lean projection theorems
preserve those outcomes. Failed transition syscalls, capacity/metadata/checksum
variants, arbitrary production callers, full sync/durability, aggregate custody,
remaining wrapper lifecycle and full event composition remain open. No raw unit
is promoted.

## Native synchronization failure and retries (2026-09-06)

[Three native one-shot sync-failure sequences](reports/cycle19-file-sync-failures/README.md) release all 30 allocation
generations and four OS epochs. Failed commit retains the active transaction;
retry clears it. Down still unmaps/closes after failed sync, and commit while
unmapped clears the transaction without syncing. An 18-clause policy, 73 trace/
process mutations and five Lean theorems preserve these bounded observations.
Other transition failures, general callers, full sync/durability, aggregate
custody, wrapper lifecycle and full event composition remain open. No raw unit
is promoted.

## Failed native unmap and down recovery (2026-09-06)

[Four native failed-down sequences](reports/cycle19-file-unmap-failures/README.md) release all 40 allocation generations and
seven OS epochs. Suppressed unmap retains the original mapping, descriptor,
up ring and counter. A successful preceding sync remains visible after failure;
combined sync/unmap failure remains unsynced. Retry or parent teardown later
releases the original epoch. A 19-clause policy, 75 mutations and five Lean
theorems preserve this boundary. Failed close/reopen, other sync failures,
general callers, full sync/durability, aggregate custody, wrapper lifecycle and
full event composition remain open. No raw unit is promoted.

## Native down/reopen descriptor failures (2026-09-06)

[Four native sequences](reports/cycle19-file-descriptor-failures/README.md)
release all 40 allocation generations and acquire five descriptor/mapping epochs.
Failed close after unmap moves the chunk into the down ring with a zero up
counter but retains its original descriptor. Down/up retries reject without
releasing it. Parent teardown either releases it or, with a second suppressed
close, leaves an observed descriptor residual after enclosing allocations are
freed. Harness reclamation remains separate. Suppressed reopen creates no new
epoch; successful retry acquires one. The independent adapter, 20-clause runtime
policy and seven Lean theorems preserve this conditional observation boundary.
General reopen size/map failures, production caller provenance, aggregate custody,
remaining wrapper lifecycle and full event composition remain open. No raw unit
is promoted.

Boolean field profile routing landed (2026-09-07,
`docs/reports/w2-boolean-field-profiles/README.md`): typed C record/field and
Boolean operation routes come from loaded profile data, with exact declared
assumptions on summaries. An unrelated record fixture exercises both supported
models. Full 185-TU resource diagnostics and source projections are unchanged;
chunk reports add only three assumption maps. Frame-helper ABI generalization
and source-to-model identity premises remain open.


Monkey typed import profiles landed (2026-09-07,
`docs/reports/w2-monkey-typed-profile/README.md`): function roles, records,
sizeof bounds, exact callback signatures, callee bounds and callback effects
load from profile data. Malformed or unavailable declarations reject; wrappers
retain their existing opaque-call grammar. Full 185-TU diagnostics are unchanged;
18 syntax summaries now retain explicit declared profile assumptions. These are
syntax projections and trusted declarations, not proofs of source custody or
whole-program safety. Configured orchestration and general source/model binding
remain open.

## Revision 0.3 review integration (2026-09-07)

The [v0.3 specification](QCORE_SPEC_r0_3.md) and [acceptance ledger](reviews/r0_3/REVIEW.md)
now govern current claims. R01/R02 lifetime state, R05 declaration validation,
R06 exact/maximum custody and R07 literal bounds are repaired at their recorded
boundaries. R04 checks ordinary i32/unit results while retaining an explicit
separate custody payload; owned-result/per-exit typing remains open. R03 records
mutex/handoff premises instead of claiming a checked source invariant. R08/R10
replay rejects scope/module/bounds substitution and retains declared native trust.
These repairs take priority over broader safety promotion. Full kernel/provider
consolidation, richer source lowering, W4 general removal/recursion/cost and 45 unmapped
CT cases remain open. Cycle19 is not scope-limited and raw units stay unpromoted.

Use [persistent build inputs](CYCLE19_BUILD_INPUTS.md) for new audits. Superseded
language documents are in [obsolete](obsolete/README.md); dated evidence remains
in its original report packages.
