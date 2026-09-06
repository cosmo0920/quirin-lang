# Cycle 19: descriptor failures during down and reopen

Four fresh native processes exercise the pinned ChunkIO down/up/parent-teardown
chain on exclusive existing-file fixtures. Selected close or open calls are
suppressed with EIO; they do not execute the real syscall. All other selected
calls forward normally, and the pinned source bodies remain unchanged.
ASan, UBSan and leak checking pass. All 40 allocation generations are released
(four temporary and 36 parent teardown). Five descriptor/mapping epochs are
acquired: the library releases all five mappings and four descriptors. One
descriptor survives parent release and is reclaimed separately by the harness.

| Fixed sequence | Observed result |
|---|---|
| Suppress close during down; retry down/up; parent teardown | Down returns -1 after successful unmap, clears mapped lengths, decrements the up counter and moves to the down ring, but retains the original descriptor/inode. Retry down returns -1 because the map is absent; up returns -1 because the descriptor remains. Neither calls close/open. Parent teardown releases the original descriptor. |
| Same, also suppress parent close | Parent teardown releases all enclosing allocations while the original descriptor remains live. A post-parent probe binds its original inode. Only the separately recorded harness close releases it. |
| Successful down; suppress reopen; retry up | Failed open returns -1 without acquiring a new epoch and leaves the chunk down with no descriptor/map. Retry up opens/maps a new epoch and reloads the original four bytes; parent teardown releases it. Numeric descriptor and mapping-address reuse is allowed after the old epoch is released. |
| Successful down; suppress reopen; parent teardown | Failed up leaves the chunk down with no descriptor/map. Parent teardown releases enclosing allocations without another close syscall or counter decrement. |

Down-ring membership and a zero up counter therefore cannot discharge descriptor
custody. Failed down is not an unconditional state rollback. This complements
the [failed-unmap observations](../cycle19-file-unmap-failures/README.md), where
mapping, descriptor, up ring and counter all survive failure. A suppressed close
has a known retained endpoint; these fixtures do not infer the post-state of
an arbitrary kernel close error or establish a production-reachable defect.

Source correspondence at Fluent Bit revision
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97`:
`lib/chunkio/src/cio_file.c:920` unmaps and updates size before checking close;
`cio_file_unix.c:419` retains the fd field on close failure;
`cio_file.c:816` rejects up if a descriptor is already present;
`cio_chunk.c:556` synchronizes state-ring membership even after file-down failure;
`cio_file.c:963` attempts parent cleanup and releases the backend regardless of
close outcome. The recorded executable and trusted observer bind these fixed
native paths; the Lean projection is not a general source refinement proof.

The independent adapter checks 11 operation results, exact arming/consumption,
ordered native diagnostics, allocation separation, mapping and descriptor epochs,
reciprocal rings and counters, byte content, inode/endpoint probes, parent cleanup
order and process completion. Seventy-five trace/process mutations reject.
Positive relocation/reuse tests preserve epoch distinctions. A 20-clause runtime
QCore policy separates parent descriptor residuals from harness reclamation.
Seven Lean theorems cover failed-close down state, blocked retries, failed-open
state preservation, fresh successful reopen and parent descriptor residuals.
Successful size/unmap/map/header operations are premises of the modeled arms;
arbitrary failures, flags and callers remain outside them.

```sh
python3 -B scripts/cycle19_file_descriptor_failure_observations.py --fluent-bit /path/to/pinned/fluent-bit --output /tmp/cycle19-descriptor-failures
python3 -B -m unittest discover -s scripts/tests -v
```

LeakSanitizer requires execution without ptrace containment. `metadata.json`
binds the source/build, observer, driver, adapters and runtime policy. Compressed
gate logs, `validation.json` and `parity.json` record passing checks: 407 workspace
tests, 56 contract modules, 39 Python tests, formatting, Clippy, both reevaluation
probes, the pinned Fluent Bit audit and Lean build/axiom audit. All four prior
source/diagnostic JSON outputs compare equal; no project axioms or placeholders
were introduced. `SHA256SUMS` binds the evidence package.

Result boundary: artifact = recorded native executable and pinned ChunkIO
sources; property = observed descriptor custody across down/reopen failures and
parent teardown; semantic profile = recorded 64-bit Linux, checksum and trim
disabled, clean 4096-byte existing files; scope = four exclusive one-file
sequences; bounds = one stream/chunk, four content bytes, at most two open/map
epochs, one or two suppressed closes or one suppressed open per process, no
concurrency; assumptions = trusted compiler/observer, selected wrappers,
libc/endpoint probes and exclusive fixtures; evidence version = trace schema 1
plus SHA256SUMS. No static Safe/Unsafe certificate or durability claim is issued.
No raw unit is promoted and Cycle 19 remains in progress.

Remaining work includes reopen size/map/corruption failures, capacity and metadata/
checksum variants, production caller/constructor provenance, full synchronization
and OS failure semantics, aggregate source custody, remaining Fluent Bit wrapper
lifecycle and full Monkey/Cycle 18 event composition. The descriptor transition
model must retain these unresolved obligations when composed with those callers.
