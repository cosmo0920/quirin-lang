# OP-APP-RK-CALLBACKS-VIA-POLL-CB: census, design and prediction

Frozen before any implementation code, at the brief's pause point. The census
found that the user's decision cannot be carried out with the current surface
and rules alone. Section 3 lists the proposals that need the user's choice;
nothing in it is implemented. Base: local `main` at `2afce9973`. Rule
placement: amend 12n and C04.18 in place; a new id, if one is truly needed,
is `30`.

> **USER DECISION 2026-10-14** (as `w48-common-r27.md` relays it, its list joined into prose): destroy_app /
> application rk callbacks: "Closed over FB's registrations". The callbacks in
> `rk->rk_conf` (consume_cb, dr_cb, dr_msg_cb, error_cb, log_cb, rebalance_cb,
> stats_cb, throttle_cb, background_event_cb, interceptors' on_destroy) are
> read like rko_op_cb. Their candidates are exactly the callbacks Fluent Bit
> stores via `rd_kafka_conf_set_*_cb` (and equivalents) in the corpus, closed
> run-wide. Any unseen or unclassified store opens the set (Unknown). Each
> candidate needs its own contract. Rejected: named Pending, a custody-neutral
> declared assumption, keeping Unknown.

## 1. Census on the base

Every body below was read by hand. librdkafka's `src/` is byte-identical at
`66910c10` and `046accf6` ([census](artifacts/rk-conf-census.json),
`librdkafka_src_sha256`). The Fluent Bit candidates `cb_kafka_msg`,
`cb_kafka_logger` and `oauthbearer_token_refresh_cb` are identical at both
revisions (`scripts/body.py`). `out_kafka`'s other files differ, but their
setter calls do not. [`scripts/rk_conf_census.py`](scripts/rk_conf_census.py)
is a textual reading aid; Clang's AST is the authority at implementation.

### 1.1 The rk_conf callback members

`struct rd_kafka_conf_s` (`rdkafka_conf.h:220-654`) has 19 function-pointer
members, plus 14 interceptor method lists
(`interceptors.on_conf_set` … `on_broker_state_change`, `rd_list_t` of
`rd_kafka_interceptor_method_t`).

| member | default (`pdef`) | registrations in the run | invoked at |
| --- | --- | --- | --- |
| `dr_msg_cb` | none | FB `out_kafka/kafka_config.c:108` ← `cb_kafka_msg` | `rd_kafka_poll_cb` (`rdkafka.c:5934`) |
| `log_cb` | **`rd_kafka_log_print`** | FB `kafka_config.c:111` ← `cb_kafka_logger`; `rd_kafka_set_logger` (`rdkafka.c:417`, a formal; no caller in the run) | `rd_kafka_poll_cb` (`:6001`), `rd_kafka_log_buf` (`:303`) |
| `sasl.oauthbearer.token_refresh_cb` | none | FB `src/aws/flb_aws_msk_iam.c:775` ← `oauthbearer_token_refresh_cb`; `rd_kafka_new` on `&rk->rk_conf` (`rdkafka.c:2655-2690`) ← `rd_kafka_oauthbearer_unsecured_token` and four OIDC refreshers | `rd_kafka_oauthbearer_refresh_op` (an `rko_op_cb` candidate, `rdkafka_sasl_oauthbearer.c:180`), `:1330` |
| `consume_cb` | none | none (`rd_kafka_consume_callback*` hand theirs through `struct consume_ctx`; no caller in the run) | `rd_kafka_consume_cb` via `ctx->consume_cb` (`rdkafka.c:4950`) |
| `rebalance_cb`, `offset_commit_cb`, `dr_cb`, `error_cb`, `throttle_cb`, `stats_cb`, `background_event_cb` | none | none | `rd_kafka_poll_cb`; `background_event_cb` in `rd_kafka_call_background_event_cb` (`rdkafka_background.c:51`); `offset_commit_cb` copied into an op's `rko_u.offset_commit.cb` (`rdkafka_cgrp.c:4364`, `:4757`) |
| `socket_cb`, `open_cb` | `rd_kafka_socket_cb_linux`, `rd_kafka_open_cb_linux` | none | transport (`rdkafka_transport.c:1133`), offset file (`rdkafka_offset.c:159`) |
| `connect_cb`, `closesocket_cb`, `resolve_cb`, `ssl.cert_verify_cb` | none | none | transport, SSL |
| `sasl.scram_H` | none | direct stores `SHA1`, `SHA256`, `SHA512` (`rdkafka_sasl_scram.c:752-760`) | SCRAM |
| `ut.handle_ProduceResponse` | none | the string API (a hidden test property) | `rdkafka_request.c:5043` |
| interceptor lists | empty | `rd_kafka_conf_interceptor_add_*`: no caller in the run | `rd_kafka_interceptors_on_*`, for example `method->u.on_destroy(rk, …)` (`rdkafka_interceptor.c:412`) |

Fluent Bit also calls `rd_kafka_conf_set_opaque` (`in_kafka.c:352`,
`kafka_config.c:227` / `:236` at `046accf6`, `flb_aws_msk_iam.c:777`). It sets
the data pointer every callback receives; it is not a callback.

### 1.2 How a value reaches a member (the stores)

1. **The property-table writer.** No dedicated setter stores into its member.
   Each one is
   `rd_kafka_anyconf_set_internal(_RK_GLOBAL, conf, "dr_msg_cb", dr_msg_cb)`
   (`rdkafka_conf.c:3007-3013`; the macro is at `:2606-2615`). The macro looks
   the name up in `rd_kafka_properties[]` and ends in
   `rd_kafka_anyconf_set_prop0`'s `_RK_C_PTR` case:
   `*_RK_PTR(const void **, conf, prop->offset) = istr;` (`:2087-2088`). That
   is a type-punned store through a byte offset into a `void *conf`. The same
   writer serves four other routes:
   * `rd_kafka_defaultconf_set` for the `pdef` defaults (`:2500-2520`, called
     by `rd_kafka_conf_new`, `:2523-2530`);
   * `rd_kafka_anyconf_copy` (`rd_kafka_conf_dup`), which copies a property's
     value from another conf (`:2880-2936`);
   * the string API `rd_kafka_conf_set`, which refuses `_RK_C_PTR` unless the
     property is `_RK_HIDDEN` (`:2243-2255`, `allow_specific` 0 at `:2585`).
     The only hidden pointer property is `ut_handle_ProduceResponse`.
   * Fluent Bit forwards every `rdkafka.*` property to the string API
     (`flb_kafka.c:84-86`).

   Clang's member facts (rule 12n) see none of these stores.
2. **The struct copy.** `rk->rk_conf = *conf;` (`rd_kafka_new`,
   `rdkafka.c:2574`) copies the whole application conf into the handle.
   Invocations read `rk->rk_conf.F`; the setters write `conf->F`.
3. **Direct member stores:** `rd_kafka_set_logger`, `scram_H`, and the
   `rd_kafka_new` setter calls on `&rk->rk_conf`.
4. **Unseen: the plugin loader.** The build has `WITH_PLUGINS 1` and
   `WITH_LIBDL 1` (`generated/config.h:35-36`). The property
   `plugin.library.paths`, reachable through Fluent Bit's forwarded
   `rdkafka.*` properties, makes `rd_kafka_plugin_new` call
   `conf_init(conf, …)` through `rd_dl_sym` (`rdkafka_plugin.c:90-101`). Code
   outside the run then holds the conf. It can call any setter or
   `rd_kafka_conf_interceptor_add_*` on it. The interceptors' `on_new`
   (`rd_kafka_interceptors_on_new(rk, &rk->rk_conf)`, `rdkafka.c:2585`) also
   hands foreign code the handle's own conf.

### 1.3 The invocation sites (bodies at both revisions)

* **`rd_kafka_poll_cb(rk, rkq, rko, cb_type, opaque)`** (`rdkafka.c:5813-6096`).
  * It returns `PASS` and leaves the op with its caller on seven paths: event
    setup under `RD_KAFKA_Q_CB_EVENT`; `FETCH` with no `consume_cb` or a
    `RETURN` type; an offset-commit reply with no op callback; a share ack
    with no callback; `CONSUMER_ERR` and `ADMIN_RESULT` under `RETURN`;
    `SHARE_FETCH_RESPONSE`.
  * `KEEP` comes from `rd_kafka_op_call` or `rd_kafka_metadata_update_op`.
  * `YIELD` re-enqueues the op or destroys it.
  * `HANDLED` destroys it.
  * It invokes `rebalance_cb`, `error_cb`, `dr_msg_cb`, `dr_cb`,
    `throttle_cb`, `stats_cb` and `log_cb` from `rk->rk_conf`, each behind a
    non-null test. It reaches `consume_cb` through `rd_kafka_consume_cb`.
  * It also invokes three op or handle members: `rko_u.offset_commit.cb`,
    `rko_u.terminated.cb` and `rk_share_consumer.share_acknowledgement_commit_cb`.
  * It reaches `rko_op_cb`'s 27 candidates through `rd_kafka_op_call`.
  * Its other calls (`rd_kafka_assign`, `rd_kafka_purge`, `rd_kafka_q_reenq`,
    `rd_kafka_msg_destroy`, …) act on the handle and on the op's payload.
* **`rd_kafka_consume_cb`** (static, `rdkafka.c:4932-4955`). It destroys the
  op on both paths and returns `HANDLED`. It invokes
  `consume_ctx.consume_cb` through its `opaque`.
* **`rd_kafka_background_queue_serve`** (static,
  `rdkafka_background.c:67-103`).
  * When `background_event_cb` is set and the op is eventable, it hands the
    op to that callback ("Event must be destroyed by application").
  * Otherwise it calls `rd_kafka_poll_cb(…, RD_KAFKA_Q_CB_CALLBACK, …)`. It
    returns `HANDLED` or `YIELD` as that call does, and destroys the op
    otherwise. That includes `KEEP`; this was read and is not a finding.
* **`rd_kafka_destroy_app(rk, flags)`** (`rdkafka.c:1150-1287`).
  * It releases `rk` (`rd_kafka_destroy_final`) only without
    `RD_KAFKA_DESTROY_F_IMMEDIATE`.
  * Its effects include `rd_kafka_consumer_close` (which serves with
    `rd_kafka_poll_cb`), `rd_kafka_log` (`log_cb`), the share consumer's
    close, and, in `rd_kafka_destroy_final`, the interceptors' `on_destroy`.
    The interceptor lists are filled only from outside the run (1.2.4).

### 1.4 Candidates and their effects

| candidate | registered by | body | reading |
| --- | --- | --- | --- |
| `cb_kafka_msg(rk, rkmessage, opaque)` | FB out_kafka | logs through `flb_plg_warn`/`debug`, `rd_kafka_err2str` | identity: every formal borrowed, `resource_return_none` |
| `cb_kafka_logger(rk, level, fac, buf)` | FB out_kafka | `rd_kafka_opaque`, `rd_kafka_name`, `flb_plg_*` | identity |
| `oauthbearer_token_refresh_cb(rk, config, opaque)` | FB msk_iam | builds and frees a payload, a provider and credentials; `rd_kafka_oauthbearer_set_token[_failure](rk, …)` | identity on its formals; balanced inside |
| `rd_kafka_log_print` | `log_cb`'s default | `fprintf(stderr, …)` | identity |
| `rd_kafka_oauthbearer_unsecured_token`, the four OIDC refreshers | `rd_kafka_new` | read in implementation if decided | |

### 1.5 Sizes on the base, and what the run holds

**BEFORE run.** Binary `bin/before` (`cf786013…`, built from `2afce9973`),
null property selected. Corpus **936 / 9 / 3,889**, `ec4ccfeb…`, which is
the brief's figure; 539.5 s. `run.json`'s `binary_sha256` matches.

* System: 93 / 4 / 351 (`ec4ff7b0…`, 78.6 s).
* Slice: 52 / 1 / 347, 4 Pending (`36e52a76…`, 38.9 s).
* Null property: corpus 1,113 / 1,435 / 2,355 / 18 Unsafe; system 84 / 125 /
  245; slice 71 / 103 / 232.

Every figure equals `w48-common-r27.md` ([summary](artifacts/summary-before.json)).

**Member sizes** ([size](artifacts/size-before-corpus.json); leaves /
functions / decided alone). These reproduce the ledger entry:

| member | leaves | functions | decided alone |
| --- | ---: | ---: | ---: |
| `rd_kafka_poll_cb` | 4 | 4 | 0 |
| `rd_kafka_q_serve` | 3 | 3 | 0 |
| `rd_kafka_q_serve_maybe_consume` | 0 | 0 | 0 |
| `rd_kafka_consume_cb` | 1 | 1 | 0 |
| `rd_kafka_background_queue_serve` | 0 | 0 | 0 |
| `rd_kafka_destroy_app` | 2 | 2 | 2 (`rd_kafka_destroy`, `rd_kafka_destroy_flags`) |
| `rd_kafka_conf_new` | 13 | 12 | 0 |
| `rd_kafka_new` | 14 | 14 | 0 |
| `rd_kafka_log0` | 47 | 35 | 3 |
| the four setters Fluent Bit calls | 0 | 0 | 0 |

**Rows** ([rows](artifacts/rows-before-corpus.json)):

* `rd_kafka_poll` and `rd_kafka_queue_poll_callback` are Unknown on one leaf
  each, "indirect memory access is outside the resource projection" (the
  `rk->rk_rep` / `rkqu->rkqu_q` actual).
* `rd_kafka_poll_cb` is Unknown on 67 leaves.
* `rd_kafka_q_wait_result` refuses first at the uncontracted
  `rd_kafka_q_pop`, so C04.18's premise there rests on nothing yet.

**The run holds none of Fluent Bit's registrars.** The corpus's 185 units
are librdkafka 94, cmetrics 31, monkey-libevent 29, ctraces 17, chunkio 13
and `flb_input_chunk.c`. The system and tail-pause selections name no Kafka
unit. So `plugins/out_kafka/*`, `plugins/in_kafka/*`, `src/flb_kafka.c` and
`src/aws/flb_aws_msk_iam.c` are in no route, and neither are the three
Fluent Bit candidates. A closure "over the run" in the sense of rule 12n
would therefore not see Fluent Bit's registrations at all (section 3, Q8).

## 2. What the current rules give

1. **Every rk_conf member is refused, fail-closed.**
   * Under rule 12n the closure of `rd_kafka_conf_s.dr_msg_cb` is empty: the
     property-table writer is invisible, so no store names a function.
   * `rd_kafka_s.rk_conf.log_cb` holds only `rd_kafka_set_logger`'s formal,
     and that function has no caller.
   * An empty set is refused (`callback-member-open`).
2. **A unit decides only over its own facts** (rule 12n:
   `member_candidates` reads `closure(&[unit.member_facts()], …)`). The
   run's fold only demotes. Fluent Bit's registrations sit in
   `kafka_config.c` and `flb_aws_msk_iam.c`; the invocations are declared on
   librdkafka functions called from `rdkafka.c`, `rdkafka_queue.c` and
   `rdkafka_background.c`. No unit holds both, so even a perfect run closure
   decides nothing.
3. **Rule 12m refuses `rd_kafka_poll_cb` as a `supplied_owned` candidate,
   whatever the rk_conf reading is.** The poll callback keeps the op on
   `PASS`. The user's 2026-10-14 decision says: "A callback that takes the
   object only on some return codes is refused". So every served-queue call
   whose candidates include `rd_kafka_poll_cb` refuses, including the
   `rko_serve` members of `rd_kafka_q_serve*` and `rd_kafka_q_pop_serve*`.
   * C04.13 lets a return-code disposition stand only beside plain
     predicates and kinds. The loader also rejects every callback or
     retention clause on a relational declaration
     (`contract_loader.rs`: "a retention clause stands only beside plain
     parameter and result predicates"). So the poll callback's split (taken
     on `HANDLED`/`KEEP`/`YIELD`, kept on `PASS`) cannot stand beside its
     member invocations.
   * `rd_kafka_q_serve0` asserts against `PASS`
     (`rdkafka_queue.c:630`, `rd_kafka_assert(NULL, res != RD_KAFKA_OP_RES_PASS)`).
   * `rd_kafka_q_pop_serve0` returns the `PASS` op as its result.
4. **Two key gaps in rule 12n's closure.** The member key is the outermost
   record.
   * `rk->rk_conf.dr_msg_cb` is keyed `rd_kafka_s.rk_conf.dr_msg_cb`, while
     `conf->dr_msg_cb` is keyed `rd_kafka_conf_s.dr_msg_cb`. The struct copy
     `rk->rk_conf = *conf` is read as "values already counted", which holds
     only if the two keys are one. This is follow-up
     `CALLBACK-MEMBER-EMBEDDED-RECORD-KEYS`.
   * The members of a union are keyed apart.
     `rd_kafka_interceptor_method_s.u.generic = func` (`rdkafka_interceptor.c:116`)
     and the read `u.on_destroy` (`:412`) are different keys.
   * Today no declaration invokes such a member. Both gaps must close before
     any declaration does.
5. **Rule 14e's `absent` admits only `absent`** on the right of `|->`, by the
   user's decision ("Admit only absent as the right-hand side for now"). So
   `rd_kafka_conf_new`'s default `log_cb = rd_kafka_log_print` has no
   spelling.

## 3. Proposals awaiting the user's choice

**Q1 (new value form): a member default that names a function.** The decision
says "Any default callback is a candidate too". `log_cb`, `socket_cb` and
`open_cb` start as `rd_kafka_log_print`, `rd_kafka_socket_cb_linux` and
`rd_kafka_open_cb_linux`.

* **D1, maps-to with a designator (recommended):**

  ```qcore
  extern fn rd_kafka_conf_new() -> ptr<mut, u8>
  contract {
      requires resource_contract_v1;
      quant live #result: rd_kafka_conf_s.log_cb |-> rd_kafka_log_print;
      quant live #result: rd_kafka_conf_s.socket_cb |-> rd_kafka_socket_cb_linux;
      quant live #result: rd_kafka_conf_s.open_cb |-> rd_kafka_open_cb_linux;
      quant live #result: rd_kafka_conf_s.dr_msg_cb |-> absent;   // and every other member
      ensures normal resource_return_owned(nonnull);
  };
  ```

  The right of `|->` is `absent` or a function designator that a provider
  declares. The designator joins the member's run set as a store. Rule 14e's
  body check cannot read a table-driven store, so this is a declared
  assumption with the body read.
* **D2:** B1's word with a designator instead of a parameter, for example
  `ensures normal resource_callback_member_registered(result,
  rd_kafka_conf_s.log_cb, rd_kafka_log_print);`.
* **D3, no new form:** a member with a `pdef` default stays open. `log_cb`
  then never decides, and neither does `rd_kafka_poll_cb`'s log arm.

**Q2 (brief-given form; confirm its meaning): a setter's registration into a
parameter holder.**

```qcore
extern fn rd_kafka_conf_set_dr_msg_cb(conf: ptr<mut, u8>, dr_msg_cb: fnptr) -> unit
contract {
    requires resource_contract_v1;
    requires resource_premise(conf_property_writer, "rd_kafka_anyconf_set_prop0's _RK_C_PTR store writes a callback member only from a dedicated setter, a pdef default, rd_kafka_anyconf_copy of the same property, or the string API for _RK_HIDDEN properties");
    ensures normal resource_borrowed(conf);
    ensures normal resource_borrowed(dr_msg_cb);
    ensures normal resource_callback_member_registered(conf, rd_kafka_conf_s.dr_msg_cb, dr_msg_cb);
    ensures normal resource_return_none();
};
```

* **Meaning.** Every call that any unit of the run makes to the setter is a
  store into `rd_kafka_conf_s.dr_msg_cb` of its actual, read like a store of
  a formal. Rule 11p resolves the actual: a designator joins the set, a null
  constant adds nothing, and a formal of a function whose address is never
  taken stands for its callers' actuals. Anything else opens the set. An
  address-taken setter opens the set. On a tracked `conf` the call also
  records rule 13w's path state.
* **Evidence.** Rule 13y cannot validate the body (it stores through the
  table), so the clause is a declared assumption. The premise (C04.18,
  existing surface) names the writer classification of 1.2.1 that the
  reading rests on.
* **Placement.** `resource_callback_member_registered` today admits only
  `result`. This widens it to a depth-one pointer parameter.

**Q3 (no new word; confirm): the copy into `rk->rk_conf`.** A member key is
the **innermost named record**. `rk->rk_conf.dr_msg_cb`,
`conf->dr_msg_cb` and the setter's registration are then all
`rd_kafka_conf_s.dr_msg_cb`, and `rk->rk_conf = *conf` is a same-record copy.
A declaration may spell the exact path, `rd_kafka_s.rk_conf.dr_msg_cb`; the
consumer normalises it through the unit's records. A union's function-pointer
members share their stores. This closes the two gaps in section 2.4.

**Q4 (product decision): the plugin loader (1.2.4).**

* **U1, strict reading of "any unseen store opens the set":**
  `rd_kafka_plugin_new`'s call of a `rd_dl_sym` result with the conf opens
  every rk_conf member and every interceptor list. No rk_conf member decides
  in this corpus.
* **U2:** a named premise beside `P-CG22-MEMBER-CLOSED-WORLD`, for example
  `P-RK-NO-PLUGIN-LIBRARY` ("`plugin.library.paths` is not configured"). It
  is published with every row that rests on an rk_conf member invocation. The
  set closes over the run's registrations: Fluent Bit's, plus librdkafka's
  own on `&rk->rk_conf`.

**Q5 (rule change): deciding a unit over the run's set (section 2.2).**

* **R1:** a facts pre-pass reads every unit's member facts (one Clang AST
  read per unit, as `callback_member_closure::read_unit` already does for
  data-only units). A unit then decides a member invocation over the run's
  closed set. The fold keeps its check (run set equals the set the decision
  used) and still never promotes. The cost is roughly one extra AST read per
  corpus unit of wall time.
