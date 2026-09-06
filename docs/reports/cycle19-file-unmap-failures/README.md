# Cycle 19: failed unmap during down and subsequent recovery

Four fresh native processes suppress one `munmap` call with EIO. Two also
suppress the immediately preceding `msync` with EIO. Suppressed calls do not
execute the real syscall; all other selected calls forward normally. The pinned
ChunkIO source bodies remain unchanged. ASan, UBSan and leak checking pass.
All 40 allocations are released (four temporary and 36 parent teardown), along
with all seven descriptor/mapping epochs.

| Fixed sequence | Observed result |
|---|---|
| Clean chunk, failed down, up, retry down, up | Failed unmap returns -1 from down while retaining the original map, descriptor, four content bytes, up ring and up counter. Up returns -1 because the chunk remains mapped. Retried down releases the original epoch; up acquires a fresh epoch. |
| Append, successful sync then failed unmap, retry down/up | Failed down retains seven bytes and all OS identities, but synced changes from false to true before the unmap failure. Retry does not issue another sync. |
| Append, failed sync then failed unmap, retry down/up | Both failures leave the chunk mapped and unsynced, with seven bytes and its original OS epoch. Retry syncs successfully before unmap/close, then up reloads seven bytes in a fresh epoch. |
| Append, failed sync then failed unmap, parent teardown | The same live mapping/descriptor and up membership survive the failed down and rejected up. Parent teardown later syncs, unmaps and closes that original epoch before releasing enclosing allocations. |

Failed down is therefore not an unconditional state rollback: a preceding sync
may already have succeeded. Failure also cannot imply down-ring membership or
discharge of mapping custody. Recovery here is limited to the selected successful
retry or teardown calls, not arbitrary later callers or kernel failures.

The independent adapter checks 17 public operation results, explicit suppression
arming/consumption, ordered native EIO diagnostics, concrete allocation separation,
original mapping/descriptor epochs, every ring and counter snapshot, logical and
header lengths, byte content, exact parent cleanup order and process completion.
Seventy-five trace/process mutations reject; a positive relocation test allows a
new mapping address only after releasing the old epoch. A 19-clause runtime
QCore policy retains the observation boundary. Five Lean theorems model failed
unmap retaining the file state, prior successful sync surviving failure, combined
failure preserving state, and retry releasing the same epoch. The successful
arm assumes successful size update and close. These conditional projections
are not a source refinement proof.

```sh
python3 -B scripts/cycle19_file_unmap_failure_observations.py --fluent-bit /path/to/pinned/fluent-bit --output /tmp/cycle19-unmap-failures
python3 -B -m unittest discover -s scripts/tests -v
```

LeakSanitizer requires execution without ptrace containment. `metadata.json`
binds source/build identities, instrumentation, drivers and adapters; compressed
gate logs, `validation.json` and `parity.json` record passing checks and unchanged
prior pinned source/verdict outputs. `SHA256SUMS` binds the evidence package.

Result boundary: artifact = recorded native executable and pinned ChunkIO
sources; property = observed failed-down state/identity conservation and exact
recovery outcomes; semantic profile = recorded 64-bit Linux, checksum and trim
disabled, MS_ASYNC; scope = four exclusive one-file sequences; bounds = one
stream/chunk, at most two open/map epochs, at most three appended bytes, exactly
one suppressed unmap per process and one suppressed sync in two processes, no
concurrency; assumptions = trusted compiler/observer, selected wrappers,
libc/probes and exclusive fixtures; evidence version = trace schema 1 plus
SHA256SUMS. Runtime evidence does not establish a static Safe/Unsafe certificate,
arbitrary OS failure post-states or stable-storage durability. No raw unit is
promoted and Cycle 19 remains in progress.

Failed close/reopen transitions, other sync failure points, capacity and metadata/
checksum variants, production caller provenance, full synchronization/durability,
aggregate source custody, remaining Fluent Bit wrapper lifecycle and full
Monkey/Cycle 18 composition remain open.
