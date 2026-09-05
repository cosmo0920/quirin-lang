# Cycle 7.5: Native Windows/MSVC Conformance

**Status: Done**

Cycle 7.5 validates the Cycle 7 C emitter on native Windows with the MSVC
toolchain. It remains a separate portability result rather than retroactively
changing the completed Cycle 7 acceptance result.

## Rust and C work

- The compiled-C integration harness asks the Rust `cc` crate to select the
  compiler for the native Rust host target.
- On `target_env = "msvc"`, the test rejects `clang-cl` and requires the native
  MSVC command-line family selected by `cc`.
- MSVC compilation uses `/std:c11`, `/TC`, `/W4`, and `/WX`, with `/RTC1` and
  `/sdl` providing the available debug-runtime and security checks.
- GNU/Clang compilation retains the Cycle 7 C11 warning and
  AddressSanitizer/UndefinedBehaviorSanitizer options unchanged.
- The native executable path uses the Windows `.exe` suffix and MSVC's `/Fe:`
  output option.
- The normal-transfer, partial-initialization, LIFO cleanup, error-tag, and
  signed-value corpus is compiled and executed on MSVC. Each result is compared
  with both the source and cleanup-CFG interpreters.
- The generated harness frees every allocation transferred in a successful
  result; failure paths exercise reverse-order cleanup of initialized slots.

## Acceptance criteria

- The `cc` crate selects MSVC and successfully compiles every emitted test
  program with warnings treated as errors.
- Compiled Windows observations match the source and CFG interpreters.
- Available native runtime instrumentation reports no invalid generated-code
  behavior.
- Each acquired runtime identity is released or transferred exactly once, and
  the test harness releases every transferred allocation.
- Cycle 7's non-MSVC output and tests remain unchanged.

## Verification procedure

Cycle 7.5 changes only the platform-specific Rust integration harness and adds
no core semantic definitions or proof obligations. Its verification therefore
intentionally excludes the Lean build and theorem audit:

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

## Verified completion result

On a native `x86_64-pc-windows-msvc` host, the `cc` crate selected native
`cl.exe` from MSVC 14.51.36231. All four emitted C programs compiled with
warnings treated as errors, ran with the MSVC runtime checks enabled, and
matched both Rust interpreters. The complete 90-test Rust workspace suite and
strict Clippy check also passed. No Lean verification step was run because
Cycle 7.5 changes no core semantics.
