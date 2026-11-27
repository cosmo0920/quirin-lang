# Windows Visual Studio 2026 toolchain repair

Date: 2026-09-07.

## Reproduction

The development machine has Visual Studio Community 2022 and Visual Studio
Community 2026 (18.7.3). Only the 2026 installation has Clang, version 22.1.3:

```text
C:\Program Files\Microsoft Visual Studio\18\Community\VC\Tools\Llvm\x64\bin\clang.exe
```

Neither `clang` on PATH nor `QCORE_CLANG` was configured. The existing `vswhere`
query already returned both installations, so discovery alone was not the cause
of all test failures. The first workspace run stopped at 45 failed compiler
library tests. Subsequent testing exposed a further CRLF-sensitive mutation in
the generic source tests.

## Changes

- Keep the authoritative `QCORE_CLANG` override and PATH precedence. Request
  newest-first `vswhere` results, continue after empty locator output, and fall
  back to edition directories under both Program Files roots: `18` for VS 2026
  and `2022` for VS 2022. Native ARM64 LLVM directories are considered first on
  ARM64 hosts. Every selected executable still passes the Clang version probe.
- Add three discovery regression tests for custom installation roots, older
  installations without Clang, missing/empty locators, default edition layouts,
  and environment precedence. Existing override/no-fallback tests remain.
- Build the Cycle19 compile-database fixture with the discovered executable,
  and exercise that test in Windows CI with Clang removed from PATH.
- Preserve explicit synthetic counter types without redefining Windows Clang's
  implicit `size_t`. Supply the copied Monkey POSIX fixtures with their declared
  errno accessor and POSIX wrapper declarations on Windows.
- Run the POSIX LP64 cleanup model tests in isolated child processes on Windows.
  A test-only executable forwards to the discovered Clang with
  `--target=x86_64-unknown-linux-gnu`. Parent tests check child success and that
  exactly one test executed. No parent environment is changed, and native
  Windows analysis retains its normal target. The synthetic parent fixture
  declares `free` directly instead of requiring a target libc installation.
- Normalize CRLF before multiline test mutations and restrict Unix-only lint
  helpers/imports to their supported compilation targets.

## Validation

Final Windows result: **828 passed, 0 failed, 0 ignored**, including doctests.
Formatting, workspace Clippy with warnings denied, and both reevaluation probes
passed. The POSIX child executions are counted through their parent tests.

Commands run from the repository root:

```powershell
git submodule update --init --recursive
cargo test --workspace --no-fail-fast --target-dir target/windows-validation -- --test-threads=8
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p qcore-compiler --example reevaluation_probe
cargo run -p qcore-compiler --example reevaluation_followup
```

The separate target directory avoids a test executable held open by another
already-running test process. Eight test threads reduce subprocess contention;
the earlier default-thread run also completed and identified the final six
nested-cleanup fixture failures. All 14 affected nested/parent tests passed after
the final fixture repair. A missing-Clang child control failed as required.
Local logs are under `target/windows-*.log`. Rust is 1.97.1 for Windows MSVC.
The pinned Libevent submodule was initialized and its revision checked.

`lake build` in `proof/` could not run: neither `lake` nor an installed Lean toolchain
was available. WSL contains only Docker Desktop. Lean definitions are unchanged.

No checker semantics, production POSIX ABI profile, source pin, or verdict rule
changes. The cross-target fixtures establish their existing syntax/model
properties, not native Linux execution on Windows or Windows filesystem cleanup
guarantees. Hosted CI and a native ARM64 machine were not exercised locally.
