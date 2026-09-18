# `AGGREGATE-OWNED-FIELD` (wave 39) — an owned buffer inside a record: the IR first, then the route

**Card:** `AGGREGATE-OWNED-FIELD` · **Wave:** 39 · **Base:** `e878b362`
(`origin/main` at branch time, confirmed equal) · **Date:** 2026-09-18.

**Headline.** The cycle-3 ownership IR now addresses an owner by a **path**
(`Place::Field`, `NAME.FIELD`, one level), and the four rules standing
constraint 15k had recorded as missing are landed **in the IR**: construction
moves in, a field read of an owned field is the partial move and leaves a hole,
`drop RECORD;` releases every live owned field in declaration order and nothing
twice, and a leak at normal exit names the path. Lean parity is
`QCore.AggregateOwnedField` (25 theorems, audited). The route then admits a
record field declared **`bytes`**, a construction that moves an acquired buffer
in, `RECORD.FIELD` at every custody position a buffer name stands at, the move
out, and the whole-record drop; emission holds the pointer as a struct member
and the initializer *is* the move; the published example runs clean under
ASan/UBSan/LSan. **No word reserved, no code allocated; `Q055` is narrowed
(the one lift); every one of the 286 previously admitted programs in the tree
builds the operation list it always built and the 139 that emit C emit it byte
for byte. `AGGREGATE-OWNED-FIELD` is retired by exact name and six residuals
are registered.**

Sections: 1 what was true · 2 the IR · 3 byte identity · 4 Lean · 5 the route
(admitted/refused table) · 6 emission · 7 native · 8 keyword census ·
9 falsification · 10 gate · 11 residuals and retirements · 12 for the
integrator · 13 file ownership.

---

## 1. What was true at the base, re-derived

Read from `e878b362`, not inherited from the brief:

* `crates/qcore-core/src/ownership.rs`: every owner-addressing field of the IR
  was a plain `String` — `Read(String)`, `Write(String)`, `Drop(String)`,
  `Move { from, to }`, `bindings: BTreeMap<String, Binding>`,
  `ResourceEntry.owner: String`, `NormalPathLeak(Vec<String>)`. `p.buf` had
  nothing to be spelled into. There was no aggregate binding kind.
* `crates/qcore-core/src/std_buffer.rs`: the custody walk's record arm was an
  explicit no-op (`Construct | Field | Bind | Line | Return => {}`), and
  `derived_effects` said "a record operation derives nothing … an owned field
  would make the first of those false, which is one more reason it is a
  separate card". `FieldType` had one constructor, `I32`.
* `Q055` (`BufferError::CrossesBoundary`) refused a buffer at a field position:
  `a_buffer_at_a_field_position_is_refused_by_the_existing_rule` pinned it.
* The tracker held `AGGREGATE-OWNED-FIELD` (registered by the wave-38 close,
  `recorded_by: WAVE-38-CLOSE`) with the discharge "the four rules … landed in
  the IR first (an IR card before a route card), with `Q055`'s lift stated
  deliberately and constraint 15k amended in the same change; … allocates no
  code". `AGGREGATE-VIEW-FIELD` was named in 15k and registered nowhere.

## 2. Owner paths in the IR — the four rules, landed in `ownership.rs`

### 2.1 The vocabulary

```rust
pub struct FieldPath { pub base: String, pub field: String }   // `BASE.FIELD`
pub enum Place { Name(String), Field(FieldPath) }
```

`bindings` is keyed by `Place` and `ResourceEntry.owner` is a `Place`. **Every
existing `OwnershipOp` variant is unchanged**; a `String` owner in one is
always `Place::Name`, so a plain owner spelled `"p.buf"` and the field `buf` of
`p` are different keys (`a_field_path_and_a_dotted_plain_name_are_different_places`).
One level was measured sufficient: the record route admits no record-typed
field, and CT09's scenario (the one CT case the record family maps) is flat;
the step beyond is `AGGREGATE-NESTED` (§11).

### 2.2 The five operations and the four rules

