# OUTER-CONTRACT: outer contracts and the runtime projection (stage 1)

Card **OUTER-CONTRACT** (wave 48, round 31; card-high). Base: local `main`
`94aef2d3b`. The card was first built and measured on `a423fca5d`, which
differs from the design card's base `3f8f1a617` only in records. It was then
rebased twice, each time measured again against main's own binary
(section 3.4); sections 3.1–3.3 are the first base's measurement, and
their row lists are the final base's exactly:

* onto `b157c9cba`, where RETENTION-UNMATCHED-REMOVAL-POISONS changed rule
  12t, the retention code and libevent's contracts;
* onto `1474edaf1`, where NONNULL-MEMBER-UNIFIED-QUALIFIER merged the member
  clause into `nonnull_member(...) where quant #scope [[stable|scope]]` and
  re-pinned the null views. The envelope covers facts by their key
  (`stable:flb_*_instance.p`, unchanged), not by the clause's spelling.
* onto `94aef2d3b`, where FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS taught the
  gateway's lexical reader to read function-pointer members and formals
  (rule 5g). The reach is read from Clang's AST by its own reader, which
  already gives a function pointer no data edge, so it is not affected.

This card implements the
design of [OUTER-CONTRACT-DESIGN](../outer-contract-design/README.md) as the
user decided it. New gateway rule [31](../../agents/gateway.md#constraint-31);
rule 13h is amended in place. Contract specification C04.20 and grammar G05g
are new. `QCore.OuterContract` is the Lean model.
[PREDICTION.md](PREDICTION.md) was frozen before the measurement.

## 1. Authority

USER DECISIONS 2026-10-17, verbatim, relayed by the coordinator:

- **D1:** "B: loader envelope"
- **D2:** "Yes, as proposed": `#loaded`, `#registered`, `quant #outer [[contract]] NAME, "text"`, `quant #outer where #P in #loaded [[runtime]]`, and the envelope lines `quant #outer #loaded: TAG.FIELD |-> #kept;` and `quant #outer #loaded: #premise NAME;`
- **D3:** "Both from one run": publish static and runtime verdicts side by side from one dependency record; amend rule 13h for this reader.
- **D4:** "Later": register the flb_lib host (M8) as a follow-up.

The brief also scoped the card. This is stage 1 only. Stage 2 converts
librdkafka's `rk_no_plugin_library` into an envelope. It is registered, with
NONNULL-MEMBER-UNIFIED-QUALIFIER as its prerequisite, because that card is now
migrating the clauses that carry the premise.

## 2. What was built

| Piece | Where |
| --- | --- |
| The clause kind `quant #outer …` | `qcore_frontend::frontend` (`SourceContractClauseKind::QuantOuter`). Every other consumer rejects it: the resource and bounds loaders, views, inclusion. |
| `outer_contract_v1` loader, total-or-reject, named diagnostics | `crates/qcore-contract/src/outer_contract.rs` |
| `outer_view` in the load-path manifest, disjoint from the other two views | `load_path_manifest.rs`, `qcore_audit::providers::probe_resource_views` (a crate's outer modules are loaded together), `providers/contracts.py`, `providers/contract-load-path.json` |
| `quirin-audit pin --outer` and `outer-contracts.lock`; `--verify-contracts` verifies all three lock kinds | `crates/qcore-audit/src/contract_pin.rs` |
| Row record: `run_set_members` (rule 12n) and `stated_members` (rule 14e) | `qcore_audit::contract_dependencies::annotate_unit` |
| Reach from Clang's AST of the loader's own unit | `crates/qcore-c-gateway/src/outer_record_graph.rs` |
| Selection, pins, reach, facts, contributions, join, and the projection document | `crates/qcore-audit/src/outer_selection.rs`; hooks in the pinned, configured and no-build routes; `AuditReport::write_outer_projection`; checkpoints refuse the projection |
| `PendingReason::OuterContract` and the category `outer-contract` (human-resolvable, subject: analyser) | `qcore-schemas` 18.0.0 (a major bump: a new variant on an exhaustive enum) |
| The Fluent Bit envelope | `providers/quirin-provider-fluent-bit/contracts/fluent_bit/outer.qv`, `outer-contracts.lock` |

**The Fluent Bit contract.** `flb_external_plugin` is defined on
`flb_plugin_load` and cited on `flb_plugin_proxy_create`:

- binder `#config`;
- the guard `when #result == 0` on the C loader only;
- envelope: `flb_{input,output,filter,processor}_instance.p |-> #kept`.

Both bodies were read at 66910c10 and 046accf6 and are byte-identical there.
Both are pinned at the two revisions ([pin.sh](scripts/pin.sh); logs in
`private/ocd/pin-1/`):

- `pin --outer` exit 0 at each revision;
- `pin --outer --check` exit 0 at each, with 2 recorded and 0 not in force;
- `pin --verify-contracts` exit 0.

The contract's identity is
`flb_external_plugin@732783069826298b1a234c3637f62d7d4bd47c65a54484eb8c46a121ed5eab4e`.

**Closed-world facts.** A row's closed-world facts are:

- null rows: `stable:` and `member:` dependencies;
- custody rows: `run_set_members` and `stated_members`;
- both: every listed premise not in `NOT_CLOSED_WORLD`.

`NOT_CLOSED_WORLD` holds the build, reading and tracking premises:
`asserted_allocation`, `nested_call_admission`, `literal_argument`,
`mk_event_descriptors`, `untracked_handback`, `ingestion_active`,
`synchronous_output_reachability` and rule 13e's two. Any other premise is
closed-world, including a new one until it is listed. This corrects the design
census, which read `cjson_list_invariant`, `served_queue` and
`share_ack_batches_nonempty` as not closed-world. Each of the three says that
only the run's own code writes the state it names, so each is closed-world.

## 3. Measurement

| Run | Binary | Code | Projection |
| --- | --- | --- | --- |
| BEFORE | main's `2d42ddb4…` | identical to the base | none |
| AFTER | `7498ed1e…` (head `89c7b6e9a`) | this card | with the null property and the projection |

Each AFTER route ran under the heavy lock ([run.sh](scripts/run.sh),
[measure.sh](scripts/measure.sh), [run.json](artifacts/run.json)); every exit
was 0. Wall times inside the lock: corpus 911 s, system 223 s, slice 175 s. The
comparison is [compare.py](scripts/compare.py) and
[compare.json](artifacts/compare.json), which holds every moved row with its
reasons.

### 3.1 Static scope: unchanged

| Route | Custody report | Contract sidecar | Null rows and summary |
| --- | --- | --- | --- |
| corpus | `f2eb590a…` → `ab2cc808…`: identical once the two new dependency lists are removed (`run_set_members` on 140 rows, `stated_members` on 127); every status identical | byte-identical | byte-identical (`e95b1013…`) |
| system | byte-identical (`e6167449…`) | byte-identical | byte-identical |
| slice | byte-identical (`32ea1665…`) | byte-identical | byte-identical |

Static headline: corpus 939 / 9 / 3,886, null 1,114 / 1,435 / 2,354 / 18;
system 95 / 4 / 349, null 86 / 125 / 243; slice 52 / 1 / 347 / 4 Pending,
null 69 / 121 / 216. No static verdict moved.

### 3.2 Runtime scope: `flb_external_plugin` loaded

This is the only contract, so the scenario `flb_external_plugin` equals `all`.
Every loader reaches every record. The witness, read by Clang on
`src/flb_plugin.c` and `src/flb_plugin_proxy.c` on each route, is that
`struct flb_config`'s member `dso_plugins` is a `void *`.

| Route | Custody Safe → Pending / Unknown | Null Safe → Pending / Unknown | Combined Safe → Pending / Unknown | Runtime custody | Runtime null | Runtime combined |
| --- | ---: | ---: | ---: | --- | --- | --- |
| corpus | 0 / 32 | 1 / 162 | 1 / 32 | 907 / 0 / 3,918 / 9 | 2,386 / 1 / 2,516 / 18 | 793 / 1 / 4,100 / 27 |
| system | 0 / 0 | 2 / 0 | 2 / 0 | 95 / 0 / 349 / 4 | 209 / 2 / 243 / 0 | 63 / 2 / 385 / 4 |
| slice | 0 / 0 | 16 / 0 | 0 / 0 | 52 / 4 / 347 / 1 | 174 / 16 / 216 / 0 | 31 / 2 / 372 / 1 |

Runtime columns are Safe / Pending / Unknown / Unsafe. No Unknown, Pending or
Unsafe row moved, and nothing was promoted.

### 3.3 Every runtime-moved row, read

**Pending: the envelope's facts.**

* Corpus 1 and system 2 (both configurations): `flb_chunk_route_plugin_matches`
  (`flb_input_chunk.c`), combined Safe → Pending.
  * It tests `!o_ins || !o_ins->p || !o_ins->p->name`, calls `strlen`, and
    reads `o_ins->p->name` again.
  * The second read rests on `stable:flb_output_instance.p`, which the
    envelope covers.
  * Reason: `rests on outer contract flb_external_plugin@7327… through
    stable:flb_output_instance.p`.
* Slice 16: null Safe (conditional) → Pending. Combined stays Unknown, because
  custody is Unknown.
  * The functions: `flb_input_pause`, `flb_input_resume`,
    `flb_input_instance_pre_run`, `flb_input_instance_exit`,
    `flb_input_thread_instance_pause`, `flb_input_thread_instance_resume`,
    `flb_input_thread_instance_exit` and the caller
    `flb_input_rate_gate_maybe_resume`, each in both configurations.
  * Each keeps its entry fact `ins->p` across a call only through
    `stable:flb_input_instance.p`.
  * The three scope functions with no intervening call (`flb_input_plugin_pause`,
    `_resume`, `flb_input_thread_instance_pre_run`) rest on no closed-world
    fact and do not move. Their entry condition is published, and a loaded
    caller is bound by it.

**Unknown: librdkafka's closed-world facts.** All corpus. Fluent Bit's envelope
does not cover them, and a Fluent Bit plugin reaches them through `void *`.

* **Custody, 32 rows; each is also its combined row.**
  * 18 rest on `rk_no_plugin_library` alone. They are rows that
    LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED and -REMAINDER listed:
    `rd_kafka_crash`, `rd_kafka_dump`, the three `*ConsumerGroupOffsets_free`,
    `rd_kafka_topic_partition_result_free`, `rd_kafka_topic_partition_destroy`
    and `_destroy_free`, `rd_kafka_topic_partition_list_destroy_free`,
    `rd_kafka_handle_OffsetCommit_error`, `rd_kafka_handle_OffsetFetch_err_action`,
    `rd_kafka_share_oauthbearer_set_token_failure`,
    `rd_kafka_oidc_token_metadata_aws_iam_refresh_cb`, `rd_kafka_pause_partitions`
    and `_resume_partitions`, and the three telemetry unit tests.
  * `rd_kafka_toppar_purge_and_disable_queues` rests on `rk_no_plugin_library`
    and `served_queue`.
  * `rd_kafka_event_destroy` and `ut_op_destroy_free` rest on the
    `rd_kafka_op_s.rko_op_cb` and `rk_conf.sasl.oauthbearer.token_refresh_cb`
    run sets and on four premises (`rk_no_plugin_library` among them).
  * `cJSON_AddItemToArray` rests on `cjson_list_invariant`.
  * Nine `cJSON_Create*` / `cJSON_Print*` rows rest on `cjson_allocator_hooks`.
  * `ut_create_fetch_op_with_offset` rests on the stated members
    `rd_kafka_op_s.rko_op_cb` and `.rko_type`.
* **Null, 162 rows,** in 24 librdkafka units.
  * 87 rest on rule 14t's declared members alone, with no premise listed:
    `rd_kafka_topic_s.rkt_rk` and `.rkt_topic`,
    `rd_kafka_toppar_s.rktp_rkt`, `rd_kafka_cgrp_s.rkcg_rk`,
    `rd_kafka_transport_s.rktrans_rkb`.
  * 75 list `rk_no_plugin_library`. 18 rest on the premise alone, and 57 also
    on `stable:rd_kafka_broker_s.rkb_rk` or on declared members.
  * The combined rows `rd_kafka_broker_needs_connection`,
    `rd_kafka_cgrp_consumer_heartbeat_preconditions_met` and
    `rd_kafka_cgrp_update_session_timeout` move with them.

These Unknowns are the decided semantics, not a regression. With Fluent Bit
plugins loaded, nothing vouches that loaded code leaves librdkafka's objects
alone. Stage 2 (`RK-PLUGIN-LIBRARY-OUTER-ENVELOPE`) is what can make them
Pending.

### 3.4 After the rebases

The combined arbiter was run with [arbiter.sh](scripts/arbiter.sh) and
[runs.py](scripts/runs.py), once per base:

| Base | Main's binary | Tip's binary (head) | Artifacts |
| --- | --- | --- | --- |
| `b157c9cba` | `401f4b8f…` | `4dbe1215…` (`6bb6ea5ab`) | [run-rebased.json](artifacts/run-rebased.json), [compare-rebased.json](artifacts/compare-rebased.json) |
| `1474edaf1` | `d7082915…` | `56fa40be…` (`b1bc87261`) | measured; superseded by the next row (the same result) |
| `94aef2d3b` (final) | `82fe2ae9…` | `f5d3eb78…` (`2ef0ae282`) | [run-final.json](artifacts/run-final.json), [compare-final.json](artifacts/compare-final.json) |

Main's binary ran on main's own tree, and the tip's binary ran with the
projection. Every exit was 0, and each corpus `run.json` `binary_sha256`
matches its binary. On both bases the result is the first base's, row for row
(the final base's digests are below; the `b157c9cba` ones are in its
artifacts):

* **Static scope:** identical. The system report (`daa8ed82…`) and the slice
  report (`5b54df5e…`) are byte-identical. The corpus report (`4e3e56d8…` →
  `fd731251…`) is identical once the two new lists are removed (140 and 127
  rows). The sidecars, the null rows (corpus `533727d2…`, system `91dd87b0…`,
  slice `c24c889a…`) and their summaries are byte-identical. On the final
  base, main's own custody totals are corpus 941 / 9 / 3,884 and system
  97 / 4 / 347 (FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS); the null totals and
  the slice are unchanged.
* **Runtime scope on the final base, all loaded:**
  * corpus custody 909 / 0 / 3,916 / 9 (the same 32 Safe → Unknown), null
    2,386 / 1 / 2,516 / 18, combined 793 / 1 / 4,100 / 27;
  * system custody 97 / 0 / 347 / 4, null 209 / 2 / 243 / 0, combined
    63 / 2 / 385 / 4;
  * slice null 174 / 16 / 216 / 0, custody and combined unchanged.
* **Runtime scope:** the same tallies and the same moved rows on every route
  (the moved-row lists compare equal). The contract identity is unchanged.

The digests differ from the first base's because of main's own changes, which
appear identically on both sides.

### 3.5 Prediction against outcome

* **Static scope:** predicted identical, outcome identical. The null summary
  turned out byte-identical too; the prediction had allowed for its
  load-path digest to differ.
* **Null and system and slice rows:** predicted exactly: corpus null 1 / 162,
  system 2 / 0, slice 16 / 0.
* **Custody and combined (corpus):** predicted 0 / 31 and 1 / 31; outcome
  0 / 32 and 1 / 32. The extra row is `ut_create_fetch_op_with_offset`, which
  rests on a constructor's stated members. The prediction had left stated
  members out and called its counts lower bounds there.

## 4. Lean

`proof/QCore/OuterContract.lean`, audited by `proof/AuditOuterContract.lean`.
The theorems:

- `Verdict.join_comm`, `join_assoc`;
- `static_projection_is_identity`;
- `runtime_never_promotes`;
- `a_row_without_closed_world_facts_is_unchanged`;
- `a_covered_fact_is_pending_with_its_contract`;
- `a_pending_contribution_names_its_contract`;
- `every_outer_pending_names_a_contract`;
- `an_uncovered_fact_is_never_safe_or_pending`;
- `runtime_is_the_join`;
- `reachability_over_approximates`;
- `projection_vectors_hold`.

`QCore.ReasonCategory` gains `outerContract`. That makes eleven categories,
four of them human-resolvable (`exactly_four_categories_are_human_resolvable`,
renamed from `exactly_three_…` and re-audited), and adds the family
`outerContractCovered`. There are no placeholders and no project axioms.

## 5. Gate

GATE_SECTION

## 6. Ledger

`named_followups` went from 794 to 797 on main `94aef2d3b` (792 to 795 on the first base, 794 to 797 on `b157c9cba`, 793 to 796 on `1474edaf1`)
([update_ledger.py](scripts/update_ledger.py); idempotent;
`dimensions.named_followups` equals the list length). No entry was retired:
the design card registered none. Registered:

* `RK-PLUGIN-LIBRARY-OUTER-ENVELOPE`: stage 2, the `rk_plugin_library`
  envelope in place of the 163 premise clauses. Its PREREQUISITE is
  NONNULL-MEMBER-UNIFIED-QUALIFIER, and its own DISCHARGE names the envelope
  lines and the cross-library decision.
* `FLB-LIB-HOST-OUTER-CALLER`: the library-mode host (M8), D4 "Later".
* `OUTER-PROJECTION-CHECKPOINT-ROUTE`: the projection on checkpointed and
  composed runs, which refuse it today.

## Files

* **Scripts:**
  * [pin.sh](scripts/pin.sh), [run.sh](scripts/run.sh), [arbiter.sh](scripts/arbiter.sh), [build.sh](scripts/build.sh), [measure.sh](scripts/measure.sh), [runs.py](scripts/runs.py);
  * [compare.py](scripts/compare.py), [predict.py](scripts/predict.py);
  * [gate.sh](scripts/gate.sh), [provider-gate.sh](scripts/provider-gate.sh), [heavy.sh](scripts/heavy.sh);
  * [update_ledger.py](scripts/update_ledger.py).
* **Artifacts:** [compare.json](artifacts/compare.json) and
  [run.json](artifacts/run.json) (first base),
  [compare-rebased.json](artifacts/compare-rebased.json) and
  [run-rebased.json](artifacts/run-rebased.json) (`b157c9cba`),
  [compare-final.json](artifacts/compare-final.json) and
  [run-final.json](artifacts/run-final.json) (`1474edaf1`),
  [prediction.json](artifacts/prediction.json).
* **Private outputs:** `private/oc/`, plus the slice runs in
  `/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/oc-tip/`.
  The BEFORE runs stay in `private/ocd/` and
  `/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/ocd-main/`.
