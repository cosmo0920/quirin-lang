# W48-QRLIB-MACOS — card 5m: the macOS linkage path, run

Base `a235ead6`, main `0631c00b` merged. Discharges `W48-QRLIB-MACOS`, registered by
[W48-QRLIB-LINKAGE](../w48-qrlib-linkage/README.md), whose macOS section this
card ran.

## Host

The user's Mac: macOS 27.0.1 (`26A434`), arm64, Apple clang 21.0.0
(`clang-2100.3.34.2`), `ld-27037.1`, SDK
`/Applications/Xcode.app/.../MacOSX.sdk`. Logs are worktree-private under
`private/5m-run1/` (first run) and `private/5m-run2/` (after the fix).

## First run: one refusal

`cargo test -p qcore-c-gateway --lib linkage::` passed (29). Of the compiler
binaries, `ffi_struct_import` (23), `float_ffi` (3) and `usable_records` (19)
passed, and `qrlib_linkage` passed 9 of 10: the Mach-O C archive and the Rust
`staticlib` linked through their generated manifests under the three builds,
unchanged. `libm_links_through_the_sdk_text_stub_under_three_builds` refused:

```text
tbd: `/usr/lib/libSystem.B.dylib` does not list the target `arm64-macos`
```

The SDK's `usr/lib/libm.tbd` is a symbolic link to `libSystem.tbd`, a 40-document
v4 stub whose Apple Silicon targets are `arm64e-macos`, `arm64e-maccatalyst` and
`arm64e.x1-*`; no `arm64-macos`. The math functions are in its
`/usr/lib/system/libsystem_m.dylib` document, which the reader already reached
through `reexported-libraries`. Of the SDK's `usr/lib` stubs, 445 list
`arm64e-macos`; 20 across `usr/lib` and the frameworks list `arm64-macos`.

## What Apple `ld` reads

[probe_ld_slices.sh](probe_ld_slices.sh) links an arm64 program calling
`qr_probe` (not a builtin, so nothing is folded; a first attempt with `floor`
became `frintm` and proved nothing) against one-document stubs:

| Stub targets | `_qr_probe` exported for | Link |
|---|---|---|
| `arm64e-macos` | `arm64e-macos` | 0, bound `from libSystem` |
| `arm64-macos` | `arm64-macos` | 0, bound `from libSystem` |
| `x86_64-macos` | `x86_64-macos` | 1, `missing required architecture arm64` |
| `arm64-macos, arm64e-macos` | `arm64e-macos` only | 1, undefined `_qr_probe` |

So `ld` reads an exact `arm64` slice when one is listed (and then does not see
`arm64e` entries), reads `arm64e` only when `arm64` is absent, and lets no other
architecture stand in. `arm64e` is arm64 with pointer authentication; a stub
holds names only, and the runtime images are built for `arm64e`.

## The change

`crates/qcore-c-gateway/src/linkage/tbd.rs`: `read` tries the requested target;
when the first document does not list it and the target is `arm64-macos`, it
reads the `arm64e-macos` slice, and records the slice read in
`TextStub::slice`. The slice is chosen once, from the first document, so a
re-exported document that does not list it contributes nothing (its symbols
refuse as missing). Both the YAML (v3/v4) and JSON (v5) readers follow it. The
manifest text is unchanged: the substitution is a function of the pinned stub's
bytes and the recorded target. No Lean change: `QCore.QrlibLinkage` models
admission over a symbol table's facts, not the reader that produces them.

Tests (`crates/qcore-c-gateway/tests/unit/linkage.rs`):
`an_arm64_link_reads_an_arm64e_slice_only_when_no_arm64_slice_is_listed` pins
the four probe rows from bytes (v4 and v5) plus `x86_64-macos` never
substituting; `the_sdk_libm_stub_exports_the_math_functions_through_libsystem_m`
(`cfg(target_os = "macos")`) reads the real SDK stub for the host and resolves
the six libm functions to `libsystem_m.dylib`.

CI: `.github/workflows/ci.yml` gains a `macos` job on `macos-15` running the two
commands below after recording the SDK and toolchain.

## Two gate failures only this Mac sees

The first standing-gate run here (at `e9f909c0`, logs `private/5m-gate/`)
failed two steps on code this card did not change; both were fixed here at the
user's direction.

* **`float_ieee_vectors` (1 of 6,926).** `proof/QCore/Ieee754Vectors32.lean`
  and `Ieee754Vectors64.lean` were generated on x86-64 Linux and wrote the
  kernel's raw NaN results: 16 rows per table held x86-64's default NaN
  `0xFFC00000` / `0xFFF8000000000000` (sign bit set), and AArch64 computes
  `0x7FC00000` / `0x7FF8000000000000`, so the regenerated table differed. The
  language leaves a NaN's payload and sign unspecified and the Lean theorems
  compare arithmetic and conversion rows with `sameValue`, so the generator now
  writes `scalar_floats::canonical` of each arithmetic and conversion result
  (the kernel-vs-host assertion still runs on the raw bits first). Regenerated:
  exactly those 16 lines per table changed, each only by that rewrite;
  negation rows (proved bit for bit) are untouched; counts unchanged.
