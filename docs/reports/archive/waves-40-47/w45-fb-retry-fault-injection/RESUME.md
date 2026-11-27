# Resume — W45-FB-RETRY-FAULT-INJECTION

Read `README.md` and `evidence.json` first. This card produced native
observations only; no analyzer/contract/Lean change and no verdict move.
Before relying on anything, recheck:

- the Fluent Bit pin: `git -C /home/cosmo/Documents/GitHub/fluent-bit
  rev-parse HEAD` must be `864c3d97f3589515285a19396ed8dfc85cda08ae`;
- the binary hash: `sha256sum
  /home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w45-fb-retry-fault-injection/attempt-1/build/bin/fluent-bit`
  must be `fc5ab4b5ac970ac3f22b833c4b9d89be00ff2fa7bc280995b4abf783b2b87d33`.

## What exists

- Published: `runs/*.log` (full gdb+program output for every control/fault
  run), `runs/*.cmd` (exact command, PIDs, SIGTERM timestamp), `runs/*.exit`
  (wrapper exit code), `runs/gdb/*.gdb` (the exact scripts run, including the
  fixed C3 fault script and the discarded first attempt's log for the
  record).
- Not published (private, worktree-local, gitignored): the run harness
  scripts (`private/w45-fb-retry-runs/run_one.sh` and the individual
  `gdb/*.gdb` sources they were copied from) and the raw per-run `logs/`
  directory, byte-identical to the published copies (matching SHA-256 in
  `evidence.json`).
- The Fluent Bit binary and build logs referenced above were prepared by an
  earlier step, outside this worktree, under the repository's `private/`
  tree; this card never rebuilt them and never wrote to the read-only
  `/home/cosmo/Documents/GitHub/fluent-bit` checkout.

## To re-run (from this worktree)

1. Verify the binary hash above.
2. For each candidate, run gdb in batch mode against the same command line:
   `-i dummy -p rate=1 -o retry -p retry=3 -m '*' -f 1 -v`, using the script
   under `runs/gdb/`.
3. Resolve gdb's real PID as the child of the launching `timeout` PID, and
   the inferior's PID as the child of gdb's PID (do not use `$!` from
   `timeout ... &` directly — that is `timeout`'s PID, not gdb's; see
   `evidence.json`'s `harness_bug_found_and_fixed` for what goes wrong if you
   do). Send `kill -TERM` only to the inferior's PID, never by pattern.
4. Wait for `[engine] caught signal (SIGTERM)`, the grace-period sequence,
   and `[engine] service has stopped (...)` in the log before treating the
   run as complete.

## Open

`FB-STALE-COMPLETION-REACHABILITY` (C4) was not attempted here, unchanged
from the run guide's scope. The semantic card `FB-RETRY-LIFETIME` stays
open — these are native observations under injected premises, not a
Rust/Lean implementation or a production reproduction.
