# QCore-C agent briefing

Consolidated, compressed project state for LLM agents. Humans should read
[README.md](README.md). Keep this file, `CLAUDE.md`, and `README.md` in sync
when project status changes; this file is the source of truth for agents.

## What this project is

QCore-C is a research language for quantitative types, owned resources, typed
recoverable errors, checked cleanup, and checked concurrency, lowering to C11.
It also ships a conservative checker for *existing* C source (the
"unrestricted-C gateway"). Normative design: `docs/QCORE_SPEC_r0_2.md`
(revision 0.2; §§0–37 inherited from `docs/qcore_quantitative_types_spec.md`,
§§38–59 the concurrency amendment). Work proceeds in numbered bounded
"cycles" recorded under `docs/cycles/`.

## Repository map

| Path | Contents |
|---|---|
| `compiler/` | Single Rust crate `qcore-compiler` (lib `qcore_compiler`, bin `qcorec`): parser, checker, reference semantics, C backend, legacy-C analyzer, verifier-plugin host, first-party providers. Monolithic today; decomposition target in `docs/GENERALIZATION_PLAN.md` §3 |
| `compiler/src/legacy_c.rs` | Generic unrestricted-C checker (Clang gateway, CFG, resource abstract interpretation, certificate recheck); contracts enter via `ResourceContracts` |
| `compiler/src/verifier_plugin.rs` | Cycle 20 plugin protocol v1, manifests, registry, process providers, certificate firewall |
| `compiler/src/verifier_plugins/{libevent,chunkio}/` | In-process first-party providers (Libevent/Monkey, ChunkIO) |
| `compiler/src/external_{integration,source}.rs` | Cycle 19 library custody mechanisms + pinned source-set tables (over-fitted; see plan) |
| `compiler/examples/cycle19_fluent_bit_libraries_audit.rs` | Production audit orchestration for the pinned Fluent Bit corpus |
| `contract/qcore/` | Declarative `.qcore` contract corpus (45 files); counts pinned by `compiler/tests/qcore_contract_corpus.rs` |
| `proof/` | Lean 4 executable core and safety theorems; `AuditCycleN.lean` print axiom surfaces |
| `docs/cycles/` | Per-cycle requirement/acceptance records (`CYCLE_2` … `CYCLE_20`) |
| `docs/reports/` | Archived evidence packages (pinned audits, observations, triage) |
| `docs/reevaluation/` | Raw audit logs, probe JSON, `ct_manifest.json` (CT01–CT50 tracking) |
| `third_party/libevent` | Submodule pinned to the audited revision; a test fails if absent or moved |

## Status ledger (do not overstate)

- **Cycles 1–16: complete.** Sequential language (dimensions, quant axes,
  interval usage grades, ownership, resource flow, typed errors, error-region
  cleanup, CFG lowering to tagged-error C11 ABI, executable small-step model),
  structural front end, whole-program checker, multi-file driver with CLI,
  translation certificates, annotated legacy-C slice, restricted concurrency
  checker, advanced control, combined closure certificate.
- **Cycle 17: complete.** Principal product inference, nonzero-net loop
  invariants, fail-closed unrestricted-C11 preprocessing gateway, explicit
  weak-memory candidate executions, native threaded custody backend, ledger
  morphisms. MSVC + ASan platform evidence recorded.
- **Cycle 18: bounded implementation complete; aggregate acceptance pending.**
  Libevent event-reactor checker (generation-safe lifecycle/custody, callback
  authority, priority schedules, cross-context conflicts, source pinning) +
  Lean theorems. Linux/macOS-kqueue/Windows-IOCP evidence recorded; Libevent
  sanitizer and deployed observation-hook evidence pending.
- **Cycle 19: in progress.** Unrestricted-C gateway applied to pinned Fluent
  Bit (CMetrics, CTraces, librdkafka, ChunkIO, input-chunk wrapper, Monkey).
  Latest triage: 185 TUs, 4,775 raw functions, 98.2% projected — **2 Safe /
  98 provisional Unsafe / 81 Unknown**. **Raw units remain unpromoted** until
  per-path identity imports and real observation evidence satisfy the
  fail-closed gates in `docs/cycles/CYCLE_19.md`.
  Source-bound context/stream/chunk/backend teardown now composes bounded
  memory model identities and 43 conditional mixed-backend cases; 26 retain
  unresolved OS custody after parent free. Five sanitized native memory cases
  now bind constructor allocations, rings and exact nested release order (82
  generations, 13 chunks). Nine injected allocation failures plus a control
  account for 55 more generations, including six rollback releases. Null chmod
  and chunk-name success outcomes and zero error output on backend failure are
  preserved. General caller/failure provenance, full sync/OS guarantees,
  aggregate custody, remaining wrapper lifecycle and full Monkey/Cycle 18
  composition remain open. Latest evidence:
  `docs/reports/cycle19-constructor-failures/README.md` (400 workspace tests,
  45 contract modules; source/verdict parity preserved).
