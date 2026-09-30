# W48-QR-TASK-RECORD-CAPTURES: task workers receive and return records

Card 3 of the `.qr` language track (base `bcdc13b9` = local `main` =
origin/main; the worktree's HEAD was already that commit, so no reset or
fast-forward was needed). Task workers now **receive and return records**:
a record crosses a spawn and a `join` as its leaves, whole or field by field,
and a worker constructs, reads, writes and renders records of its own.
Grammar: implemented grammars §G02a ("Records across tasks"), §G02b, §G02j
and §G07.13; semantics: specification §L16 and the `Q5xx`/`Q6xx` rows and card
paragraph of §L21. The [prediction](PREDICTION.md) was frozen before any
source change, after the BEFORE fixture freeze.

Where the base stood (card 2's survey and a probe agree): every module that
spawns and declares a record type refused at the declaration, `Q500`, naming
`AGGREGATE-TASK-CAPTURE`.

## The mechanism

**A record is its leaves.** On the scoped-task route a record binding is one
task slot per **leaf** — each scalar field and each owned `Buffer<T>` field,
depth-first in declaration order, a nested record contributing its own leaves
under its field's name (card 2's leaf order). A record parameter is one worker
parameter per leaf; a record capture is its leaves' captures (a copy of each
scalar, a move of each owned Buffer); a field capture is the existing capture
of one leaf. So every custody question about a record is the unedited
checker's per-resource question about an owned leaf, and the ledger's
reservation, rollback, commit, completion and join act on a record's owned
leaves together, in one creation. A field write binds a **fresh** leaf slot
(slots are single-assignment); a refill is the core's new `RefillField`,
admitted exactly into a hole (`OverwriteOfLiveField` otherwise).

**The core sees a record whole in one place**: a worker's result.
`ScalarType::Record(key)` names a `TaskRecord` (its leaf paths and types) in
the new `TaskModule::records`; `return r;` / `return move r;` lowers to
`Return { value: ScalarOperand::Record(leaves) }`; the ledger completes it as
`TaskOutcome::Record(leaf outcomes)`, whose owned leaves must be exactly the
identities the completion carries; `join` binds the record value and the new
`UnpackRecord` splits it into fresh leaf slots, each carried identity in
exactly one owned leaf. `WorkerExit::returned` became a list (one identity per
owned leaf; one for an owned Buffer, none for a scalar), and a record result
that owns keeps its actual exits, as an owned result always did.

**The judgment's aggregate kind** (`qcore_core::transfer`): `leaf_mode` asks
each leaf the flat judgment in the mode the record asks of it (`move`
duplicates a leaf that duplicates and moves every other); `admits_aggregate`
is the conjunction — `copy` exactly when every leaf copies, `move` exactly when
every leaf crosses and some leaf moves; `write` is refused by name before any
leaf is asked; a sharing mode has no whole-record protocol. A refusal names the
first failing leaf and its own flat refusal — the missing contract. The
frontend consults it beside its enumeration and requires agreement, as for
every flat capture. The flat vocabulary, both flat matrices and every flat
theorem are unchanged.

**The lowering** (`crates/qcore-frontend/src/source_tasks/records.rs`) reads the
task route's record declarations (a field is a §G02t scalar, `own<Buffer<T>>`
or a record; `bytes` refuses), and marks what the core cannot see: a record
**moved whole** — into an outer record at construction, or to a worker on the
path on which the creation succeeded — is used no more, since its scalar
leaves would otherwise be stale copies. A record left in different states by
two continuing `match` arms is unsettled and used no more (no join of two slot
maps). `mut` joins the task effect row, admitted only in a module that
declares a record, and a field write requires it. On the composed route a line
renders a scalar field (`print dec r.f;`, `log info dec r.f;`), read by this
route alone.