* **clippy (`-D warnings`).** `compiler/tests/task_record_captures.rs` and
  `closures.rs` keep native-run helpers (`NEXT`, `BUILDS`, `Run`,
  `build_and_run`, `REFUSE_CREATION`, their `Command`/atomic imports) that only
  `cfg(target_os = "linux")` tests use; they are now gated the same way.
  `qrlib_linkage.rs`'s `Built::trace`, filled on every Unix host and read only
  by the Linux `--trace` row, is allowed dead off Linux.

## Second run: green

```sh
cargo test -p qcore-c-gateway --lib linkage::
cargo test -p qcore-compiler --test qrlib_linkage --test ffi_struct_import \
    --test float_ffi --test usable_records -- --nocapture
```

Exit 0 and 0: 31 passed (29 + the two new rows); 10 + 23 + 3 + 19 passed.
libm (`ilogb`, `sqrt`, `fabs`, `fmax`, `nextafter`, `floor`) links through
`libm.tbd` with the runtime premise `install_name /usr/lib/libSystem.B.dylib`
(the file is in the dyld shared cache, not on disk; the premise names it, as
§L14.8 item 2 says) and matches the reference under the three builds.

## Standing gate (merged tree `c374de96`)

Recorded in [gate.json](gate.json) and `gate/`; logs `private/5m-gate4/`.
**Not fully green on macOS**, for pre-existing harness reasons only
(`W48-MACOS-GATE-HARNESS`):

| Step | Exit | Result |
|---|---|---|
| submodule, fmt, clippy, both probes | 0 | clean |
| `check_lean_audits.py` | 0 | 255 audits |
| `run_test_binaries.py` | 101 | 6,966 passed, 1 failed, 10 ignored; the failure, `subcommand_plugins::a_timeout_is_unknown_and_kills_the_process_group`, is an exclusive load-sensitive row, green 3/3 in isolation |
| provider suite | 101 | 126 passed, 1 failed (`packages::real_packages_preserve_unknown_and_reject_mutations`; the five packages verify by hand) |
| matrix, docs index | 0 | regenerated with this gate |
| Python | 1 | 365 ran, 15 failed: 13 `/var` → `/private/var` alias (none with a resolved `TMPDIR`), 2 `__CF_USER_TEXT_ENCODING` |

Gate run 1 (at `e9f909c0`) found the vector-table and clippy failures fixed
above; runs 2 and 3 were stopped as superseded by later commits and the merge.
No analysis route changed, so no corpus arbiter was run.

## IEEE 754 floats on macOS

The float cards' native rows (W48-QR-FLOATING-POINT,
W48-QR-FLOAT-BUFFER-PAYLOADS) were already `cfg(unix)` and build through the
Apple `<threads.h>` shim, but compiled under a recorded target hard-coded to
`x86_64-unknown-linux-gnu`. The float route records the triple and reads only
the pointer width, so the verdicts were unaffected, but a Mac run carried a
Linux label. `float_types.rs`, `float_extern.rs`, `float_tasks.rs` and
`float_buffer_payloads.rs` now record `{ARCH}-apple-darwin` on macOS (Linux
unchanged; `float_ffi.rs` already used the running compiler's triple, and
`float_ieee_vectors.rs`'s generator target is a table identity, not a host).
Then, logs `private/5m-run3/float.log`:

```sh
cargo test -p qcore-compiler --test float_types --test float_extern --test float_tasks \
    --test float_buffer_payloads --test float_ffi --test float_ieee_vectors
```

Exit 0: 13 + 4 + 13 + 3 + 3 + 6 = 42 passed. That covers the native boundary
table bit-identical to the reference (NaN as NaN), renderings per stream and the
seeded renderer sample, the excess-precision/fast-math refusal, float extern
and FFI callees under the three builds, float task workers, and the buffer,
string, payload and protected-float rows. The `macos` CI job runs these
binaries too. `float_payload_rows_are_clean_under_thread_sanitizer` is not
among them: `tests/support/thread_sanitizer.rs` is Linux-only by its own
diagnosis (`W48-QR-FLOAT-APPLE-TSAN`).

## Boundaries

* One arm64 Mac. The CI job's first run is not read
  (`W48-QRLIB-MACOS-CI-RUN`).
* Intel macOS is out of scope: the current macOS no longer runs on Intel Macs (user decision, 2026-09-30), so no x86_64 run is registered. The reader's `x86_64-macos`
  handling stays, tested from bytes only; `x86_64h-macos` is not read for it.
* No sanitizer claim beyond what the three builds already state.

## Ledger

[followups.json](followups.json), applied by [apply_ledger.py](apply_ledger.py)
(`named_followups` 658 → 660 after merging main `0631c00b`): retired
`W48-QRLIB-MACOS`; registered `W48-QRLIB-MACOS-CI-RUN` (S),
`W48-QR-FLOAT-APPLE-TSAN` (M) and `W48-MACOS-GATE-HARNESS` (M). An Intel-run follow-up was drafted and dropped
on the user's decision above.