- **Cycle 20: active.** Verifier-plugin ecosystem. Protocol v1 carries
  Boolean assertions only: built-ins return `Unknown`, the core **refuses all
  v1 `Safe` proposals** (including empty scopes), unchecked `Unsafe`
  proposals downgrade to `Unknown` (`QP010`). `verify scope` in source is
  configuration validation only — compiling a resolved scope fails with
  `Q701`; obligation generation and dispatch are unimplemented (finding VP05).
- **Reevaluation (2026-09-05): 6 + 7 findings, all repaired**
  (`docs/REEVALUATION_SPEC.md`, `docs/REEVALUATION_FOLLOWUP.md`). Open
  boundaries that remain true statements: interprocedural parameter/return
  custody inference is **unimplemented** (LC01); semantic witness schema v2
  is unimplemented; process providers have process-group containment, not an
  OS sandbox; mutex model supports initially-unlocked non-recursive mutexes
  only; CT01–CT50 conformance corpus is **100% unmapped**.
- **Active direction:** `docs/GENERALIZATION_PLAN.md` — generalize the
  checker away from pinned source sets and decompose the monolithic crate
  into a pluggable contract/verification system (workstreams W0–W7). W1 is
  partially executed: `compiler/src/contract_loader.rs` (total-or-reject v1)
  feeds the Cycle 19 audit from a contract manifest. Execution is tracked in
  `docs/ROADMAP_AFTER_CYCLE_9.md`'s workstream table.

## Hard invariants (violating these is a defect, not a style issue)

1. **Fail-closed.** Missing contracts, unmatched profiles, unsupported
   constructs, provider absence/failure/mismatch, unproven call effects →
   `Unknown`. Never optimistic fallback. Refactors must not flip any
   `Unknown` to `Safe`.
2. **Verdict contract.** Every published result carries
   `(artifact, property, semantic profile, scope, bounds, assumptions,
   evidence version)`. `Safe` only after all obligations discharge; `Unsafe`
   only for a validated reachable violation; everything else `Unknown`.
   Projection coverage is not safety coverage; unknowns stay in the
   denominator; a provisional `Unsafe` is never presented as an upstream
   defect.
3. **Parity before deletion.** A hardcoded analysis route is removed only
   after its replacement reproduces the pinned verdicts/coverage, with the
   diff archived under `docs/reports/`.
4. **Gate ordering** (`docs/REEVALUATION_SPEC.md` §9): certificate
   containment (G1) before provider expansion; Cycle 19 routes stay until
   G4 parity.
5. **Pinned expectations are intentional.** `qcore_contract_corpus.rs` pins
   per-file contract/clause counts (editing `contract/qcore` requires
   updating its tables); a test asserts the `third_party/libevent` submodule
   revision; pinned commit hashes are load-bearing identity checks.
6. **Lean parity.** Semantic changes need matching Lean definitions/theorems;
   audits must stay placeholder-free with no project axioms.

## Standing per-change gate

Run before claiming any change complete:

```sh
git submodule update --init --recursive   # once per checkout
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo run -p qcore-compiler --example reevaluation_probe      # expect exit 0
cargo run -p qcore-compiler --example reevaluation_followup   # expect exit 0
cd proof && lake build
```

Clang is required for existing-C analysis tests (`QCORE_CLANG` overrides
discovery; invalid/empty value is an error, never a silent fallback). `qcorec
check`/`emit` and generated-C tests work without Clang (MSVC on Windows).
Lean toolchain via `proof/lean-toolchain` (elan).

## Key entry points

- Library: `parse_source_program`, `elaborate_source`, `check_whole_program`,
  `compile_files` / `compile_files_with_registry`,
  `analyze_unrestricted_c11` / `validate_unrestricted_c11`.
- CLI: `qcorec check <file>`, `qcorec emit -o out.c <file>`,
  `qcorec plugins list|inspect <id>`.
- Diagnostics namespaces: `Q1xx–Q4xx` core, `Q6xx` concurrency, `Q7xx`
  event/scope (note: `Q700/Q701` currently collide between spec §54 and
  Cycle 20 scope resolution — see plan W7), `QP***` plugin protocol.

## Where to read more

Direction and tasks: `docs/GENERALIZATION_PLAN.md`. Architecture boundary and
open gates: `docs/cycles/CYCLE_20.md`. Trust/verdict rules and audit history:
`docs/REEVALUATION_SPEC.md`, `docs/REEVALUATION_FOLLOWUP.md`. Language:
`docs/QCORE_SPEC_r0_2.md`, `docs/QCORE_MODULE_MODES.md`,
`docs/QTT_ALIGNMENT.md`. Contract corpus conventions:
`contract/qcore/README.md`.
