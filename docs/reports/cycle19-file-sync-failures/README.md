# Cycle 19: native synchronization failure and retry observations

Three fresh native processes suppress exactly one `msync` call, returning EIO
without executing that syscall. The pinned ChunkIO sources remain unchanged.
All 30 allocation generations are released (three temporary, 27 parent teardown),
and all four descriptor/mapping epochs are released. ASan, UBSan and leak
checking pass on all three processes.

| Fixed sequence | Observed result |
|---|---|
| Begin, append, failed commit, retry, repeated commit | Failed commit returns -1, retaining transaction-active, checkpoint 4, logical/header length 7, mapping and descriptor. Retry succeeds, sets synced and clears transaction-active. Repeated commit succeeds without another sync. |
| Begin, append, failed commit, rollback, commit | Rollback still succeeds after failed commit, restores logical length 4 and retains mapped header length 7. Commit with no active transaction syncs successfully, leaving the logical/header distinction intact. |
| Begin, append, down with failed sync, commit while down, up | Down succeeds despite the suppressed sync, unmaps/closes and moves the chunk to the down ring, retaining transaction-active and synced=false. Commit on the unmapped chunk succeeds and clears transaction-active without a sync syscall; synced remains false. Up opens/maps a new epoch and reloads seven bytes. |

The third sequence prevents importing “commit succeeded” as proof that an OS sync
occurred. The first prevents importing failed commit as transaction abort. File
bytes remain `seedadd` with header length 7 in all three fixtures. Observed bytes
following unmap/reopen do not prove stable-storage durability or behavior under
power loss. A suppressed syscall is an injected failure premise, not evidence
that arbitrary kernel failures preserve the same post-state.

The strict independent adapter checks all 15 operation results, suppression
arming/consumption, exact EIO diagnostics, every intermediate allocation/ring/
counter/content snapshot, OS identities and epochs, parent release order, final
inode/bytes and process completion. Seventy-three trace/process mutations reject.
The 18-clause QCore runtime policy keeps static-certificate and durability fields
false. Five Lean theorems extend the executable projection with conditional
commit/sync/down behavior; successful unmap, size-update and close are explicit
premises, and the projection is not a source refinement proof.

```sh
python3 -B scripts/cycle19_file_sync_failure_observations.py --fluent-bit /path/to/pinned/fluent-bit --output /tmp/cycle19-sync-failures
python3 -B -m unittest discover -s scripts/tests -v
```

LeakSanitizer needs an environment without ptrace containment. `metadata.json`
binds the pinned source/build, dependencies, sanitizer environment, observer,
adapters and artifacts. `validation.json`, compressed logs and `parity.json`
record all standing checks and unchanged prior pinned source/verdict outputs.

Result boundary: artifact = recorded native executable and pinned ChunkIO
sources; property = observed sync-failure state/identity conservation and exact
operation outcomes; semantic profile = recorded 64-bit Linux build, checksum and
trim disabled, MS_ASYNC; scope = three exclusive one-file sequences; bounds =
one stream/chunk, at most two open/map epochs, three appended bytes and exactly
one suppressed msync EIO per process, no concurrency; assumptions = trusted
compiler/observer, selected wrappers, libc/probes and exclusive fixtures; evidence
version = trace schema 1 plus SHA256SUMS. This is runtime research evidence,
not a static Safe/Unsafe certificate. No raw unit is promoted.

Failed unmap/close/reopen transitions, other sync failure points, capacity,
metadata/checksum variants, arbitrary production callers, full synchronization/
durability, aggregate source custody, remaining wrapper lifecycle and full
Monkey/Cycle 18 composition remain open. Cycle 19 remains in progress.
