# Cycle 19: native residency, transaction and lock transitions

Five fresh-process native sequences bind ChunkIO filesystem state to allocation
generations, reciprocal rings, descriptor/mapping epochs, logical content,
mapped header length and original inode identity. All 50 allocations are
released (five temporary, 45 parent teardown); all seven descriptor/mapping
acquisitions are released. All five runs complete ASan/UBSan/leak checking.

| Fixed sequence | Observed result |
|---|---|
| Down, repeated down, up, repeated up | First down/up succeeds; repeated calls return -1. Up reloads the same inode with a fresh descriptor/mapping epoch, even when numeric fd/address are reused. State-list membership and up counter change together. |
| Begin, repeated begin, append, rollback, repeated rollback | Both begins succeed, append changes logical/header lengths from 4 to 7, rollback restores logical length 4 but leaves header length 7; repeated rollback returns -1. |
| Begin, repeated begin, append, commit | Commit syncs, clears transaction-active and keeps seven content bytes. |
| Lock, repeated lock, begin, write, unlock, repeated unlock | Repeated lock/unlock return -1; transaction begin while locked returns -2 (CIO_RETRY); direct write still succeeds and appends three bytes while lock remains set. |
| Begin/append/rollback, then down/up | Rollback restores logical length 4 while header length remains 7. Down syncs/unmaps/closes; up reloads seven bytes (`seedadd`) from the mapped header. |

The last two outcomes are bounded research observations. They do not establish
production reachability or a validated upstream defect. In particular, logical
rollback cannot be replaced with a byte-restoration assumption, and the lock
flag cannot be imported as a universal write guard. The native source is compiled
without modification; the harness records the outcomes instead of repairing them.

The independent adapter checks all 26 operation results, complete intermediate
state, live allocation separation, OS identities/epochs, ring membership,
logical/header length differences, exact teardown, final bytes, process exit,
diagnostics and fixture cleanup. A positive relocation test allows a new mapping
address after reopen while mutation tests reject stale epochs. A 16-clause QCore
runtime policy retains the observation boundary. Six Lean theorems cover an
executable bounded projection of begin/append/rollback/down/reload, including the
four-to-seven rollback/reload example. Successful native syscalls, fixed buffer
capacity and the observer/adapter remain external premises to that model.

```sh
python3 scripts/cycle19_file_transition_observations.py --fluent-bit /path/to/pinned/fluent-bit --output /tmp/cycle19-transitions
python3 -B -m unittest discover -s scripts/tests -v
```

`metadata.json` binds the pinned source/build, dependencies, sanitizer settings,
observer, adapters and artifacts. `validation.json`, compressed gate logs and
`parity.json` record the standing checks and unchanged pinned audit outputs.

Result boundary: artifact = recorded native executable and pinned ChunkIO
sources; property = observed state/identity conservation and exact operation
outcomes; semantic profile = recorded 64-bit Linux build with checksum and trim
disabled; scope = five exclusive one-file sequences; bounds = one stream/chunk,
at most two open/map epochs, three appended bytes, no injected syscall failure
or concurrency; assumptions = trusted compiler/observer, complete selected
wrappers, libc/probes and exclusive fixture execution; evidence version = trace
schema 1 plus SHA256SUMS. Successful MS_ASYNC is not a durability proof. This is
runtime evidence, not a static Safe/Unsafe certificate.

Failed transition syscalls and sync/commit retries, capacity limits, checksum and
metadata variants, arbitrary callers, full synchronization/durability, aggregate
source custody, remaining Fluent Bit wrapper lifecycle and full Monkey/Cycle 18
composition remain open. Cycle 19 remains in progress and no raw unit is promoted.