**Emission.** A module returning a record gets a task result with a `leaves`
array (`QCORE_TASK_RECORD_CAPACITY`) and a logger arm per record kind: the
header `QCORE_TASK_RESULT TASK RECORD N` (through the fixed-width result arm,
installed for it) and one result line per leaf; `UnpackRecord` moves each leaf
out of the collected Result, clearing the stale copy. Every other module keeps
its bytes. The reference interpreter follows (`ScalarValue::Record`,
`render_result`), and the F02 encoding refuses a record outcome rather than
flattening it (constraint 15n's refusing side).

**One bound narrowed on purpose.** "At most one write-leased resource at a
time" became a per-resource rule: the ledger decides every lease per resource,
and two resources are two allocations, so two fields leased to two workers at
once hold no byte in common. Overlapping leases of one field, and a read share
beside a write lease, stay refused. The freeze found no tracked program that
the lift moves.

## Items

| Item | Status |
|---|---|
| 1. Record parameters, results and captures; the aggregate kind | **done**: a record of scalars by `copy`, one that owns by `move` and back through `join`; `AGGREGATE-TASK-CAPTURE` retired; both judgment theorems re-decided over records (below) |
| 2. Field-level captures | **done**: `copy r.f`, `move r.buf` (a hole the creator refills or does not use), `read r.buf share …`, `lend r.buf LOW..HIGH`, disjointness per leaf; a sub-record of scalars by `copy`, one that owns field by field (`W48-QR-NESTED-OWNED-SUBRECORD-MOVE`) |
| 3. Record operations inside workers | **done**: construct, read, write, refill, drop, return; a worker's `ArithmeticError` still reaches its creator through `join` (tested with a record result) |
| 4. §G02j rendering of fields | **done**, in the parent and in a worker, natively (one worker logging to stderr, the parent printing beside the trace) |
| 5. Stretch | **not done**: an imported C struct as a worker value needs the FFI route to host tasks, which it does not (`W48-QR-FFI-ROUTE-SCOPE`), so it is registered (`W48-QR-TASK-FFI-STRUCT-WORKER`, L) with the unsafe-FFI assumption as its condition; `W48-QR-OWNED-RECORD-ACROSS-CALL` did not become small — the leaf machinery would carry over, but `source.resource-calls.v1` has its own byte representation and statement set (amended, still L) |

## Corrections

| What | Before | After | Why |
|---|---|---|---|
| Card 2's survey, task row (`usable_records_survey.rs` pins; the ten `task_*` cells) | all `Q500` (`AGGREGATE-TASK-CAPTURE`) | construct, read, copy, nested, param, lend admitted; `result` `Q050` (a whole record rendered); `write` `Q300` (no `mut`); `move` `Q500` (`bytes`, `WRITE-LEND-TASKS-EXTENT`); `loop` `Q500` (the route's `for`) | the lift; card 2's archived survey JSON is frozen and not edited |
| `owned_field_route_reach.rs`'s named row | both records refused, naming `AGGREGATE-TASK-CAPTURE` | a record of scalars admitted; a `bytes` field refused at `Q500` naming `WRITE-LEND-TASKS-EXTENT` | moved by name, not deleted |
| "at most one write-leased resource at a time" | global | per resource | two fields leased at once; no tracked program moves |
| `WorkerExit::returned`, `AbstractSlot::Handle::returned`, `State::exit` | one optional identity | a list | a record result hands back one identity per owned leaf |
| `TaskModule` | no records | `records` (empty for every earlier module) | the record result type |
| reference rendering of a task result | inline in `render_task_event` | factored into `render_result`, byte for byte | a record's leaves reuse the arms |

## Evidence

