# Roadmap after Cycle 9

Cycle 9 completed the original sequential prototype roadmap, not full conformance to the normative Quirin specification. Every subsequent cycle and workstream requires executable Rust tests, matching placeholder-free Lean definitions and safety theorems (`proof/Audit*.lean`), strict Clippy, and explicit verdict/assumption accounting. For current measured figures across all evaluation profiles, see [`docs/STATUS.md`](STATUS.md); for active engineering queues, see [`docs/plans/README.md`](plans/README.md).

*(Cycle 7.5 was completed separately as a native Windows/MSVC portability validation effort.)*

---

## Completed bounded milestones (`Cycles 10–17`)

Each cycle below is complete **only at its bounded milestone scope** (see [`docs/development/cycle-status.md`](development/cycle-status.md)), not as a declaration of whole-language conformance.

### Cycle 10: Source front end
**Status: completed at bounded milestone.** See [`cycles/CYCLE_10.md`](cycles/CYCLE_10.md).
Structured declarations, spans, name resolution, quantitative index types, and source diagnostics across semantic passes.

### Cycle 11: Whole-program checking and summaries
**Status: completed at bounded milestone.** See [`cycles/CYCLE_11.md`](cycles/CYCLE_11.md).
Connected usage, ownership, borrows, resource-flow vectors, typed errors, error regions, and function summaries into a unified checking pipeline.

### Cycle 12: End-to-end compiler and CLI
**Status: completed at bounded milestone.** See [`cycles/CYCLE_12.md`](cycles/CYCLE_12.md).
Lowered checked source subsets to cleanup-explicit CFG and C11 behind the `quirin` CLI (`check`, `emit`), backed by source/reference/C differential tests.

### Cycle 13: Translation validation and existing-C gateway foundation
**Status: completed at bounded milestone.** See [`cycles/CYCLE_13.md`](cycles/CYCLE_13.md).
Pass-boundary CFG validation, translation certificates, and initial conservative Clang-gated C resource projection preserving `Unknown` for unmodeled constructs.

### Cycle 14: Concurrency and resource-race verification
**Status: completed at bounded milestone.** See [`cycles/CYCLE_14.md`](cycles/CYCLE_14.md).
Identity-indexed access shares, happens-before vector clocks, lock-order and mutex capabilities, and quiescence-checked destruction (`model.concurrent14`).

### Cycle 15: Advanced control and fallible cleanup
**Status: completed at bounded milestone.** See [`cycles/CYCLE_15.md`](cycles/CYCLE_15.md).
Explicit cancellation safe points, fallible destructor policies, primary/secondary error preservation, and one-shot vs. multi-shot handler linearity restrictions.

### Cycle 16: Combined metatheory and research boundaries
**Status: completed at bounded milestone.** See [`cycles/CYCLE_16.md`](cycles/CYCLE_16.md).
Unified preservation, progress, and ledger safety across the implemented reduced calculi while explicitly labeling open research boundaries.

### Cycle 17: Research extensions
**Status: completed at bounded milestone.** See [`cycles/CYCLE_17.md`](cycles/CYCLE_17.md).
Bounded execution across principal inference, loop/summary extensions, legacy-C gateway scaling, categorical alignment, C11 weak-memory candidate checking, and native concurrent thread lowering.

---

## Active production & ecosystem cycles (`Cycles 18–20`)

### Cycle 18: Libevent event-driven concurrency verification
**Status: bounded implementation and Linux native observations complete; aggregate acceptance and foreign-host observations pending.** See [`cycles/CYCLE_18.md`](cycles/CYCLE_18.md).

Models event bases, event generations, pending/active queues, callback custody, priorities, persistent events, cross-thread notification, and bufferevent locking without treating readiness, priority, or wall-clock timing as synchronization evidence.
- **Open follow-ups** (`acceptance_matrix.json#named_followups`): `CYCLE18-CALLBACK-TASK-HANDOFF`, `CYCLE18-NATIVE-PLATFORM-MATRIX` (4 Linux `Observed` / 3 foreign-host `Unknown`), and `CYCLE18-AGGREGATE-POLICY-OBLIGATION-SYNC`.

### Cycle 19: Fluent Bit resource and event integration
**Status: active / in progress (raw units remain unpromoted).** See [`cycles/CYCLE_19.md`](cycles/CYCLE_19.md), [`CYCLE19_BUILD_INPUTS.md`](CYCLE19_BUILD_INPUTS.md), and the [Fluent Bit system coverage queue](plans/fluent-bit-system-coverage/README.md).

