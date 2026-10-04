# W45-CALLEE-CASCADE-RESIDUAL: the callees that decide Unknown functions alone

Card W45-CALLEE-CASCADE-RESIDUAL (wave 48, round 20, the user's top priority
of 2026-10-04). Base: local `main` at `3905139cd`, which it stayed at.
Contracts only: 21 declared, pinned contracts in four provider crates; no
analyzer code, no gateway rule (the reserved ids `12s`–`12w` are unused) and
no Lean, because no semantics were added (each contract is checked by the
existing loader, ABI and duty gates). [PREDICTION.md](PREDICTION.md) was
committed at `19ef75241`, before any contract was written.

## 1. BEFORE (task 1)

**Reproduced.** The BEFORE binary (`private/w45ccr/bin/before`, sha256
`bb89368e…`, equal to every run's `run.json` `binary_sha256`) gives the corpus
`ce57d194…`, **756 / 9 / 4,069**, byte for byte the brief's baseline; system
85 / 4 / 359 and slice 45 / 1 / 354 with 4 Pending, the brief's totals (their
digests embed this worktree's paths: `42d597d0…`, `a0254744…`).

**The population.** [`scripts/rank.py`](scripts/rank.py) is the corpus arm of
CG10-INDIRECT-ACCESS-SOLE-RESIDUALS' `decides.py`: a function is in it only when
every leaf reads ``call to uncontracted function `F` ``, so a function that
also needs an actual neither reader recognises (an access leaf kept by rule
12k), or any other obligation, is not counted. **704** reproduce; 517 of them
are decided by one callee alone. System 29 (20 alone), slice 38 (27 alone).
Rankings: [corpus](artifacts/rank-before-corpus.json),
[system](artifacts/rank-before-system.json), [slice](artifacts/rank-before-slice.json).

**The cited heads all reproduce:** `cJSON_New_Item` 10, `LZ4_decompress_generic`
9, `rd_kafka_confval_set_type` 5, `rd_kafka_interceptor_method_add` 4,
`rd_kafka_queue_new0` 3, `rd_list_add` 3, `rd_list_clear` 3; the
`ctr_attributes_set_*` chain is 2 each for its six setters (their
`ctr_span.c` callers), and each setter's own row is blocked by its
`cfl_kvlist_insert_*` alone.

## 2. The ranking, read

Every callee that decides at least 2 corpus functions alone (90 on the BEFORE)
was read at both revisions with [`scripts/body.py`](scripts/body.py) and
placed by reading ([classification](artifacts/classification-before.json)).
librdkafka (with its vendored cJSON, LZ4 and nanopb), chunkio and cfl are
byte-identical between `66910c10` and `046accf6` ([`scripts/treediff.py`](scripts/treediff.py));
ctraces differs, and of the bodies used here only the two `ctr_mpack_consume_*`
readers differ (a range check). [`scripts/tally.py`](scripts/tally.py) over the
AFTER ranking ([tally](artifacts/tally-after.txt); the BEFORE tally is
[here](artifacts/tally-before.txt)):

| class | callees | functions alone | head |
| --- | ---: | ---: | --- |
| decision 1: callbacks, retained pointers, custody-taking inserts | 36 | 94 | `cmt_mpack_unpack_array` 5, `ctr_mpack_unpack_array` 5, `rd_kafka_confval_set_type` 5, `LZ4_compress_HC_extStateHC` 4, `rd_kafka_interceptor_method_add` 4 |
| reader: an enum typedef parameter or result | 9 | 28 | `LZ4_decompress_generic` 9, `rd_kafka_error_new_v` 4, `rd_kafka_error_new` 3 |
| decision 3: reference counts | 9 | 19 | `rd_kafka_queue_new0` 3, `rd_atomic32_add` 2, the `*_destroy` that destroy partition lists |
| decision 2: in-place lifecycles, out slots, reallocation | 7 | 18 | `sds_cat_safe` 4, `cfl_sds_cat_safe` 3, `format_metric` 3 |
| decision 8: the extended interior (child-pointer) reading | 6 | 16 | `rd_kafka_admin_result_ret_groups` 4, `_resources` 3, `_topics` 3 |
| destructor (`W44-CALLEE-DESTRUCTOR-SUMMARY`) | 4 | 8 | `mk_fifo_queue_destroy`, `rd_kafka_AclBinding_destroy`: their callers release an element of a list or array, or a `void *` formal; at most 1 each would move |
| reader: `const T **` | 3 | 7 | `rd_flags2str` 3 |
| other | 2 | 4 | `print` (a static cJSON helper; a name-keyed declaration of so generic a name is not written), `rd_kafka_metadata_new_topic_mockv_internal` (takes the caller's `va_list`, rule 11w) |
| decision 7: Monkey backend inline | 1 | 2 | `_mk_event_wait_2` |
| below 2 alone (not read) | 640 | 277 | |

**Corrections to the cited figures, by reading.** None of the cited counts is
wrong, but four of the cited heads are not contractable as the registration
implies: `LZ4_decompress_generic` (9) takes two untagged enum typedefs, which
the declared-ABI reader refuses (`legacy_declared_abi.rs` `project_scalar`:
"an enumeration's compatible integer type is implementation-defined"), so any
declaration is withdrawn at every call site, the `W43-LIBC-ABI-READER-SHAPES`
enum shape; `rd_kafka_confval_set_type` (5) stores `valuep` in its `PTR` arm
(class 1); `rd_kafka_queue_new0` keeps a queue reference (class 3);
`rd_list_clear` runs the free callback over elements the list retained
(class 1). `ctr_attributes_set_array`/`_kvlist` go through
`cfl_kvlist_insert_array`/`_kvlist`, which take custody of `value` on success
only (`cfl_kvlist.c:353-437`): a custody-taking reading, stopped at and recorded.
**A correction to my own frozen prediction:** its table counted four callees
(8 functions) as an inline-function-pointer reader gap; CG22 taught the reader
that declarator, and the four pass `cmp`/`hash` or a test callback, so they
are class 1 (above).

## 3. The contracts (task 2)

Each states only what its body does at both revisions, cites the lines, and
says what it does not claim. Pinned by [`scripts/pin.sh`](scripts/pin.sh)
(`quirin-audit pin` at both trees, then `pin --check --crate` for each changed
crate at both, all exit 0; `pin --verify-contracts` exit 0; the whole-tree
`pin --check` exits 1 at both, as on the base, its one row not in force being
the libevent crate's tip-mode `mk_vhost_set`); manifests synchronized
(`providers/contracts.py sync` and `check` 0; the resource views are
unchanged, every module was already selected).

| crate, module | declarations |
| --- | --- |
| `qcore-provider-librdkafka` `cjson.qv` | `cJSON_New_Item`: `hooks` borrowed, result `resource_return_owned(nullable)` (released by the declared `cJSON_Delete`) |
| `qcore-provider-librdkafka` `lz4_stream.qv` | `LZ4_compress_HC`, `LZ4_decompress_unsafe_generic`, `LZ4_compress_default`: neutral (one-shot; the stream-state forms that leave pointers into `src` in a caller's state are class 1 and not declared) |
| `qcore-provider-librdkafka` `kafka_mem.qv` | `load_descriptor_values`, `rd_kafka_aborted_txns_next_offset`, `rd_kafka_anyconf_desensitize`, `rd_kafka_dump0`: neutral; `rd_kafka_metadata_copy_internal`: one fresh `rd_tmpabuf` allocation, owned nullable |
| `qcore-provider-ctraces` `context_lifetime.qv` | `ctr_mpack_consume_uint_tag`, `_int_tag` (true at both revisions, which differ by a range check); `ctr_attributes_set_string`, `_bool`, `_int64`, `_double` (the cfl insert copies key and value): neutral |
| `quirin-provider-cfl` `sds_lifetime.qv` | `cfl_kvlist_count`; `cfl_kvlist_insert_string`, `_bool`, `_int64`, `_double`: neutral, every allocation the list's, nothing given retained |
| `qcore-provider-chunkio` `chunk_residency.qv` | `_cio_file_up`: neutral, the body `cio_chunk_up_force`'s declaration already reads |

No summary route applied: every top callee's own body is Unknown, and none is
a header body that would validate once its own callee is declared (the
`rd_avg_add` shape).

**Deliberate pin updates.** `compiler/tests/contract_loader.rs`: 272 → 293 v1
declarations (selection 60 and rejected 218 unchanged), nineteen
previous-mode neutral rows and two owned-result rows; `compiler/tests/qcore_contract_corpus.rs`:
21 clause rows. No other consumer pins these modules' or inventories' digests.

## 4. Measurement (task 4)

| route | BEFORE | AFTER | `resource-diagnostics.json` |
| --- | --- | --- | --- |
| corpus (`66910c10`) | 756 / 9 / 4,069 | **803 / 9 / 4,022** | `ce57d194…` → `2734991a…` |
| system | 85 / 4 / 359 | 85 / 4 / 359, no row moves | `42d597d0…` → `0821eb6b…` |
| tail-pause slice | 45 / 1 / 354, 4 Pending | unchanged, no row moves | `a0254744…` → `5d9f9490…` |

The same binary measured both sides (contracts are read at run time); the
system and slice digests move only in each unit's `resource_contract_sources`
and `inventory_sha256` identity, not in any row.

**Corpus: +47 Unknown → Safe, no other status move**
([comparison](artifacts/compare-corpus.json), [`scripts/compare.py`](scripts/compare.py)),
each attributed to the callee whose leaf it lost:

* `cJSON_New_Item` 7: `cJSON_CreateNull`, `_True`, `_False`, `_Bool`,
  `_Number`, `_Array`, `_Object`;
* the ctraces setters 8 (`ctr_span_set_attribute_*` and
  `ctr_span_event_set_attribute_*` for string, bool, int64, double) and the
  four setters' own rows through the cfl inserts 4;
* `LZ4_decompress_unsafe_generic` 4 (`LZ4_decompress_fast`, `_extDict`,
  `_withPrefix64k`, and `_usingDict`), plus `LZ4_uncompress` through
  `LZ4_decompress_fast`'s now-validated summary, 1;
* `load_descriptor_values` 3 (`pb_field_iter_next`, `_find_extension`, and
  `pb_field_iter_find`);
* 2 each: `LZ4_compress_HC` (`LZ4_compressHC_limitedOutput`,
  `LZ4_compressHC2_limitedOutput`), `LZ4_compress_default`, `_cio_file_up`
  (`cio_file_up`, `cio_file_up_force`), `cfl_kvlist_count`
  (`ctr_attributes_count`, `get_attributes_count`), each `ctr_mpack_consume_*`,
  `rd_kafka_dump0` (`rd_kafka_dump`, `rd_kafka_crash`),
  `rd_kafka_anyconf_desensitize`, `rd_kafka_metadata_copy_internal`,
  `rd_kafka_aborted_txns_next_offset`.

**No Safe → non-Safe move, no correction; no leak or violation added**
(`leak` 40 → 40, `summary_conflict` 2 → 2, `Unsafe` 9 → 9). Every added leaf
is an `unsupported` refusal in a function that stays Unknown: 18 ``nullable
resource `X` used by a non-null contract`` and 18 ``unresolved resource
identity`` where a `cJSON_Create*` result now flows as an owned nullable
pointer (the `cJSON_Add*ToObject` helpers, `cJSON_CreateString`/`_Raw`, the
`cJSON_Create*Array` loops, `cJSON_Duplicate_rec`), 3 loop re-acquisitions in
those loops, 4 access refusals (a pointer store into the new node, below) and
2 nested `LZ4_compressBound` calls. 50 Unknown functions changed leaves
without a status move (97 moving rows in all).

**Prediction against outcome.** Predicted +35 to +52 with a ceiling of 49,
measured **+47** (49 − 5 + 3). Five predicted movers stayed Unknown, each on an obligation
the freed statement exposed: `cJSON_CreateStringReference`,
`_ObjectReference`, `_ArrayReference` store a pointer into the new node
(`item->child = (cJSON*)cast_away_const(child)`, a refused pointer store into
a tracked owner, which then makes the returned `item` unresolved);
`LZ4_compressHC` and `LZ4_compressHC2` pass `LZ4_compressBound(srcSize)` as an
actual (a nested call). Three movers were not counted in the ceiling
(`LZ4_decompress_fast_usingDict`, `pb_field_iter_find`, `LZ4_uncompress`).
System and slice: no move, as predicted. No new leak or Unsafe, within the
predicted 0 to 3.

**Parity, compared in full** ([`scripts/parity.py`](scripts/parity.py)):
system and slice streams are multiset-identical once Clang AST node addresses
(process memory) are erased (141,186 and 50,426 rows; stream order differs
with `--jobs 4`). The corpus stream (669,449 → 668,705 rows) differs in 111
functions: every one is a moving function, a declared callee's own row
(`ctr_mpack_consume_*`, `rd_kafka_metadata_copy_internal`, `rd_kafka_dump0`,
`rd_kafka_anyconf_desensitize`, `rd_kafka_aborted_txns_next_offset`,
`_cio_file_up`), a caller of a declared callee whose leaves did not change
(`parse_array`, `parse_object` over `cJSON_New_Item`;
`clone_exemplars_to_kvlist`, `pack_cfl_variant_kvlist` over the cfl
declarations; `otel_span_set_attributes`/`_links` over the newly Safe
`get_attributes_count`), or the unit-level call-graph rows of the changed
units ([corpus](artifacts/parity-corpus.json), [system](artifacts/parity-system.json),
[slice](artifacts/parity-slice.json)).

**`mk_vhost_set` at the tip** ([`scripts/tip-probe.sh`](scripts/tip-probe.sh),
[rows](artifacts/tip-rows-after.txt)): unchanged from CG10-INDIRECT-ACCESS-SOLE-RESIDUALS'
record; no Monkey contract changed. The tip's one refusal is the uncontracted
`mk_vhost_set_property` (its DocumentRoot store, real behaviour, refused).

## 5. The remaining population and its head

On the AFTER ([ranking](artifacts/rank-after-corpus.json)): **652** Unknown
functions are blocked by uncontracted calls alone, 473 by one callee alone:
704 less 46 of the 47 new Safe rows (the 47th, `pb_field_iter_find`, also
held an unresolved-identity leaf, which went with its callee's) and less 6
that now hold another leaf (the three cJSON reference creators, the two
`LZ4_compressHC` wrappers, and `ctr_resource_create_default`, whose `res` is
unresolved once its setter calls are read); none entered. The head is
`LZ4_decompress_generic` 9 (the enum reader shape), then `cmt_mpack_unpack_array`
5, `ctr_mpack_unpack_array` 5, `rd_kafka_confval_set_type` 5 (class 1),
`LZ4_compress_HC_extStateHC` 4, `rd_kafka_admin_result_ret_groups` 4 (class 8),
`rd_kafka_error_new_v` 4 (enum), `rd_kafka_interceptor_method_add` 4,
`sds_cat_safe` 4 (class 2). Every remaining callee that decides 2 or more is a
decision, a reader shape, a destructor whose callers would move at most one
each, or `print`; the yield of a further contract is at most about 1, the
card's stopping rule.

**Decisions not taken** (amending `W48-CALLEE-DECISION-TABLE`, functions
decided alone on this AFTER): class 1 36 callees / 94, among them the new
custody-taking inserts (`add_item_to_object` 3, `add_item_to_array` 2,
`replace_item_in_object` 2, `ctr_attributes_set_array`/`_kvlist` over
`cfl_kvlist_insert_array`/`_kvlist` 2 each), the retaining stream states
(`LZ4_compress_HC_extStateHC` 4, `LZ4_compress_HC_continue` 3,
`LZ4_compress_fast_continue` 3, `LZ4_compress_fast_extState` 2),
`append_to_ring_buffer` (stores `ins` in the raw chunk it hands to the ring
buffer; corpus 2, system 2) and `cJSON_ParseWithLengthOpts` (stores `value`
in the file-scope `global_error`); class 2 7 / 18; class 3 9 / 19; class 7
1 / 2; class 8 6 / 16 (`rd_kafka_admin_result_ret_*`, `get_object_item`,
`get_or_create_external_metadata_kvlist`, `rd_kafka_topic_partition_get_private`).

## 6. Gate

Recorded in section 7 after the run.

## Files

* Contracts: `providers/qcore-provider-librdkafka/contracts/librdkafka/{cjson,lz4_stream,kafka_mem}.qv`,
  `providers/qcore-provider-ctraces/contracts/ctraces/context_lifetime.qv`,
  `providers/quirin-provider-cfl/contracts/cfl/sds_lifetime.qv`,
  `providers/qcore-provider-chunkio/contracts/chunkio/chunk_residency.qv`, their
  `contracts.lock` and (for the three `qcore-provider-*` crates) `contracts.json`.
* Pins: `compiler/tests/contract_loader.rs`, `compiler/tests/qcore_contract_corpus.rs`.
* Scripts: `build.sh`, `measure.sh`, `measure-binary.sh`, `rank.py`,
  `body.py`, `defs.py`, `treediff.py`, `tally.py`, `pin.sh`, `sync.sh`,
  `compare.py`, `parity.py`, `tip-probe.sh`, `tip_rows.py`, `targeted.sh`,
  `gate.sh`, `provider-gate.sh`, `finish-gate.sh`, `update_ledger.py`.