| Operation | Rule | Refusal it carries |
|---|---|---|
| `Compose { owner, fields: [(field, from)] }` | **construction moves in**: each `from` is `require_owned`, must be unborrowed, its ledger owner becomes `Field(owner.field)`, `from` becomes `Moved`; the aggregate binding records its owned fields in the order given (declaration order, from the route) | `UseAfterMove(from)` on a later use of `from`, and at the second of two fields naming one source; `BindingAlreadyExists("p.buf")` for a field composed twice; `MoveWhileBorrowed` |
| `MoveOut { from: path, to }` | **partial move**: the field's resource moves to the plain `to`; the field is `Moved` — a hole | `UseAfterMove("p.buf")` on a second read or move; `BindingAlreadyExists(to)` |
| `Drop(name)` on an aggregate | **drop-through**: every field is checked for a live loan **before** any release; then every `Owned` field is released **in declaration order**, `Moved`/`Dropped` fields (holes) are tolerated, and the aggregate becomes `Dropped` | `DropWhileBorrowed(path)`; `DoubleDrop(name)` on a second whole drop; `UseAfterDrop(path)` on a field access afterwards |
| `check_ownership` at exit | **leak that sees into the aggregate**: `owned_places()` spells every `Owned` binding, plain names first, then `BASE.FIELD` paths | `NormalPathLeak(["z", "p.buf"])` |
| `Copy(name)` on an aggregate | **copy rule**: refused by type, hollow or not | `CopyOfOwned(name)` |
| `ReadField` / `WriteField` / `DropField` | `Read`/`Write`/`Drop` at a path; `DropField` leaves a hole | the plain arms' refusals, spelled with the path |

The record itself is never a plain owner: a `Read`, `Write`, `Move`,
`TransferOut`, `BorrowShared`/`BorrowMutable`, `LendRange`, `WriteRange` or
`ReadRange` naming it is `AggregateAtOwnerPosition` (new variant); a field path
whose base has no fields is `NotAnAggregate` (new variant). Both `Display`
strings say why. `validate_invariant` gained one clause: every declared owned
field of an aggregate has a field binding beside it (owned, moved or dropped),
and an aggregate is never itself a field.

**Nothing twice, in Rust.** `drop_through` collects the live fields, refuses on
any loan before releasing, then releases each collected resource once and
marks it `Dropped`; the invariant check after every step makes a second
release of the same identity a `LedgerInvariantViolation` even if an arm tried
it. The **declaration order** is a specification the IR follows and the value
half and the emitter publish; in the IR alone it is observable only through a
partial run (`DropField` then `Drop`) since releases commute on the ledger —
stated rather than pretended (§9, M2/M3 pin it where it is observable).

### 2.3 Tests (`crates/qcore-core/tests/unit/ownership.rs`, 12 new, 34 total in the module)

`composing_moves_the_named_bindings_into_their_fields`,
`a_binding_composed_into_two_fields_refuses_at_the_second`,
`moving_a_field_out_leaves_a_hole_a_second_read_refuses`,
`dropping_an_aggregate_releases_its_live_fields_and_tolerates_a_hole`,
`a_drop_through_releases_in_declaration_order_not_acquisition_order`,
`a_drop_through_is_not_blocked_by_a_loan_of_a_field_that_left`,
`an_unreleased_field_is_a_normal_path_leak_named_by_its_path`,
`an_aggregate_is_never_copied`,
`an_aggregate_at_a_plain_owner_position_refuses_by_name` (eight operations),
`a_field_path_and_a_dotted_plain_name_are_different_places`,
`an_aggregate_of_no_owned_field_drops_nothing_and_is_still_not_copied`, plus
the one pre-existing test that read `owner()` as a `&str` now reading
`owner_spelling()`.

Two callers outside the file changed shape and nothing else:
`source_loop::custody_signature` keys by `owner_spelling()`, and
`whole_program::collect_ownership_identities`' exhaustive match gained the five
variants in its no-identity arm (they introduce none).

## 3. Byte identity — every existing program's operation list, measured

`scripts/byte_identity.sh` extracts nothing itself; it is run against a
`git archive e878b362` scratch tree and this worktree. It copies
`scripts/custody_census.rs` into each tree's `compiler/examples/`, builds it in
release mode, runs it **from each tree's own root over the same relative
spelled paths** (constraint 11), and `scripts/compare_census.py` compares per
program: execution profile, the **custody operation list** read through the
routes' public projections (`std_buffer::custody_operations`,
`std_string::string_custody_operations`, `std_socket::custody_operations`),
the emitted C, and the reference observation. The sequential route's driver
retains checked functions' final states rather than the lists it handed the
checker, so that route is compared by its C and observation and the tool says
so (its list is plain-name operations only; no arm of `whole_program.rs` but
one exhaustive match changed).

