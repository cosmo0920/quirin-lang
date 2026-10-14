# OP-APP-RK-CALLBACKS-VIA-POLL-CB: application rk_conf callbacks

Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (wave 48, round 27). It amends gateway
rule **12n** in place, along with contract specification **C04.18**,
**C04.19.3** and **C04.19.4** and grammar **G05** / **G05d**; no new rule id
was needed. Lean: `QCore.AppRkCallbacks`.

Timeline:

* The census and the frozen [prediction](PREDICTION.md) were made on
  `2afce9973` and committed at the brief's pause point.
* After the user's decision (recorded verbatim in the prediction, section 5),
  the card was implemented on main `b7facc899`, which includes
  OP-CALLBACK-CANDIDATE-CONTRACTS and SERVE-FORWARDED-CALLBACK-FORMALS.

## 1. The decision

There were two decisions, both on 2026-10-14:

* **First:** "Closed over FB's registrations".
* **Second, after the census:** "Build the agent's recommended set". In detail:

| Choice | What it means |
| --- | --- |
| **D1** | A function default is stated with `|->`. |
| **Q2** | Each setter is declared `resource_callback_member_registered(conf, rd_kafka_conf_s.FIELD, cb)`, with a premise covering the property-table writer. |
| **Q3** | A member is keyed by its innermost named record, and a union's members share their stores. |
| **U2** | Every dependent verdict carries `P-RK-NO-PLUGIN-LIBRARY`. |
| **R1** | A facts pre-pass runs first, so each unit decides over the run's set. The fold stays demote-only. |
| **F1** | Member facts are read from every Fluent Bit production unit. The three Fluent Bit candidates are declared assumptions, each with its body read. |
| **E1** | A run-closed empty set invokes nothing. |
| **S1** | Rule 12m is kept. |

The census ([prediction](PREDICTION.md), sections 1–2) found four things:

* **No setter stores into its member.** Every write goes through
  `rd_kafka_anyconf_set_prop0`'s type-punned `_RK_C_PTR` store.
* **The run held none of Fluent Bit's Kafka units.**
* **A plugin library could register from outside the run.** librdkafka is
  built `WITH_PLUGINS`, and Fluent Bit forwards `rdkafka.plugin.library.paths`.
* **Rule 12m refuses `rd_kafka_poll_cb`.** It keeps the op on `PASS`.

## 2. What the card built

