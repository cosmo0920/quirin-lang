# CG28-WEAK-MEMORY-EVENTS

Wave 48, round 9, High. Card CG28 of the
[Clang gap queue](../../plans/clang-gap-cards/README.md), gap entries W02–W06
(evidence Q34); core model plus routes G, T and the atomic-publication
extractor. Gateway rule [11t](../../agents/gateway.md#constraint-11t) (a
gateway rule because the route changes; the core change is described there
too). Lean: `QCore.WeakMemory` (new,
[`proof/QCore/WeakMemory.lean`](../../../proof/QCore/WeakMemory.lean), audited
in `proof/AuditWeakMemory.lean`) and `QCore.AtomicSourceRoute` (extended).
Base `b6a3dc42` (= `main` = `origin/main` at launch).

**Published guarantee (phase 4).** The property this card publishes is the
core checker's: **C11 (N1570) validity and data-race freedom of one supplied
candidate execution**, now including read-modify-write, compare-exchange and
fence events. Its scope is one graph (`SINGLE_GRAPH_ASSUMPTION`; execution
coverage is CG29's), and it is reported apart from custody coverage: route G's
verdicts stay custody verdicts and claim no ordering.

## 1. What changed

- **Core model** ([`weak_memory.rs`](../../../crates/qcore-core/src/weak_memory.rs),
  additive). New events `AtomicReadModifyWrite`, `AtomicCompareExchange`
  (with `CompareExchangeStrength`) and `AtomicFence` (with `FenceScope`); new
  errors `AtomicityViolation` and `InvalidCompareExchange`; new
  `compare_exchange_failure_admitted`. The existing read and write projections
  include the new events, so reads-from, modification order, coherence, S and
  race checks cover them unchanged.
  - *RMW atomicity* (N1570 7.17.3): an RMW reads the immediate
    modification-order predecessor of its own write.
  - *Compare-exchange*: success is an RMW with the success order; failure is
    a load with the failure order. A strong exchange fails only on a
    differing value; a weak one may also fail on an equal value (spurious
    failure). The failure order is neither release nor acq_rel and no
    stronger than the success order (7.17.7.4p2).
  - *Release sequences*, **one standard: C11 as published (N1570
    5.1.2.4p10)**: the head, then every modification by the head's thread or
    that is an atomic RMW. Not C++20's P0982 weakening, not RC11. The
    existing same-thread-store rule was already N1570's; RMWs now continue a
    sequence from any thread.
  - *Fences*: synchronises-with now also follows 7.17.4p2–p4 (fence–fence,
    fence–operation, operation–fence, through the hypothetical release
    sequence of the modification after a release fence). A seq_cst thread
    fence joins S, with 7.17.3's three fence observation rules (not C++14's
    LWG 2130 rule). A signal fence orders only against a handler of its own
    thread, which the model has none of: it synchronises nothing and joins
    no S.
- **consume (decision).** Kept **out of scope**. The model has no consume
  order and no dependency-ordered-before relation. Treating consume as
  acquire would add happens-before edges a consume read does not create
  (acquire orders every later access; consume only dependent ones), so the
  race check could miss a real race. That strengthening is unsound for the
  published property, so it is refused: a consume load (unchanged), and now a
  consume read-modify-write or compare-exchange order (success or failure),
  by the named refusal `ConsumeOrder`. A consume **fence** is an acquire fence
  by 7.17.4.1p2's own text and is routed as one; that is the standard's
  meaning, not a strengthening.
