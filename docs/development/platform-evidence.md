# Platform evidence and boundaries

Current reference, routed by [AGENTS.md](../../AGENTS.md). Rule numbers are stable.
Paths in backticks are repository-relative. Read every applicable rule,
including cross-topic rules, before editing; the index is not a substitute.

<!-- preserved:platform-records:start -->
## Platform records

This is the home for per-platform observations; `AGENTS.md` keeps the toolchain
requirements that are part of the gate.

- **Cross-platform build (user-validated, 2026-09-11):** the codebase builds on
  Linux, macOS and Windows with a small number of recorded limitations (the
  per-platform residual lists in the reports below), and **ARM64 Windows**
  starts the `quirin` compiler and compiles emitted C sources. The ARM64
  observation is the user's own environment report — compiler runs and emitted C
  builds; no test-suite, sanitizer or byte-comparison claim is made for that
  architecture. `QUIRIN-ARM64-GATE` and `QUIRIN-ARM64-RUN` were carried here as
  "candidate residual names" for three waves and reached no tracker; both are
  **registered in `named_followups`** as of `DOCS-COMPACT-6` (wave 34), each
  with its own discharge condition.
- **Windows:** native C11 thread subset and MSVC/Clang fixtures —
  `docs/reports/windows-c11-threads/README.md`,
  `docs/reports/windows-vs2026/README.md`,
  `docs/reports/windows-clang-fixtures/README.md`. Visual Studio Clang
  discovery orders vswhere newest first with VS 2026/2022 fallbacks.
  - **`SOCKET-V1-WINDOWS-RUN` discharged (user-validated, 2026-09-12):** both
    published `source.socket.v1` examples compiled by Microsoft's actual
    `cl.exe` (Visual Studio 18 Community, MSVC 19.51.36248), executed against
    Winsock loopback, compared byte for byte with the checked reference
    observation, the complete 49-test `std_socket` target passing —
    `docs/reports/socket-v1-windows-run/README.md`. Scope: native execution and
    byte agreement, **no sanitizer claim** (MSVC ships no ASan/UBSan in this
    harness).
  - **Open, each with an exact test-name discharge condition.** Seven of them
    share one shape and it is stated once here rather than seven times: the
    card's native test ran the **Linux arm only**, the `tests/support/windows_c.rs`
    path is carried unchanged (its MSVC rows written and wired through
    `run_msvc`), **no such run has happened and none is claimed**. The members,
    each with the report that owns its detail: `HTTP-V1-WINDOWS-RUN` and
    `STRING-SLICE-IO-WINDOWS-RUN`
    (`docs/reports/windows-native-sockets-harness/README.md`);
    `SOCKET-INET-WINDOWS-RUN` for the two declared-address examples
    (`docs/reports/socket-inet-addresses/README.md`);
    `SLICE-PERMISSIONS-WINDOWS-RUN` (wave 27, where the emitted difference is
    one pointer displacement inside an existing initialiser —
    `docs/reports/ct07-slice-permissions/README.md`);
    `WRITE-LEND-WINDOWS-RUN` (wave 29,
    `docs/reports/slice-permissions-write-lend/README.md` §13, left unchanged by
    wave 30's `CT07-WRITE-REMAP`); `WRITE-LEND-TASKS-WINDOWS-RUN` (wave 31, on
    the `STRING-CONCAT-WINDOWS-RUN` precedent, macOS compiling the same test
    file — `docs/reports/write-lend-tasks/README.md` §9); and
    `OWNED-FIELD-WINDOWS-RUN` (wave 39, `AGGREGATE-OWNED-FIELD` §7, where
    `examples/records/owned_field.qcore` ran under ASan/UBSan/LSan on Linux
    only). **Their tracking is deliberately uneven and that is the open
    question**: `WRITE-LEND-WINDOWS-RUN` and `WRITE-LEND-TASKS-WINDOWS-RUN`
    were the two `STD-MODULES-O`'s roll-up did not mention and that reached no
    tracker at all, and both are **registered in `named_followups`** as of
    `DOCS-COMPACT-6` (wave 34); `SLICE-PERMISSIONS-WINDOWS-RUN` and
    `OWNED-FIELD-WINDOWS-RUN` are named here and **not** registered, by the
    wave-39 close's decision, until
    `PLATFORM-RESIDUAL-FAMILY-PARTIALLY-TRACKED` is decided.
  - Four tests keep `runs_sanitized` in their names while their Windows arm
    claims no sanitizer coverage (`WINDOWS-RUN-TEST-NAME-SANITIZED`).
  - **`W48-QRLIB-WINDOWS-RUN` (wave 48), wired, not run.** Card
    `W48-QRLIB-LINKAGE` added `cargo test -p qcore-c-gateway --lib linkage::`
    and `cargo test -p qcore-compiler --test qrlib_linkage` to the
    `windows-2022` `msvc-core` job: the COFF/PE readers from bytes, an MSVC
    static `.lib` (`cl.exe` + `lib.exe` through `cc`) and one `ntdll.dll`
    function (the first of `RtlNtStatusToDosError`,
    `RtlNtStatusToDosErrorNoTeb` that the SDK's `ntdll.lib` imports by name and
    the image's `System32\ntdll.dll` exports by name, not as a forwarder),
    each generated into a `.qrlib`, checked, emitted and linked by `cl.exe`
    through `run_msvc_with_libraries`. The card ran on Linux only: the
    integrator reads the job after landing, and no Windows result is claimed
    until then.
- **`W48-QRLIB-MACOS` (5m) discharged (user's Mac, 2026-09-30).** On macOS
  27.0.1 (arm64, Apple clang 21.0.0, `ld-27037.1`) the first run refused one
  shape: the SDK's `libm.tbd` (a link to `libSystem.tbd`) lists `arm64e-macos`
  and no `arm64-macos`. The reader now takes the `arm64e-macos` slice for an
  unlisted `arm64-macos`, the rule Apple `ld` was probed to follow; then
  `cargo test -p qcore-c-gateway --lib linkage::` (31 passed, including the
  real SDK stub) and
  `cargo test -p qcore-compiler --test qrlib_linkage --test ffi_struct_import --test float_ffi --test usable_records`
  (10 + 23 + 3 + 19 passed) ran green: libm through the stub under the three
  builds (runtime premise `/usr/lib/libSystem.B.dylib`), a Mach-O C archive
  and a Rust `staticlib`. The `macos` CI job (`macos-15`) runs the same
  commands; its first run is not yet read (`W48-QRLIB-MACOS-CI-RUN`). Intel macOS is out of scope: the current macOS no longer runs on Intel Macs (user decision, 2026-09-30).
  [Report](../reports/w48-qrlib-macos/README.md).
- **macOS:** pthread-backed C11 thread subset, Apple ASan/UBSan (LSan is
  Linux-only, so leak detection stays a Linux observation), and Apple Clang
  option transport — `docs/reports/macos-cycle19-compiler-options/README.md`.
  Configured-audit inputs compare canonical existing paths so the
  `/var` → `/private/var` alias cannot invalidate a pinned task.
  - **`EVENT-V1-APPLE-RUN` discharged (user-validated, 2026-09-12):** the
    `std_event` suite, including its `cfg(not(windows))` native comparison, ran
    green on the user's macOS environment — the exact run that residual named
    as its discharge. Scope: Apple ASan/UBSan and byte comparison; no LSan
    claim.
  - Open: `SOCKET-V1-APPLE-RUN`, until a macOS run at or after the socket card
    (`47712702`) covers `std_socket`'s native tests, and `SOCKET-INET-APPLE-RUN`
    beside it for the declared-address examples
    (`docs/reports/socket-inet-addresses/README.md`).
- **Linux:** the sanitizer, mapping (`MAP_FIXED_NOREPLACE`) and OS-endpoint
  observations are Linux-only by scope, not by omission.

<!-- preserved:platform-records:end -->
