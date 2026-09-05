# QCore-C

QCore-C is a research language for quantitative types, owned resources, typed
recoverable errors, and checked cleanup before lowering to C11. The normative
design is in [`qcore_quantitative_types_spec.md`](qcore_quantitative_types_spec.md).

The implementation is split into:

- `compiler/`: the Rust parser, checker, reference interpreter, and C backend;
- `proof/`: the Lean 4 executable core and safety proofs.

The current implementation includes the first six vertical slices: dimension
and quantitative-axis declarations, interval-graded usage demand analysis,
ownership without exceptional exits, and resource-flow summaries with checked
peak and cumulative bounds, followed by path-sensitive typed errors without
cleanup packs, and sealed error-region cleanup with checked `quant open` and
linear discharge. Active cycle requirements, including the complete Lean
verification procedure, are in [`CYCLE_6.md`](CYCLE_6.md). Completed cycle
records are archived under [`docs/cycles/`](docs/cycles/).

## Build

```sh
cargo test --workspace
cd proof
lake build
```

The Lean toolchain is selected through `proof/lean-toolchain`. Elan must have a
Lean 4 stable toolchain installed before `lake build` can run.
