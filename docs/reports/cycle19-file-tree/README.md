# Cycle 19: native filesystem construction and residual teardown

Six pinned native one-stream/one-file executions bind constructors, reciprocal
rings, allocation generations, filesystem identities and exact nested release
order. Two context-destroy cases use an empty asynchronous file and a written
full-sync file. A third explicitly closes/deletes a written file before parent
teardown. Three more suppress one native syscall and return EIO: unmap, close or
sync. Other observed syscalls execute unchanged against exclusive `/tmp` fixtures.

Every case releases all eleven allocation generations: two temporary stream-path
allocations during construction and nine during teardown. Across six cases this
is 66 generations, with 12 temporary and 54 teardown releases. The adapter checks
the fixed constructor allocation sequence, returned objects/error/write result,
the complete one-child ring graph and exact source-function/OS-call/free nesting.
It checks backend bookkeeping before release, both embedded list removals, total
and up counters, empty parents and final descriptor/mapping/pathname probes.
The descriptor and pathname initially reference the same device/inode; retained
files have that same identity after teardown. Probe arguments retain the original
descriptor and mapping identities even after their C owners have been released.

The failure outcomes remain distinct:

- Suppressed unmap leaves a live mapping and an up-counter of one through backend,
  chunk, stream and context release; the descriptor closes successfully.
- Suppressed close leaves a live descriptor after all allocations are released;
  mapping cleanup and both counters complete.
- Suppressed sync leaves the backend's synced field false, but the enclosing
  source still unmaps, closes and releases its allocations. Cleanup cannot imply
  successful sync or durability.

After recording these residuals, the harness separately releases the retained OS
resources and deletes only its own fixture files/directories. Injection does not
establish that a real kernel EIO has the same post-state. No general close-failure
custody rule, crash-consistency claim or upstream defect is inferred.

All fourteen selected ChunkIO/CRC units and the observer are rebuilt with ASan
and UBSan; leak checking passes outside ptrace. Stderr contains the three expected
injected EIO diagnostics. The observer, allocator/function/syscall instrumentation,
compiler, Linux probes and exclusive harness execution remain trusted. The adapter
deliberately pins the recorded 64-bit Linux ABI and one-page mapping; different
layouts, scenarios, source profiles or allocation plans reject.

A 16-clause runtime QCore policy checks derived observations. Two new Python tests
cover the six cases and 40 trace mutations. Four additional Lean theorems model
the final independent allocation-release and OS-residual ledgers; they do not
prove native source, Python or instrumentation. The shared observer's optional
filesystem hooks are inactive in its existing memory profiles. All five positive,
ten allocation-failure/control and four unsupported-constructor native regressions
still pass their independent adapters and ASan/UBSan/leak checks.

Reproduce from the repository root:

```sh
python3 -B scripts/cycle19_file_teardown_observations.py --fluent-bit ../fluent-bit --output /tmp/cycle19-file-tree
python3 -B scripts/cycle19_teardown_observations.py --fluent-bit ../fluent-bit --output /tmp/cycle19-file-tree-positive
python3 -B scripts/cycle19_constructor_failure_observations.py --fluent-bit ../fluent-bit --output /tmp/cycle19-file-tree-failure
python3 -B scripts/cycle19_unsupported_constructor_observations.py --fluent-bit ../fluent-bit --output /tmp/cycle19-file-tree-unsupported
python3 -B -m unittest discover -s scripts/tests -v
```

All standing checks, both reevaluation probes, two audit-example tests, Lean
build/dependency audit and pinned external audit pass: 407 workspace tests,
49 contract modules and 15 Python tests. Prior source/diagnostic/contract evidence
is byte-identical. Aggregate verdicts remain 2 Safe, 98 provisional Unsafe,
81 Unknown, four data-only units and zero gateway failures across 185 units.

Cycle 19 remains in progress. General filesystem constructor/failure combinations,
arbitrary callers, full sync/OS guarantees, aggregate source custody, remaining
wrapper lifecycle and full Monkey/Cycle 18 event composition remain open. No raw
unit is promoted.
