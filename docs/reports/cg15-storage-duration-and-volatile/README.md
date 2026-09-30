# CG15-STORAGE-DURATION-AND-VOLATILE

Card CG15 of the Clang gap queue (wave 48, round 7), gap entries M24 and M23,
routes G and T. Gateway rule [11h](../../agents/gateway.md#constraint-11h).
Base local `main` `c374de96` (= `origin/main`); `main` moved to `b59aaae2`
(CG07) during the card, which merged it at `09e61220` and re-measured (§4). Evidence Q10, Q11, Q12, Q16, Q29 and Q39 of the
[supplied analysis](../../reviews/clang-gap-2026-09-29/supplied/).

## 1. What changed

New logic lives in [`storage_duration.rs`](../../../crates/qcore-c-gateway/src/storage_duration.rs)
(the storage reader, the decision table `FORM_TABLE`, the static-address
reader, the `volatile` facts and trace) and the route-G adapter
[`legacy_storage_duration.rs`](../../../crates/qcore-c-gateway/src/legacy_storage_duration.rs).
Thin hooks:

| Hook | Where | What |
|---|---|---|
| `static_duration_declaration` | `legacy_c.rs`, statement walker, asked first | a `DeclStmt` whose every declarator is static or thread duration (a tag it defines too) is `NoOp` |
| `settle_volatile` | `legacy_c.rs`, after `CfgBuilder::build` | `volatile` bindings refused, known values forgotten, refinements dropped, custody decisions refused |
| `static_address_return` | `legacy_c.rs`, `return_expression_instruction`, asked last | a returned static address carries no identity |
| `static_address_argument` | `legacy_call_argument_access.rs`, after the typed actual | the same for a borrowed or data-tail actual |
| `mask_static_declarators` | `typed_field_access.rs::bound_names` | the lexical closure does not count a static initializer as a write |
| static initializer skip | `typed_root_declarations.rs` (`VarDecl` arm) | the AST closure does not either |
| `T_STATIC_REFUSAL` | `typed_source.rs`, statement arm | T names the static-duration refusal |

**Shared foundations:** none. `elaboration.rs`, `typed_place.rs` and
`call_instantiation.rs` are untouched; the elaborator's `("volatile access",
Refused("CG15"))` row stays refused (its route never needed it: see §5).

## 2. BEFORE and the cited claims

BEFORE (`private/cg15/before`, exit 0) reproduces the round's baseline byte
for byte: `resource-diagnostics.json` `3a25c4e9…`, 608 Safe / 10 provisional
Unsafe / 4,216 Unknown; system 41 / 2 / 191 (`f9deddc3…`); tail-pause slice
28 / 1 / 240 / 1 Pending (`a71501d2…`). Identities in
[artifacts/identity.json](artifacts/identity.json).

| Claim | Reproduced? |
|---|---|
| M24: T rejects global references and locals with a storage class (Q10–Q12) | **Yes**: "global or unresolved local reference"; a static local reached T's generic "unmodeled callback statement DeclStmt" (its `DeclStmt` arm requires no storage class) |
| M24: G's global handling is not a value model for storage lifetimes (Q16, Q39) | **Yes, and stronger than stated**: a braced static declaration was refused, and a plain one (`static void *cache = 0;`) was lowered as a store on **every** call, so `if (cache) q_free(cache);` was `Safe` by pruning; Q39's reader counted the constant initializer as a binding |
| M23: T rejects `volatile` (Q10–Q12) | **Yes**: "qualified access requires additional synchronization/alias evidence" |
| M23: G's custody-neutral handling guarantees no volatile I/O order (Q16) | **Yes**; and G kept bindings and null facts on a `volatile` object: `void * volatile p = q_alloc(); void *q = p; q_free(q);` and `if (p) q_free(p);` were `Safe` |
| Q29: the contract model has no storage or registry vocabulary | **Yes** (`contract.rs` unchanged since the cited revision, `88873acd…`) |

`typed_source.rs` and `typed_access_path.rs` changed after the cited
revision (their line numbers moved); the claims were reproduced by behaviour
(the CG00 controls in `tests/clang_gap_phase2.rs` and the probes in §6), not
by line. No claim failed to reproduce.

## 3. Prediction against outcome

[PREDICTION.md](PREDICTION.md) was committed after BEFORE and before any AFTER
measurement, but after the hooks had been exercised on synthetic fixtures (it
says so). Re-derived list: 8 functions freed alone (CG00's 7 plus
`rd_kafka_features2str`).

| Measure | Predicted | Outcome |
|---|---|---|
| Unknown → Safe, corpus | 6 (band 3–10) | **7** |
| of the re-derived 8 | ≥ 5 | **6** (the five name tables, `rd_kafka_broker_name`) |
| `rd_kafka_fetch_pos2str` | uncertain | stays Unknown (its `rd_snprintf` also formats a by-value record member and a nested call's result) |
| `rd_kafka_features2str` | stays Unknown | stays (pointer arithmetic on the buffer, a store through it) |
| Safe → non-Safe | 0 (0–3) | **0** |
| provisional Unsafe added | 0 (0–3) | **0**, no leak or violation leaf added |
| M24 leaves removed | 70–110 of 132 | **80** (all 65 declarations, 15 path leaves) |
| M23 leaf | stays | stays |
| system | 0–3 | **+1** |
| tail-pause slice | 0–2 | **0** (byte-identical) |

The seventh corpus gain, `LZ4F_getErrorName`, was outside the list: its
`static const char *codeError = "…";` was not refused but lowered per call as
a binding to the literal, so its `return codeError;` was an unresolved alias.

## 4. AFTER, attributed

AFTER2 (`private/cg15/after2`, binary from `dd84ce7b`'s sources): corpus
**615 / 10 / 4,209** (`3964371f…`), system **42 / 2 / 190** (`41b7eadd…`),
tail-pause slice 28 / 1 / 240 / 1 Pending, byte-identical to BEFORE.
AFTER1 (`1937b10f`, before a statement defining its record tag was admitted)
had the same statuses; the seven functions whose leaves differ between the
two are the tag-defining tables. AFTER3 (`8b556dc0`: the final sources, the
gate's two repairs included, less a `#[cfg(test)]` on a function the release
build never calls) is byte-identical to AFTER2 in every corpus artifact.