* **R2:** leave 12n unit-local. The rk_conf members decide only in a unit
  that also holds their registrations, which is none in this corpus.

**Q6 (rule change, possibly a new word): a closed empty run set.** A member
that the run closes with no candidate (for example `error_cb`, `rebalance_cb`
or `background_event_cb`: no registration, no default) holds a null pointer,
and every declared invocation site tests it first. The decision ("candidates
are exactly the callbacks FB stores") reads that as no candidate. Rule 12n
refuses an empty set.

* **E1 (no word):** a run-closed empty set decides as "invokes nothing". The
  guard is part of the declaration's body reading (a declared assumption).
* **E2 (new word):** the same, but only where the clause says the invocation
  is guarded, for example `resource_callback_member_invoked(rk,
  rd_kafka_conf_s.error_cb, guarded, rk, _, _, _)`.
* **E3:** keep the refusal. `rd_kafka_poll_cb`'s contract then never
  decides, because seven of its members are empty for Fluent Bit.

**Q7 (rule change, user's 12m decision): `rd_kafka_poll_cb` at the serves
(section 2.3).**

* **S1:** keep 12m. `rd_kafka_poll_cb` stays refused as a `supplied_owned`
  candidate. Every served-queue call through it keeps refusing
  (`callback-supplied-kept` instead of `callback-uncontracted`).
* **S2:** admit a candidate that keeps the object only on a return the serve
  asserts against. This needs two things:
  * a serve clause naming the code, `rd_kafka_q_serve0`'s `PASS`;
  * a disposition beside callback clauses, so the poll callback's split can
    be stated.

  Both are new surface. They overlap the op callback protocol (`HANDLED`,
  `KEEP`, `YIELD`) that OP-CALLBACK-CANDIDATE-CONTRACTS must state for
  `rko_op_cb`'s candidates.

**Q8 (product decision): which units the closure ranges over (1.5).** The
decision says "closed run-wide", but the run holds no Fluent Bit Kafka unit.
Rule 12n's premise `P-CG22-MEMBER-CLOSED-WORLD` ("every unit that can store
into the member is in the run") fails for every rk_conf member in every
route.

* **F1:** read the member facts and setter calls of every unit of Fluent
  Bit's production compile database, facts only, as rule 12n already reads a
  data-only unit (`read_unit`). The closed world is then the program the
  corpus is cut from. The cost is one Clang AST read per production unit,
  roughly the system route's unit count again or more.
* **F2:** read the facts of the units that call a declared registering
  setter. Choosing those units by text is not sound by itself, so this needs
  a premise naming the selection.
* **F3:** leave the run as it is. Every rk_conf member refuses as "a
  registering unit is outside the run", which is fail-closed and yields
  nothing.

## 4. Prediction

| choice | rows predicted to move | Safe gain | C04.18 on `rd_kafka_q_wait_result` |
| --- | --- | --- | --- |
| any choice with U1, R2 or F3 | leaves only: the setter, constructor and invocation calls trade `call to uncontracted function` for `callback-member-open` | **0** | unchanged |
| U2 + R1 + F1 + D1 + Q3 + E1 + S1 | leaves only; the rk_conf members close (`dr_msg_cb` {`cb_kafka_msg`}, `log_cb` {`cb_kafka_logger`, `rd_kafka_log_print`}, `token_refresh_cb` {FB's, librdkafka's five}); `rd_kafka_poll_cb` stays refused by 12m at every serve | **0** | unchanged: the poll callback is refused, and `rko_serve`'s other three and `rko_op_cb`'s 27 candidates belong to OP-CALLBACK-CANDIDATE-CONTRACTS |
| U2 + R1 + F1 + D1 + Q3 + E1/E2 + S2 | the served-queue calls through `rd_kafka_poll_cb` decide only if `rd_kafka_op_call`'s `rko_op_cb` (27, sibling card), the op members `rko_u.offset_commit.cb` / `terminated.cb` and the share ack member also close | **0** expected in this card | the poll callback's share of the premise is discharged by reading (it never keeps `rkq`; `rd_kafka_q_reenq` takes no reference); the rest waits for the sibling card |

The first served-queue caller in each family also refuses before any callback
is read. `rd_kafka_poll` hands `rk->rk_rep`, and the access gate refuses that
member actual (the CG22 report counts 14 such refusals at `rk->rk_rep`). So
even a decided poll callback predicts no Safe row among the callers the
ledger entry sizes. **Predicted under every option: no Safe → non-Safe move, no
Unknown → Safe move, null property unchanged.** The card's yield is
vocabulary and closure correctness, not verdicts.

**Recommendation:** D1, Q2 as written, Q3, U2, R1, F1, E1, S1. That realises the
decision's closure exactly, keeps 12m as the user decided it, and leaves the
served-queue yield to the op callback protocol card. Fluent Bit's three
candidates are in no analysed unit, so each contract (in
`quirin-provider-fluent-bit`) is a declared assumption with its body read at
both revisions. Rule 14f cannot validate them in this run. `rd_kafka_log_print`
and librdkafka's OAUTHBEARER refreshers are in `rdkafka.c` and the SASL units,
where rule 14f checks them. `rd_kafka_destroy_app` is then expressible only up to its interceptors: under
U2 they are empty, which the run closes as "invokes nothing" under E1. Its
release of `rk` without `RD_KAFKA_DESTROY_F_IMMEDIATE` is C04.5's literal
premise `flags = 0` on `rd_kafka_destroy`, which is existing vocabulary. Its
consumer close stays refused through the poll callback (S1). It is named as a
residual.
