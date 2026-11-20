# CG10-EMBEDDED-LIST-BORROWED-CONTAINER: embedded lists as borrowed container operations (gateway rule 69)

USER DECISION 2026-11-20, on round 3's decision class 1, question 1
([round 3](../cg10-indirect-access-sole-residuals-r3/README.md) section 4):
"Read embedded lists as borrowed container operations". Base `0111ddf38`
(round 3's tip, merged with main `b7f86a0de`). [Gateway rule 69](../../agents/gateway.md#constraint-69),
`crates/qcore-c-gateway/src/legacy_embedded_container.rs`, Lean
`QCore.EmbeddedListContainer` (19 audited theorems).
[PREDICTION.md](PREDICTION.md) was committed (`8655df642`) before the
implementation (`4aa64055c`).

## 1. Census (method step 1)

The 23 rows round 3 classes as blocked by the embedded-list class alone on the
opt-in route (Fluent Bit 046accf6, round 3's tip run `run-tip2`, binary
`cdcbf730…`: 1,140 Safe / 2 Pending / 3,655 Unknown / 7 Unsafe, 240 rows
blocked by the access refusal alone): [census/rows.md](census/rows.md) (one
row each: helper, embedded list, contract), from `scripts/rows.py` and
`scripts/rows_table.py` over round 3's tip premises. By helper:

| rows | helper (contract in force) | what it does to the elements | what decides the statement once the actual is read (base census stream) |
|---:|---|---|---|
| 5 | `rd_list_destroy` (2), `rd_list_clear` (3): registry invoked over the elements, elements dropped (+ unregistered, `released_if_constructed`) | **releases** them through the list's registration | route G: `the registry holder … is not one this body names (callback-registry-undecided)` (rule 13w) |
| 3 | `rd_list_add`: `resource_element_added(elem, rl)` | the element's custody passes only under a releasing registration the body holds (rule 13x) | the same registry refusal |
| 14 | `rd_kafka_interceptor_method_add`: all `Borrowed`, `resource_retained_by(func, list)`, `resource_retained_by(ic_opaque, list)` | adds an element it allocates itself; retains two caller objects in the caller's list | not in the stream (a whole-call `return`); rule 12t's retention |
| 1 | `cfl_list_size` (`pack_static_labels`): `Borrowed`, `None` result | none (reads the links) | the call is **nested** in `mpack_start_array`'s actual: refused `call` |

Every helper has a contract; none has an unstated effect. Rule 46 is not
involved in any row: it governs an element destroyer's unlink of an
intrusive `mk_list` link member, and none of these helpers unlinks a member
of an element it is handed.

## 2. The reading (method step 3): rule 69

A list helper handed `& R -> M …` / `& R . M …` (`R` a formal: the caller's
record) is a `Borrowed` operation on the caller's container. Rule W44 already
reads such an actual at a `Borrowed` position, and a helper with no container
clause is already admitted that way (`el_count(&c->in.on_new)` is `Safe` on
the base). What the gate hid is that every **container clause** on such a
container is refused by its own rule: rule 13w reads a registration only when
the body holds it (a holder reached through a formal never), rule 13x moves
an element only into a releasing registration the body holds, rule 12t
decides a retention only for a frame or tracked holder. W44 admits only a
reading that lowers, so it restored the gate's text.

Rule 69, asked last (after rule 38i at a statement, after rule 12k at a
whole-call `return`), only for exactly the gate's refusal of a brace-free
statement reading no tracked descriptor: when the first call has such an
embedded container actual and the statement, classified again with its
actuals read (`reclassified_with_borrowed_actuals`, shared with rule 12k), is
refused by a container custody refusal of that call quoting that actual
(`callback-registry-undecided`, `callback-element-custody-undecided`,
`retention-undecided`), the node keeps the gate's writes and takes that
description. It **admits nothing**: the decision's soundness clauses hold by
construction (a releasing helper is never a no-op, an added element never
passes to a container whose registration the body does not hold, an unstated
effect would not be a container refusal and keeps the gate). The holder is
recognised by its custody refusal, not by the helper's name, so a packer
registered in a caller's record (`msgpack_packer_init(&ic->mp_pck, …)`) is
read alike.

**What rule 69 needs from rule 46: nothing, and rule 46 is not amended.**
What would discharge the rows is a registration or retention carried by the
caller's object (CG22-REGISTRY-OUTSIDE-THE-FRAME,
RETENTION-UNDECIDED-PENDING), not container custody of elements.

