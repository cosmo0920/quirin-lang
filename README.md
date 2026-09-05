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

The current implementation includes all sixteen completed bounded vertical slices: dimension
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
the conservative executable source subset; and independent source/CFG/C11
translation certificates plus a Clang-gated, interprocedural, field/array-aware
conservative legacy-C analysis slice with documented widening; and a restricted
concurrent event checker for access authority, vector-clock happens-before,
scoped tasks, mutex capabilities, lock order, and destruction custody; and
bounded advanced-control semantics for safe-point cancellation, fallible LIFO
cleanup, deterministic double errors, and linear higher-order capture; and a
combined closure certificate with preservation, progress, cross-ledger resource
safety, concurrent race freedom, and supported-C11 observation agreement.
Cycle 17 adds principal product inference certificates, nonzero-net bounded
loop invariants, a fail-closed unrestricted-C11 preprocessing gateway, explicit
weak-memory candidate executions, a native threaded custody backend, and
mechanically interpreted ledger morphisms. Its implementation is complete;
its native Linux/Windows platform gate now includes a recorded MSVC and
AddressSanitizer run. Cycle 18 adds a bounded, fail-closed Libevent event-reactor
checker with generation-safe lifecycle/custody tracking, explicit callback
authority, priority schedule exploration, cross-context conflict checks, source
pinning, and corresponding Lean theorems. Its Linux, native macOS/kqueue, and
native Windows/IOCP development evidence is recorded; Libevent sanitizer and
deployed observation-hook evidence remain pending. Cycle 19 applies the
unrestricted-C gateway to pinned Fluent Bit CMetrics, CTraces, librdkafka,
ChunkIO, the central input-chunk wrapper, and Monkey builds. It retains partial
summaries across reduced-parser failures, adds a bounded fail-closed ChunkIO
lifecycle checker without promoting raw units to Safe, and links the
forced-Libevent Monkey source back to the Cycle 18 trust boundary with checked
socket-pair, close, and error-query custody. The completed
Cycle 16 requirements and verification procedure are in
[`CYCLE_16.md`](docs/cycles/CYCLE_16.md). The promoted research milestone and its
implementation/acceptance record is in [`CYCLE_17.md`](docs/cycles/CYCLE_17.md). The subsequent Libevent
event-driven verification plan is in [`CYCLE_18.md`](docs/cycles/CYCLE_18.md).
The active Fluent Bit resource/event milestone is in
[`CYCLE_19.md`](docs/cycles/CYCLE_19.md). All
cycle records are archived under [`docs/cycles/`](docs/cycles/), including the
[Cycle 19 Fluent Bit library and chunk audit](docs/cycles/CYCLE_19_FLUENT_BIT_LIBRARIES_VALIDATION.md).
The subsequent extraction into versioned verifier providers is planned in
[`CYCLE_20.md`](docs/cycles/CYCLE_20.md).

The source front end is available through `parse_source_program` and
`elaborate_source`; the integrated checker is `check_whole_program`; and the
file compiler API is `compile_files`. The CLI binary is `qcorec`.

Cycle 8 preserves Cycle 7's validation on the non-MSVC C toolchain. The
[Cycle 7.5 validation](docs/cycles/CYCLE_7_5.md) now also compiles and executes the
same differential C corpus with native MSVC, strict warnings, and Windows
runtime checks.
The post-Cycle-9 sequence and promoted research boundary are in the
[post-Cycle-9 roadmap](docs/ROADMAP_AFTER_CYCLE_9.md).

## Build

```sh
git submodule update --init --recursive
cargo test --workspace
cd proof
lake build
```

The Rust test suite verifies that `third_party/libevent` is checked out at the
audited revision and still contains the source contracts used by the Cycle 18
model. The CI workflow initializes the submodule before running this gate.

The Lean toolchain is selected through `proof/lean-toolchain`. Elan must have a
Lean 4 stable toolchain installed before `lake build` can run.
