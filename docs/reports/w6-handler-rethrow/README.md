# CT20: join and account for results before rethrow

> Historical checkpoint: source filenames, runnable references or discovery scripts in this archive were updated by the [extension migration](../extension-physical-migration/README.md). Recorded results, hashes and gate counts describe the original revision, not a new run of the maintained files. Use [current status](../../STATUS.md) for present measurements.


Baseline `e4593ec`, 2026-09-07. CT20 is now mapped to an executed source fixture:
an in-scope handler for failed second creation joins its earlier child, drops the
returned owned Buffer and raises the bound payload-free `SpawnError`. This uses
the existing `ScopedOwnedTasksV1` implementation. No parser, checker, core,
backend, contract or Lean semantics change is needed.

The first child owns a buffer containing 42 and returns that same ownership.
The second child receives a copied integer. On second-create failure, the first
child's completion record remains obligated until its join; the resulting owned
buffer is then dropped before the error escapes. On initial-create failure the
parent retains its original buffer and drops it before raising the bound error.
Allocation failure acquires no buffer or task. Success joins both children,
drops the returned buffer and returns 42.

| Scenario | Root result | Joined children | Released Buffer generations |
|---|---|---|---|
| Both creations succeed | `I32 42` | 2 | 1 |
| Initial Buffer allocation fails | `AllocError` | 0 | none acquired |
| First creation fails | `SpawnError` | 0 | 1, retained by parent |
| Second creation fails | `SpawnError` | 1 | 1, returned by earlier child |

The `unjoined.qcore` control removes the handler join and rejects with Q600
`ScopeBusy`. The `undropped.qcore` control joins but omits the buffer drop and
rejects with Q600 at the error exit for discarded ownership. Both controls use
the same supported source syntax as the accepted case. Unsupported syntax is
not being mistaken for semantic rejection.

## Executed evidence

The CT harness adds a reusable `native_task_scenarios` fixture mode. Each matrix
records allocation/create decisions, fault indices, root outcome, child count,
owned result identities and the complete expected native custody log. Scenario
names must be unique and nonempty; scripts and fault indices must agree. The
harness compiles actual QCore source, executes its checked task CFG, emits and
compiles C11, and compares native stdout byte-for-byte with reference execution.
Every created child must be joined, and all declared negative controls execute.
The manifest's fixture inventory and per-stage claims remain exact and fail closed.

The native header supplies controlled Buffer-allocation/thread-create failures.
It records actual Buffer and environment pointers, exact thread handles and
join completion. It rejects environment reclamation before the corresponding
join, buffer reclamation while a child remains unjoined, wrong-pointer releases
and any residual resource at process exit. Environment allocation uses the real
`calloc` with its original count/size, and real Linux C11 threads execute the
workers. These are test adapters, not source-provided success flags or claims
about arbitrary native allocation or thread failures.

All four fixed runs also pass ASan, UBSan and LSan. They run untraced because
LSan cannot operate under the traced sandbox. Their generated C, executable
hashes, stdout, stderr and compile commands are archived in `native-runs.json`
and the adjacent compressed artifacts. A typed error is an observed program
result, not a failed process in this diagnostic executable.

Reproduce the source/native/reference comparison:

```sh
cargo test -p qcore-compiler --test ct_corpus
cargo run --quiet -p qcorec -- emit -o /tmp/qcore-ct20.c \
  compiler/tests/fixtures/ct/CT20/case.qr
clang -std=c11 -pthread -Wall -Wextra -Werror -g -O1 \
  -fsanitize=address,undefined -fno-omit-frame-pointer \
  -DQCORE_CT_FAIL_ALLOC=0 -DQCORE_CT_FAIL_CREATE=2 \
  -include compiler/tests/fixtures/ct/CT20/task_scenarios.h \
  /tmp/qcore-ct20.c -o /tmp/qcore-ct20
ASAN_OPTIONS=detect_leaks=1 /tmp/qcore-ct20
```

## Status and boundaries

Mapping advances from **8/50 to 9/50**, with **41 cases still unmapped**. Parser,
type checker, reference execution, task CFG and native backend stages execute
for CT20 on Linux. Other platforms check target rejection and explicitly report
the Linux execution stages unexecuted. The source fixture's proof stage remains
unassessed. Existing Lean transfer/exit results are separate model evidence.

This mapping covers one owned Buffer result and copied scalar results with
payload-free errors. It does not establish loans, owned error payloads, general
error handlers, worker allocation, full Rust/Lean/C refinement or all native
failure behaviors. The broader W6 and generalization goals remain open.

The standing gate passes **767 workspace tests**, format/Clippy, both probes,
Lean build and all **20** axiom audits. All **31** excluded-provider tests and
**43** Python tests pass. The new CT case executes within the existing manifest
test, so the Rust test count does not increase. Full 185-unit Cycle19 artifacts
remain byte-identical to baseline: **2 Safe /42 provisional Unsafe /137 Unknown**
plus four data-only units; **3 checked /93 Unknown** source-effect functions.
No raw result is promoted. Validation and exact parity records are archived here.
Subagents remain stopped by their reported usage limit; work continued locally.
