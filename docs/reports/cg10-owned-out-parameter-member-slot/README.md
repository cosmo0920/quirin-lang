# CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT: an owned value delivered into a member slot of a caller-supplied object

Card CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT. USER DECISION 2026-11-21 (through
the integrator's brief): launch the card. Base `8e7e8d509` (integrate/r48).
[PREDICTION.md](PREDICTION.md) was committed (`7de74455f`) before the first
implementation commit (`bedc1a5c1`). Gateway [rule 72](../../agents/gateway.md#constraint-72),
contract specification §C04.13.1, five pinned declarations, Lean
`QCore.OwnedOutMemberSlot` (13 audited theorems).

## 1. The 13 rows and their callees (step a)

Every caller and callee body was read at Fluent Bit `046accf6`
([PREDICTION.md](PREDICTION.md) section 1 has the table). The rows are round
4's (`../cg10-indirect-access-sole-residuals-r4/census/rank-tip.json`); on
this card's base run each is `Unknown` with the access refusal as its only
leaf (section 4). One shape: a callee stores a fresh allocation through an out
slot (`*output_buffer = cfl_sds_create_size(len)`, `*outbuf = rd_malloc(…)`)
whose actual is the address of a member of an object the caller reaches from
one of its own formals:

* cmetrics `unpack_opts_{ns,ss,name,desc}` (`&opts->F`, `opts = map->opts`,
  `map = (struct cmt_map *) context`) and `unpack_static_label_component`
  (`&label->key`, `&label->val`, `label = (struct cmt_label *) context`), all
  through `cmt_mpack_consume_string_tag`;
* ctraces `unpack_instrumentation_scope_{name,version}`,
  `unpack_link_trace_state`, `unpack_span_schema_url`,
  `unpack_span_status_message` (`&context->scope_span->instrumentation_scope->name`,
  `&context->span->status.message`; `struct … *context = ctx`), through
  `ctr_mpack_consume_string_or_nil_tag`;
* librdkafka `rd_kafka_msgset_writer_compress_{gzip,lz4,snappy}`
  (`&ciov->iov_base`, `ciov` a formal), through `rd_kafka_gzip_compress`,
  `rd_kafka_lz4_compress`, `rd_kafka_snappy_compress_slice`.

Each callee returns `0` with a fresh, owned value in the slot (ctraces: or
NULL for a nil tag) and nonzero with the slot unwritten or written NULL. None
of the 13 holds a frame-owned record: every slot is a member of the caller's
caller's object.

## 2. The contract form (step b)

**No new grammar.** The callees are C04.13 return-code dispositions with an
out slot: `resource_on_return(eq, 0, resource_out(slot, nonnull|nullable))`,
`resource_on_return(ne, 0, resource_borrowed(slot))` (the class leaves the
slot unwritten), every other formal borrowed, `resource_return_none()`. What
was missing was the consumer's reading of a member-path actual, now §C04.13.1:
where the slot's object is the caller's, every class delivers fresh custody or
nothing through it, so the class needs no decision; the delivered custody is
that object's.

**On the brief's step (c) wording.** The brief asks that "after the call the
member holds an owned value tracked like the field-link model's assignment
case: its later release ends the record, and an unreleased one is a leak
obligation". That is the reading for a member of a record this frame owns (a
tracked owner). None of the 13 has one: in each the member lies in an object
the caller supplied, where the field-link model's own semantics is the
owner-escape case (`cascade_field_links_on_escape`: a linked child escapes
with its escaping owner). Rule 72 implements that case: the delivered custody
is the caller's object's, owed by no exit of this frame, and nothing the frame
owed is dropped (`QCore.OwnedOutMemberSlot.an_admitted_delivery_keeps_the_frames_obligations`,
`the_delivered_identity_is_held`). The tracked-owner case keeps the gate's
refusal: all five declared callees are dispositions, whose unwritten class
cannot be read as a field store without a decided class, and no plain-`Out`
heap declaration exists in production, so implementing it would have no
consumer and no measurement. It is registered
(CG10-OWNED-OUT-MEMBER-TRACKED-OWNER).

## 3. The reading (rule 72, step c)

`owned_out_member.rs` (pure: the minted contract, the slot reader, the
caller-supplied roots, the decision), `legacy_owned_out_member.rs` (the
parser's lowering), and the flow in rule 10v's `legacy_pointer_store.rs`
(value class `owned-out`). In order:

1. **The minted contract.** A callee with exactly one out slot (plain `Out`,
   or a disposition `Out`/unwritten there and equal elsewhere) whose contract
   with that slot `Borrowed` is the neutral element is installed under
   `qcore_owned_out_member__F` (a reserved prefix).
2. **The statement.** Only for exactly the gate's refusal of `CALL`,
   `NAME = CALL` or `return CALL`, asked after every other route and before
   rule 12k: the slot actual is `& R -> F (-> G)* (. H)*`, `R` a
   caller-supplied root (an unrebound formal, or a name every write of which
   is a root, an object-pointer cast of one or a member read through one:
   premise P-72-CALLER-OBJECT). The statement is classified under the minted
   name with the slot replaced by the null constant; the other actuals are
   read by rule W44 (the lz4 caller's `?:` included).
3. **The lowering.** The slot's chain read, the call's block, and a rule-10v
   `PointerStore` of class `owned-out` (access: the call; fallback: the gate's
   refusal).
4. **The flow.** The owner is rule 10v's, read under rule 14n's named premise
   where the chain read is (`opts = map->opts` binds `opts` target-free and
   possibly NULL): untracked admits; tracked or unresolved refuses; then 10v's
   old-value premise (P-CG11-CALLER-SLOT).
5. **The summary.** A body with such a store publishes no summary (10v's
   `has_pointer_store`; rule 70's retention summary refuses it), so no caller
   reads `Borrowed` for a formal whose member now owns custody.

**Fixtures** (`crates/qcore-c-gateway/tests/owned_out_member_slot.rs`, a new
target, recorded in the gate read census): the eight census forms are `Safe`
with the production modules and `Unknown` without them; refused (never
`Safe`): a record this body allocated, a pointer to this frame's own object, a
rebound formal, an indexed slot, a call inside a larger expression, `&v.key`
of a local record; a buffer dropped beside the delivery is `Unsafe`; a caller
of a body with a delivery sees `call to uncontracted function`; a local `&s`
slot keeps C04.13's reading (`Safe` when released on the `0` edge, `Unsafe`
when not, refused untested); the refusal text is the gate's. Unit tests in
`tests/unit/owned_out_member.rs` (5: the decision vectors, the slot vectors,
the roots, their refusals, the minted contract).

## 4. Measurement

Route and inputs: [evidence/runs.md](evidence/runs.md) (the opt-in `current
--monkey-linked` over Fluent Bit 046accf6, as rounds 3 and 4 took it; the
prepared inputs at `private/fb-046accf6` were used directly, no path failure).

| run | tree | Safe / Pending / Unknown / Unsafe | access rows | access alone | access leaves |
|---|---|---|---:|---:|---:|
| base | `8e7e8d509` | 1,159 / 2 / 3,636 / 7 | 2,135 | 209 | 9,027 |
| tip 1 (not evidence) | `bedc1a5c1` | 1,159 / 2 / 3,636 / 7 | 2,135 | 209 | 9,027 |
| **tip 2** | `af8c51fc0` | **1,169 / 2 / 3,626 / 7** | **2,125** | **200** | **9,011** |

**Tip 1 is not evidence.** The opt-in run selects its modules through
`cycle19-resource-contracts.json` (`opt_in_providers.py`), which tip 1 did not
list: no new declaration was consulted (641 on both runs) and the report
equalled the base row for row. `af8c51fc0` adds the three modules to the
manifest (also the pinned routes' selection); tip 2 is the same binary
(`6070c09a…`, the code of `bedc1a5c1`, unchanged since: later commits change
tests, a profile digest and this report) on that tree.

**Ten moves, every one Unknown -> Safe** ([evidence/moved-rows.jsonl](evidence/moved-rows.jsonl)),
each from exactly one base leaf, the access refusal at the slot's call:

| row | callee (declared assumption) | note |
|---|---|---|
| `cmt_decode_msgpack.c::unpack_opts_ns`, `_ss`, `_name`, `_desc` | `cmt_mpack_consume_string_tag` | `opts = map->opts` binds `opts` target-free and possibly NULL: the owner is read under rule 14n's named premise (null premise carried: 38 -> 42 Safe rows) |
| `cmt_decode_msgpack.c::unpack_static_label_component` | `cmt_mpack_consume_string_tag` | two slots, one per arm |
| `ctr_decode_msgpack.c::unpack_instrumentation_scope_name`, `_version` | `ctr_mpack_consume_string_or_nil_tag` | a two-link chain |
| `ctr_decode_msgpack.c::unpack_link_trace_state`, `unpack_span_schema_url` | `ctr_mpack_consume_string_or_nil_tag` | |
| `ctr_decode_msgpack.c::unpack_span_status_message` | `ctr_mpack_consume_string_or_nil_tag` | a `.` path in the final object |

**No Safe -> non-Safe move, no new Unsafe, no Pending moved** (Unsafe 7 and
Pending 2, the same rows). Safe rows on a declared assumption 536 -> 546;
declarations consulted 641 -> 646; `refuted contracts: 0` on both.

**The three librdkafka rows did not move** (against the prediction). Their
statement also reads `msetw->msetw_rkb` at a borrowed position, and the writer
is `typedef struct rd_kafka_msgset_writer_s { … } rd_kafka_msgset_writer_t;`
(`rdkafka_msgset_writer.c:52-94`): rule W44 reads no member through a base
spelled with a defining typedef, so the call's block is refused before rule
72's store is reached and the row keeps the gate's text. Rule 72 reads their
slot (`&ciov->iov_base`, `ciov` a formal): the fixture
`a_writer_base_spelled_through_a_defining_typedef_keeps_w44s_refusal` shows
the same call `Safe` once the base is read elsewhere. Registered
(CG10-W44-TYPEDEF-DEFINING-BASE-ACTUAL).

**Thirteen rows change and stay Unknown** ([evidence/changed-rows-base-tip2.txt](evidence/changed-rows-base-tip2.txt)),
re-attributions only, no leak or violation added:

* `unpack_opts_unit` (`result = cmt_mpack_consume_string_tag(reader, &value)`,
  then `map->unit = value`): the `call to uncontracted function` and three
  `unresolved resource identity value` leaves go (C04.13 reads the tested
  `&value`); its one leaf left is the access refusal at `map->unit = value`,
  an owned value stored into a formal's member (MEMBER-EFFECT-STORES), so it
  joins the access-alone population (209 - 10 + 1 = 200).
* `cmt_mpack_unpack_map`: `call to uncontracted function` and two
  `unresolved resource identity result` go; `loop re-acquisition: an earlier
  generation that may be live is still held` comes at the loop's `&key_name`
  call (rule W43's loop-generation refusal: an `Unknown` leaf, not a leak
  claim; the body destroys the key after the callback walk).
* `unpack_event_name`, `unpack_span_name`, `unpack_span_trace_state`,
  `unpack_scope_span_schema_url`, `unpack_resource_span_schema_url`: the
  access leaf at the call goes (rule 72 admits it); each keeps the access
  refusal at `cfl_sds_destroy(context->span->name)` before it (a member
  release through a formal, rule 5k).
* `unpack_link_span_id`, `unpack_link_trace_id`, `unpack_span_span_id`,
  `unpack_span_trace_id`, `unpack_span_parent_span_id` and
  `rdkafka_telemetry.c::rd_kafka_push_telemetry_payload_compress`: `call to
  uncontracted function` becomes `relational-untested` (a local `&NAME` slot
  whose return value is not tested where it stands, C04.13 unchanged).

Access rows 2,135 -> 2,125, leaves 9,027 -> 9,011 (16 removed, none added).

**Against the prediction.** Predicted 9-13 moves, measured 10 (the ten
cmetrics/ctraces rows; the three librdkafka rows stay, for the reason above,
which the prediction did not name); other callers predicted 0-8 changed,
measured 13 changed, none moved, all re-attributions; access-alone predicted
205-209, measured 200 (the prediction counted from round 4's 218; this base
already stood at 209). No Safe -> non-Safe, no new Unsafe, as predicted. The
prediction's note that the corpus lz4 call had a literal `1` came from a
census text at `66910c10`: `rdkafka_msgset_writer.c` is byte-identical at both
revisions, the call has the `?:` at both.

**Pinned routes (corpus, system, slice).** `quirin-audit measure --declared`
cannot run on this host (no `build/cycle19` inputs or baselines). The
committed measure-prediction block names the three librdkafka rows; given the
W44 refusal measured here (the unit is byte-identical at `66910c10`), the
integrator should expect **no** corpus move from them, and none from the
cmetrics and ctraces decoders, which the corpus route does not analyse.
System and slice: none.

## 5. Declarations (step d)

`cmetrics/mpack_string_out.qv`, `ctraces/mpack_string_out.qv`,
`librdkafka/compress_out.qv` (their own files; the RK-TOPIC-PARTITION-LIST-
CONSTRUCTORS card's constructors are not declared), each body quoted with its
ABI, pinned with `quirin-audit pin` at `66910c10` (a shared clone of the local
Fluent Bit repository) and `046accf6` ([scripts/pin.sh](scripts/pin.sh): `pin`
0 and 0 for each crate; `pin --check --crate` 0 at both for cmetrics and
ctraces, 0 at `66910c10` and 1 at `046accf6` for librdkafka only for
`rd_kafka_log0`/`rd_kafka_log_buf`, in force through their `66910c10`
identity, the pre-existing state round 4 recorded; `pin --verify-contracts` 0
after the third crate). Every body is byte-identical at both revisions
([scripts/cmp.sh](scripts/cmp.sh)). `providers/contracts.py sync` (exit 0)
regenerated the three `contract-inventory.lock`, `contracts.json` and
`contract-load-path.json`; `contracts.lock` carries the pins. The
`cmetrics-counter` source policy pins the cmetrics inventory by digest
(`contract/profiles/cmetrics-counter/source-contracts.json`), re-pinned.

**Not validated.** None of the five bodies validates in its home unit: on the
tip each callee row is `Unknown` (`cmt_mpack_consume_string_tag`: uncontracted
`mpack_read_tag` and the store through the slot; `ctr_mpack_consume_string_or_nil_tag`:
uncontracted `ctr_mpack_peek_type`/`ctr_mpack_consume_string_tag`;
`rd_kafka_gzip_compress`, `rd_kafka_snappy_compress_slice`,
`rd_kafka_lz4_compress`: uncontracted zlib/snappy/LZ4F calls and stores
through `*outbuf`), so no exit-specific summary decides the dispositions and
each is a declared assumption pinned to the text read (C04.13: a body that
yields no summary leaves the declaration a declared assumption). Registered
(CG10-OOM-DECLARATIONS-UNVALIDATED). The nonzero class's slot may be written
NULL, which C04.13's unwritten class states as untracked; each declaration's
NOT CLAIMED says so.

## 6. Gate

Scoped card gate `python3 scripts/gate_scope.py --base 8e7e8d509 --out
private/oom/gate-3` through `private/toolchain/env.sh` (Clang 21, Lean 4.35)
on `677e822ac` (the code, contracts, tests and pins of the final tip; later
commits change this report only); mode full (the diff reaches
`qcore-c-gateway`), provider gate included. Record [gate/gate-3.json](gate/gate-3.json),
logs in the worktree's `private/oom/gate-3/`.

| step | exit | result |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `run_test_binaries.py` | 0 | 672 units, 717 summaries; 9,848 passed, 0 failed, 21 ignored, 148 filtered |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `check_lean_audits.py` | 0 | 475 audits |
| `python3 -m unittest discover -s scripts/tests` | **1** | 779 tests, 1 failure: `test_build_matrix_sections ... test_each_derived_dimension_matches_the_section_it_counts` (`dimensions.named_followups` 1041 against 1044 entries) |
| provider package build / verify-all / verify | 0 / 0 / 0 | |
| provider `cargo test` / fmt / clippy | 0 / 0 / 0 | 49 summaries, 168 passed, 0 failed, 1 ignored |

**The Python failure is the base's.** `docs/reviews/r0_3/acceptance_matrix.json`
is unchanged from `8e7e8d509` (`git diff --stat` empty): its derived dimension
was not regenerated after round 4's amendments added three entries, which the
integrator's matrix regeneration moves. Rerun alone with the card base
(`QUIRIN_CARD_GATE_BASE=8e7e8d509 python3 -m unittest
scripts.tests.test_build_matrix_sections`, `private/oom/python-matrix-rerun.log`):
the same single failure.

**Earlier attempts, not evidence.** `gate-1` was stopped by this card when tip
1 showed the manifest gap. `gate-2` (code at `af8c51fc0`): rust suite 9,844
passed, 3 failed: the two K03 pins asserting the refusal of `q_make(&s->p)`
(`cg03_call_instantiation`, `clang_gap_phase1`), which rule 72 admits and
which were flipped deliberately (`9931633bb`), and
`source_contract_config::shipped_cmetrics_selection_loads_canonical_policy`
(`contract inventory digest mismatch`: the `cmetrics-counter` policy pins the
cmetrics inventory, re-pinned in the same commit); the same Python failure;
every other step 0. The gate read census was recorded for the new target
(`scripts/gate_read_census.py record --packages qcore-c-gateway` and `reduce`,
exit 0 / 0; one unit added, `read: []`).

## 7. Follow-ups (in [integration-amendments.json](integration-amendments.json))

* **CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT** amended, not retired: the contract
  form and reading landed and freed 10 of the 13; the three librdkafka rows
  are blocked by CG10-W44-TYPEDEF-DEFINING-BASE-ACTUAL.
* **CG10-INDIRECT-ACCESS-SOLE-RESIDUALS** amended with this card's
  re-measurement (access alone 209 -> 200).
* **CG10-W44-TYPEDEF-DEFINING-BASE-ACTUAL** (new): rule W44 reads no member
  actual through a base spelled with a defining typedef
  (`rd_kafka_msgset_writer_t *msetw`, `msetw->msetw_rkb`).
* **CG10-OWNED-OUT-MEMBER-TRACKED-OWNER** (new): rule 72's tracked-owner case
  (a slot of a record this frame allocated, read through O04-B08's field
  link) keeps the refusal; no production declaration has a plain heap out slot.
* **CG10-OOM-DECLARATIONS-UNVALIDATED** (new): the five declarations are
  trusted assumptions; their bodies yield no summary.
* Not registered again: MEMBER-EFFECT-STORES (`unpack_opts_unit`), rule 5k's
  member releases (the five ctraces destroy-then-consume callbacks), the
  loop-generation refusal in `cmt_mpack_unpack_map`.

**Files likely to conflict.** `legacy_c.rs` (one `mod`, the environment, two
Parser fields, two hooks before rule 12k), `legacy_probe_state.rs`,
`tests/unit/legacy_probe_state.rs`, `pointer_store.rs` (`ValueClass::OwnedOut`),
`legacy_pointer_store.rs`, `legacy_null_premise.rs`, `member_store_retention.rs`,
`legacy_reserved_spelling.rs`, `contract_dependencies.rs`, `lib.rs`;
`providers/qcore-provider-librdkafka/{contracts.lock,contract-inventory.lock,contracts.json}`
(RK-TOPIC-PARTITION-LIST-CONSTRUCTORS), the cmetrics and ctraces locks,
`providers/contract-load-path.json`, `cycle19-resource-contracts.json`,
`contract/profiles/cmetrics-counter/source-contracts.json`, the generated
`docs/agents/gateway.md`, `docs/agents/README.md`, `proof/QCore.lean`,
`docs/QCORE_CONTRACT_SPEC_r0_3.md`, `docs/reports/gate-scope-precise-selection/read-census.json`,
and the two K03 test files. The OpenSSL provider is not touched.
