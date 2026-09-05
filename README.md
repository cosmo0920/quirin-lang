# QCore-C

QCore-C is a research language for quantitative types, owned resources, typed
recoverable errors, and checked cleanup before lowering to C11. The normative
design is in [`qcore_quantitative_types_spec.md`](qcore_quantitative_types_spec.md).

The implementation is split into:

- `compiler/`: the Rust parser, checker, reference interpreter, and C backend;
- `proof/`: the Lean 4 executable core and safety proofs.

The current implementation includes the first three vertical slices: dimension
and quantitative-axis declarations, interval-graded usage demand analysis, and
ownership without exceptional exits. Active cycle requirements, including
continuous Lean verification, are in [`CYCLE_3.md`](CYCLE_3.md). Completed cycle
records are archived under [`docs/cycles/`](docs/cycles/).

## Build

```sh
cargo test --workspace
cd proof
lake build
```

The Lean toolchain is selected through `proof/lean-toolchain`. Elan must have a
Lean 4 stable toolchain installed before `lake build` can run.
