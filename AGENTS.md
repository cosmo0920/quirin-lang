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
| `compiler/` | Rust library crate `qcore-compiler` (lib `qcore_compiler`): parser, checker, reference semantics, C backend, gateway compatibility API, verifier-plugin host, first-party providers. Still the L1/L2 grab-bag; decomposition target in `docs/GENERALIZATION_PLAN.md` §3 |
| `crates/qcore-core/` | L1 semantic-kernel seed (W1 relocation): `usage`, `ast`, `index`. No workspace dependencies; the rest of the kernel arrives at W5 consolidation |
| `crates/qcore-frontend/` | L1 front end (W1 relocation): `lexer`, `parser`, `check`, `quant_index`, `frontend` elaboration. Depends on `qcore-core` only |
| `crates/qcore-contract/` | L1 contract machinery (W1 relocation): total-or-reject `contract_loader`, `contract_observation`. Depends on core/frontend/schemas; `chunk_contract` joins after the W5 chunk-summary schema split |
| `crates/qcore-schemas/` | L0 published schema surface (W0 executed 2026-09-06): plugin protocol v1 types, resource-contract data model, mechanism vocabulary, source-identity records. Serde types only; a test forbids workspace dependencies |
| `crates/qcore-c-gateway/` | L2 existing-C analyzer, custody summary inference, Clang discovery and typed-AST transport; depends only on contract/schemas plus JSON transport |
| `crates/qcorec/` | CLI binary `qcorec` plus its process-level tests |
| `crates/qcore-c-gateway/src/legacy_c.rs` | Generic unrestricted-C checker (Clang gateway, CFG, resource abstract interpretation, certificate recheck); contracts enter via `ResourceContracts` |
| `crates/qcore-c-gateway/src/legacy_summary.rs` | W4 interprocedural custody summary inference for internal C functions: SCC-ordered derivation, validation by re-analysis, declared-contract precedence. Relocated with Clang and typed-AST transport (2026-09-07); compiler compatibility reexports retained |
| `compiler/src/verifier_plugin.rs` | Cycle 20 plugin protocol v1, manifests, registry, process providers, certificate firewall |
| `compiler/src/verifier_plugins/{libevent,chunkio}/` | In-process first-party providers (Libevent/Monkey, ChunkIO) |
| `compiler/src/external_{integration,source}.rs` | Cycle 19 library custody mechanisms + pinned source-set tables (over-fitted; see plan) |
| `compiler/examples/cycle19_fluent_bit_libraries_audit.rs` | Production audit orchestration for the pinned Fluent Bit corpus |
| `contract/qcore/` | Declarative `.qcore` contract corpus (56 files); counts pinned by `compiler/tests/qcore_contract_corpus.rs` |
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
  composition remain open. The unsupported backend close arm is now classified
  symbolically: outer releases retain unresolved backend custody and an explicit
  `Unknown`; constructor reachability is not inferred. Conditional parent teardown
  now preserves exact opaque stream/chunk/backend-kind identities. Four sanitized
  unsupported-type constructor cases return null chunks with unchanged errors;
  all 24 generations are released (eight rollback, sixteen teardown). Twelve
  sanitized real Linux cleanup calls now bind close/unmap/delete endpoints to
  exclusive temporary fixtures, including inode witnesses and retained mapping
  after invalid-length unmap failure. They do not establish enclosing filesystem
  teardown, arbitrary OS failure behavior or durability. Six further native
  one-file constructor/teardown cases account for 66 allocation generations.
  Suppressed unmap/close calls returning EIO leave observed mapping/descriptor
  residuals after parent free; failed sync still allows cleanup. These fixtures
  do not prove general filesystem callers, failures or durability. Twelve filesystem
  allocation-failure/control processes now account for 81 acquired generations:
  73 recorded releases and eight outstanding prefix generations across two UBSan
  null-argument terminations. Ten returning cases complete leak checking; the two
  interrupted cases do not establish teardown. Failed root/chunk-name copies,
  null chmod success and a stream returned without a directory stay distinct.
  Eight further constructor OS-failure/control processes release all 88 allocation
  generations. Fstat/stat fallback and POSIX resize return/errno stay distinct;
  suppressed mapping plus rollback-close failures retain the original descriptor/
  inode after null chunk return and parent release. Harness reclamation is separate.
  These observations do not prove arbitrary kernel failure post-states.
  Seven existing-file profiles release all 70 allocation generations while
  retaining original inode/content and distinct corruption/read-only errors.
  Suppressed chmod plus rollback-close failures preserve descriptor custody after
  parent release; harness reclamation stays separate. General profiles remain open.
  Five native residency/transaction/lock sequences release all 50 allocation
  generations and seven OS epochs. Rollback preserves mapped header length even
  while restoring logical length; down/up reloads appended bytes. Locked begin
  returns retry while direct write still succeeds. These fixed observations do
  not establish production reachability or general transaction/lock guarantees.
  Three native one-shot sync-failure sequences release 30 more generations and
  four OS epochs. Failed commit preserves the active transaction; retry clears
  it. Down still unmaps/closes after failed sync, and unmapped commit clears the
  transaction without syncing. These are fixed injected observations, not general
  synchronization or durability guarantees.
  Four failed-down sequences release 40 more generations and seven OS epochs.
  Suppressed unmap retains mapping/descriptor identity, up ring and counter;
  prior successful sync survives failure, while combined sync/unmap failure
  stays unsynced. Retry or parent teardown later releases the original epoch.
  Four descriptor-failure sequences release 40 more generations and acquire five
  OS epochs. Failed close during down leaves the original descriptor in the down
  ring with no mapping and a zero up counter; retries of down/up reject. Parent
  teardown releases that descriptor unless its close is also suppressed, retaining
  one observed residual after parent free. Harness reclamation is separate.
  Failed reopen acquires no epoch; successful retry acquires a fresh one. General
  reopen failures, caller provenance, aggregate custody and event composition
  remain open. Latest evidence:
  `docs/reports/cycle19-file-descriptor-failures/README.md`
  (407 workspace tests, 56 contract modules; prior source/verdict parity preserved).
