# Resume — W45-FB-BACKLOG-FAULT-INJECTION

Read `README.md`, `evidence.json` and `followups.json`. This card produced native
observations only; no analyzer/contract/Lean change and no verdict move. Recheck the
Quirin base (`6bd9b383`) and the Fluent Bit pin
(`git -C /home/cosmo/Documents/GitHub/fluent-bit rev-parse HEAD` must be
`864c3d97f3589515285a19396ed8dfc85cda08ae`) before relying on anything.

## What exists

- Private build/run tree (not in the worktree; delete-safe):
  `/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w45-fb-backlog-fault-injection/attempt-01/`
  with `fluent-bit/` (clone), `build/` (Debug), `build-asan/` (Debug+ASan),
  `conf/`, `gdb/`, `run/storage-pristine/`, `logs/`. Binary hashes and full-log
  hashes are in `evidence.json`; the small published excerpts are under `evidence/`.
- Published: `gdb/*.gdb`, `evidence/*.conf`, `evidence/chunk-*.txt`,
  `evidence/zp-*.wd.txt`, `evidence/pf-*.{pf,asan}.txt`.

## To re-run (from the private attempt dir)

1. `rm -rf run/storage && cp -a run/storage-pristine run/storage` before every run.
2. Zero-progress fault: `timeout -k 5 90 gdb -batch -x gdb/zp-fault.gdb --args ./build/bin/fluent-bit -c ./conf/phase2-backlog.conf`.
   Controls: `gdb/zp-control.gdb` (valid head), `gdb/zp-corrupted.gdb` (`CIO_CORRUPTED`).
3. Partial fan-out (ASan): `export ASAN_OPTIONS=detect_leaks=0:abort_on_error=1:halt_on_error=1`
   then `timeout -k 5 25 gdb -batch -x gdb/pf-fault.gdb --args ./build-asan/bin/fluent-bit -c ./conf/phase2-backlog.conf`.
   Control: `gdb/pf-control.gdb`.
4. Regenerate the fixture from scratch with `conf/phase1-generate.conf` (a `dummy`
   input, two closed-port `tcp` outputs) under a `timeout` watchdog; then snapshot
   `run/storage` to `run/storage-pristine`.

Kill only processes you started, by captured PID; Fluent Bit ignores SIGTERM during
retries, so use `timeout -k` (SIGKILL escalation). There may be an unrelated
`fluent-bit-dev` process from another session — never touch it.

## Open

`FB-AUDIT-03-LEDGER-BOUND` (P2, boundary at `F+B+P=L`/`L+1`, `SIZE_MAX`/`SSIZE_MAX`,
map truncation) was not attempted. `FB-AUDIT-03-CROSS-OUTPUT-EVICTION` (P2) also
remains. The semantic cards `FB-BACKLOG-CUSTODY`, `FB-BACKLOG-ACCOUNTING` and
`FB-ENGINE-PROGRESS` stay open — a native observation does not discharge a Rust/Lean
implementation card.
