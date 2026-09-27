# CT14 and CT37: completed children retain custody until join

Baseline `8876cd4`, 2026-09-07. CT14 is newly mapped to real source/native
conformance, and CT37 now runs a real worker allocation through a completion
record and parent join. No language, checker, core, native runtime, contract or
Lean semantics change is needed. This uses the existing owned-task implementation
and worker-local allocation support.

## The tested boundary

CT14 moves a Buffer containing 42 into its child. The worker returns that same
generation. The native adapter waits until the actual worker body has returned
and the runtime trampoline has written its completion record before letting
creation return to the parent. The accepted source then joins, consumes the owned
Result, reads 42 and drops the buffer. Two source controls instead read or drop
the original moved binding before join; both reject with Q600. Worker completion
alone therefore cannot restore that source binding's authority.

CT37's worker allocates its own Buffer containing 42 and returns it. The same
adapter forces completion before creation returns. At join entry, it checks that
the original allocation and environment remain outstanding. Join transfers the
worker's generation 4294967297 to the parent, which consumes and drops it. Source
controls that discard the owned Result or omit the returned buffer's drop reject
with Q600. The earlier sequential fixture still checks scratch release and the
exact transferred payload `[6]` in a separate test; that evidence is preserved.

The adapter invokes the real generated worker/trampoline on a real C11 thread.
A release/acquire flag announces the end of the worker body; this proves body
and completion-record completion, not that every native thread-exit instruction
has finished. It does not call join early. The parent's actual join checks the
exact thread handle and precedes both environment and Buffer reclamation.
Allocation/free hooks track the actual pointers and reject duplicate, mismatched,
premature or missing releases. At exit, both pointers must be discharged and the
child must have been joined. The adapter aborts on unexpected real allocation or
thread-creation failure and uses a bounded alarm to detect a stalled fixture.
These fixed runs do not characterize arbitrary native failures or schedules.

| Case | Ownership before worker | Completed, unjoined storage | After explicit join |
|---|---|---|---|
| CT14 | Parent Buffer generation 1, moved into child | Original Buffer plus completion environment | Generation 1 returned, read, dropped |
| CT37 | No Buffer in parent | Worker Buffer generation 4294967297 plus completion environment | Fresh generation returned, read, dropped |

The source checker validates all supported exits. The reference executes each
child eagerly and retains its owned payload until join. Generated native stdout
matches reference output byte-for-byte; the adapter's separate stderr proves that
the tested native worker body actually completed before the parent could join.
The early finish is forced by the test adapter, not asserted by QCore source.
Private worker events remain emitted at join under the existing observation
contract; this is not a general partial-order trace checker or a CT49 mapping.

## Evidence and reproduction

Both native cases also pass ASan, UBSan and LSan in untraced local Linux processes.
LSan requires untraced execution. Generated C, compile commands, executable hashes,
stdout/stderr and source/header hashes are archived in `native-runs.json` and
adjacent artifacts. The tiny per-case headers include the shared checked-in
`compiler/tests/fixtures/task_worker_allocations/completion_before_join.h`;
that shared input is archived and hashed as well.

```sh
cargo test -p qcore-compiler --test ct_corpus
cargo run --quiet -p qcorec -- emit -o /tmp/qcore-ct37.c \
  compiler/tests/fixtures/ct/CT37/case.qr
clang -std=c11 -pthread -Wall -Wextra -Werror -g -O1 \
  -fsanitize=address,undefined -fno-omit-frame-pointer \
  -include compiler/tests/fixtures/ct/CT37/completion_before_join.h \
  /tmp/qcore-ct37.c -o /tmp/qcore-ct37
ASAN_OPTIONS=detect_leaks=1 /tmp/qcore-ct37
```

The existing Lean model includes
`ScopedTasks.unjoined_capsule_is_unavailable_to_every_task` and
`OwnedTaskExecution.completed_owned_payload_is_inaccessible_to_every_task`.
They establish the model custody rule under checked-transfer premises. The source
fixtures' proof stages remain unassessed; model theorems are not being promoted
to full Rust/source/native refinement proofs. No new model theorem is needed for
this conformance-only change.

## Status and remaining scope

CT coverage advances from **9/50 to 10/50 mapped**, with **40 cases unmapped**.
CT14 and CT37 execute parser, checker, reference, CFG and Linux native stages.
Other platforms retain explicit unexecuted native-stage status. Four negative
controls execute against supported ownership syntax; unsupported syntax is not
being counted as semantic rejection.

All **785 workspace tests**, **31 provider tests**, **43 Python tests**, format,
Clippy, both reevaluation probes, Lean build and all **20 Lean audits** pass.
The additional Rust test preserves CT37's former sequential payload evidence;
the new CT mapping executes within the existing manifest test. The 58-module
contract corpus is unchanged. Full 185-unit Cycle19 source, diagnostics,
chunk-contract and source-effect artifacts remain byte-identical to baseline:
**2 Safe /42 provisional Unsafe /137 Unknown**, plus four data-only units, and
**3 checked /93 Unknown** source-effect functions. No raw unit is promoted.

This covers an owned Buffer with explicit join and affine Result consumption.
It does not establish loan/share restoration, general owned graphs, global
heap-peak bounds, arbitrary native failure behavior or all scheduling orders.
Broader W6, provider extraction and the W2–W7 generalization goal remain open.
Existing subagents remain stopped by their reported usage limit; work continued
locally using the now-implemented source/native path.
