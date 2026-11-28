# FB-LEDGER-MEM-BUF-LIMIT-BOUND census

Fluent Bit `046accf6`, read from the prepared checkout
`private/fb-046accf6/fluent-bit-clang-source`. Every row below is checked by
[scripts/census.py](scripts/census.py), which also records the SHA-256 of
its 24 inputs and fails on any occurrence it cannot classify
([census.json](census.json): 93 rows; family hits: fields 54, content
writers 15, memfs length 4, append entry points 26; failures 0). Paths are
relative to the checkout. `IC` is `src/flb_input_chunk.c`.

## 1. What is counted

* **Resident bytes `R`**: `cio_stream_size_chunks_up(storage->stream)`
  (`lib/chunkio/src/cio_stream.c:258-276`), the sum of
  `cio_chunk_get_content_size` over the stream's up chunks. For a memory
  chunk that is `mf->buf_len` (`cio_chunk.c:326`). A memory chunk is always
  up, and `cio_chunk_down` does nothing to it (`cio_chunk.c:556-571`).
  Metadata is stored outside `buf_len` and is not counted.
* **`mem_chunks_size`**: a snapshot of `R`. It is assigned only from
  `flb_input_chunk_total_size` (IC:2987-2995), or 0 at creation:

| Site | Assignment | When |
|---|---|---|
| `src/flb_input.c:663` | `= 0` | instance creation |
| IC:3017 | `= total` (`:3014`) | `flb_input_chunk_set_limits` |
| IC:3151, IC:3159 | `= total` | `flb_input_chunk_set_up_down` (the second one only after a filesystem `cio_chunk_down`) |

Each instance owns one stream named after its unique `plugin.N` name
(`src/flb_storage.c:583-603`, `src/flb_input.c:458-459`).

## 2. Every site that increases `R`

Only a content write increases `R`. Every call that can change a chunk's
content length:

| Site | Effect | Behind the gate? |
|---|---|---|
| IC:3455 `flb_input_chunk_write(ic, final_data_buffer, final_data_size)` | `+final_data_size` on success (`cio_memfs.c:112`); unchanged on failure (`:101-105`) | **yes**: after the paused gate (IC:3314-3320) on every path; see §4 for the shutdown-flush arm |
| IC:799 | body of `flb_input_chunk_write` | the wrapper of the row above |
| IC:811 | body of `flb_input_chunk_write_at` | no caller |
| IC:1540, :1573, :1613 | truncation to a recovered offset | shrinks; `flb_input_chunk_map` of filesystem backlog chunks only |
| `src/flb_storage.c:956` | write into the DLQ stream (`rejected`, filesystem) | not an input stream |
| `src/flb_fstore.c:313` | write in an `flb_fstore` context (output plugins) | not an input stream |
| `cio_chunk.c:196`, `:208`, `:220` | `write_at` and `write` internals | covered by their callers |
| `cio_chunk.c:492` | rollback restores the length at `tx_begin` | does not grow `R` beyond its earlier value |
| `cio_memfs.c:56` | a new chunk starts empty | 0 bytes |

Creating a chunk (`input_chunk_get`, `cio_chunk_open`) adds 0 content bytes.
`cio_chunk_up_force` moves a filesystem chunk into `chunks_up` and does
nothing to a memory chunk.

**Append entry points.** All 26 hits (§census.json) reach
`input_chunk_append_raw` (IC:3251) on the engine thread. A threaded input
only enqueues (`flb_input_chunk_append_raw` IC:3798-3801,
`append_to_ring_buffer` IC:3616). The engine's collector drains the queue
after its own paused test (IC:3747) and calls `input_chunk_append_raw`
(IC:3773) or `flb_input_log_append_processed` (IC:3768), which is the only
caller that sets `local_append` (`flb_input_log.c:1611-1625` to `:1392`,
`:1503`).

## 3. The limit test and the pause/resume transitions

| Site | What it does |
|---|---|
| IC:2955-2966 `flb_input_chunk_is_mem_overlimit` | `limit <= 0` → false (`:2957`); `mem_chunks_size >= mem_buf_limit` → true (`:2961`) |
| IC:3004-3077 `flb_input_chunk_set_limits` | snapshot (`:3017`), then resume (`mem_buf_status = RUNNING`, `:3033`) only if not overlimit, `is_running`, `is_ingestion_active` and paused (`:3029-3032`). The collector resume (`:3049-3074`) reads the status and writes none |
| IC:3081-3132 `flb_input_chunk_protect` | storage overlimit → pause storage, return (`:3085-3092`); **rate gate fired → return** (`:3095-3098`); filesystem → return (`:3100-3102`); memory overlimit (`:3105`): `memrb` → return (`:3111-3113`), else `flb_input_pause` and `mem_buf_status = PAUSED` (`:3125-3126`) |
| IC:3314-3320 (the gate) | refuse when `flb_input_paused(in)` and not (`is_shutting_down` and `FLB_INPUT_SHUTDOWN_FLUSH`) |
| `include/fluent-bit/flb_input.h:783-807` | `flb_input_paused` is true when `mem_buf_status`, `storage_buf_status` or (metrics on) `rate_gate_status` is paused |
| `src/flb_input.c:661` | status `RUNNING` at creation |
| `src/flb_task.c:940` | `flb_task_destroy` calls `set_limits` after destroying its chunk |

