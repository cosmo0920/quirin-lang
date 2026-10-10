# W45-CALLEE-CASCADE-RESIDUAL round 3: contracts for the callees that each free two or more

Card W45-CALLEE-CASCADE-RESIDUAL round 3 (wave 48, card-high, 2026-10-11), on
the user's request "Fill the gap of absence of contracts." Base: local `main`
at `ef6b32192`, which it stayed at. Contracts and records only: fourteen
pinned declarations in seven provider crates, the pin tables, no analyzer
code, no gateway rule (the reserved `15v`–`15y` are unused) and no Lean,
because no semantics were added (each declaration is checked by the existing
loader, ABI, pin and body-standing gates). [PREDICTION.md](PREDICTION.md) was
committed at `3eba9abbc`, before any contract was written.

## 1. BEFORE

**Reproduced.** The BEFORE binary (`private/w45r3/bin/before`, sha256
`551fa31b…`, equal to every corpus run's `run.json` `binary_sha256`; the
same binary measured every AFTER, since contracts are read at run time):

| route | totals | `resource-diagnostics.json` |
| --- | --- | --- |
| corpus (`66910c10`) | 915 / 9 / 3,910 | `ded14632…`, the brief's |
| system | 91 / 4 / 353 | `f8aab120…` |
| tail-pause slice | 50 / 1 / 349, 4 Pending | `1654a682…` |
| null property, corpus | 1,113 Safe unconditional / 1,435 conditional / 2,355 Unknown / 18 Unsafe, 29 findings | |

The system and slice digests embed this worktree's paths.

**The population** ([`scripts/rank.py`](scripts/rank.py), CG22's lineage: a
function counts only when every leaf reads ``call to uncontracted function
`F` ``, so a function that needs another fix is not counted): corpus **506**,
**352** decided by one callee alone; system 27 / 19; slice 42 / 28
([corpus](artifacts/rank-before-corpus.json), [system](artifacts/rank-before-system.json),
[slice](artifacts/rank-before-slice.json)). **Correction to the brief's
figures:** the brief cites CG22's 501 / 348. The null cards since moved five
functions into the population; two are `cfl_list_size`'s callers, which put
`cfl_list_size` in the 2-tier. The corpus 2-tier is 34 callees; the slice
adds `file_pending_flush` and `flb_hash_table_del` (2 each).

## 2. Declared, in the brief's order

Every body was read at both revisions with [`scripts/body.py`](scripts/body.py);
every one used here is byte-identical at `66910c10` and `046accf6`. Each
declaration states only what its body does and says what it does not claim
(in the module text). Pinned with `quirin-audit pin` at both trees, then
`pin --check --crate` for each changed crate at both (all 0), `pin
--verify-contracts` 0 ([`scripts/pin.sh`](scripts/pin.sh), logs
`private/w45r3/pin-p{1,2,3}/`). The whole-tree `pin --check` exits 1 at both,
as on the base: its one row not in force is the libevent crate's tip-mode
`mk_vhost_set`. `providers/contracts.py sync` and `check` exit 0 (no resource
view changed; every module was already selected).

| stage | declaration | crate, module | clauses | per-clause standing (corpus run a3) |
| --- | --- | --- | --- | --- |
| a | `rd_kafka_AclBinding_destroy`, `_ConfigResource_destroy`, `_NewPartitions_destroy`, `_NewTopic_destroy` | librdkafka `client.qv` | released nonnull | declared assumption: the bodies are Unknown in `rdkafka_admin.c` (no body evidence), none refuted |
| a | `rd_kafka_UserScramCredentialAlteration_destroy` | same | released nullable (it tests NULL) | declared assumption, as above |
| b | `mk_fifo_queue_destroy` | libevent `monkey/intrusive_list.qv` | `ctx` borrowed, `q` released nonnull | declared assumption (body Unknown in `mk_fifo.c`) |
| c | `unit_test_telemetry` | librdkafka `client.qv` | ten borrowed, `resource_callback_invoked(set_metric_value, _, _)` | partly validated: rule 13y covers the invocation; the borrowed clauses are uncovered |
| c | `cfl_list_size` | cfl `sds_lifetime.qv` | `head` borrowed, none | declared assumption (the header body is not examined as a row) |
| c | `flb_hash_table_del` (slice) | fluent-bit `hash_table_publication.qv` | `ht`, `key` borrowed, none | not in the corpus; system and slice consult it |
| d | `rd_bytes_hash` | librdkafka `kafka_mem.qv` | neutral | **validated** (W4, entry) |
| d | `rd_kafka_mem_calloc` | same | three borrowed, owned nullable | **validated** (W4, entry) |
| d | `cio_file_native_get_size`, `cio_stream_size_chunks_up` | chunkio `chunk_sizes.qv` | neutral | **validated** (W4, entry) |
| d | `cmt_atomic_initialize` | cmetrics `context_lifetime.qv` | neutral | **validated** (W4, entry) |

