# CONTRACT-ADOPTION-NOT-IN-FORCE

Card brief: `private/integrator-briefs/r13-common.md` (round 13, 2026-11-05),
card 1. Base `c8c7edf21`. The card is new. It is registered and retired
through its [amendments](integration-amendments.json).

> **USER REQUEST 2026-11-05** (verbatim): "Now, we're still needed to fill the
> gap of the missing contracts." Then, on the integrator's two cards:
> "Launch 1 and 2".

**Question.** The user's `quirin-audit current` run at Fluent Bit `046accf6`
reports 107 callees as "call to uncontracted function", 473 blocked
(function, callee) pairs, although a provider module has an `extern fn`
declaration for each. Why is no declaration in force, and which can be brought
into force mechanically?

**Answer.** None of the 107 can be brought into force mechanically. No
declaration is missing from a module list, a view or the route's manifest.
None is unpinned, and the census found no name or lookup defect. Every
callee falls into one of three classes:

- **A, 66 callees, 290 pairs.** The only declaration is a bounds-v1
  (null-deref) contract. Such a contract states no resource effect.
- **B, 39 callees, 180 pairs.** The only declarations are declared models or
  typed-language contracts, which the resource loader refuses by vocabulary.
- **C, 2 callees, 3 pairs.** A resource-v1 declaration is in force, but its
  calling unit withdraws it, as the contract specification requires.

Each class needs new contract text or a new rule, so step 2 adopts nothing.
§4 gives the owners. §5 gives the recommendation.

## 1. Inputs and method

- **Run report.** The main checkout's
  `build/cycle19/clang/046accf6aded7906f66a1baf0202da349e521736/fluent-bit-clang-source/reports/resource-diagnostics.json`
  (SHA-256 `e965950e…d017f`; 186 configurations; evidence version
  `contract-hypothesis-graph.v1:c8c7edf21…`). Its contract manifest digest
  `2226e66b…17a0` equals this base's `cycle19-resource-contracts.json`.
- **Declarations.** Every tracked `providers/**/contracts/**/*.qv`, read
  through `git ls-files`. The SDK's example fixture crate is excluded.
- **Views.** The resource loader's own probe, run at the base:
  `target/debug/examples/provider_resource_views providers/contract-load-path.json`,
  saved as [census/probe.json](census/probe.json) with the worktree path
  replaced by `<repository>`. The probe prints each refused module's reason.
