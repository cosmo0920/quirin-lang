# FB-LEDGER-MEM-BUF-LIMIT-BOUND: mem_buf_limit + one write, certified under named premises

Card FB-LEDGER-MEM-BUF-LIMIT-BOUND, 2026-11-28, which discharges the
`mem_buf_limit` part of FB-LEDGER-ADVISORY-LIMITS. User decision of
2026-11-28: certify **limit + one write only**. `max_chunks_up` (with
`cio_chunk_up_force`, `sb.c:412`, `:633`) and `storage.backlog.mem_limit`'s
batch test (`sb.c:605`, `:611`) stay uncertified by that decision. Base
`287dce8ed` (main, r86). Probe `1dae0daee`, prediction `6e157a91f`, semantic
commit `748c0a6a7` (the measured tip; later commits change records only).

## Headline

* **Census** ([CENSUS.md](CENSUS.md), checked by
  [scripts/census.py](scripts/census.py) with the SHA-256 of all 24 inputs in
  [census.json](census.json)). It covers every site at Fluent Bit `046accf6`
  that changes an input instance's resident chunk bytes, tests
  `mem_buf_limit`, or writes `mem_chunks_size`, `mem_buf_status` or
  `mem_buf_limit`: 93 rows, and every grep hit is classified.
  `mem_chunks_size` is only ever a snapshot of
  `cio_stream_size_chunks_up`. The only call that grows a memory instance's
  content is the append's one write (`src/flb_input_chunk.c:3455`), and it
  sits after the paused gate (`:3314-3320`).
* **The bound.** For `storage.type memory` or `memrb` with `mem_buf_limit
  = L > 0`, every state the transitions reach has `R <= L + W` and
  `mem_chunks_size <= L + W`. Here `R` is the stream's resident content bytes
  and `W` bounds `final_data_size`, the bytes one `input_chunk_append_raw`
  call writes. For `memory`, a running instance is below `L`. The bound is
  tight: `L - 1 + W` is reached.
* **One write is `final_data_size`.** It is `buf_size`, or the filter
  chain's output for a log chunk. No site bounds it (only `buf_size != 0`,
  `:3322`), so `W` is a named premise, P-MBL-WRITE-BOUND.
* **Premises** (CENSUS §4): engine-thread serialisation with no lock (the
  entry-point census is the evidence for appends), no re-entrant append from
  filters, the stream processor or output accounting, the write bound, one
  stream per instance, no `size_t` wrap, a non-zero limit set only at
  configuration time, and destroy removing content. Per store, `memory` also
  needs P-MBL-NO-SHUTDOWN-FLUSH and P-MBL-RATE-GATE-QUIET, and `memrb` needs
  P-MBL-FILTER-NONEXPANDING.
* **Sites the pause does not cover, each a premise with a Lean
  counterexample** (CENSUS §5):
  * **Rate gate.** `protect`'s rate-gate return (`:3095-3098`) comes before
    the memory test. It is compiled in the pinned production build, so an
    over-limit instance can stay running and grow past `L + W`.
  * **Shutdown flush.** The gate's shutdown-flush arm (`:3314-3316`) admits
    appends to a paused instance.
  * **Filesystem storage** (`:3100-3102`) is never paused by
    `mem_buf_limit`.
  * **Expanding filters on `memrb`**: a filter that writes more than
    `buf_size` grows the instance.
  * **`mem_buf_limit 0`** disables the test.

  These are readings of the C, not defect claims.
* **Lean**: `QCore.MemBufLimitBound`, 27 audited theorems, no `sorry`, no
  project axioms. It holds the invariant and the bound
  (`mem_buf_limit_plus_one_write`, `running_mem_below_limit`), its
  tightness (`bound_reached`), the gate, protect and resume decision tables
  (`gate_vectors`, `protect_vectors`, `resume_vectors`, with
  `gateC_is_admitted`, `protect_matches` and `setLimits_resumes` tying them
  to the transition functions), and the five counterexamples.
* **No verdict moves** (§4). A numeric invariant over a run is not a custody,
  null or lifetime property of a function, and no route reports resource
  ceilings, so no route can carry it.

## 1. The model

The state is `(resident, snapshot, paused)`. The steps are:

* `append`: the `memrb` release; the gate restricted to `mem_buf_status`;
  then one of four exits, `refused`, `emptied`, `unroutable` or `stored`.
  `stored` runs `set_limits`, an optional eviction and `protect`.
* `release`: a chunk destroyed outside an append.
* `setLimits`: `flb_task_destroy`'s refresh.

The other pause reasons (storage, rate gate) only add refusals at the gate.
The model ignores them there, so it admits a superset of the C's appends.
The invariant is `resident <= snapshot <= L + W`, and for `memory`,
`not paused -> snapshot < L`. Each step preserves it under its per-step
premises (`Admissible`) and the configuration premises (`Certified`).

