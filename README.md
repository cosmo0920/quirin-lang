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
summaries across reduced-parser failures, imports structured and named C
control flow as explicit CFG edges, adds counted ref/list/queue/arena and
evidence-gated concurrency custody, derives scoped mechanism profiles from
pinned librdkafka, CFL, and ChunkIO source contracts, composes complete ChunkIO
imports with the bounded lifecycle checker, and joins exact Monkey
callback/failure contracts plus dispatcher/backend observations to the Cycle
18 event certificate. Raw units remain unpromoted until per-path identity
imports and real observation evidence satisfy those fail-closed gates. The
library, ChunkIO-import, and Monkey-composition contracts also have native,
compiler-checked QCore regression sources under
[`contract/qcore`](contract/qcore/README.md); the front end retains each
contract clause as structured source syntax for future semantic passes.
The completed Cycle 16 requirements and verification procedure are in
[`CYCLE_16.md`](docs/cycles/CYCLE_16.md). The promoted research milestone and its
implementation/acceptance record is in [`CYCLE_17.md`](docs/cycles/CYCLE_17.md). The subsequent Libevent
event-driven verification plan is in [`CYCLE_18.md`](docs/cycles/CYCLE_18.md).
The active Fluent Bit resource/event milestone is in
[`CYCLE_19.md`](docs/cycles/CYCLE_19.md). All
cycle records are archived under [`docs/cycles/`](docs/cycles/), including the
[Cycle 19 Fluent Bit library and chunk audit](docs/cycles/CYCLE_19_FLUENT_BIT_LIBRARIES_VALIDATION.md).
The subsequent extraction into versioned verifier providers is tracked in
[`CYCLE_20.md`](docs/cycles/CYCLE_20.md). Cycle 20 is now active; its foundation includes versioned
length-delimited JSON verifier requests, fail-closed registry dispatch,
process-provider timeout/output limits, a certificate firewall,
first-party in-process migration scaffolding, and
source-level `verify scope` declarations. Provider-specific Cycle 19 parity
and standalone package hardening remain tracked in the Cycle 20 record.
Protocol v1 carries caller assertions without semantic witnesses: built-ins
return `Unknown`, and the core refuses all v1 `Safe` proposals, including empty
evidence. Unchecked `Unsafe` proposals also become `Unknown` (`QP010`).
Manifest capabilities describe routing only. Use `resolve_source_scopes` for
configuration validation; compiling a source-level scope fails with `Q701`
until obligation generation and dispatch are implemented. Unix process providers
use nonblocking pipes and process-group cleanup; request/response budgets count
JSON payload bytes, excluding the four-byte header. See the
[follow-up repair record](docs/REEVALUATION_FOLLOWUP.md#repair-and-containment-on-2026-09-05)
for the ownership, mutex, and provider boundaries.

The source front end is available through `parse_source_program` and
`elaborate_source`; the integrated checker is `check_whole_program`; and the
file compiler API is `compile_files`. The CLI binary is `qcorec`.

The [QTT alignment record](docs/QTT_ALIGNMENT.md) describes enforced source
parameter usage grades, per-invocation lambda checking in the reduced usage IR,
and the executable Lean demand checker. General source lambdas and dependent
function typing remain outside this implementation boundary.

Cycle 8 preserves Cycle 7's validation on the non-MSVC C toolchain. The
[Cycle 7.5 validation](docs/cycles/CYCLE_7_5.md) now also compiles and executes the
same differential C corpus with native MSVC, strict warnings, and Windows
runtime checks.
The post-Cycle-9 sequence and promoted research boundary are in the
[post-Cycle-9 roadmap](docs/ROADMAP_AFTER_CYCLE_9.md).

The [reevaluation specification](docs/REEVALUATION_SPEC.md) records a recent
research review, six reproduced certificate/weak-memory discrepancies, and
the proposed acceptance gates across the language, backends, and providers.
The [six-finding repair record](docs/REEVALUATION_SPEC.md#six-finding-repair-on-2026-09-05)
documents the memory fixes, plugin certificate containment, and regression results.

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