- **Route** ([`atomic_source_route.rs`](../../../crates/qcore-c-gateway/src/atomic_source_route.rs)).
  - *Routed rows*: the read-modify-write rows, the compare-exchange rows, the
    thread- and signal-fence rows (new class `SignalFence`) and the `_Atomic`
    compound assignment and increment. `Operation` gains `ReadModifyWrite`,
    `CompareExchange`, `ThreadFence` and `SignalFence`; `RoutedAtomic` gains
    `failure_order`, `strength`, `expected` and `value_by_pointer`.
  - *Events*: a routed access becomes the model's event through
    `RoutedAtomic::event(location, EventValues)`. `event_kind` now answers
    `None` for an operation that is not plain, instead of falling through to
    `AtomicInit`.
  - *Compare-exchange operands*: the strength comes from the name for
    `__c11_*` and from the evaluated weak flag for the GNU forms. The generic
    `__atomic_compare_exchange` takes its desired value by pointer, typed by
    the named object's own (desugared) type.
  - *Refusals by name*: the legacy `__sync` rows (new class `LegacySync`: GCC
    full barriers, not C11 orders); a consume RMW or compare-exchange order;
    an invalid or too strong failure order; an unevaluated weak flag
    (`UnevaluatedStrength`). `__atomic_exchange` (by-pointer value and
    result) is reclassified `ByPointer`. The refusal variants
    `ReadModifyWrite`, `CompareExchange` and `Fence` are gone, because nothing
    refuses for those reasons any more.
- **Route G** ([`legacy_atomic_route.rs`](../../../crates/qcore-c-gateway/src/legacy_atomic_route.rs)).
  - *Fences*: no custody effect.
  - *Read-modify-writes*: the object's dereference with a write.
  - *Compare-exchange*: the object's dereference with a write; the
    `expected` operand's write (`*e` through the typed reader, or `&x`
    forgetting the scalar local); the desired value's read.
  - *Produced values*: an RMW's old value and a compare-exchange's outcome
    may go to a scalar destination or a scalar `return`.
  - *Qualified forms*: an `_Atomic` compound assignment or increment through
    `*p` is the RMW's effect.
  - *Operands*: an operand value must be a literal, an enumerator or a
    non-pointer named object.
  - *Hooks*: unchanged positions (still fallbacks after every older route).
- **Route T**: no code change. It names the routed events through the route's
  answer, and no typed-source consumer decides one.
- **Publication** ([`atomic_publication_source.rs`](../../../crates/qcore-c-gateway/src/atomic_publication_source.rs)):
  a routed RMW, compare-exchange or fence of the flag stays refused, now by
  the text "… is routed to the weak-memory model but is not a publication
  shape this route decides (CG19-PUBLICATION-ONE-FLAG-ONE-PAYLOAD)".

Shared foundations (`elaboration.rs`, `typed_place.rs`, `typed_access_path.rs`,
`call_instantiation.rs`, `legacy_c.rs`, `typed_source.rs`) are **not**
touched. Added shared items are all in `qcore_core`: the three event
variants, `CompareExchangeStrength`, `FenceScope`, the two error variants and
`compare_exchange_failure_admitted` (re-exported).

## 2. Cited claims

Each claim was reproduced before it was relied on. The BEFORE controls ran at
the base, from a `git archive` of `b6a3dc42` with its own target directory
(`private/cg28/before-controls-*.log`, every exit 0). The controls were CG00's
`w02_w03_w04_core_has_no_rmw_cas_or_fence_event`,
`w05_w06_core_has_no_consume_order` and
`w02_w03_w04_atomic_rmw_cas_and_fences_are_uncontracted_calls`, and CG19's
`g_routes_the_cmetrics_plain_load_and_store` and
`g_refuses_rmw_cas_fence_and_consume_by_name`, with route unit tests 7 / 7.

- **W02 (RMW, exchange, `fetch_*`): reproduced.** The core's header said RMWs
  are not represented, and the event enum had no kind for them. At the
  source, CG19 refused them by name.
- **W03 (compare-exchange): reproduced.** There was no CAS event. The entry's
  "updates to expected" is represented at the source, where route G writes
  the `expected` object. In the model, the observed value is carried on the
  event, and the non-atomic write to `expected` is the graph's own event: it
  is not tied to the compare-exchange.
- **W04 (fences): reproduced.** The core excluded fences, and the source route
  refused them by name. W04's warning that a neutral call cannot establish
  happens-before is respected: a fence synchronises only through the 7.17.4
  rules.
- **W05 (consume): reproduced.** There is no consume order. Decided: it stays
  out of scope, refused (§1).
- **W06 (release sequences): reproduced.** Before this card the sequence
  followed N1570 for plain stores only. It now follows N1570 with RMWs, and
  no other standard.