Roots: `compiler/tests/fixtures examples crates contract docs/reports`.
Result (`measure/census/summary.txt`, reproduced in `census-summary.txt`
beside this file):

```
programs in both trees: 521
admitted in both and byte-identical on profile, custody, C and observation: 286
refused in both (same codes unless listed): 235
differ: 0
present in one tree only: 1
  ONLY card examples/records/owned_field.qcore
```

Of the 286 admitted, **139 emit C at the base and all 139 emit byte-identical
C at the card** (the card emits 140, the new example being the one more); 30
carry a published custody operation list and all 30 are identical. By route at
the card: 181 sequential, 14 string, 14 mutex, 13 composed epoch, **10
buffer**, 10 owned tasks, 7 std-io, 7 composed std-io, 6 socket, 6 flat epoch,
4 read shares, 3 copy tasks, and the rest. `examples/` in full is inside the
scan, so the examples suite's programs are covered structurally. Wave 27's
figure was 80 emitting programs under narrower roots; this census is a
superset and the number is the roots', not a growth in anything.

## 4. Lean parity — `proof/QCore/AggregateOwnedField.lean`

Registered in `proof/AuditAggregateOwnedField.lean` (imported by
`proof/QCore.lean`; 138 audit files, `AuditAll` included); every `#print
axioms` line reports at most `propext`, `Classical.choice`, `Quot.sound`
(`measure/lean-audit.log`), no `sorry`, no project axiom, warnings as errors.

