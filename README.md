# QCore-C

QCore-C is a research language for quantitative types, owned resources, typed
recoverable errors, and checked cleanup before lowering to C11. The normative
design is in [`qcore_quantitative_types_spec.md`](qcore_quantitative_types_spec.md).

The implementation is split into:

- `compiler/`: the Rust parser, checker, reference interpreter, and C backend;
- `proof/`: the Lean 4 executable core and safety proofs.

The current implementation includes the first eight vertical slices: dimension
and quantitative-axis declarations, interval-graded usage demand analysis,
ownership without exceptional exits, and resource-flow summaries with checked
peak and cumulative bounds, followed by path-sensitive typed errors without
cleanup packs, sealed error-region cleanup with checked `quant open` and linear
discharge, cleanup-explicit CFG lowering to a tagged-error C11 ABI, and a
reduced executable small-step model integrating natural indices, usage grades,
owned identities, typed errors, and one error region. Active
cycle requirements, including the complete Rust/C/Lean verification procedure,
are in [`CYCLE_8.md`](CYCLE_8.md). Completed cycle records are archived under
[`docs/cycles/`](docs/cycles/).

The source language is not yet end-to-end: the human-facing parser currently
accepts dimension and quantitative-axis declarations, while the later semantic
passes and C backend are checked Rust IR APIs. A full function grammar and CLI
compiler remain future work.

Cycle 8 preserves Cycle 7's validation on the current non-MSVC C toolchain.
Native Windows/MSVC conformance remains explicitly postponed to the
[Cycle 7.5 plan](docs/CYCLE_7_5.md); no Cycle 7.5 implementation is included.

## Build

```sh
cargo test --workspace
cd proof
lake build
```

The Lean toolchain is selected through `proof/lean-toolchain`. Elan must have a
Lean 4 stable toolchain installed before `lake build` can run.