- **Q34: reproduced** by reading `weak_memory.rs` at the base.
- **Correction to the brief: the Lean `WeakMemory` module did not exist.**
  The brief and the queue name "Lean parity in the WeakMemory module". At the
  base the only Lean model of the checker was
  `QCore.ResearchExtensions.C11Execution`, five abstract Booleans. This card
  creates `QCore.WeakMemory` as an executable model of the checker's atomic
  fragment.
- **Correction to this card's own prediction (a slip, no effect): the unit
  name.** The corpus function is in `cmt_atomic_clang.c`; the prediction named
  `cmt_atomic_gcc.c`, a different unit with the same function text.

## 3. Prediction and outcome

[PREDICTION.md](PREDICTION.md) was frozen and committed (`c7c19cbb`, rebased
as `08068863`) before any AFTER measurement. It discloses that the code
preceded it, as CG19's did.

| Prediction | Outcome |
|---|---|
| `cmt_atomic_compare_exchange` Unknown → Safe, its only leaf removed | **Unknown → Safe**. Its one leaf, ``atomic compare-exchange `__atomic_compare_exchange` is not represented by the weak-memory model (CG28)``, is gone. |
| Nothing else moves: 630 / 10 / 4,194 | **630 / 10 / 4,194**. One move, no same-status function with a changed leaf, and no other leaf delta. |
| No Safe → non-Safe move, no added leak or violation | None. |
| System and slice byte-identical | **Byte-identical**, with no `atomic-route` row in either parity stream. |

The prediction held exactly. There is no correction: no function moved
Safe → non-Safe. The prediction named the unit `cmt_atomic_gcc.c`; the
measured unit is `cmt_atomic_clang.c`, with the same function text (§2).

## 4. Measurement

The release `quirin-audit` was run with `QCORE_TYPED_PATH_PARITY` on. The
corpus arbiter (`scripts/run_pinned_audit.py … 66910c10 --jobs 4`) ran
through [`scripts/measure.sh`](scripts/measure.sh) and
[`scripts/corpus.sh`](scripts/corpus.sh). The system route and the tail-pause
slice (`attempt-1`, run `cg28-NAME`) were run as CG05 ran them. Every exit was
0 except the records named below.

| Run | Tree | Binary | Corpus | System | Tail-pause slice |
|---|---|---|---|---|---|
| BEFORE | base `b6a3dc42` | `068fda9d…` | **629 / 10 / 4,195** `3d53a5d0…` | 44 / 2 / 188 `9f786381…` | 29 / 1 / 239 / 1 P `3565659a…` |
| AFTER (on the base) | `4dded2f1` | `82bb3d82…` | **630 / 10 / 4,194** `d7be9d8c…` | 44 / 2 / 188 `9f786381…` | 29 / 1 / 239 / 1 P `3565659a…` |
| BEFORE (main) | `main` `b934c560` | CG25's | 630 / 10 / 4,194 `5ade5aab…` (CG25-RESOURCE-KINDS' figure) | 44 / 2 / 188 | 29 / 1 / 239 / 1 P |
| AFTER (rebased tip, on `main` `b934c560`) | TIP-HEAD | TIP-BINARY | TIP-CORPUS | TIP-SYSTEM | TIP-SLICE |

**BEFORE.** The corpus equals the brief's baseline exactly. System and slice
are the deduplicated totals (`scripts/compare.py`); they equal the
baseline's. The raw digests differ from the brief's (`79404fed…`,
`93cc660c…`) only in path-dependent fields. The configured route hashes the
contract sources of `contract/profiles/fluent-bit` into
`compile_configuration`, and that path names the measuring worktree.
Structurally, CG05's `after3` slice report and this BEFORE differ in exactly
48 fields, all `resource_contract_sources` or `inventory_sha256` under
`compile_configuration`, and in nothing else (`scripts/normalized.py`).
CG17-LAYOUT-DERIVED-EXTENTS reports the same effect. This is not a status
difference, so the run was not stopped.

**Attribution** (corpus BEFORE → AFTER on the base;
[`artifacts/corpus-comparison.json`](artifacts/corpus-comparison.json)).
There is one move, `cmt_atomic_clang.c::cmt_atomic_compare_exchange`
Unknown → Safe, by removing its only leaf. The leaf delta is otherwise empty,
and no same-status function changed.