The invariant needs `resident <= snapshot` because the snapshot goes stale
between refreshes. For `memory`, that matters only through the snapshot
refresh in `set_limits`, which happens before `protect` on the stored path.
For `memrb`, the release decision reads a stale snapshot: if that snapshot
is below `L`, the resident bytes are too. If it is at or above `L`, the
release frees at least `buf_size >= final_data_size` bytes first.

## 2. The checked reading of the bodies

The probe's fixtures (`crates/qcore-c-gateway/tests/unit/ledger_transition.rs`,
module `mem_buf_limit`, compiled into the crate's own unit tests) reproduce
`flb_input_chunk_set_limits`, `flb_input_chunk_protect` and
`input_chunk_append_raw`. Route T reads them, and the tests read every path:

| Test | Reads | Lean counterpart |
|---|---|---|
| `protect_pauses_only_past_the_limit_and_only_when_no_earlier_return_fires` | the 6 protect paths as `(storage overlimit, rate gate, store, memory overlimit, pauses)`; the pause write follows `flb_input_pause` | `protect_vectors` row for row, `protectPausesMem` |
| `set_limits_snapshots_the_total_and_resumes_only_below_the_limit` | `mem_chunks_size` written once per path, from `flb_input_chunk_total_size`; resume rows | `resume_vectors`, `resumes` |
| `every_path_to_the_write_passes_the_paused_gate` | the 19 `append_entry` paths: the 6 that fall through are `{not memrb, memrb below limit, memrb released} x {running, shutdown flush}`; a failed release returns before the gate | `gate_vectors`' admitted rows, `gateC` |
| `one_call_writes_once_and_its_size_is_final_data_size` | the 26 `append_write` paths: 4 single writes of `buf_size` or the filtered size, 8 failed and rolled back, 14 with no write | `Exit.refused`, `Append.written` |
| `every_stored_write_reaches_set_limits_then_protect` | the 13 `append_finish` paths: 1 emptied, 6 unroutable, 6 stored (`set_limits` then `protect`, nothing after) | `Exit.emptied`, `.unroutable`, `.stored` |

There are seven mutation controls. A pause moved above the memory test
reads different protect rows, and a resume without the limit test loses the
refusal row. The other five are refused outright: a snapshot written from
another value, an entry segment without the gate, a second write, a protect
before `set_limits`, and a stored path that skips protect.

The deviations are stated in the fixture's doc comment. None touches a
statement that changes chunk content or `mem_buf_status`:

* The static inline helpers are declared functions.
* The log calls are dropped, because route T refuses their unsequenced
  `flb_input_name(...)` arguments, as with FB-LEDGER-LOG-ARGUMENT-ORDER.
* Route T reads at most 64 paths per function. `set_limits` keeps its
  memory half, and the append is read as three consecutive segments, each
  entered only where the one before falls through.
* The filter's out-parameters are read through declared functions.
* Metrics, tag clean-up, the chunk lock and the filesystem-only down branch
  are dropped.

[scripts/census.py](scripts/census.py) checks the pinned text itself, as
the fixtures do not. It confirms that the write follows the memrb release
and the gate, that the function's only `return 0`s are the three exits
(`:3513`, `:3543`, `:3600`), and that `protect` follows the last
`set_limits`. It also confirms that `protect`'s tests run in the order the
table assumes.

## 3. Hook points

None in shared code. The changes are one new module inside the existing
unit-test file `crates/qcore-c-gateway/tests/unit/ledger_transition.rs`
(appended at its end), the new Lean module and audit, and the generated
`proof/QCore.lean` line. No crate source, gateway rule, provider contract,
lock, schema, profile or route changes. No rule id is used, because 95 was
not needed.

## 4. Measurement

PREDICTION ([PREDICTION.md](PREDICTION.md)): no row moves on any route.
**Measured: as predicted.**

| Run | Exit | Units Safe/Unsafe/Unknown/data-only | Functions Safe/Pending/Unknown/Unsafe |
|---|---|---|---|
| base `287dce8ed` | 0 | 9 / 6 / 162 / 4 (181) | 1,209 / 2 / 3,586 / 7 (4,804) |
| tip `748c0a6a7` | 0 | 9 / 6 / 162 / 4 (181) | 1,209 / 2 / 3,586 / 7 (4,804) |