| Piece | Where | What it does |
| --- | --- | --- |
| Innermost keys (Q3) | `callback_member_closure.rs` (`callback_key`, `RecordKeys`, `Tables`, `Prepared`) | Keys a store by its innermost named record, so `rk->rk_conf.F` and `conf->F` are both `rd_kafka_conf_s.F`. A union's members share one key (`U.*` or `TAG.PATH.*`). A declared spelling is normalised through the run's record tables. A whole-object write of a record that holds the member by value opens it. A union key aliases the records it embeds. Under an `?.NAME` store, only function-pointer names are read. |
| Setter registration (Q2) | `CallbackClause::MemberRegistered`; the loader in `contract_callback.rs` | `resource_callback_member_registered(P, TAG.FIELD, F)` with a parameter holder. At the call it is the identity on the caller's state. For the closure, every call of the setter is a store of its `F` actual (`member_run_facts::declarations_unit`). |
| Function default (D1) | `MemberValue::Function`; `contract_constructed_state.rs` | `quant live #result: TAG.FIELD |-> f;` and `resource_callback_member_registered(result, TAG.FIELD, f)` mean the same. `f` is any identifier that names no parameter and is not `result`, `NULL` or `nullptr`. The default is a store of `f`. |
| Run facts (R1, E1) | `member_run_facts.rs`, `callback_contracts::decide_run`, the hooks in `legacy_callback_route.rs` | With the run's facts installed on the analysing thread (`scoped`), a member invocation decides over the run's set. A closed empty set is the identity. A member that the run's records do not define refuses. |
| Pre-pass (F1) | `qcore_audit::member_prepass`, hooked in `pinned_route.rs` | Preprocesses every unit of the production compile database (plus the run's Monkey units) with its own recorded command. Reads every C unit whose text names a token of an invoked member's closure graph. Tokens are record tags that can carry a store and the setters' and formals' functions; selection iterates to a fixed point. A unit that names a token but is not read opens every member. `contract/profiles/fluent-bit/member-facts-scope.json` names the units outside the program, each with its evidence ([scope check](artifacts/scope-check.json)). |
| Fold | `callback_member_fold::fold_with` | With the pre-pass, the run's closure is the pre-pass's. A row is kept exactly when that closure is closed. The fold never promotes. |

**Scope evidence** ([scope check](artifacts/scope-check.json),
[`scripts/scope_check.py`](scripts/scope_check.py)):

* **librdkafka's C++ wrapper (`src-cpp`).** It builds the `rdkafka++` target.
  The `fluent-bit-bin` and `fluent-bit-shared` link lines name
  `librdkafka.a` and no `rdkafka++`.
* **Six generated parsers.** They are absent from the pinned inputs. Their
  grammar sources name no final token.
* **LuaJIT (`src/` and `host/`).** These units need headers that `buildvm`
  generates. Their raw texts name no token.
* **Three parser drivers, `fluent-bit.c` and `flb_dump.c`.** They include a
  generated bison header or libbacktrace's `backtrace.h`, which are absent.
  Their raw texts and every header they resolve name no token.

## 3. Contracts

All are written by [`scripts/write_contracts.py`](scripts/write_contracts.py).
librdkafka's `src/` is byte-identical at `66910c10` and `046accf6`
([census](artifacts/rk-conf-census.json)). Every body was read by hand.

Pinning ([`scripts/pin.sh`](scripts/pin.sh), logs `private/oprk/pin-a1/`):

* both crates' checks are 0 at both revisions;
* the whole tree is 1 at both, only for the libevent tip-mode
  `mk_vhost_set`, by design;
* `--verify-contracts` is 0;
* `providers/contracts.py sync` and `check` are 0.

**`client.qv`**:

* **The seventeen dedicated callback setters**, each a declared assumption:
  * `background_event_cb`, `dr_cb`, `dr_msg_cb`, `consume_cb`,
    `rebalance_cb`, `offset_commit_cb`, `error_cb`, `throttle_cb`, `log_cb`,
    `stats_cb`, `oauthbearer_token_refresh_cb`, `socket_cb`, `connect_cb`,
    `closesocket_cb`, `open_cb`, `resolve_cb`, `ssl_cert_verify_cb`;
  * each carries the premises `conf_property_writer` and
    `rk_no_plugin_library`.
* **`rd_kafka_conf_new`**: owned non-null, under `asserted_allocation`. Its
  stated defaults are `log_cb |-> rd_kafka_log_print`,
  `socket_cb |-> rd_kafka_socket_cb_linux` and
  `open_cb |-> rd_kafka_open_cb_linux`.
* **`rd_kafka_consume_cb`**: the op consumed;
  `consume_ctx.consume_cb` invoked through `opaque`.
* **`rd_kafka_background_queue_serve`**: the op consumed. It invokes
  `rd_kafka_s.rk_conf.background_event_cb` and the eight rk_conf callbacks
  that `rd_kafka_poll_cb` invokes. Its op-carried callbacks are not stated,
  because a member invocation reads a borrowed or released object and the op
  here is consumed.
* **`rd_kafka_oauthbearer_refresh_op`**: the op borrowed;
  `rd_kafka_s.rk_conf.sasl.oauthbearer.token_refresh_cb` invoked.
* **The librdkafka candidates**, each the identity: `rd_kafka_log_print` and
  the five OAUTHBEARER refreshers (`rd_kafka_oauthbearer_unsecured_token` and
  four OIDC refreshers).
* **OP-CALLBACK-CANDIDATE-CONTRACTS' declarations:**
  * the nineteen `rk_log_callback` premises are restated as
    `rk_no_plugin_library`;
  * the mock cluster's `rk_closesocket_callback` goes with them, since
    `closesocket_cb` closes empty.

**`fluent_bit/logging.qv`**: Fluent Bit's three registered callbacks,
`cb_kafka_msg`, `cb_kafka_logger` and `oauthbearer_token_refresh_cb`. Each is
the identity and a declared assumption (no route defines them). They are
appended to an already-selected module, so no module count moves.

**Pinned tables:**

* 425 → **455** resource-v1 declarations;
* 57 → **78** retaining declarations;
* the clause rows ([`scripts/edit_pins.py`](scripts/edit_pins.py)), with the
  mock cluster's row going 9 → 8.

**Not declared:**

* **`rd_kafka_poll_cb`.** It takes the op on `HANDLED`, `KEEP` and `YIELD`
  and keeps it on `PASS`. No clause states that split beside its callback
  clauses (S1).
* **`rd_kafka_destroy_app`.** It is a residual, recorded in the
  `W48-CALLEE-DECISION-TABLE` amendment.
* **`rd_kafka_toppar_op_serve`.** It remains in
  `OP-CALLBACK-CANDIDATES-APP-CALLBACKS`.

## 4. The run's sets

From the corpus sidecar (AFTER `a5`, [summary](artifacts/summary.json)):

| Member | Run set |
| --- | --- |
| `rd_kafka_conf_s.dr_msg_cb` | {`cb_kafka_msg`} |
| `log_cb` | {`cb_kafka_logger`, `rd_kafka_log_print`} |
| `sasl.oauthbearer.token_refresh_cb` | {`oauthbearer_token_refresh_cb`, `rd_kafka_oauthbearer_unsecured_token`, the four OIDC refreshers} |
| `consume_cb`, `dr_cb`, `error_cb`, `rebalance_cb`, `stats_cb`, `throttle_cb`, `background_event_cb`, `consume_ctx.consume_cb` | closed and empty (E1) |
| `rd_kafka_op_s.rko_serve` | closed over 4, as before |
| `rd_kafka_op_s.rko_op_cb` | **open** |

`rko_op_cb` is open because Q3 keys the op's `rko_u` union as one key, and
that key is open over the run. The causes are a cast value, a `memmove` into
the `rd_kafka_group_member_s` the union holds, and whole-object assignments
of `rd_kafka_fetch_pos_s` and `rd_kafka_replyq_s`. Over the analysed units
alone it had closed over 27 (`RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION`).

**The pre-pass** (`a5`):

* 1,444 units: 845 read, 0 unread, 100 outside the program by the scope file;
* 3 rounds and 68 final tokens;
* 105 s alone ([`scripts/prepass.sh`](scripts/prepass.sh)).

## 5. Measurement

Runs:

* **BEFORE** `before2` (`d2bbd2e5…`) was built from `git archive b7facc899`
  into `private/oprk/base-src` and run there with its own contracts.
* **AFTER** `a5` (`6e51a944…`) was built from this branch.

Every route selected the null property, and each `run.json` names its binary.

| Route | BEFORE | AFTER | Report sha256 | Wall time (s) |
| --- | --- | --- | --- | --- |
| Corpus | 936 / 9 / 3,889 | **936 / 9 / 3,889** | `0a38c3b3…` → `620fbd75…` | 515.3 → 668.2 |
| System | 93 / 4 / 351 | 93 / 4 / 351, no row moves | `32b5a4ff…` → `e2cabcf7…` | 76.3 → 77.7 |
| Slice | 52 / 1 / 347, 4 Pending | the same, no row moves | `51b7836e…` → `f650d8ed…` | 38.5 → 38.3 |
| Null property | 1,113 / 1,435 / 2,355 / 18 Unsafe; 84 / 125 / 245; 71 / 103 / 232 | tallies equal; every row differs only in its contract identity digest | | |

**No Safe gain and no Safe → non-Safe move**, as predicted.

**Every moving row** ([corpus](artifacts/compare-corpus.json)): 97 corpus
rows, all Unknown before and after.

* **Lost leaves:**
  * 118 `rd_kafka_op_destroy` `callback-member-open` ("no store … in this
    unit");
  * 13 `call to uncontracted function rd_kafka_conf_new`;
  * one `rd_kafka_consume_cb`, decided in `rd_kafka_poll_cb` over the empty
    `consume_ctx.consume_cb`;
  * `rd_kafka_op_req0` / `_req` `callback-member-open`;
  * one other.
* **Gained leaves:**
  * 131 `rd_kafka_op_destroy` `callback-member-open` over the run's open
    `rko_op_cb`. Of these, 13 are leaves that OP-CALLBACK-CANDIDATE-CONTRACTS
    measured as decided; they refuse again, which is a leaf correction with
    no status move.
  * `rd_kafka_op_req0` / `_req` `callback-uncontracted` (`rko_serve` now
    closes over the run, and `rd_kafka_poll_cb` has no contract);
  * one `unresolved resource identity`.

No leak, violation or Unsafe was added. The fold demoted nothing.

**The served-queue callers.** `rd_kafka_poll` and
`rd_kafka_queue_poll_callback` keep their single leaf, the access-gate
refusal of `rk->rk_rep` / `rkqu->rkqu_q`. `rd_kafka_poll_cb` is unchanged at
4 / 4 / 0 (leaves / functions / alone).

**C04.18.** `rd_kafka_q_wait_result` refuses first at the uncontracted
`rd_kafka_q_pop`. The premise stays (see the C04.18 amendment).

**Prediction against outcome.** The prediction's second row held:

* leaves only;
* the rk_conf members close as listed;
* the null property is unchanged;
* `rd_kafka_poll_cb` stays refused (S1).

Three outcomes were not foreseen: the union key opens `rko_op_cb` over the
run, the pre-pass adds about 150 s of wall time, and the S1 refusal shows as
`callback-uncontracted` (not `callback-supplied-kept`), because the poll
callback cannot be declared at all.

**Discarded attempts** (retained under `private/oprk/`, not evidence):

* **`a1`.** Its pre-pass left 386 units unread (the gateway's strict option
  reader) and read 1,007 units (every member name was a token). The run was
  stopped.
* **`a2`–`a4`.** Pre-pass-only runs ([`scripts/prepass.sh`](scripts/prepass.sh))
  that refined the readers and the token set.

## 6. Lean

`proof/QCore/AppRkCallbacks.lean` is audited by
`proof/AuditAppRkCallbacks.lean`: 20 theorems, standard axioms only, no
placeholder and no project axiom. It covers:

* **Registrations as stores.**
  `a_registration_closes_the_member_only_with_every_run_store` and
  `the_registered_set_is_exactly_the_read_and_registered_sets`.
* **The run's decision.** `an_open_run_member_never_decides`,
  `a_closed_empty_run_set_invokes_nothing`,
  `a_closed_run_set_decides_as_the_unit_rule`,
  `a_unit_empty_set_stays_refused` and
  `the_fold_keeps_a_run_decision_exactly_when_closed`.
* **The pre-pass.** `skipping_a_silent_unit_keeps_the_run_set` and
  `an_unread_naming_unit_keeps_it_open`.
* **D1.** `the_function_shorthand_and_its_parts_mean_the_same`,
  `both_function_levels_are_one_canonical_set`,
  `the_absent_value_is_the_earlier_absent_state` and
  `a_stated_default_is_a_candidate`.
* **Q3.** `an_embedded_store_and_a_direct_store_share_the_key`,
  `a_normal_key_restarts_nowhere` and `normalise_is_idempotent`.
* **Vectors.** `key_vectors_hold` (Rust `KEY_VECTORS`) and
  `run_decision_vectors_hold` (Rust `RUN_DECISION_VECTORS`).

Lean audits go 320 → **321**.

Tests:

* `crates/qcore-c-gateway/tests/app_rk_callbacks.rs`, which covers a closed
  member, an open member (an unclassified store, an address-taken setter, an
  unread unit), a default candidate, E1 with a misspelled member, an embedded
  store, a record copy and union sharing;
* the unit tests in `member_run_facts`, `callback_member_closure` and
  `member_prepass`;
* the loader tests in `op_constructor_clause.rs` and
  `contract_constructed_state`.

## 7. Ledger

[`scripts/update_ledger.py`](scripts/update_ledger.py), 774 → **776**.

* **Retired:**
  * `CALLBACK-MEMBER-EMBEDDED-RECORD-KEYS`;
  * `LIBRDKAFKA-LOG-CALLBACK-PREMISE-UNSTATED`. Its second branch holds: the
    logger's run-closed candidates are declared identities, and
    `rk_log_callback` is retired everywhere.
* **Amended:**
  * this card's entry, now `rd_kafka_poll_cb` only. Its DISCHARGE (a contract
    or a named Pending for the poll callback) does not hold.
  * `OP-CALLBACK-CANDIDATES-APP-CALLBACKS`, now `rd_kafka_toppar_op_serve`;
  * `W48-CALLEE-DECISION-TABLE`, for `rd_kafka_destroy_app`.
* **Registered:**
  * `LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED`: 80 custody declarations and 86 null
    views do not list `P-RK-NO-PLUGIN-LIBRARY`;
  * `MEMBER-PREPASS-OTHER-ROUTES`;
  * `MEMBER-PREPASS-WALL-TIME`;
  * `RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION`.

## 8. Gate

See [gate/](gate/) and [gate.json](gate.json).

## Files

* **Schema and loader:**
  * `crates/qcore-schemas/src/{callback,constructed_state}.rs`;
  * `crates/qcore-contract/src/{contract_callback,contract_constructed_state}.rs`.
* **Gateway:**
  * `crates/qcore-c-gateway/src/{callback_member_closure,member_run_facts,callback_contracts,legacy_callback_route,legacy_callback_forward,legacy_constructed_route,contract_clause_standing,legacy_c,legacy_preparation,lib}.rs`;
  * `tests/app_rk_callbacks.rs`.
* **Audit:** `crates/qcore-audit/src/{member_prepass,callback_member_fold,report,pinned_route,lib}.rs`.
* **Contracts and profile:**
  * `providers/qcore-provider-librdkafka/contracts/librdkafka/client.qv`;
  * `providers/quirin-provider-fluent-bit/contracts/fluent_bit/logging.qv`;
  * the locks;
  * `contract/profiles/fluent-bit/member-facts-scope.json`.
* **Pins:** `compiler/tests/{contract_loader,qcore_contract_corpus}.rs`.
* **Lean:** `proof/QCore/AppRkCallbacks.lean`, `proof/AuditAppRkCallbacks.lean`,
  `proof/QCore.lean`.
* **Records:**
  * `docs/agents/gateway.md` (12n) and `docs/agents/README.md`;
  * `docs/QCORE_CONTRACT_SPEC_r0_3.md` (C04.18, C04.19.3, C04.19.4);
  * `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` (G05, G05d);
  * `docs/STATUS.md`;
  * `docs/development/increments/wave-48-part7.md`;
  * `docs/reviews/r0_3/acceptance_matrix.json`.
