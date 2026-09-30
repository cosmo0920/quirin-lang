# Native threading examples

These QCore programs emit self-contained C11 and run native threads on Windows,
Linux and macOS. The Windows instructions below use MSVC's `cl.exe`.

| Example | What it demonstrates | Successful main result |
|---|---|---|
| [two_workers.qr](two_workers.qr) | Two copied inputs, both workers started before either join, recovery if the second creation fails; both results logged and one printed by the source | `7` |
| [owned_buffer.qr](owned_buffer.qr) | Transfer a buffer to a worker and receive it back at join; the parent uses a separate buffer meanwhile, and logs the number it reads back out of the returned buffer. It also shows both pipeline spellings: `second \|> qcore_buffer_read \|> log info dec;` chains a model intrinsic into a sink, while the post-join read keeps its own binding because the number is used twice and a pipeline is linear | `42` |
| [shared_readers.qr](shared_readers.qr) | Two read-only workers share one buffer; joining both restores the parent's write access, with the parent logging before, between and after the joins | `7` |
| [core_context.qr](core_context.qr) | Minimal source counterpart of the Rust core prelude: owned handoff, explicit join and creation rollback, with the rollback arm logging on a branch the successful run never reaches | `42` |

All four also `#import std.logger;` and `#import std.fmt;` (and `two_workers`
additionally `#import std.io;`), which is what lets them say something of their
own beside the trace the compiler synthesizes — see
[Reading the output](#reading-the-output).

## Mapping source examples to the core prelude

The frontend lowers these examples into the checked models exposed by
`qcore_core::prelude::concurrency`. The prelude is a Rust import surface for
compiler and library implementers. Every `.qr` example explicitly opts in:

```qcore
#import std.concurrency;
concurrency checked;
```

The import makes the bundled concurrency vocabulary available in that source
file. Without it, concurrency forms reject with `Q015`. The marker selects the
checked task profile, and `thread scope` constructs a particular scoped context.
Imports do not propagate from other input files. Unknown module paths reject
with `Q014`; user-file imports are not yet supported.

| QCore source operation | Core model operation | Lean model |
|---|---|---|
| `#import std.concurrency;` | Exact `ModuleImports::resolve`, initially empty | `ModuleImports` proves explicit selection and fail-closed resolution |
| `thread scope context` | `ScopedTaskLedger::enter_scope` / `exit_scope` | `TaskExecution` checks handle quiescence; it is not a complete lexical-scope model |
| `spawn context worker(move buffer)` | `reserve`, then `commit` or `rollback` | `ScopedTasks` proves custody transfer and rollback; `TaskExecution` models creation outcomes |
| Worker `return move buffer` | `complete` retains a completion capsule | `ScopedTasks` / `TaskExecution` distinguish completion from collection |
| `join move handle` | Creator `join` collects the outcome and returns custody | `ScopedTasks` / `TaskExecution` prove custody return and one-time handle consumption |
| `read buffer share 512` in `shared_readers` | `reserve_with_loans` and share reclamation on join | `ReadShare` proves the lending budget and restoration after the last share |

Run the [Rust custody-model counterpart](../../crates/qcore-core/examples/concurrency_prelude.rs)
of `core_context.qr`:

```sh
cargo run -p qcore-core --example concurrency_prelude
```

It executes the model for successful and failed creation, including refusal of
scope exit before join. It does not start native threads or model buffer values.
To check, emit and run the QCore counterpart on Linux:

```sh
mkdir -p target/thread-examples
cargo run -p qcorec -- check examples/threads/core_context.qr
cargo run -p qcorec -- emit examples/threads/core_context.qr -o target/thread-examples/core_context.c
clang -std=c11 -pthread -Wall -Wextra -Werror target/thread-examples/core_context.c -o target/thread-examples/core_context
target/thread-examples/core_context
```

`import QCore.ConcurrencyPrelude` makes the corresponding bounded Lean models
and selected proved properties available. It does not prove arbitrary native
execution or combined task/mutex behavior. The integration test runs the real
source example on Unix with successful and injected failed creation, comparing
both traces with the core reference execution and checking exact buffer release.

## Build and run on Windows

Install Rust's MSVC toolchain and Visual Studio's **Desktop development with
C++** workload, including the Windows SDK. Open **Developer PowerShell for
Visual Studio** configured for x64, then change to the repository root. The
developer shell sets up the compiler, headers and linker libraries; adding
`cl.exe` alone to PATH is insufficient. See Microsoft's
[command-line C walkthrough](https://learn.microsoft.com/en-us/cpp/build/walkthrough-compile-a-c-program-on-the-command-line).

Build and run all four examples:

```powershell
.\examples\threads\build-windows.ps1 -Run
```

Omit `-Run` to build only. Generated `.c`, `.obj` and `.exe` files go to
`target\thread-examples\`. For example, run the first executable again with:

```powershell
.\target\thread-examples\two_workers.exe
```

To perform the steps yourself, from the repository root:

```powershell
New-Item -ItemType Directory -Force target\thread-examples | Out-Null
cargo run -p qcorec -- emit examples\threads\two_workers.qr -o target\thread-examples\two_workers.c
cl /nologo /std:c11 /utf-8 /W4 /WX target\thread-examples\two_workers.c /Fotarget\thread-examples\two_workers.obj /Fe:target\thread-examples\two_workers.exe
.\target\thread-examples\two_workers.exe
```

For an already emitted file named `generated.c`, the same compile/link step is:

```powershell
cl /nologo /std:c11 generated.c /Fe:generated.exe
.\generated.exe
```

`/std:c11` selects C11; [`/Fe`](https://learn.microsoft.com/en-us/cpp/build/reference/fe-name-exe-file)
names the executable, and `/Fo` names the intermediate object file. `cl`
compiles and links by default; `/c` would stop after producing an object file.
`/W4 /WX` enables warnings and treats them as errors.

The emitted C embeds the Windows thread compatibility subset and uses the
Windows SDK and C runtime. There is no separate pthread library or adapter
source to download or link. Lean/elan is needed for the project's proof checks,
but not for building these executables.

## Reading the output

Each program's system observations remain structured events submitted to the
typed diagnostic logger at `trace` or `debug`; its ordinary sink is null. The
published programs explicitly select `log_level trace;`, so their system trace
is rendered on stdout for teaching and debugging.

Since card `COMPOSE-CONCURRENCY` a
module may both spawn and print, on the composed route
`source.std-io-tasks.v1` — so `first_result |> log info dec;` is a line the
program asks for. Before that
card it was measured impossible: a module that both spawned and printed stayed
with the task route, which refused the `io` effect (`Q020`) and the statement
(`Q500`).

**A source line may stand anywhere in the parent's body, and nowhere in a
worker's.** That split is read off the emitted runtime rather than guessed: every
write an emitted task program performs is performed by the parent's thread — a
worker's trace events are buffered in its own private array and replayed by the
parent inside the join — so a line placed at any point of the parent has a
determined position relative to every other line. A line inside a worker body
would be written by that worker's thread, concurrently with the parent's writes
and with its siblings', and nothing in the checked model orders those; it is
refused at `Q050`. `shared_readers.qr`'s worker carries a comment where such
a line would go.

Both transcripts below are not prose: `thread_examples.rs` reads them out of
this file and compares them with a real sanitized run, so they cannot drift.
That test also builds and runs all four examples under ASan/UBSan (and LSan on
Linux), compares source-authored stdout and stderr per stream, and separately
checks that the structured task observation remains non-empty:

```sh
cargo test -p qcore-compiler --test thread_examples
```

A normal `two_workers` run writes this on standard output:

<!-- transcript:stdout -->

```text
QCORE_TASK_SPAWN 1 0
QCORE_TASK_SPAWN 2 0
QCORE_TASK_RESULT 2 I32 7
QCORE_TASK_JOIN 2
QCORE_TASK_RESULT 1 I32 42
QCORE_TASK_JOIN 1
7
QCORE_TASK_RESULT 0 I32 7
```

and this on standard error:

<!-- transcript:stderr -->

```text
[INFO] 2
[INFO] 42
[INFO] 7
```

The bare `7` is the source's own `second_result |> print dec;`, and the three
`[INFO]` lines are its `log` statements: the creation decision this run took, and
then both joined results. The relative order of the two streams is **not**
modeled and is not asserted anywhere — C gives no order between two separately
buffered streams — but each stream's own order is checked.

Task lifecycle and buffer events remain in the structured diagnostic route at
`debug` and `trace` respectively. They become process output here only because
the example opts in. Successful execution exits with process status `0`, even
when the QCore main result is `7` or `42`.

The buffer examples retain allocation, access, transfer and release diagnostic
events without printing them. They explicitly handle allocation or creation refusals so previously acquired
buffers and started siblings can be discharged. A native join failure remains
fatal in the emitted runtime, preserving storage that a worker might still use.

These examples use the implemented scoped-task grammar and the `Buffer`
intrinsics (one `i32` payload per buffer). They are not a general thread-pool or
parallel-array API. See the [Windows thread implementation report](../../docs/reports/windows-c11-threads/README.md)
for the supported adapter subset and validation bounds.

## Records across tasks

Card `W48-QR-TASK-RECORD-CAPTURES` lets a worker receive and return a
**record**. A record crosses a spawn as its leaves: a record of scalars by
`copy`, one that owns a Buffer by `move` (every owned field's custody
transferred, its scalars duplicated), and it comes back through `join` whole.
A single field crosses by the existing capture of that one leaf — `copy r.f`,
`move r.buf`, `read r.buf share N`, `lend r.buf LOW..HIGH`. These three are
built and run with Clang on Linux only (`-std=c11 -pthread -Wall -Wextra
-Werror -pedantic`, under UBSan and ThreadSanitizer); the Windows script above
builds the four examples before them.

| Example | What it demonstrates | Successful main result |
|---|---|---|
| [record_workers.qr](record_workers.qr) | A nested record that owns a `Buffer<u32>` moves to a worker, which reads and writes the buffer and two scalar fields (one nested) and returns the record with `return move job;`; the parent unpacks it at `join`, prints its fields and releases it with `drop done;` | `0` |
| [record_field_leases.qr](record_field_leases.qr) | Two owned fields of one record leased for writing to two workers at the same time — two leaves, two resources, so no byte has two writers — and read back by the parent after both joins | `0` |
| [record_spawn_failure.qr](record_spawn_failure.qr) | A record moved to a worker; on the arm where the creation failed the record is the parent's again, its scalar field readable and its buffer still owned and released | `0` (created), `2` (refused) |

A record result's trace is its header, `QCORE_TASK_RESULT TASK RECORD N`, then
one result line per leaf in leaf order — depth-first, in declaration order.
The F02 observable alphabet has no record outcome, so these runs sit outside
F02 relational comparison (the reference encoding refuses them); their native
evidence is the byte comparison below.

<!-- transcript:record_workers -->

```text
QCORE_BUFFER_ALLOC 0 1 U32 21
QCORE_TASK_SPAWN 1 2
QCORE_BUFFER_READ 1 1 U32 21
QCORE_BUFFER_WRITE 1 1 U32 64
QCORE_TASK_RESULT 1 RECORD 4
QCORE_TASK_RESULT 1 U16 9
QCORE_TASK_RESULT 1 I64 41
QCORE_TASK_RESULT 1 I32 41
QCORE_TASK_RESULT 1 BUFFER 1
QCORE_TASK_JOIN 1
41
9
41
QCORE_BUFFER_READ 0 1 U32 64
64
QCORE_BUFFER_DROP 0 1
QCORE_TASK_RESULT 0 I32 0
```

<!-- transcript:record_field_leases -->

```text
QCORE_BUFFER_ALLOC 0 1 0
QCORE_BUFFER_ALLOC 0 2 0
QCORE_TASK_SPAWN 1 1
QCORE_BUFFER_LEASE 1 1 0 4
QCORE_TASK_SPAWN 2 1
QCORE_BUFFER_LEASE 2 2 0 4
QCORE_BUFFER_BYTE_WRITE 1 1 0 1
QCORE_BUFFER_BYTE_WRITE 1 1 3 1
QCORE_TASK_RESULT 1 UNIT
QCORE_TASK_JOIN 1
QCORE_BUFFER_LEASE_RETURN 1 1
QCORE_BUFFER_BYTE_WRITE 2 2 0 2
QCORE_BUFFER_BYTE_WRITE 2 2 3 2
QCORE_TASK_RESULT 2 UNIT
QCORE_TASK_JOIN 2
QCORE_BUFFER_LEASE_RETURN 2 2
QCORE_BUFFER_READ 0 1 16777217
QCORE_BUFFER_READ 0 2 33554434
7
0x01000001
0x02000002
QCORE_BUFFER_DROP 0 1
QCORE_BUFFER_DROP 0 2
QCORE_TASK_RESULT 0 I32 0
```

<!-- transcript:record_spawn_failure -->

```text
QCORE_BUFFER_ALLOC 0 1 U8 65
QCORE_TASK_SPAWN 1 1
QCORE_BUFFER_DROP 1 1
QCORE_TASK_RESULT 1 I32 3
QCORE_TASK_JOIN 1
3
QCORE_TASK_RESULT 0 I32 0
```

Each transcript is the reference execution's stdout, and
`compiler/tests/task_record_captures.rs` compares it with the native run at
`-O0` and `-O2` under UBSan, at plain `-O2`, and under ThreadSanitizer through
the harness's pthread binding (`TASK-ROUTE-TSAN-PTHREAD-BINDING`) — the lease
example also with both workers forced live together and forced in reverse.
The spawn-failure example is run a second time with every creation refused
through the runtime's own `QCORE_TASK_CREATE` seam, against the reference run
whose creation fails: `3`, then `65` read back out of the restored buffer.