**Lean** (`proof/QCore/EmbeddedListContainer.lean`, audited by
`proof/AuditEmbeddedListContainer.lean`): a container model where
`a_borrowed_only_helper_changes_no_custody`; the counterexample
`a_releasing_helper_is_not_a_noop` and
`the_noop_reading_misses_a_use_after_release` (reading `destroyElements`
under a releasing registration as a borrow leaves the released element live,
so a later use is judged sound); `an_unknown_registration_decides_no_destroy`,
`an_unknown_registration_decides_no_add`,
`an_element_added_to_an_undecided_container_has_no_one_outcome`; the rule
`reattribute` with `only_the_gate_refusal_is_read`,
`the_result_is_the_gate_refusal_or_a_container_refusal`,
`the_writes_are_kept`, `the_rule_never_admits`,
`the_verdict_of_every_node_is_unchanged`,
`the_status_of_a_function_is_unchanged`, four keep-the-gate theorems; the
recogniser `embeddedContainer` bound to the Rust's `CONTAINER_VECTORS`
(`container_vectors_hold`). No `sorry`, no project axiom.

**Fixtures.** `crates/qcore-c-gateway/tests/embedded_list_borrowed_container.rs`
(5 tests over `callbacks.qv`'s `c_list_*` and the new
`fixtures/contracts/embedded_list.qv`): destroy/clear/add on `&c->waiters`,
`&c->in.on_new`, `&k->conf.in.on_new` name the registry; a retaining helper
names the retention at a statement, a declaration and a whole-call return; a
borrowed-only helper was already `Safe`; a local record, a cast actual, a
released container (rule 38i's text) and a consuming element position keep
their old answers. Unit tests in the module: `container_vectors_hold`,
`only_a_container_refusal_about_the_actual_is_named`. One pinned fixture moved
deliberately (`retaining_call_minted_name::a_packer_in_a_formal_record_keeps_the_refusal`:
still `Unknown`, its leaf now the registry refusal; `0d13feac5`).

## 3. Measurement (method step 4)

Opt-in `current --monkey-linked` over Fluent Bit 046accf6 exactly as round 3
(`openssl-ssl-ctx-callback-slots/evidence/opt-in/opt-in-current.sh`, round 3's
prepared inputs read-only, a side tree `git archive f916d3007` as round 3's
runs used, the binary the only difference). Base: round 3's `run-tip2`
([evidence/run-base.txt](evidence/run-base.txt)); tip: binary `878fe032…`
([evidence/run-tip.txt](evidence/run-tip.txt); its summary lines equal the
base's but for the preparation timings).

| run | Safe / Pending / Unknown / Unsafe | access rows | access alone | access leaves |
|---|---|---:|---:|---:|
| base (`cdcbf730…`) | 1,140 / 2 / 3,655 / 7 | 2,190 | 240 | 9,149 |
| **tip (`878fe032…`)** | **1,140 / 2 / 3,655 / 7** | **2,160** | **223** | **9,090** |

Both exit 0. **No status move**: no Safe -> non-Safe, no new Unsafe, Pending
unchanged ([evidence/changes.json](evidence/changes.json),
`scripts/changes.py`). **41 rows change leaves, every one `Unknown` before
and after**: 59 access leaves removed, 59 added (43
`callback-registry-undecided`, 16 `retention-undecided`), none other.

* **12 of the 23 rows** are re-attributed: the 8 `rd_list_*` rows to the
  registry refusal (`rd_kafka_interceptors_destroy` all 15 leaves) and the 4
  `rd_kafka_conf_interceptor_add_on_*` rows to the retention refusal.
* **10 rows stay**: every `rd_kafka_interceptor_add_on_*`
  (`&rk->rk_conf.interceptors.on_X`). Rule 69's parity rows
  ([evidence/parity-interceptor-unit.jsonl](evidence/parity-interceptor-unit.jsonl),
  a one-unit census run) answer `an actual neither reader recognises`: rule
  W44's readers do not read this three-hop address (a typedef'd embedded
  record, then an untagged struct member, then the list), though they read
  `&rk->rk_conf.partition_assignors` and `&conf->interceptors.on_X`. A reader
  gap, not a container question (follow-up CG10-EMBEDDED-CONTAINER-THREE-HOP-ACTUAL).
* **`pack_static_labels` stays**, as predicted (the nested call).
* **29 rows outside the 23** are re-attributed alike: registry refusals in
  rdkafka destructors and adders (`rd_kafka_NewTopic_destroy`,
  `rd_kafka_headers_destroy`, `rd_kafka_header_add`,
  `rd_kafka_cgrp_partition_add`, `rd_list_init(&conf->interceptors.…)`, …)
  and 11 `rd_kafka_AdminOptions_*` rows (10 setters and `_copy_to`) to rule
  12t's retention (`rd_kafka_confval_set_type(&options->broker, …)`). 18 of them now carry no
  access leaf.

**Against the prediction.** No verdict move: as predicted. The 8 `rd_list_*`
rows and `pack_static_labels`: as predicted. Interceptor rows: 4 of the
predicted 14 (range 0-14); the 10 three-hop actuals were not foreseen.
Access-alone 240 -> 223 against a predicted 218 (range 218-232). Access
leaves removed: 26 on the 23 rows (predicted at most 37) and 33 outside them
(predicted 10-80).

**Pinned routes.** `quirin-audit measure --declared` cannot run on this host
(no `build/cycle19` inputs or baselines). Prediction for the integrator, in
[PREDICTION.md](PREDICTION.md)'s `measure-prediction` block: **no move** on
corpus, twelve-home, system or slice; leaf descriptions change on librdkafka
rows handing a formal's embedded container to `rd_list_*`,
`rd_kafka_interceptor_method_add` or `rd_kafka_confval_set_type`, and on any
`msgpack_packer_init(&x->…)` over a formal.

## 4. Gate (method step 5)

Scoped card gate `sh private/toolchain/env.sh python3 scripts/gate_scope.py
--base 0111ddf38 --out private/gate-N` (mode full: the diff reaches
`qcore-c-gateway`). Records: [gate/gate-1.json](gate/gate-1.json),
[gate/gate-2.json](gate/gate-2.json); logs in the worktree's `private/gate-1/`,
`private/gate-2/`.

| step | gate-1 (`4aa64055c`) | gate-2 (`0d13feac5`) |
|---|---|---|
| submodules | 0 | 0 (1 submodule) |
| `cargo fmt --all -- --check` | 0 | 0 |
| `cargo clippy --all-targets -- -D warnings` | 0 | 0 |
| `run_test_binaries.py` | **101**: 668 units, 9,794 passed, **1 failed** (the pinned fixture above), 21 ignored, 148 filtered | **0**: 668 units, 713 summaries; 9,795 passed, 0 failed, 21 ignored, 148 filtered |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | 0 / 0 |
| `check_lean_audits.py` | 0 (466 audits) | 0 (466 audits) |
| `python3 -m unittest discover -s scripts/tests` | 0 (779) | 0 (779) |
| gate | exit 1 | **exit 0, `gate_scope: 8 steps passed`** |

No flaky rerun was needed. Later commits add this report and the amendments
only.

**Gate read census.** `gate_read_census.py record --packages qcore-c-gateway`
and `reduce` on `0d13feac5` (logs `private/el/read-census/`, record and reduce
exit 0; 260 units traced, 1 not ok under the tracer:
`detach_lookup_membership`, 7 passed 1 failed under `strace`, which passes in
gate-2): the new target `qcore-c-gateway::test::embedded_list_borrowed_container`
reads no repository file at run time (its fixture contracts are
`include_str!`), as `retaining_call_minted_name` (unchanged from the committed
entry); [evidence/read-census-entries.json](evidence/read-census-entries.json).
The committed census (`docs/reports/gate-scope-precise-selection/read-census.json`)
is one commit's whole-suite observation and was **not** edited: adding one
package's entries would mix commits. For the integrator: the new unit's entry
is the empty read set above.

## 5. Follow-ups ([integration-amendments.json](integration-amendments.json))

`python3 scripts/apply_amendments.py integration-amendments.json --check
--profiles-base 0111ddf38` exits 0 on this tree **after** the integrator's r41
registration of CG10-EMBEDDED-LIST-BORROWED-CONTAINER is applied to the matrix
(`private/el/check-amend.sh` inserts it from the brief's entry text, checks,
and restores the matrix byte for byte); without it the check refuses the
retirement (`no registered follow-up has this id`), as it must on the base.

* **CG10-EMBEDDED-LIST-BORROWED-CONTAINER retired**: the reading is rule 69
  (Lean parity), the releasing helpers' element effects carried (never a
  no-op), prediction before implementation, no Safe -> non-Safe move.
