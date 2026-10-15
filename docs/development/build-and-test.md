# Building and validating QCore-C

[Development table of contents](../../DEVELOPMENT.md)

Run commands from the repository root. The [standing per-change gate](../../AGENTS.md#standing-per-change-gate)
is authoritative; this page supplies setup and platform details.

## Contents

- [Build and test](#build-and-test)
- [Windows](#windows)

<!-- migrated-section:start -->
## Build and test

The Rust compiler and `quirin check` / `quirin emit` do not require Clang.
Compiling emitted C requires a C11 compiler (MSVC on Windows via the Rust
MSVC toolchain, a native compiler elsewhere). The full test suite also
exercises existing-C analysis, which needs Clang for preprocessing, syntax
validation, and its JSON AST; `cl.exe` cannot replace that dependency.

```sh
git submodule update --init --recursive
cargo test --workspace --no-fail-fast
python3 scripts/check_lean_audits.py
```

The Rust suite verifies that `third_party/libevent` is at the audited
revision and still contains the source constructs the Cycle 18 model relies
on; CI initializes the submodule before this gate. The Lean toolchain is
selected through `proof/lean-toolchain` (install elan first). The audit runner
starts Lake inside `proof`, builds incrementally, then runs every `Audit*.lean`
with warnings treated as failures. The QCore library build also treats warnings
as errors. The pin is Lean 4.35.0; the runner refuses to build or audit when
`lake env lean --version` inside `proof` reports another version. Use
`ite_eq_left`, `ite_eq_right`, `ite_true` and `ite_false` instead of the
deprecated `if_*` names, and never name anything `given`, `requires`,
`ensures`, `recall` or `monotonicity_by` (Lean 4.35 syntax). The runner reports
elapsed time every 15 seconds, limits the build to 600 seconds and each audit
to 120 seconds, and stops the
active process tree on timeout or Ctrl-C. Limits can be set with
`--build-timeout`, `--audit-timeout`, and `--heartbeat`; `--verbose` shows Lake
build commands. Use `--skip-build` only after a successful build of the current
sources.

For a build without audits, use `(cd proof && lake build)`. From the repository
root, `lake -d proof ...` can make elan query the default `stable` release before
Lake applies `-d`; selecting the working directory first uses the pinned
toolchain directly. `QCore.lean` only imports the proof modules: checking that
file alone does not run the axiom audits. Keep `proof/.lake` for incremental
reuse. To profile the trace relation independently:

```sh
(cd proof && lake env lean -Dprofiler=true QCore/TaskTraceRelation.lean)
```

### Windows

Install Rust's MSVC toolchain and Visual Studio or Build Tools with
**Desktop development with C++** (MSVC and the Windows SDK). Use an **x64
Native Tools Command Prompt** or **Developer PowerShell** when compiling
native C so the SDK, linker, and runtime DLLs are available. Generated-C
tests discover MSVC through the `cc` crate; no `cc` or `clang` command on
PATH is required.

Scoped task and checked mutex emission embeds the required C11 thread subset
on Windows, adapted from Monkey winpthreads. In the developer shell:

```powershell
cargo run -p qcorec -- emit examples/threads/two_workers.qr -o generated.c
cl /nologo /std:c11 /W4 /WX generated.c /Fe:generated.exe
.\generated.exe
```

The [threading examples](../../examples/threads/README.md) cover copied inputs,
owned-buffer transfer and shared readers. Build and run all three with
`.\examples\threads\build-windows.ps1 -Run`; the script puts C, object and
executable files in `target\thread-examples\`. The MSVC core CI job runs the
same script with Clang discovery disabled.

No extra header, object file or pthread library is needed. The native test
harness uses MSVC's `/experimental:c11atomics` for its synchronization observers;
the emitted task runtime itself does not require that switch. The supported
surface is joinable creation/join, equality/yield and plain non-recursive
mutexes, not the full C11 header. See
[the Windows thread report](../reports/windows-c11-threads/README.md).

For existing-C analysis and the full test suite, also install Visual
Studio's optional **C++ Clang tools for Windows** or a standalone LLVM
installation (see
[Microsoft's Clang setup instructions](https://learn.microsoft.com/en-us/cpp/build/clang-support-msbuild)).
QCore locates the analysis compiler in this order:

1. `QCORE_CLANG`, when set, names a single executable (no extra arguments).
2. `clang` on PATH.
3. On Windows, LLVM under `VSINSTALLDIR`, then Visual Studio installations
   found via `vswhere`, newest first (including Build Tools and previews).
4. Standard Visual Studio 2026 (`Microsoft Visual Studio\18`) and 2022
   (`Microsoft Visual Studio\2022`) edition directories under Program Files,
   including when `vswhere` is missing or returns no installations.

Both `clang.exe` and `clang-cl.exe` work, using Clang's GCC driver mode for
analysis. For example, VS 2026 Community installs its x64 compiler at
`C:\Program Files\Microsoft Visual Studio\18\Community\VC\Tools\Llvm\x64\bin\clang.exe`.
Discovery tries subsequent installations when an earlier one lacks Clang.

The Windows suite also runs syntax-only POSIX LP64 cleanup fixtures in isolated
child tests, forwarding to the discovered Clang with an explicit Linux target.
These exercise the existing POSIX models, not native Windows cleanup behavior;
ordinary Windows analysis keeps its native target.
Local VS 2026 validation passed 828 tests, formatting, Clippy and both
reevaluation probes; Lean was unavailable. See the
[Windows validation report](../reports/windows-vs2026/README.md).

An explicit path works without changing PATH, including paths with spaces:

```powershell
$env:QCORE_CLANG = 'C:\Program Files\LLVM\bin\clang.exe'
cargo test --workspace --no-fail-fast
```

An invalid or empty `QCORE_CLANG` is an error; it never silently selects
another compiler. If no Clang is found, existing-C analysis reports an
unavailable parser with installation guidance and does not issue a safety
certificate; ordinary QCore checking and C emission remain available:

```powershell
cargo build --bin quirin
.\target\debug\quirin.exe check examples\cycle12.qcore
.\target\debug\quirin.exe emit -o generated.c examples\cycle12.qcore
```

Windows CI runs a core MSVC job (Clang discovery disabled; checking,
emission, native generated-C execution) and an MSVC-plus-Clang analysis job
(full suite including MSVC AddressSanitizer; Clang removed from PATH and
`VSINSTALLDIR` cleared to exercise `vswhere` discovery). Linux CI runs the
full suite.

<!-- migrated-section:end -->
