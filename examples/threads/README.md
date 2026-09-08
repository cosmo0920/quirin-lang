# Native threading examples

These QCore programs emit self-contained C11 and run native threads on Windows,
Linux and macOS. The Windows instructions below use MSVC's `cl.exe`.

| Example | What it demonstrates | Successful main result |
|---|---|---|
| [two_workers.qcore](two_workers.qcore) | Two copied inputs, both workers started before either join, recovery if the second creation fails | `7` |
| [owned_buffer.qcore](owned_buffer.qcore) | Transfer a buffer to a worker and receive it back at join; the parent uses a separate buffer meanwhile | `42` |
| [shared_readers.qcore](shared_readers.qcore) | Two read-only workers share one buffer; joining both restores the parent's write access | `7` |

## Build and run on Windows

Install Rust's MSVC toolchain and Visual Studio's **Desktop development with
C++** workload, including the Windows SDK. Open **Developer PowerShell for
Visual Studio** configured for x64, then change to the repository root. The
developer shell sets up the compiler, headers and linker libraries; adding
`cl.exe` alone to PATH is insufficient. See Microsoft's
[command-line C walkthrough](https://learn.microsoft.com/en-us/cpp/build/walkthrough-compile-a-c-program-on-the-command-line).

Build and run all three examples:

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

The runtime prints task events. A normal `two_workers` run prints:

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
