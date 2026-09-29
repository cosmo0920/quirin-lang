# W48-QR-SCALAR-TYPES-R6: typed payloads and the remaining positions

The remaining positions of card 0 of the `.qr` language track, payloads first
(base `8a378bf3` = origin/main; the worktree's HEAD was already that commit, so
no reset or fast-forward was needed). It completes **all five** items of the
brief: `W48-QR-TYPES-PAYLOADS`, `W48-QR-TYPES-EPOCH-TASKS`,
`W48-QR-F02-FIXED-OUTCOMES`, `W48-QR-TYPES-COMPOSED-STRING` and the two
emitted-C defects (`W48-QR-LOOP-BOUND-NAME-UNUSED-C`,
`W48-QR-INLINE-COMPARISON-WERROR`). Card 0's decisions, output rule and
evidence apply unchanged, and the machinery of R3–R5 is reused, not forked:
§G02t's `q_scalar_*` helpers, the shared `scalar_values::Decider`, the buffer
route's reader, runner, typed environment and emitter, R4's fixed-width task
union and adapter arm. Grammar: implemented grammars
[§G02t](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02t-fixed-width-scalar-types)
("Typed payloads", "The composed epoch route's task half", "Fixed-width
outcomes in F02", "The composed string/buffer route", "Emitted C under
`-Werror`"), §G02b and §G02c; semantics: specification §L14.0, §L20, §L21;
diagnostics rows `Q020`, `Q077`, `Q300`, `Q600`. The prediction was frozen
before any source change ([PREDICTION.md](PREDICTION.md)).

## Item 1: typed Buffer and protected payloads

**Rules.** The payload type is part of a Buffer's type: `own<Buffer<T>>`,
`borrow<Buffer<T>>` and `lend<Buffer<T>>` for every §G02t type `T`, and
`own<Buffer>` is `own<Buffer<i32>>` (the same program spelled either way is one
program, byte for byte). The payload type is named at acquisition by the
initializer — `qcore_buffer_new(VALUE)` holds a payload of `VALUE`'s type — a
whole read yields it and a whole write takes exactly it (a literal ranged
against it); a capture, a loan and a lease deliver exactly the parameter's
payload type. A lease's byte extent and an owner byte read's are the payload
type's **size** (`bool`/`i8`/`u8` 1, … `i64`/`u64` 8, `isize`/`usize` the
recorded width / 8), never an assumed four. A byte **write** into a `bool`
payload is refused: a byte other than 0/1 is not a `bool`. An error variant's
payload stays the `i32` Buffer (`V(own<Buffer<T>>)` refuses at `Q300`,
`W48-QR-TYPES-ERROR-PAYLOAD`). `protected TYPE` declares a mutex-protected
payload of the type's size; `protected i32` emits `protected 4`'s bytes, and a
write into a `protected bool` stores the fill's low bit.

**Mechanism.** `task_execution::ScalarType::{OwnedBuffer, ReadLoan,
WriteLease}` carry the payload type (`OWNED_BUFFER` etc. are the `i32` ones)
and so do the checker's abstract values, so every type comparison — capture,
result, join — compares payloads. The checker's `State` holds the recorded
target and decides `payload_extent` (`ScalarType::size_bytes`); it records each
Buffer slot's payload type (`CheckedTaskModule::buffer_payload`). The reference
execution stores `(type, value)` cells and computes byte views with
`scalar_types::object_bytes`/`from_object_bytes`; a typed payload's events are
`TaskEvent::Typed*`, rendered `QCORE_BUFFER_ALLOC 0 1 U16 258`, and an `i32`
payload's are the historical events. The emitted C, only for a module naming a
typed payload, adds a `union qcore_buffer_typed` member beside the historical
`int32_t payload`, per-type `qcore_buffer_{new,read,write,byte_read,byte_write}_T`
helpers (no byte write for `bool`), `_Static_assert(sizeof(T) == N)` per payload
type, and logs typed events under a `=TYPE` name with the bits copied by
`memcpy` (a `uint64_t` above `INT64_MAX` is never converted to `int64_t`);
`diagnostic_stdout_adapter_definition_for` installs the typed-payload arm only
where needed. The F02 relation refuses a typed payload event
(`UnsupportedObservedEvent`) rather than narrowing it
(`W48-QR-F02-TYPED-PAYLOADS`). On the mutex route `LoweredMutexModule`
publishes each typed resource's payload and the target assumption, and
`mutex_c_emit::{emit,interpret}_typed_mutex_*` apply the `bool` fill.

