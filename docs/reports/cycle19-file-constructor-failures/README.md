# Cycle 19: filesystem constructor allocation failures

Twelve fresh-process Linux executions inject one ENOMEM at each of eleven
allocations on the pinned one-stream/one-file constructor path, plus a control.
The observer records the immediate context and stream results, allocation
attempts and generations, temporary and rollback frees, surviving reciprocal
rings, exact nested teardown and original descriptor/mapping/inode identities.
The external driver records each process result and the remaining fixture tree
before removing only its own temporary directory.

| Failed allocation | Observed outcome |
|---|---|
| None | File chunk returned; complete allocation, mapping and descriptor teardown; pathname retained |
| 1: context | Null context; no acquired generation |
| 2: chmod copy | Context and file chunk returned with null chmod; complete teardown |
| 3: root-path copy | Context returned with null root; UBSan stops `check_stream_path` at `cio_stream.c:65` before stream return |
| 4: temporary stream path | Null stream; context teardown completes |
| 5: mkdir path copy | Stream returned without its directory; file open fails with ENOENT, null chunk and error -1; rollback and parent teardown complete |
| 6: stream | Null stream; temporary paths and context released |
| 7: stream name | Stream allocation rolled back; context teardown completes |
| 8: chunk | Null chunk and unchanged error sentinel -999; parent teardown completes |
| 9: chunk name | UBSan stops `cio_file_native_filename_check` at `cio_file_unix.c:382` before chunk return |
| 10: file path | Null chunk, unchanged error sentinel; chunk/name rollback and parent teardown complete |
| 11: file backend | Null chunk, unchanged error sentinel; path/name/chunk rollback and parent teardown complete |

Across all prefixes, 81 allocation generations are acquired: 17 temporary
releases, 10 rollback releases and 46 teardown releases are recorded. The two
interrupted cases retain eight outstanding prefix generations (two for failed
root copying and six for failed chunk-name copying). The driver never turns
process exit into C release events or invents a terminal allocation census for
these cases. It observes their filesystem entries only after process exit;
this does not establish their descriptor/mapping state before termination.

All fourteen selected ChunkIO/CRC units and the observer are rebuilt with ASan
and UBSan. Ten returning cases complete leak checking outside ptrace. The two
interrupted cases have exact checked UBSan null-argument diagnostics, native
caller stacks and exit status 1; they **do not pass** sanitizer/leak checking or
establish completed teardown. These are bounded injected-failure observations,
not static `Unsafe` certificates or claims about normal production reachability.
The pinned upstream checkout is unchanged.

The independent adapter checks the complete ordered trace, canonical scalar
values, fresh nonoverlapping allocations, separate temporary/rollback/teardown
ledgers, exact constructor results, ring graph, counter and unlink states, native
OS results, and fixture inode continuity. A 16-clause runtime QCore policy checks
the derived counts and completion boundaries. Three Python tests include 55
trace/process mutations. Four Lean theorems model conservation and the distinction
between interrupted prefixes and complete teardown; they do not prove the C
observer, Python checker, compiler or OS. The shared filesystem observer's main
guard is the only change to the prior harness, and its six native regression
cases pass their original adapter with ASan/UBSan/leak checking.

Reproduce from the repository root:

```sh
python3 -B scripts/cycle19_file_constructor_failure_observations.py --fluent-bit ../fluent-bit --output /tmp/cycle19-file-constructor-failures
python3 -B scripts/cycle19_file_teardown_observations.py --fluent-bit ../fluent-bit --output /tmp/cycle19-file-constructor-regression-tree
python3 -B -m unittest discover -s scripts/tests -v
```

All standing checks pass: 407 workspace tests, 50 contract modules, formatting,
Clippy with warnings denied, both reevaluation probes, two audit-example tests,
18 Python tests, Lean build and the Cycle 19 axiom audit. Pinned source and all
three diagnostic/contract inventories are byte-identical to the prior report.
Across 185 units, verdicts remain 2 Safe, 98 provisional Unsafe and 81 Unknown,
with four data-only units and zero gateway failures.

The profile requires the recorded 64-bit Linux ABI, one-page mappings, Clang
sanitizer diagnostic format and exclusive temporary fixtures. The compiler,
allocator/function/syscall instrumentation, libc and Linux probes, external
process census and source/build metadata remain trusted. It does not cover
multiple failures, arbitrary constructor options/callers, native OS failure
combinations, full sync/durability, aggregate source custody, remaining wrapper
lifecycle or full Monkey/Cycle 18 event composition. Cycle 19 remains in progress;
no raw unit is promoted.