- **Cycle 20: active.** Verifier-plugin ecosystem. Protocol v1 carries
  Boolean assertions only: built-ins return `Unknown`, the core **refuses all
  v1 `Safe` proposals** (including empty scopes), unchecked `Unsafe`
  proposals downgrade to `Unknown` (`QP010`). W3's bounded evidence-v2
  increment derives source obligations and dispatches resolved scopes; trusted
  observation replay can certify recorded agreement only, under the explicit
  `observations_trusted` assumption and observed-run bounds. A standalone
  schemas-only process fixture exercises positive and forged evidence.
  Lifecycle source effect/ABI import stays incomplete, all unsupported and
  empty inventories stay `Unknown`, and first-party v1 providers remain
  unpromoted. Host relocation and production profile/source import remain open.
  See `docs/reports/w3-evidence-v2/README.md`.
- **Reevaluation (2026-09-05): 6 + 7 findings, all repaired**
  (`docs/REEVALUATION_SPEC.md`, `docs/REEVALUATION_FOLLOWUP.md`). Open
  boundaries that remain true statements: interprocedural parameter/return
  custody inference is **implemented for parameter/return custody with
  validation** (LC01, W4, extended 2026-09-07 with validated `Out`,
  `Stored`, `Reallocated` and possibly-null returns; `Removed`, recursive
  call-graph components, and cost summaries are still not inferred, and every such body
  keeps its callers `Unknown`); semantic witness schema v2 is implemented for bounded
  replay (production lifecycle import remains unimplemented); process providers have process-group containment, not an
  OS sandbox; mutex model supports initially-unlocked non-recursive mutexes
  only; CT01–CT50 conformance corpus is **45/50 unmapped**
  (CT02/CT12/CT16/CT19/CT37 are mapped as sequential-core source fixtures;
  see the W6 note below).