| Rule | Theorems |
|---|---|
| ledger by place | `release_removes_exactly_its_own`, `release_keeps_every_other`, `move_preserves_length` |
| 1 construction moves in | `compose_moves_each_source_in`, `compose_two_moves_both_in`, `compose_moves_no_quantity`, `compose_holds_the_sources` |
| 2 partial move | `move_out_leaves_a_hole`, `a_hole_refuses_a_second_move`, `move_out_keeps_the_other_fields` |
| 3 drop-through (the headline) | `dropThrough_released`, **`drop_through_releases_exactly_the_live_fields`** (the released list *is* the live fields' resources in declaration order), **`drop_through_releases_nothing_twice`** (nodup, from the one-owner-per-resource invariant the Rust checker maintains), `drop_through_tolerates_a_hole`, `drop_through_is_in_declaration_order`, `drop_through_releases_from_the_ledger` |
| 4 leak sees into the aggregate | `an_unreleased_field_is_a_leak_named_by_its_path`, `drop_through_discharges_the_leak` |
| copy rule | `an_aggregate_is_never_copied`, `a_hollow_aggregate_is_still_not_copied` |
| type agreement at construction | `agreement_is_exactly_kind_for_kind` (one equivalence, both refusing directions), `a_scalar_at_a_bytes_field_is_refused`, `a_buffer_at_an_i32_field_is_refused` |
| neutrality | `a_record_of_scalars_composes_nothing` |

`QCore.SourceRecord`'s theorems are unchanged; its prose said "an owned field
would make that false, which is exactly why it is a separate card" and
"`FieldType` has one constructor", both now false, and both paragraphs were
amended to name this module as the other half. Not proved: the lowering's
token reading (a tested Rust boundary), the C initializer performing the move
(the sanitized run), and the two by-name window refusals (§5).

## 5. The route — `source.record.v1` with an owned field

**Spelling.** `type NAME { FIELD: bytes, … }`. `bytes` is the acquisition's
own reserved word (`STD-BUFFER`), so the field type reserves nothing (§8).
`Buffer` — the task route's word — was measured and **rejected**: it names
`qcore_buffer_new`'s four-byte payload, a different resource with a different
model (`BUFFER_PAYLOAD_BYTES = 4`, the task ledger), and a field spelled one
while meaning the other would be a collision by design.

**Route selection did not move.** A record with an owned field holds a buffer
`bytes(N)` acquired, so `has_source_buffer` is true and the module takes
`source.buffer.v1` exactly as any buffer module does; `drop s;` on a string is
re-measured at `Q068` (`the_refusals_that_stay_where_they_were`).

**The lowering** (`crates/qcore-frontend/src/{source_record,source_std_buffer}.rs`):
`let X = RECORD.FIELD[I];` extends the record reader's `Field` shape by the
`[` after the field; `RECORD.FIELD[I] = V;` and `drop RECORD.FIELD;` are the
new `read_field_path_statement` (one shape rule, also used by
`block_record_span` so a field-path statement outside the entry refuses as a
record outside the entry does); `extent(RECORD.FIELD)` is the extent arm's
dotted form. `let NAME = RECORD.FIELD;` is **one** spelling for the scalar
read and the move out; the declared type decides it in the model, never the
reader. The index and value operands are read by the buffer route's own
operand reader, as a construction's field values are.

**The model** (`std_buffer.rs`, record arms only): four new events
(`FieldStore`, `FieldLoad`, `FieldExtent`, `FieldRelease`); the custody walk
builds `Compose` for a construction of a type with an owned field (its owned
fields in declared order, each from a custody name — a non-name contributes no
move on the walk's standing rule and the value half refuses it), `MoveOut` for
a field read of an owned field, `ReadField`/`WriteField`/`DropField` for the
path positions, and the plain `Drop` for `drop RECORD;`; the value half
carries each owned field's extent, bytes and liveness (`OwnedField`), publishes
`RecordConstruction.owned`, `BufferCertificate.moves` (`FieldMove`) and the
released paths in `releases`, and the recheck compares `moves`
(`BufferRecheckFailure::Moves`). Every custody question is the IR's at `Q054`.

### 5.1 Admitted

| Program | Evidence |
|---|---|
| `examples/records/owned_field.qcore` — construct from two acquired buffers, scalar read, `extent(f.body)`, store and loads through both fields, move out, store through the moved-out binding, `drop f;`, `drop taken;` | `the_flagship_builds_exactly_the_aggregate_operations` pins the 16-operation list; certificate `acquisitions [(h,2),(b,4)]`, `releases ["f.head","taken"]`, `moves [FieldMove{event 17, f.body → taken, 4}]`, `constructions[0].owned [(head,h),(body,b)]`, accesses named `f.body`/`f.head`; native run §7 |
| `drop p;` releasing two fields declared in the other order than acquired | `a_whole_record_drop_releases_the_live_fields_in_declared_order`: releases `["p.second","p.first"]`, C frees `f_second` before `f_first` |
| `drop p.first; drop p;` | `a_whole_record_drop_tolerates_a_hole_and_a_field_is_not_released_twice`: releases `["p.first","p.second"]`, each C free once |
| a record of scalars | contributes no operation (`a_record_of_scalars_still_contributes_no_operation`), its programs byte-identical (§3) |

### 5.2 Refused, with codes — no new code, each condition stated in §L21

| Written | Code | Refusal | Test |
|---|---|---|---|
| `b[0] = 1;` after `Packet { buf: b }` | `Q054` | `use of `b` after move` | `using_the_buffer_after_moving_it_into_a_record_refuses_as_a_use_after_move` |
| `let c = p.buf; let x = p.buf[0];` / a second `let d = p.buf;` | `Q054` | `use of `p.buf` after move` | `using_a_field_after_moving_it_out_refuses_by_its_path` |
| no `drop p;` | `Q054` | `live resources escape normal exit: p.buf` | `an_unreleased_owned_field_is_a_leak_named_by_its_path` |
| move out, `drop p;`, no `drop c;` | `Q054` | `… normal exit: c` | same |
| `drop p; drop p;` | `Q054` | `resource `p` is dropped more than once` | `a_whole_record_drop_releases…` |
| `drop p.buf; drop p.buf;` | `Q054` | `resource `p.buf` is dropped more than once` | `…tolerates_a_hole…` |
| `Pair { first: a, second: a }` | `Q054` | `use of `a` after move` | `one_buffer_moved_into_two_fields_refuses_at_the_second` |
| `let z = p.buf;` over a live `z` | `Q012` | name resolution's duplicate `let`, **before** this route; the IR's `BindingAlreadyExists` arm is model-level only (unit-tested) | `a_move_out_binding_that_shadows_a_live_name_refuses` |
| `Packet { buf: 2 }`, `{ buf: n }` (scalar), `{ buf: q }` (record) | `Q052` | `FieldTakesABuffer`: "takes an owned buffer this entry acquired" | `a_non_buffer_at_a_bytes_field_is_refused` |
| `Point { x: b }` — a buffer at an `i32` field | **`Q055`**, unchanged | `CrossesBoundary` | `a_buffer_at_a_bytes_field_is_admitted_and_at_an_i32_field_still_refused` |
| a lend at a `bytes` field | `Q051` (the lend-window gate fires first; the construction's own arm is the lend's `Q055`) | | `a_lend_at_a_bytes_field_is_the_lend_crossing_refusal` |
| `p.tag[0] = 1;`, `drop p.tag;`, `extent(p.tag)` | `Q052` | `FieldIsNotABuffer`: "`p.tag` is a `Packet` field declared `i32`, not a buffer" | `a_custody_position_at_a_scalar_field_is_refused` |
| `p.nope[0]` | `Q052` | `NoSuchField` | same |
| `p.buf[4] = 1;` on a 4-byte field | `Q053` | `index 4 is outside `p.buf`` | `an_index_through_a_field_is_bounded_by_the_buffer_it_holds` |
| `drop p;` on a record of scalars | `Q052`, unchanged | "is not a buffer this entry acquired" | `the_refusals_that_stay_where_they_were` |
| `print dec p;` on a record that owns | `Q052`, unchanged | "read the field you mean first" | same |
| `drop s;` on a string | `Q068`, unchanged | | same |
| a `lend(…)` in an entry that composes a record that owns | `Q051` | `OWNED-FIELD-LEND-WINDOW` | `a_lend_beside_an_owned_field_and_an_owned_field_in_a_loop_body_refuse_by_name` |
| a construction of a record that owns in a `for` body; a field read of an outer one in a body | `Q051` | `OWNED-FIELD-LOOP-BODY` | same |
| `type Packet { buf: Buffer }` | `Q051` | `UnknownFieldType`, naming `AGGREGATE-FIELD-TYPES` | `the_field_type_is_spelled_bytes_and_buffer_is_not_one` |
| a record that owns on the composed string route | `Q068`, unchanged | | `the_composed_string_route_still_refuses_a_record_with_an_owned_field` |

One message-text pin moved deliberately: `type Point { x: u8 }` used to be
told the owned-field gap by name; it now names the gap that remains
(`a_field_type_this_revision_does_not_model_is_refused` asserts
`AGGREGATE-FIELD-TYPES` and the absence of `AGGREGATE-OWNED-FIELD`).

### 5.3 Diagnostics — §L21 and the AGENTS rows

Stated in `docs/QCORE_SPEC_r0_3.md` §L21 (the `Q051`–`Q057` row's wave-39
amendment and the `Q1xx` row) and in AGENTS.md's table: **`Q055` narrowed** (a
buffer at a field position refuses only at an `i32` field — the deliberate
lift, the first *lift* under the icebox); `Q051` widened (malformed field-path
statement; the two by-name refusals); `Q052` widened (`FieldTakesABuffer`,
`FieldIsNotABuffer`); `Q053` widened (an index past the extent a field holds,
naming the path); `Q054` carrying the path-extended checker's vocabulary;
`Q056` not widened (the recheck comparing `moves` and the released paths is
that condition working). `Q103` covers the grown `OwnershipError` by
definition and **no sequential program reaches it** — a measured boundary,
`OWNED-FIELD-SEQUENTIAL-UNREACHED`, not a claim; `Q100` is not widened (the
copy refusal is the checker's `CopyOfOwned`). The two model-level refusals
`FieldTakesAScalar` (`Q055`) and the route's `CrossesBoundary` (`Q055`) are one
condition with one code however it is reached.

## 6. Emission, and constraint 15k's byte-neutrality argument re-derived

`crates/qcore-core/src/std_buffer_c_emit.rs`: a `bytes` member is
`unsigned char *f_NAME;`; the construction's designated initializer names the
old binding (`.f_head = b_h`) — **the move, in C**; accesses through a field
are the plain statements with the member and the field's extent
(`qcore_bytes_store(r_f.f_body, (size_t) 4u, …)`); the move out is
`unsigned char *b_taken = r_f.f_body;`; `drop RECORD.FIELD;` is one
`qcore_bytes_drop(r_p.f_x);`; `drop RECORD;` is one free per member **still
held, in declaration order**, the emitter tracking liveness exactly as the
checker published it. A paragraph about owned members (`RECORD_PRELUDE_OWNED`)
is **appended** to the record header only for a module declaring one, so the
header's bytes are unchanged for every module of scalar records — which §3
measured.

**15k's argument, re-derived with the owned field enumerated.** "A program
that acquires no buffer carries no buffer prelude" rested on "among admitted
programs, reading or writing a byte implies acquiring a buffer". The card adds
four ways to read or write a byte — through a field — and one way to hold a
buffer in a new place. A field holds only a buffer supplied by **name** at
construction, a name the value half admits only if `extents` holds it, which
only `bytes(N)` writes; a construction **moves** (`Compose`) and acquires
nothing. So every byte read or written through a field is preceded by an
`Acquire`, the prelude scan's premise stands, and the two accessor scans were
extended to `FieldStore`/`FieldLoad` (a program whose only store goes through a
field still needs `qcore_bytes_store`, or `-Werror` refuses the C — the
fail-closed failure, but a failure). No new acquisition path exists.

## 7. Native, sanitized

`examples/records/owned_field.qcore` (the family's README extended: form,
rules, the `frame-struct` emitted block and the `owned_field stdout`
transcript, read back by `examples_suite`). Run by
`the_flagship_runs_clean_under_the_sanitizers_and_matches_the_reference`
(`compiler/tests/aggregate_owned_field.rs`) and by `examples_suite`'s
`the_record_examples_run_clean_and_match_their_reference_execution` (the
family's third entry): `cc -std=c11 -Wall -Wextra -Werror -pedantic -g -O0
-fsanitize=address,undefined -fno-omit-frame-pointer`, `ASAN_OPTIONS=
detect_leaks=1:halt_on_error=1`, exit 0, stderr empty, stdout
`7 4 72 81 67 111` equal to the reference execution's projection. It is a
**positive leak control**: one buffer is freed through the whole-record drop,
the other through the moved-out binding, and M12 (§9) shows LSan/ASan would
report the emitter freeing a moved-out member. Linux only; the `windows_c`
path is carried unchanged and **no Windows or Apple run is claimed**
(`OWNED-FIELD-WINDOWS-RUN` is *not* registered as a residual: it is the same
footing as `SLICE-PERMISSIONS-WINDOWS-RUN` and would be one more name for one
recorded gap — the integrator may prefer otherwise, §12).

## 8. Keyword census (constraint 14) — run, not assumed

`scripts/keyword_census.py`, 526 `.qcore`/`.qc` files, comments stripped
(`keyword-census.txt` beside this file): `: bytes` in field-type position
**1** occurrence — the card's own example; `: Buffer` **0**; a record
declaration with a `bytes` field **1** (the example); `NAME.NAME[` **4** (all
in the example); `drop NAME.NAME` **0** (spelled by tests only);
`extent(NAME.NAME)` **1** (the example). **+0 words, +0 tokens, no lexer rule
changed.** `Buffer` rejected on the ground in §5, recorded in AGENTS.md
constraint 14's ledger.

## 9. Falsification — mutations, shipped

`scripts/mutations.py` (driver `scripts/run_mutations.sh`), log
`mutations.log` beside this file. Each mutation is an exact text replacement
of one arm, the named test must go **red**, the file is restored from its byte
copy and shown sha256-equal; a green mutation is reported VACUOUS and fails the
harness.

MUTATION-LOG-PLACEHOLDER

## 10. Gate

GATE-PLACEHOLDER

## 11. Residuals and retirements

**Retired (1), by exact name, after `scripts/register.py --show` printed the
entry** (`register-show.log`): `AGGREGATE-OWNED-FIELD`. Its discharge read
"the four rules … landed in the IR first (an IR card before a route card),
with `Q055`'s lift stated deliberately and constraint 15k amended in the same
change; … allocates no code" — met word for word: §2 (IR first, the four
rules), §5.3 (`Q055`'s lift stated), constraint 15k amended in this tree (§12),
zero codes.

**Registered (6), `recorded_by: AGGREGATE-OWNED-FIELD`, each with a discharge
condition** (`register.log`): `AGGREGATE-VIEW-FIELD` (the view-field step,
15h's window cannot reach through an aggregate; `AGGREGATE-STRING-COMPOSE`
blocks its route), `AGGREGATE-NESTED` (one-level path by measurement; a
recursive path with the drop-through over a tree and a termination argument
for the declaration graph), `AGGREGATE-TASK-CAPTURE` (a record that owns in a
spawn capture: the transferability judgment has no aggregate kind and the
scoped-task ledger is name-keyed; unreachable by route selection today),
`OWNED-FIELD-LEND-WINDOW`, `OWNED-FIELD-LOOP-BODY` (the two by-name refusals,
each with the projection change that lifts it), and
`OWNED-FIELD-SEQUENTIAL-UNREACHED` (`Q103`'s vocabulary widening reached by no
sequential program). `named_followups` 228 → 233.

**Measured boundaries stated, not registered:** the IR's drop-through order is
observable only through the value half and the emitter (§2.2); the
`BindingAlreadyExists` arm of `MoveOut` is reachable from source by no program
(`Q012` precedes it) and is unit-tested at the model level (§5.2); no Windows
or Apple run (§7).

## 12. For the integrator

1. **Headline numbers moved in this tree:** AGENTS.md `named_followups`
   **228 → 233** (line after `named_followups`, six registered and one retired
   as named); Lean audits **137 → 138** (`AuditAggregateOwnedField.lean`);
   workspace tests: see §10 for the figure to carry into the "Merged gate" row
   at the merge (this card adds 12 unit + 22 end-to-end tests and touches the
   examples suite's lists).
2. **Standing constraint 15k amended in this tree, deliberately** — the
   former "do not give a field custody without the four rules that do not
   exist" superseded by the rules' location and content, the `bytes` spelling,
   the `Q055` lift, the two by-name refusals, and the re-derived prelude
   argument; quoted in AGENTS.md at 15k in the card's words. Constraint 14's
   ledger gained the `AGGREGATE-OWNED-FIELD` +0 bullet; the Q-space note
   records the **sixteenth** instance and the first *lift*; the `Q051`–`Q057`
   and `Q100`/`Q103` table rows carry the widenings.
3. **§L21** (`docs/QCORE_SPEC_r0_3.md`): the `Q051`–`Q057` row's wave-39
   amendment and the `Q1xx` row's sentence.
4. **Increment record:** `docs/development/increments/wave-39.md` created
   (wave-38's header shape), paragraph prepended; `README.md` pointer and table
   row added. Union with siblings' paragraphs at the merge.
5. **Matrix:** `docs/reviews/r0_3/acceptance_matrix.json` `named_followups`
   edited by `scripts/register.py` and regenerated with `--gate
   docs/reports/aggregate-owned-field/gate.json` (§10).
6. **Decisions I could not take:** whether `OWNED-FIELD-WINDOWS-RUN` deserves
   its own name (§7 — I left it under the family's existing footing); whether
   `Q103`'s vocabulary sentence should instead be recorded as not applicable
   when the namespace decision is taken (`OWNED-FIELD-SEQUENTIAL-UNREACHED`'s
   discharge offers both); whether the `AGGREGATES-V1` frozen report's §3.1
   table ("present on `f122cc03`? no") wants a beside-the-archive note — it is
   a checkpoint and I did not edit it (constraint 21).

## 13. File ownership and overlaps (wave-39 scope)

Owned by this card and edited: `crates/qcore-core/src/ownership.rs` (and its
unit file), `crates/qcore-core/src/std_buffer.rs` **record arms only** — the
`Construct`/`Field` arms of the custody walk and the value half, the new
`Field*` arms, the two gates in `check_buffer_program`, `body_bindings`, the
recheck; the `lend_windows` function and the `Repeat` arms (LOOPS-ON-COMPOSED-
ROUTES) were **not** edited, and the one line added inside
`lend_custody_operations` is its catch-all arm listing the four new variants.
Edited with no other card's ownership and noted here: `source_record.rs` (core
and frontend), `source_std_buffer.rs`, `std_buffer_c_emit.rs`, `std_string.rs`
and `std_string_c_emit.rs` (the exhaustive matches' new arms only — not
`last_view_uses`), `whole_program.rs` (one match arm), `source_loop.rs` (one
`owner_spelling()` call), `examples_suite.rs` (the records lists),
`source_record.rs` tests (one API spelling, one message pin), the Lean files,
and the ledger documents. Not touched: `socket_footprint.rs`,
`disjoint_footprint.rs`, the `std_socket` route files, `ct_corpus.rs`,
`ct07_proof_leg.rs`, `thread_sanitizer.rs`, `task_route_tsan.rs`.
