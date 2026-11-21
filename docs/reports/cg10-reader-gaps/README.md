# CG10-READER-GAPS: three reader gaps behind the indirect-access refusal (rules 71, 71a, 71b)

USER DECISION 2026-11-21: "launch the reader-gap card for the
indirect-memory-access refusals". The card bundles three `named_followups`
entries: CG10-EMBEDDED-CONTAINER-THREE-HOP-ACTUAL,
CG10-CAST-VIEW-RETURN-POSITION and CG10-RETAINED-BY-FUNCTION-POINTER-SPELLINGS.
Base `9b107caf1` (main). Gateway [rule 71](../../agents/gateway.md#constraint-71),
[71a](../../agents/gateway.md#constraint-71a),
[71b](../../agents/gateway.md#constraint-71b); rules 12h and 70 point at
them. Lean `QCore.LogicalAndOperand` (4 audited theorems),
`QCore.ReturnCastView` (6), `QCore.FunctionTypedefPointer` (7).
[PREDICTION.md](PREDICTION.md) was committed (`59abd0eb8`) before the
implementation (`0e03da4b3`, the measured binary).

## 1. Diagnosis (method step 1)

**(a) The three-hop address was not the gap.** Fixtures over the base read
`&rk->rk_conf.interceptors.on_send` (typedef'd embedded record, untagged
member, `rd_list_t`) with W44's typed actual reader exactly as
`&conf->interceptors.on_X`. A debug run of the base gateway over the
preprocessed `rdkafka_interceptor.c` with the parity stream on showed the
real cause in the never-bound join's row for `rk`: `nested-reading-binds`,
`lexical-binds-ast-never-bound` (closure reason `e:nested`). The rows open
with `RD_KAFKA_INTERCEPTOR_REJECT_IF_SHARE(rk)`, whose condition expands to
`(rk)->rk_type == RD_KAFKA_CONSUMER && (rk)->rk_conf.share.is_share_consumer`;
the tokens are single characters, so rule 10o's parenthesised-writes reader
read `& & ( rk )` as `& ( rk )`, the address of `rk`. The closure bound
`rk`, and an address actual needs every root never bound (rule 10n), so rule
69's reading answered `an actual neither reader recognises`.

**(b)** All five rows are whole-call returns `return f(…, (T *)p->m, …)`. Rule
12i declines a returned call that hides a place behind an identity-preserving
view (rule 15z's guard), and H-U had no return placement.

**(c)** Clang desugars only a type's top level: `T *f` with a function
typedef `T` is spelled `T *` (checked on Clang 21's JSON:
`cb_t *` has no `desugaredQualType`), so rule 70 read it as an object
pointer. `function_parameters` (the W4 lexical parameter reader) refuses
every `(` in a declarator list. `cio_set_log_callback`'s `void (*log_cb)` is
spelled `void (*)` and reaches the store through an implicit `BitCast` to
`int (*)(…)`.

## 2. Readings

* **Rule 71** (`logical_and_operand.rs`, one hook in
  `lexical_nested_assignment::parenthesised_writes`): two `&` tokens after
  an operand end are a logical AND, not an address. `X & &(p)` is a bitwise
  AND with a pointer operand (C11 6.5.10p2 forbids it), so a unit Clang
  accepted spelled `&&`; a run of three (`a && &(p)`) still ends in a unary
  `&`. Lean: `a_unary_address_is_read` (no real address missed, with the
  forbidden spelling as a hypothesis), `a_logical_and_is_one`,
  `only_a_logical_and_is_dropped`, `decision_vectors_hold`.
* **Rule 71a** (`return_cast_view.rs`; hooks in `legacy_return_call_access.rs`,
  `legacy_cast_semantics.rs::uncast_returned_views`,
  `typed_access_positions.rs::argument_node`): where rule 15z's guard
  declines a whole-call return, every view it names is removed and the uncast
  call is admitted by its own classification or by rule 12i unchanged; W44's
  typed actual reader matches an uncast actual to the cast's operand node
  while the reading is asked (thread-local guard). Read only when every
  removed view points at `void`, a character, a scalar or scalar typedef
  (`AggregateFacts::scalar_typedef`) or a tagged `struct`; a union or record
  typedef pointee (`(LZ4_stream_t *)`) keeps the refusal under rule 10w's
  design. This amends the route map's return step 2 (rule 12i's row; the
  frozen `gateway-analysis-path-optimization/ROUTE-MAP.md` is not edited):
  no row is added to the fallback tables and no older route's answer moves,
  which the identity run (section 3: every changed row is accounted for)
  bears out. Lean: `nothing_is_left_hidden`, `the_same_identities_are_handed`,
  `a_kept_view_keeps_the_refusal`, `only_a_whole_call_is_read`,
  `a_removed_view_is_its_place`, `return_view_vectors_hold`.
* **Rule 71b** (`function_typedef_pointer.rs`; a hook in
  `pointer_store::read_store`, `AggregateFacts::function_typedef`, the typed
  reader's `aggregates` field): a named formal spelled `NAME *`, every
  `TypedefDecl` of `NAME` directly a function type (by Clang's type-node
  kinds, through `ParenType`/`ElaboratedType` only), is rule 10v's
  function-pointer name; rule 70 adds no retention for it. Lean:
  `decision_vectors_hold`, `a_disagreeing_declaration_keeps_the_object_reading`,
  `a_pointer_array_typedef_is_an_object`, `a_pointer_typedef_is_not_a_function_typedef`,
  `an_alias_is_not_followed`, `read_only_for_a_function_typedef`,
  `a_function_pointer_adds_no_obligation` (over `QCore.MemberStoreRetention`).
* **Refusals recorded:** a formal declared `R (*f)(A)`: `function_parameters`
  is shared by the W4 derivation and validation, selection frames,
  recursion, neutral callees, self-link frames, member release and every
  body's formal set, so reading the declarator is its own census
  (CG10-W4-FUNCTION-POINTER-DECLARATOR-FORMALS). An object pointer stored into
  a function-pointer member: ISO C defines no such conversion (6.3.2.3;
  J.5.7 is an extension; rule 12h's `function-object`), so
  `cio_set_log_callback` stays.

**Fixtures** (`crates/qcore-c-gateway/tests/reader_gaps.rs`, 7 tests, new
contract fixture `fixtures/contracts/reader_gaps.qv`): rule 71 names the
retention for the `REJECT_IF_SHARE` spelling and admits a borrowed-only
helper, while `a && &(rk)` and `keep(&(rk))` keep the gate; rule 71a admits a
character view, a typedef'd base, strcmp's two views, a scalar-typedef view and
a tagged-struct view, and keeps a union view (alone and beside a removable
one), a return inside an expression, a releasing position and a released
tracked object behind a view non-`Safe`; rule 71b discharges a caller handing
its own `cb_t *` formal and keeps an object typedef's retention; the two
refusals are pinned. Unit tests: `rule_71_decision_vectors_hold`,
`the_parenthesised_reader_no_longer_reads_a_logical_and_as_an_address`,
`rule_71a_decision_vectors_hold`, `rule_71a_reads_each_pointee_by_its_type`,
`rule_71b_decision_vectors_hold`,
`only_a_direct_function_type_is_a_function_typedef`. **One pin flipped
deliberately**: `indirect_access_sole_residuals_r3::rule_12h_round_3_keeps_every_other_cast_refused`'s
return row (`return is_count((const char *)a->name);`) is now `Safe`, with a
scalar-expression control added.

## 3. Measurement

Opt-in `quirin-audit current --monkey-linked` over Fluent Bit 046accf6 as
round 3 took it, through [scripts/opt-in-current.sh](scripts/opt-in-current.sh):
round 3's script with one change, the two compile databases passed from a
copy whose paths are rewritten. The prepared inputs were moved from the
worktree that prepared them (`agent-a113e…/private/fb-046accf6`) to the main
checkout's `private/fb-046accf6`, and its databases still name the old paths
(the first base attempt, `private/rg/run-base-failed-stale-db`, exit 1:
`compile database contains no selected cmetrics sources`; not evidence). One
side tree (`git archive 9b107caf1`) for both runs, release `indirect-census`
builds, the binary the only difference. Logs: worktree `private/rg/`.

| run | revision | binary | Safe / Pending / Unknown / Unsafe | access rows | access alone | access leaves |
|---|---|---|---|---:|---:|---:|
| base | `9b107caf1` | `61d72850…` | 1,150 / 2 / 3,645 / 7 | 2,154 | 222 | 9,066 |
| **tip** | `0e03da4b3` | `185c65db…` | **1,152 / 2 / 3,643 / 7** | **2,141** | **213** | **9,047** |

Both exit 0 ([evidence/run-base.txt](evidence/run-base.txt),
[evidence/run-tip.txt](evidence/run-tip.txt)); the base equals rule 70's tip
(1,150 / 2 / 3,645 / 7). Comparison:
[evidence/compare-base-tip.json](evidence/compare-base-tip.json) (round 3's
`compare.py`), [evidence/changed-rows.txt](evidence/changed-rows.txt) (rule
70's `changed_rows.py`), [evidence/named-rows.jsonl](evidence/named-rows.jsonl)
([scripts/named_rows.py](scripts/named_rows.py)).

* **Two moves, both Unknown -> Safe** (rule 71):
  `rdkafka_assignment.c::rd_kafka_assignment_pause` and `_resume`, whose
  `RD_KAFKA_IS_SHARE_CONSUMER(rk)` guard had bound `rk` (4 leaves each
  removed: the condition, the access, two `unresolved resource identity rk`).
  Both rest on declared assumptions (527 -> 529; no declaration introduced).
* **No Safe -> non-Safe move, no new Unsafe, Pending unchanged, no leak
  added** (leak diagnostics 10 and 10).
* 16 rows change leaves: 19 access leaves removed, none added. The 10
  `rd_kafka_interceptor_add_on_*` rows are re-attributed to rule 12t's
  `retention-undecided` (through rule 69); `reset_historical_metrics` loses
  4 access leaves and an `rk` identity leaf (rule 71, still Unknown on its
  other leaves); `rd_kafka_group_MemberMetadata_consumer_read` and
  `rd_kafka_mock_cgrp_classic_member_assignment_set` move from the access
  text to rule 38i's `member-store-through-formal` (rule 71: the formal is
  now read as never bound and the store reaches rule 38i);
  `sort_by_map_elem_val_toppar_list_cnt` loses its return leaf (rule 71a).

### Per-row evidence (the 16 named rows)

| row | entry | base -> tip | remaining blocker |
|---|---|---|---|
| `rd_kafka_interceptor_add_on_{destroy, send, acknowledgement, consume, commit, request_sent, response_received, thread_start, thread_exit, broker_state_change}` (10) | THREE-HOP | Unknown -> Unknown, **re-attributed** (access -> `retention-undecided`) | rule 12t: the caller's object retained in the caller's container (RETENTION-UNDECIDED-PENDING) |
| `print_string` | CAST-VIEW | Unknown, unchanged | the uncast callee `print_string_ptr` is unsummarised (Unknown: `ensure`), so the uncast call is uncontracted; rule 12k does not re-attribute an uncast spelling (CG10-UNCAST-RETURN-UNCONTRACTED-ATTRIBUTION) |
| `pb_dec_bool` | CAST-VIEW | Unknown, unchanged | the same: `pb_decode_bool` unsummarised (`pb_decode_varint32`) |
| `pb_enc_fixed_length_bytes` | CAST-VIEW | Unknown, unchanged | the same: `pb_encode_string` unsummarised (`pb_write`'s registry) |
| `LZ4F_localSaveDict` | CAST-VIEW | Unknown, unchanged | a view to the union typedef `LZ4_stream_t *` is kept (rule 10w); `LZ4_saveDict`/`LZ4_saveDictHC` are unsummarised in `lz4.c`/`lz4hc.c` |
| `sort_by_map_elem_val_toppar_list_cnt` | CAST-VIEW | Unknown -> Unknown, return leaf removed | `const rd_kafka_topic_partition_list_t *al = a->value, *bl = b->value;` (two pointer declarators from member reads) |
| `cio_set_log_callback` | FN-POINTER | Unknown, unchanged | recorded refusal: an object pointer converted to a function pointer |
| `rd_kafka_buf_set_maker` (`make_cb`) | FN-POINTER | Safe, unchanged | its callers stay uncontracted: the `R (*f)(A)` formal `free_make_opaque_cb` (recorded refusal, CG10-W4-FUNCTION-POINTER-DECLARATOR-FORMALS); rule 71b changes its summary's retentions when one is published |

**Against the prediction.** Rule 71: the ten re-attributions as predicted;
2 Unknown -> Safe (predicted 0-15) and 6 more rows change leaves (predicted
20-300: `&& (p)` is common, but rarely the only seed of a name). Rule 71a:
0 of the predicted 0-3 moves, the three callees being unsummarised; LZ4 and
`sort_by_…` as predicted; the predicted `print_string` and `pb_dec_bool`
corpus moves are **withdrawn**. Rule 71b: no move, as predicted. No Safe ->
non-Safe and no new Unsafe, as predicted.

**Pinned routes.** `quirin-audit measure --declared` cannot run on this host
(no `build/cycle19` inputs or baselines). Prediction for the integrator,
replacing PREDICTION.md's block:

```measure-prediction
route corpus
move rd_kafka_assignment_pause Unknown -> Safe
move rd_kafka_assignment_resume Unknown -> Safe
route system
none
route slice
none
```

each where the route analyses `rdkafka_assignment.c` (corpus and
twelve-home); leaf re-attributions on librdkafka rows using
`RD_KAFKA_IS_SHARE_CONSUMER` and similar `&& (p)` macros; no Safe ->
non-Safe move or new Unsafe on any route.

## 4. Gate

Scoped card gate `sh private/toolchain/env.sh python3 scripts/gate_scope.py
--base 9b107caf1 --out private/gate-1` on `6d60f52cd` (later commits add
this report, its evidence and the read-census entry only); mode full (the
diff reaches `qcore-c-gateway` and `proof/QCore.lean`). **Exit 0,
`gate_scope: 8 steps passed`** (record [gate/gate-1.json](gate/gate-1.json),
logs in the worktree's `private/gate-1/`):

| step | exit | result |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | (1,861 s, mostly waiting for a shared build slot) |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `run_test_binaries.py` | 0 | 671 units, 716 summaries; 9,827 passed, 0 failed, 21 ignored, 148 filtered |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `check_lean_audits.py` | 0 | 472 audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | 779 tests |

No flaky rerun was needed and no earlier gate attempt exists. Before the
gate: one clippy run (`private/rg/clippy1.log`, exit 101: an enum variant
postfix and a `?` operator, fixed in `6d60f52cd`, no semantic change) and
`cargo fmt` applied. **Gate read census:** `gate_read_census.py record
--packages qcore-c-gateway` and `reduce` on `d333e464d` (logs
`private/rg/read-census/`, both exit 0; 262 units traced, 1 not ok under the
tracer, `detach_lookup_membership`, as rule 69's run found; it passes in the
gate); the one new unit `qcore-c-gateway::test::reader_gaps` (reads no
repository file: its contracts are `include_str!`) was added to
`docs/reports/gate-scope-precise-selection/read-census.json`, no existing
unit rewritten. `apply_amendments.py
docs/reports/cg10-reader-gaps/integration-amendments.json --check
--profiles-base 9b107caf1` exits 0.

## 5. Follow-ups ([integration-amendments.json](integration-amendments.json))

* **Retired:** CG10-EMBEDDED-CONTAINER-THREE-HOP-ACTUAL (rule 71; the 10 rows
  re-attributed), CG10-CAST-VIEW-RETURN-POSITION (rule 71a; each row moved
  or recorded with its blocker above), CG10-RETAINED-BY-FUNCTION-POINTER-SPELLINGS
  (rule 71b reads `T *`; the other two spellings are recorded refusals).
* **Amended:** CG10-INDIRECT-ACCESS-SOLE-RESIDUALS (access-alone 222 -> 213
  on the opt-in route).
* **Registered:** CG10-W4-FUNCTION-POINTER-DECLARATOR-FORMALS (the shared
  parameter reader and `R (*f)(A)`), CG10-UNCAST-RETURN-UNCONTRACTED-ATTRIBUTION
  (rule 12k does not re-attribute a returned call whose uncast spelling meets
  an uncontracted callee: 3 rows), CG10-UNPARENTHESISED-LOGICAL-AND-SEED (the
  closure's own seed `seed_addresses_and_writes` still reads `x && p`, with
  no parentheses and nothing after `p`, as the address of `p`: conservative,
  unmeasured), CG10-OPT-IN-PREPARED-INPUT-PATHS (the moved prepared inputs'
  compile databases name a removed worktree).

## 6. Files likely to conflict

`crates/qcore-c-gateway/src/lib.rs` (three `mod` lines),
`lexical_nested_assignment.rs` (one condition),
`legacy_return_call_access.rs` (rule 12i's guard; a new `Form` variant and
`lowered_uncast_call`), `legacy_cast_semantics.rs` (one method),
`typed_access_positions.rs::argument_node` (one call: round 4's W44 actual
readers may edit nearby), `typed_access_path.rs` (one `Reader` field and its
accessor), `pointer_store.rs::read_store` (one branch), `aggregate_value.rs`
(one field, two methods), `tests/indirect_access_sole_residuals_r3.rs` (one
pin), `docs/agents/rules/12h.md` and `70.md` (one sentence each), and the
generated `docs/agents/gateway.md`, `docs/agents/README.md`,
`proof/QCore.lean`. No provider, contract or lock is touched.
