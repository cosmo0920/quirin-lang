# QCore-C

QCore-C is a research language for quantitative types, owned resources, typed
recoverable errors, and checked cleanup before lowering to C11. The normative
design is in [`qcore_quantitative_types_spec.md`](qcore_quantitative_types_spec.md).

The implementation is split into:

- `compiler/`: the Rust parser, checker, reference interpreter, and C backend;
- `proof/`: the Lean 4 executable core and safety proofs.

The current first vertical slice supports dimension and quantitative-axis
declarations, including parse/pretty-print round trips and static validation.

## Build

```sh
cargo test --workspace
cd proof
lake build
```

The Lean toolchain is selected through `proof/lean-toolchain`. Elan must have a
Lean 4 stable toolchain installed before `lake build` can run.

