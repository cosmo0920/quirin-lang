# Cycle 19: constructor OS failures and rollback custody

Eight fresh-process executions bind the pinned native filesystem constructors to
OS-call arguments, outcomes, allocation generations, reciprocal rings and exact
parent teardown. Wrappers forward real calls to exclusive Linux fixtures except
for ten selected suppressed calls. These injections establish their recorded
post-states, not general kernel failure semantics.

| Scenario | Observed constructor and cleanup outcome |
|---|---|
| Control | File chunk returned; descriptor/mapping and all allocations released during parent teardown |
| Open returns EACCES | Null chunk, error -1; no descriptor or file created; backend/path/name/chunk rolled back |
| First `fstat` returns EIO | Real fallback `stat` succeeds on the same inode; construction and teardown complete |
| First `fstat` and fallback `stat` return EIO | Null chunk, error -1; rollback closes descriptor; empty file retained |
| POSIX allocation returns ENOSPC | Null chunk, error -1; rollback closes descriptor; empty file retained |
| `mmap` returns MAP_FAILED/ENOMEM | Null chunk, error -1; rollback closes descriptor; resized file retained |
| Mapping failure followed by close returning EIO | Null chunk, error -1; all C allocations released, but original descriptor/inode remains live after constructor return and parent release |
| Second `fstat` and fallback `stat` return EIO | Null chunk, error -1 at the later size lookup; rollback closes descriptor; empty file retained |

All 88 acquired allocation generations are released: 16 temporary stream-path
releases, 24 constructor rollback releases and 48 parent-teardown releases. Six
failed constructors return no linked chunk; the surviving stream/context still
requires teardown. The backend's MAP_FAILED sentinel and nonzero size bookkeeping
are preserved on map failure, without treating the sentinel as a live mapping.

The residual descriptor case records the same descriptor number and device/inode
at acquisition, after null chunk return and after all enclosing allocations are
freed. The harness then records a separate successful close and EBADF probe.
External process records bind the surviving pathname to that same inode before
removing the exclusive fixture. Neither a null constructor result nor an empty
allocation ledger implies that OS custody has been discharged.

The pinned profile defines `CIO_HAVE_POSIX_FALLOCATE` without Linux `fallocate`.
Native strategy selection first sets errno to EOPNOTSUPP and then calls
`posix_fallocate`. The injected allocation failure returns positive ENOSPC (28)
without changing errno (95). Both subsequent native diagnostics therefore print
EOPNOTSUPP. The checker preserves the result and errno independently, including
the enclosing constructor's error -1. It does not reinterpret the positive
POSIX result as success or rewrite errno to match the return value.

All fourteen selected ChunkIO/CRC translation units and the observer are rebuilt
with ASan/UBSan. All eight runs complete leak checking outside ptrace. The
independent adapter checks the complete ordered trace, exact allocation plan,
OS identities and argument/result/errno records, rollback backend state, ring
membership and unlinking, counter discharge, residual probes and separate
harness reclamation. Three Python tests contain 60 trace/process mutations.
A 17-clause QCore runtime policy and four Lean rollback-ledger theorems retain
the separation between allocation release, null returns and descriptor custody.
The Lean account does not prove the C observer, Python checker, compiler or OS.

Reproduce from the repository root:

```sh
python3 -B scripts/cycle19_file_os_constructor_observations.py --fluent-bit ../fluent-bit --output /tmp/cycle19-file-os-constructor
python3 -B -m unittest discover -s scripts/tests -v
```

All standing checks pass: 407 workspace tests, 51 contract modules, formatting,
Clippy with warnings denied, both reevaluation probes, two audit-example tests,
21 Python tests, Lean build and the Cycle 19 axiom audit. Pinned source and all
three diagnostic/contract inventories are byte-identical to the previous report.
Across 185 units, verdicts remain 2 Safe, 98 provisional Unsafe and 81 Unknown,
with four data-only units and zero gateway failures.

The recorded 64-bit Linux ABI, fixed empty one-file constructor options, one-page
mapping, POSIX allocation strategy and diagnostic profile are explicit bounds.
The observer/compiler, complete wrapping of selected calls, libc, native probes,
source/build metadata and exclusive fixture execution remain trusted. Existing
observers and their adapters are unchanged.

Arbitrary callers/options, ACL failures, existing/corrupted files, down/up and
transaction transitions, other combined OS failures, full sync/durability,
aggregate source custody, remaining wrapper lifecycle and full Monkey/Cycle 18
composition remain open. These are bounded observations, not static `Safe` or
`Unsafe` certificates or claims about production reachability. Cycle 19 remains
in progress and no raw unit is promoted.