**Lean** (`QCore.ScalarPayloads`, [audit](../../../proof/AuditScalarPayloads.lean),
15 theorems): the size table (`size_is_bits_over_eight`,
`an_i32_payload_is_four_bytes`, `a_target_width_payload_needs_the_target`,
`sizes_are_one_to_eight`); every byte pattern of an integer payload's size is a
value and a `bool`'s byte is not (`every_integer_pattern_is_a_value`,
`a_bool_byte_is_mostly_not_a_bool`, `a_bool_fill_is_a_bool`); and
`WRITE-LEND-TASKS` over typed extents — the typed lease admission is the
untyped one plus "ends inside the payload", every typed-reachable ledger state
is reachable by the untyped ledger, so **no byte has two write authorities**
(`no_byte_has_two_write_authorities_typed`, the exclusivity theorem re-proved,
not weakened), every held byte is a payload byte, a fitting worker footprint
touches only payload bytes, and at `i32` the admission is the four-byte rule
(`an_i32_lease_is_decided_as_before`). The size table is replayed against the
Rust extent (`lean_payload_sizes_match_the_rust_extent`).

## Item 2: fixed-width tasks on the composed epoch route

`source_epoch_tasks::ledger` no longer refuses a fixed-width task half, and the
driver threads the recorded target to it (`compile_task_source(…, target)`) and
publishes it. Nothing the epoch accounts reads a scalar type: a `copy` of any
type is `copy;`, an owned-Buffer result of any payload type is retained, and
every byte figure stays declared. The value statement adds two error paths, and
both are already accounted: a worker's `ArithmeticError` is collected through
`join`, the join-failure edge CT23's recovery plan carries (derived by
`joins_can_fail`); a creator's raising statement is a creator exit, which the
unedited scope rule admits only with no child of the scope outstanding — a
raise with a child outstanding is refused at `Q600 ScopeBusy`
(`a_creator_raise_with_a_child_outstanding_is_refused`). **Lean**
(`QCore.ScalarEpochTasks`, 7 theorems): the derived shape and the plan–ledger
agreement are type-blind (`the_derived_shape_is_type_blind`,
`agreement_is_type_blind`), a raising worker is a join-failure edge, and an
admitted creator exit abandons no child credit
(`an_admitted_creator_exit_abandons_no_child_credit`).

## Item 3: fixed-width outcomes in the F02 alphabet

`ObservedOutcome::Fixed { ty, value }`; `reference_trace` encodes a
fixed-width worker outcome instead of refusing it, and `parse_observed_trace`
reads `QCORE_TASK_RESULT TASK TYPE NUMBER` (`true`/`false` for `BOOL`; a number
outside its type and an unknown name do not parse). **Lean**
(`QCore.TaskTraceFixedOutcomes`, 12 theorems), by CT03-PROOF-ALPHABET's method
but on a stronger footing: the production relation
(`QCore.TaskTraceWrites.relate`) is **invariant under every faithful, injective
renaming of completion outcomes**, proved through all sixteen order flags and
the three R-conjuncts (`relate_is_invariant_under_faithful_renaming`). The
extended relation is the production relation over an injective, faithful
encoding of the extended alphabet (`encode_is_injective`,
`encode_is_faithful`), so its verdicts do not depend on the encoding chosen
(`xrelate_does_not_depend_on_the_encoding`); **a trace without a fixed outcome
relates exactly as before**, for arbitrary traces
(`fixed_free_traces_relate_exactly_as_before`); and it is not vacuous: a run
returning `u8` 255 is admitted, and the same run reporting 254, 255 at `u16`,
or an `i32` 255 is refused with class `3`. The adapter was deliberately **not**
unified: its fixed-result arm stays installed only where needed, so neither
pinned F02 C digest moves (the follow-up had sized a re-pin; it is not needed
for the encoding).

