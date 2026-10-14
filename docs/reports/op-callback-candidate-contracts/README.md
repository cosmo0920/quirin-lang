# OP-CALLBACK-CANDIDATE-CONTRACTS: contracts for the op-callback candidates

Card OP-CALLBACK-CANDIDATE-CONTRACTS (wave 48, round 27), gateway rules
**12n** and **13u** amended in place (no new rule id, no new surface word, no
Lean change). Base: local `main` at `2afce9973`; the census and the frozen
[prediction](PREDICTION.md) were committed (`26a60356b`) before the contract
writer ran.

## 1. The op callback protocol

Read at `66910c10` and `046accf6` (byte-identical): the result enum and its
remarks (`rdkafka_op.h:249-264`), `rd_kafka_op_call` (`rdkafka_op.c:933-943`),
`rd_kafka_op_handle_std` and `rd_kafka_op_handle` (`:1056-1126`),
`rd_kafka_op_destroy`'s head (`:345-354`) and the serving loops
(`rdkafka_queue.c:398-505`, `:558-650`).

| result | `rko_op_cb(rk, rkq, rko)` | `rko_serve(rk, rkq, rko, cb_type, opaque)` |
| --- | --- | --- |
| `PASS` | not handled; the op stays with `rd_kafka_op_handle` (handed to the serve callback or returned by a pop): borrowed | the callee keeps it (`q_serve0` asserts it never happens; `pop_serve0` returns it) |
| `HANDLED` | `rd_kafka_op_handle` destroys the op (`rd_kafka_op_call` clears `rko_op_cb` first): borrowed | taken (destroyed, replied or handed on) |
| `KEEP` | taken; forbidden under `RD_KAFKA_RESP_ERR__DESTROY` | taken |
| `YIELD` | taken (the callback must have re-enqueued or destroyed it); `rd_kafka_op_call` also returns `YIELD` for any result while `rd_kafka_yield_thread` is set | taken |

`rd_kafka_op_destroy` calls `rko_op_cb(rko->rko_rk, NULL, rko)` with
`rko_err = RD_KAFKA_RESP_ERR__DESTROY` and asserts neither `YIELD` nor `KEEP`:
there every candidate borrows the op.

Against the declared invocations (`rd_kafka_op_destroy`: `rko_op_cb(_, _, rko)`;
the serve and request families: `rko_op_cb(_, _, _)` and
`rko_serve(_, _, supplied_owned, cb_type, _)`):

* an `rko_op_cb` candidate is declared as rule 13u's identity (every formal
  `resource_borrowed`, `resource_return_none()`) only when it returns only
  `HANDLED` (or `PASS`) and releases, keeps or moves out nothing the op holds;
* an `rko_serve` candidate is declared as rule 12m's taking candidate
  (`resource_consumed(rko, nonnull)`) only when it never returns `PASS`;
* a candidate taking the op on `KEEP` only takes it on some return codes:
  rule 12m refuses that (user decision 2026-10-14), and C04.13 cannot state it
  anyway (a disposition needs a signed result; `rd_kafka_op_res_t` is `u32`,
  rule 12u). It is **not declared**, never weakened into a borrow.

This reading is rule 12m's (`QCore.OpCallbacks`:
`a_taking_candidate_takes_the_object` for the two serve candidates,
`a_kept_supplied_object_is_refused` for what is not declared) and rule 13u's
identity (`QCore.CallbackContracts.identity_only_borrows_or_requires`); no
decision rule changes, so Lean is unchanged (**319** audits).

## 2. Per-candidate outcome