Stage (a): each destructor releases its object and everything it owns; none
drops a reference or releases a child it does not own. The embedded lists it
destroys own their elements through library free callbacks fixed at
construction (`rd_kafka_ConfigEntry_free`, `rd_list_destroy_free`), and every
string or byte buffer it frees was copied in by its constructor. Stage (d) is
the optional 1-tier, entered only for callees whose own body is Safe in its
home unit (six were found; five are kept, below), and stopped there: the
yield per contract is 1.

**Withdrawn after measurement (two).**

* `cio_file_hash` (1-tier, `resource_return_borrowed_from(cf, nullable)`):
  on run a1 its own row moved **Safe → Unknown**, because the body's W4
  summary is neutral and `contradicts the declared contract` (a
  `SummaryConflict`), and its caller `cio_chunk_hash` did not move (its base
  is the member read `ch->backend`). Withdrawn; the AFTER has no Safe →
  non-Safe move. Registered: `BORROWED-RESULT-SUMMARY-CONFLICT`.
* cJSON's static `print` (2-tier, owned nullable under the
  `cjson_allocator_hooks` premise): the identity reading binds it to cJSON's
  definition alone (at both revisions the one definition named `print` under
  `lib/ src/ plugins/ include/` is `cJSON.c:1234`; the other textual hits are
  comments), and run a1 measured +2 (`cJSON_Print`, `cJSON_PrintUnformatted`).
  But `qcore_contract_corpus`'s compile regression failed on `cjson.qv`:
  `has_print_calls` (`crates/qcore-frontend/src/source_scalar_calls.rs:94-98`)
  treats any `print (` token pair as a `.qr` print call and lowers the whole
  module on the scalar-calls route, which refuses every `extern fn` (Q020).
  Fixing the frontend is outside a contracts card; withdrawn and registered:
  `QR-PRINT-ROUTE-MATCHES-DECLARATION`.

**Skipped, by reading** (functions decided alone on the BEFORE):

