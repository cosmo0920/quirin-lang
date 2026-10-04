# GAPO-PARITY-DECISIONS-NONDETERMINISTIC: a consultation's parity decisions are its own

Card GAPO-PARITY-DECISIONS-NONDETERMINISTIC (wave 48, round 19; determinism
fix). Base: local `main` at `0ebe77f86`. Authority: the ledger entry of the
same name and section 4 of the
[optimization card's report](../gateway-analysis-path-optimization/README.md).
Gateway rule **12q** (new); no Lean change (section 6).
[PREDICTION.md](PREDICTION.md) was frozen at `fd91fd536`, before the first
implementation commit.

**Outcome.** The corpus parity stream (`QCORE_TYPED_PATH_PARITY`) is now one
multiset over five `--jobs 4` runs of one binary; five runs of the BEFORE
binary wrote two. Every `resource-diagnostics*.json`, sidecar, `audit.log` and
console is byte-identical to BEFORE on the corpus, system and tail-pause
routes (corpus **756 / 9 / 4,069**, `ce57d194…`; system **85 / 4 / 359**;
slice **45 / 1 / 354 / 4 Pending**): no verdict moved, and none could
(section 2). AFTER against BEFORE, the corpus stream loses **122
`typed-place` rows** (43 row kinds in 30 functions) and changes no other row:
each was written by a statement consultation whose own Clang node did not
match, and carried only decisions another reader had left on the worker
thread. System and slice streams are unchanged.

## 1. The defect, reproduced and minimised

`typed_place::DECISIONS` was a thread-local list. Every typed access-path
reader pushed into it when it decided a place (rule 10t's tracked place in
`tracked_chain`, rule 12j's record-object place through
`record_place_row`), whichever consumer built the reader; only the recording
consultations took it (`legacy_c::typed_path_plan` and the four hooks of
`legacy_typed_positions`), and they cleared it only once their own node had
matched (`StatementIndex::statement_with` after the unique match, `argument`
after `argument_node`, `expression_node` at its start). Readers built by
consumers that record nothing (`legacy_aggregate_value` three times,
`legacy_elaboration`, `legacy_atomic_route` twice, the residual-shape probes
of `typed_position_shapes`, the parity-only second asks of
`legacy_typed_positions`) left their decisions behind, and the next
recording consultation whose node did not match wrote them as its own
`typed-place` row. Within one unit that is deterministic (the leftovers come
from the same unit's earlier consultations); across units it is not: which
unit a worker thread analysed before depends on the scheduler.

**Five BEFORE runs** (`private/gapo-nd/before1`…`before5`, the base's release
binary copied to `private/gapo-nd/bin/before`, `7d050d1c…`, every run.json's
`binary_sha256` checked equal): four corpus streams of **669,571** rows, one
(`before3`) of **669,570**; the one row apart is the optimization card's,
`typed-place` at `cio_file_st_get_meta_len` line 2223 (statement
`__qcore_returned_value = ( uint16_t ) …`, `new_admitted: false`) carrying a
rule-12j `st` place that belongs to another consultation. System and slice
streams agreed across their two runs, and every output file agreed across all
runs ([identity](artifacts/identity-before.json)).

**Minimised** ([before_unit_test.rs](scripts/before_unit_test.rs),
[before-unit.sh](scripts/before-unit.sh)): two consultations on one thread at
the base, a position reader whose consumer records nothing
(`StatementIndex::condition` on `s->next->in.x`), then a statement
consultation whose node does not match (`s->v = 9`). The second consultation's
record (what `record_decisions` writes) must hold none of the first's
decisions; at the base it holds one, **3 of 3 runs fail**
([logs](artifacts/before-unit/)). After the card the same scenario is
`consultation_decisions::tests::a_declined_consultation_after_an_unrecorded_reader_holds_nothing`,
and `a_consultation_records_exactly_its_own_decisions_in_any_order` drives four
consultations (a 10t place, a 12j place, a 10v classification, a declined
one) forward, backward, interleaved and after unrecorded readers, requiring
each record to equal the consultation's record when it runs alone.

## 2. Could a verdict depend on the leaked state?

No, by reading and by measurement.

* **Reading.** Both lists (`typed_place::DECISIONS` and
  `pointer_store::DECISIONS`) were private to their modules, pushed only when
  `parity_enabled()` (the environment variable is set), and read only by the
  two `record_decisions` functions, which hand the rows to
  `typed_access_path::record_row` (the parity sink) and return `()`. No
  value derived from either list reaches a plan, an instruction, a transfer
  or a report row. The same holds for the third parity-only state replaced
  here, the call graph behind `call-instantiation` rows (section 4).
* **Measurement.** BEFORE and AFTER, with the stream on and off
  (`before-np`, `after-np`, `PARITY=0`), write byte-identical
  `resource-diagnostics*.json`, sidecars and `audit.log` on the corpus
  (section 5): the published results do not depend on whether parity
  bookkeeping runs at all.

## 3. The fix (rule 12q)

* `consultation_decisions.rs` (new): `Decisions`, the place decisions and
  pointer-store classifications of one consultation, kept only on parity
  runs, written as the `typed-place` and `pointer-store` rows by
  `record_places` and `record_classifications` (the rows' bytes are
  unchanged).
* `typed_access_path::Reader` gains `decisions` (set by `recording`); the
  readers of `tracked_chain`, `record_object_place::record` and
  `pointer_store::read_store` write into it. `StatementIndex::statement_with`,
  `condition`, `assignment`, `returned` and `argument` take an
  `Option<&Decisions>`: the five recording consumers create one per
  consultation on their own stack and record it; every other caller passes
  `None`, and its reader keeps nothing. The four `clear_decisions` calls and
  both thread-local lists are gone. A consultation therefore starts with an
  empty record and its record is recorded or dropped on every exit path,
  early returns and declined probes included, by construction.
* `legacy_call_instantiation`: the parity call graph (`unit_graph`) is read
  once per parser (`Parser::call_graph_components`) instead of from a
  thread-local keyed by the token slice's address and length, which a later
  stream allocated at the same address with the same length would have
  answered with a stale graph.
* `value_connective::on_edge`, `with_edge_root`, `deciding` and route G's
  `LOWERING` flag restore their thread-local by a drop guard (the previous
  value, an unwind included) instead of by a statement after the body.
  `LOWERING` used to be reset to `false`; it is only ever set while `false`
  (route G declines while it is set), so restoring the previous value is the
  same value. `edge_scopes_are_restored_on_unwind` pins the restore.

## 4. Audit of thread-local and cell state in `crates/qcore-c-gateway`

Every `thread_local!`, process-wide `static` with interior mutability, and
`Cell` / `RefCell` / `OnceCell` field in the crate at the base.
"Leak" means state one consultation, scope or unit leaves for another.

| State | Owner, scope | Reset discipline at the base | Finding |
|---|---|---|---|
| `typed_place::DECISIONS` (TL) | none: every typed reader | taken by recording consumers, cleared only after their node matched | **leaked** (the defect); replaced by `Decisions` |
| `pointer_store::DECISIONS` (TL) | none: the statement reader | taken by `typed_path_plan` right after its only producer | same pattern, not observed to leak (one producer, one consumer); replaced by `Decisions` |
| `legacy_call_instantiation::UNIT_GRAPH` (TL) | parity rows of `call-instantiation` | cache keyed by `(tokens.as_ptr(), tokens.len())`, never cleared | **stale on address reuse** (parity only); per-parser `OnceCell` |
| `value_connective::EDGE`, `EDGE_ROOT`, `DECIDING` (TL) | rules 10z/11m edge lowering (semantic) | saved and restored around a closure; not on unwind | **unwind leaves the scope set**; drop guards |
| `legacy_value_connective::LOWERING` (TL) | route G leaf lowering (semantic) | set `true`, reset to `false`; not on unwind | as above; drop guard restoring the previous value |
| `typed_access_path::ANALYSED_SOURCE` (TL) | one analysis scope | `SourceScope` drop guard restores the outer | sound |
| `typed_access_path::PARITY_MUTED` (TL) | the W4 re-check | drop guard | sound |
| `typed_access_path::UNIT_ASTS`, `UNIT_LAYOUTS`, `typed_c_ast::FILTERED_DECLARATIONS` (TL) | Clang results across a worker's scopes | caches keyed by the complete launch identity, compared byte for byte; successes only; bounded | sound: a hit is the launch's own answer (`clang_reuse` module documentation) |
| `legacy_condition_acquisition::CONSULTATIONS` (TL) | `#[cfg(test)]` only | test log | not built in release |
| `typed_access_path::PARITY_SINK`, the census sinks (`*_census.rs`, `legacy_callee_scope`, `legacy_parenthesised_callee`, `legacy_call_argument_access`, `legacy_return_family`, `legacy_route_table::STREAM`, `legacy_leak_witness::WITNESSES`) (Mutex) | process output streams | append only | sound (outputs, not inputs) |
| `indirect_census::UNIT_SEQ`, `provenance_rounds::SEQ`, `legacy_leak_witness::INVOCATION` (Atomic) | census-feature sequence numbers | monotonic | sound: census runs analyse one unit at a time (validation policy) |
| `clang::RESOLVED`, `c_numeric::MODELS`, `index_extent::TARGETS` (Mutex) | process-wide Clang resolution and target facts | keyed by the resolution environment, the executable, the full command; `MODELS` keeps a failure (fail-closed) | sound |
| `lib.rs` `CLANG_AVAILABILITY`, `CLANG_WARNING` | `#[cfg(test)]` only | once | not built in release |
| `Parser` (`legacy_c.rs`): `typed_path_index`, `typed_root_declarations`, `declared_no_return`, `call_graph_components` (OnceCell) | one parser (one function's CFG build) | lazily read facts of the parser's own stream | sound |
| `Parser::typed_position_uses` (Cell) | one parser | saved and restored around each desugared plan and residual probe | sound |
| `Parser::condition_widening` (Cell) | one condition plan | reset to 0 at each plan's start, read after it | sound: no route of a condition plan reaches `planned_condition`, so plans do not nest (read) |
| `Parser::variadic_tail_widening`, `variadic_tail_readings`, `typed_place_origin` (Cell/RefCell) | one classification | set around it, reset to a constant after | sound: a nested setting could only reset early, which narrows (the occurrence and the tail readings refuse) |
| `GContext::aliasing`, `ClangSelections::read` (OnceCell) | one context, one body | lazily read facts | sound |
| `legacy_counted_users::Unit` `derived`, `verified`, `consulted` (RefCell) | one unit's counted-user check | memo maps of that unit | sound |
| `clang_reuse` `UnitCache` / `LaunchCache` entries (RefCell) | the TL caches above | see above | sound |
| `thread_entry_census` reader cells (`decisions`, `notes`, `property_readings`, `copy_literal`) | one census reader | accumulated for that reader's report; `copy_literal` set and cleared around one copy reading | sound |
| `SourceContext::ast`, `FunctionEntry::by_line` (OnceCell) | one scope, one function | lazily computed from the scope's own text | sound |

A worker of `qcore_audit::parallel::run_ordered` catches a panic and goes on
to its next unit; the consumer re-raises it at its position in unit order, so
a run with a panic publishes nothing. With the drop guards, no gateway
thread-local is left set by an unwind even for the units analysed in between.

`GAPO-RETURN-PROBE-COUNTER-RESTORATION` is not this mechanism: it concerns
plain parser fields (`next_site`, `element_lowerings`, the list frames and
element origins) that declined classification probes do not restore, which
is per-parser and deterministic and can change site numbering and a
lowering choice. It stays registered; this card does not touch it.

## 5. Measurement

Five corpus runs per side, system and slice twice per side, `--jobs 4`, each
run under the shared lock with its own copy of the binary
([measure.sh](scripts/measure.sh), [runs.sh](scripts/runs.sh)); the AFTER
binary is `private/gapo-nd/bin/after`, `a574fa58…` (tip `ac31392d9`). Identity
proofs by [identity.py](scripts/identity.py) (the optimization card's script
without its VARYING exception: every row of every stream is compared),
row classes by [parity_classes.py](scripts/parity_classes.py).

| Comparison | Files | Corpus parity | System parity | Slice parity |
|---|---|---|---|---|
| BEFORE x5 against `before1` ([proof](artifacts/identity-before.json)) | all identical | **two multisets**: 669,571 rows x4 (`e1a81751…`), 669,570 x1 (`3007a75b…`, `before3`, without the `cio_file_st_get_meta_len` row) | identical (141,186 rows, 2 runs) | identical (50,426 rows, 2 runs) |
| AFTER x5 against `after1` ([proof](artifacts/identity-after.json)) | all identical | **one multiset**: 669,449 rows x5 (`072a299c…`), no row set apart | identical (141,186, 2 runs) | identical (50,426, 2 runs) |
| AFTER `after1`, `after2` against BEFORE `before1` ([proof](artifacts/identity-before-after.json)) | **all 20 identical** (corpus, system, slice) | 122 rows removed, none added or changed ([classes](artifacts/classes-before-after.json)) | identical | identical |
| Parity off, `before-np` and `after-np` against `before1` ([proof](artifacts/identity-parity-off.json)) | all identical | not written | not run | not run |

Totals, BEFORE and AFTER alike: corpus **756 / 9 / 4,069** (`resource-diagnostics.json`
`ce57d194…`, main's figure), system **85 / 4 / 359** (`72862acb…`), slice
**45 Safe / 1 Unsafe / 354 Unknown / 4 Pending** (`21adb1f5…`). No function
moved, so no correction is owed.

The 122 removed rows ([classes](artifacts/classes-before-after.json)): every
one `position: "typed-place"`, `of_position: "statement"`,
`new_admitted: false`, in 43 row kinds over 30 functions (15 librdkafka
telemetry value builders `calculate_*`, 12 other librdkafka functions,
`pb_decode_inner`, `utf16_literal_to_utf8` and `cio_file_st_get_meta_len`;
most kinds are written two or three times because a unit is analysed under
several scopes). Their places are rule 12j's (42 rows),
rule 10t's (59) or both (21), and none belongs to the statement the row
names: each was written after the statement's own Clang node did not match,
so its consultation decided nothing. 121 of them recur in every BEFORE run
(the leftovers came from the same unit's earlier consultations on that
thread); the 122nd is the `cio_file_st_get_meta_len` row, whose leftover came
from whatever unit the worker had analysed before. No other row of any
stream changed: every row a recording consultation writes after its own
match carries the same places as before.

Timing (each route inside the lock, `--jobs 4`, interleaved with the other
card's work and this card's gate, [timing.json](artifacts/timing.json)):
corpus BEFORE 374.1-400.7 s (median 389.1), AFTER 382.6-402.7 s (median
397.0, +2.0 %); system 53.4, 49.2 against 53.8, 50.2 s; slice 27.5, 22.6
against 23.6, 23.7 s; with the stream off, corpus 344.6 s on both sides. The
ranges overlap; the per-parser call graph is read only on parity runs, so
any cost it adds is confined to them.

## 6. Lean

None owed. The card moves parity bookkeeping from thread-local lists into a
value the consumer owns, replaces an address-keyed parity cache, and makes
four restores unwind-safe. No decision table, transfer, route order or
verdict changes, and no Lean definition models the parity stream or the
restored scopes' bookkeeping. Lean audits stay at 287.

## 7. Prediction against outcome

| Prediction | Outcome |
|---|---|
| 1. Verdicts and published diagnostics byte-identical | **Held**: every file identical on all three routes, totals unchanged |
| 2. BEFORE nondeterminism reproduces in `typed-place` rows only | **Held**: one row, `cio_file_st_get_meta_len` line 2223 (`st`), in four of five runs |
| 3. Five AFTER corpus runs one multiset; system and slice twice each identical | **Held** |
| 4. AFTER against BEFORE: removals only, all `typed-place` rows of declined consultations; 1 to 50 row kinds | **Held**: 122 rows, 43 kinds, all `of_position: "statement"`, no content change, no other position touched |
| 5. Timing within noise (about 2 %) | **Borderline**: corpus median +2.0 % with overlapping ranges on a shared host; parity-off runs equal |
| 6. No Lean change | **Held** (section 6) |
| 7. The audit's other fixes verdict- and parity-neutral | **Held**: the call-graph, guard and flag changes moved no row (system and slice streams and every non-`typed-place` corpus row identical) |

## 8. Gate

GATETABLE

## 9. Follow-ups

* `GAPO-PARITY-DECISIONS-NONDETERMINISTIC`: **retired**. Its DISCHARGE holds:
  every consumer records into a value it owns (the reader records nothing
  without one), and five `--jobs 4` runs of one binary write identical
  multisets.
* `GAPO-RETURN-PROBE-COUNTER-RESTORATION`: unchanged (section 4).
* Nothing new registered: every leaking state the audit found is fixed here.

## Files

* [PREDICTION.md](PREDICTION.md); scripts: [measure.sh](scripts/measure.sh),
  [runs.sh](scripts/runs.sh), [identity.py](scripts/identity.py),
  [parity_classes.py](scripts/parity_classes.py),
  [before_unit_test.rs](scripts/before_unit_test.rs),
  [before-unit.sh](scripts/before-unit.sh), [gate.sh](scripts/gate.sh),
  [finish-gate.sh](scripts/finish-gate.sh),
  [update_ledger.py](scripts/update_ledger.py).
* Artifacts: identity proofs and row classes in [artifacts/](artifacts/);
  run outputs in `private/gapo-nd/` of the card's worktree.
* Code: `crates/qcore-c-gateway/src/consultation_decisions.rs` (new),
  `tests/unit/consultation_decisions.rs` (new), and thin edits in
  `typed_access_path.rs`, `typed_access_positions.rs`, `typed_place.rs`,
  `record_object_place.rs`, `pointer_store.rs`, `legacy_c.rs`,
  `legacy_typed_positions.rs`, `legacy_call_argument_access.rs`,
  `typed_position_shapes.rs`, `legacy_call_instantiation.rs`,
  `value_connective.rs`, `legacy_value_connective.rs`, `lib.rs`.