**Census** of atomic leaves
([before](artifacts/before-census.json), [after](artifacts/after-census.json),
`scripts/census.py`, which matches every `__atomic_*`, `__c11_atomic_*`,
`__sync_*` and `_Atomic …` name):

- *BEFORE*: 1 leaf on 1 function. This was the only atomic refusal the brief
  asked about (RMW, exchange, `fetch_*`, compare-exchange, fences, consume);
  everything else is 0.
- *AFTER*: 0.
- *System and slice*: 0 before and after.

The AFTER parity stream holds 12 `atomic-route-*` rows on 3 functions, each
admitted with `SequentiallyConsistent`: the compare-exchange (a `return`),
CG19's load and store. The repeats are the rebuilds of the W4 re-check. The
BEFORE stream holds 7.

**Run records** (kept under `private/cg28/`).
- `before-repeat` re-ran the BEFORE binary by mistake:
  `run_pinned_audit.py` always runs `target/release/quirin-audit`, and
  `measure.sh`'s `BINARY` override reaches only the system and slice. Its
  result is byte-identical to the BEFORE (`3d53a5d0…`), which is a
  determinism check. `scripts/corpus.sh` now installs the binary first.
- `after-diskfull` (exit 101, `No space left on device` in the parity sink)
  and `after-diskfull2` (stopped by this card at 2.9 GB free) are host-disk
  failures. The host is shared by eleven cards; the coordinator then
  serialised heavy steps under a lock.
- The reported AFTER is the third corpus run of the AFTER binary.

**Published property, measured separately.** The weak-memory checker's
results are those of its tests and the Lean vectors (§5, §6). The corpus
arbiter reports custody only.

## 5. Tests

- **Flipped BEFORE controls.**
  - *Core*: `clang_gap_core.rs`
    `w02_w03_w04_core_has_no_rmw_cas_or_fence_event` now matches the three
    new kinds. `w05_w06_core_has_no_consume_order` is unchanged, by
    decision.
  - *Phase 4*: `clang_gap_phase4.rs`
    `w02_w03_w04_atomic_rmw_cas_and_fences_are_uncontracted_calls` is now
    custody-`Safe` for all three forms.
  - *CG19's route tests*: in `atomic_source_route.rs`, the cmetrics
    compare-exchange and the RMW and CAS controls are `Safe`, the
    enumerator-order fence is refused for its order, and the `_Atomic` RMWs
    through a pointer are `Safe`.
  - *Publication*: in `atomic_publication_source.rs`, the exchange of the
    flag keeps a named refusal with the new text.
- **New core tests** (`crates/qcore-core/tests/unit/weak_memory_cg28.rs`, 5).
  - *Parsed litmus graphs*: the 20 litmus graphs, **parsed from
    `WeakMemory.lean`**, run through `check_c11_execution`, with error
    categories equal to `litmusVectors`.
  - *Named outcomes*: MP release/acquire accepted; MP relaxed a `DataRace`;
    MP with fences accepted, with happens-before from the payload write to
    the payload read; the CAS-weak loop accepted; strong spurious failure
    refused; a lost update an `AtomicityViolation`.
  - *Order validity*: `compare_exchange_failure_admitted` matches Lean's
    `casOrdersValid` on all 25 pairs.
  - *Fences and S*: RMW and fence orders; a signal fence's claimed edge is
    refused; seq_cst RMW and fence membership of S.
- **New route tests.**
  - *Unit* (`tests/unit/atomic_source_route.rs`): `casVectors` through
    `classify_builtin`, and routed fence, RMW and CAS events run through the
    core checker. `routeVectors` grows 18 → 26 rows and `orderVectors`
    16 → 32.
  - *Source* (`tests/atomic_weak_memory_events.rs`, 7 Clang tests): the
    cmetrics shape; RMW forms; compare-exchange forms and the weak retry loop
    `do { ok = atomic_compare_exchange_weak(a, &e, 1); } while (!ok);`;
    fences, including consume-as-acquire; the refusal controls (consume RMW
    and CAS orders, a stronger or release failure order, an unevaluated weak
    flag, `__sync_synchronize`, the sized `__sync_fetch_and_add_8`,
    by-pointer `__atomic_exchange`, an opaque operand, a named `_Atomic`
    object, a pointer-valued exchange); dereference obligations kept (a
    released object or `expected` pointer is not `Safe`); route T's names.

