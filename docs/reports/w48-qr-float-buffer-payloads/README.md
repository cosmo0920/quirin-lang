# W48-QR-FLOAT-BUFFER-PAYLOADS: floats on the buffer route and in payloads

The float card's second half, base `d11179c9` (= origin/main at launch, the
W48-QR-FLOATING-POINT tip; the worktree's HEAD was already that commit, so no
reset or fast-forward was needed; local `main` had meanwhile moved to
`93789344` with W48-QRLIB-LINKAGE, unpushed, and is not this card's base). It
closes `W48-QR-FLOAT-BUFFER-ROUTE` and `W48-QR-FLOAT-PAYLOADS`: all three
numbered items of the brief are done. The float card's seven user decisions
apply unchanged, and its machinery is reused, not forked: `scalar_floats.rs`,
`parse_admitting_floats` and the per-route opt-in, the shared
`scalar_values::Decider` with its comparison effects, card R6's `size_bytes`
payload extents and `WRITE-LEND-TASKS` typed ledger. Grammar: implemented
grammars [§G02t](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02t-fixed-width-scalar-types)
("Floats on the byte-buffer route and in payloads"); semantics:
specification §L14.0 and
[§L14.7](../../QCORE_SPEC_r0_3.md#l147-floating-point-and-its-limitations)
items 14–19; diagnostics `Q098`. The prediction was frozen before any source
change ([PREDICTION.md](PREDICTION.md)).

## Mechanism

* **Buffer route** (`source_std_buffer.rs`, `std_buffer.rs`,
  `std_buffer_c_emit.rs`). The entry's row reads `partial_eq`/`partial_ord`
  beside its `EffectRow` (`BufferProgram::float_effects`, the extern route's
  shape); the checker's `Decider` is built with them as `permitted`, and the
  effects the statements used are published (`BufferCertificate::float_comparisons`,
  compared by the recheck). Records are read with
  `record_declarations_admitting_floats`; a field value may be a float literal
  (`FieldOperand::Float`, read once at the field's type); `as` takes
  `f32`/`f64`; the value statement is read with
  `parse_value_expression_admitting_floats` (`ValueOperand::Float`), and a
  float literal's conversion and the binding `let NAME = FLOAT;` are
  `ValueOperation::Convert(Float(_), T)` — `FLOAT` is `FLOAT as f64`. The
  checker's `convert` admits float sides (never `bool`) through §G02t's
  `convert`; a faulting float → integer conversion is refused now (`Q053`,
  named by its `dec` rendering). The emitter spells float literals with
  `c_literal`, calls §G02t's float helpers, emits the IEEE environment and the
  renderers the lines use (`RecordEnvironment::float_prelude`) and includes
  `<float.h>`/`<string.h>`, all only when `std_buffer::uses_floats` holds.
* **Loop bounds** (`qcore_core::source_loop`). `typed_bound` refuses an
  `f32`/`f64` bound by name (`BoundRefusal::FloatBound`, `Q098`) before its
  number is read, and `index_admits` refuses a non-integer index: a float's
  number in the tier is its bit pattern, which the base would have read as a
  count had a float ever reached the tier (none could before this card).
* **Composed string route** (`source_std_string.rs`, `std_string.rs`,
  `std_string_c_emit.rs`). The entry row's comparison effects
  (`StringProgram::float_effects`) feed the embedded `BufferEvaluation`;
  `source_std_buffer::value_statement` claims float spellings and the float
  binding; `EmbeddedTyped` carries the float environment and the renderers a
  top-level line uses.
* **Payloads** (`source_tasks.rs`, `task_c_emit.rs`,
  `task_execution/reference.rs`, `diagnostic_log_c_emit.rs`). `Buffer<T>`
  reads `T` with `parse_admitting_floats`; the checker's extent is
  `size_bytes` (4, 8) unchanged. A float payload's event carries its bit
  pattern, copied with `memcpy` (`qcore_buffer_bits_f32`/`_f64`), and the
  reference traces `F32 BITS`/`F64 BITS`, every NaN `nan`, which the sink's
  float payload branch (`with_float_payloads`, installed only for a float
  payload) prints.
* **Mutex route** (`source_mutex.rs`, `frontend.rs`, `mutex_c_emit.rs`).
  `protected f32`/`f64` is `protected 4`/`8` to the model, byte for byte; the
  emitted C adds the float environment (`typed_preamble`).
* **Driver**: `CompiledModule::float_assumption` covers the buffer, string and
  mutex routes.
* **Lean**: `QCore.FloatPayloads`, `AuditFloatPayloads.lean` (19 theorems) —
  a new module, `QCore/FloatTypes.lean` untouched.

## Items

| Item | Position | Status |
|---|---|---|
| 1 | `source.buffer.v1` (§G02k, §G07.12, §G07.13): float record fields (literals, reads, writes), conversions, value statements, the literal binding, loop bodies; comparisons under the entry row's `partial_eq`/`partial_ord` | **done**; `W48-QR-FLOAT-BUFFER-ROUTE` retired |
| 1 | a float loop bound | refused by name on every route (`Q098`, `BoundRefusal::FloatBound`) |
| 2 | `Buffer<f32>`/`Buffer<f64>` (§G02b), extents 4/8, byte views under the byte rules; `protected f32`/`f64` (§G02c) | **done**; `W48-QR-FLOAT-PAYLOADS` retired |
| 2 | typed disjointness at float extents | re-proved (`QCore.FloatPayloads`), not weakened |
| 3 | the composed string route's typed tier gains floats | **done** (the second half of `W48-QR-FLOAT-BUFFER-ROUTE`) |

The buffer route's entry row **can** state the comparisons, so no float
comparison is refused there by name: the row reads `partial_eq`/`partial_ord`
beside `io`/`alloc`/`mut`, and an unknown effect keeps its old message byte
for byte.

## Specification limitations written (§L14.7)

Item 14 (positions) is rewritten: every position card 0 widened for integers
now admits floats, the float loop bound is refused on every route including
one a lowering folds, and the `.qrlib` spellings are card 5's. New normative
items: **15** check-time floats on the buffer and string routes (computed by
the checker with host IEEE operations and again by the C, agreement observed;
no run-time `OutOfRange` path there); **16** a byte view of a float payload is
its IEEE object representation, every byte pattern is a value, and **a NaN's
sign and payload bits in it are not specified** — measured: `0.0 / 0.0`
divided at run time is x86-64's sign-set default NaN, folded by Clang `-O2`
the sign-clear quiet NaN; **17** a float payload's trace is its bit pattern,
every NaN `nan`, and F02 refuses it (`W48-QR-F02-TYPED-PAYLOADS`); **18**
`protected f32`/`f64` hold bytes and nothing reads them as floats; **19**
comparisons on the buffer and string routes are gated by the entry's row
alone, every loop iteration included, and the certificate publishes the
effects used. §L14.0 gains the positions and two MUSTs (a float bound is
refused; a payload's extent is its format's size); implemented grammars §G02t
gains the section above and its table rows; diagnostics `Q098` gains the float
bound.

## Corrections

| What | Before | After | Reason |
|---|---|---|---|
| `float_tasks.rs::negative_controls_refuse` | a row refusing `own<Buffer<f64>>` with an empty needle | removed, pointing to `float_buffer_payloads.rs` | item 2 discharges the refusal it pinned (its program also routed to the scalar route, whose refusal the empty needle matched) |
| `source_record.rs::a_field_type_this_revision_does_not_model_is_refused`, `owned_field_route_reach.rs::every_other_unknown_field_type_keeps_the_message_it_had` | `f64` / `f32` as the unmodelled field type | `f16` | item 1 admits `f32`/`f64` fields; the rows keep pinning `AGGREGATE-FIELD-TYPES` with a spelling no route models |
| ten test files' `BufferProgram`/`StringProgram` literals | no `float_effects` | `float_effects: Default::default()` | the new field; compile only |
| `BufferEvaluation::compute` | `&self` | `&mut self` | it records the comparison effects used |

## Evidence

**Fixture identity.** Card 0's `freeze_fixtures.py` over every tracked
`.qr`/`.qv`/`.qrlib` file, debug `quirin` at the base (`0319bb6b…`, 702 files)
and at `2a1f7781` (`55a6158d…`, 703 files; later commits change tests, docs and
records only) ([BEFORE](artifacts/fixtures-before.json),
[AFTER](artifacts/fixtures-after.json),
[comparison](artifacts/fixtures-compare.txt)): **all 702 existing files are
byte-identical** in exit, stdout and stderr of `check` and `emit`; the one new
file is `examples/scalar/float_buffer.qr` (check 0, emit 0). No existing
refusal became an admission and no admission a refusal among tracked files.
The float card's census script ([after](artifacts/census-after.json)) finds
the new spellings only in that example; 118 reserved words, none added.

**Native** (Clang 21.1.8; `-O0` and `-O2` with `-fsanitize=undefined,integer
-fno-sanitize=float-divide-by-zero -fno-sanitize-recover=all`, and plain
`-O2`, all `-std=c11 -Wall -Wextra -Werror -pedantic -ffp-contract=off`;
`compiler/tests/float_buffer_payloads.rs`):

| Table | Per type | Result |
|---|---|---|
| buffer-route boundary table: per left operand of the float card's 21 values, `+ - * /` against all 21 (`hex`), all six comparisons under both effects (`dec`), negation, conversion to every integer type where it fits and to the other float | 21 programs | both streams equal to an independent reading (the kernel, value by value), exit 0 |
| integer → float: every integer type's boundary values and ±(2^24 or 2^53 + {−1, 1, 3}), read from record fields | 1 program | equal |
| records, the literal binding, a loop body computing and comparing a float, `log` on stderr | 1 program | equal |
| the composed string route | 1 program | equal |
| **total** ([summary](artifacts/native-buffer.json)) | **48 programs, 144 runs, 27,900 lines** | **0 sanitizer reports** |
| float payloads: each of the 21 values allocated, moved to a worker that reads it and writes the next value, returned, split into two disjoint leases (2+2 / 4+4 bytes) whose workers write one byte each, read by byte and whole | 21 programs × 3 builds ([summary](artifacts/native-payloads.json)) | the reference trace (every NaN `nan`) line for line, exit 0, 0 reports — except **2 runs per type** that differ **only in the sign bit of a NaN payload's top byte** (the `-O2` builds fold `0.0 / 0.0`), which the rows admit exactly there (§L14.7 item 16, `W48-QR-FLOAT-NAN-BYTE-VIEW`) |
| ThreadSanitizer through the pthread binding: payloads ±0, a subnormal, +inf and a NaN; unforced, forced together, forced reverse | 15 rows ([summary](artifacts/native-tsan.json)) | **30 rows clean**, 0 races, the reference trace |
| `protected f32`/`f64` under the mutex observation sink | 1 program × 3 builds | the typed reference observation line for line |
| `examples/scalar/float_buffer.qr` | 3 builds | the page's transcript, both streams |

**Negative controls** (each a test row): a float comparison without its
effect, `!=` under `partial_ord` only, `>=` under `partial_eq` only; a mixed
`f32`/`f64` operation and field; an implicit int ↔ float (an integer literal
as a float operand and at a float field, a float literal as an integer
operand, an integer name written into a float field, a float at a byte
position, an integer literal written into a float payload); a lease past a
float payload's bytes (an `f64`-sized lease on an `f32`, one byte past an
`f64`) and an owner byte read past it; a write of the other float type into a
payload; a payload moved into the other float type's parameter; a float loop
bound, directly and through a conversion the lowering folds; float → integer
of a non-fitting value, NaN and an infinity (refused at check time, `Q053`);
a literal that rounds to ±inf (statement and field); float `%`; `f64 as
bool`; on the composed string route its own forms at `Q077`/`Q098`.

**Lean** (`QCore.FloatPayloads`, [audit](../../../proof/AuditFloatPayloads.lean),
19 theorems, no placeholder, no project axiom; `QCore/FloatTypes.lean`
untouched): the payload size table with floats
(`a_float_payload_is_its_format_size`, `integer_payload_sizes_are_unchanged`,
`payload_sizes_are_one_to_eight`); the byte view (`toBytes_length`,
`fromBytes_lt`, `fromBytes_toBytes`, `a_byte_write_keeps_a_float_value`,
`every_float_pattern_is_its_bytes`, `float_patterns_are_every_byte_pattern`);
`WRITE-LEND-TASKS` at float extents
(`no_byte_has_two_write_authorities_payload`,
`every_held_byte_is_a_payload_byte_payload`, `float_lease_ledgers`,
`an_f64_sized_lease_on_an_f32_is_refused`, `the_float_halves_are_admitted`);
the buffer route's comparisons as the float card's `Admitted` for one
call-free entry (`buffer_entry_admitted_iff`,
`buffer_entry_without_effect_compares_nothing`,
`unrolled_comparisons_are_the_body's`,
`a_loop_body_comparison_needs_its_effect`); and `floatSizeAgreement_holds`,
replayed by `lean_float_payload_sizes_match_the_rust_extent`. **What is
modelled:** a float payload as a bit pattern of 4/8 little-endian bytes; the
buffer route's comparisons as the sites of one call-free function. IEEE
arithmetic stays observed (`W48-QR-FLOAT-IEEE-LEAN-MODEL`, the parallel card).

## Prediction and outcome

| Measure | Predicted | Outcome |
|---|---|---|
| fixture freeze (702) | byte-identical, new entries only the card's examples, 75% | **702 identical**, one new example |
| existing tests changed | 1–4 files, 60% | 3 behavioural rows moved (above) + 10 compile-only struct literals (not predicted: the literals' count) |
| native tables | equal bit for bit, 0 reports, 70% | buffer and string routes **equal, 0 reports**; payloads equal except the NaN sign bit of a byte view in 2 runs per type (not predicted: measured, stated as §L14.7 item 16, registered) |
| TSan payload rows | 0 reports, 80% | **30 rows clean** |
| Lean | one module, 8–16 theorems, 80% | **one module, 19 theorems**, 251 audits |
| corpus arbiter | identical to `2fc3af8c…`, 95% | **byte-identical** |
| gate first run | green, 40% | **not green**: tests 101 (the three rows pinning the discharged refusals) and provider 101 (one load-sensitive `QP006` provider timeout while a test build ran beside it); fixed / rerun green |
| items | 1 (75%), 2 (60%), 3 (45%) | **all three** |

## Gate

| Step | Command | Exit | Count |
|---|---|---|---|
| submodule | `git submodule update --init --recursive` | 0 | at `c3e28405` |
| fmt | `cargo fmt --all -- --check` | 0 | at `e3fa19d8` |
| tests | `python3 scripts/run_test_binaries.py` | 0 | at `e3fa19d8`: 396 units, 409 summaries; **6,938 passed, 0 failed, 11 ignored** (`private/test-binaries/20260930T122635Z-3790016/summary.json`). First run at `c3e28405`: **101**, 6,935 / 3 / 11 (`float_tasks`, `source_record`, `owned_field_route_reach`: rows pinning refusals this card discharges; corrected above) |
| clippy | `cargo clippy --all-targets -- -D warnings` | 0 | at `e3fa19d8` (and 0 at `c3e28405`) |
| probe | `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | at `c3e28405` |
| follow-up probe | `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | at `c3e28405` |
| Lean | `python3 scripts/check_lean_audits.py` | 0 | **251 audits** (250 + `AuditFloatPayloads.lean`), at `c3e28405` |
| provider suite | `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast` | 0 | at `e3fa19d8`: **129 passed, 0 failed, 1 ignored**, 43 summaries. First run: **101**, 128 / 1 / 1 — `configured_source_contract::pinned_bundle_and_legacy_path_have_identical_process_evidence`, `QP006 provider timed out`, while this card's own three-binary `cargo test` ran beside it (constraint 16's load-sensitive shape; `providers/` is unchanged) |
| matrix | `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/w48-qr-float-buffer-payloads/gate.json` | see gate.json | |
| docs index | `python3 scripts/build_docs_index.py` | see gate.json | |
| Python | `python3 -m unittest discover -s scripts/tests` | see gate.json | |

Between `c3e28405` and `e3fa19d8` only test files and report artifacts
changed, so the probes and the Lean audits were not rerun. Logs
`private/w48-fbp/gate/` and `private/w48-fbp/gate2/`; exits in [gate/](gate/),
record in [gate.json](gate.json). Clang 21.1.8 recorded in this worktree.

## Follow-ups

Ledger ([followups.json](followups.json), applied by the idempotent
[apply_ledger.py](apply_ledger.py); rerun reports every entry already applied):
retires `W48-QR-FLOAT-BUFFER-ROUTE` and `W48-QR-FLOAT-PAYLOADS`; amends
`W48-QR-FLOAT-MSVC-NATIVE` (the new float emission is Linux/Clang-only too)
and `W48-QR-F02-TYPED-PAYLOADS` (float payload events are refused by the same
encoding); registers **`W48-QR-FLOAT-NAN-BYTE-VIEW`** (S: canonicalise a NaN at
every typed float payload write, so a NaN payload's byte view is determined).
649 → 648.

## Files likely to conflict

`crates/qcore-core/src/std_buffer.rs` (`BufferProgram::float_effects`,
`ValueOperand::Float`, `ValueOperation::Convert`, `FieldOperand::Float`,
`uses_floats`, the certificate's `float_comparisons`),
`std_buffer_c_emit.rs` (`RecordEnvironment::{renderers, floats}`,
`EmbeddedTyped::{hold_floats, use_renderer}`), `std_string.rs`
(`StringProgram::float_effects`, `uses_floats`), `std_string_c_emit.rs`,
`source_loop.rs` (`BoundRefusal::FloatBound`), `task_c_emit.rs`
(`qcore_buffer_bits_T`), `diagnostic_log_c_emit.rs` (`with_float_payloads`),
`mutex_c_emit.rs` (`typed_preamble`), `task_execution/reference.rs`;
`crates/qcore-frontend/src/source_std_buffer.rs`, `source_std_string.rs`
(effect rows), `source_tasks.rs` (`buffer_payload`), `source_mutex.rs`,
`frontend.rs` (`protected_type`); `crates/qcore-verification/src/driver.rs`
(`float_assumption`); `proof/QCore.lean`; specification §L14.0/§L14.7,
grammars §G02t, `docs/agents/diagnostics.md` (`Q098`), STATUS, the acceptance
matrix. Card 5 (`.qrlib`) meets it in `driver.rs` and §L14.7 item 14; the
parallel Lean card does not (its `FloatTypes.lean` is untouched here).

## Scripts

* Card 0's `freeze_fixtures.py`/`compare_fixtures.py`, the float card's
  `census_float_spelling.py`, unchanged.
* Native summaries: `QCORE_W48_FBP_NATIVE_SUMMARY=… QCORE_W48_FBP_PAYLOAD_SUMMARY=…
  QCORE_W48_FBP_TSAN_SUMMARY=… cargo test -p qcore-compiler --test
  float_buffer_payloads -- native thread_sanitizer`.
* [`apply_ledger.py`](apply_ledger.py).

## Corpus arbiter

`QCORE_TYPED_PATH_PARITY=OUT/parity.jsonl python3 scripts/run_pinned_audit.py
OUT e3fa19d8 --inputs-root …/build/cycle19 --jobs 8`, AFTER only, with this
card's release `quirin-audit` (`f0025447…`): exit 0.
`resource-diagnostics.json` sha256 `2fc3af8c39f0cf04…` (602 Safe / 10 Unsafe /
4,222 Unknown) is **byte-identical** to the base's archive (W48-QR-FLOATING-POINT's
AFTER), and so are `resource-diagnostics.chunk-contracts.json` (`cbbcfd9c…`)
and `monkey-source.json` (`57acfc58…`)
([identity](artifacts/arbiter-corpus.identity)). No C-audit path was touched.