`moved.py` reports `{}` and `changed_rows.py` finds no row whose status or
leaf multiset differs. [identity.py](scripts/identity.py), which blanks
`evidence_version` (it carries the measuring checkout's HEAD) and its
derived digests, finds all 4,804 function rows identical
([identity.json](artifacts/identity.json); [totals](artifacts/)). The
release binaries' SHA-256 differ (base `5d870fde…`, tip `9920aaf0…`)
although the crate sources are identical between the two trees (only a
unit-test module and Lean changed). The reports agree row for row, so the
byte difference is not a semantic one.

The runs used opt-in `current --monkey-linked` over
`private/fb-046accf6`, each from its own `git archive` tree and release
binary, through `scripts/gate_slot.py audit`
([scripts](scripts/), FB-BACKLOG-ACCOUNTING's copies). The copied
`opt-in-current.sh` pointed at `opt_in_providers.py` under its pre-archive
path, so the first base attempt refused at once ("a contract load path is
given but no provider is selected"; kept as `private/mbl/run-base-attempt1/`,
not evidence). The fixed path is under `docs/reports/archive/`. Logs:
`private/mbl/run-{base,tip}/` in this worktree. The system-slice route was
not run: the bound is on no route, and no crate source changed. The pinned
routes (`quirin-audit measure --declared`) cannot run on this host (no
`build/cycle19` inputs), so no pinned figure is claimed.

## 5. Gate

`python3 scripts/gate_scope.py --base 287dce8ed`, run through
`private/toolchain/env.sh` (Ubuntu clang 21, Lean 4.35.0). The diff reaches
`qcore-c-gateway` only through its unit-test module (a test-only reach), so
the Rust step is the selected targets. Lean runs in full because
`proof/QCore.lean` changed.

FINAL_GATE

An earlier attempt (`private/gate/`, on `2c07d93d6` plus the uncommitted
report) failed two steps, both the card's own and both fixed:

* Clippy on `qcore-c-gateway --lib --profile test` (exit 101):
  `enum_variant_names` on the test enum `Memrb::NotMemrb`, renamed.
* The Python suite (814 tests, 1 failure):
  `test_report_hygiene...test_real_repository_passes_hygiene_and_index_checks`,
  because `docs/reports/INDEX.md` was stale for the new report directory.
  It was regenerated with `scripts/build_docs_index.py`.

Every other step passed there: submodules 0, fmt 0, the other four Clippy
runs 0, Rust 14 units with 1,605 passed / 0 failed / 2 ignored, and Lean 537
audits. No base failure was seen. No provider path changed, so the provider
gate was not run. The LLP64 fixture scan over the card's test file exits 0
(`0 counted`). `check_report_hygiene.py` passes.

## 6. Follow-ups

Amended in [integration-amendments.json](integration-amendments.json):

* **FB-LEDGER-ADVISORY-LIMITS.** The `mem_buf_limit` part is certified as
  limit + one write under the named premises. The rate-gate and
  shutdown-flush sites are recorded as counterexamples outside it.
  `max_chunks_up` and `storage.backlog.mem_limit` stay uncertified by the
  user's choice. Not retired, because the other two limits remain.

Registered:

* FB-MBL-PINNED-BODIES: the checked reading runs on reproductions. Route T
  reads the pinned functions only cut into segments and without their log
  calls.

## 7. Increment paragraph

**FB-LEDGER-MEM-BUF-LIMIT-BOUND** (the mem_buf_limit part of FB-LEDGER-ADVISORY-LIMITS; user decision 2026-11-28: certify limit + one write only; Lean `QCore.MemBufLimitBound`; no gateway rule; [report](../../reports/fb-ledger-mem-buf-limit-bound/README.md)). A checked census (CENSUS.md; census.py with the SHA-256 of its 24 inputs, 93 rows, every grep hit classified) of Fluent Bit 046accf6 shows that mem_chunks_size is only a snapshot of cio_stream_size_chunks_up, that the append's one write (flb_input_chunk.c:3455, final_data_size bytes) is the only call growing a memory instance's content and sits after the paused gate (:3314-3320), that mem_buf_status is written only at creation, set_limits' resume (only below the limit) and protect's pause (after the write), and that mem_buf_limit is set only at configuration time. Lean proves mem_chunks_size <= L + W and resident bytes <= L + W for storage.type memory and memrb, where W bounds final_data_size (no site bounds it: premise P-MBL-WRITE-BOUND), under engine-thread serialisation (no lock), no re-entrant append, one stream per instance, no size_t wrap, a non-zero limit and destroy removing content, plus no shutdown-flush append and a quiet rate gate for memory and non-expanding filters for memrb; a running memory instance stays below L, and L - 1 + W is reached. Counterexamples, none a defect claim: protect's rate-gate return before the memory test (:3095-3098, compiled in the pinned build), the gate's shutdown-flush arm, filesystem storage (never paused by mem_buf_limit), expanding filters on memrb, and limit 0. Fixtures in qcore-c-gateway's ledger_transition unit tests read route T's paths through set_limits, protect and the append (three segments, as T reads at most 64 paths; log calls dropped) row for row against the Lean gate, protect and resume vectors, with seven mutation controls. max_chunks_up and storage.backlog.mem_limit stay uncertified by the user's choice. Opt-in current --monkey-linked, base 287dce8ed against tip 748c0a6a7: no moved row; 4,804 function rows and 181 unit statuses identical. Follow-up: FB-MBL-PINNED-BODIES.
