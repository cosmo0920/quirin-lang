# Cycle 19: existing-file profiles and rollback custody

Seven fresh-process native executions bind existing ChunkIO files to allocation
generations, reciprocal rings, descriptor/mapping epochs, inode identities, byte
content and exact rollback or parent teardown. All 70 allocation generations are
released: seven temporary, 20 rollback and 43 parent-teardown releases. All seven
runs complete ASan/UBSan/leak checking.

| Fixed profile | Observed result |
|---|---|
| Valid header and `seed` payload, read/write | Returns mapped chunk with four content bytes; parent teardown releases mapping and descriptor |
| Suppressed chmod returns EPERM | Null chunk, public error -1 and chunk-local error 0; rollback closes descriptor |
| Suppressed chmod EPERM and rollback close EIO | Same null/error result, but original descriptor/inode remains live after parent release; harness reclamation is separate |
| Invalid magic byte | Null chunk, public error -3 and chunk-local error -11; temporary mapping and descriptor released |
| Eight-byte short header | Null chunk, public error -3 and chunk-local error -13; eight-byte mapping and descriptor released |
| Empty file, read-only | Null chunk, public error -3 and chunk-local error -12; no mapping acquired |
| Valid header and `seed` payload, read-only | Returns chunk mapped with PROT_READ; parent teardown releases mapping and descriptor |

Every case retains its original inode and complete byte content. Read-only open
still applies the configured chmod in this profile. The two suppressed chmod
calls preserve mode 0600; the other calls set 0700. These observations say
nothing about arbitrary filesystem ACLs or kernel close-failure post-states.

The independent Python adapter checks exact events, live storage separation,
allocation and OS identity ledgers, errors, content, list topology, nested
release order, process diagnostics and fixture cleanup. A 17-clause runtime
QCore policy checks the adapted observations; four Lean theorems preserve the
specific error boundary, bytes/identity and non-promotion. The shared replay
primitives do not alter existing adapters or source verdicts.

Reproduce from the repository root with:

```sh
python3 scripts/cycle19_file_profile_observations.py --fluent-bit /path/to/pinned/fluent-bit --output /tmp/cycle19-existing
python3 -B -m unittest discover -s scripts/tests -v
```

The source revision, compile entries, transitive dependencies, compiler, build
commands, sanitizer settings and artifact hashes are in `metadata.json`. The
checker rejects process failure and unexpected diagnostics, including incomplete
LeakSanitizer execution. Native runs require a host where sanitizer process
inspection is permitted. `validation.json`, compressed logs and `parity.json`
record the standing checks and unchanged pinned audit outputs.

Result boundary: artifact = the recorded native executable and pinned ChunkIO
sources; property = observed allocation/OS custody and existing-file outcomes;
semantic profile = recorded 64-bit Linux build; scope = seven exclusive one-file
fixtures; bounds = one stream, one attempted chunk, at most one mapping, no
concurrency; assumptions = trusted compiler/observer, complete selected wrappers,
libc/probes and exclusive fixture execution; evidence version = trace schema 1
plus SHA256SUMS. This is runtime evidence, not a static Safe/Unsafe certificate.

Arbitrary caller provenance, other corruption/ACL/OS-failure combinations,
residency and transaction transitions, full synchronization/durability,
aggregate source custody, remaining Fluent Bit wrapper lifecycle and full
Monkey/Cycle 18 composition remain open. Cycle 19 remains in progress, with no
raw unit promoted and all prior Unknown boundaries retained.
