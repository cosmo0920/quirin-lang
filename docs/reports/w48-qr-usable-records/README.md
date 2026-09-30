# W48-QR-USABLE-RECORDS: records usable as values

Card 2 of the `.qr` language track (base `ca634dc1` = local `main` =
origin/main; the worktree's HEAD was already that commit, so no reset or
fast-forward was needed). Records become usable as values for `.qr`-declared
records (§G07.13) and imported C structs (card 1): they are **written**, they
**nest**, and they **cross `.qr` calls**. Grammar: implemented grammars
[§G07.13](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g0713-the-first-source-level-aggregate-sourcerecordv1),
[§G02s](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02s-scalar-functions-and-explicit-output-sourcescalar-callsv1)
and [§G02u](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02u-imported-c-structs-sourceffiv1);
semantics: specification §L14.6 and §L21; rule: constraint 15k's amendment in
[buffers-records](../../agents/buffers-records.md#15k--appended-card-w48-qr-usable-records-wave-48).
The [prediction](PREDICTION.md) was frozen before any source change, after the
BEFORE fixture freeze and the Step-0 survey.

## Step 0: the survey

One fixture per (route, operation) cell,
`compiler/tests/fixtures/usable_records/survey/`, written by
[`make_survey.py`](make_survey.py) before any source change and pinned by
`compiler/tests/usable_records_survey.rs`, which checks each through the
library API with what its route needs (the extern cells with
`survey_scale.qv` selected, the FFI cells with the recorded target of the
running Clang). `ok` is admitted, a code is the first refusal, `BASE → TIP`
where the card moved it ([base](artifacts/survey-base.json),
[tip](artifacts/survey-tip.json), [`survey_table.py`](survey_table.py)):

| Operation | buffer | scalar | extern | task | ffi |
|---|---|---|---|---|---|
| construct | ok | Q051 → **ok** | Q025 | Q500 | ok |
| read | ok | Q051 → **ok** | Q025 | Q500 | ok |
| write | Q051 → **ok** | Q051 → **Q020** | Q025 | Q500 | Q026 → **ok** |
| write_owned | Q051 → **ok** | — | — | — | — |
| write_released | Q051 → **Q054** | — | — | — | — |
| nested | Q051 → **ok** | Q051 → **ok** | Q025 | Q500 | Q051 |
| param | Q051 → **ok** | Q051 → **ok** | Q026 | Q500 | Q026 → **ok** |
| result | Q051 → **ok** | Q051 → **ok** | Q026 | Q500 | Q026 → **ok** |
| copy | ok | Q051 → **ok** | Q025 | Q500 | Q026 → **ok** |
| move | Q054 | Q051 → **Q020** | ok | Q500 | Q051 |
| loop | ok | Q051 → **Q020** | Q026 | Q500 | Q026 |
| loop_owned | Q051 | — | — | — | — |
| lend | ok | Q051 | Q025 | Q500 | Q026 |
| lend_owned | Q051 | — | — | — | — |
| view_field | Q051 | — | — | — | — |

Reading it:

* **What worked at the base**: construct, read, copy of a record of scalars,
  a record in a loop body and beside a lend on `source.buffer.v1`; construct
  and read on `source.ffi.v1`. A record never reached `source.scalar-calls.v1`
  (a record declaration selected the buffer route, which models its entry
  only), the task route refuses every record declaration (`Q500`,
  `AGGREGATE-TASK-CAPTURE`, card 3), and the extern route refuses every
  record statement.
* **What was missing**, and so the scope: field writes (buffer, FFI), nesting
  (buffer), a record as a parameter or result (anywhere), and FFI records
  beyond one flat entry.
* The buffer `param`/`result` fixtures hold no buffer, so at the tip they
  reach `source.scalar-calls.v1` (below); `write_released` moves from "not a
  statement" to the IR's use after drop. `scalar write` is refused by name
  (`W48-QR-SCALAR-CALLS-FIELD-WRITE`), `scalar move` (a `bytes` field across a
  call) by name (`W48-QR-OWNED-RECORD-ACROSS-CALL`), and `scalar loop` because
  that route has no `for` — unchanged vocabulary, now reached.
* **A finding beside it**: `extern_move` is admitted at the base and the tip:
  the extern route admits a module whose record declaration nothing reads
  (no code, no custody), recorded as `W48-QR-EXTERN-ROUTE-UNREAD-RECORD-DECLARATION`.
  A second finding is a correction below (a read of a released record).

The two `write` fixtures gained `mut` in their effect rows after the base
survey (a write derives `mut`); both were re-measured at the base with the
base binary and refuse with the same codes ([record](artifacts/survey-base-remeasure.txt)).

## The mechanism

**The owner path, generalised.** `ownership::FieldPath` holds a non-empty path
(`o.inner.buf`) instead of one field — the same type, so a one-level path is
the path it always was, with the same order and spelling, and every leak list
written before the card is unchanged. An aggregate binding lists its owned
**leaves**, depth-first in declaration order. Three IR operations are new:
`ComposeNested` (construction with a record part: every leaf of the moved
aggregate is re-keyed under the field, keeping its state; `Compose` is the same
code with buffer parts only), `RefillField` (a buffer moved into an owned leaf
that is a hole; over a live field, `OverwriteOfLiveField`), and `AccessRecord`
(a scalar field access asks that the aggregate is live). The drop-through walks
the leaves. All custody decisions stay in the IR (15h's precedent).

**`source.buffer.v1`.** `FieldType::Record(name)`; the declaration graph is
checked acyclic, at most 8 deep and 256 leaves (`admit_graph`). Values are
trees (`FieldValue::Record`); `project_path` and `write_path` read and replace
one field. Every field-path event carries a path. `RECORD.PATH = VALUE;`
(`BufferEvent::FieldWrite`) writes a scalar field with the construction's own
typing, derives `Mut`, and — at an owned field — is the refill. The C is a
member assignment through `r_a.f_b.f_c`; nested structs are defined before
their holders.

**`source.scalar-calls.v1`.** `Type::Record(id)`, `RecordDecl`,
`Expr::Construct` (fields by name, ordered and checked total by the checker)
and `Expr::Field` (a path of names, resolved against the operand's type). The
evaluator carries a record as its **leaves** (`Program::field_slice` gives a
field's slice), so a call copies it in and a `return` copies it out. Route
selection: a module that declares a record, holds no buffer and declares a
second function reaches this route — every such module was refused before.
The C passes and returns `struct qcore_record_NAME` by value.

**`source.ffi.v1`.** `FfiStatement::Write` (listed fields only, typed, `Mut`),
`FfiStatement::Copy`, `Return(Option<value>)`, and `FfiFunction`: a `.qr`
helper taking and returning scalars and imported records by value, checked at
**every call** with the call's values (the entry's statement rules, the graph
acyclic and at most 16 deep, every helper reached from the entry, an absent
row inferred). The certificate lists the helpers' signatures and every write.
The driver refuses a function in a file of the bundle that does not itself
import `std.ffi.unsafe` (`Q032`). Every result is still an `AssumedVerdict`
under the unsafe-FFI assumption.

## Items

| Item | Status |
|---|---|
| 1. Field writes | **done** on `source.buffer.v1` (scalar fields typed; owned fields by refill into a hole; a released record's fields a use after drop) and `source.ffi.v1` (listed fields only). On `source.scalar-calls.v1` a write refuses by name (`W48-QR-SCALAR-CALLS-FIELD-WRITE`): its rows cannot state `mut`. |
| 2. Nested records | **done** on `source.buffer.v1` with the owner path generalised; `AGGREGATE-NESTED` retired. Nesting an imported struct stays `W48-QR-FFI-NESTED-AND-ARRAY-FIELDS` (the C layout reader), a whole sub-record write `W48-QR-RECORD-FIELD-WHOLE-WRITE`, a whole read of a sub-record that owns `W48-QR-NESTED-OWNED-SUBRECORD-MOVE`. |
| 3. Records across calls | **done for every record without an owned field**: by value on `source.scalar-calls.v1` (nested records included). A record holding `bytes` across a call — the C04-custody half — is **not done** and stays refused by name: `W48-QR-OWNED-RECORD-ACROSS-CALL`, size L (it needs `source.resource-calls.v1`, whose byte representation and statement set carry no record, extended with an aggregate move and per-leaf extents). |
| 4. FFI records beyond one flat entry | **done**: writes, copies, `.qr` helpers by value, the file-mark gate; `W48-QR-FFI-ROUTE-SCOPE` amended (the rest — library mode, branches, loops, buffers, tasks in an FFI file — stays, size M). |
| 5. Stretch | **not done**: `AGGREGATE-VIEW-FIELD`, `OWNED-FIELD-LOOP-BODY` and `OWNED-FIELD-LEND-WINDOW` keep their refusals (the latter two now asked of a record that owns through a nested record too). |

## Corrections

| What | Before | After | Why |
|---|---|---|---|
| A scalar field **read** of a record with an owned field after `drop f;` | admitted | `Q054` use after drop | the same live-record rule the write needs; the refusing direction, and no tracked fixture had the shape (the freeze) |
| `aggregate_owned_field.rs`'s pinned operation list | no operation for `let t = f.tag;` | one `AccessRecord(f)` | the rule above |
| `owned_field_route_reach.rs`'s nested row | `AGGREGATE-NESTED` refused by name | admitted in both declaration orders; a self-containing record refused | the lift |
| `source_record.rs`: a record construct in a helper | the buffer route's "models the entry only" | checked by `source.scalar-calls.v1` (a helper with no `return` refused there, one with it admitted) | the route-selection change; nothing is admitted unread |
| `unread_annotations.rs`: `records/point.qr` with an appended helper | an entry-only row (`Q051`) | a new test: the helper is checked (a valid one admitted, each ill-typed one `Q020`) | the same |
| `RecordRefusal::NestedFieldType` | the `AGGREGATE-NESTED` refusal | removed; `RecursiveFieldType`, `NestingTooDeep`, `TooManyLeaves`, `PathThroughNonRecord`, `FieldTakesARecord`, `WriteOfRecordField`, `SubRecordOwns` added | the lift and its boundaries |
| `RecordRefusal::FieldIsNotABuffer.declared` | a `ScalarType` | the declared type's spelling | a record field is not a buffer either |
| `FieldType` | `Copy` | `Clone` (it holds a record name) | the record field |
| `FfiStatement::Return` | a unit variant | `Return(Option<FfiValue>)` | a helper returns a value |
| Survey fixtures `buffer_write`, `ffi_write` | `effects { io }` | `effects { io, mut }` | a write derives `mut`; re-measured at the base, same codes |

## Evidence

**Negative controls** (`compiler/tests/usable_records.rs`, one row each, run
first in the file): a write of the wrong type (`Q052`), out of range (`Q053`),
a `bool` at an integer field (`Q053`); a write and a read through a released
record (`Q054`); a refill over a live owned field (`Q054`); a path through a
non-record (`Q052`) and an undeclared path component (`Q052`); a record field
given another record or a literal (`Q052`); a self-containing record (`Q051`);
a whole sub-record write and a whole read of a sub-record that owns (`Q051`);
nested ownership kept under `OWNED-FIELD-LEND-WINDOW` and
`OWNED-FIELD-LOOP-BODY`; a moved sub-record used again and a nested leaf
leaked (`Q054`, named `o.inner.buf`). A record parameter whose custody mode is
not by value (`own<P>`, `borrow<P>`, `ptr<const, P>`, `move p`), a record that
owns crossing a call, another record type where one is taken, a record in
arithmetic or a line, an incomplete construction, a field write, and a record
result of a throwing function (`Q020`). On the FFI route: a write out of range
(`Q053`), of another type (`Q052`), of the opaque `note` and of a non-member
(`Q051`), a write without `mut` (`Q029`); a helper nothing calls, a helper
calling itself, a pointer parameter (`Q026`), a declared outcome on a helper
call (`Q025`); and **an FFI record reaching a non-FFI file** — a bundle whose
entry file does not import `std.ffi.unsafe` — refused (`Q032`). The IR's unit
tests add six rows (`crates/qcore-core/tests/unit/ownership.rs`).

**Fixture identity.** Card 0's `freeze_fixtures.py` over every tracked
`.qr`/`.qv`/`.qrlib` file, debug `quirin` at the base (`05539493…`, 634 files)
and at the tip (`cc2cb554…`, 694 files) ([BEFORE](artifacts/fixtures-before.json),
[AFTER](artifacts/fixtures-after.json), [comparison](artifacts/fixtures-compare.txt)):
**all 634 existing files are byte-identical** in exit, stdout and stderr of
`check` and `emit`; the 60 differing entries are the new files only (55 survey
cells, the survey contract, three examples and one contract). No existing
refusal became an admission or the reverse: the route-selection change reaches
no tracked program.

**Native** ([summary](artifacts/native-records.json)): the three published
examples — [`records/nested_writes.qr`](../../../examples/records/nested_writes.qr),
[`scalar/record_calls.qr`](../../../examples/scalar/record_calls.qr) and
[`ffi/record_passing.qr`](../../../examples/ffi/record_passing.qr), the last
linked with the real C library [`qr_point.c`](../../../examples/ffi/qr_point.c)
— each built by Clang 21.1.8 with `-std=c11 -Wall -Wextra -Werror -pedantic` at
`-O0` and `-O2` under `-fsanitize=undefined,integer -fno-sanitize-recover=all`
and at plain `-O2`: exit 0, both streams equal to the reference observation,
**0** sanitizer reports, in all nine runs. The nested example also runs clean
under ASan/LSan (the refilled buffer and the moved-out one each released once).
Each page's transcript is compared with the reference observation.

**Lean** (`QCore.UsableRecords`, [audit](../../../proof/AuditUsableRecords.lean),
26 theorems, no placeholder, no project axiom): a field write changes exactly
that field (`write_changes_exactly_that_field`, `write_leaves_every_other_path`);
a nested path addresses exactly one sub-object
(`a_nested_path_addresses_exactly_one_sub_object`,
`a_path_through_a_leaf_addresses_nothing`, `sibling_slices_are_disjoint`, and
over the IR's leaf list `compose_keeps_paths_distinct`); the nested
composition conserves custody and the drop-through releases exactly the held
leaves (`nested_composition_conserves_custody`,
`drop_through_tree_releases_exactly_the_live_leaves`); the refill is admitted
exactly into a hole (`a_refill_is_admitted_exactly_into_a_hole`); custody is
conserved across a record-passing call (`a_record_passing_call_conserves_custody`,
`a_record_that_owns_is_never_passed`); an FFI record never leaves an FFI file
without the assumption (`an_admitted_ffi_bundle_marks_every_function_file`,
`every_ffi_bundle_result_carries_the_assumption`, reusing
`QCore.FfiStructImport`); and non-vacuity rows (`the_sample_write_is_seen_only_at_its_path`,
`an_unmarked_function_file_is_refused`). The Lean models are transcriptions of
the Rust (a tree, a leaf list, a slice layout), bound to it by the test rows,
not by extraction.

## Prediction and outcome

| Measure | Predicted | Outcome |
|---|---|---|
| fixture freeze AFTER vs BEFORE | byte-identical but for new files, 75% | **634 / 634 byte-identical**; the 60 entries are new files |
| existing tests changed | 3–8 rows pinning a lifted refusal, 60% | **5**: the aggregate operation list (a correction), the nested row, the entry-only record row, the `point.qr` entry-only row, and mechanical `field` → `path` constructions in three test files |
| survey at the tip | write, nested, param/result admitted on buffer/scalar/ffi; task and extern unchanged, 70% | as predicted, with `scalar write` refused by name rather than admitted |
| negative controls | each refused, 85% | **each refused**, each a test row |
| native runs incl. a real C library | clean, 0 reports, 75% | **clean**, 9 runs, 0 reports |
| Lean | one module, ~15–25 theorems, 75% | one module, **26** theorems |
| corpus arbiter | identical to card 1's AFTER `c012e785…` (586 / 10 / 4,238), 95% | **identical**: `c012e785…`, 586 / 10 / 4,238, full hash, sidecars identical |
| standing gate first run | green, 50% | every gate step 0 on the first run; the Python suite afterwards failed 3 of 365 (two the matrix not yet regenerated, one constraint 29's tripwire on the new survey test's directory walk) |
| items | 1 (85%), 2 (60%), 3 by value (55%) / with owned fields (25%), 4 (55%), 5 (15%) | **1, 2, 4 done; 3 done by value, not with owned fields; 5 not done** |

Not predicted: the read-after-release correction (found by the survey's own
write row), the FFI file-mark gap (a non-FFI file of a bundle could already
use an imported record at the base), and that the scalar-calls route's rows
cannot state `mut`.

## Gate

[`gate.json`](gate.json), exits in [`gate/`](gate/), logs private to the
worktree (`private/w48-c2/gate`), run at `0f486097` by `private/w48-c2/gate.sh`
(each exit to a file, no step piped):

| Step | Exit | Count |
|---|---|---|
| submodule (`--init --recursive`) | 0 | |
| `cargo fmt --all -- --check` | 0 | |
| `scripts/run_test_binaries.py` | 0 | 387 units, 400 summaries: 6,809 passed / 0 failed / 11 ignored |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| `scripts/check_lean_audits.py` | 0 | 245 audits (244 + `AuditUsableRecords.lean`) |
| provider suite, `--no-fail-fast` (providers unchanged, run anyway) | 0 | 129 passed / 0 failed / 1 ignored, 43 summaries |
| matrix (`build_matrix.py --gate gate.json`) | 0 | `named_followups` 637 |
| docs index | 0 | |
| Python (`unittest discover -s scripts/tests`) | 0 | 365 tests |
| Clang | present | Ubuntu clang 21.1.8 ([record](gate/clang-version.txt)) |

The first Python run after the gate failed 3 of 365: two rows were the matrix
not yet regenerated for the ledger, and the third was constraint 29's
tripwire, which found a `read_dir` walk in this card's new survey test. The
survey now lists its cells with the tracked-file helper (an untracked decoy
cell left in the directory is ignored, measured once); that test, fmt and
clippy of that target were rerun, each 0, before the matrix, docs index and
Python ran.

## Corpus arbiter

[Identity record](artifacts/arbiter-corpus.identity). `scripts/run_pinned_audit.py`
at pin `66910c10` over the recorded Cycle-19 inputs, `--jobs 8`, release
`quirin-audit` (`72288e2f…`) built at `029c7019`, the last commit that changes
a crate: exit 0; `resource-diagnostics.json` `c012e78506cdb4ca…`, **byte-identical**
to the base archive (W48-QR-C-STRUCT-IMPORT's AFTER), so 586 Safe / 10 Unsafe /
4,238 Unknown; `resource-diagnostics.chunk-contracts.json` and
`monkey-source.json` byte-identical to every archive since card 0. No C
analysis path changed: the card's code is the `.qr` routes, the ownership
IR's path generalisation and the driver's `.qr` route selection.

## Follow-ups

Ledger ([followups.json](followups.json), applied by the idempotent
[`apply_ledger.py`](apply_ledger.py); a second run changes nothing): retires
`AGGREGATE-NESTED` on its own discharge condition; amends
`W48-QR-FFI-ROUTE-SCOPE`, `W48-QR-FFI-MUT-POINTER` and
`W48-QR-TYPES-MSVC-NATIVE`; registers `W48-QR-OWNED-RECORD-ACROSS-CALL` (L),
`W48-QR-RECORD-WHOLE-MOVE` (S–M), `W48-QR-NESTED-OWNED-SUBRECORD-MOVE` (S–M),
`W48-QR-RECORD-FIELD-WHOLE-WRITE` (S/M), `W48-QR-SCALAR-CALLS-FIELD-WRITE` (S),
`W48-QR-RECORD-THROWS-RESULT` (S) and
`W48-QR-EXTERN-ROUTE-UNREAD-RECORD-DECLARATION` (S): 631 → 637. Still open and
untouched: `AGGREGATE-VIEW-FIELD`, `OWNED-FIELD-LOOP-BODY`,
`OWNED-FIELD-LEND-WINDOW`, `AGGREGATE-TASK-CAPTURE` (card 3),
`W48-QR-FFI-NESTED-AND-ARRAY-FIELDS`.

## Files likely to conflict with cards 3–4

`crates/qcore-core/src/ownership.rs` (`FieldPath`, `ComposeNested`,
`RefillField`, `AccessRecord` — card 3's whole-record move and task captures
extend these); `crates/qcore-core/src/source_record.rs` (`FieldType::Record`,
`owned_leaves`, `field_type_at`, `admit_graph`); `crates/qcore-core/src/std_buffer.rs`
and `std_buffer_c_emit.rs` (path-carrying field events, `FieldWrite`);
`crates/qcore-core/src/scalar_calls.rs` and `scalar_calls_c_emit.rs`
(`Type::Record`, `Expr::Construct`/`Field`, the leaf evaluator — card 4's
closures over records meet these); `crates/qcore-frontend/src/source_scalar_calls.rs`
(record parsing); `crates/qcore-core/src/ffi.rs`, `ffi_c_emit.rs` and
`crates/qcore-frontend/src/source_ffi.rs` (`FfiFunction`, writes, copies);
`crates/qcore-verification/src/driver.rs` (the scalar-calls route selection for
record modules, the FFI file-mark gate); implemented grammars §G07.13, §G02s,
§G02u; specification §L14.6/§L21; `docs/agents/buffers-records.md`
(15k amendment); `proof/QCore.lean`.

## Scripts

* [`make_survey.py`](make_survey.py) — writes the survey fixtures; `--check`
  fails on drift.
* [`survey_table.py`](survey_table.py) — renders the survey table above.
* Card 0's `freeze_fixtures.py` and `compare_fixtures.py`, unchanged.
* [`apply_ledger.py`](apply_ledger.py) — applies `followups.json`, idempotently.
* The survey: `QCORE_W48_RECORDS_SURVEY=<absolute path> cargo test -p
  qcore-compiler --test usable_records_survey`; the native summary:
  `QCORE_W48_RECORDS_NATIVE_SUMMARY=<absolute path> cargo test -p
  qcore-compiler --test usable_records the_published_examples_run_clean_under_three_builds`.
