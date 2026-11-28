# OUT-SLOT-MEMBER-THEN-FRESH: an owned member, its move-out, and a slot delivered or left unwritten

Card OUT-SLOT-MEMBER-THEN-FRESH. Base `287dce8ed` (main, r86).

* **First decision, then stop.** USER DECISION 2026-11-28 chose option (a),
  declaring the op's member custody. The card verified the claim and stopped
  with options (section 2), committing the probe pins `6aa742484`, the
  prediction `90d1d41b4` and the records.
* **Second decision, implemented.** USER DECISION 2026-11-28 on that stop
  chose **(a1)+(b)**. The card committed the amended
  [PREDICTION.md](PREDICTION.md) `1d0c037d0`, then the semantic commit
  `b1cfce380`.

It adds gateway [rule 96](../../agents/gateway.md#constraint-96), amends rule
[74l](../../agents/gateway.md#constraint-74l) in place with mode (b), adds
contract specification C04.19.4.1, and adds Lean `QCore.MemberMoveOut` (19
audited theorems).

**Outcome.**

* **The vocabulary.** It states an owned member of a possibly-null result
  (`resource_member_owned … when FIELD == VALUE`).
* **Rule 96.** It moves a member's object to a reader that the next statement
  nulls, whether the body linked the object or the member is a stated owned
  member.
* **Mode (b).** A slot that is delivered or left unwritten is read into a
  known-null slot only.
* **The fixtures.** They show each positive reading: a stated move-out
  released is `Safe`, a frame-linked move-out released is `Safe`, and a
  known-null caller of a mode-(b) callee is `Safe`. Every other caller is
  refused by name.
* **The two named rows stay `Unknown`.** Their bodies store the moved member
  into the slot before `rd_kafka_op_destroy(rko)`. That call may write caller
  memory, so rule 11r(8) poisons the bound slot and no summary validates
  (section 4). No compiled caller passes a known-null slot in any case.

**Measured: nothing moved.** The opt-in route, base against the semantic
tip, gives the same 1,209 / 2 / 3,586 / 7, no row moved and no leaf changed,
as predicted.

## 1. Probes at base

**Current route** (opt-in `current --monkey-linked`, Fluent Bit `046accf6`,
section 6): both rows are `Unknown`, with 3 indirect accesses, rule 74l's
slot refusal (`…list_new stored through *partitions` / `*topics`) and
unresolved `rko`, `rkcg` and the slot. Their only compiled caller is
`rd_kafka_share_subscription`, which forwards its slot. No Fluent Bit source
calls either function, and `examples/transactions.c` is not compiled.

**Fixtures** ([evidence/probe-fixtures-base.txt](evidence/probe-fixtures-base.txt),
the synthetic `c_*` op of `op_constructors.qv`) at base:

* a member read out of an op has no identity (`UnknownAlias`), even when the
  frame linked it;
* the member-then-fresh shape keeps rule 74l's refusal, and its caller keeps
  `call to uncontracted function`;
* the loader refuses a result state on a nullable result
  (`quant-live-unbacked`), a result member holding an object
  (`quant-live-value`), and an uncovered return-code class.

## 2. The custody claim (option (a))

* **The body moves the member out.** `rd_kafka_assignment`
  (`rdkafka_subscription.c:316-340`) does `*partitions =
  rko->rko_u.assign.partitions; rko->rko_u.assign.partitions = NULL;
  rd_kafka_op_destroy(rko);`, and `rd_kafka_subscription` (`:344-368`) does
  the same with `subscribe.topics`.
* **The destroy would free it otherwise.** `RD_IF_FREE(…,
  rd_kafka_topic_partition_list_destroy)` runs for `GET_ASSIGNMENT` and
  `GET_SUBSCRIPTION` (`rdkafka_op.c:380-390`), and the provider's
  `rd_kafka_op_destroy` arms state it.
* **The op owns the member.** The cgrp handler stores a copy
  (`rdkafka_cgrp.c:7519-7541`) into the request op and replies with that
  same op. `rd_kafka_op_reply` ORs in `RD_KAFKA_OP_REPLY`
  (`rdkafka_op.c:804-816`).

So the member is moved out and nulled before the destroy whose arm would free
it, and the claim is true. Under option (a) alone the vocabulary could not
state it. That stop led to the decision implemented below.

## 3. The change

**(a1) An owned member of a result** (C04.19.4.1;
`qcore_schemas::constructed_state::OwnedMember`,
`contract_constructed_state.rs`):

```qcore
ensures normal resource_member_owned(result, TAG.PATH) when FIELD == VALUE;
```

* **What it states.** Where the result's decided discriminant is exactly
  `VALUE`, the member holds an object the result owns, or null. This is a
  declared assumption.
* **What it needs.** `resource_discriminant_set` on the same declaration;
  `FIELD` the discriminant's member; `TAG.PATH` a member of that record
  other than the discriminant and every callback member.
* **Nullable results.** A declaration stating an owned member may return
  `resource_return_owned(nullable)`. Its states then describe the object on
  the non-null branch, and the gateway records them on the possibly-null
  result (`legacy_callback_route.rs` hook).
* **What still needs a non-null result.** The liveness part and every
  declaration without an owned member keep `quant-live-unbacked`.

**Rule 96** (`legacy_member_move_out.rs`, `LegacyInstruction::MoveOut`):

* **The lowering.** A read `D = B->M;` whose next statement is
  `B->M = NULL;` becomes a move-out over the read's own lowering.
* **Linked.** A link the body holds alone at `B->M` gives `D` an alias of
  the child, and the clear drops the link.
* **Stated.** An unspent owned-member state gives `D` a fresh possibly-null
  site and spends the state. This case is admitted only in a body that
  writes `M` nowhere else and never takes its address.
* **The guards.** An object handed to a call spends its owned states. A store
  over an unspent owned member is refused.
* **Everything else** is the read's own lowering, dispatched in full.
* **Relation to earlier rules.** The linked case is a bound read of a link
  that holds (O04-READ-BOUND's premise), taken at the read the clear follows.
  CARRIED-CHILD-TEARDOWN-AFTER-UNLINK asks for the same read at a release
  position after an unlink, and stays open. The stated case is the declared
  counterpart of rule 12o's arm: the object moved out is the one the arm
  would free, and the clear leaves the arm nothing.

**Rule 74l, mode (b)** (`legacy_keep_or_deliver_summary.rs` `mode`,
`qcore_schemas::keep_or_deliver::KeepOrDeliverSummary::null_only`,
`legacy_keep_or_deliver.rs`):

* **When it applies.** Rule 85's null entry reads W4's `Out { nullable }`
  and the owned entry fails.
* **The summary.** It is published `null_only`. The status does not need to
  partition: on a non-zero return the slot holds the delivery or whatever the
  caller left there.
* **The caller.** The lowering's null arm reads the delivered contract. The
  other arm is refused as `call to `F`, whose summary delivers its slot or
  leaves it unwritten, with a slot actual not known null`.
* **Unchanged.** The callee's row keeps 74l (2)'s refusal, and a C04.13
  disposition with an uncovered class is still refused.

**Lean** `QCore.MemberMoveOut`:

* `stateAdmitted` and `ownedAt` model (a1);
* `reading`, bound to `MOVE_OUT_VECTORS`, models rule 96, with
  `one_holder_after_the_move` and `an_uncleared_member_is_held_twice`;
* `slotMode`, bound to `SLOT_MODE_VECTORS` and run against Rust `mode`,
  models mode (b), with
  `a_null_only_mode_delivers_only_into_a_known_null_slot`,
  `the_null_only_reading_is_the_effect` and
  `an_unseeded_partial_store_reads_nothing`.

There are no placeholders and no project axioms.

**Provider** (`client.qv`, `rd_kafka_op_req2`): the declaration adds
`resource_discriminant_set(result, rd_kafka_op_s.rko_type, type, 1073741824)`
(the reply flag) and owned `rko_u.subscribe.topics` / `rko_u.assign.partitions`
under `23|REPLY` / `24|REPLY`. These are declared assumptions about the
replier, stated in the comment.

* `providers/contracts.py sync` exit 0.
* `quirin-audit pin --crate qcore-provider-librdkafka --function
  rd_kafka_op_req2` was run at `66910c10`, then `046accf6`, each exit 0, from
  a shared clone under `private/m/fb-pin`. The first voided one earlier
  identity and recorded a new one; the second was recorded beside it.
* `sync` exit 0 again, then `check` exit 0.
* The lock diffs (`contracts.lock`, `contract-inventory.lock`,
  `contracts.json`, `contract-load-path.json`) are in the semantic commit.
* The edit is registered in the SP2 premise checkpoint chain
  ([migration.json](migration.json), `tests/support/sp2_premise_checkpoint.rs`).

**Declaring `rd_kafka_assignment` / `rd_kafka_subscription` themselves.**
This was asked, but not done: mode (b) is a validated summary in rule 85's
table, and no contract clause states "delivers or leaves unwritten, read only
into a known-null slot". A declaration in today's vocabulary would be false:
`resource_out` claims every exit, and a C04.13 disposition needs every class
covered. What their bodies need is declared: `rd_kafka_op_req2`'s owned
members.

## 4. The named rows and their callers

The upstream order binds the slot before the destroy:

```text
*partitions = rko->rko_u.assign.partitions;   // move-out into the slot
rko->rko_u.assign.partitions = NULL;
rd_kafka_op_destroy(rko);                      // may write caller memory
if (!*partitions && !err) …
```

* **Validation fails.** In rule 85's validation the move-out is decided and
  the slot holds the moved list. `rd_kafka_op_destroy`'s actual is a local
  pointer's value, which rule 11r(8) reads as possibly designating caller
  memory, so the bound slot is poisoned. No mode reads, so there is no
  summary (pinned: `a_slot_bound_before_the_destroy_stays_refused`). The
  `!*partitions && !err` test and the indirect accesses block the body too.
* **The reported row.** The slot is unsanctioned, so the move-out reads the
  fallback.
* **No caller becomes `Safe`.** `rd_kafka_share_subscription` forwards its
  slot to its own caller, not a known-null `&NAME`. No other compiled caller
  exists.

Each row stays `Unknown` at its named leaves (section 6). Reaching them needs
either a premise sparing a call whose actual is a fresh owned object of the
frame from rule 11r(8), or the destroy moved before the slot store upstream.
Neither is this card's (follow-up below).

## 5. Pins

`crates/qcore-c-gateway/tests/op_constructor_declarations.rs` holds the
OUT-SLOT-MEMBER-THEN-FRESH section, 9 tests:

| test | kind |
|---|---|
| `a_stated_owned_member_moves_out_to_the_reader` | CHANGE PIN: released `Safe`, dropped `Leak` |
| `every_other_read_keeps_its_own_lowering` | fallbacks: another type, a second move-out, no clear, a call before, a store over the member |
| `an_ops_member_read_carries_no_custody` | kept from base |
| `a_moved_out_linked_member_goes_to_the_reader` | CHANGE PIN: released `Safe` (base `Unknown`), dropped `Leak`, no clear never `Safe` |
| `a_known_null_slot_takes_the_delivery` | CHANGE PIN: mode (b) caller `Safe`, ignored `Leak` |
| `a_slot_not_known_null_is_refused` | CHANGE PIN (text): held, uninitialised and possibly-null slots |
| `the_null_only_callees_own_row_keeps_its_refusal` | kept |
| `a_slot_bound_before_the_destroy_stays_refused` | the upstream order |
| `the_vocabulary_refuses_every_other_statement` | loader refusals |

Other tests:

* fixture contracts `c_op_req` and `c_item_make` (`op_constructors.qv`);
* `tests/unit/keep_or_deliver.rs`: `slot_mode_vectors_hold`,
  `a_null_only_summary_has_no_kept_reading`;
* `legacy_member_move_out.rs` unit tests: `the_vectors_hold`,
  `the_shapes_read`;
* `qcore-schemas`: the null-only reading has no kept contract.

## Gate

**The stop.** Scoped gate run 1, at `c7ece88bf`, recorded as
[gate/gate-1.json](gate/gate-1.json). Every step exited 0 except the Python
suite: `INDEX.md` was stale after the report was added, regenerated after.

**The implementation.** Scoped gate run 2: `python3 scripts/gate_scope.py
--base 287dce8ed --out private/gate` through `private/toolchain/env.sh`,
head `b1cfce380`, with this report's draft uncommitted. It ran in full mode,
with Lean all and the provider gate selected. The record is
[gate/gate-2.json](gate/gate-2.json), with logs in the worktree's
`private/gate/`.

| step | exit | result |
|---|---:|---|
| submodules, fmt | 0 / 0 | |
| `cargo clippy --all-targets -- -D warnings` | **101** | `compiler/examples/alias_aggregate_probe.rs` matches every `LegacyInstruction` variant and lacked `MoveOut` |
| `run_test_binaries.py` | **101** | the same build failure; no test ran |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `check_lean_audits.py` | 0 | 537 audits |
| `python3 -m unittest discover -s scripts/tests` | **1** | 814 tests, 1 failure: `docs/reports/INDEX.md` stale against the uncommitted report draft |
| provider package build / verify-all / verify | 0 / 0 / 0 | |
| provider `cargo test` | 0 | 182 passed, 0 failed, 1 ignored |
| provider fmt / clippy | 0 / 0 | |

The three failures were this card's own, and none came from base:

* **The example.** The missing arm was added in `fb5f6e2e3`.
* **Clippy.** Workspace `cargo clippy --all-targets -- -D warnings` then
  exits 0 (`private/m/clippy-all.log`).
* **The Rust suite.** Rerun standalone, `run_test_binaries.py` exits 0: 725
  units, 770 summaries, 10,424 passed, 0 failed, 24 ignored
  (`private/m/rust-suite-2.log`).
* **The index.** It was regenerated with `build_docs_index.py`. The Python
  suite then exits 0 on 814 tests (`private/m/python-4.log`), run with
  `QUIRIN_CARD_GATE_BASE=287dce8ed` as the gate runs it. Without that
  variable, `test_build_matrix_sections` compares the committed matrix's
  audit count (536) with the live one (537), the expected card-gate
  difference.

The whole gate was not rerun after these fixes. That would mean the provider
tests again (6,543 s), and the fixes touch one example's printer and
generated indexes.

Other checks:

* the LLP64 fixture scan over the changed test paths exits 0;
* `providers/contracts.py check` exits 0;
* `check_report_hygiene.py` exits 0;
* `apply_amendments.py … --check --profiles-base 287dce8ed` exits 0.

## 6. Measurement

[evidence/runs.md](evidence/runs.md): opt-in `current --monkey-linked` over
Fluent Bit `046accf6` (`private/fb-046accf6`, read-only), each revision from
its own `git archive` tree and its own release binary.

| run | Safe / Pending / Unknown / Unsafe | access rows / alone / leaves |
|---|---|---|
| base `287dce8ed` | 1,209 / 2 / 3,586 / 7 | 2,091 / 201 / 8,909 |
| stop tip `6aa742484` | 1,209 / 2 / 3,586 / 7 | 2,091 / 201 / 8,909 |
| semantic tip `b1cfce380` | **1,209 / 2 / 3,586 / 7** | 2,091 / 201 / 8,909 |

**No row moved and no leaf changed**
([moved-rows-base-tip-b1cfce380.jsonl](evidence/moved-rows-base-tip-b1cfce380.jsonl)
is `{"moves": {}}`, [changed-rows-base-tip-b1cfce380.txt](evidence/changed-rows-base-tip-b1cfce380.txt)
is empty). There is no `Unknown -> Safe` move to justify. Against the
prediction:

* **The named rows.** They keep their base leaves exactly
  ([named-rows-tip-b1cfce380.txt](evidence/named-rows-tip-b1cfce380.txt)). The
  predicted overwrite leaf did not appear, for a reason the prediction
  missed: in both bodies the call `rd_kafka_op_req2(rkcg->rkcg_ops, …)`
  reads a member of the unresolved `rkcg` (the `rd_kafka_cgrp_get` result),
  so its effects, the stated owned member included, never apply. `rko`
  stays unresolved and no owned state exists to overwrite.
* **Elsewhere.** No other `rd_kafka_op_req2` result's decided discriminant,
  and no frame-linked read-and-null in the corpus, changed a row. This
  matches the block's `none`.
* **`rd_kafka_share_subscription`.** It keeps `call to uncontracted function
  rd_kafka_subscription`. No compiled caller passes a known-null slot.

**System-slice route: not run.** Neither row is in `system-selection.json`.
**Pinned routes** (`quirin-audit measure --declared`) cannot run on this host
(no `build/cycle19` inputs or baselines). Not run, and no result is claimed.

## 7. Follow-ups

* **OUT-SLOT-MEMBER-THEN-FRESH** is retired. (a1)+(b) are implemented with
  Lean parity, and the two rows were re-measured: still `Unknown`, at the
  leaves of section 6.
* **OUT-SLOT-CALL-WRITE-FRESH-OBJECT** is new. Rule 11r(8) poisons a bound
  out slot at a call whose actual is a local pointer to a fresh owned object
  the frame acquired (`rd_kafka_op_destroy(rko)`). Whether such an actual
  may still designate caller memory (through the object's own members) is a
  premise for the user to decide.
