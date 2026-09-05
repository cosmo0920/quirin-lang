# Cycle 16: Full Metatheory and Research Closure

**Status: Done**

Cycle 16 closes the numbered prototype roadmap for the language actually
implemented in this repository. The closure is bounded and compositional: it
does not relabel unimplemented research extensions as completed language
features.

## Rust implementation

`compiler/src/research_closure.rs` composes the independent Cycle 13–15
boundaries. `close_research_program` returns one certificate only when:

- source, cleanup CFG, and instrumented C11 observations validate;
- the concurrent event trace passes access, happens-before, capability,
  scoped-task, and custody checks;
- the advanced-control execution satisfies cancellation, cleanup, and capture
  restrictions; and
- sequential, concurrent, and advanced resource identities are each acquired
  or initially held exactly once and discharged exactly once in the closed
  product program.

The certificate retains all three component results, aggregate ledger counts,
and a terminal progress witness. Negative tests show that a target mismatch, a
concurrency defect, or undestroyed custody prevents closure.

The final acceptance run also compiles and executes the exhaustive Cycle 13
Boolean outcome corpus under Clang AddressSanitizer and
UndefinedBehaviorSanitizer when that toolchain is available.

## Lean verification

`proof/QCore/Metatheory.lean` defines the combined checked state and proves:

1. preservation for the implemented product step;
2. progress to a terminal state;
3. resource safety across the sequential, concurrent, and advanced ledgers;
4. race freedom inherited from the concurrent certificate;
5. simultaneous acceptance of usage, ownership, flow, and error interfaces;
   and
6. source/target observation agreement for the supported C11 boundary.

`proof/AuditCycle16.lean` audits all Cycle 16 theorem dependencies. No project
proof contains `sorry`, `admit`, or a project-defined axiom.

## Explicit research boundary

The closure does **not** claim principal inference, general nonzero-net loop
inference, unrestricted legacy C, categorical unification, full C11 weak-memory
semantics, or a native concurrent backend. These remain enumerated in both the
Rust and Lean models. The native target theorem applies to the supported
sequential C11 subset and its checked observation protocol; concurrency and
advanced control currently remain verified IRs.

## Verification

```sh
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
if rg -n '\b(sorry|admit)\b|^[[:space:]]*axiom\b' proof --glob '*.lean'; then exit 1; fi
cd proof
lake build
lake env lean AuditCycle16.lean
```

Cycle 16 completes the numbered development sequence. Further work must begin
as an explicitly scoped post-roadmap extension.
