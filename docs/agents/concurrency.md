# Concurrency rules

Current rule text, routed by the [rule router](README.md), which says what to
read. Rule numbers and anchors are stable; backticked paths are repository-relative.

<a id="constraint-15n"></a>

<!-- preserved:constraint-15n:start -->
15n. **The F02 trace boundary refuses a range write lease, and refusing is the
    measured answer — do not turn it into a drop.** `reference_trace` crosses
    from the reference run's observation alphabet
    (`qcore_core::task_execution::TaskEvent`) into the F02 observable alphabet
    (`ObservedEvent`). Card `CT03-PROOF-ALPHABET` grew the observable side by
    the T05 Buffer write and proved that conservative; card `WRITE-LEND-TASKS`
    added two range-write-lease constructors to the observation side, and they
    stay **outside** the observable alphabet. `TRACE-ALPHABET-LEASE-BOUNDARY`
    (wave 32) discharged the residual that recorded the arm as an unproven,
    unexercised integration-time resolution. Three facts must survive
    refactors. **First, the arm is reachable today** — card
    `WRITE-LEND-TASKS`' own admitted flagship produces a reference run whose
    third event is a `BufferLeased`, so this is a live path and not a
    defensive one; `compiler/tests/trace_alphabet_lease_boundary.rs` drives it,
    and **three of its five tests** were shown non-vacuous by mutating the arm —
    not five, as this constraint claimed until wave 34
    (`LEASE-BOUNDARY-MUTATION-COUNT-OVERSTATED`; the card's own report published
    the three counts honestly and the ledger's paraphrase inflated them). The
    measurement, re-derived against `crates/qcore-core/src/task_trace_relation.rs`:
    silent-drop 3/5, admitting mis-encode 3/5, wrong index 2/5, union of
    arm-sensitive tests **3**. The other two exercise **neighbouring routes by
    construction** and no arm mutation could falsify them:
    `dropping_the_lease_events_would_have_produced_an_admitted_order` builds the
    dropped encoding itself and calls `reference_trace` on an already-lease-free
    observation, and `the_rendered_lease_lines_are_outside_the_parsers_alphabet_too`
    exercises `parse_observed_trace`. The discharge claim itself is **not**
    weakened by this correction: the arm is genuinely driven.
    **Second, dropping is an *admitting* mistake, not a lossy one**: the
    flagship's lease-free remainder is a trace `AdmittedCausalOrder::declare`
    **accepts**, as a causal order for a run in which two workers held
    outstanding write authority the order never mentions — measured, and proved
    as `QCore.TaskTraceLeaseBoundary.the_dropped_encoding_would_have_been_admitted`.
    A `filter` in place of the `return Err` therefore moves in the direction
    invariant 1 forbids. **Third, the index a refusal carries is the offending
    event's own position and the first such** — proved
    (`refusal_points_at_a_refused_event`, `refusal_is_the_first`), so the Rust
    pin `index: 2` is an instance rather than a coincidence; and on success the
    encoding is positionwise, which is the formal content of "never silently
    dropped" (`encoding_is_positionwise`, `encoding_preserves_length`). The two
    other construction routes into `AdmittedCausalOrder` cannot carry a lease
    at all: `parse_observed_trace` has no tag for the rendered lease lines, and
    `declare` takes `Vec<ObservedEvent>`, which has no lease constructor. **Amended, card
    `WRITE-LEND-TASKS-EXERCISE` (wave 36), which DECIDES the forward question
    `TRACE-ALPHABET-LEASE-GROWTH-UNDECIDED` rather than leaving it open.** That
    residual asked whether the observable alphabet should grow to carry leases
    once a worker exercised its authority. A worker now does, and **the answer
    is no — the alphabet does not grow**, on a ground the exercise itself
    supplies rather than on the absence of one: `AdmittedCausalOrder` is a
    **sequence**, so admitting a run means stating one causal order over its
    events, and two workers holding disjoint write leases write their bytes
    with no causal edge between them and no synchronization until their joins.
    Any order the alphabet could carry would **assert an ordering the run does
    not have** — which is the situation constraint 15a answers with a
    window-and-emitter relation, and is the opposite of F02's premise. The two
    byte events therefore join the lease events on the **refusing** side, so
    the alphabet did not grow and no existing digest moved. Reachability is
    deliberately *not* the ground, and that is measured: in a real exercised
    program the lease event always precedes the byte write, so growing the
    alphabet by the byte events alone would have decided nothing — both facts
    are driven, the refusal on a synthetic observation carrying no lease event
    so the arm is exercised rather than shadowed, and the shadowing itself on a
    compiled program. The operative consequence is unchanged and is now a
    decision rather than a pending question: write-lend programs sit outside
    F02 relational comparison **by construction**, their native evidence being
    sanitizer, allocation-count and the byte-identical stdout of a strict
    comparison whose windows are the joins. Re-opening this needs CT03's
    method — a conservativity theorem plus a rule shown non-vacuous in both
    directions — **and** a causal-order semantics for concurrent byte writes,
    which is the half the sequence shape denies. **Appended, card
    `CT07-TWO-THREAD-PROOF-LEG` (wave 39; it amends nothing above).** The
    refusal is now also what a proof leg rests on: CT07's `proof` stage is
    bound **through the scoped-task ledger's lease set** —
    `QCore.Ct07LeaseLedger.replay_reaches` turns the reference execution's
    lease opens and closes into a `Reachable` witness for wave 31's
    `no_byte_has_two_write_authorities` — precisely because the trace route is
    closed, and the leg's last assertion **pins** that closure
    (`AdmittedCausalOrder::derive` refuses CT07's own observation;
    `CT07-PROOF-LEG-F02-PREMISE-PIN`). A card that re-opens this boundary by
    CT03's method therefore removes that one assertion **deliberately in the
    same change**, states whether CT07's proof stage stays on the ledger
    binding or is additionally derived through the trace, and records the
    choice here — the leg must not be left to go red as a side effect, and
    the ledger binding must not be dropped on the ground that a trace route
    now exists: the two bind different hypotheses (a state the ledger admits;
    an order the alphabet admits), and neither implies the other.
    `trace-alphabet-lease-boundary`, `ct03-proof-alphabet`, `write-lend-tasks`,
    `write-lend-tasks-exercise`, `ct07-two-thread-proof-leg`.
<!-- preserved:constraint-15n:end -->

<a id="constraint-15o"></a>

<!-- preserved:constraint-15o:start -->
15o. **(card `MUTEX-SHARE-CROSS`.)** A `protected N` payload crosses a task
    boundary only through `thread scope NAME share PLACE under MUTEX { … }` on
    `source.mutex.guard.v1`, and the decision is in **three** halves that must
    not be merged: classification is `qcore_core::transfer`'s (the fifth row;
    each sharing mode requests **one** protocol and is admitted exactly when
    the kind's discipline is that protocol — do not restore the arm that took
    whatever protocol the kind named); the **absent** and **mismatched**
    protocol gates are the crossing's, because the judgment cannot see a
    generation's declared footprint; and **unverified** is the *unedited*
    `CheckedMutexInvariantV1` checker's own `Q611`. Do not re-implement a
    guard rule beside the judgment. The clause is **opt-in**: a child without
    it lowers byte-identically, and the load-bearing reach rule
    (`Frame::admits_reach`) applies only to a declaring child and only in the
    refusing direction. `MUTEX_SHARING_GAP` stays `false` — the *task* route's Buffer still has
    no mutex. A checked mutex trace is rendered when the backend has a
    rendering for its task structure and published without C when it does
    not. Since card `MUTEX-SHARE-CROSS-EMISSION` (wave 36) there are **two**
    rendered shapes, decided by `mutex_c_emit::classify_trace`: the single-
    task trace, byte for byte what it always emitted, and a **non-nested
    fork/join** trace whose children's events are *contiguous* between their
    `Spawn` and `Join` — which is what the declared crossing lowers to —
    rendered with real `thrd_t` children over the existing `mtx_t` emission,
    the generations and protected allocations moved to file scope so a
    child's own function can name them. Everything else stays outside and is
    published **without** C, never with C the backend did not produce: a
    guard-local join, a nested spawn, a child's events interleaved with its
    parent's, a custody transfer, a registered waiter, and a typed-error
    exit inside a child, whose exit status the model gives no meaning at the
    join. Three things must survive refactors. The **contiguity** premise is
    what makes "create the thread, then block in `thrd_join`" a realisation
    of the *checked* order rather than one interleaving of it — do not widen
    the admitted shape without re-deriving it. The **creator performs no
    trace event while its child runs**, proved as `QCore.MutexShareCrossEmis
    sion.the_creator_performs_no_event_while_its_child_runs`, so this
    backend witnesses a real thread boundary and **nothing about contention,
    a data race or the C memory model**; no such claim may be made from it
    (`MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION`). **The ThreadSanitizer rows
    that exist since waves 37 and 39 do not change that, and measuring why
    sharpens it**: `TSAN-ROWS-2` runs CT46's own case as a subject and it is
    clean, but that case is **serial by construction**, so the forcing header
    attaches to nothing, no worker is released and the clean row witnesses
    nothing about contention — which is the theorem above, observed. A clean
    detector is in any event evidence about the schedules realised, never a
    proof of race freedom, and the rows are Linux only
    (`docs/reports/{task-route-tsan,tsan-rows-2}/README.md`). And the
    **operation/log adjacency** is
    load-bearing evidence, not formatting: the native comparison compares
    the *logged* sequence, so deleting a real `mtx_lock` while keeping its
    log line was measured byte-identical and clean under the sanitizers —
    one renderer writes both, adjacently, and `run_mutex_crossing` asserts
    that on the emitted text (`MUTEX-SHARE-CROSS-EMISSION-LOGGED-SEQUENCE`).
    `mutex-share-cross`, `mutex-share-cross-emission`.
    **Landed at the wave-35 close, three waves after the card proposed it**
    (`MUTEX-SHARE-CROSS-INTEGRATOR-LIST-UNLANDED`, the measured cost of
    `WAVE-32-CLOSE-UNSWEPT`): the words above are the card's §13.1 block quote
    reproduced by card `SWEEP-BLIND-SPOTS` §6.1 and landed unchanged. The
    number `15o` is the integrator's, the card having left it unassigned.
    Wave 35's `MUTEX-SHARE-CROSS-ARMS` supersedes one sentence of that card's
    report from its own record (constraint 21, not by editing it): the four
    arms of these rules that no test drove are now **two driven** by
    `crates/qcore-frontend/tests/mutex_share_cross_arms.rs` and **two proved
    unreachable** and kept as backstops justified by
    `QCore.MutexShareCrossArms` — and the two dead arms are **not** dead for
    the same reason, arm C's admitting protocol being a literal (so its residue
    is empty for *every* kind, independent of the only-pair property) while
    arm D does rest on that property. `mutex-share-cross`,
    `mutex-share-cross-arms`, `sweep-blind-spots`.
<!-- preserved:constraint-15o:end -->

<a id="constraint-27"></a>

<!-- preserved:constraint-27:start -->
27. The mutex model supports initially-unlocked non-recursive mutexes only.
    Dynamic shapes, condition variables, try/timed locks, rwlocks and recursive
    acquisition are represented refusals, not omissions. `remaining-t02`.
<!-- preserved:constraint-27:end -->

## Field-footprint profile boundary — CT10

`source.bitfield-concurrency.v1` derives one
declared record layout and actual `write OBJECT.FIELD` spawn captures into
`DisjointFootprintsV1`. Its certificate proves only field-footprint
independence on the explicit `SpawnAndJoinSucceed` premise; it runs no
scoped-task ledger, emits no C and publishes no reference/native execution.
Adjacent bit-fields in one declared storage unit refuse at `Q661`; ordinary
disjoint fields and distinct bit-field units are admitted controls. Declared
target layout and Rust-to-Lean correspondence remain premises.

It must not be exported as a checked scoped-task certificate. Ordinary
record/task refusals are unchanged outside this explicit-layout profile.
[Scope and controls](../reports/ct10-source-layout/README.md).
