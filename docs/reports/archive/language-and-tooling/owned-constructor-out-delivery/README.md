# OWNED-CONSTRUCTOR-OUT-PARAMETER-DELIVERY: an owned result stored through a formal's slot

Card OWNED-CONSTRUCTOR-OUT-PARAMETER-DELIVERY. USER DECISION 2026-11-23: picked
as one of the "soundness first" set of four implementation cards. Base
`2c60986b0` (main). [PREDICTION.md](PREDICTION.md) was committed (`58a0f324a`)
before the pins (`4e2de537a`) and the implementation (`fcf76ce01`). Gateway
[rule 74l](../../../../agents/gateway.md#constraint-74l), Lean
`QCore.OwnedOutFormalDelivery` (16 audited theorems).

**Outcome.** No new effect mode and no Safe move. The out-parameter delivery
summary the discharge asks for already exists (W4's `Out` mode), and none of
the four rows is a delivery it can soundly state. The misleading `discarded
owned result` is replaced by a named refusal, the discharge's alternative.

## 1. Census

**The four rows.** Bodies read at Fluent Bit `046accf6`.

| row | slot | store on | also | callers |
|---|---|---|---|---|
| `rdkafka_subscription.c::rd_kafka_assignment` (:337) | `*partitions`, formal | some paths: `*partitions = rko->rko_u.assign.partitions` on every path past the op, then the fresh list only when that is NULL and `err` is 0; the two early returns leave it unwritten | re-stores the slot (another object's member first); returns `err`, releases nothing in the slot | public API: the application owns the list |
| `rdkafka_subscription.c::rd_kafka_subscription` (:365) | `*topics`, formal | the same shape | the same | public API; `rd_kafka_share_subscription` forwards its own formal |
| `rdkafka_request.c::rd_kafka_handle_OffsetFetch` (:1223) | `*offsets`, formal | some paths: `if (!*offsets) *offsets = …_list_new(16);` (in-out: keeps the caller's list when it has one); the `goto err` paths before it leave it unwritten | then reads and fills `*offsets` | `rd_kafka_op_handle_OffsetFetch` passes a copy it owns; `rdkafka_assignment.c`, `rdkafka_admin.c` pass theirs |
| `rdkafka_sticky_assignor.c::ut_create_topic_partition_lists` (:2235) | `lists[i]`: an element of the frame's `rd_calloc` array, **not a formal's slot** | every iteration | the array is returned | unit tests own the result |

Every one has other leaves on the base run (`indirect memory access` and
unresolved identities for the first two, 26 access leaves and nine
uncontracted callees for OffsetFetch, `rd_kafka_topic_partition_list_add`
for the unit-test helper), so none is blocked by the out slot alone.

**Every other `*formal = owned_ctor(…)` on the opt-in route.** The base
report's `discarded owned result` leaves at a line spelling `*NAME = …`
([evidence/census-base.txt](evidence/census-base.txt), `scripts/census.py`):
21 leaves in 15 rows (the 3 slot rows above and 12 more). The other 9 leaves
in 5 rows are element stores (`lists[i]`, `ALL_RACKS[i]`, `metrics[idx]`,
`tqh[qi]`).

| row | slot | store on | why no `Out` summary |
|---|---|---|---|
| `cio_file_unix.c::cio_file_native_lookup_user`, `_group` | `*result`, formal | some paths; written twice when the name is NULL | `calloc` is nullable (tested through `*result`), error returns leave it unwritten, a second store over the first |
| `cmt_mpack_utils.c::cmt_mpack_consume_string_tag`, `ctr_mpack_utils.c::ctr_mpack_consume_string_tag`, `_binary_tag` | `*output_buffer`, formal | the `0`-return class only | declared C04.13 dispositions (rule 72's callees); the bodies do not validate (uncontracted `mpack_*`; CG10-OOM-DECLARATIONS-UNVALIDATED) |
| `rdkafka_msgset_writer.c::rd_kafka_gzip_compress` | `*outbuf`, formal | the `0`-return class only | the same, uncontracted zlib calls |
| `rdkafka_broker.c::rd_kafka_broker_share_session_add_remove_toppar` | `*toppars_add_list`, formal | `if (!*slot)` only | in-out, then `rd_list_add` into it |
| `rdkafka_cgrp.c::rd_kafka_cgrp_consumer_assignment_with_metadata` | `*missing_topic_ids`, formal | `if (!*slot)` inside a loop, under `if (missing_topic_ids)` | in-out, nullable slot pointer; REGISTRY-UNDECIDED-CALL-POISONS-EVERY-BINDING |
| `rdkafka_conf.c::rd_kafka_anyconf_set_prop0` (4 leaves) | `*str`, `*kstr`: **locals** pointing into the conf object (`_RK_PTR`) | per `switch` case | the old value is released and replaced in place; no formal names the slot |
| `rdkafka_request.c::rd_kafka_handle_ApiVersion` | `*apis`, formal | after a `goto err` | partial path |
| `rdkafka_telemetry_encode.c::resource_attributes` | `*attributes`, formal | **every exit** (NULL or `rd_malloc`) | the sound shape (`Out { nullable }`); blocked by uncontracted `count_attributes`, `set_attributes` and three unresolved identities |
| `rdkafka_telemetry_encode.c::serialize_Metric` | `*metric_name`, formal | its one path | 25 access leaves before the store |
| `mk_string.c::mk_string_build` | `*buffer`, formal | every exit, then `mk_mem_realloc` of it | re-store through an uncontracted realloc, possibly-null result |

## 2. Soundness

**What W4 already states** (`legacy_summary.rs`). `out_parameter_candidates`
marks every formal the body stores through; the probe sanctions `*NAME`;
`out_mode` reads `Out { nullable }` only when every reachable exit holds a
fresh, live, non-null heap site in the slot or the null constant, with at
least one site; condition (6) of the W4 gate (`out_slots_match`) validates it;
the summary is published `InferredValidated`, and a caller's `fill(&p)` binds
`p` to a fresh obligation. This is exactly the discharge's "out-parameter
summary": ownership leaves on every path that stores, no path leaks (the body
must be `Safe`), and callers take custody. Lean already models it
(`QCore.CustodySummary.outSlotMatches`, `validated_summary_out_slots_match`).

**Why the four rows cannot use it.** A slot written on some exits only, an
in-out slot that keeps the caller's object, and a slot first holding another
object's member all leave an exit whose slot is `Unwritten` or `Ambiguous`,
which `out_mode` refuses (`a_partial_path_store_reads_no_mode`,
`an_ambiguous_slot_reads_no_mode`). Stating them needs either a relational
disposition on a return code the bodies do not keep (`rd_kafka_assignment`
writes the slot even when `err` is nonzero) or a new keep-or-deliver mode for
an in-out slot. Neither has a precedent in the effect vocabulary, so per the
brief it is not implemented here; it is registered for a user decision
(OUT-SLOT-CONDITIONAL-DELIVERY, section 5). The rows have other blockers in
any case (section 1).

**The refusal.** With no summary, the row's own analysis runs with no out
slot, and `legacy_transfer.rs` refused the call as `discarded owned result
of resource API \`F\``. The result was not discarded. It is now `owned result
of resource API \`F\` stored through \`*NAME\` without an out-parameter
contract`, worded as its sibling for a variable's store (`store through
\`*NAME\` without an out-parameter contract`). Kind, state poisoning and the
summary gate are unchanged, so no `Unknown` can move
(`the_renaming_keeps_every_leaf`, `only_the_unsanctioned_slot_changed`). A
real discard (`make();`) and an element store keep their text.

## 3. Changes

* `crates/qcore-c-gateway/src/owned_out_formal.rs` (new): the refusal text
  and `OUT_MODE_VECTORS`. `legacy_transfer.rs`: the unsanctioned-destination
  arm of the contract call uses it. `legacy_summary.rs`: `out_mode` is
  `pub(crate)` so the vectors run against it. `lib.rs`: one `mod`.
* Rule [74l](../../../../agents/rules/74l.md); generated `docs/agents/gateway.md`,
  `docs/agents/README.md`.
* `proof/QCore/OwnedOutFormalDelivery.lean`, `proof/AuditOwnedOutFormalDelivery.lean`;
  generated `proof/QCore.lean`. The module imports `QCore.CustodySummary` and
  reuses its `OutSlot`, `validate` and `outSlotMatches`.

## 4. Pins

Unit (`tests/unit/owned_out_formal.rs`, 6): the out-mode vectors against
`out_mode` (every-exit delivery, partial path, null only, store then release,
escaped, contained, possibly-null, ambiguous, no exit); a non-heap delivery;
the summaries of the census shapes (only the every-exit body publishes `Out`;
partial, in-out, store-then-release, second store and possibly-null publish
nothing); the refusal text in the reduced importer; a real discard.

End to end (`tests/owned_out_formal_delivery.rs`, 9, through Clang): the
every-exit delivery is `Safe`, its caller that releases is `Safe`, **a caller
that ignores the delivered object is `Unsafe` (leak in the caller)**; the
`rd_kafka_assignment`/`rd_kafka_subscription` shape and the OffsetFetch in-out
shape are `Unknown` with the new text; the element store keeps `discarded`;
**a partial-path store stays `Unknown`** and its caller keeps `call to
uncontracted function`; **a store followed by a release** (and a second
store) stays `Unknown`, its caller uncontracted; a possibly-null result and a
local pointer's slot stay `Unknown`; a real discard keeps its text.

**Failing at the base.** With the module and pins in place and
`legacy_transfer.rs` unchanged (`4e2de537a`): end to end 6 failed, 3 passed
(the six text pins: assignment/subscription, OffsetFetch, partial path,
store-then-release, possibly-null, local slot; each fails on its
`slot_refusal` assertion, the base reporting `discarded owned result`); unit
1 failed (`an_unsanctioned_slot_store_is_refused_by_name_not_as_a_discard`),
5 passed. Logs: `private/ocod/pins-at-base-{e2e,unit}.log`. At `fcf76ce01`:
9/9 and 6/6 (`private/ocod/pins-tip-*.log`). The other pins hold at both, as
pins of kept verdicts should.

## 5. Measurement

[evidence/runs.md](evidence/runs.md): opt-in `current --monkey-linked` over
Fluent Bit `046accf6`, base and tip each from its own `git archive` tree and
release binary.

| run | Safe / Pending / Unknown / Unsafe | access rows / alone / leaves |
|---|---|---|
| base `2c60986b0` | 1,172 / 2 / 3,622 / 8 | 2,118 / 215 / 8,992 |
| tip `fcf76ce01` | **1,172 / 2 / 3,622 / 8** | 2,118 / 215 / 8,992 |

**No status move** (`moved-rows-base-tip.jsonl`: `{"moves": {}}`). **15 rows
change leaf text, 21 leaves**, all `Unknown -> Unknown`
([evidence/changed-rows-base-tip.txt](evidence/changed-rows-base-tip.txt)):
exactly the 21 `*NAME` leaves of the census, each `discarded owned result`
becoming the slot refusal; no other leaf added or removed. The 9 element-store
leaves are unchanged.

**The four rows:** `rd_kafka_assignment` (`*partitions`),
`rd_kafka_subscription` (`*topics`) and `rd_kafka_handle_OffsetFetch`
(`*offsets`) re-worded, `Unknown`; `ut_create_topic_partition_lists`
unchanged, `Unknown`.

**Against the prediction.** Predicted: no status move, the three slot rows
re-worded, the element row unchanged, "a few to a few tens" of other rows
re-worded. Measured: exactly that, 12 other rows. The `measure-prediction`
block (no pinned-route move) stands.

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this
host (no `build/cycle19` inputs or baselines); not run, no result claimed.
Expected there: no status move; leaf text only, where a pinned report
carries `discarded owned result` at a `*NAME` store (the committed
`contract/contributions/{35,37}/eight-home.json` quote `rdkafka_conf.c:2070`
and similar origins, which a remeasurement will word anew).

## 6. Gate

Scoped card gate `python3 scripts/gate_scope.py --base 2c60986b0 --out
private/ocod/gate-1` through `private/toolchain/env.sh` (Clang 21, Lean
4.35), head `cad60e651` (code, tests, Lean and rule final; later commits
change this report only). Mode full (the diff reaches `qcore-c-gateway`);
provider gate not selected. Record [gate/gate-1.json](gate/gate-1.json), logs
in the worktree's `private/ocod/gate-1/`.

| step | exit | result |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `run_test_binaries.py` | 0 | 681 units, 726 summaries; 9,986 passed, 0 failed, 22 ignored, 148 filtered |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `check_lean_audits.py` | 0 | 492 audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | 807 tests |

The gate read census (`docs/reports/gate-scope-precise-selection/read-census.json`)
was not re-recorded for the new test target; its coverage test tolerates new
units, and the target reads only compile-time fixtures.

## 7. Follow-ups (in [integration-amendments.json](integration-amendments.json))

* **OWNED-CONSTRUCTOR-OUT-PARAMETER-DELIVERY** retired: the delivery summary
  is W4's `Out` mode, which the four rows cannot soundly use; the refusal is
  named (rule 74l).
* **OUT-SLOT-CONDITIONAL-DELIVERY** (new, needs a user decision): a slot
  written on some exits only. Three sub-shapes: keep-or-deliver in-out
  (`if (!*out) *out = make();`: OffsetFetch, `add_remove_toppar`,
  `assignment_with_metadata`), a slot first holding another object's member
  then a conditional fresh object (`rd_kafka_assignment`,
  `rd_kafka_subscription`), and error-path partial stores whose return code
  does not partition the slot (`cio_file_native_lookup_*`,
  `rd_kafka_handle_ApiVersion`). Each needs a new effect mode or an inferred
  return-code disposition, which has no precedent in the vocabulary.
* **OWNED-RESULT-ELEMENT-STORE-READ-AS-DISCARD** (new): an owned result
  stored into an element (`lists[i] = make();`) is still refused as
  `discarded owned result` (9 leaves in 5 rows, among them
  `ut_create_topic_partition_lists`).
* Not registered again: RK-TOPIC-PARTITION-CONSTRUCTORS-UNVALIDATED,
  CG10-OOM-DECLARATIONS-UNVALIDATED, REGISTRY-UNDECIDED-CALL-POISONS-EVERY-BINDING
  (the other blockers named in section 1). **Interaction:**
  LATER-DECLARATOR-INITIALISER-UNREAD's later declarator read as a `*b`
  store gets this refusal's text when its initialiser is an owned call,
  until that card lands; none appeared on this run.

**Files likely to conflict.** `crates/qcore-c-gateway/src/legacy_transfer.rs`
(one arm), `legacy_summary.rs` (`out_mode` visibility), `lib.rs` (one
`mod`); the generated `docs/agents/gateway.md`, `docs/agents/README.md`,
`proof/QCore.lean` (rerun the generators). The other running cards touch
`legacy_transfer.rs` or the declarator readers (LATER-DECLARATOR-INITIALISER-UNREAD).