## Item 4: the typed tier on the composed string/buffer route

A composed module reads a top-level conversion and value statement with
`source.buffer.v1`'s reader (`source_std_buffer::value_statement`, which claims
only those two shapes, so none of the string route's own `let` forms is
stolen); its loops of any integer index type and their bodies' statements were
already that route's. All of them are decided by `BufferRun`/`BufferEvaluation`
over the typed environment under the recorded target
(`check_string_program_for_target`, `lower_source_string_for_target`); a typed
name renders at its type and refuses at an `i32` position of the string route
by name (`Q077`, "`n` is a `u8` and this position takes an `i32`");
`StringCertificate` publishes the typed environment and the target and the
recheck compares them. The emitter keeps one
`std_buffer_c_emit::EmbeddedTyped` for the whole entry, so a top-level
statement and every body see the same typed names, and defines every helper
once. **Lean** (`QCore.ScalarComposedString`, 8 theorems): the decision is the
buffer route's, per statement and per iteration; a typed statement never
conflicts with a live loan and the window of a body — at every iteration of an
unrolled loop — is its typed-free skeleton's
(`an_unrolled_window_ignores_typed_statements`), non-vacuous both ways.

## Item 5: the emitted C under `-Werror`

* **`W48-QR-LOOP-BOUND-NAME-UNUSED-C`.** `emit_events` discards
  (`(void) s_NAME;`) a scalar that nothing after it in its own C block names,
  decided on the emitted text — an `i32` loop bound named only by its loop, an
  unused top-level binding, an unused body binding — on `source.buffer.v1` and,
  through `EmbeddedTyped`, the composed route. A read binding gets none, so
  every program whose bindings are read keeps its bytes. The measured shape is
  wider than R5's: an unused **body** binding (`let j = i as u8;`) emitted C
  that `-Werror` refused at the base too, on the buffer route alone.
* **`W48-QR-INLINE-COMPARISON-WERROR`.** `Helper::Compare(cmp, ty)`,
  `static bool q_scalar_CMP_T(T a, T b)`, is every value-statement comparison on
  the scalar, extern and task routes, R5's buffer-route form shared.

**Census** ([`werror_census.py`](werror_census.py)): every tracked fixture
whose `emit` exits 0 compiled with `clang -std=c11 -Wall -Wextra -Werror
-pedantic -c` — **154 / 154** at the base and **154 / 154** at the tip
([base](artifacts/werror-census-base.json), [tip](artifacts/werror-census-tip.json)):
neither defect had a tracked instance; both are exercised by
`compiler/tests/emitted_c_werror.rs` and
`scalar_extern_values.rs::a_self_comparison_builds_under_werror_and_runs`.

## Positions

| Item | Position | Status |
|---|---|---|
| 1 | `Buffer` payload of every type (`own`/`borrow`/`lend<Buffer<T>>`), typed lease extents, `protected TYPE` | **done**; `W48-QR-TYPES-PAYLOADS` retired |
| 2 | the composed epoch route's task half | **done**; `W48-QR-TYPES-EPOCH-TASKS` retired |
| 3 | a fixed-width worker outcome in the F02 alphabet | **done**; `W48-QR-F02-FIXED-OUTCOMES` retired |
| 4 | the typed tier on the composed string/buffer route | **done**; `W48-QR-TYPES-COMPOSED-STRING` retired |
| 5 | an unused buffer-route binding; an inline comparison | **done**; both retired |
| — | a typed Buffer as an error variant's payload | refused by name (`Q300`), new `W48-QR-TYPES-ERROR-PAYLOAD` |
| — | a typed payload event in the F02 alphabet | refused by the encoding, new `W48-QR-F02-TYPED-PAYLOADS` |
| — | a byte write into a `bool` payload | refused (`Q600`) — a rule, not a position: no byte but 0/1 is a `bool` |