**Negative controls** (`compiler/tests/task_record_captures.rs`, run first, one
test each, in the brief's order): a record with an owned field captured by
`copy` — `Q600` naming the leaf `p.buf` and the `move` contract; a moved field
used before refill — `Q600` (the checker, on that path; the same program
refilling first is admitted); two lends of one field that overlap —
`Q600 OverlappingLeases` (disjoint ranges admitted); a read share and a write
lease of one leaf — `Q600`; a record into a parameter of another record type —
`Q600`, "record types are nominal"; a record moved while a leaf is lent —
`Q600 LoanOutstanding`; spawn failure with a moved record — the failure arm
reads `p.a` and must release `p.buf` (forgetting it is `Q600`), while on the
`Ok` arm `p` is moved (`Q600`); a `write p` capture — `Q600`, refused by name.
Beside them: `move` of a record of scalars and `read p share N` of a whole
record (`Q600`), a field write without `mut` (`Q300`), a refill over a live
field (`Q600 OverwriteOfLiveField`), `return p;` of a record that owns (`Q600`),
a `bytes` field (`Q500`), a record used after moving into another record, an
unsettled record (`Q600`, `W48-QR-TASK-RECORD-ARM-MERGE`), a whole move of an
owning sub-record (`Q600`), `hex` of a `bool` field and a whole record rendered
(`Q050`). Admissions check the IR handed to the core: a record parameter's leaf
types, the leaf operands of a record capture, the `UnpackRecord` after `join`.
The judgment's Rust rows (`crates/qcore-core/tests/unit/transfer.rs`) re-decide
the aggregate over all 307 leaf lists of length ≤ 2 from the whole kind
vocabulary, every mode, same and other record types.

**Fixture identity.** Card 0's `freeze_fixtures.py` over every tracked
`.qr`/`.qv`/`.qrlib` file, debug `quirin` at the base (`e72a538d…`, 694 files)
and at the tip (`0f452fe0…`, 697 files) ([BEFORE](artifacts/fixtures-before.json),
[AFTER](artifacts/fixtures-after.json), [comparison](artifacts/fixtures-compare.txt)):
**684 of 694 existing files byte-identical** in exit, stdout and stderr of
`check` and `emit`; the **10** that differ are card 2's `task_*` survey cells,
attributed row by row above; the 3 new entries are the examples. No F02-pinned
trace moved (no admitted program before the card returns a record).

**Native on real threads** ([summary](artifacts/native-task-records.json)):
[`record_workers.qr`](../../../examples/threads/record_workers.qr) (a nested
record owning a `Buffer<u32>` moved to a worker, written, returned whole),
[`record_field_leases.qr`](../../../examples/threads/record_field_leases.qr)
(two fields leased to two workers at once) and
[`record_spawn_failure.qr`](../../../examples/threads/record_spawn_failure.qr)
(the spawn-failure control, run a second time with every creation refused
through the runtime's `QCORE_TASK_CREATE` seam) — Clang 21.1.8,
`-std=c11 -pthread -Wall -Wextra -Werror -pedantic`, at `-O0` and `-O2` under
`-fsanitize=undefined,integer -fno-sanitize-recover=all` and at plain `-O2`:
**12 runs**, exit 0, both streams equal to the reference, **0** reports; and
under **ThreadSanitizer** through the existing pthread binding
(`TASK-ROUTE-TSAN-PTHREAD-BINDING`): each example unforced, and the lease
example forced with both workers live together and forced in reverse, each
forced row asserting both workers ran — **5 rows**, clean, **0** races, stdout
equal to the reference. The rendering row (a worker logging three fields, the
parent printing fields beside the trace) runs the same three builds, both
streams equal to the reference and stderr inside the declared reordering
relation. The page transcripts are read out of `examples/threads/README.md`
and compared with the reference.

**Lean** ([audit](../../../proof/AuditTaskRecordCaptures.lean), no placeholder,
no project axiom): in `QCore.Transferability` — the aggregate judgment is the
conjunction of its leaves (`theAggregateJudgmentIsTheConjunctionOfItsLeaves`),
its boundary decided exactly once over every leaf list and every mode
(`theAggregateJudgmentIsDecidedExactlyOnce`, `theAggregateBoundaryIsDecidedExactlyOnce`)
and over the union of the two vocabularies
(`theBoundaryIsDecidedExactlyOnceOverRecords`), a `write` of any record or any
kind refused by name (`aWriteCaptureOfARecordIsRefusedByName`,
`aWriteCaptureIsRefusedByNameOverRecords`), nominal record types
(`aRecordIntoAnotherRecordTypeIsRefused`), every admitted aggregate transfer
backed by the flat rule (`everyAdmittedAggregateTransferIsBacked`) — the flat
`theBoundaryIsDecidedExactlyOnce` and `aWriteCaptureIsRefusedByName` kept
unchanged beside them; in the new `QCore.TaskRecordCaptures` — custody
conserved across spawn and join (`custody_is_conserved_across_spawn_and_join_for_records`),
unpacking places every carried identity once
(`unpacking_places_every_carried_identity_once`), spawn failure restores the
record exactly (`spawn_failure_restores_the_record_exactly`), no byte has two
write authorities with per-leaf lends (`no_byte_has_two_write_authorities_per_leaf`)
and two fields leased at once is reachable (`two_fields_leased_at_once_is_reachable`),
a refill admitted exactly into a hole (`a_leaf_refill_is_admitted_exactly_into_a_hole`),
and a record crossing exactly when every leaf may
(`a_record_crosses_exactly_when_every_leaf_may_cross`). The models are
transcriptions of the Rust, bound to it by the test rows, not by extraction.

## Prediction and outcome

| Measure | Predicted | Outcome |
|---|---|---|
| fixture freeze | byte-identical except the ten `task_*` survey cells, with the listed movements, 70% | **as predicted**, cell for cell: 684 / 694 identical, the ten cells moved exactly as listed |
| existing tests changed | 2–5 files, 60% | **2** pinned-outcome files (the survey table, the route-reach row) plus mechanical `records:` fields in two core test files |
| negative controls | each refused, the restoration admitted and observed, 85% | **as predicted** |
| native incl. TSan | clean, 60% | **clean**: 12 sanitizer runs, 5 TSan rows, rendering row × 3, 0 reports |
| Lean | one module, ~15–25 theorems, plus the judgment re-decided, 70% | one module (**12** theorems) and **13** judgment theorems in `QCore.Transferability` |
| corpus arbiter | identical to the base's archive, 95% | ARBITER-OUTCOME |
| standing gate first run | green, 45% | GATE-OUTCOME |
| items | 1 (70%), 2 (60%), 3 (65%), 4 (55%), 5 (15% / 10%) | **1–4 done; 5 not done** |

Not predicted: the per-resource lift of the one-leased-resource bound was the
price of item 2's "disjoint lends of two fields to two workers"; the task
route's `?` spawn form cannot carry a record that owns (its failure edge exits
holding the restored Buffer — the existing rule), so every owning example uses
`match`; and the worker's error edge makes any raising statement impossible
while it holds a record that owns (the existing exit rule, per leaf).

## Gate

GATE-TABLE

## Corpus arbiter

ARBITER-TEXT

## Follow-ups

Ledger ([followups.json](followups.json), applied by the idempotent
[`apply_ledger.py`](apply_ledger.py); a second run changes nothing): retires
`AGGREGATE-TASK-CAPTURE` on its own discharge condition; amends
`W48-QR-FFI-ROUTE-SCOPE`, `W48-QR-OWNED-RECORD-ACROSS-CALL`,
`W48-QR-NESTED-OWNED-SUBRECORD-MOVE`, `W48-QR-RECORD-WHOLE-MOVE`,
`W48-QR-RECORD-FIELD-WHOLE-WRITE`, `WRITE-LEND-TASKS-EXTENT` and
`W48-QR-TYPES-MSVC-NATIVE`; registers `W48-QR-TASK-RECORD-ARM-MERGE` (S),
`W48-QR-TASK-RECORD-EPOCH` (M), `W48-QR-TASK-RECORD-F02` (S/M),
`W48-QR-TASK-FFI-STRUCT-WORKER` (L), `W48-QR-TASK-RECORD-BYTE-ACCESS` (S) and
`W48-QR-TASK-LEAF-SHARE-BOUNDS` (S): 637 → 642.

## Files likely to conflict with card 4

`crates/qcore-frontend/src/source_tasks.rs` (`Shape`, `Signature`, the
builder's record environment, `spawn_inputs`, `result_match`, `propagate`,
the statement arms) and the new `source_tasks/records.rs` — card 4's closures
with capture lists lower to these captures; `crates/qcore-core/src/transfer.rs`
(`leaf_mode`, `admits_aggregate`, `AggregateRefusal`) and
`proof/QCore/Transferability.lean`; `crates/qcore-core/src/task_execution.rs`
(`ScalarType::Record`, `ScalarOperand::Record`, `UnpackRecord`, `RefillField`,
`WorkerExit::returned`), `task_execution/reference.rs`, `task_c_emit.rs`
(`record_runtime`, `result_tail`), `scoped_tasks.rs` (`TaskOutcome::Record`);
implemented grammars §G02a/§G02j/§G07.13; specification §L16/§L21;
`examples/threads/README.md`.

## Scripts

* Card 0's [`freeze_fixtures.py`](../w48-qr-scalar-types/freeze_fixtures.py)
  and [`compare_fixtures.py`](../w48-qr-scalar-types/compare_fixtures.py),
  unchanged.
* [`apply_ledger.py`](apply_ledger.py) — applies `followups.json`, idempotently.
* The native summary: `QCORE_W48_TASK_RECORDS_NATIVE_SUMMARY=<absolute path>
  cargo test -p qcore-compiler --test task_record_captures
  the_published_examples_run_clean_under_three_builds`.
