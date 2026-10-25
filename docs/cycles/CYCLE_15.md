# Cycle 15: Advanced Control and Cleanup

**Status: Done**

Evaluation profile: none (a language and model cycle: its acceptance is the standing gate's Rust tests and Lean audits; it claims no audit measurement over a pinned source set).

Cycle 15 implements bounded fragments for asynchronous cancellation, fallible
destructors, deterministic double-error handling, and higher-order ownership
capture. It intentionally does not add unrestricted control effects.

## Rust implementation

`compiler/src/advanced_control.rs` provides an executable advanced-control IR:

- cancellation requests are pending until an explicit safe point (a return is
  also a safe point);
- live resources are destroyed exactly once in reverse acquisition order;
- destructor failures on a normal exit produce `CleanupFailure`;
- destructor failures during an error or cancellation produce `DoubleError`
  while preserving the original primary exit;
- owned captures are unique, permitted only for one-shot handlers, and must be
  consumed by exactly one invocation on every accepted exit; and
- resumable and multi-shot handlers may be invoked repeatedly only when their
  captures are shared rather than owned.

The slice does not implement stack copying, continuation serialization,
signals, forced preemption, or foreign unwinding. Cancellation delivery is
cooperative and the double-error value is a research-level semantic boundary,
not yet part of the Cycle 12 C ABI.

## Lean verification

`proof/QCore/AdvancedControl.lean` proves LIFO cleanup and cleanup linearity,
that secondary cleanup failures preserve an existing error or cancellation,
and that accepted owned handler captures are unique, one-shot, and invoked
exactly once. `proof/AuditCycle15.lean` audits these theorems.

## Verification

```sh
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
if rg -n '\b(sorry|admit)\b|^[[:space:]]*axiom\b' proof --glob '*.lean'; then exit 1; fi
cd proof
lake build
lake env lean AuditCycle15.lean
```

Cycle 16 is the next implementation milestone.