## Corrections

| What | Before | After | Reason |
|---|---|---|---|
| `compiler/tests/scalar_loop_bounds.rs::the_composed_string_route_refuses_typed_loops` | pinned R5's `Q077` refusal | renamed `…_admits_typed_loops`, asserts admission | item 4 discharges the refusal it pinned |
| `examples/scalar/fixed_width.qr`, `examples/scalar/typed_workers.qr` emitted C | `(v2 < v0)`, `(… > …)` inline | `q_scalar_lt_u8(v2, v0)`, `q_scalar_gt_i64(…)` | item 5's comparison function; no other fixture moved |
| an unused body binding on `source.buffer.v1` (probe `u2`, untracked) | `check` 0, `emit` 0, C refused by `-Werror` at the base | discarded, builds | measured beyond R5's loop-bound shape; same fix |
| test constructions of `ScalarType::{OwnedBuffer, ReadLoan, WriteLease}` (`task_execution/tests.rs` ×5, `read_loans.rs` ×1) and six test encoders of `ObservedOutcome` | unit variants; four outcome arms | `OWNED_BUFFER` etc.; a `Fixed` arm refusing like `Error` | the new payload field and outcome; no behaviour change |

## Evidence

**Fixture identity.** The first half's `freeze_fixtures.py` over every tracked
`.qr`/`.qv`/`.qrlib` file, debug `quirin` at the base (`11891c95…`, 632 files)
and at the tip (`6564711f…`) ([BEFORE](artifacts/fixtures-before.json),
[AFTER](artifacts/fixtures-after.json), [comparison](artifacts/fixtures-compare.txt)):
**630 of 632 files are byte-identical** in the exit, stdout and stderr of both
`check` and `emit`; the two others differ only in their emitted C, by item 5's
comparison function (above). Every `i32`-payload program — every task, lease,
loan, error-payload and mutex fixture — keeps its bytes, and so do the F02
fixtures. No existing refusal became an admission and no admission a refusal
among tracked files.

**Native** (Clang 21.1.8; each program at `-O0` and `-O2` under
`-fsanitize=undefined,integer -fno-sanitize-recover=all` and at plain `-O2`,
all with `-Wall -Wextra -Werror -pedantic`; [summary](artifacts/native-payloads.json)):

| Table | Per build | Result |
|---|---|---|
| every type × every boundary value (57 values: 7 per signed type, 4 per unsigned, `false`/`true`): the payload moved to a worker that reads it and writes the next boundary value, returned through `join`, lent to a reader, read and released; and every type's payload split between two disjoint byte leases (one for a one-byte payload, a read-only lease for `bool`), each worker writing one byte, then read byte by byte and whole by the owner | **114** programs, **342** runs, **6,129** trace lines (`scalar_payloads.rs`) | stdout byte-identical to the reference trace, the whole value after the byte writes equal to an independent little-endian reconstruction, exit 0, **0** sanitizer reports |
| the spawn-failure control: a `u64::MAX` payload restored to its owner when creation fails, then released | 1 program | the reference's trace, exit 0 |
| every `protected TYPE` (11) under the mutex observation sink | 11 programs | the typed reference observation line for line; a `bool` cell reads 0/1 |
| composed epoch task halves (`u64`, `i8`, `usize`; the overflow of `u64` and of `i8` through `join`) | 5 programs (`scalar_epoch_tasks.rs`) | the reference trace; the overflow reaches the creator as `ArithmeticError::Overflow` |
| every non-`i32` worker outcome through F02 | 10 programs (`scalar_f02_outcomes.rs`) | the native stdout parses and **relates** to the reference's admitted order |
| every integer type's composed string/buffer program | 10 programs (`scalar_composed_string.rs`) | both streams equal to the reference |
| the unused-binding shapes and the self-comparisons on the scalar, task and extern routes | 6 programs (`emitted_c_werror.rs`, `scalar_extern_values.rs`) | build under `-Werror`, the reference's output |