| Function | Move | Cause |
|---|---|---|
| `rd_kafka_AclOperation_name`, `…AclPermissionType_name`, `…ConfigSource_name`, `…ResourcePatternType_name`, `…ResourceType_name` | Unknown → Safe | the braced `static const char *names[]` is no event (2) and neither closure counts its initializer, so `return names[x];` is the typed reader's borrowed read |
| `rd_kafka_broker_name` | Unknown → Safe | `ret[reti]` of a thread-local `char[4][N]` is a static address: the `rd_snprintf` actual and the `return` (3) |
| `LZ4F_getErrorName` | Unknown → Safe | the static pointer's initializer no longer binds it per call; `return codeError;` reads static storage (3) |
| system `flb_engine_adaptive_flush_interval` | Unknown → Safe | `static const double multipliers[] = {…}` is no event |

81 more corpus functions (all Unknown) change leaves only
([artifacts/compare-corpus.json.gz](artifacts/compare-corpus.json.gz)): every
change is a removed refusal (braced declarations −65, unresolved aliases −22,
assignments −13, returns −11, indirect accesses −9, four others −1 each),
except three, read: `rd_sockaddr2str` and `rd_kafka_msgset_writer_write_msgq`
each trade a refusal for an unresolved-alias leaf on a name already
unresolved in the function (a static-address actual and a callee's table now
admitted expose the next premise), and `rd_kafka_desensitize_str`, whose
`volatile char *volatile s` cursor now carries two refusals of rule 11h in
place of four older ones. No Safe → non-Safe, so no correction. Two system
functions lose leaves by the same causes; the slice does not move.

Remaining M24 ([artifacts/population-after2.json](artifacts/population-after2.json)):
**52 leaves on 17 Unknown functions**, all path leaves. Read: 29 read an
element of an untagged static table (`static const struct { … } t[]`, refused
by the typed place, and mostly in loops with increments), 3 write through or
step the formatter buffer (`*ret[reti] = …`, `ret[reti] + of`, a `memcpy`
destination), 18 sit in debug-macro actuals, sub-object stores and nested
calls. M23's one leaf (`rd_kafka_desensitize_str`) stays.

### On main `b59aaae2`

