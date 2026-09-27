# Leaf-worker Buffer allocation and CT16 cleanup

Baseline `376f102`, 2026-09-07. `ScopedOwnedTasksV1` now permits nonrecursive,
task-free leaf workers to allocate actual heap Buffers, return a fresh Buffer,
or release their local allocations before returning or raising a payload-free
error. CT16 now exercises these operations in a real Linux C11 child. Its earlier
sequential implicit-cleanup fixture remains a separate executed control.

## Ownership and identity

The checker derives each worker exit from its complete owned inventory: captures
plus successful local allocations. An owned result is either an original capture
or a fresh local. Every other resource must be dropped. Scalar, unit and error
exits must drop the entire inventory. A fresh result is acquired under child
custody in the caller's abstract ledger, retained in the completion record, then
transferred through explicit join and affine Result consumption. A result type
annotation alone cannot manufacture a resource. Creation failure continues to
restore original captures and preserve earlier siblings.

Each native task has a private successful-allocation ordinal. The resource
identity is `task * 2^32 + ordinal`, with task in `0..2^32-1` and ordinal in
`1..2^32-1`. Task zero is the root, preserving its existing IDs. Failed allocations
consume an attempt but no generation. These are lifetime identities, not memory
addresses. No shared allocation counter is introduced, and a worker's events
remain private until join. The existing finite DAG checker bounds supported
execution; the native runtime also aborts before exhausting either namespace.

`execute_task_module_with_worker_allocations` accepts optional decisions keyed
by successful runtime task identity. Root decisions stay in the existing
`TaskExecutionScript`; absent workers, root keys and unused worker decisions
reject. Missing decisions default to successful model allocation. The optional
native `QCORE_BUFFER_MALLOC_FOR_TASK(size, task, attempt)` hook selects the same
worker/attempt independently of thread scheduling, while its default preserves
the existing `QCORE_BUFFER_MALLOC` hook. Fault scripts are execution controls,
not proof of arbitrary allocator behavior.

## Executed checks

CT16's worker allocates two Buffers, releases both, then raises
`WorkerError::Failed`. Failure of the second allocation releases the first before
raising the bound `AllocError`; failure of the first acquires nothing. The creator
joins before collecting each worker error. Failed creation starts no child.
Negative source controls omit cleanup from either error arm and reject with Q600.
The fixture header checks real allocation pointers, thread handles, ordered
releases, join-before-environment-free, and zero residuals at exit.

| Sanitized scenario | Observed root result | Acquired Buffers |
|---|---|---|
| CT16 worker typed error | `WorkerError::Failed` | 2, both released in worker |
| CT16 first allocation fails | `AllocError` | 0 |
| CT16 second allocation fails | `AllocError` | 1, released in worker |
| CT16 creation fails | `SpawnError` | 0 |
| Fresh worker result | `I32 42` | 1, returned through join and released by parent |
| Two workers, reverse allocation order | `I32 42` | 2, returned and released by parent |

All six fixed processes pass ASan, UBSan and LSan, run untraced because LSan
cannot operate under the traced sandbox. The last adapter forces task 2 to
allocate before task 1; their generations remain 8589934593 and 4294967297.
Generated C, fixture inputs, compile commands, executable hashes and exact
stdout/stderr are archived in `native-runs.json` and adjacent compressed files.

The Rust tests compare actual source → checked task IR → generated native C
observations byte-for-byte with reference execution. Additional tests cover
returning a fresh local on allocation success versus the original capture on
failure, releasing a prior local on either allocation error, missing alloc
effects, discarded results, omitted cleanup, invalid scripts and identity bounds.
The old blanket rejection of even fully cleaned worker allocations is replaced
with acceptance plus rejection of an unclean uncalled worker definition.

Fourteen new Lean theorems extend the owned-task model to account for captures
and allocated locals, disallow aliasing between those inventories, require
cleanup of unreturned locals, and establish bounded identity injectivity and
per-task ordinal behavior. Complete inventory extraction and the relationship
to Rust/source/native execution remain explicit premises. This is model proof,
not a full compiler refinement theorem. All 20 Lean audit modules pass with no
project axioms or placeholders.

Reproduce the main checks:

```sh
cargo test -p qcore-compiler --test owned_source_tasks --test ct_corpus
cargo test -p qcore-frontend --test owned_tasks
cargo test --workspace
cargo run --quiet -p qcorec -- emit -o /tmp/qcore-ct16.c \
  compiler/tests/fixtures/ct/CT16/case.qr
clang -std=c11 -pthread -Wall -Wextra -Werror -g -O1 \
  -fsanitize=address,undefined -fno-omit-frame-pointer \
  -DQCORE_CT_ALLOC_TASK=1 -DQCORE_CT_FAIL_ALLOC=2 -DQCORE_CT_FAIL_CREATE=0 \
  -include compiler/tests/fixtures/ct/CT16/worker_allocation.h \
  /tmp/qcore-ct16.c -o /tmp/qcore-ct16
ASAN_OPTIONS=detect_leaks=1 /tmp/qcore-ct16
```

## Status and limits

All **775 workspace tests**, **31 provider tests**, **43 Python tests**, format,
Clippy, both reevaluation probes, Lean build and **20 Lean audits** pass. The
58-module contract corpus is unchanged. Full 185-unit Cycle19 source,
diagnostics, chunk-contract and source-effect artifacts are byte-identical to
the baseline: **2 Safe /42 provisional Unsafe /137 Unknown**, plus four data-only
units; source effects remain **3 checked /93 Unknown**. No raw unit is promoted.

CT coverage remains **9/50 mapped, 41 unmapped**. CT16 now executes parser,
checker, reference, CFG and Linux native stages; its source proof stage remains
unassessed. Other platforms explicitly retain unexecuted native-stage status.
This supports one concrete immutable Buffer type, explicit cleanup, finite leaf
workers and payload-free errors. General implicit worker cleanup, loans, owned
error payloads, general owned graphs, shared memory, mutexes, nested workers,
arbitrary native failure behavior and other-platform execution remain open.
The broader W6 and W2–W7 goals remain active. Existing subagents reported usage
limits, so this increment continued locally without claiming new parallel work.