**Negative controls** (each a test row): a write of another type, a literal
outside the payload type, an `i32` annotation of a typed read, an owner byte
read past a one-byte payload, a typed payload in an error variant, a capture
into another payload type by move and by loan, a byte write into a `bool`
payload, an `isize`/`usize` payload and a `protected usize` with no recorded
target, a lease one byte past every type's size; a changed fixed-width outcome
(number, type, and an `i32` in its place) and six malformed outcome lines; an
epoch plan disagreeing with a typed region and a creator raise with a child
outstanding; on the composed route a typed name at a slice bound, a folded
fault, `hex` of a `bool`, a typed byte index, a target width with no record and
a typed loop's store into a live loan (`Q078`).

**Lean** (four new audit files, **42** theorems, no placeholder, no project
axiom; named above). The binding to Rust is the vector tables and the native
tables, not an extraction theorem.

## Prediction and outcome

| Measure | Predicted | Outcome |
|---|---|---|
| fixtures AFTER vs BEFORE (632) | byte-identical except new examples, 75% | **630 identical**; 2 examples moved by item 5's comparison function (not predicted: the prediction was written before item 5's form was chosen) |
| existing tests changed | only IR constructions, 60% | IR constructions, six outcome encoders, and one R5 test pinning the refusal item 4 discharges |
| F02-pinned traces and digests | unmoved, 90% | **unmoved** |
| native tables under UBSan | all agree, 0 reports, 75% | **all agree, 0 reports** |
| Lean | one new audit file, 85% | **four** audit files, **42** theorems |
| corpus arbiter | byte-identical to the base's archive, 97% | **matches** the base's own archived AFTER, `8642ec02…` (CG05's; the brief's `ae17152b…` was superseded on main) |
| standing gate | green, new tests only, 60% | **green**: the first run had tests 0 and clippy 101 (two lints in new code); fixed and fmt, clippy and the full test suite rerun; +27 tests |
| items completed | 1 (70%), 2 (30%), 3 (25%), 4 (15%), 5 (20%) | **all five** |

## Gate