`mem_buf_status` is written at exactly three sites: `flb_input.c:661`,
IC:3033 and IC:3126. `mem_buf_limit` is written only at configuration time:
`flb_input.c:662` (0 at creation), `:859` (the property),
`filter_rewrite_tag/rewrite_tag.c:66`, `filter_log_to_metrics/log_to_metrics.c:914`
and `filter_multiline/ml.c:138` (each when its emitter instance is created).

`input_chunk_append_raw` in order: `memrb` release (`:3280-3310`, `memrb`
only, when overlimit: frees at least `buf_size` bytes or returns -1), gate
(`:3314-3320`), `buf_size == 0` (`:3322`), chunk (`:3358`), up (`:3371-3378`),
filters (`:3442-3452`), **write** (`:3455`), failure → rollback (`:3479-3488`),
then one of three `return 0` exits: emptied (`:3510-3514`: destroy and
`set_limits`), unroutable (`:3541-3544`: destroy), stored (`set_limits`
`:3547`, output size update `:3590`, `protect` `:3599`, `return 0` `:3600`).
The script checks this order and that these are the only `return 0`s.

Neighbours that read the fields but change neither the counter nor the
status (rows in census.json): metrics (`flb_storage.c:202-243`), dump
(`flb_dump.c:81-99`), sosreport, the Calyptia configuration dump, log
arguments, the ingress queue limit derived from `mem_buf_limit`
(`flb_input_ingest.c:468-475`: bytes queued before the append, outside
`mem_chunks_size`), and the emitter's own stricter pre-append test
(`in_emitter/emitter.c:118-149`).

## 4. The bound and its premises

**What one write is.** `final_data_size` at IC:3455: `buf_size`
(`:3439`) or, for a log chunk, the filter chain's output size
(`:3442-3452`). The only bound on it is `buf_size != 0` (`:3322`). Nothing
bounds `buf_size`, and a filter can return more bytes than it was given. So
`W`, the largest `final_data_size`, is a named premise (P-MBL-WRITE-BOUND),
never assumed.

**Bound.** For an instance with `storage.type memory` or `memrb` and
`mem_buf_limit = L > 0`, at every state the transitions reach:

    R <= L + W   and   mem_chunks_size <= L + W

and, for `memory`, a running instance has `R <= mem_chunks_size < L`. The
bound is reached (`R = L - 1 + W`, `QCore.MemBufLimitBound.bound_reached`).

**Premises** (stated, not proved):

| Id | Premise |
|---|---|
| P-MBL-ENGINE-THREAD | Every append, release and `set_limits` of one instance runs on the engine thread, one at a time. No lock serialises the counter, and §2's entry points are the evidence for appends. |
| P-MBL-NO-REENTRY | Nothing called inside an append writes to the same instance's stream: the filters (`:3443`), the stream processor (`:3525`), `flb_input_chunk_update_output_instances` (`:3590`, which can only release) and the log calls. |
| P-MBL-WRITE-BOUND | `final_data_size <= W`. |
| P-MBL-OWN-STREAM | The instance's stream holds only its own chunks. |
| P-MBL-PHYSICAL | The `size_t` sums do not wrap. |
| P-MBL-LIMIT-SET | `mem_buf_limit > 0`, written only at configuration time (§3). |
| P-MBL-DESTROY | `flb_input_chunk_destroy` and `flb_task_destroy(task, FLB_TRUE)` remove the chunk's content from the stream. |
| P-MBL-NO-SHUTDOWN-FLUSH (`memory`) | No append while paused under `is_shutting_down && FLB_INPUT_SHUTDOWN_FLUSH`. |
| P-MBL-RATE-GATE-QUIET (`memory`) | `protect` reaches the memory test: the rate gate (`rate_gate_enabled`, metrics builds) does not return first. |
| P-MBL-FILTER-NONEXPANDING (`memrb`) | `final_data_size <= buf_size`. |

## 5. Sites the pause does not cover: the counterexamples

None of these is hidden or assumed away. Each is a premise above with a Lean
counterexample, and none is a defect claim:

* **Shutdown flush** (IC:3314-3316): a paused instance that is shutting
  down with `FLB_INPUT_SHUTDOWN_FLUSH` still appends, and each such append
  adds its bytes (`shutdown_flush_breaks_bound`,
  `shutdown_flush_append_grows`).
* **Rate gate first** (IC:3095-3098, on in the pinned production build,
  which defines `FLB_HAVE_METRICS`): `protect` returns before the memory
  test, so an instance past its limit keeps `mem_buf_status = RUNNING`. Once
  the rate gate resumes it, the next write passes `L + W`
  (`rate_gate_breaks_bound`).
* **Filesystem storage** (IC:3100-3102): `mem_buf_limit` never pauses it.
  The resident bytes are governed by chunk up/down (IC:3559-3575,
  `set_up_down`) and `max_chunks_up`, which stay uncertified by the user's
  decision (`fs_not_paused_by_mem_buf_limit`).
* **`memrb`** never pauses on memory. It releases at least `buf_size` bytes
  before writing, which keeps the bound only while filters do not expand
  (`memrb_expanding_filter_breaks_bound`).
* **`mem_buf_limit 0`** disables the test (`zero_limit_unbounded`).

Out of scope by the user's decision of 2026-11-28 and not certified here:
`max_chunks_up` with `cio_chunk_up_force` (`sb.c:412`, `:633`) and
`storage.backlog.mem_limit`'s batch test (`sb.c:605`, `:611`).