## 6. Lean

**`QCore.WeakMemory`** (new; 31 audited theorems in
`proof/AuditWeakMemory.lean`). It is an executable model of the checker's
atomic fragment: the same event kinds, the projections, sequenced-before,
the closures, release sequences, synchronises-with, reads-from, modification
order, compare-exchange consistency, atomicity, coherence, S with the fence
rules, and races. `check` returns the failing categories.

| Theorem | What it states |
|---|---|
| `check_accepts_iff` | An accepted graph satisfies each component check. |
| `accepted_rmw_is_atomic` | **RMW atomicity**: every RMW reads the immediate modification-order predecessor of its own write. |
| `accepted_sw_is_expected`, `accepted_expected_is_supplied` | Supplied synchronises-with equals the derived relation. |
| `accepted_is_race_free` | An accepted graph is race-free. |
| `release_sequence_definition` | **The release sequence** is N1570 5.1.2.4p10's definition, as an iff. |
| `rmw_continues_release_sequence`, `foreign_store_ends_release_sequence` | An RMW of any thread continues a sequence; a store of another thread ends it. |
| `release_acquire_synchronises`, `fence_fence_synchronises`, `fence_operation_synchronises`, `operation_fence_synchronises` | **Fence synchronisation**: each rule produces its edge. |
| `signal_fence_is_not_a_thread_fence`, `signal_fence_neither_releases_nor_acquires` | A signal fence takes part in no synchronisation. |
| `cas_failure_no_stronger` | **The CAS failure order** is neither release nor acq_rel and is no stronger than the success order. |
| `strong_failure_observes_another_value`, `weak_failure_may_be_spurious`, `success_observes_expected`, `failed_cas_writes_nothing`, `failed_cas_reads_with_failure_order` | Compare-exchange outcomes. |
| `failures_nil_iff` | The checker's category list is empty exactly when every component passes. |
| `mp_release_acquire_accepted`, `mp_relaxed_races`, `mp_fences_accepted`, `sb_seq_cst_weak_outcome_refused`, `sb_sc_fences_weak_outcome_refused`, `rmw_lost_update_refused`, `release_sequence_continues_through_rmw`, `release_sequence_ends_at_foreign_store`, `cas_weak_loop_accepted`, `litmus_vectors_hold` | The litmus outcomes; `litmus_vectors_hold` covers all 20 shared vectors. |

**Shared litmus vectors (20).**
- *Message passing*: release/acquire; relaxed; fence–fence; fence–acquire;
  release–fence; signal fences; signal fences with a claimed edge.
- *Store buffering*: relaxed; seq_cst; seq_cst fences, with the weak outcome
  and with the store seen.
- *Read-modify-writes*: a lost update; an RMW chain.
- *Release sequences*: continued by an RMW; ended by a foreign store;
  continued by a same-thread store.
- *Compare-exchange*: the weak loop; a strong spurious failure; a failure
  order stronger than the success order; an acquiring failure.

They are proved by kernel `decide`. The Rust test parses each `def` from the
`.lean` file, runs it through `check_c11_execution` and compares the error
categories, so the vectors are written once and bound in both directions.

**`QCore.AtomicSourceRoute`** (extended, 32 audited theorems, 23 before).
- *Classes*: `formTable` gains the `signalFence` and `legacySync` classes and
  reclassifies `__atomic_exchange` as `byPointer`.
- *Routing*: `routeOrder` covers RMW, compare-exchange and fences.
  `routeFailureOrder` and `routeCas` (success, failure, strength) are new.
- *Theorems replaced*: `read_modify_write_refused`, `compare_exchange_refused`,
  `fence_refused` and `route_class_is_plain` gave way to
  `read_modify_write_routed`, `fence_routed` and `route_class_is_routed`.
- *Theorems added*: `consume_fence_is_acquire`, `consume_order_refused`,
  `legacy_sync_refused`, `routed_failure_admitted_by_model` (a routed failure
  order is one `QCore.WeakMemory.casOrdersValid` admits),
  `routed_failure_no_stronger`, `routed_cas_failure_admitted`,
  `cas_strength_by_name`, `gnu_cas_strength_by_weak_flag`,
  `gnu_cas_unevaluated_weak_flag_refused` and `cas_vectors_hold`.