Census: `scripts/candidate_census.py` ([census](artifacts/candidate-census.json),
a textual reading aid); every body read by hand at both revisions
(`scripts/body.py`, byte-identical). "App": an application callback the body
reaches; Fluent Bit registers three (`log_cb` = `cb_kafka_logger` and
`dr_msg_cb` in `out_kafka`, the oauthbearer `token_refresh_cb` in
`flb_aws_msk_iam.c`). Every candidate but `rd_kafka_oauthbearer_refresh_op`
and `rd_kafka_broker_monitor_op_cb` reaches `rd_kafka_log_buf` itself
(the monitor's callbacks do).

| candidate | member | results | op after each | under `__DESTROY` | standing |
| --- | --- | --- | --- | --- | --- |
| `rd_kafka_broker_monitor_op_cb` | op_cb | HANDLED | borrowed | returns | **declared**; premises `broker_monitor_callback`, `rk_log_callback` |
| `rd_kafka_cgrp_assignor_handle_Metadata_op` | op_cb | HANDLED | borrowed | returns | **declared**; `assignor_callback`, `rk_log_callback` |
| `rd_kafka_cgrp_consumer_handle_Metadata_op` | op_cb | HANDLED | borrowed | returns | **declared** |
| `rd_kafka_cgrp_handle_Metadata_op` | op_cb | HANDLED | borrowed | returns | **declared** |
| `rd_kafka_offset_reset_op_cb` | op_cb | HANDLED | borrowed | **runs a whole offset reset** (stated in its comment) | **declared** |
| `rd_kafka_offset_validate_op_cb` | op_cb | HANDLED | borrowed | **runs an offset validation** | **declared** |
| `rd_kafka_share_ack_commit_cb_register_op` | op_cb | HANDLED | borrowed | sets a flag | **declared** |
| `rd_kafka_share_fetch_reply_op_cb` | op_cb | HANDLED | borrowed | **runs the share-fetch reply** (new ops, counters) | **declared** |
| nine `rd_kafka_txn_op_*` (all but `send_offsets_to_transaction`) | op_cb | HANDLED | borrowed | return | **declared** (9) |
| `rd_kafka_txn_op_send_offsets_to_transaction` | op_cb | HANDLED, **KEEP** | taken on KEEP (the `AddOffsetsToTxn` request's opaque) | returns | residual: KEEP |
| `rd_kafka_admin_worker` | op_cb | HANDLED, **KEEP** | taken on KEEP (waiting for a broker, the controller, the broker list or a response) | fails the request | residual: KEEP |
| `rd_kafka_topic_partition_list_query_leaders_async_worker_op_cb` | op_cb | HANDLED, **KEEP** | taken on KEEP (the eonce waits for metadata) | replies, moving `rko_u.leaders.partitions` out | residual: KEEP |
| `rd_kafka_admin_fanout_worker` | op_cb | HANDLED | borrowed | runs | residual: destroys the fanout parent |
| `rd_kafka_DeleteRecords_leaders_queried_cb` | op_cb | HANDLED | borrowed | error path | residual: destroys the fanout op in `rko_u.leaders.opaque` |
| `rd_kafka_ListOffsets_leaders_queried_cb` | op_cb | HANDLED | borrowed | error path | residual: the same |
| `rd_kafka_share_commit_async_fanout_op` | op_cb | HANDLED | borrowed | runs | residual: hands `ack_batches` on and clears it |
| `rd_kafka_share_fetch_fanout_op` | op_cb | HANDLED | borrowed | runs | residual: the same |
| `rd_kafka_share_commit_sync_fanout_op` | op_cb | HANDLED | borrowed | runs | residual: moves `ack_batches`, `results` and the reply-queue reference into the group |
| `rd_kafka_oauthbearer_refresh_op` | op_cb | HANDLED | borrowed | returns | residual: calls `token_refresh_cb` (app-rk) |
| `rd_kafka_cgrp_op_serve` | serve | HANDLED | **taken on every return** | — | **declared**: `resource_consumed(rko, nonnull)` |
| `rd_kafka_mock_cluster_op_serve` | serve | HANDLED | **taken on every return** (`rd_kafka_op_reply`) | — | **declared**; `rk_closesocket_callback`, `rk_log_callback` |
| `rd_kafka_toppar_op_serve` | serve | HANDLED | taken on every return | — | residual: calls `rko_u.offset_commit.cb` (app-rk) |

**19 declared** (17 `rko_op_cb`, 2 `rko_serve`), **11 residual** (3 `KEEP`; 6
release or move out an object the op's members hold, a custody effect a plain
declaration cannot state: P-B08-SHALLOW would read it as untouched; 2 calling
an application callback). The "real work under `__DESTROY`" candidates the
ledger flagged (the 12 with no early test) are each either declared with that
work described in the declaration's comment (it touches no custody the
contract states: new ops hold their own references) or undeclared because the
work moves or releases what the op holds.

**The log callback.** `rd_kafka_log_buf` (`rdkafka.c:280-305`) calls
`rk_conf.log_cb` when no log queue is set, and `out_kafka` registers
`cb_kafka_logger` with none. That is an application rk_conf callback (the
user's decision on `rd_kafka_destroy_app`), so every declaration names the
dependency as the premise `rk_log_callback` (C04.18, published with the
declaration), which the app-rk card's logger contract discharges
(`OP-CALLBACK-CANDIDATES-APP-CALLBACKS`). The librdkafka declarations already
in force share the reach without naming it (`LIBRDKAFKA-LOG-CALLBACK-PREMISE-UNSTATED`).

## 3. Contracts

`qcore-provider-librdkafka/contracts/librdkafka/client.qv`, its own section
written by `scripts/write_contracts.py` after OP-CONSTRUCTOR-DECLARATIONS'
(the app-rk card writes its own), every declaration pinned at both revisions
(`scripts/pin.sh`, logs `private/opcand/pin-a1/`: the librdkafka crate's
pin and check 0 at both, the whole-tree check 1 at both only for the libevent
crate's tip-mode `mk_vhost_set`, by design; `--verify-contracts` 0);
`providers/contracts.py sync` and `check` 0. Pins: resource-v1 declarations
406 → **425** (`compiler/tests/contract_loader.rs`, all in the function map),
the previous-mode table, the per-declaration clause counts
(`compiler/tests/qcore_contract_corpus.rs`; targeted run
`private/opcand/targeted-a2.log`, 14 passed). Standing (the corpus sidecar):
all 19 `declared-assumption`, `mode: pinned`, each analysed and unvalidated
in its one defining unit (no body there yields a W4 summary); declarations
consulted 372 → 391.

## 4. Measurement

One binary, `private/opcand/bin/before` (`bcb78670…`, built from `2afce9973`):
`client.qv` is read at run time, so BEFORE ran it on the base contracts and
AFTER (`a1`) on the card's; every route with `QCORE_NULL_PROPERTY`
([summary](artifacts/summary.json); each `run.json` names its binary).

| route | BEFORE | AFTER | report sha256 | wall (s, AFTER) |
| --- | --- | --- | --- | ---: |
| corpus `66910c10` | 936 / 9 / 3,889 | **936 / 9 / 3,889** | `ec4ccfeb…` → `0a38c3b3…` | 524.1 |
| system | 93 / 4 / 351 | 93 / 4 / 351 | `61d1983d…` → `7a752ba7…` (contract digests only) | 78.7 |
| slice | 52 / 1 / 347, 4 Pending | 52 / 1 / 347, 4 Pending | `8045d095…` → `ab6d13a6…` (the same) | 38.9 |
| null, corpus | 1,113 / 1,435 / 2,355 / 18 Unsafe | identical verdicts | | |
| null, system / slice | 84 / 125 / 245; 71 / 103 / 232 | identical verdicts | | |

The null rows differ only in `identity.resource_contracts_sha256`
([null](artifacts/null-compare.txt)).

**No Safe gain**, no Safe → non-Safe, no leak, violation or `Unsafe` added
(as predicted). **Every moving row** ([corpus](artifacts/compare-corpus.json);
system and slice: none) is Unknown before and after:

* **`rd_kafka_op_destroy`'s callers**: its 13 `callback-uncontracted` leaves
  (7 rows in `rdkafka.c`, whose own set is `{rd_kafka_share_fetch_reply_op_cb}`:
  `rd_kafka_committed`, `rd_kafka_consume_cb`, `rd_kafka_consumer_close0`,
  `rd_kafka_memberid`, `rd_kafka_poll_cb`, `rd_kafka_seek_partitions`,
  `rd_kafka_share_commit_sync`; 1 in `rdkafka_offset.c`,
  `rd_kafka_commit_queue`, set `{offset_reset, offset_validate}`) now decide
  as the identity; each row stays Unknown on its other leaves. Leaves of
  `rd_kafka_op_destroy`: 131 / 83 / 0 → **118 / 75 / 0** (leaves / functions
  / alone; [before](artifacts/size-before-corpus.json),
  [after](artifacts/size-after-corpus.json)), all `callback-member-open`: the
  unit stores nothing into `rko_op_cb`. Had any of these rows become `Safe`,
  rule 12n's fold would have demoted it (the run's 27 candidates are in no
  unit's set); the fold demoted nothing.
* **`rd_kafka_cgrp_op_serve`'s own row**: its declaration is its entry
  contract, so `rko` is now a tracked owned formal; three statements naming
  it (`rdkafka_cgrp.c:7390` and `:7401`, the two `rd_kafka_dbg` expansions
  reading `rd_kafka_op2str(rko->rko_type)`, and `:7414`, the `NAME` arm's
  `rko->rko_u.name.str = RD_KAFKAP_STR_DUP(...)`) trade `indirect memory
  access is outside the resource projection` for `unmodeled expression may
  mutate a formal resource identity`.

**C04.18 on `rd_kafka_q_purge0`: kept.** Its premise is about a reference on
the served queue reachable through `rk` or the op (the callbacks receive
`rkq = NULL`); no candidate contract states that (they are custody over their
formals only), 11 candidates stay undeclared, and a re-declaration with the
member clause would be demoted by the fold in every unit. Nothing moves:
`rd_kafka_q_purge0`'s declaration is unchanged and
`rd_kafka_toppar_purge_and_disable_queues` keeps its status.

**Prediction against outcome**: as predicted on every route (no status move,
the 13 leaves, the C04.18 decision, null unchanged); not predicted: the
`rd_kafka_cgrp_op_serve` own-row leaf trade.

## 5. Ledger

`scripts/update_ledger.py` (idempotent). **`OP-CALLBACK-CANDIDATE-CONTRACTS`
is not retired**: its DISCHARGE asks for a contract in force for every
candidate. It is amended in place to the nine candidates that need a clause or
a decision (3 `KEEP`, 6 interior release or move). Registered:
`OP-CALLBACK-CANDIDATES-APP-CALLBACKS` (the two application-callback
candidates and the `rk_log_callback` premise, after
`OP-APP-RK-CALLBACKS-VIA-POLL-CB`), `MEMBER-CANDIDATE-DEPENDENCIES-UNLISTED`
(a row deciding a member through candidates its body does not spell does not
list them or their premises; 0 Safe or Pending rows on this run),
`LIBRDKAFKA-LOG-CALLBACK-PREMISE-UNSTATED`. `dimensions.named_followups` 771 →
**774**.

## 6. Gate

Standing gate and provider gate **g1** at `b9ba30ec5` (every record but this
section), one exclusive flock on the shared heavy lock held for the whole gate
(12:10:51Z-12:28:58Z; `scripts/gate-run.sh`, `gate-whole.sh`, `gate.sh`; exits
in [gate/](gate/), logs `private/opcand/gate-g1/`, [gate.json](gate.json)):
`cargo check --workspace --all-targets` 0; submodule 0; `cargo fmt --check` 0;
`clippy --locked --all-targets -D warnings` 0; `run_test_binaries.py` **493
units, 506 summaries, 8,023 passed, 0 failed, 14 ignored**, exit 0
(`private/opcand/tests-g1/`), no rerun; both reevaluation probes 0;
`check_lean_audits.py` **319** audits passed; Python suite 365 OK (08); docs
index check 0 (09); provider gate: `package.py` build 0, verify
`qcore.libevent` 0 and `qcore.librdkafka` 0, `cargo check --locked` in
`providers/` 0, `contracts.py check` 0, `cargo test` 133 passed / 0 failed,
providers Python suite 25 OK, fmt 0, clippy 0; `cargo clean --profile dev` 0.
Then `build_matrix.py . --gate gate.json` (09a) 0, the Python suite again
(09b) 0, the docs index (10b) 0 and its check (10c) 0. Clang:
`gate/clang-version.txt`.

## Files

* Contracts: `providers/qcore-provider-librdkafka/contracts/librdkafka/client.qv`,
  `contracts.lock`, `contracts.json`; pins
  `compiler/tests/{contract_loader,qcore_contract_corpus}.rs`.
* Records: `docs/agents/gateway.md` (12n, 13u), `docs/STATUS.md`,
  `docs/development/increments/wave-48-part7.md`,
  `docs/reviews/r0_3/acceptance_matrix.json`, `docs/reports/INDEX.md`.