Main moved during the card (CG07-GENERIC-AND-STATEMENT-EXPRESSIONS landed);
the card merged it at `09e61220` (conflicts only in the shared records and the
two rule, router and Lean import lists, each resolved as a union) and
re-measured. BEFORE with a binary built at `b59aaae2`: corpus 608 / 10 /
4,216 (`8b9e4047…`, the integrator's figure), system 41 / 2 / 191
(`8a98515b…`), slice 28 / 1 / 240 / 1 Pending (`4e2f5265…`). AFTER on the
merge: corpus **615 / 10 / 4,209** (`66c2f850…`) with exactly the seven
moves and the same leaf deltas on the same 81 other functions as against
`c374de96` ([artifacts/compare-corpus-main.json.gz](artifacts/compare-corpus-main.json.gz),
[artifacts/identity.json](artifacts/identity.json)).

System on the merge **42 / 2 / 190** (`264a553a…`; the same one move,
`flb_engine_adaptive_flush_interval`, and the same two functions losing
leaves); slice byte-identical to its BEFORE (`4e2f5265…`).

## 5. The rule

Rule 11h's text is the authority; in short:

* **Storage** is Clang's (`storage_of`). A static or thread-duration object
  outlives every frame.
* **Static initialisation is no event** (a soundness repair). The two synthetic
  wrong Safes are pinned in `tests/storage_duration_volatile.rs`
  (`soundness_repairs_that_were_safe_on_the_base`); no corpus function carried
  one (no Safe moved).
* **A static object's value and address carry no identity**; a custody-taking
  use refuses.
* **A tracked identity stored into static storage** is rule 10's refused
  publication, unchanged: the escape to no named party is never `Safe` and
  never a per-call leak. Rule 10 records why the escape model was withdrawn
  (it promoted a publish-without-release body to `Safe`). A registry contract
  would decide it; the contract model has none (Q29), and one corpus leaf
  (`unittest_fetcher_share_filter_forward`, `ut_mock_rk`) is this form.
* **`volatile`.** The premise and its census are §7. The flow keeps no
  binding, known value or refinement of a `volatile` object; a branch on a
  `volatile` read whose arms differ in custody refuses (a policy: an
  unrefined branch explores both arms, so soundness does not rest on the
  refusal); each access is a trace event with a count and C's order
  (`volatile_trace`, parity rows only). Neutral uses stay admitted: G's
  `*v + *v` and `v->r` controls are unchanged (Safe), and the trace records
  the two reads of `*v + *v` as an unsequenced pair on one object, never
  ordered. The elaborator's volatile row stays refused: the neutral forms
  reach G through the typed scalar route, and the elaborator's order model
  would need a new event variant, which this card did not add.
* **Route T** refuses a static-duration local by name; no T consumer decides
  a lifetime longer than the frame. Global references keep T's refusal.

## 6. Tests, flips and controls

* [`tests/storage_duration_volatile.rs`](../../../crates/qcore-c-gateway/tests/storage_duration_volatile.rs):
  the pinned shapes against Lean's `shapeVectors` (11), the brief's two
  refusal controls (`static void *cache; cache = q_alloc();` is refused by
  name, neither Safe nor a leak; `if (*vp) q_free(p);` is never Safe, and on a
  `volatile` object route G reads the decision is refused by rule 11h), the
  soundness repairs, static addresses, and the three CT41 complement shapes.
* `tests/unit/storage_duration.rs`: `formTable` = `FORM_TABLE`, the qualifier
  and storage readers, the trace's count and order, the declarator mask.
* **Flipped CG00 controls** (`tests/clang_gap_phase2.rs`,
  `m24_static_storage_is_refused_or_unresolved`): T's refusal of
  `static int s;` is now `T_STATIC_REFUSAL` (was the generic `DeclStmt`
  refusal); two flips are added beside it: a braced static table read, and a
  thread-local buffer's address as an actual and a return value, both Safe
  (were refused). The remaining M24 controls (a global publication refused,
  a global release an unresolved alias, a thread-local scalar read Safe) and
  every M23 control hold unchanged.
* **Flipped elsewhere** (found by the first gate run, §11):
  `tests/container_root_reach.rs`'s
  `a_member_of_a_static_record_is_still_refused_before_the_effect`, now
  `a_member_of_a_static_record_reaches_the_container_rule`: the head actual
  `&global_queue.items` is a static address, so `mk_list_add(&e->_head,
  &global_queue.items)` reaches the container rule and discharges into the
  static list exactly as the existing `&registry` case does (Unknown →
  Safe; the refusal was the recorded conservative class of
  `W47-JOIN-AST-CONSERVATIVE-CLASSES`, not a boundary). No corpus function
  has this shape (no corpus Safe beyond the seven).
* **Narrowed, not flipped:** `tests/publication_custody.rs`'s
  `a_volatile_qualifier_moves_no_resource_projection_verdict` (CT-CTWIN-SYNC's
  invariance). Its third shape stores into `struct b_handle *volatile
  g_published`; the first build refused that store as a `volatile` binding,
  replacing rule 10's publication reason. Rule 10 decides a store into a
  static-storage object, so the binding refusal now skips a destination
  there (the binding it leaves keeps no refinement), and the shape is
  invariant again; the test's comment records the narrowed claim.
* The first build also asked Clang for the AST of every function
  (`configured_cli`'s one-invocation-per-stage pin went red): rule 11h's CFG
  pass is now asked only where the unit spells `volatile` up to the body's
  end, which every `volatile` object a function names must be.

## 7. `GATEWAY-VOLATILE-PREMISE`: the census

Every place in `legacy_c.rs` where a value-derived must-fact decides an
admission or a CFG edge, read at `c374de96`:

| Fact | Site | Read from |
|---|---|---|
| null-test arm pruning (an `if` arm deleted; a loop exit edge kept with a refusal) | `refine_edge` → `refine_pointer_null`, the work list's `Infeasible` arms | the tested name's binding |
| descriptor validity pruning | `refine_validity` | the name's binding |
| distinct-pair may-alias discharge | `BranchOnDistinct` | the two names' bindings |
| collection element nullness, counted-loop bound and exit equation | `refine_collection_edge`, `CountedLoopHeader`/`Step` | element places of a `T **` base; the index and bound names |
| a non-nullable release's non-null requirement, a known-null release | `transfer` of `ContractCall`/`Release` | the argument's binding (`may_be_null`) |
| scalar known values | `Clear`, `ForgetScalar` | the name's binding |

Each reads a binding keyed by a name or an element place (an unbound name
refines nothing: `refine_pointer_null` returns `Refined`). Rule 11h reads the
qualifier and removes every one for a `volatile` object named directly
(`settle_volatile`: bindings refused, `Clear` forgotten, the four branch and
loop instructions made unrefined); a `volatile` element is refused by
`indexed_collection` and a `volatile` member by the typed-place, store and
aggregate rules. What remains is the premise for non-`volatile` objects,
recorded in rule 11h beside 5a and 10a: C makes an asynchronous change to one
undefined (a data race, a signal handler's store outside `volatile
sig_atomic_t`), so it is `AOM-POISON-NO-UB`'s. Measured: one corpus function
holds a `volatile` object (`rd_kafka_desensitize_str`, 6 binding rows), one
`volatile` condition is custody-neutral, and the trace records 76 accesses in
46 statements of 4 functions, none an unsequenced same-object pair
([artifacts/parity-corpus-after2.json](artifacts/parity-corpus-after2.json));
the system route has 840 accesses in 3 functions and no `volatile` binding.

## 8. `CT41-VOLATILE-COMPLEMENT-UNMEASURED`: the three shapes

Measured through route G (`the_volatile_complement_shapes_answer_as_measured`):

| Use | Answer |
|---|---|
| a memory-mapped register (`*(volatile unsigned *)ADDR = 1u; while (*(volatile unsigned *)ADDR2 & 1u) { }`, and through a `volatile unsigned *reg`) | **Unknown**: the fixed-address place is no place route G reads (unrecognized destination, loop condition refusal) |
| an object a signal handler writes (`static volatile sig_atomic_t stop_flag`) | a spin-wait on it with custody around it is **Safe** (custody-neutral); a release it decides is **refused** by rule 11h |
| an object live across `setjmp` (`void * volatile p`) | **refused**: the `volatile` binding (rule 11h) and `setjmp` in the condition (uncontracted) |

No CT case asks these; they are recorded, not mapped.

## 9. Lean

[`QCore.StorageDuration`](../../../proof/QCore/StorageDuration.lean), audited
in [`AuditStorageDuration.lean`](../../../proof/AuditStorageDuration.lean),
25 theorems, no placeholders and no project axioms: lifetimes
(`a_static_object_outlives_every_frame`,
`a_thread_local_object_outlives_every_frame`, `outlives_frame_iff`); the
declaration (`a_static_declaration_is_no_event`,
`per_call_initialisation_misjudges_the_second_call`); the escape
(`a_static_store_escapes_the_frame`, `a_static_store_is_refused_not_a_leak`,
`refused_is_sticky`, `run_refused_is_sticky`); static reads
(`a_static_read_has_no_identity`, `a_static_read_changes_no_custody`,
`releasing_a_static_read_refuses`); `volatile`
(`a_volatile_read_need_not_return_the_stored_identity`,
`a_known_value_misjudges_a_volatile_branch`,
`a_custody_decision_on_a_volatile_read_is_refused`,
`a_neutral_volatile_branch_is_sound`); the trace (`trace_count`,
`before_bounded`, `sequenced_accesses_are_ordered`,
`unsequenced_accesses_are_never_ordered`); the table (`forms_unique`,
`unlisted_form_refused`, `atomic_access_is_cg19s`,
`every_refused_row_is_a_custody_form`, `shape_vectors_follow_the_table`).
Bound to Rust by `cg15_lean_form_table_matches_the_rust_table` and
`cg15_shape_vectors_match_lean_and_the_gateway`.

## 10. Ledger

* **Retired** `CG15-STORAGE-DURATION-AND-VOLATILE`: every DISCHARGE clause
  holds (BEFORE reproduced, prediction frozen, every move attributed, the two
  absorbed entries closed, Rust controls and audited Lean).
* **Retired** `GATEWAY-VOLATILE-PREMISE` (both halves of its discharge: the
  census, §7, recorded as a standing premise in rule 11h, and the importer
  reads the qualifier) and `CT41-VOLATILE-COMPLEMENT-UNMEASURED` (the three
  shapes measured and recorded, §8).
* **Registered** `CG15-STATIC-TABLE-RESIDUALS` (52 leaves, 17 functions) and
  `CG15-STATIC-REGISTRY-CONTRACT` (1 leaf).

## 11. Gate

[`scripts/gate.sh`](scripts/gate.sh) on the merged tip `4fc762bb` (code of
`09e61220`), exits in [gate/](gate/), figures in [gate.json](gate.json); logs
in `private/cg15/gate3`.

| Step | Exit | Count |
|---|---|---|
| `git submodule update --init --recursive` | 0 | |
| `python3 scripts/run_test_binaries.py` | **101** | 402 units, 415 summaries, **7,057 passed, 1 failed, 11 ignored** (`private/test-binaries/20260930T202532Z-2782636`) |
| its one failure alone (`cargo test -p qcore-compiler --test cycle20_verifier_plugins`) | 101, 101, then **0** (`03b`–`03d`) | 20 passed on the third |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| `python3 scripts/check_lean_audits.py` | 0 | 257 audits (256 + `AuditStorageDuration.lean`) |
| `python3 -m unittest discover -s scripts/tests` | 1, then **0** after `build_matrix.py . --gate gate.json` (`09a`, `09c` 0; `09b`, `09d` 0) | 365 |
| `python3 scripts/build_docs_index.py` | 0, 0 | |

The runner's one failure is constraint 16's load-sensitive exclusive row
`vp04_vp07_vp08_process_lifecycle_and_payload_budgets` (`elapsed < 500ms`),
in a crate this card does not touch, at host load average 50–68 from the
concurrent cards; checked alone it failed twice more under that load and
passed at the third attempt. It is not weakened. The first Python run failed
only on the matrix's derived Lean audit count (256 → 257), which the
regeneration moves.

Two earlier runs on the card's own tree are kept as evidence
(`private/cg15/gate`, `private/cg15/gate2`). The first (tip `b67715df`)
failed three gateway and audit tests and clippy, each repaired as §6 says
(`container_root_reach`, `publication_custody`, `configured_cli`; a
test-only function), plus the same timing row; the second (the repaired
code, before the merge) passed 401 units, **7,043 passed, 0 failed**, exit 0,
clippy 0, 256 Lean audits, Python 1 only on the audit count. `providers/` is
unchanged, so the provider gate was not due. Clang 21.1.8
([gate/clang-version.txt](gate/clang-version.txt)).
