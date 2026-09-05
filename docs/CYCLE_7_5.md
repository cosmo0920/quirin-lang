# Cycle 7.5: Native Windows/MSVC Conformance

**Status: Postponed**

Cycle 7.5 will validate and, where necessary, adapt the Cycle 7 C emitter for
native Windows and MSVC. It is not part of the completed Cycle 7 acceptance
result and must be run later in an environment with a native MSVC toolchain.

## Planned Rust and C work

- Select the native MSVC compiler through the Rust `cc` crate.
- Replace GNU/Clang-only test flags with MSVC-appropriate C language, strict
  warning, and runtime-instrumentation options.
- Confirm that emitted fixed-width values, enum tags, payload unions, labels,
  initialization slots, and ownership moves compile cleanly as C under MSVC.
- Compile and execute the same normal-transfer, partial-initialization, LIFO
  cleanup, error-tag, and signed-value corpus used by Cycle 7.
- Compare each Windows executable's observable value or error, payload
  identities, and resource trace with both Rust interpreters.
- Add focused regressions for any required Windows-specific emitter changes.

## Acceptance criteria

- The `cc` crate selects MSVC and successfully compiles every emitted test
  program with warnings treated as errors.
- Compiled Windows observations match the source and CFG interpreters.
- Available native runtime instrumentation reports no invalid generated-code
  behavior.
- Each acquired runtime identity is released or transferred exactly once, and
  the test harness releases every transferred allocation.
- Cycle 7's non-MSVC output and tests remain unchanged.

## Current boundary

The Cycle 7 compiled-C integration test is intentionally inactive for the
`msvc` target environment. It emits an explicit test note pointing to this
cycle instead of treating GNU/Clang flags as an MSVC validation. This deferral
must be removed only when the native Windows matrix above is implemented and
verified.