- *Theorems widened*: `order_one_never_routed`, `consume_never_routed` and
  `unevaluated_order_refused` now cover the new operations.
- *Vectors*: `routeVectors` grows to 26, `orderVectors` to 32, and
  `casVectors` (12) is new. Each is run through the Rust functions by
  `tests/unit/atomic_source_route.rs`.

**Axioms and placeholders.** No placeholders and no project axioms.
`AuditAll.lean` passes: 15,728 public theorems, 77,902 declarations, only
`propext`, `Classical.choice` and `Quot.sound`. Five `WeakMemory` theorems use
`Classical.choice` through `simp`.

**Premises by review.** The Rust-to-Lean correspondence of the checker's
mutex and spawn/join parts, which the Lean model omits and the vectors do not
use; faithful Clang extraction (trust boundary 28).

## 7. Scope stated and axes

- **Covered:**
  - *Events*: RMW (exchange, fetch-and-op, op-and-fetch), compare-exchange
    (strong, weak, `_n`, generic) and thread and signal fences.
  - *Orders*: every order except consume, plus the consume fence.
  - *Qualified forms*: the `_Atomic` compound assignment and increment
    through a pointer.
  - *Operands*: values that are literals, enumerators or non-pointer named
    objects.
- **Refused by name:**
  - *Orders*: consume orders; enumerator orders in G and T
    (CG19-ENUMERATOR-ORDER-UNEVALUATED-IN-G-AND-T).
  - *Builtins*: the `__sync` builtins; the by-pointer load, store and
    exchange; `__atomic_test_and_set`, whose object Clang wraps in a
    `BitCast` the reader does not strip.
  - *Shapes*: an `_Atomic` RMW of a named object; pointer- or
    aggregate-valued atomics; operands with effects.
- **Not claimed:**
  - *By route G*: any ordering, race-freedom or liveness claim.
  - *By any route*: race freedom over all executions (CG29), reclamation
    (CG29) and liveness (W11).
  - *Graph premises, not checked*: an RMW's arithmetic and the write a failed
    compare-exchange makes to `expected`.
- **Evaluation order:** none introduced. Each atomic is one event, and route
  G's operand reads are those of the plain access.

## 8. Follow-ups

- **Retired: `CG28-WEAK-MEMORY-EVENTS`.** Every DISCHARGE clause holds:
  - the BEFORE reproduces the cited refusals (§2), with the one correction
    stated, about the missing Lean module;
  - `PREDICTION.md` was frozen;
  - the AFTER attributes every move;
  - no follow-up was absorbed;
  - there are Rust refusal controls, and audited Lean with no placeholders or
    project axioms;
  - the §4 acceptance holds: axes in §7, route G and the core consumer named,
    no order introduced.
- **Registered:**
  - `CG28-LEGACY-SYNC-BUILTINS`: `__sync_*` refused by name or as unlisted;
    0 corpus leaves.
  - `CG28-UNREAD-ATOMIC-OPERAND-SHAPES`: a named `_Atomic` object's RMW,
    `__atomic_test_and_set`, by-pointer forms, operands with effects and
    pointer values; 0 corpus leaves.
- **Amended:**
  - `CG29-EXECUTION-COVERAGE-AND-RECLAMATION`: the prerequisite landed, with
    the events available to it.
  - `CG19-PUBLICATION-ONE-FLAG-ONE-PAYLOAD`: an RMW, compare-exchange or
    fence of the flag is routed but not a publication shape (new text); no
    production route builds a graph holding the new events.
  - `CG19-ENUMERATOR-ORDER-UNEVALUATED-IN-G-AND-T`: it now also covers the
    new events; 0 corpus leaves.
- **Decided, not registered:** consume stays out of scope (§1). Reopening it
  needs a dependency-ordering relation in the model and a source extraction
  of dependencies; it is a phase-4 choice, not open work.
- **Ledger:** 670 → 671 on `main` `b934c560` (666 → 667 on the base).
  `scripts/update_ledger.py` was applied once, to `main`'s matrix.

## 9. Gate

MEASURED-GATE