| Step | Command | Exit | Count |
|---|---|---|---|
| submodule | `git submodule update --init --recursive` | 0 | |
| fmt | `cargo fmt --all -- --check` | 0 | rerun at `b1410565` |
| tests | `python3 scripts/run_test_binaries.py` | 0 | 383 units, 396 summaries; **6,745 passed, 0 failed, 11 ignored** (`private/test-binaries/20260929T215155Z-3081128/summary.json`, at `b1410565`; the first run at `7b0fff4a` gave the same totals); no load-sensitive rerun was needed |
| clippy | `cargo clippy --all-targets -- -D warnings` | 0 | first run at `7b0fff4a` **101** (`unnecessary_lazy_evaluations` in `task_execution.rs`, then `get_last_with_len` in `source_tasks.rs`); fixed, rerun 0 at `b1410565` |
| probe | `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | |
| follow-up probe | `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | |
| Lean | `python3 scripts/check_lean_audits.py` | 0 | **242 audits** (238 + the four new files) |
| provider tests | `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast` | 0 | **129 passed, 0 failed, 1 ignored** (43 summaries) — run as the brief asks; `providers/` is unchanged |
| provider fmt | `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | 0 | |
| provider clippy | `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings` | 0 | |
| arbiter | `run_pinned_audit.py` (below) | 0 | |
| matrix | `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/w48-qr-scalar-types-r6/gate.json` | 0 | after the ledger, 631 → 627 |
| docs index | `python3 scripts/build_docs_index.py` | 0 | |
| Python | `python3 -m unittest discover -s scripts/tests` | 0 | **365 tests OK** (after the matrix regeneration) |
| clean | `cargo clean --profile dev` | 0 | |

Exits in [gate/](gate/), record in [gate.json](gate.json); logs under
`private/w48-r6/gate/`. The six new and extended test files add 27 tests; the
base's full count was not remeasured here. Clang 21.1.8 recorded in this
worktree ([gate/clang-version.txt](gate/clang-version.txt)). The fixture
freezes used the debug `quirin` built at the base (`11891c95…`) and at
`7b0fff4a` (`6564711f…`); later source commits are the two lint fixes only.

## Corpus arbiter

`QCORE_TYPED_PATH_PARITY=OUT/parity.jsonl python3 scripts/run_pinned_audit.py
OUT b1410565 --inputs-root …/build/cycle19 --jobs 8`, AFTER only, with this
card's release `quirin-audit` (`e6d5df27…`, built from `b1410565`): exit 0.
`resource-diagnostics.json` sha256 `8642ec029ec5e98a…` **matches** the base's
own archived AFTER — CG05-NESTED-CALL-EFFECTS's corpus AFTER on the merged base
(567 / 10 / 4,257, `8642ec02…`; only the 8-hex prefix is archived, in its README
and gate.json) — which superseded the brief's `ae17152b…` (the w47-round9
archive) when the Clang-gap cards moved corpus figures on main; the two sidecars
are byte-identical to every archive since R3: `resource-diagnostics.chunk-contracts.json`
`cbbcfd9c…`, `monkey-source.json` `57acfc58…`
([identity](artifacts/arbiter-corpus.identity)). No C-audit path was touched.

## Follow-ups

Ledger ([followups.json](followups.json), applied by the idempotent
[`apply_ledger.py`](apply_ledger.py)): retires `W48-QR-TYPES-PAYLOADS`,
`W48-QR-TYPES-EPOCH-TASKS`, `W48-QR-F02-FIXED-OUTCOMES`,
`W48-QR-TYPES-COMPOSED-STRING`, `W48-QR-LOOP-BOUND-NAME-UNUSED-C` and
`W48-QR-INLINE-COMPARISON-WERROR`; registers `W48-QR-TYPES-ERROR-PAYLOAD` (a
typed Buffer as an error variant's payload) and `W48-QR-F02-TYPED-PAYLOADS`
(typed payload events in the F02 alphabet); amends `W48-QR-TYPES-MSVC-NATIVE`.
631 → 627.

## Files likely to conflict with cards 1–4

`crates/qcore-core/src/task_execution.rs` (`ScalarType::{OwnedBuffer, ReadLoan,
WriteLease}(payload)`, `payload_extent`, the checker's `AbstractValue` payloads,
`buffer_payloads`), `task_execution/reference.rs` (typed cells and events),
`task_c_emit.rs` (`typed_payload_runtime`, the comparison helper),
`task_trace_relation.rs` (`ObservedOutcome::Fixed`), `scalar_types.rs`
(`size_bytes`, `object_bytes`), `scalar_calls_c_emit.rs` (`Helper::Compare`),
`extern_c_emit.rs`, `diagnostic_log_c_emit.rs`, `std_buffer_c_emit.rs`
(`EmbeddedTyped`, the `emit_events` discard), `std_string.rs` and
`std_string_c_emit.rs` (the typed tier), `mutex_c_emit.rs`;
`crates/qcore-frontend/src/source_tasks.rs` (`Scalar::{OwnedBuffer, ReadLoan,
WriteLease}(payload)`, `buffer_payload`), `source_epoch_tasks.rs`,
`source_std_string.rs`, `source_std_buffer.rs` (`value_statement`),
`source_mutex.rs`, `frontend.rs` (`protected TYPE` in name resolution);
`crates/qcore-verification/src/driver.rs` (targets to the string, epoch and
mutex routes); implemented grammars §G02t/§G02b/§G02c, specification
§L14.0/§L20/§L21, `docs/agents/diagnostics.md`, `docs/agents/buffers-records.md`;
`proof/QCore.lean`. Card 3 (task record captures) meets this card in the task
route's payload-typed values and captures; card 2 (usable records) in the
buffer route's emitter.

## Scripts

* The first half's `freeze_fixtures.py` and `compare_fixtures.py`, unchanged.
* [`werror_census.py`](werror_census.py) — compiles every emitting fixture's C
  under `-Werror`.
* [`apply_ledger.py`](apply_ledger.py) — applies `followups.json`, idempotently.