Integrates conservative C custody, bounds (`c.bounds.v1`), null-dereference (`c.null.deref.v1`), descriptor/stream/directory tracking, callback contracts (`C04.19`), and ChunkIO/Monkey/Libevent/CMetrics/CTraces/librdkafka models on pinned Fluent Bit translation units.
- **Bounded native & symbolic evidence archived**: nested memory teardown ([report](reports/cycle19-nested-cleanup/README.md), [native](reports/cycle19-native-teardown/README.md)), constructor failure matrices ([memory](reports/cycle19-constructor-failures/README.md), [unsupported close/constructors](reports/cycle19-unsupported-close/README.md), [file constructors](reports/cycle19-file-constructor-failures/README.md), [OS constructors](reports/cycle19-file-os-constructors/README.md)), and Linux filesystem/residency/sync/unmap/descriptor observations ([file OS](reports/cycle19-file-os/README.md), [file tree](reports/cycle19-file-tree/README.md), [existing files](reports/cycle19-file-existing/README.md), [transitions](reports/cycle19-file-transitions/README.md), [sync failures](reports/cycle19-file-sync-failures/README.md), [unmap failures](reports/cycle19-file-unmap-failures/README.md), [descriptor failures](reports/cycle19-file-descriptor-failures/README.md), [W43 descriptor native check](reports/w43-descriptor-native-check/README.md)).
- **Remaining Cycle 19 gates**: complete chunk lifecycle composition (`0 Safe / 96 Unknown` in `w43-chunk-source-profile`), production Monkey callback composition, multi-output backlog accounting/custody, and full event-program composition.

### Cycle 20: Verifier plugin ecosystem
**Status: active; host firewall, reproducible provider packages, and v2 bounded replay implemented; full provider semantic parity and legacy-route retirement remain open.** See [`cycles/CYCLE_20.md`](cycles/CYCLE_20.md).

Core owns obligation derivation, source/build identity, the certificate firewall, and `Safe < Pending < Unknown < Unsafe` composition (`⊕`); providers own library-specific contracts and candidate witnesses. Protocol v1 `Safe` proposals are permanently refused, unchecked `Unsafe` allegations downgrade to `Unknown` (`QP010`), and uncertified user/subcommand plugin claims yield `Pending` at best (`C11.1`–`C11.2`). Under [`REEVALUATION_SPEC.md`](REEVALUATION_SPEC.md) §9, G1 containment precedes provider expansion and G4 archived parity precedes legacy Cycle-19 route deletion.

---

## Generalization across and beyond Cycles 19–20 (`W0`–`W7`)

The [checker generalization plan](GENERALIZATION_PLAN.md) and [pluggable-cards plan](plans/pluggable-cards/README.md) govern crate layering and decoupled card execution:

| Workstream | Scope & acceptance | Current standing |
|---|---|---|
| **W0: Workspace layering** | Extract `qcore-schemas` (L0) & `qcorec` (L3); downward-only dependency guard | **Completed** — [w0-schema-crate](reports/w0-schema-crate/README.md), enforced by `workspace_layering.rs` |
| **W1: Contracts as data** | Total-or-reject `.qv` loader, `#include` inheritance, lock files & supersession | **Completed** — `crates/qcore-contract`, 11 locked provider crates (`154` pinned modules) + user plugin crates |
| **W2: Profiles & build transport** | Versioned source-set & evaluation profiles; unmodified CMake/Clang transport | **Completed for bounded profiles** — `contract/profiles/`, `contract/evaluations/`, `quirin-audit prepare clang-build` |
| **W3: Obligations & dispatch** | Core obligation inventories, `qcore-verifier-host`, v2 witness replay | **Bounded dispatch active** — [w3-evidence-v2](reports/w3-evidence-v2/README.md), [w3-verifier-host](reports/w3-verifier-host/README.md); lifecycle source effects open |
| **W4: Validated custody inference** | SCC-ordered parameter/return summary inference & certificate rederivation | **Implemented & extended** — `qcore-c-gateway` (`Out`, `Stored`, `Removed`, `Reallocated`, recursion, `CG01`–`CG30` rules) |
| **W5: Configured audit & providers** | `quirin-audit`, `quirin-source-analysis`, reproducible `providers/*` packages, gateway rule crates | **Implemented** — `crates/qcore-audit`, `crates/gateway-rules/*`; G4 route-deletion parity open |
| **W6: Generic CT01–CT50 corpus** | Source-level conformance harness keyed to `ct_manifest.json` | **50/50 mapped** (48 distinct fixtures; 5 case-bound Lean proofs; 10 gated sanitized-native rows; mapping ≠ conformance) |
| **W7: Hygiene & governance** | Lean 4.35 CI gate, diagnostic namespaces, routed `docs/agents/` & `STATUS.md` | **Active** — `512+` Lean audits (`check_lean_audits.py`), `task_context.py`, `gate_scope.py`, `land.py` |

For wave-by-wave chronological history beyond the early 2026-09 integration checkpoints ([`reviews/r0_3/REVIEW.md`](reviews/r0_3/REVIEW.md)), see the [increment history index](development/increments/README.md).
