# Fluent Bit system coverage: prospective queue and status

> [!NOTE]
> **Current status (partially executed; active production queue).** `P0` (`W45-FB-SYSTEM-SELECTION`), initial `P1.1` backlog custody & fault-injection slices, initial `P1.3` singleplex retry & fault-injection slices, `P2` engine loop census + counted-loop measure (gateway rule 44), and the `W47` tail-pause loop-owner (`10p`) and condvar-join (`10q`) rules have **landed**. Remaining backlog accounting (`P1.2`), broader multi-output backlog/retry composition, and engine slice runs remain open in `acceptance_matrix.json#named_followups`. See [`docs/STATUS.md`](../../STATUS.md) for current `fluent-bit-system` (112 Safe / 332 Unknown / 4 provisional Unsafe at `2c07390b6`) and `fluent-bit-tail-pause` figures, and note that CT corpus mapping is now **50/50** (mapping remains distinct from production safety).

## 1. Queue status overview

| Order | Card / Track | Kind | Current standing & landed evidence | Open follow-ups |
|---|---|---|---|---|
| **P0** | `FB-SYSTEM-CENSUS` | Measurement | **Completed** — [W45-FB-SYSTEM-SELECTION](../../reports/archive/waves-40-47/w45-fb-system-selection/README.md) | `W45-FB-SYSTEM-SLICE-RESIDUALS` |
| **P1.1** | `FB-BACKLOG-CUSTODY` | Semantic & fault-injection | **Initial slices landed** — [W45-FB-BACKLOG-CUSTODY](../../reports/archive/waves-40-47/w45-fb-backlog-custody/README.md), [W45-FB-BACKLOG-FAULT-INJECTION](../../reports/archive/waves-40-47/w45-fb-backlog-fault-injection/README.md) | `W45-FB-BACKLOG-CIO-RETURN-CODES`, `EXIT-WRAPPERS`, `MAP-ARMS`, `PUBLICATION-BLOCKED`, `REACHABLE-RELEASE`, `REFERENCE-VOCABULARY`, `TRANSIENT-STICKY-ERROR`, `FB-AUDIT-03-*` |
| **P1.2** | `FB-BACKLOG-ACCOUNTING` | Semantic implementation | **Open (prospective)** | Per-transition byte/count ledger & pinned admission inequality |
| **P1.3** | `FB-RETRY-LIFETIME` | Semantic & fault-injection | **Singleplex slice landed** — [W45-FB-RETRY-LIFETIME](../../reports/archive/waves-40-47/w45-fb-retry-lifetime/README.md), [W45-FB-RETRY-FAULT-INJECTION](../../reports/archive/waves-40-47/w45-fb-retry-fault-injection/README.md) | `W45-FB-RETRY-UNSPENT-RESCHEDULE`, `FB-RETRY-TIMER-ALLOC-FAULT`, `FB-SINGLEPLEX-QUEUE-ALLOC-FAULT`, `FB-SINGLEPLEX-RETRY-FLUSH-FAULT` |
| **P2** | `FB-ENGINE-PROGRESS` | Measurement + rule 44 | **Census & rule 44 landed** — [W45-FB-ENGINE-PROGRESS](../../reports/archive/waves-40-47/w45-fb-engine-progress/README.md) (121 loops), [W45-FB-ENGINE-PROGRESS-MEASURE](../../reports/archive/waves-40-47/w45-fb-engine-progress-measure/README.md), [list-finiteness rule 48](../../reports/archive/waves-40-47/w45-fb-list-finiteness-premise/README.md) | `W45-FB-ENGINE-PROGRESS-MEASURE-SLICE-RUN`, `W45-FB-ENGINE-REVISITS-UNEXERCISED` |
| **P3.5** | Input pause/resume & engine stop | Concurrency & native observation | **Bindings, rules `10p`/`10q`, & double-stop observation landed** — [tail-pause bindings](../../reports/archive/waves-40-47/w47-tail-pause-bindings/README.md), [loop-owner instance flow](../../reports/archive/waves-40-47/w47-loop-owner-instance-flow/README.md), [condvar-join publication](../../reports/archive/waves-40-47/w47-condvar-join-publication/README.md), [double-stop observation](../../reports/archive/waves-40-47/w47-fb-double-stop-observation/README.md) | Per-unit routes still refuse `10p`/`10q` callees; `W47-TAIL-REQUEST-CONTROL-API` awaits upstream API; event-loop lifetime imports (`FB-EVENT-LOOP-*`) remain open |
| **P3** | `FB-SYSTEM-EXTENSIONS` | Candidate inventory | Shutdown, worker queues, CMetrics/CTraces serialization, crash durability | Select at most one bounded slice per future card |

