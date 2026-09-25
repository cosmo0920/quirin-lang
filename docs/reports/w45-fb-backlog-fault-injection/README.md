# W45-FB-BACKLOG-FAULT-INJECTION — native fault-injection observations

This card runs the two priority-1 experiments the Codex accounting investigation
named in [`fb-system-audit-cards/accounting`](../fb-system-audit-cards/accounting/README.md)
(`FB-AUDIT-03-ZERO-PROGRESS`, `FB-AUDIT-03-PARTIAL-FANOUT`), on a real Fluent Bit
build, with controls. It is **native observation only**: no analyzer, contract,
provider or Lean change, and **no Quirin verdict moves**. A reproduced behaviour
here is a *local observation of the candidate under an injected premise*, never an
upstream defect claim, a confirmed production bug, or a production failure-rate
statement. Nothing was sent upstream. Evidence binding, hashes and commands are in
[`evidence.json`](evidence.json); resume steps in [`RESUME.md`](RESUME.md).

## Build and binding

- Quirin base (worktree/main): `6bd9b383`.
- Fluent Bit source: `864c3d97f3589515285a19396ed8dfc85cda08ae`. Confirmed on the
  user checkout `/home/cosmo/Documents/GitHub/fluent-bit` (`rev-parse HEAD`), on the
  fresh `git clone --no-hardlinks` working tree (its branch ref = the pin), and in
  the built binary's `--version` (`Fluent Bit v5.1.3, Git commit 864c3d97…`). No
  source change; no submodules (all deps vendored under `lib/`).
- Toolchain: Clang 21.1.8, CMake 4.2.3, Ninja 1.13.2.
- Two builds under `private/w45-fb/w45-fb-backlog-fault-injection/attempt-01/`
  (never inside the worktree): a Debug build (`fluent-bit`, sha256
  `ea5f21be…`) and a Debug + `-DSANITIZE_ADDRESS=On` build (sha256 `10392a47…`).
  Configure/build exits all 0. Full commands and log hashes in `evidence.json`.

## Fixture

A filesystem-storage backlog with fan-out to two output queues over one physical
chunk each:

1. **Generate** (`evidence/phase1-generate.conf`): `dummy` input with
   `storage.type filesystem`, two `tcp` outputs to closed ports (127.0.0.1:1 and
   :2, `retry_limit no_limits`) so delivery fails and chunks persist. 13 chunks
   persisted under `run/storage/dummy.0/` (twelve 36864 B, one 4096 B); names,
   sizes and SHA-256 in `evidence/chunk-listing.txt` and `evidence/chunk-hashes.txt`.
2. **Backlog** (`evidence/phase2-backlog.conf`): restart against the same
   `storage.path` with `storage.backlog.mem_limit 50M` and the two `tcp` outputs.
   The internal `storage_backlog` input re-injects the persisted chunks; because
   both outputs `Match test.data`, each physical chunk is segregated into **two**
   output backlog queues. The control run's engine log shows each chunk flushed to
   both `out_id=0` and `out_id=1`, confirming the shared-chunk fan-out.

A pristine copy (`run/storage-pristine`) is restored before every run. Every native
run has a watchdog (`timeout -k`, or a gdb intercept cap) with its exit captured;
Fluent Bit ignores `SIGTERM` while retries are pending, so control runs are ended
by the watchdog (exit 124/kill) — expected, not a finding.

## FB-AUDIT-03-ZERO-PROGRESS — observed-local

Premise: `cio_chunk_up_force` returns `CIO_ERROR` persistently for every nonempty
queue head. Injected with gdb forcing `cio_chunk_up_force` (the call at
[`sb.c:633`](../../../plugins/in_storage_backlog/sb.c)) to return `-1`, gated to
fire only once `cb_queue_chunks` has been entered so startup segregation is
untouched. A breakpoint at the callback's closing `return` (`sb.c:724`) counts
yields; the interception counter caps at 400.

| Run | Collector enters | Collector returns (yields) | Notes |
|---|---|---|---|
| **Fault** (persistent `CIO_ERROR`) | 1 | **0** | 400 `up_force`→`CIO_ERROR` interceptions (200 passes × 2 queues); callback never returned |
| Control (valid head) | 15 | 15 | first pass brings up all 13 chunks and queues them; queues drain |
| Control (`CIO_CORRUPTED`) | 19 | 19 | 13 forced `-3` each log "removing corrupted chunk from the queue" (`sb.c:635-647`), wrappers removed, queues drain |

Under the fault the `while` at `sb.c:611-612` stays true: the `CIO_ERROR` arm
(`sb.c:649-650`) `continue`s without touching `total`, the queue, or
`empty_output_queue_count`, and the nonempty heads never let
`empty_output_queue_count` reach the backlog count. The callback did **not** yield
(no return between entry and the 400-interception cap). The valid-head control
drains and yields every pass; the `CIO_CORRUPTED` control removes wrappers and
yields — so the no-progress behaviour is specific to the persistent-`CIO_ERROR`
premise. Loop-yield: **no** under fault; **yes** under both controls. Stale
reference: none. Evidence: `evidence/zp-*.wd.txt`, `gdb/zp-*.gdb`.

## FB-AUDIT-03-PARTIAL-FANOUT — observed-local

Premise: the **second** wrapper insertion fails for a chunk that fans out to two
outputs. Injected with gdb forcing the 2nd `sb_allocate_chunk` call (`flb_calloc`
at `sb.c:121`, from the append loop `sb.c:341-350`) to return `NULL` during startup
segregation, so `sb_append_chunk_to_segregated_backlogs` returns `-3`. Observed
under the ASan build.

- **Fault (ASan): heap-use-after-free.** The first output's wrapper was inserted
  and charged; on the `-3` return `sb_segregate_chunks` (`sb.c:472`) called
  `cio_chunk_close` (`cio_chunk.c:133`), freeing the physical `cio_chunk` and its
  name string (`strdup` in `cio_chunk_open`, `cio_chunk.c:66`, at scan time). The
  first wrapper survived with its stale chunk pointer and its
  `fs_backlog_chunks_size` charge still counted. The next collector pass read the
  freed memory:
  `cb_queue_chunks` (`sb.c:633`) → `cio_chunk_up_force` → `cio_file_up_force` →
  `_cio_file_up` (`cio_file.c:828`) → `cio_log_print`. Full ASan report (free site,
  use site, allocation site) in `evidence/pf-fault.asan.txt`.
- **Control (ASan, no fault):** 26 `sb_allocate_chunk` calls (13 chunks × 2
  outputs), all wrappers inserted and charged, 13 queued, **zero** ASan errors.

So a later traversal *does* touch the surviving wrapper, and the touch is a read of
freed memory. Loop-yield: n/a (aborts on the read). Stale reference: the first
`sb_out_chunk` wrapper → freed `cio_chunk`, charge retained. Evidence:
`evidence/pf-fault.asan.txt`, `evidence/pf-*.pf.txt`, `gdb/pf-*.gdb`.

## FB-AUDIT-03-LEDGER-BOUND — not attempted

Optional per the brief; skipped rather than rushed. Its discharge condition stays
open in `followups.json`.

## Boundaries

Both results are injected-premise observations on a locally built, unmodified-source
binary: a gdb-forced return value and a gdb-forced allocation failure. The hang
(observed under a watchdog) and the use-after-free (observed under ASan) are local
observations of the two named candidates under those premises. They are not upstream
defect claims and imply no production failure rate. No Quirin `Unknown` becomes
`Safe`/`Unsafe` from any of this.
