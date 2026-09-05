# QCore-C

QCore-C is a research language for quantitative types, owned resources, typed
recoverable errors, checked cleanup, and checked concurrency before lowering
to C11. The consolidated proposed revision 0.2 design is in
[`docs/QCORE_SPEC_r0_2.md`](docs/QCORE_SPEC_r0_2.md); its inherited sequential
baseline remains in
[`docs/qcore_quantitative_types_spec.md`](docs/qcore_quantitative_types_spec.md).

The implementation is split into:

- `compiler/`: the Rust parser, checker, reference interpreter, and C backend;
- `proof/`: the Lean 4 executable core and safety proofs.

The current implementation includes the first twelve vertical slices: dimension
and quantitative-axis declarations, interval-graded usage demand analysis,
ownership without exceptional exits, and resource-flow summaries with checked
peak and cumulative bounds, followed by path-sensitive typed errors without
cleanup packs, sealed error-region cleanup with checked `quant open` and linear
discharge, cleanup-explicit CFG lowering to a tagged-error C11 ABI, and a
reduced executable small-step model integrating natural indices, usage grades,
owned identities, typed errors, and one error region, followed by a
conservative fixed-point resource analyzer for an annotated legacy-C subset;
a complete structural source front end with byte spans, name resolution, type
classification, one typed source IR, and pass-attributed diagnostics; and a
whole-program checker that connects the sequential semantic passes, computes
call summaries, checks recursive contracts, transfers resource identities
across calls, and validates accepted source paths against checked CFGs; and an
end-to-end multi-file compiler driver with a stable CLI, source-located
diagnostics, emitted-source mapping, and native-C differential validation for
the conservative executable source subset.
Active cycle requirements, including the complete Rust/C/Lean verification procedure,
are in [`CYCLE_12.md`](CYCLE_12.md). Completed cycle records are archived under
[`docs/cycles/`](docs/cycles/).

The source front end is available through `parse_source_program` and
`elaborate_source`; the integrated checker is `check_whole_program`; and the
file compiler API is `compile_files`. The CLI binary is `qcorec`.

Cycle 8 preserves Cycle 7's validation on the non-MSVC C toolchain. The
[Cycle 7.5 validation](docs/CYCLE_7_5.md) now also compiles and executes the
same differential C corpus with native MSVC, strict warnings, and Windows
runtime checks.
The sequenced Cycle 12 and later work is in the
[post-Cycle-9 roadmap](docs/ROADMAP_AFTER_CYCLE_9.md).

## Build

```sh
cargo test --workspace
cd proof
lake build
```

The Lean toolchain is selected through `proof/lean-toolchain`. Elan must have a
Lean 4 stable toolchain installed before `lake build` can run.