> [!IMPORTANT]
> **Halted track:** [GUARDED-CONTRACT-ADOPTION-SUPPLEMENT](../../reports/guarded-contract-adoption/SUPPLEMENTAL_CARD.md) remains **explicitly halted at user request** and requires explicit human authorization before restart.

---

## 2. Evidence and transfer boundaries

1. **CT scenarios transfer by property, never by library name.** Ownership, shared chunk generations, error-path cleanup, borrower quiescence, and budget conservation guide Fluent Bit analysis, but neither 50/50 CT mapping nor Libevent's reactor model automatically certifies Monkey's engine backend or asynchronous Fluent Bit tasks/callbacks.
2. **Trusted premises remain explicit.** ChunkIO residency (`chunk_residency.qv`), counter bounds (`counters.qv`), and wrapper busy flags (`busy_state.qv`, distinct from the underlying chunk lock) are declared premises, not proofs of C bodies.
3. **Safety vs. progress and timing.** Bounded memory does not imply progress, bounded callback work does not imply wall-clock latency (CT39), and readiness/priority order never establishes synchronization or fairness (Cycle 18). Provisional `Unsafe` findings and local native fault observations are candidates, never upstream defect claims.

---

## 3. Card specifications for remaining open work

### FB-BACKLOG-CUSTODY (remaining multi-output & wrapper arms)
- **Property**: Distinguish output-queue wrapper identity, per-output delivery obligations, and underlying shared chunk ownership across fan-out insertion, cross-queue removal, handoff, load failure, and teardown (`FB-AUDIT-03-CROSS-OUTPUT-EVICTION`, `PARTIAL-FANOUT`, `W45-FB-BACKLOG-*`).
- **Acceptance**: Audited Lean preservation + production source-bound import; premature close, stale generation, double ownership, and partial-fanout rollback leaks must refuse; unsupported edges remain `Unknown`.

### FB-BACKLOG-ACCOUNTING (`P1.2`, open)
- **Property**: Maintain distinct ledgers for physical resources (resident bytes, mappings, descriptors), per-output delivery charges, and budget credits without unsigned/signed wraparound or silent overshoot.
- **Acceptance**: Derive the exact admission inequality from pinned C (distinguishing strict bounds, batch overshoot, and advisory limits); validate across shared chunks, up/down residency transitions, repeated removals, and partial rollback with Rust + audited Lean + mutation controls.

### FB-RETRY-LIFETIME (remaining scheduler & allocation-fault edges)
- **Property**: Preserve task/chunk obligations across dispatch, pending retry, completion, allocation/flush faults (`FB-RETRY-TIMER-ALLOC-FAULT`, `FB-SINGLEPLEX-*`), and shutdown without assuming cancellation waits for in-flight callbacks.

### FB-ENGINE-PROGRESS (remaining slice run & revisit discharge)
- **Scope**: Execute `W45-FB-ENGINE-PROGRESS-MEASURE-SLICE-RUN` on the system slice and address `W45-FB-ENGINE-REVISITS-UNEXERCISED` while keeping loop-measure facts orthogonal to custody verdicts.

### FB-SYSTEM-EXTENSIONS (candidate inventory)
1. **Cross-plugin shutdown**: Teardown ordering across inputs, outputs, timers, and plugin descriptors (`FB-PLUGIN-DESCRIPTOR-RESIDUAL-READERS`, `FB-TEARDOWN-RELEASE-ACTUAL-IDENTITIES`, `FB-UNJOINED-TEARDOWN-READINGS`).
2. **Worker queues & shared metadata**: Mutex-guarded metadata, stale refinement invalidation, and thread handoff (`CT26`–`CT34`, `CT48`).
3. **CMetrics / CTraces serialization & reset**: Borrowed payload lifetimes and encoder-failure cleanup.
4. **Crash durability & persist/ack/delete protocols**: Requires a separate durability specification beyond memory custody.

---

## 4. Historical note on `FB-PLAN-GATE-HOST-TIMEOUT`

At the original 2026-09-16 review checkpoint, `standalone_host::closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request` timed out on an 8 KiB pipe backoff quantum. That transport defect was **repaired in wave 35** (`VERIFIER-HOST-CONTROL-BUDGET` using `poll(2)`), and the remaining high-load sensitivity is tracked as constraint 16's sixth isolated-verification row (`STANDALONE-HOST-CLOSED-INPUT-LOAD`), run in `run_test_binaries.py`'s `EXCLUSIVE` phase (`FB-PLAN-STALE-RED-ASSERTION` resolved at wave 36 close).
