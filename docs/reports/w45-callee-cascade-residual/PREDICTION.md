# W45-CALLEE-CASCADE-RESIDUAL: prediction (frozen before any contract)

Base: local `main` at `3905139cd` (CHUNKIO-META-LEN-BOUNDS-CONTRACTS gate-g2
records). BEFORE binary `private/w45ccr/bin/before`, sha256 `bb89368e…`
(each run's `run.json` `binary_sha256` equals it).

## 1. BEFORE identity (reproduced)

| route | statuses | `resource-diagnostics.json` |
| --- | --- | --- |
| corpus (`66910c10`, `--jobs 4`, 381 s) | 756 Safe / 9 Unsafe / 4,069 Unknown | `ce57d194…`, byte for byte the brief's baseline |
| system | 85 / 4 / 359 | `42d597d0…` (this worktree's paths) |
| tail-pause slice | 45 / 1 / 354, 4 Pending | `a0254744…` (this worktree's paths) |

The population reproduces: **704** Unknown corpus functions whose every leaf
is ``call to uncontracted function `F` `` (`scripts/rank.py`, the corpus arm
of CG10-INDIRECT-ACCESS-SOLE-RESIDUALS' `decides.py`: a function that keeps
any other leaf, an unread actual included, is not in it). 517 of them are
decided by one callee alone. System 29 (20 alone), slice 38 (27 alone).

The cited heads all reproduce: `cJSON_New_Item` 10, `LZ4_decompress_generic`
9, `rd_kafka_confval_set_type` 5, `rd_kafka_interceptor_method_add` 4,
`rd_kafka_queue_new0` 3, `rd_list_add` 3, `rd_list_clear` 3; the
`ctr_attributes_set_*` chain is `_int64`/`_string`/`_bool`/`_double`/`_array`/`_kvlist`
2 each (their callers in `ctr_span.c`), each `ctr_attributes_set_*` row itself
blocked by its `cfl_kvlist_insert_*` alone (1 each).

## 2. The ranking, read

Every callee deciding at least 2 functions alone (90 callees, 239 functions)
was read at both revisions ([artifacts/classification-before.json](artifacts/classification-before.json),
`scripts/body.py`; librdkafka, chunkio and cfl are byte-identical between the
trees, ctraces differs in the two `ctr_mpack_consume_*` bodies only by a range
check). `scripts/tally.py`:

| class | callees | functions alone |
| --- | ---: | ---: |
| decision 1, callbacks, retained pointers, custody-taking inserts | 32 | 86 |
| **contract (this card)** | **17** | **45** |
| reader: enum typedef parameter or result | 9 | 28 |
| decision 3, reference counts | 9 | 19 |
| decision 2, in-place lifecycles, out slots, reallocation | 7 | 18 |
| decision 8, the extended interior reading (child pointers) | 5 | 14 |
| reader: inline function-pointer declarator | 4 | 8 |
| destructor (callers release an element or a `void *`; yield ≤ 1) | 4 | 8 |
| reader: `const T **` | 3 | 7 |
| other (`print`, a generic name; a `va_list` actual) | 2 | 4 |
| decision 7, Monkey backend inline | 1 | 2 |
| below 2 alone (not read) | 650 | 278 |

Corrections to the cited heads, by reading: `LZ4_decompress_generic` (9) is
not contractable today: two of its parameters are untagged enum typedefs,
which the declared-ABI reader refuses (`W43-LIBC-ABI-READER-SHAPES`'s enum
shape), so any declaration is withdrawn at every call site.
`rd_kafka_confval_set_type` (5) retains `valuep` in its `PTR` arm (class 1),
besides an enum parameter. `rd_kafka_queue_new0` keeps a queue reference
(class 3). `rd_list_clear` runs the free callback over retained elements
(class 1). `ctr_attributes_set_array`/`_kvlist` go through
`cfl_kvlist_insert_array`/`_kvlist`, which take custody of `value` on success
only: a decision, not written.

## 3. The contracts and their predicted movers

Each is read at both revisions and states only what both bodies do; each is
pinned with `quirin-audit pin` at both trees.

| crate, module | callee | predicted movers (corpus, alone) |
| --- | --- | ---: |
| librdkafka `cjson.qv` | `cJSON_New_Item`: `hooks` borrowed, result owned nullable | 10 (`cJSON_Create{Null,True,False,Bool,Number,Array,Object,StringReference,ObjectReference,ArrayReference}`) |
| librdkafka `lz4_stream.qv` | `LZ4_compress_HC`, `LZ4_decompress_unsafe_generic`, `LZ4_compress_default`: neutral | 4 + 3 + 2 |
| librdkafka `kafka_mem.qv` | `load_descriptor_values`, `rd_kafka_aborted_txns_next_offset`, `rd_kafka_anyconf_desensitize`, `rd_kafka_dump0`: neutral; `rd_kafka_metadata_copy_internal`: result owned nullable | 2 each, 10 |
| ctraces `context_lifetime.qv` | `ctr_mpack_consume_int_tag`, `_uint_tag`; `ctr_attributes_set_string`/`_int64`/`_bool`/`_double`: neutral | 2 + 2 + 8 |
| cfl `sds_lifetime.qv` | `cfl_kvlist_count`; `cfl_kvlist_insert_string`/`_int64`/`_bool`/`_double`: neutral (key and value copied) | 2 + 4 (the `ctr_attributes_set_*` rows) |
| chunkio `chunk_residency.qv` | `_cio_file_up`: neutral, `cio_chunk_up_force`'s reading | 2 |

Ceiling 49 (45 alone plus the four `ctr_attributes_set_*` rows). **Predicted
corpus: +35 to +52 Unknown → Safe** (a freed statement may expose another
obligation, e.g. an owned `cJSON_New_Item` result stored or a cast actual
read; a cascade through `ctr_span_*` and the `LZ4_*` wrappers' own callers
may add a few). **No Safe → non-Safe move.** New leak or `Unsafe` leaves:
0 to 3, only where an owned `cJSON_New_Item` or metadata copy now flows
(each read by hand, provisional). **System and slice: no status move** (no
contracted callee is in either route's ranking).

Not attempted, by name and reason, in section 2's table: every decision
class (recorded in `W48-CALLEE-DECISION-TABLE` with these sizes), the reader
gaps (recorded on `W43-LIBC-ABI-READER-SHAPES` and
`LOADER-ABI-INLINE-FNPTR-DECLARATOR`), the destructors
(`W44-CALLEE-DESTRUCTOR-SUMMARY`) and `print`.

**No Lean**: contracts only, no semantics added; their existing checkers
apply.