| callee | alone | why |
| --- | ---: | --- |
| `cJSON_DetachItemViaPointer` | 3 | a detach: custody of `item` moves from the parent's child list to the result, which is `item` itself; `return_alias` would say the result owns nothing and `return_owned` that it is fresh. A decision (`W48-CALLEE-DECISION-TABLE`) |
| `rd_kafka_destroy_app` | 2 | releases `rk` only without `RD_KAFKA_DESTROY_F_IMMEDIATE` (a literal premise would free only `rd_kafka_destroy`), and invokes application-registered callbacks held in `rk` (log, consumer-close rebalance and commit, interceptors' `on_destroy`): an open callback set registered elsewhere, outside C04.19. A decision |
| `rd_kafka_metadata_new_topic_mockv_internal` | 2 | a `va_list` parameter: the one declaration with one, `rd_kafka_error_new_v`, is withdrawn as `abi-unreadable-declaration` in `rdkafka_error.c` on this base (4 leaves). `W43-LIBC-ABI-READER-SHAPES` |
| `rd_kafka_topic_partition_list_difference0`, `_intersection0`, `_union0` | 2 each | `cmp`/`hash` stored into a frame `RD_MAP_INITIALIZER` and called through the map; rule 13y would withdraw an invocation clause, and a borrowed callback is forbidden. `CALLBACK-STORED-IN-FRAME-MAP` |
| `sched_thread_cleanup` | 2 | releases every element of a caller's intrusive list; no clause states it. `W44-CALLEE-DESTRUCTOR-SUMMARY` |
| slice `file_pending_flush` | 2 | two distinct static definitions of one name (`tail_dockermode.c`, `tail_multiline.c`); a name-keyed declaration binds both |
| the brief's list: `rd_kafka_log0` 3, `add_item_to_object` 3, `format_metric` 3, `rd_kafka_assignor_add` 3, `get_or_create_external_metadata_kvlist` 2, `rd_atomic32_add` 2, `_mk_event_wait_2` 2, `call_once` 2, `mk_header_prepare` 2, `rd_kafka_parse_Metadata0` 2, `rd_kafka_op_destroy` 2, `rd_kafka_q_serve0` 2, `rd_kafka_q_pop_serve0` 2, `rd_kafka_q_serve_maybe_consume` 2 | | decision classes, as the brief says |
| `cJSON_ParseWithLengthOpts` 2, `rd_kafka_txn_clear_partitions_flag` 2, `setupRackAwareAssignment0` 2, `ut_init_member_internal` 2 | | class 1 (a global holder; declared, withheld), class 3 (a container's reference set), class 2 (unit-test constructors into caller storage) |

## 3. Measurement

| route | BEFORE | AFTER (runs a3, a4) | `resource-diagnostics.json` |
| --- | --- | --- | --- |
| corpus | 915 / 9 / 3,910 | **933 / 9 / 3,892** | `ded14632…` → `cdb0267b…` |
| system | 91 / 4 / 353 | **93 / 4 / 351** | `f8aab120…` → `27e27b95…` (a3), `7ece6fe4…` (a4) |
| tail-pause slice | 50 / 1 / 349, 4 Pending | **52 / 1 / 347, 4 Pending** | `1654a682…` → `c45ae1d3…` (a3), `6b91084b…` (a4) |
| null property, corpus | 1,113 / 1,435 / 2,355 / 18, 29 findings | identical (every null verdict and check unchanged) | |
| null property, system and slice | 84 / 125 / 245 and 71 / 103 / 232 | identical | |

Corpus arbiter wall time 520.7 s BEFORE, 520.1 s AFTER (`--jobs 4`, inside
the lock). Run a4 is the final tree (after gate g1 moved
`mk_fifo_queue_destroy` from `monkey/memory.qv` to `monkey/intrusive_list.qv`,
section 5): its corpus report is byte-identical to a3's (`cdb0267b…`), and
its system and slice rows and every null row equal a3's (only the
contract-source identities in their digests differ).

**Corpus: +18 Unknown → Safe, no other status move**
([comparison](artifacts/compare-a3-corpus.json), [`scripts/compare.py`](scripts/compare.py)),
each attributed to the declaration whose leaf it lost:

* the five Admin-API destructors: `rd_kafka_AclBinding_free`,
  `_ConfigResource_free`, `_NewPartitions_free`, `_NewTopic_free`,
  `_UserScramCredentialAlteration_destroy_free`;
* `unit_test_telemetry`: `unit_test_telemetry_gauge`, `_sum`, and
  `unittest_telemetry_decode` through their validated summaries (all 18
  setters were Safe in-unit, so every callback candidate is the identity);
* `cfl_list_size`: `count_metrics`, `is_metric_empty`, and `get_metric_count`
  through `is_metric_empty`'s summary;
* the 1-tier: `rd_bytes_hash` (`rd_kafka_Uuid_hash`, `rd_kafka_Uuid_map_hash`),
  `rd_kafka_mem_calloc` (`LZ4_createStreamHC`), `cio_file_native_get_size`
  (`cio_file_real_size`, `cio_file_update_size`), `cio_stream_size_chunks_up`
  (`flb_input_chunk_total_size`, whose own declaration is now validated too),
  `cmt_atomic_initialize` (`cmt_initialize`).

**No Safe → non-Safe move, no leak or violation added** (Unsafe 9 → 9).
51 Unknown functions changed leaves without a status move (69 moving rows):
the five `_destroy_array` loops now read `indirect memory access is outside
the resource projection` at the element actual; `mk_fifo_queue_destroy`'s two
callers and three ConfigResource response parsers read `unresolved resource
identity`; 33 cmetrics and ctraces functions lose a `cfl_list_size` leaf (25 an
uncontracted-call leaf, 7 a branch-condition or access leaf at a statement
calling it, 1 through `count_metrics`' summary); the
`cio_file_update_size` callers and `LZ4_createHC`, `LZ4_createStreamDecode`
gain an unresolved-identity or nullable-to-non-null leaf where the freed call
exposed the next obligation. Added leaves: 16 unresolved identity, 6 access,
1 nullable-to-non-null; every one in a function that stays Unknown.

**System +2** (`flb_input_chunk_total_size` in two configurations,
`cio_stream_size_chunks_up`); **slice +2** (`flb_tail_scan_unregister_aged_out_inode`,
`_ignored_file_size`, `flb_hash_table_del`); nothing else moves
([system](artifacts/compare-a3-system.json), [slice](artifacts/compare-a3-slice.json)).

**Prediction against outcome.** Predicted corpus +14 to +20, measured +18:
stage (a) +5 as predicted (the five `_free`; the five `_destroy_array` stay on
the access leaf, which corrects the brief's "about 10"), `mk_fifo_queue_destroy`
0 (predicted 0 to 1), `cfl_list_size` +3 and `unit_test_telemetry` +3 (2
each predicted, plus one summary each), the 1-tier +7 (predicted 4 to 6, plus
two summaries), `print` +2 on run a1 then withdrawn. The prediction missed
two things: `cio_file_hash`'s own row regressing (predicted 0 to 1 movers, no
cost) and the compile-route collision of `print`. System +2 (predicted 0 to
1: two configurations), slice +2 as predicted. Null property unchanged, as
predicted.

**The Safe roll-up** ([`scripts/clause_rollup.py`](scripts/clause_rollup.py),
[artifact](artifacts/rollup-a3-corpus.json)): corpus Safe rows resting only
on validated declarations **101 → 108**; on no declaration 399 → 397 (the
own rows of `rd_bytes_hash` and `cmt_atomic_initialize` now consult their
declaration); on a declared assumption 367 → 380; carrying
a null premise 31 → 32 (`count_metrics`, null-Unknown). Declarations
consulted 346 → 359: validated 57 → 63, partly validated 36 → 37, declared
assumption 252 → 258. Of the 18 new Safe rows, 5 rest only on validated
declarations (`cmt_initialize`, `LZ4_createStreamHC`, `cio_file_real_size`,
`cio_file_update_size`, `flb_input_chunk_total_size`), 2 on a validated one
beside `memcpy` (the `Uuid` hashes), 3 on a partly validated one
(`unit_test_telemetry`), and 8 on declared assumptions (the five destructors,
`cfl_list_size`). Combined custody-and-null Safe 806 → 819.

**Parity, compared in full** ([`scripts/parity.py`](scripts/parity.py)):
corpus 690,246 → 689,574 rows, 71 functions differ, every one a moving
function, a declared callee's own row (`rd_kafka_UserScramCredentialAlteration_destroy`),
a caller of `cfl_list_size` whose leaves did not change
(`append_sample_to_metric`, `unpack_basic_type_meta`,
`create_export_service_request`) or unit-level rows; system 3 functions
(moving, unit rows); slice 10 (moving, `tail_scan_path` over
`flb_hash_table_del`, unit rows)
([corpus](artifacts/parity-a3-corpus.json), [system](artifacts/parity-a3-system.json),
[slice](artifacts/parity-a3-slice.json)).

The round-12 `mk_vhost_set` tip probe was not run: its scratch inputs are no
longer in the scratchpad, no contract it reads changed except
`monkey/intrusive_list.qv`'s new `mk_fifo_queue_destroy`, which `mk_lib.c` does not
call.

## 4. The remaining population and its head

On the AFTER ([ranking](artifacts/rank-a3-corpus.json)): **481** Unknown
functions blocked by uncontracted calls alone, **329** by one callee alone
(system 26 / 18, slice 40 / 26). 26 callees still decide 2 or more corpus
functions alone (57 functions): `rd_kafka_log0` 3, `add_item_to_object` 3,
`cJSON_DetachItemViaPointer` 3, `format_metric` 3, `rd_kafka_assignor_add` 3,
then twenty-one at 2. Each is a decision class, a reader extension, an
element-releasing destructor or the withdrawn `print` (section 2). The
1-tier holds 272 callees; the one left there with a Safe home body is the
withdrawn `cio_file_hash`.

## 5. Gate

Standing gate (`docs/agents/validation.md`) through [`scripts/gate.sh`](scripts/gate.sh)
and the provider gate through [`scripts/provider-gate.sh`](scripts/provider-gate.sh);
exits in [gate/](gate/), summary [gate.json](gate.json), filled in by
[`scripts/finish-gate.sh`](scripts/finish-gate.sh).

**Gate g1** (code steps at `a22d2ea01`, logs `private/w45r3/gate-g1/`) was
red: `run_test_binaries.py` exit 101, 7,890 passed and 6 failed in four
units, each a pin the new declarations touch, none an analysis result:
`source_contract_config` (the cmetrics-counter source policy pins
`qcore-provider-cmetrics/contracts.json`'s digest, `2a38c6fe…` →
`39db29d6…`, as CG22 and NULL-SAFETY-PROPERTY updated it),
`contract_inherit_qcore_provider` (2: `monkey/memory.qv` is pinned as
exactly the three inherited libc wrappers, so `mk_fifo_queue_destroy` moved
to `monkey/intrusive_list.qv` beside the `mk_list_del` its body calls, and
was re-pinned at both revisions), `contract_view_census` and
`contract_views` (2: `cfl_list_size` now has a resource-v1 declaration
beside its null result range, a multi-module declaration; registered as a
`null_deref_beside` relation in `qcore_contract::contract_views::REGISTERED_RELATIONS`,
21 → 22, census 19 → 20 functions). Every other step of g1 exited 0 (check,
submodule, fmt, clippy, both probes, 313 Lean audits); the provider gate g1
(at `218fb4d0f`) exited 0 at every step. Gate g2 is the gate of record.

**Deliberate pin updates.** `compiler/tests/contract_loader.rs`: v1
declarations 379 → 393 (selection 61 and rejected 314 unchanged), the
retaining table 44 → 45 (`unit_test_telemetry`), and the thirteen
function-map declarations' previous-mode rows; `compiler/tests/qcore_contract_corpus.rs`:
fourteen clause rows; `compiler/tests/contract_view_census.rs` and
`crates/qcore-contract/tests/contract_views.rs`: relations 21 → 22
(`null_deref_beside` 8 → 9); `contract/profiles/cmetrics-counter/source-contracts.json`:
the cmetrics inventory digest. Each crate's `contracts.lock` (and, for the
`qcore-provider-*` crates, `contracts.json`) by `pin` and `sync`.

## 6. Ledger

Follow-ups 760 → 763 ([`scripts/update_ledger.py`](scripts/update_ledger.py)):
amended `W45-CALLEE-CASCADE-RESIDUAL`, `W44-CALLEE-DESTRUCTOR-SUMMARY`,
`W48-CALLEE-DECISION-TABLE` (two new members: the detach and
`rd_kafka_destroy_app`) and `W43-LIBC-ABI-READER-SHAPES` (the `va_list`
member); registered `QR-PRINT-ROUTE-MATCHES-DECLARATION`,
`CALLBACK-STORED-IN-FRAME-MAP` (3 callees / 6 functions) and
`BORROWED-RESULT-SUMMARY-CONFLICT`.

## Files

* Contracts: `providers/qcore-provider-librdkafka/contracts/librdkafka/{client,kafka_mem}.qv`,
  `providers/qcore-provider-libevent/contracts/monkey/intrusive_list.qv`,
  `providers/quirin-provider-cfl/contracts/cfl/sds_lifetime.qv`,
  `providers/quirin-provider-fluent-bit/contracts/fluent_bit/hash_table_publication.qv`,
  `providers/qcore-provider-chunkio/contracts/chunkio/chunk_sizes.qv`,
  `providers/qcore-provider-cmetrics/contracts/cmetrics/context_lifetime.qv`;
  their `contracts.lock` and `contracts.json`.
* Pins: `compiler/tests/contract_loader.rs`, `compiler/tests/qcore_contract_corpus.rs`,
  `compiler/tests/contract_view_census.rs`, `crates/qcore-contract/tests/contract_views.rs`,
  `contract/profiles/cmetrics-counter/source-contracts.json`; the view registry
  `crates/qcore-contract/src/contract_views.rs` (one `REGISTERED_RELATIONS` row).
* Records: `docs/STATUS.md`, `docs/development/increments/wave-48-part6.md`,
  `docs/reviews/r0_3/acceptance_matrix.json`, `docs/reports/INDEX.md`.
* Scripts: `build.sh`, `measure.sh`, `measure-binary.sh`, `heavy.sh`,
  `rank.py`, `body.py`, `pin.sh`, `sync.sh`, `compare.py`,
  `parity.py`, `parity-all.sh`, `clause_rollup.py`, `totals.py`,
  `targeted.sh`, `gate.sh`, `provider-gate.sh`, `finish-gate.sh`,
  `write_gate_json.py`, `update_ledger.py`, `records.py`.