- **Census.** [census/census.py](census/census.py) writes
  [census/census.json](census/census.json) (every row with its callers,
  declarations, views, selectors, pins and the calling units' body evidence)
  and [census/census.md](census/census.md) (the table in §3). To rerun it,
  from the repository root:

```sh
python3 docs/reports/contract-adoption-not-in-force/census/census.py \
  --report <main checkout>/build/cycle19/clang/046accf6aded7906f66a1baf0202da349e521736/fluent-bit-clang-source/reports/resource-diagnostics.json \
  --probe docs/reports/contract-adoption-not-in-force/census/probe.json \
  --out docs/reports/contract-adoption-not-in-force/census
```

The census recomputes the integrator's figures exactly: 2,790 uncontracted
callees and 7,264 pairs, of which 107 callees and 473 pairs are declared
somewhere.

## 2. Why none is mechanical

Each reason class the brief names, checked:

| Reason (brief) | Finding |
|---|---|
| Not in the crate's load path or module list | Every declaring module is in its crate's `modules` in `providers/contract-load-path.json`. |
| Not in the profile's interpreted view | True for classes A and B, by vocabulary rather than by omission. The probe's three views equal the committed views for every crate. The route's manifest equals the union of the resource views. **No tracked module outside a resource view contains a `requires resource_contract_v1;` declaration** (whole tree, not only the 107). The corpus test `every_corpus_module_outside_the_v1_vocabulary_is_rejected_not_skipped` (`compiler/tests/contract_loader.rs`) already pins both halves: a module loads exactly when every declaration carries the selector, and the manifest's four tables (functions, pending review, relational, retaining) hold every v1 declaration of the corpus. A view or load-path fix would therefore have nothing to admit. |
| Unpinned or pin mismatch | None. Each of the 68 selector-carrying declarations (66 bounds, 2 resource) has a pin reviewed at `046accf6` and `66910c10` in its crate's `bounds-contracts.lock` or `contracts.lock`. The 39 class-B declarations have no selector, so no pin lock lists them. |
| Held: human review | None. A held callee gets a hold diagnostic, not "uncontracted" (`legacy_classify.rs`, the seven call holds answered first). |
| Held: refuted by body | None in the C06.4 sense. `cJSON_ParseWithLengthOpts` is rule 13a's withdrawal (class C). |
| Held: withdrawn C06.3-no-body | `sb_release_output_queue_space`, class C. Its caller is `src/flb_input_chunk.c`. Its body, `plugins/in_storage_backlog/sb.c`, is in none of the run's 186 configurations. C06.3 rule 1 withdraws a checked specification from every unit that does not define the body. |
| Name / arity / type mismatch | None reached. Class A and B names have no resource declaration to match. The two class C declarations are withdrawn for body reasons, not by the ABI gate. |
| Lookup bug | None. All 107 callees are explained by vocabulary or by the specification's withdrawal, and every resource-v1 declaration in the tree is in force on this route. |

**Why classes A and B are not adoptable.** The loader's module documentation
(`crates/qcore-contract/src/contract_loader.rs`, "Interpreter selection")
fixes three disjoint vocabularies. "A resource projection of a lifecycle module
is written as its own v1 module ... it is never inferred by dropping predicates
this loader cannot check." So these declarations cannot be adopted. A resource
contract for any of these callees is a new declaration, read from its body at
the pinned revisions, pinned and measured. That is authoring, not adoption,
and the brief's step 2 excludes it.

- A bounds-v1 contract (class A) states null-dereference obligations only.
  Reading it as a resource summary would invent the callee's custody effect
  (for `rd_kafka_broker_destroy_final`, a release).
- The class B modules say outright that they grant no interpreted call
  permission. For example `cfl/intrusive_list.qv`: "This is not
  resource_contract_v1 and grants no ordinary call summary". The cmetrics
  models carry similar text, and `monkey/wrapper_paths.qv` is "intentionally
  outside cycle19-resource-contracts.json".

**Class C stays held.**

- `sb_release_output_queue_space` is `resource_body_checked` (C06.3). A
  specification is never applied in a unit the audit did not analyse.
  Separately, its body does not establish it, and W45-FB-BACKLOG-REACHABLE-RELEASE
  owns that.
- `cJSON_ParseWithLengthOpts` is a derived-out and retained declaration. In
  `cJSON.c`, its only calling unit, rule 13a does not read the slot's two
  stores, so the declaration stands nowhere. CLASS1-UNDECLARABLE-RETAINERS
  owns the missing slot vocabulary.

**Ceiling.** Suppose all 107 callees were in force and nothing else surfaced.
Then 20 of the run's 508 "only uncontracted" functions would have no
uncontracted call left:

- nine Monkey `mk_event_*` wrappers, each blocked only by its `_mk_event_*`
  backend (class B);
- `cmt_expire` and `cmt_map_metric_get_val` (class B);
- `cio_stream_destroy_all` (class B);
- the two `cJSON_Parse*` wrappers (class C);
- `write_init_header` (class A, the bounds-only `cio_file_st_set_content_len`);
- five librdkafka functions (class A).

## 3. Census table

Columns:

- **Pairs.** Distinct (function, callee) pairs in the run.
- **Class.** A, B or C, as in the summary above.
- **Refusal.** For classes A and B, the module's first resource-loader
  refusal, from the probe; no declaration in either class carries the
  selector. For class C, the calling unit's `contract_body_evidence`.
- **Owner.** The ledger entry that tracks the callee's resource side (§4).

| # | Callee | Pairs | Class | Declaring module | Module's first resource-loader refusal, or the withdrawal | Owner |
|---|---|---|---|---|---|---|
| 1 | `rd_kafka_broker_destroy_final` | 51 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 2 | `rd_kafka_op_throttle_time` | 46 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 3 | `cfl_list_add` | 43 | B | `cfl/intrusive_list.qv` | resource modules accept only extern functions | `W45-SUB-OBJECT-STORE-FORMAL-VALUE` |
| 4 | `rd_kafka_topic_partition_list_add` | 35 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 5 | `rd_kafka_buf_new_request0` | 30 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 6 | `cfl_list_del` | 24 | B | `cfl/intrusive_list.qv` | resource modules accept only extern functions | `W45-SUB-OBJECT-STORE-FORMAL-VALUE` |
| 7 | `cio_error_reset` | 18 | B | `chunkio/error_frame.qv`; `chunkio/error_state.qv` | unsupported parameter type or usage annotation | `W44-CALLEE-CROSS-UNIT-SUMMARY` |
| 8 | `rd_kafka_topic_name` | 16 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 9 | `cmt_map_metric_get` | 14 | B | `cmetrics/map_lifetime.qv` | resource modules accept only extern functions | `CMT-MAP-OPTS-CONTRACT-ADOPTION` |
| 10 | `rd_kafka_destroy` | 14 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 11 | `rd_kafka_broker_set_state` | 10 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 12 | `cmt_map_create` | 8 | B | `cmetrics/map_lifetime.qv` | resource modules accept only extern functions | `CMT-MAP-OPTS-CONTRACT-ADOPTION` |
| 13 | `cmt_map_destroy` | 8 | B | `cmetrics/map_lifetime.qv` | resource modules accept only extern functions | `CMT-MAP-OPTS-CONTRACT-ADOPTION` |
| 14 | `cmt_opts_exit` | 6 | B | `cmetrics/opts_lifetime.qv` | resource modules accept only extern functions | `CMT-MAP-OPTS-CONTRACT-ADOPTION` |
| 15 | `cmt_opts_init` | 6 | B | `cmetrics/opts_lifetime.qv` | resource modules accept only extern functions | `CMT-MAP-OPTS-CONTRACT-ADOPTION` |
| 16 | `rd_kafka_q_deq0` | 6 | B | `librdkafka/typed_queue.qv` | resource modules accept only extern functions | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 17 | `cmt_metric_set` | 5 | B | `cmetrics/metric_setters.qv` | unsupported error, any-exit, or quantitative clause | `CMT-MAP-OPTS-CONTRACT-ADOPTION` |
| 18 | `rd_kafka_topic_new0` | 5 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 19 | `rd_kafka_broker_connect_auth` | 4 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 20 | `cmt_map_metric_get_val` | 3 | B | `cmetrics/map_lifetime.qv` | resource modules accept only extern functions | `CMT-MAP-OPTS-CONTRACT-ADOPTION` |
| 21 | `cmt_metric_hist_count_set` | 3 | B | `cmetrics/metric_setters.qv` | unsupported error, any-exit, or quantitative clause | `CMT-MAP-OPTS-CONTRACT-ADOPTION` |
| 22 | `cmt_metric_hist_sum_set` | 3 | B | `cmetrics/metric_setters.qv` | unsupported error, any-exit, or quantitative clause | `CMT-MAP-OPTS-CONTRACT-ADOPTION` |
| 23 | `rd_kafka_broker_buf_enq0` | 3 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 24 | `rd_kafka_broker_retry_bufs_move` | 3 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 25 | `rd_kafka_broker_set_nodename` | 3 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 26 | `rd_kafka_broker_share_fetch_session_clear` | 3 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 27 | `_mk_event_loop_destroy` | 2 | B | `monkey/wait_source_policy.qv`; `monkey/wrapper_paths.qv` | unsupported parameter type or usage annotation; unsupported predicate `source_revision == "66910c10a4d7eafa810b84229cf2dcf7b0f26f97"` | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 28 | `_mk_event_wait_2` | 2 | B | `monkey/wait_source_policy.qv`; `monkey/wrapper_paths.qv` | unsupported parameter type or usage annotation; unsupported predicate `source_revision == "66910c10a4d7eafa810b84229cf2dcf7b0f26f97"` | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 29 | `cJSON_ParseWithLengthOpts` | 2 | C | `librdkafka/cjson.qv` | failed:13a, withdrawn:13a | `CLASS1-UNDECLARABLE-RETAINERS` |
| 30 | `cfl_list_init` | 2 | B | `cfl/intrusive_list.qv` | resource modules accept only extern functions | `W45-SUB-OBJECT-STORE-FORMAL-VALUE` |
| 31 | `cio_file_st_set_content_len` | 2 | A | `chunkio/file_st_bounds.qv` | unsupported or duplicate requirement `bounds_contract_v1` | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 32 | `cmt_map_metric_destroy` | 2 | B | `cmetrics/map_lifetime.qv` | resource modules accept only extern functions | `CMT-MAP-OPTS-CONTRACT-ADOPTION` |
| 33 | `cmt_metric_hist_set` | 2 | B | `cmetrics/metric_setters.qv` | unsupported error, any-exit, or quantitative clause | `CMT-MAP-OPTS-CONTRACT-ADOPTION` |
| 34 | `cmt_metric_set_exp_hist_count` | 2 | B | `cmetrics/metric_setters.qv` | unsupported error, any-exit, or quantitative clause | `CMT-MAP-OPTS-CONTRACT-ADOPTION` |
| 35 | `cmt_metric_set_exp_hist_sum` | 2 | B | `cmetrics/metric_setters.qv` | unsupported error, any-exit, or quantitative clause | `CMT-MAP-OPTS-CONTRACT-ADOPTION` |
| 36 | `rd_kafka_ShareAcknowledgeRequest` | 2 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 37 | `rd_kafka_broker_active_toppar_del` | 2 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 38 | `rd_kafka_broker_buf_retry` | 2 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 39 | `rd_kafka_broker_conn_closed` | 2 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 40 | `rd_kafka_broker_connect_up` | 2 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 41 | `rd_kafka_broker_session_reset` | 2 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 42 | `rd_kafka_broker_set_api_versions` | 2 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 43 | `rd_kafka_broker_toppar_msgq_scan` | 2 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 44 | `rd_kafka_broker_wakeup` | 2 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 45 | `rd_kafka_idemp_request_pid_failed` | 2 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 46 | `rd_kafka_queue_destroy` | 2 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 47 | `rd_kafka_ssl_hmac` | 2 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 48 | `rd_kafka_transport_new` | 2 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 49 | `_mk_event_add` | 1 | B | `monkey/wrapper_paths.qv` | unsupported parameter type or usage annotation | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 50 | `_mk_event_channel_create` | 1 | B | `monkey/wrapper_paths.qv` | unsupported parameter type or usage annotation | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 51 | `_mk_event_channel_destroy` | 1 | B | `monkey/wrapper_paths.qv` | unsupported parameter type or usage annotation | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 52 | `_mk_event_del` | 1 | B | `monkey/wrapper_paths.qv` | unsupported parameter type or usage annotation | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 53 | `_mk_event_inject` | 1 | B | `monkey/wrapper_paths.qv` | unsupported parameter type or usage annotation | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 54 | `_mk_event_loop_create` | 1 | B | `monkey/wait_source_policy.qv`; `monkey/wrapper_paths.qv` | unsupported parameter type or usage annotation; unsupported predicate `source_revision == "66910c10a4d7eafa810b84229cf2dcf7b0f26f97"` | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 55 | `_mk_event_timeout_create` | 1 | B | `monkey/wrapper_paths.qv` | unsupported parameter type or usage annotation | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 56 | `_mk_event_timeout_destroy` | 1 | B | `monkey/wrapper_paths.qv` | unsupported parameter type or usage annotation | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 57 | `cfl_arena_create_with_options` | 1 | B | `cfl/arena_lifetime.qv` | resource modules accept only extern functions | `W47-ARENA-POOL-RESULT-MODE` |
| 58 | `cfl_arena_destroy` | 1 | B | `cfl/arena_lifetime.qv` | resource modules accept only extern functions | `W47-ARENA-POOL-RESULT-MODE` |
| 59 | `cio_chunk_counter_total_up_add` | 1 | B | `chunkio/counters.qv` | unsupported parameter type or usage annotation | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 60 | `cio_chunk_counter_total_up_sub` | 1 | B | `chunkio/counters.qv` | unsupported parameter type or usage annotation | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 61 | `cio_destroy` | 1 | B | `chunkio/context_destructor.qv` | resource modules accept only extern functions | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 62 | `cio_memfs_close` | 1 | B | `chunkio/memory_backend.qv` | unsupported parameter type or usage annotation | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 63 | `cio_stream_destroy` | 1 | B | `chunkio/stream_destructor.qv` | resource modules accept only extern functions | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 64 | `cmt_map_metrics_expire` | 1 | B | `cmetrics/map_lifetime.qv` | resource modules accept only extern functions | `CMT-MAP-OPTS-CONTRACT-ADOPTION` |
| 65 | `cmt_summary_count_set` | 1 | B | `cmetrics/metric_setters.qv` | unsupported error, any-exit, or quantitative clause | `CMT-MAP-OPTS-CONTRACT-ADOPTION` |
| 66 | `cmt_summary_quantile_set` | 1 | B | `cmetrics/metric_setters.qv` | unsupported error, any-exit, or quantitative clause | `CMT-MAP-OPTS-CONTRACT-ADOPTION` |
| 67 | `cmt_summary_sum_set` | 1 | B | `cmetrics/metric_setters.qv` | unsupported error, any-exit, or quantitative clause | `CMT-MAP-OPTS-CONTRACT-ADOPTION` |
| 68 | `flb_input_pause` | 1 | A | `fluent_bit/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 69 | `flb_input_resume` | 1 | A | `fluent_bit/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 70 | `rd_kafka_broker_active_toppar_add` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 71 | `rd_kafka_broker_add` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 72 | `rd_kafka_broker_connect` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 73 | `rd_kafka_broker_connect_done` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 74 | `rd_kafka_broker_consumer_serve` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 75 | `rd_kafka_broker_feature_disable` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 76 | `rd_kafka_broker_feature_enable` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 77 | `rd_kafka_broker_features_set` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 78 | `rd_kafka_broker_fetch_backoff` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 79 | `rd_kafka_broker_fetch_toppars` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 80 | `rd_kafka_broker_handle_purge_queues` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 81 | `rd_kafka_broker_idle_check` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 82 | `rd_kafka_broker_internal_serve` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 83 | `rd_kafka_broker_produce_toppars` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 84 | `rd_kafka_broker_serve` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 85 | `rd_kafka_broker_session_add_partition_to_toppars_in_session` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 86 | `rd_kafka_broker_session_remove_partition_from_toppars_in_session` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 87 | `rd_kafka_broker_session_update_epoch` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 88 | `rd_kafka_broker_session_update_partitions` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 89 | `rd_kafka_broker_share_fetch_session_leave` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 90 | `rd_kafka_broker_share_rpc` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 91 | `rd_kafka_broker_share_session_toppar_add` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 92 | `rd_kafka_broker_timeout_scan` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 93 | `rd_kafka_buf_retry` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 94 | `rd_kafka_bufq_purge` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 95 | `rd_kafka_dr_implicit_ack` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 96 | `rd_kafka_features_check` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 97 | `rd_kafka_fetch_reply_handle_partition_error` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 98 | `rd_kafka_group_MemberMetadata_consumer_read` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 99 | `rd_kafka_handle_Produce_metadata_update` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 100 | `rd_kafka_message_destroy` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 101 | `rd_kafka_msgbatch_handle_Produce_result` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 102 | `rd_kafka_outq_len` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 103 | `rd_kafka_req_response` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 104 | `rd_kafka_sasl_broker_init` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 105 | `rd_kafka_sasl_broker_term` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 106 | `rd_kafkap_share_leader_changes_apply` | 1 | A | `librdkafka/null_deref.qv` | unsupported error, any-exit, or quantitative clause | `DECLARED-NAMES-RESOURCE-V1-PROJECTIONS` |
| 107 | `sb_release_output_queue_space` | 1 | C | `fluent_bit/storage_backlog.qv` | withdrawn:C06.3-no-body | `W45-FB-BACKLOG-REACHABLE-RELEASE` |

By module:

| Declaring module(s) | Class | Callees | Pairs | Owner |
|---|---|---|---|---|
| `librdkafka/null_deref.qv` | A | 63 | 286 | DECLARED-NAMES-RESOURCE-V1-PROJECTIONS (new) |
| `cfl/intrusive_list.qv` | B | 3 | 69 | W45-SUB-OBJECT-STORE-FORMAL-VALUE (amended) |
| `cmetrics/map_lifetime.qv` | B | 6 | 36 | CMT-MAP-OPTS-CONTRACT-ADOPTION (amended) |
| `cmetrics/metric_setters.qv` | B | 9 | 20 | CMT-MAP-OPTS-CONTRACT-ADOPTION (amended) |
| `chunkio/error_frame.qv` + `error_state.qv` | B | 1 | 18 | W44-CALLEE-CROSS-UNIT-SUMMARY (already names `cio_error_reset`, 18 leaves) |
| `cmetrics/opts_lifetime.qv` | B | 2 | 12 | CMT-MAP-OPTS-CONTRACT-ADOPTION (amended) |
| `monkey/wrapper_paths.qv` (3 also in `wait_source_policy.qv`) | B | 10 | 12 | DECLARED-NAMES-RESOURCE-V1-PROJECTIONS (new) |
| `librdkafka/typed_queue.qv` | B | 1 | 6 | DECLARED-NAMES-RESOURCE-V1-PROJECTIONS (new) |
| `librdkafka/cjson.qv` | C | 1 | 2 | CLASS1-UNDECLARABLE-RETAINERS |
| `chunkio/file_st_bounds.qv` | A | 1 | 2 | DECLARED-NAMES-RESOURCE-V1-PROJECTIONS (new) |
| `cfl/arena_lifetime.qv` | B | 2 | 2 | W47-ARENA-POOL-RESULT-MODE (already names `cfl_arena_destroy`) |
| `chunkio/counters.qv` | B | 2 | 2 | DECLARED-NAMES-RESOURCE-V1-PROJECTIONS (new) |
| `fluent_bit/null_deref.qv` | A | 2 | 2 | DECLARED-NAMES-RESOURCE-V1-PROJECTIONS (new) |
| `chunkio/context_destructor.qv`, `memory_backend.qv`, `stream_destructor.qv` | B | 3 | 3 | DECLARED-NAMES-RESOURCE-V1-PROJECTIONS (new) |
| `fluent_bit/storage_backlog.qv` | C | 1 | 1 | W45-FB-BACKLOG-REACHABLE-RELEASE |

## 4. Step 2: adoption, and the records

**Adopted: none.** Nothing changed in a module list, a view,
`contract-load-path.json`, a lock or the route's lookup, so `providers/contracts.py
sync` has nothing to regenerate.

The named owners were read. None is duplicated. The amendments do this:

- **CMT-MAP-OPTS-CONTRACT-ADOPTION, amended.** It gets the measured cost at
  `046accf6`: 17 map, opts and setter callees, 68 blocked pairs. Its
  discharge is unchanged. These declared models stay outside every
  interpreted view.
- **W45-SUB-OBJECT-STORE-FORMAL-VALUE, amended.** It now names the three CFL
  list callees and their 69 pairs. `cfl_list_add` needs the derived Stored
  mode this entry already tracks. `cfl_list_del` needs its inverse, the
  removal from a derived container. `cfl_list_init` is the self store that
  rule 10v admits with the summary withheld. `cfl/intrusive_list.qv` is
  rule 36b's declared model and grants no summary.
- **DECLARED-NAMES-RESOURCE-V1-PROJECTIONS, registered.** It covers 82
  callees and 313 pairs: class A and the 16 unowned class B callees. A
  resource-v1 projection of each must be written as its own module, stated
  from the body at `66910c10`, `34155955` and `046accf6`, pinned through
  `quirin-audit pin` and measured. It is never derived from the bounds or
  model text.
- **Unchanged.** W44-CALLEE-CROSS-UNIT-SUMMARY, W47-ARENA-POOL-RESULT-MODE,
  W45-FB-BACKLOG-REACHABLE-RELEASE and CLASS1-UNDECLARABLE-RETAINERS already
  name their callees and discharge conditions.
- **This card's id.** Registered and retired.

## 5. Recommendation

The 473 pairs will not close by adoption. They close by new contract text
plus the vocabulary and route work their owners name. By pairs, the next
lever is the librdkafka broker family in class A: 286 pairs, though only
five functions are freed alone. By freed functions, it is the nine Monkey
`_mk_event_*` backends: nine functions, but `wrapper_paths.qv` records
their failure paths as findings, so each needs a truthful projection rather
than a neutral one. Both are in DECLARED-NAMES-RESOURCE-V1-PROJECTIONS. No
semantic choice here needs the user.

## 6. Prediction and gate

[PREDICTION.md](PREDICTION.md) was committed before anything else changed.
It predicts no move on any route. The card gate is recorded in
[gate.json](gate.json).

**Measurement (integrator, 2026-11-05).** Not run. The card changes only
files under `docs/` (the census, this report and its records). It changes no
crate, provider contract, lock, load path, view or input the routes read, so
`base` and `tip` analyse identical trees and the declared prediction (`none`
on every route) holds by construction. Landed on `d0dae815f`.