* **CG22-REGISTRY-OUTSIDE-THE-FRAME amended**: its "118 access-gate refusals
  at member actuals of the now-classified calls" now name the registry where
  a formal's embedded container is the holder (43 leaves on the opt-in route).
* **RETENTION-UNDECIDED-PENDING amended**: 16 retention leaves (4
  interceptor adders; 12 in 10 `rd_kafka_AdminOptions_*` setters and
  `_copy_to`) now name
  rule 12t's undecided retention in a caller's container.
* **CG10-EMBEDDED-CONTAINER-THREE-HOP-ACTUAL (new)**: the 10
  `rd_kafka_interceptor_add_on_*` rows; W44's readers recognise
  neither reading of `&rk->rk_conf.interceptors.on_X`.
* **AG1-TWO-BORROWED-CALLS amended**: `pack_static_labels`'
  `mpack_start_array(writer, cfl_list_size(&cmt->static_labels->list))` is the
  nested half.
* **CG10-INDIRECT-ACCESS-SOLE-RESIDUALS amended**: the embedded-list class
  alone 23 -> 11; access-alone 240 -> 223.

## 6. Files likely to conflict

`crates/qcore-c-gateway/src/legacy_c.rs` (one `mod` block beside rule 12k's,
one hook after rule 12k's return attribution, one after rule 38i's statement
attribution: the sibling card CG10-MEMBER-STORE-RETAINED-BY may hook nearby),
the generated `docs/agents/gateway.md`, `docs/agents/README.md`,
`proof/QCore.lean` (rerun the generators), and
`crates/qcore-c-gateway/tests/retaining_call_minted_name.rs` (one fixture).
New files only otherwise. No provider, contract or lock is touched.
