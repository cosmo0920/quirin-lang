# CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 3: the access refusal, re-measured; rules 67, 68 and a 12h amendment

Card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS, round 3, widened by the user to the
other kinds of indirect memory access. USER DECISION (2026-11-20, verbatim,
through the integrator's brief): "Also, we always blocked indirect-memory
access so we must proceed to jump on it." and "Also, we need to proceed
other types of indirect memory accesses." Base `f916d3007`, merged with main
`ffb41c748` (two record-only commits) and then `b7f86a0de` (r39, provider-only). Gateway [rule 67](../../../../agents/gateway.md#constraint-67)
(lifts rule 14a's one-call limit), [rule 68](../../../../agents/gateway.md#constraint-68),
and [rule 12h](../../../../agents/gateway.md#constraint-12h) amended in place.
Lean `QCore.SiblingBorrowedCalls` (7 audited theorems), `QCore.PointerTypedefMember`
(6), `QCore.CastActualPlace` (4); `QCore.ResidualAccessReadings` amended
(`scalarCallAdmitted`, its vectors; `two_calls_are_never_read` withdrawn).
[PREDICTION.md](PREDICTION.md) was committed before each implementation
commit (revision 1 before rules 67 and 68, revision 2 before the cast reading).

## 1. Census (method step 1)

**Route.** This host has no `build/cycle19` inputs or baselines. The census
and the measurement use the opt-in `quirin-audit current --monkey-linked`
over Fluent Bit 046accf6, exactly as
[`opt-in-current.sh`](../../contracts-and-premises/openssl-ssl-ctx-callback-slots/evidence/opt-in/opt-in-current.sh)
takes it (the side tree's user load path, `SOURCE_DATE_EPOCH=1767225600`,
`--jobs 12`), inputs prepared on this host into `private/fb-046accf6` by
`quirin-audit prepare` (binary `04a697a8…`, a recent main build; `prepare.json`
records it), one side tree (`git archive f916d3007`) for every run, the
binary the only difference. Base binary: the `indirect-census` release build
of `f916d3007` (`2f832bda…`), exit 0, 36 min.

**Base:** **1,139 Safe / 2 Pending / 3,656 Unknown / 7 Unsafe**; **2,198**
Unknown rows carry `indirect memory access is outside the resource
projection` (9,186 leaves: librdkafka 1,688 rows, cmetrics 192, Monkey 146,
ctraces 82, fluent-bit-chunk 50, chunkio 40); **242** carry nothing else.
(AG1 counted 2,285 / 232 on the user's run of an older binary and without
the opt-in OpenSSL load path.)

**AST census** (AG1's scripts, [scripts/](scripts/), paths parameterised):
`prep_units.py` rebuilt all 180 analysed texts sha256-exact; `classify.py`
9,202 leaf groups, 0 unbound; `premise.py`; `refine.py`. Tables:
[census/premise-table.md](census/premise-table.md),
[census/refined-sole-shapes.md](census/refined-sole-shapes.md),
[census/candidates.md](census/candidates.md) (each reading's reach).

**The analyzer's own reading.** A census-feature run forces one job
(`qcore-audit/src/parallel.rs::effective_jobs`: census streams are
process-global), so a full census run would take about nine hours; the first
attempt was stopped after 21 minutes (`private/r3/run-base-census-killed`, not
evidence). The streams come instead from eight parallel `current --selection`
runs (`scripts/split_selection.py`, `scripts/run_current_census.py`, 129
non-Monkey units holding an access row, base binary, all exit 0).
[census/stream-shapes.md](census/stream-shapes.md) joins each base access
leaf to its gate site: **call actuals of an uncontracted callee** dominate
(1,228 + 967 + 378 + 234 + 203 + 155 + … leaves; the lexical gate refuses
before the callee's contract is looked up, W45-CALLEE-CASCADE-RESIDUAL), then
actuals of contracted callees whose member the declaration reader leaves
`Unresolved` (647 + 374), member stores with no call (353 + 102), and the
typedef causes (`TypedefDisagreeing`, `TypedefDefinesAggregate`,
`ArrayDeclarator`). 522 leaves match no site (Monkey, which a selection does
not analyse, and unrecorded sites).

### Table (a): sole-blocker classes on the base, by functions freed alone

| sub-shape | alone | mentioning | owner |
|---|---:|---:|---|
| several sub-shapes | 46 | | |
| uncontracted callee, an actual neither reader reads | 40 | 55 | contracts (W45-CALLEE-CASCADE-RESIDUAL) |
| embedded list address handed to a list helper | 22 | 22 | **decision class 1** / rule 46 |
| pointer member read at an actual | 17 | 20 | rule 67 (two sibling calls); the callee's mode; the return position (section 2.3) |
| formal's or local's pointer stored into a member | 13 | 19 | **decision class 1** (`retained_by`), rule 10v |
| sub-object address at another position | 12 | 17 | callee's mode, interior bounds |
| formal array's elements released | 12 | 12 | rule 5k, by design (AG1-FORMAL-ARRAY-ELEMENTS-RELEASED) |
| owned result into a formal's member | 11 | 14 | MEMBER-EFFECT-STORES |
| `return rd_kafka_op_err_destroy(rd_kafka_op_req(…))` | 8 | 18 | contract (RK-OP-ERR-DESTROY-CONTRACT) |
| call through a member or function pointer | 8 | 10 | rule 11p |
| custody-free read beside `?:`, a cast, a descriptor | 7 | 8 | rules 10z, 38 |
| pointer arithmetic stored into a member | 7 | 8 | rule 10v, by design |
| union member access / pointer store through a union | 5 / 4 | 10 / 18 | rule 10w, by design |
| owned result into a local member | 5 | 8 | B08 premises; rule 68 for a pointer-typedef member |
| 14 smaller sub-shapes | 25 | | each at most 5 |

### Table (b): every Unknown row mentioning the refusal, by premise (base)

| premise | leaf groups | rows mentioning | sole rows (premise alone) | owner |
|---|---:|---:|---:|---|
| `read:pointer-member` | 2,085 | 877 | 42 | the callee's contract or mode; rule 67 (two calls); rule 68 (typedef members) |
| `address:sub-object` | 1,867 | 688 | 35 | uncontracted callees; decision class 1 (embedded lists); callee mode |
| `call:uncontracted-callee` | 1,688 | 711 | 40 | contracts (W45-CALLEE-CASCADE-RESIDUAL) |
| `form:union` | 732 | 273 | 10 | rule 10w (CG13-UNION-LEAVES-OTHER-CAUSES) |
| `store:pointer-name` | 448 | 278 | 13 | decision class 1, rule 10v |
| `store:pointer-call-result` | 367 | 220 | 17 | B08 (rules 61, 68), MEMBER-EFFECT-STORES, W44-CALL-RESULT-INDIRECT-DESTINATION |
| `store:pointer-other` | 354 | 230 | 10 | rule 10v |
| `store:pointer-identity-free` / `-null` | 274 / 215 | 82 / 137 | 0 / 0 | rule 10v's premises (a tracked root) |
| `read:custody-free` | 184 | 112 | 7 | rules 10z, 38 |
| `form:aggregate-value` | 178 | 115 | 5 | rule 11f |
| `root:file-scope` / `root:call-result` / `root:static-local` | 165 / 106 / 17 | 86 / 72 / 9 | 2 / 2 / 1 | CG05-CALL-ROOTED-PLACES, storage duration |
| `form:indirect-call` / `call:indirect-callee` | 155 / 17 | 116 / 13 | 4 / 4 | rule 11p |
| every other premise | 349 | | 6 | |

## 2. Readings (method steps 2-4)

### 2.1 Rule 67: several sibling `Borrowed`-only calls (AG1-TWO-BORROWED-CALLS; lifts rule 14a's limit)

Rule 14a read a scalar statement whose value holds **exactly one** call with a
`None`-result, all-`Borrowed` contract and recorded "**Do not** admit two
calls" (`two_calls_are_never_read`). Rule 67 lifts that limit for **sibling**
calls: in rule 14a's statement shapes and hook, every call (none inside
another's callee or actuals) is read as an opaque scalar by the typed path
reader and lowered alone in source order with W44's reads; the statement is
admitted only when every callee's contract in force is `None`-result and
all-`Borrowed` (no hold), and one call that does not lower restores the
counters and keeps the refusal. **Soundness**: each such call is the identity
on the custody state, so any sequence of them, in any order C's unspecified
evaluation takes (C11 6.5p2, 6.5.2.2p10), with the statement's reads anywhere
between them, decides every read alike; a releasing sibling would not
(`QCore.SiblingBorrowedCalls`: `a_borrowed_sequence_changes_no_custody`,
`every_order_leaves_the_same_state`, `a_read_between_the_calls_is_decided_alike`,
`a_releasing_sibling_does_not_commute`, `a_releasing_sibling_is_never_admitted`,
`several_calls_are_read`, `no_call_is_never_read`; `scalarCallAdmitted`
amended in `QCore.ResidualAccessReadings` and bound to `SCALAR_CALL_VECTORS`).
A nested call (`cmt_atomic_store(&d->f, cmt_atomic_load(&s->f))`,
`cmt_math_uint64_to_d64(cmt_atomic_load(…))`) stays refused.
Code: `residual_access_reading.rs`, `legacy_residual_access.rs`, and the
shared reader field `Reader::opaque_calls` (`typed_access_path.rs`, which
replaces rule 14a's `opaque_call`, read only by rules 14a and 67).

### 2.2 Rule 68: a member declared through a pointer typedef (AG1-POINTER-TYPEDEF-MEMBER)

The lexical declaration reader recorded a pointer typedef
(`typedef char *cfl_sds_t;`) as unresolved, so `cfl_sds_t name;` joined no
member class, and O04-B08's field link (`ins_scope->name = cfl_sds_create(name)`
on a record the body allocated) met the gate where `char *name;` is
admitted. Where every typedef declaration of `A` in the unit is exactly
`typedef SPEC * A;` with one `SPEC` resolving to a scalar or a struct, a
member `A NAME;` (no `*`, no array, no rejecting qualifier) joins the
single-pointer and borrowable classes, as `SPEC * NAME;` does (C11 6.7.8p3).
The shared classifier still answers `A` unresolved for locals, formals, the
scalar no-op rule and the record-pointer continuation. Code:
`pointer_typedef_member.rs`, hooks in `typed_field_access.rs`
(`pointer_typedefs`, one call in `read_typedefs`, one branch in
`read_record_body`, `pointer_typedef_member_pointee`). Lean
`QCore.PointerTypedefMember` (`an_agreeing_alias_member_is_its_pointer_spelling`,
`a_disagreeing_alias_reads_nothing`, `a_pointer_to_the_alias_is_never_single`,
`an_unresolved_pointee_is_never_single`, `decision_vectors_hold`,
`the_decision_is_the_models`). The second half of the follow-up (a pointer to
a defining-typedef **enum**, `rd_kafka_share_ack_batch_entry_new`) is not
read: an enum alias is not a pointer typedef and stays open.

### 2.3 Rule 12h amended: a pointer view of a member at a call actual

Prediction revision 2 planned this as an amendment of rule 14d (W44's actual
reader). Implemented there it read the actual but changed no row: W44
re-classifies the whole statement lexically after the typed actual is
admitted, and the lexical classifier does not parse a cast actual (unit test
`a_pointer_cast_of_a_path_is_read_at_an_actual` passed; the fixtures stayed
refused), so that hook was withdrawn before commit. The reading landed where
casts are already removed: rule 12h's hook H-U removed an identity-preserving
cast of a **plain name** at a value position and lowered the uncast
statement; it now also removes one of a **member-chain place over a name**,
at a **call actual only** (the place hook H-U0 already reads such a cast
as). Code: `legacy_cast_semantics.rs::place_view_removed`. Lean
`QCore.CastActualPlace` (`place_view_vectors_hold`,
`only_a_call_actual_is_uncast`, `a_qualified_view_is_kept`,
`a_removed_view_is_the_place` over `QCore.CastSemantics.view_preserves_identity`).
**Its limits, measured:** the return position has no H-U placement (a
route-map row, out of a card's scope), and the uncast spelling matches no
Clang node, so a Clang-matched reading of the statement (rule 14a's typed
member destination) is not reached; the five sole rows the census found
(`print_string`, `pb_dec_bool`, `pb_enc_fixed_length_bytes`,
`LZ4F_localSaveDict`, `sort_by_map_elem_val_toppar_list_cnt`) are all
`return f((T *)p->m, …)` and stay (follow-up CG10-CAST-VIEW-RETURN-POSITION).

### 2.4 Fixtures

`crates/qcore-c-gateway/tests/indirect_access_sole_residuals_r3.rs` (6
tests, each admission with near-misses): rule 67 admits two and three sibling
calls; a releasing sibling, an owned-result sibling, an uncontracted sibling
and a nested call stay non-`Safe`. Rule 68 admits the B08 store into a
`ps_t name;` member (refused with the hook switched off, checked once by a
test-only switch that was removed before commit); a pointer to the alias and
a `void *` typedef member keep their refusals. Rule 12h's amendment admits a
declaration and an assignment to a scalar local holding a cast member actual;
an integer cast, a decayed array, an offset, a typed member destination, the
return position and a releasing position keep theirs. The round-2 fixture
asserting that two calls keep the gate now points here. Unit tests:
`residual_access_reading.rs` (two sibling calls read, a nested call refused,
`SCALAR_CALL_VECTORS`), `pointer_typedef_member.rs` (3),
`typed_field_access.rs` (`rule_68_reads_a_member_through_an_agreeing_pointer_typedef`),
`legacy_cast_semantics.rs` (`PLACE_VIEW_VECTORS`).

## 3. Measurement (method step 5)

| run | binary | Safe / Pending / Unknown / Unsafe | access rows | access alone | access leaves |
|---|---|---|---:|---:|---:|
| base `f916d3007` | `2f832bda…` | 1,139 / 2 / 3,656 / 7 | 2,198 | 242 | 9,186 |
| tip 1 (rules 67, 68; `9b4bc52e6`) | `2a16dd32…` | 1,140 / 2 / 3,655 / 7 | | | |
| **tip 2 (+ rule 12h amended; `1379d031a`)** | `cdcbf730…` | **1,140 / 2 / 3,655 / 7** | **2,190** | **240** | **9,149** |

Every run exit 0, 181 configurations, 141 checker bindings all `Pending`
before and after. Comparison by AG1's `compare.py`
([census/compare-tip2.json](census/compare-tip2.json),
[census/compare-tip1-tip2.json](census/compare-tip1-tip2.json)):

* **One move: `cio_stream.c::get_stream_path` Unknown -> Safe** (rule 67,
  `len = strlen(ctx->options.root_path) + strlen(st->name) + 2;`, predicted).
  It rests on `strlen`'s declared assumption (Safe rows on a declared
  assumption 523 -> 524; no declaration introduced by this card).
* **No Safe -> non-Safe move, no new Unsafe, no row Unsafe or Pending moved.**
* Leaves ([census/attribution.md](census/attribution.md), `scripts/attribute.py`):
  55 rows change leaves; access leaves 9,186 -> 9,149 (37 removed, none
  added). Rule 68: 14 rows lose 28 access leaves (cmetrics and ctraces
  constructors and label helpers storing or reading `cfl_sds_t` members) and
  35 more rows lose only `unresolved resource identity` leaves (`opts`,
  `label`, `label_k`, `span`, … : the identities whose member stores are now
  read); rule 67: 5 rows, 5 access leaves; rule 12h's amendment: 1 row, 1
  leaf (`rd_kafka_topic_info_new`, re-attributed to `unresolved resource
  identity ti`). Added leaves, every one on a row that stays Unknown: 4 `loop
  re-acquisition`, 9 `unresolved resource identity` (`new_static_label`,
  `ins_scope`, `new_label`, `ev`, `opt`, `ti`), `call to uncontracted function`
  `compare` (`compare_fqname`, a function-pointer callee now reached),
  `flb_input_log_append_processed`, `input_chunk_append_raw`, and one rule
  11c `field-link escape`. These are **re-attributions**: the statements are
  now read and the next refusal is honest.
* Sole set 242 -> 240: `get_stream_path` (Safe), `ctr_instrumentation_scope_create`
  (re-attributed to `ins_scope`, prediction missed) and `compare_fqname`
  (re-attributed to the uncontracted function-pointer callee `compare`) leave
  it; `pack_static_labels` joins it (its other leaves were rule 68's; what is
  left is an embedded list address, decision class 1).

**Against the prediction.** Revision 1: `get_stream_path` moved as
predicted; `ctr_instrumentation_scope_create` did not (its stores are read
and the base identity is the next refusal); rule 67's predicted 0-2 is 1, rule
68's predicted 1-3 is 0; access leaves removed 37 against a predicted 50-300
(the "all contracted" class is dominated by `rd_kafka_dbg`/`rd_rkb_log` macro
blocks whose callees include the held logger `rd_kafka_log0` and by nested
calls, neither read); access-alone 240 against 239-241; no Safe -> non-Safe
and no new Unsafe, as predicted. Revision 2 (the cast reading): `print_string`
did not move (predicted 1-3): all five census rows are returns, which no H-U
placement reaches (section 2.3); 1 row re-attributed.

**Pinned routes (corpus, system, slice).** `quirin-audit measure --declared`
cannot run on this host (no prepared pinned-route inputs or baselines). The
prediction for the integrator, in PREDICTION.md's revision-2 block: corpus
`get_stream_path` Unknown -> Safe where the corpus analyses chunkio's
`cio_stream.c` (revision 2's `print_string` line is withdrawn by this
measurement: the integrator should expect it **missing**); twelve-home the
same; system and slice none. Leaf changes are expected on corpus rows of
cmetrics and ctraces (rule 68) and of chunkio (rule 67).

## 4. What stays refused, and why (method step 6)

On the tip ([census/tip2-refined-sole-shapes.md](census/tip2-refined-sole-shapes.md),
[census/tip2-premise-table.md](census/tip2-premise-table.md)): **240
functions blocked by the access refusal alone**; by class, functions freed
alone:

| class | alone | owner, and why it stays |
|---|---:|---|
| uncontracted callee, an actual neither reader reads | 40 | **contract**: `cmt_mpack_consume_string_tag`/`ctr_mpack_consume_string_tag` (15 `unpack_*`), `rd_kafka_log0`, `rd_kafka_timer_start0`, `rd_list_copy`; W45-CALLEE-CASCADE-RESIDUAL |
| embedded list address | 23 | **decision class 1** (container custody, rule 46) |
| pointer member read at an actual | 16 | the return position for a cast (CG10-CAST-VIEW-RETURN-POSITION), the callee's mode (`open(cf->path, …)` descriptor cells, rule 38), a nested call (AG1-TWO-BORROWED-CALLS' nested half) |
| pointer stored into a member | 13 | **decision class 1** (`retained_by`), rule 10v (Q36's missing owner premise), by design |
| sub-object address at another position | 12 | the callee's mode and interior bounds (`memset(cctx->hashTable, …)`, `rd_kafka_confval_set_type(&options->broker, …)`) |
| formal array's elements released | 12 | rule 5k, by design (AG1-FORMAL-ARRAY-ELEMENTS-RELEASED, a user decision) |
| owned result into a formal's member | 11 | MEMBER-EFFECT-STORES |
| mock API nested return | 8 | **contract**: RK-OP-ERR-DESTROY-CONTRACT |
| call through a member or function pointer | 7 | rule 11p (an open candidate set) |
| `?:`, cast, descriptor beside a custody-free read | 7 | rules 10z, 38 |
| pointer arithmetic into a member | 7 | rule 10v, by design |
| union access / pointer store through a union | 5 / 4 | rule 10w, by design |
| owned result into a local member | 5 | B08's other premises (a union member record, a deep chain) |
| 14 smaller classes | 25 | each at most 5 |
| two or more classes | 45 | |

Every class above three rows is a design refusal (rules 5k, 10v, 10w), a
decision class (1, 3), a contract question or MEMBER-EFFECT-STORES, except
**pointer member read at an actual** (16), whose largest readable part is the
return-position cast view (5, a route-map placement) and **sub-object address**
(12, the callee's mode). The entry is therefore **amended, not retired**.

**Decision questions for the user** (not decided here):

1. *Decision class 1, embedded lists* (23 rows blocked alone, 22 at the base;
   `rd_list_add(&c->config, e)`, `rd_list_destroy(&conf->interceptors.on_new)`,
   `pack_static_labels`): should a list helper handed the address of a list
   embedded in a record the body does not own be read as a borrowed container
   operation (the list's elements stay the container's), or keep the refusal
   until container custody (rule 46) states it?
2. *Decision class 1, `retained_by`* (13 rows alone, 19 mentioning:
   `pmi->member = member`, `cio_set_log_callback`): should a formal's or local's
   pointer stored into a member be read as a retention the caller must honour
   (`retained_by`), or keep the refusal?
3. *Decision class 3, reference-count keeps* (1 row alone,
   `rd_kafka_op_new_reply`'s `rd_kafka_toppar_keep0` stored into `rko->rko_rktp`):
   unchanged, named only.

## 5. Gate

Scoped card gate `python3 scripts/gate_scope.py --base b7f86a0de --out
private/gate-4` through `private/toolchain/env.sh` (Clang 21, Lean 4.35) on
`37da2d79a` (the card merged with main `b7f86a0de`, r39
OPENSSL-FREE-INDIRECT-CALLBACKS, provider-only; earlier main `ffb41c748`
merged as a fast-forward); mode full (the diff reaches `qcore-c-gateway`).
**Exit 0, `gate_scope: 8 steps passed`** (record [gate/gate-4.json](gate/gate-4.json),
logs in the worktree's `private/gate-4/`):

| step | exit | result |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `run_test_binaries.py` | 0 | 667 units, 712 summaries; 9,788 passed, 0 failed, 21 ignored, 148 filtered |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `check_lean_audits.py` | 0 | 465 audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | 779 tests |

No flaky rerun was needed. Earlier attempts, not evidence: `private/gate-1`
stopped at clippy (a collapsible `if`, a test module before items; fixed with no semantic change); `private/gate-2` (base
`ffb41c748`) exit 1 only for two pinned fixtures this card moves deliberately
(`sub_object_stores`' two-call row now `Safe` under rule 67;
`borrowed_field_actuals`' cast row now refused at its next leaf under rule
12h's amendment, still Unknown), updated in `fa14b0565`; every other step
exit 0 (Lean 464 audits, Python 779); `private/gate-3` stopped when main
moved. The measured binaries are of `9b4bc52e6` and `1379d031a`; later code
commits are the lint fix (a let chain, a moved test module) and fixture
updates, no semantic change. The gate read census
(`gate_read_census.py record`) was **not** re-recorded for the new test
target `qcore-c-gateway::test::indirect_access_sole_residuals_r3` (the census
test passes; recording traces every gateway test executable under `strace`
and was not run on this contended host): left to the integrator.

## 6. Follow-ups (in [integration-amendments.json](integration-amendments.json))

* **CG10-INDIRECT-ACCESS-SOLE-RESIDUALS** amended with this round's resize
  (not retired: section 4).
* **AG1-TWO-BORROWED-CALLS** amended: the sibling half discharged by rule 67;
  the nested half (`cmt_atomic_store(&d->f, cmt_atomic_load(&s->f))`) open.
* **AG1-POINTER-TYPEDEF-MEMBER** amended: the pointer-typedef half discharged
  by rule 68; the defining-typedef enum pointer half open.
* **CG10-CAST-VIEW-RETURN-POSITION** (new): a pointer view of a member at an
  actual of a returned call, and at a statement a Clang-matched reading
  decides, stays refused (5 sole rows).
* Not registered again: decision classes 1 and 3, rules 5k, 10v and 10w's
  refusals, the uncontracted-callee residual (W45-CALLEE-CASCADE-RESIDUAL),
  RK-OP-ERR-DESTROY-CONTRACT, MEMBER-EFFECT-STORES.

**Files likely to conflict.** `typed_field_access.rs` (rule 68's hooks; the
reading-site digest in `docs/reports/libc-reading-site-evidence/bindings.json`
re-pinned, recompute after a merge that also edits the file),
`typed_access_path.rs` (`opaque_call` -> `opaque_calls`),
`legacy_cast_semantics.rs`, `residual_access_reading.rs`,
`legacy_residual_access.rs`, `lib.rs` (one `mod` line), `docs/agents/rules/12h.md`
and `14a.md`, the generated `docs/agents/gateway.md`, `docs/agents/README.md`,
`proof/QCore.lean`, the r2 fixture file. None of `legacy_transfer.rs` or the
OpenSSL provider is touched.
