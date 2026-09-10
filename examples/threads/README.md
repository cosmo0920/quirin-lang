# Native threading examples

These QCore programs emit self-contained C11 and run native threads on Windows,
Linux and macOS. The Windows instructions below use MSVC's `cl.exe`.

| Example | What it demonstrates | Successful main result |
|---|---|---|
| [two_workers.qcore](two_workers.qcore) | Two copied inputs, both workers started before either join, recovery if the second creation fails | `7` |
| [owned_buffer.qcore](owned_buffer.qcore) | Transfer a buffer to a worker and receive it back at join; the parent uses a separate buffer meanwhile | `42` |
| [shared_readers.qcore](shared_readers.qcore) | Two read-only workers share one buffer; joining both restores the parent's write access | `7` |
| [core_context.qcore](core_context.qcore) | Minimal source counterpart of the Rust core prelude: owned handoff, explicit join and creation rollback | `42` |

## Mapping source examples to the core prelude

The frontend lowers these examples into the checked models exposed by
`qcore_core::prelude::concurrency`. The prelude is a Rust import surface for
compiler and library implementers. Every `.qcore` example explicitly opts in:

```qcore
import std.concurrency;
concurrency checked;
```

The import makes the bundled concurrency vocabulary available in that source
file. Without it, concurrency forms reject with `Q015`. The marker selects the
checked task profile, and `thread scope` constructs a particular scoped context.
Imports do not propagate from other input files. Unknown module paths reject
with `Q014`; user-file imports are not yet supported.

| QCore source operation | Core model operation | Lean model |
|---|---|---|
| `import std.concurrency;` | Exact `ModuleImports::resolve`, initially empty | `ModuleImports` proves explicit selection and fail-closed resolution |
| `thread scope context` | `ScopedTaskLedger::enter_scope` / `exit_scope` | `TaskExecution` checks handle quiescence; it is not a complete lexical-scope model |
| `spawn context worker(move buffer)` | `reserve`, then `commit` or `rollback` | `ScopedTasks` proves custody transfer and rollback; `TaskExecution` models creation outcomes |
| Worker `return move buffer` | `complete` retains a completion capsule | `ScopedTasks` / `TaskExecution` distinguish completion from collection |
| `join move handle` | Creator `join` collects the outcome and returns custody | `ScopedTasks` / `TaskExecution` prove custody return and one-time handle consumption |
| `read buffer share 512` in `shared_readers` | `reserve_with_loans` and share reclamation on join | `ReadShare` proves the lending budget and restoration after the last share |

Run the [Rust custody-model counterpart](../../crates/qcore-core/examples/concurrency_prelude.rs)
of `core_context.qcore`:

```sh
cargo run -p qcore-core --example concurrency_prelude
```

It executes the model for successful and failed creation, including refusal of
scope exit before join. It does not start native threads or model buffer values.
To check, emit and run the QCore counterpart on Linux:

```sh
mkdir -p target/thread-examples
cargo run -p qcorec -- check examples/threads/core_context.qcore
cargo run -p qcorec -- emit examples/threads/core_context.qcore -o target/thread-examples/core_context.c
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
cargo run -p qcorec -- emit examples\threads\two_workers.qcore -o target\thread-examples\two_workers.c
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

The runtime prints task events. **None of that output is a source construct.**
No line below is asked for by any of the four programs: every one is trace
instrumentation the compiler synthesizes so the emitted C can be compared with
the reference execution. A source-level `print` cannot appear in these files at
all — a module that both spawns and prints stays with the task route, which
refuses the `io` effect (`Q020`) and the statement (`Q500`); both refusals are
pinned in [`compiler/tests/thread_examples.rs`](../../compiler/tests/thread_examples.rs).
Since card `TRACE-STDIO` these lines and `std.io`'s `print`/`log` travel one
shared C-level emission route (`QCORE_IO_WRITE`), so an emitted program has a
single printing mechanism rather than two; the printed bytes are unchanged.

The transcript below is not prose: `thread_examples.rs` reads it out of this
file and compares it with a real sanitized run, so it cannot drift. That test
also builds and runs all four examples under ASan/UBSan (and LSan on Linux) and
compares each one's stdout byte for byte with the reference execution:

```sh
cargo test -p qcore-compiler --test thread_examples
```

A normal `two_workers` run prints:

```text
QCORE_TASK_SPAWN 1 0
QCORE_TASK_SPAWN 2 0
QCORE_TASK_RESULT 2 I32 7
QCORE_TASK_JOIN 2
QCORE_TASK_RESULT 1 I32 42
QCORE_TASK_JOIN 1
QCORE_TASK_RESULT 0 I32 7
```

Task `0` is the parent. The last line contains the QCore main result; successful
execution exits with process status `0`, even when that result is `7` or `42`.
Worker events are buffered and printed after join, so their printed order does
not reveal which worker the OS scheduled first or prove simultaneous execution.

The buffer examples also print allocation, access, transfer and release events.
They explicitly handle allocation or creation refusals so previously acquired
buffers and started siblings can be discharged. A native join failure remains
fatal in the emitted runtime, preserving storage that a worker might still use.

These examples use the implemented scoped-task grammar and the `Buffer`
intrinsics (one `i32` payload per buffer). They are not a general thread-pool or
parallel-array API. See the [Windows thread implementation report](../../docs/reports/windows-c11-threads/README.md)
for the supported adapter subset and validation bounds.