- **Active direction:** `docs/GENERALIZATION_PLAN.md` — generalize the
  checker away from pinned source sets and decompose the monolithic crate
  into a pluggable contract/verification system (workstreams W0–W7). W0 is
  executed (2026-09-06, `docs/reports/w0-schema-crate/README.md`): the
  workspace has three members (`qcore-schemas` L0 surface, `qcore-compiler`
  grab-bag, `qcorec` CLI) with a layering-guard test; providers remain
  compile-time crates for now (decision recorded there — the plan's
  out-of-process step is deferred, not abandoned). W1's loader
  (total-or-reject v1) feeds the Cycle 19 audit from a contract manifest,
  and its relocation is executed (2026-09-06,
  `docs/reports/w1-contract-crate/README.md`): `qcore-core`,
  `qcore-frontend`, and `qcore-contract` are extracted with a
  dependency-matrix guard test. The leftover literal contract sites are
  migrated to loader-fed `.qcore` fixtures (2026-09-06,
  `docs/reports/w1-literal-sites/README.md`). **W1 is executed**
  (2026-09-06, `docs/reports/w1-clause-vocabulary/README.md`): the
  `requires resource_contract_v1` marker is pinned corpus-wide as the
  interpreter selector — 19 declarations load and 141 are rejected, not
  skipped — the clause-family → owner table is documented in the loader
  header and `contract/qcore/README.md`, and the C ABI scalar type table is
  complete (borrowed-only, no semantic change; audit diagnostics/callbacks
  byte-identical). Unifying the resource loader with the observation/
  lifecycle interpreters is W5-adjacent; non-null error sentinels need
  target-interface work, not loader vocabulary. W6's CT harness is stood up
  (2026-09-06, `docs/reports/w6-ct-harness/README.md`):
  `compiler/tests/ct_corpus.rs` executes every case
  `docs/reevaluation/ct_manifest.json` marks mapped through the real source
  pipeline and fails closed on manifest/fixture disagreement; CT02, CT12,
  CT16, CT19 and CT37 are mapped as Cycle 12 executable-subset fixtures
  exercising the sequential ownership/typed-error/cleanup/completion-record
  core only (spawn/capture syntax, shares, mutexes and the concurrency
  backend remain unexercised). Batch 2 (2026-09-06,
  `docs/reports/w6-ct-batch-2/README.md`) adds CT19/CT37 and records the
  full triage of all 47 then-unmapped cases (2 decidable-now, 8 blocked on
  W3/W4, 33 blocked on unimplemented syntax, 4 ambiguous); 45 cases remain
  unmapped, counted by a pinned test. W4 is executed for
  parameter/return custody with
  validation (2026-09-06, `docs/reports/w4-custody-inference/README.md`):
  `crates/qcore-c-gateway/src/legacy_summary.rs` derives SCC-ordered custody summaries for
  internal C functions, publishes only summaries that a re-run of the
  intraprocedural analyzer validates, expresses them as `qcore-schemas`
  `ResourceSummaryRecord`s under `ContractTrust::InferredValidated`, and has
  the certificate recheck re-derive them independently. Declared contracts
  always win; a disagreement is a `SummaryConflict` diagnostic, never a silent
  override. On the 156 pinned Fluent Bit units measurable in that run, one unit
  moved Unknown → Safe (`chunkio/cio_error.c`) and no unit entered the Unsafe
  class; the 29-unit Monkey/forced-Libevent leg was not re-measured. The bounded extension and remaining restrictions are recorded in
  `docs/reports/w4-custody-extensions/README.md`. Gateway relocation is executed
  (2026-09-07, `docs/reports/w4-gateway-crate/README.md`), with exact
  185-unit verdict/projection/diagnostic parity. Execution
  is tracked in `docs/ROADMAP_AFTER_CYCLE_9.md`'s workstream table.

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
  event/scope (`Q700`–`Q703` are core scope diagnostics; Libevent uses
  `qcore.libevent.L00`–`L12`, see `docs/CHANGELOG_r0_2.md`), `QP***` plugin protocol.

## Where to read more

Direction and tasks: `docs/GENERALIZATION_PLAN.md`. Architecture boundary and
open gates: `docs/cycles/CYCLE_20.md`. Trust/verdict rules and audit history:
`docs/REEVALUATION_SPEC.md`, `docs/REEVALUATION_FOLLOWUP.md`. Language:
`docs/QCORE_SPEC_r0_2.md`, `docs/QCORE_MODULE_MODES.md`,
`docs/QTT_ALIGNMENT.md`. Contract corpus conventions:
`contract/qcore/README.md`.

## Reopen failure evidence (2026-09-07)

Five native reopen size-lookup/mapping failure sequences release all 50
allocation generations and six mapping epochs. Ten descriptor epochs yield eight
library closes and two residuals after parent release, reclaimed separately by
the harness. Failed size lookup retains an unmapped descriptor; failed mapping
stores MAP_FAILED, reports up-ring membership without incrementing the counter,
and retains the new descriptor. Fixed retries and already-synced behavior remain
observations, not general OS failure or durability guarantees. Evidence:
`docs/reports/cycle19-reopen-failures/README.md`. Raw units remain unpromoted.


W2 bounded profile migration landed (2026-09-07;
`docs/reports/w2-profile-manifests/README.md`): schema v1 and total-or-reject
loading replace the library enum, source inventories, mechanism/operation
summary tables and Libevent API/build/source tables with declared profile data.
Full 185-TU diagnostic/source parity is byte-identical to baseline `43040e7`
(3 Safe / 98 provisional Unsafe / 80 Unknown / 4 data-only; the third Safe was
already W4's measured change). Raw production units stay unpromoted. Monkey
typed imports, Boolean field routing, first-party registration literals and
configured orchestration remain W2/W5 work; this does not complete W2.

Frontend gates repaired (2026-09-07,
`docs/reports/w6-frontend-gates/README.md`): nonempty source function contracts
reject centrally with Q021 until enforced. Guard/thread-scope header names bind
only inside their bodies; nested concurrency lowering still rejects with Q500.
This closes the two W6 batch-2 frontend defects without claiming contract or
concurrency semantics are implemented. CT mapping remains 5/50.
