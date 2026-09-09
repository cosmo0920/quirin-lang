# QCore-C agent briefing

> **Current work status:** The 49-card remaining-work plan of 2026-09-07 is
> fully dispositioned. Current work follows the named follow-ons recorded in
> `docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`) and per-report
> follow-on lists. The overall generalization goal is not yet complete.

Consolidated, compressed project state for LLM agents. Humans should read
[README.md](README.md) for the summary and [DEVELOPMENT.md](DEVELOPMENT.md)
for implementation and status detail. Keep this file, `CLAUDE.md`,
`README.md`, and `DEVELOPMENT.md` in sync when project status changes; this
file is the source of truth for agents. New per-increment status paragraphs
belong in `DEVELOPMENT.md`'s "Increment records", not in `README.md`.

## What this project is

QCore-C is a research language for quantitative types, owned resources, typed
recoverable errors, checked cleanup, and checked concurrency, lowering to C11.
It also ships a conservative checker for *existing* C source (the
"unrestricted-C gateway"). Normative specification: `docs/QCORE_SPEC_r0_3.md` and
`docs/QCORE_CONTRACT_SPEC_r0_3.md`. Current consumer grammars are in
`docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md`; implementation acceptance is tracked
in `docs/reviews/r0_3/acceptance.json`. Revision 0.2 remains historical; broad
parser/model support is not full source-language conformance. Work proceeds in numbered bounded
"cycles" recorded under `docs/cycles/`.

## Repository map

| Path | Contents |
|---|---|
| `compiler/` | Rust library crate `qcore-compiler` (lib `qcore_compiler`): parser, checker, reference semantics, C backend, gateway/host compatibility APIs, first-party providers. Still the L1/L2 grab-bag; decomposition target in `docs/GENERALIZATION_PLAN.md` §3 |
| `crates/qcore-core/` | L1 kernel: quantitative types, ownership/error/cleanup/CFG semantics, concurrency/translation models, chunk lifecycle, generic library/container/reference-count and reactor/descriptor custody, checked task IR/reference execution and C11 task emission. No runtime or workspace dependencies; W5 consolidation remains bounded |
| `crates/qcore-frontend/` | L1 front end (W1 relocation): `lexer`, `parser`, `check`, `quant_index`, `frontend` elaboration and exact scoped copy/owned-buffer source lowering. Depends on `qcore-core` only |
| `crates/qcore-contract/` | L1 contract machinery: total-or-reject loader, observation and declared chunk-path policy evaluation. Depends on core/frontend/schemas; Boolean source imports are opaque in the C gateway; compiler compatibility adapters remain |
| `crates/qcore-schemas/` | L0 published schema surface (Rust API 3.0.0; W0 executed 2026-09-06): plugin protocol v1 types, resource-contract data model, mechanism vocabulary, source-identity records. Serde types only; a test forbids workspace dependencies |
| `crates/qcore-c-gateway/` | L2 existing-C analyzer, custody summary inference, Clang discovery, typed-AST transport and opaque Boolean-field source imports; depends only on contract/schemas plus serialization |
| `crates/qcorec/` | CLI binary `qcorec` plus its process-level tests |
| `crates/qcore-c-gateway/src/legacy_c.rs` | Generic unrestricted-C checker (Clang gateway, CFG, resource abstract interpretation, certificate recheck); contracts enter via `ResourceContracts` |
| `crates/qcore-c-gateway/src/legacy_summary.rs` | W4 interprocedural custody summary inference for internal C functions: SCC-ordered derivation, validation by re-analysis, declared-contract precedence. Relocated with Clang and typed-AST transport (2026-09-07); compiler compatibility reexports retained |
| `crates/qcore-c-gateway/src/typed_source.rs` | Opaque typed C path import and source def/use; generic tests run here, library fixtures remain in compiler compatibility adapters |
| `crates/qcore-verification/src/{counter_contract,field_destructor,list_unlink_contract,cleanup_outcome}.rs` | Trusted provider-free replay over opaque C imports and explicit contract premises |
| `crates/qcore-verifier-host/` | L2 generic registry, source-scope resolution, bounded process transport, v1/v2 firewall and composition; depends only on frontend/schemas workspace crates |
| `crates/qcore-verification/` | Trusted adapter: explicit-registry source driver, generic model/source imports, core-derived obligations, lifecycle/observation witness replay and scope dispatch; depends on core/frontend/contract/schemas/gateway/host, never providers |
| `compiler/src/verifier_plugin.rs` | Compatibility facade for the host and first-party adapters; trusted replay/derivation/dispatch live in `qcore-verification` |
| `compiler/src/verifier_plugins/{libevent,chunkio}/` | In-process first-party providers (Libevent/Monkey, ChunkIO) |
| `compiler/src/external_{integration,source}.rs` | Compatibility facades and Monkey-specific composition; pure custody is in core and generic model/source imports are in verification |
| `compiler/examples/cycle19_fluent_bit_libraries_audit.rs` | Production audit orchestration for the pinned Fluent Bit corpus |
| `crates/qcore-audit/` | L3 configured audit driver: compile-database ingestion, compile-task identity, source selection, per-unit projection and reports over W2 profiles/W1 contracts. Depends on gateway/contract/schemas/verification/host; links no provider module |
| `providers/` | Separate six-member Cargo workspace (five `qcore-provider-*` packages plus the `conformance` harness), excluded from the core workspace. Artifact crates depend only on `qcore-schemas` and serialization; `providers/package.py` builds, verifies and archives the installable packages |
| `contract/qcore/` | Declarative `.qcore` contract corpus (60 files); counts pinned by `compiler/tests/qcore_contract_corpus.rs` |
| `proof/` | Lean 4 executable core and safety theorems; `AuditCycleN.lean` print axiom surfaces |
| `docs/cycles/` | Per-cycle requirement/acceptance records (`CYCLE_2` … `CYCLE_20`) |
| `docs/reports/` | Archived evidence packages (pinned audits, observations, triage) |
| `DEVELOPMENT.md` | Human-facing implementation and status detail: repository layout, cycle records, per-increment evidence narratives, full build/validation instructions |
| `docs/reevaluation/` | Raw audit logs, probe JSON, `ct_manifest.json` (CT01–CT50 tracking) |
| `third_party/libevent` | Submodule pinned to the audited revision; a test fails if absent or moved |

## Status ledger (do not overstate)

Cycle completion labels describe their bounded milestone evidence, not full
revision-0.3 source-language conformance. `crates/` holds nine crates; the core
Cargo workspace declares **ten** members (those nine plus `compiler`), and
`providers/` is a separate six-member workspace excluded from it.

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
  42 provisional Unsafe / 137 Unknown**, plus four data-only units
  (`docs/reports/w4-nullable-removal/README.md`). **Raw units remain unpromoted** until
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
  unpromoted. Host registry/transport/firewall/composition relocation is executed
  (`docs/reports/w3-verifier-host/README.md`); production profile/source import
  and remaining trusted-kernel relocation stay open.
  See `docs/reports/w3-evidence-v2/README.md`.
- **Reevaluation (2026-09-05): 6 + 7 findings, all repaired**
  (`docs/REEVALUATION_SPEC.md`, `docs/REEVALUATION_FOLLOWUP.md`). Open
  boundaries that remain true statements: interprocedural parameter/return
  custody inference is **implemented for parameter/return custody with
  validation** (LC01, W4, extended 2026-09-07 with validated `Out`,
  `Stored`, `Reallocated`, possibly-null returns and direct `Removed` wrappers; general removal aliases and recursive
  call-graph components, and cost summaries are still not inferred, and every such body
  keeps its callers `Unknown`); semantic witness schema v2 is implemented for bounded
  replay (production lifecycle import remains unimplemented); process providers have process-group containment, not an
  OS sandbox; mutex model supports initially-unlocked non-recursive mutexes
  only; CT01–CT50 conformance corpus is **39/50 unmapped**
  (CT01/CT12 execute Linux owned-buffer transfer/creation restoration; CT02
  rejects actual post-move access. CT16 executes worker-local allocation/error cleanup; CT14/CT37 execute completed-but-unjoined native custody;
  CT19 checks real sibling failure source, CT13 has Linux creation-recovery
  evidence, CT15 has Linux native join-failure evidence, and CT20 checks
  sibling join/owned-result discharge before bound-error rethrow. CT50 checks real
  mapping address reuse with distinct Buffer generations and stale-binding rejection).
- **Active direction:** `docs/GENERALIZATION_PLAN.md` — generalize the
  checker away from pinned source sets and decompose the monolithic crate
  into a pluggable contract/verification system (workstreams W0–W7). W0 is
  executed (2026-09-06, `docs/reports/w0-schema-crate/README.md`): the initial
  workspace had three members (`qcore-schemas` L0 surface, `qcore-compiler`
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
  interpreter selector — 19 declarations load and 142 are rejected, not
  skipped — the clause-family → owner table is documented in the loader
  header and `contract/qcore/README.md`, and the C ABI scalar type table is
  complete (borrowed-only, no semantic change; audit diagnostics/callbacks
  byte-identical). Unifying the resource loader with the observation/
  lifecycle interpreters is W5-adjacent; non-null error sentinels need
  target-interface work, not loader vocabulary. W6's CT harness is stood up
  (2026-09-06, `docs/reports/w6-ct-harness/README.md`):
  `compiler/tests/ct_corpus.rs` executes every case
  `docs/reevaluation/ct_manifest.json` marks mapped through the real source
  pipeline and fails closed on manifest/fixture disagreement. CT02/CT12 now
  check real Buffer moves/restoration; CT16 executes worker-local allocation
  and error cleanup while retaining its earlier sequential implicit-cleanup
  control. CT14/CT37 execute completed-child native custody; CT37 retains its earlier sequential control. CT19 now checks actual sibling-failure source; CT15 executes
  Linux native join-failure and successful reclamation controls. CT13 checks
  explicit creation recovery and retained sibling storage. Shares and
  mutex source paths remain unexercised. Batch 2 (2026-09-06,
  `docs/reports/w6-ct-batch-2/README.md`) adds CT19/CT37 and records the
  full triage of all 47 then-unmapped cases (2 decidable-now, 8 blocked on
  W3/W4, 33 blocked on unimplemented syntax, 4 ambiguous); subsequent owned-task
  and reused-address mappings leave 39 cases unmapped, counted by a pinned test. W4 is executed for
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

## Lean CI hygiene (2026-09-07)

The CI workflow builds `proof/lean-toolchain` with Lean and runs every
`Audit*.lean` via `scripts/check_lean_audits.py`. `AuditAll.lean` checks the
compiled QCore axiom closure, allowing only `propext`, `Classical.choice` and
`Quot.sound`; stale audit names and warnings fail the gate. Both reevaluation
examples are included in Linux and Clang-enabled Windows CI. Local execution
is verified; hosted CI awaits a push. See `docs/reports/w7-lean-ci/README.md`.

Generic Lean names are also available with historical aliases:
`QCore.DescriptorCustody`, `EventComposition` / `checkEventComposition`, and
`busy_effect_preserves_lock`. The old Monkey module, composition accessors and
ChunkIO lock theorem remain usable. See
`docs/reports/w7-generic-lean-names/README.md` for name-only parity and compatibility audits.

Lean build performance (2026-09-08): TaskTraceRelation's finite-domain proofs
expand D3's generated inputs once before kernel evaluation; theorem statements,
domains, digests and allowed axioms are unchanged. The affected-module rebuild
passes in 47.5 seconds locally; the next incremental build takes 0.13 seconds.
`python3 scripts/check_lean_audits.py` now builds from the pinned proof directory,
reports progress, enforces finite time limits and cleans up workers on timeout
or cancellation. All 31 audits, eight runner tests and the standing gates pass.
Evidence: `docs/reports/lean-build-performance/README.md`.

## Standing per-change gate

Run before claiming any change complete:

```sh
git submodule update --init --recursive   # once per checkout
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo run -p qcore-compiler --example reevaluation_probe      # expect exit 0
cargo run -p qcore-compiler --example reevaluation_followup   # expect exit 0
python3 scripts/check_lean_audits.py
```

Clang is required for existing-C analysis tests (`QCORE_CLANG` overrides
discovery; invalid/empty value is an error, never a silent fallback). `qcorec
check`/`emit` and generated-C tests work without Clang (MSVC on Windows).
Lean toolchain via `proof/lean-toolchain` (elan).

## Key entry points

- Native threading examples: `examples/threads/README.md` explains `cl.exe`
  compilation; `build-windows.ps1 -Run` emits, builds and runs copied-worker,
  owned-buffer and shared-reader examples into `target/thread-examples/`.
- Editor: `editors/emacs/qcore-mode.el` provides lexical source/contract
  highlighting for `.qcore`/`.qc`; see `editors/emacs/README.md` for setup and
  ERT checks. Highlighting does not imply executable or verified support.
- Library: `parse_source_program`, `elaborate_source`, `check_whole_program`,
  `compile_files` / `compile_files_with_registry`,
  `analyze_unrestricted_c11` / `validate_unrestricted_c11`.
- CLI: `qcorec check <file>`, `qcorec emit -o out.c <file>`,
  `qcorec plugins list|inspect <id>`.
- Diagnostics namespaces: `Q1xx–Q4xx` core, `Q6xx` concurrency, `Q7xx`
  event/scope (`Q700`–`Q703` are core scope diagnostics; Libevent uses
  `qcore.libevent.L00`–`L12`, see `docs/CHANGELOG_r0_2.md`), `QP***` plugin protocol.
  The canonical per-code ownership table is `docs/QCORE_SPEC_r0_3.md` §L21:
  a **semantic pass** owns a range, `qcore_frontend::frontend::source_diagnostic`
  is the authoritative pass → code map, and any crate running that pass emits
  from its range. Codes are stable; a code change is semantic. Fixing where a
  diagnostic is *attributed* (file, line, span) is not a code change.
  `Q020` (task route) and `Q021` (sequential route) both refuse an unenforced
  source `contract { … }` and stay separate on purpose — their rejection
  coverage differs — until a source-contract consumer lands.

## Where to read more

Direction and tasks: `docs/GENERALIZATION_PLAN.md`. Architecture boundary and
open gates: `docs/cycles/CYCLE_20.md`. Trust/verdict rules and audit history:
`docs/REEVALUATION_SPEC.md`, `docs/REEVALUATION_FOLLOWUP.md`. Language:
`docs/QCORE_SPEC_r0_3.md`, `docs/QCORE_CONTRACT_SPEC_r0_3.md`,
`docs/QCORE_MODULE_MODES.md`,
`docs/QTT_ALIGNMENT.md`. Contract corpus conventions:
`contract/qcore/README.md`. Implementation and status detail for humans:
`DEVELOPMENT.md`.

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
typed imports, Boolean field routing and first-party registration are migrated
in subsequent bounded increments below; configured orchestration remains W2/W5
work. This does not complete W2.

Frontend gates repaired (2026-09-07,
`docs/reports/w6-frontend-gates/README.md`): nonempty source function contracts
reject centrally with Q021 until enforced. Guard/thread-scope header names bind
only inside their bodies; at that checkpoint nested concurrency lowering rejected with Q500.
This closes the two W6 batch-2 frontend defects without claiming contract or
concurrency semantics were implemented by that increment. CT mapping was then 5/50;
the native source-task increment below advances it.

W2 H6 provider metadata discovery is implemented (2026-09-07;
`docs/reports/w2-provider-discovery/README.md`). First-party manifest literals
are removed after field-for-field parity; registry metadata is loaded atomically
from profile packages with stable plugin-id ordering and no compiled fallback.
All shipped providers remain v1 built-ins returning Unknown; process extraction
is still W5 work. Full 185-TU diagnostic/source parity is unchanged.

Boolean field profile routing landed (2026-09-07,
`docs/reports/w2-boolean-field-profiles/README.md`): typed C record/field and
Boolean operation routes come from loaded profile data, with exact declared
assumptions on summaries. An unrelated record fixture exercises both supported
models. Full 185-TU resource diagnostics and source projections are unchanged;
chunk reports add only three assumption maps. Frame-helper ABI generalization
and source-to-model identity premises remain open.

W5's bounded configured-audit extraction is implemented (2026-09-07;
`docs/reports/w5-configured-audit/README.md`). New L3 `qcore-audit` owns
compile-db ingestion, source selection, per-unit projection and reports over
W2 profiles/W1 contracts, depending only on gateway/contract/schemas. A separate
synthetic repository runs from data alone (1 Safe / 1 provisional Unsafe /
1 Unknown). Full 185-TU compatibility output is byte-identical; raw production
units stay unpromoted. The old example is a small wrapper; specialized native
and source evidence remains in `compiler/examples/support/cycle19_compatibility.rs`
until provider extraction/parity replaces it. This is not full W5 completion.

Monkey typed import profiles landed (2026-09-07,
`docs/reports/w2-monkey-typed-profile/README.md`): function roles, records,
sizeof bounds, exact callback signatures, callee bounds and callback effects
load from profile data. Malformed or unavailable declarations reject; wrappers
retain their existing opaque-call grammar. Full 185-TU diagnostics are unchanged;
18 syntax summaries now retain explicit declared profile assumptions. These are
syntax projections and trusted declarations, not proofs of source custody or
whole-program safety. General source/model binding and provider consolidation
remain open; configured auditing is recorded separately above.

Revision 0.3 implementation review (2026-09-07): resource/task lifetime checks,
unconditional axis validation, signed literal extrema and exact/maximum loop
custody are repaired at their stated model/source boundaries. Ordinary i32/unit
result compatibility is checked with an explicit separate custody payload;
full owned-result typing remains open. Concurrency certificates retain declared
mutex/synchronization premises; checked invariant/source authority remains open.
Observation replay rejects scope/module/bounds substitution. Current requirements
and bounded acceptance are in `docs/QCORE_SPEC_r0_3.md` and
`docs/reviews/r0_3/acceptance.json`; superseded specs live in `docs/obsolete/`.

W5 kernel and contract boundaries advanced (2026-09-07): sequential and
concurrent model modules now reside in dependency-free `qcore-core`, with
byte-identical implementations/tests and compiler reexports. Chunk lifecycle
also moved unchanged. Portable, untrusted chunk-path records live in schemas
0.2.0; `qcore-contract` evaluates declared paths, while only the opaque typed
source importer can issue source-checked compatibility certificates. No plugin
wire version or admission rule changed. See
`docs/reports/w5-sequential-kernel/README.md`,
`docs/reports/w5-concurrent-kernel/README.md`, and
`docs/reports/w5-chunk-summary-boundary/README.md`.
W4 now validates direct non-null `Removed` wrappers from contained entry to
live detached exit, with independent certificate rederivation and matching
Lean relations (`docs/reports/w4-removed-custody/README.md`). All 185 pinned
resource/source results remain unchanged; recursion, general removal aliases,
provider model extraction and broader source/CT coverage remain open.

W5 standalone packaging and frontend consolidation (2026-09-07): five
schema-only executable providers reproduce built-in v1 Unknown responses and
explicitly return v2 Unknown without witnesses. Package verification and two
clean reproducible builds pass; default registry selection is unchanged.
`providers/README.md` documents build/installation/rollback. Specialized
provider models and registry-based Fluent Bit composition remain open.
Whole-program checking now lives in frontend; generic loop/ledger checking
lives in core, with preserved compiler APIs and exact implementation/test body
parity. Evidence: `docs/reports/w5-provider-packages/README.md`,
`docs/reports/w5-whole-program-frontend/README.md`, and
`docs/reports/w5-resource-accounting/README.md`.

CLI package lifecycle (2026-09-07, `docs/reports/w5-cli-packages/README.md`):
`qcorec plugins verify PACKAGE`, `enable PACKAGE --config FILE`, and
`disable ID --config FILE` support explicit pinned process selections.
`check`/`emit --plugins FILE` and list/inspect with `--config FILE` consume
that selection without default fallback. Verification reads manifest and
executable identity without executing it; this is a snapshot check, not
signature, runtime compatibility or immutable-execution proof.

Trusted verification adapter (2026-09-07): obligation derivation, witness replay
and scope dispatch now live in `crates/qcore-verification`, with byte-identical
implementations/tests and compiler reexports. It depends on core, frontend,
contract, schemas and host; providers cannot link it. Lifecycle source effects
remain incomplete and cannot promote a result. Evidence:
`docs/reports/w5-verification-adapter/README.md`.

W4 output forwarding and identity invalidation (2026-09-07): direct output
pointer forwarding now validates original sanctioned slots on every exit.
Formal shadowing, pointer mutation and direct alias chains cannot reuse stale
resource identity. Evidence: `docs/reports/w4-out-forwarding/README.md`.
That increment measured 3 Safe / 94 provisional Unsafe / 84 Unknown /
4 data-only, with no promotions. The subsequent indirect-access repair
(`docs/reports/w4-indirect-access/README.md`) rejects unmodeled dereference,
member and indexed access instead of erasing it from the resource projection.
The current 185-unit audit is 2 Safe / 42 provisional Unsafe / 137 Unknown /
4 data-only: 48 units and 455 functions downgrade, with no promotions.
These are withdrawn unsupported results, not proven upstream defects.
Typed access evidence, general pointer aliases and recursion remain open.

The generic reactor lifecycle and descriptor model now resides in
`crates/qcore-core/src/event_reactor.rs`, parameterized by an explicit accepted
profile list. The Libevent adapter retains build matching and pinned source
checks, and preserves concrete enum imports through a lossless facade.
Evidence: `docs/reports/w5-generic-reactor/README.md`. Callback source binding/protocol admission, specialized provider extraction
and registry-based corpus composition remain open.

Generic callback records (2026-09-07): `qcore-schemas::reactor` and
`qcore-verification::reactor_callback` support strict declared-model record
exchange against the complete assignment inventory. A schemas-only process
fixture exercises matching and adversarial proposals. This adds no protocol
witness admission or source-derived authority; Monkey effect records retain
their separate interpretation. Evidence:
`docs/reports/w5-reactor-callback-records/README.md`. Source binding,
specialized provider extraction and registry composition remain open.

Source-derived field-contract provider path (2026-09-07): the typed Boolean
importer now lives in `qcore-c-gateway`, with opaque source/build/profile/helper
identity snapshots and compiler compatibility APIs. `qcore-verification` derives
function-scoped evidence-v2 tasks and replays exact source paths plus pinned
contracts. The real ChunkIO 0.2.0 package proposes witnesses; the audit selects
it explicitly with `QCORE_CHUNKIO_PACKAGE`. Three real functions check across
four Boolean-state combinations each; 93 other selected functions remain
Unknown. Source, resource diagnostics and existing contract sidecars retain
byte parity. Raw unit totals remain 2 Safe / 42 provisional Unsafe / 137 Unknown /
4 data-only. Evidence: `docs/reports/w3-source-field-provider/README.md`.
Full lifecycle source inference, provider-owned source analysis and aggregate
registry composition remain open; this bounded property does not discharge them.

ChunkIO statement execution (2026-09-07): package 0.3.0 independently computes
Boolean-field paths from a version-2 typed statement program. Requests no longer
carry expected paths. The receiver retains opaque source identity, compares
complete proposed paths and evaluates the pinned contract. Parser/evaluator
semantics retain parity; source import and helper ABI checks stay trusted.
Evidence: `docs/reports/w5-provider-statement-execution/README.md`. Full lifecycle
import, other specialized provider analysis and registry composition remain open.

Contextual recursion (2026-09-07): W4 now infers validated custody for one
immutable explicit integer control, partitioned into zero/nonzero cases. Base
contexts have no SCC-call contracts; nonzero contexts call already validated
zero contexts via exact zero literals. Both contracts must agree, and the
combined CFG is revalidated and independently rederived by certificate checking.
Release, owned-return and bounded mutual-recursion source cases pass; mutations,
nondecreasing calls, zero cycles and broader controls remain outside this route.
Evidence: `docs/reports/w4-contextual-recursion/README.md`. General recursive
fixed points, broader arithmetic rankings, aliases and cost summaries remain open;
the unsigned countdown extension below covers exact `n - 1` calls.

Source task foundations (2026-09-07): structured spawn/join records now retain
ordered captures, resolved scope/worker/place symbols and initializer-before-binder
identity. The core scoped-task ledger checks reservation rollback, completion
capsules, creator-only join and scope exits; matching Lean transfer theorems and
adversarial scenarios pass. These components are not yet composed into source
execution at that checkpoint: task scopes still rejected and CT coverage was
5/50. The native source-task increment below now composes the copy-only path. Evidence:
`docs/reports/w6-source-task-foundations/README.md`.

Schema compatibility (2026-09-07): `qcore-schemas` is now 1.0.0 and all Rust
consumers require that major version. The version-2 typed program input replaced
a public summary field, requiring the documented breaking-change policy.
Protocol v1, evidence v2, program schema/profile v2 and ChunkIO package 0.3.0
retain their separate versions. Evidence:
`docs/reports/schema-1-compatibility/README.md`.

Native source tasks (2026-09-07): `compile_files` and normal `qcorec check`/`emit`
now execute the scoped-copy profile on Linux C11 threads. A typed finite task
IR checks all definitions, creation outcomes, declared worker outcomes, exact
scalar/handle/result identities and permitted effects. Native start records
survive until creator join; unexpected join failure aborts before reclamation.
`CompiledModule.checked` explicitly separates sequential and task evidence.
CT15 now maps actual native join failure with a successful join/free control;
CT19 uses a real second-spawn failure rejection and joined-sibling control.
Explicit Result handlers now recover from failed creation while preserving and
joining prior siblings; join handlers preserve exact worker errors. CT13 maps
real Linux success, first/second-create failure and join-before-free controls.
Coverage at that checkpoint was 7/50 mapped, 43 unmapped. Moves/loans, owned results, shared memory,
mutexes, nested workers, general control flow and other platforms remain open.
Evidence: `docs/reports/w6-task-recovery/README.md`.

macOS native concurrency (2026-09-08): scoped copy/owned/read-loan/read-share
tasks, typed owned-error payload tasks and checked mutex guards now check,
emit, compile and execute on macOS through an
emitted pthread-backed implementation of the C11 thread subset used by QCore.
The macOS CT harness executes the threaded native and fault-injection cases;
CT50's separate `MAP_FIXED_NOREPLACE` address-reuse observation remains
Linux-only. Apple ASan/UBSan runs remain enabled, while leak detection stays a
Linux-only observation because Apple's AddressSanitizer does not provide LSan.
Configured-audit compile inputs also compare canonical existing paths, so the
macOS `/var` to `/private/var` alias cannot spuriously invalidate a pinned task.
Archived Linux trace replay pins Linux errno values instead of reading the host
Python errno table.

Nullable removal and custody transfer validation (2026-09-07): W4 now validates
nullable `Removed` preconditions, including null-guarded nonnull removal and
reordered direct forwarding. Null refinement removes only the absent child's
containment edges; original owners and other children remain accounted for.
Confirmed false-Safe cases in merged removal, alias-selected actions, aggregate
cleanup and terminal custody reuse are repaired. Ambiguous effects retain
unresolved custody; independent leak evidence remains visible. General aliases,
heap-link discovery and relational branch analysis remain open. Evidence:
`docs/reports/w4-nullable-removal/README.md`.
The fresh 185-unit audit records 2 Safe /42 provisional Unsafe /137 Unknown
plus four data-only units. Five previous provisional Unsafe units now remain
Unknown because their residual leaks depended on unresolved effects. Source,
chunk-contract and source-effect reports are byte-identical; 22 units have
diagnostic changes. No raw unit is promoted.

Configured source-contract dispatch is implemented (2026-09-07;
`docs/reports/w5-configured-source-contracts/README.md`). The L3 audit service
loads explicit package/library/policy bindings, snapshots policy bytes and
selects providers from sealed Clang import identities. `QCORE_SOURCE_CONTRACTS`
selects the configuration; `QCORE_CHUNKIO_PACKAGE` remains a compatible package
path alias, and selecting both rejects. The selected process supplies typed
Boolean-field witnesses for independent trusted replay. A second C codebase
passes through the same service and a schemas-only process fixture under its
declared existing Boolean-axis profile. All four full 185-unit audit artifacts
are byte-identical to the preceding baseline and between both selectors:
2 Safe / 42 provisional Unsafe / 137 Unknown, plus four data-only units;
three checked function contracts and 93 Unknown entries remain unpromoted.
Hardcoded provider/policy selection and the duplicate pre-dispatch evaluation
are removed from this bounded process route. General lifecycle source import,
other specialized adapters and full W5 consolidation remain open.

Configured Boolean frame helpers are implemented (2026-09-07;
`docs/reports/w5-configured-frame-helpers/README.md`). Helper source/function/
record/policy selection now comes from configuration. The gateway validates
exact declared record/prototype/field vectors, source/build/profile identity
and constant-only metadata writes disjoint from every modeled field on the
record. A second codebase executes a separately imported helper through the
same schemas-only process and trusted replay. The reusable service rejects
unrepresented compiler options; the pinned adapter explicitly retains its
historical option projection. Actual nested C layout/linker equality remains
an outer-audit premise. Ten generic Lean frame-preservation theorems pass.
No raw Cycle19 unit is promoted: 2 Safe /42 provisional Unsafe /137 Unknown
plus four data-only units; three checked function contracts and 93 Unknown.
The helper policy inventory is explicit report metadata; general lifecycle
import and broader W5 consolidation remain open. Owned task execution advances
in the subsequent bounded W6 increment below.

W7 diagnostic attribution now requires the exact selected plugin ID plus one
1–128-byte ASCII local code (letters, digits, underscore or hyphen), in both
protocol response checks. Nested-provider spoofing, empty suffixes and control
characters reject with QP007 before verdict admission. Valid provider codes and
all existing evidence gates remain unchanged. This repairs attribution, not a
previously demonstrated Safe-admission bypass. Evidence:
`docs/reports/w7-diagnostic-attribution/README.md`.

Owned source tasks (2026-09-07): `ScopedOwnedTasksV1` adds malloc-backed
`own<Buffer>` captures and results to the checked finite task profile. Root
allocation carries AllocError; failed creation restores exact parent bindings,
while successful move consumes them permanently. Checked worker exits bind
returned identities to original inputs and account for every other input.
Native creator join returns the same buffer pointer before parent read/drop;
failed native join aborts before reclaiming environment or payload. CT01 now
maps the real round trip, CT02 rejects actual parent post-move access, and CT12
executes failed creation/restoration with a successful control. Coverage is
8/50 mapped, 42 unmapped at that checkpoint. Lean transfer/typing lemmas and fixed native sanitizer
runs are separate evidence; Rust/Lean/C refinement, loans, general owned graphs,
owned errors, shared memory, mutexes, nested workers and
other native platforms remain open. Evidence:
`docs/reports/w6-owned-source-tasks/README.md`.

Configured process audit (2026-09-07): `qcore-audit CONFIG.json` now connects
explicit source-contract package/policy selection to pinned source/compile-db
imports and independent process-witness replay. Selected helper sources are
checked against their Git blobs and imported bytes. The second-codebase command
checks two function contracts while retaining two Unknown functions, including
a build-excluded definition; its raw resource unit remains Unknown. Provider
failure or rejected evidence never falls back or promotes the resource verdict.
Report metadata preserves loaded policies/packages, source identities and exact
checked/Unknown inventories. The full Cycle19 compatibility reports remain
byte-identical. General lifecycle import, specialized provider extraction,
required native evidence and full registry composition remain open. Evidence:
`docs/reports/w5-configured-process-audit/README.md`.

Unsigned countdown custody (2026-09-07): W4 now validates exact `n - 1`
recursive calls for immutable, explicit unsigned controls, including mutual
calls with different control slots/unsigned widths. Zero bodies check with all
component contracts absent; nonzero bodies check at strictly smaller natural
ranks and must reproduce the complete base contract. Only the checked scalar
actual is abstracted for resource analysis; certificates retain original source
and independently repeat rank extraction and custody validation. Signed or
nondecreasing arithmetic, zero-context recursion and unsupported control uses
remain Unknown. Nine Lean theorems establish the rank/induction relation;
source extraction and scalar erasure are premises, and automatic stack capacity
is not proved. Six new public tests and a fixed six-allocation native sanitizer
fixture pass. All 760 workspace tests and the standing gates pass; the 185-unit
Cycle19 reports are byte-identical. General recursive inference, aliases and
cost summaries remain open. Evidence: `docs/reports/w4-unsigned-countdown/README.md`.

Generic source consolidation (2026-09-07): W5 moves typed C path import and
call-result def/use into `qcore-c-gateway`; counter, nullable-field destructor,
list unlink and cleanup-outcome replay now live in `qcore-verification`.
Both crates expose provider-free APIs while preserving opaque source summary
construction. Four unrelated-source integration tests exercise these APIs;
compile-fail checks protect the constructor/serialization boundary. Generic
symbol, short-circuit and call-use tests now run in the gateway; provider fixtures
remain with their compatibility adapters. All six production bodies have token
parity after module-reference/import-order rewrites; default policy bytes and
all four full Cycle19 artifacts are unchanged. The 767 workspace tests and
standing gates pass. Specialized reference matching, provider extraction,
registry composition and final compatibility removal remain open. Evidence:
`docs/reports/w5-generic-source-consolidation/README.md`.

Handler rethrow conformance (2026-09-07): CT20 now executes an in-scope
second-creation failure handler that joins its existing sibling, drops that
child's returned Buffer, then raises the bound payload-free SpawnError. Four
native/reference scenarios cover success, buffer allocation failure and either
creation failure. Exact allocation/thread identities, returned buffer generation
and join-before-reclamation are checked; omitting the join or returned-buffer
drop rejects at Q600. The generic CT runner now supports declarative task
scenario matrices with explicit failures/outcomes and negative source controls.
Coverage is 9/50 mapped, 41 unmapped. All 767 workspace tests, standing gates
and four fixed sanitizer runs pass. No language/checker/backend semantics change;
proof remains unassessed for this source fixture. Loans, owned errors, arbitrary
handlers and native refinement remain open. Evidence:
`docs/reports/w6-handler-rethrow/README.md`.

Worker-local Buffer allocation (2026-09-07): nonrecursive task-free leaf workers
now allocate, release and return fresh Buffers. Checked worker exits distinguish
returned captures from fresh locals and require every other capture/allocation
to be dropped, including allocation-error exits. Native and reference identities
use a task namespace plus a private successful-allocation ordinal; reversed
worker allocation order preserves both identities without a shared counter.
CT16 now executes a real worker's typed-error cleanup and either allocation
failure; its earlier sequential implicit-cleanup control remains tested.
Six fixed ASan/UBSan/LSan runs cover these four CT16 paths, a fresh returned
buffer and forced reverse allocation order. Fourteen Lean model theorems cover
local-resource accounting and bounded identity separation; source/native
refinement remains a premise. All 775 workspace tests, 31 provider tests,
43 Python tests, standing gates and 20 Lean audits pass. CT coverage remains
9/50 mapped, 41 unmapped; all four full Cycle19 artifacts are byte-identical.
Loans, owned error payloads, general owned graphs, nested workers, shared memory
and other-platform execution remain open. Evidence:
`docs/reports/w6-worker-allocation/README.md`.

Typed body comparison boundary (2026-09-07): W5 moves the remaining generic
Clang body-normalization/comparison algorithm into `qcore-c-gateway`. Its public
opaque comparison accepts caller-supplied references and unverified revision
labels; it establishes only conservative typed-body equality. The compiler's
reviewed-library `SourceMatch` remains a sealed wrapper with no public constructor,
conversion or deserialization. Existing traversal/parent reference selection,
pin checks, policy premises and serialized report shape are unchanged. Five
unrelated-C gateway tests and four compile-fail checks exercise this boundary;
all 1,447 production comparison tokens match after documented API renames.
All 784 workspace tests, standing gates, 31 provider tests, 43 Python tests and
20 Lean audits pass; all four full Cycle19 artifacts are byte-identical. Provider
semantic extraction, source-bound lifecycle composition and compatibility-route
removal remain open. Evidence: `docs/reports/w5-typed-body-boundary/README.md`.

Completed-child custody conformance (2026-09-07): CT14 is newly mapped and CT37
now executes real Linux worker allocation/completion/join. A shared native adapter
forces the actual worker body and completion-record write to finish before
creation returns, then checks that the original Buffer and environment remain
live until join. CT14 rejects parent read/drop of its moved Buffer before join;
CT37 rejects discarded or undropped owned Results. Both accepted controls join
and release the exact returned generation. The earlier CT37 sequential payload
fixture remains tested. Coverage is now 10/50 mapped, 40 unmapped. All 785 workspace
tests, standing gates, 31 provider tests, 43 Python tests, 20 Lean audits and two
fixed ASan/UBSan/LSan runs pass; all four full Cycle19 artifacts remain identical.
No source/core/backend semantics change; existing completion-custody model
proofs remain separate from source/native refinement. Loans, shared authority,
general heap-peak accounting and arbitrary native schedules remain open.
Evidence: `docs/reports/w6-completed-child-custody/README.md`.

Immutable output aliases (2026-09-07): W4 now infers wrappers whose first local
declarations copy an output formal through an immutable alias chain of the same
explicit flat pointer type. Only exact modeled Out arguments and direct indirect
stores may use those aliases. The private normalizer retains original source in
certificates and preserves the original sanctioned slot for ordinary all-exit
custody validation. Mutation, shadowing, escapes, casts, branch-local aliases,
indirect reads and unresolved nullable handbacks remain Unknown. Six new gateway
tests cover chains, multiple outputs, null branches, adversarial uses, caller leaks,
declared precedence and independent certificate rederivation. Six Lean lemmas
cover immutable binding copies and unchanged output authority; source extraction
remains a premise. A native run executes the same C wrappers and releases all four
observed allocations under ASan/UBSan/LSan. All 791 workspace tests, standing gates,
31 provider tests, 43 Python tests and 20 Lean audits pass. Broader alias analysis,
relational branches, general recursion and cost summaries remain open. Evidence:
`docs/reports/w4-immutable-out-aliases/README.md`.

Reused-address conformance (2026-09-07): CT50 now executes two owned Buffer
lifetimes backed by real Linux mappings. The first is unmapped; the second uses
MAP_FIXED_NOREPLACE and must obtain the same numeric address without replacing
any intervening mapping. Native release hooks check distinct generations 1 and 2,
exact live pointers and both successful unmaps. Stale source read/drop after the
fresh allocation reject at Q600. This replaces the old numeric-ID-reuse rejection
with actual address reuse under distinct lifetime identities. Coverage is now
11/50 mapped, 39 unmapped. All 791 workspace tests, standing gates, 31 provider
tests, 43 Python tests, 20 Lean audits and the fixed sanitizer run pass; all four
Cycle19 artifacts remain byte-identical. No core/backend semantics change. Raw
pointer or descriptor reuse, arbitrary allocators and source/native refinement
remain open. Evidence: `docs/reports/w6-reused-address/README.md`.

Generic driver and custody consolidation (2026-09-07): W5 moves the pure
reference-count/container/publication custody model into dependency-free
`qcore-core`. Generic library/chunk import gates, source-bundle checks, the
Cycle17 aggregate checker and source compilation with an explicit verifier
registry now live in `qcore-verification`, without provider dependencies.
Compiler facades preserve public type identity and default first-party discovery;
Monkey-specific composition stays in its adapter. Seven production components
retain exact token parity after documented module/import rewrites. Two existing
pure tests move with the model; six new integration/compatibility tests bring the
workspace to 797 passing tests. Standing gates, 31 provider tests, 43 Python tests
and 20 Lean audits pass. The full 185-TU audit and all four serialized artifacts
remain identical to the prior report; no verdict or CT mapping changes. Declared
model profiles and synchronization evidence remain premises, not native proofs.
Specialized provider semantics, broader lifecycle/source binding and compatibility
route removal remain open. Evidence:
`docs/reports/w5-generic-driver-consolidation/README.md`.

Public source-profile validation (2026-09-07): W2's audited-file and construct
witness rules now run at direct source-summary derivation as well as JSON load.
A manually constructed profile can no longer grant digest-based mechanisms from
an unaudited file, silently skip a missing witness, use vacuous checks or collapse
duplicate inventory entries. One shared validator owns the unchanged load rules;
invalid direct inputs return `InvalidProfile`, with no partial certificate.
Six generic regression tests reproduce and repair the bypasses; nine Lean gate
theorems cover inventory binding and whole-profile rejection under explicit
hash/predicate premises. All 803 workspace tests, standing gates, 31 provider
tests, 43 Python tests and 20 Lean audits pass. The four full 185-TU audit
artifacts retain exact parity; declared source constructs remain assumptions,
with no raw verdict promotion or CT mapping change. Evidence:
`docs/reports/w2-source-profile-boundary/README.md`.

CMetrics unit-result provider (2026-09-07): W5 adds explicit void-result Boolean
field import and process replay. `cmt_counter_allow_reset` now has a checked
function contract in a separately configured CMetrics analysis build; six other
functions remain Unknown and its raw unit stays provisional Unsafe. Native
ASan/UBSan/LSan execution covers both initial field values and unchanged remaining
object bytes. Unit inputs use semantic profile v3, a distinct result ABI, two
prestates and generic field predicates; scalar-result assertions reject. Existing
i32 profile v2 behavior and wire bytes remain unchanged. The schema Rust API is
version 2.0.0; CMetrics 0.2.0 and ChunkIO 0.4.0 share a schemas-only producer.
The 185-unit audit retains exact source/resource diagnostic bytes and all verdicts;
two source-contract reports change only 75 Unknown explanations. The existing
three checked/93 Unknown source-effect split is unchanged. Five new workspace
tests, expanded provider tests and eight Lean unit-result theorems cover the
boundary. All 808 workspace tests, 45 provider tests, 43 Python tests and 20 Lean
audits pass, together with the standing gates and two clean package builds.
Evidence: `docs/reports/w5-cmetrics-unit-provider/README.md`. Broader provider semantics,
original CMake-build equivalence, lifecycle composition and raw promotion remain
open. The contract corpus now has 59 modules; CT coverage remains 11/50 mapped.

Ordered preprocessing transport (2026-09-07): W2's configured audit preserves
regular/system/quote include classes and ordered macro definitions/undefinitions.
A CLI regression reproduces the old adapter selecting a header that hid an
unresolved call and returned Safe; the corrected import returns Unknown. Source
parents now affect quoted includes only, and typed options bind source-task
identity, including configured helpers. Empty arguments and command structure
fail closed. All 816 workspace tests, 45 provider tests, 43 Python tests and
20 Lean audits pass, together with the standing gates. All four 185-unit audit
artifacts retain exact byte parity; the separate CMetrics audit changes only
configuration/task/coverage identities. Eight new workspace tests cover the
boundary. Evidence: `docs/reports/w2-preprocessor-transport/README.md`. The historical pinned
Cycle19 option projection remains bounded; unsupported CMake flags, compiler
equivalence, broader provider semantics and raw promotion remain open.

Persistent Clang CMake audit (2026-09-07): W2's canonical CMetrics audit now
consumes an unmodified CMake compile database, including warning/debug/PIC and
assembler flags. The gateway transports four explicit flags through preprocessing,
syntax and typed AST; they require a matching Clang C executable and explicit
dialect. The original GCC command rejects rather than inheriting Clang defaults.
An independent pinned source clone absorbs upstream configure-time writes, and
`prepare_cmetrics_clang_audit.py` persists the build, database and provenance.
The native CMetrics static library builds in the normal and fresh output trees;
both audits retain one checked function/six Unknown and a provisional Unsafe
raw unit. All 819 workspace tests, 45 provider tests, 43 Python tests and 20 Lean
audits pass, with all standing gates and exact four-artifact 185-unit parity.
Evidence: `docs/reports/w2-clang-cmake-build/README.md`. General compiler,
code-generation/stack equivalence, broader providers and raw promotion remain open.

R02 terminal task state, adversarial verification (2026-09-07): V02 re-examined
"a joined child can still emit events" and found no soundness failure; no
production code changed. Evidence was the gap. Four new tests pin the full
terminal-state x event-kind matrix in `crates/qcore-core`: each joined-child
event kind now asserts its exact `TerminalTask`/`UnknownJoin`/`DuplicateTask`
diagnostic and the absence of any authority diagnostic, proving the terminal
decision precedes every custody, guard and permission check. A refused join is
not an implicit completion: exact diagnostic vectors show the child keeps its
authority and duties and stays outstanding, matching CT15 and the native
abort-before-reclamation path. A completed-but-unjoined child's captures stay
in `Custodian::Capsule` and all ten ledger operations reject with
`InactiveTask` in both `Completed` and `Joined` phases, leaving the ledger
unchanged. Nineteen new Lean theorems give the `Lifetime` model per-constructor
terminal rejection, post-join corollaries and refused-join retention, with no
new axioms or placeholders. All 823 workspace tests, 45 provider tests, 43
Python tests, 20 Lean audits and the standing gates pass; CT coverage remains
11/50 and all Cycle19 artifacts are untouched. Evidence:
`docs/reports/remaining-v02/README.md`. Source-to-trace refinement, checked
mutex invariants, loans, general owned graphs and Rust/Lean/native
correspondence remain open.

Compile-task identity (2026-09-07): the configured audit now records a complete
identity for every accepted compile task — Clang program/version/target, the
declared driver, the compile working directory, the main source path, the
complete ordered argument vector, the dialect, the ordered transported
preprocessing options and analyzed scope, the raw source bytes, the effective
preprocessing input, and named contract/profile/database/rule dependency
digests. A used header, a generated configuration header, include order, an
unread admitted macro option or a contract/profile change now retires the
recorded identity even when the selected `.c` file, the compilation database and
the preprocessed token stream are unchanged. The evidence-v2 task digest already
bound the same inputs; a stale witness is now shown to be refused in both
directions. One configuration is one (working directory, complete command) pair:
repeated identical records are one task, conflicting records reject. A read-only
census records that the pinned Cycle 19 compatibility adapter still chooses
silently between two configurations for `src/flb_input_chunk.c` and among four
for each of 27 Monkey/libevent units; repairing that changes the pinned
artifacts and remains open. Ten new tests; all 829 workspace tests, 45 provider
tests, 43 Python tests, 20 Lean audits and the standing gates pass, and the
configured CMetrics audit still reports one checked function contract, six
Unknown functions and a provisional Unsafe raw unit with identical verdicts,
diagnostics and source digests. The four pinned 185-unit artifacts could not be
re-measured: the upstream Fluent Bit checkout has moved off its pin and the
audit's own pin gate refuses to run. Evidence:
`docs/reports/remaining-g01/README.md`. Header enumeration, compiler
equivalence, build hermeticity and the compatibility adapter's ambiguity remain
open; no verdict changes and no raw unit is promoted.

Moving Fluent Bit revisions (2026-09-07): configured resource audits now accept
`source_revision_policy: checkout_head`. They resolve the actual commit once,
verify selected source bytes, and bind that commit into compile-task identities.
Historical profile digests/witnesses and provider/native evidence are not reused;
source-specific evidence remains Unknown. Source-contract selections and missing
required evidence reject in this mode. Existing configurations and the Cycle19
compatibility command retain pinned reproduction behavior. CMetrics preparation
now defaults to local HEAD, accepts `--revision`, and isolates builds by resolved
commit. At `d1ab8abff`, separate CMetrics/librdkafka/ChunkIO scopes complete:
138 units, 0 Safe / 28 provisional Unsafe / 107 Unknown / 3 data-only. Full
current-corpus auditing still rejects unsupported CTraces options and conflicting
wrapper commands; these are not revision-selection failures. No whole-unit
promotion or semantic rule change is claimed. See `docs/reports/fluent-bit-moving-revisions/README.md` and
`crates/qcore-audit/README.md` for commands and validation.

Cycle19 command correction checkpoint (2026-09-07): the existing example now
uses the current checkout by default; `--pinned-evidence` explicitly selects
historical replay. Distinct compile configurations remain visible, selected
bytes bind to the actual commit, and historical sidecars become explicit Unknown
records. Monkey preparation also accepts the current clean revision. The exact
user command now completes at `d1ab8abff`, but all 186 configurations across
185 source files remain Unknown before analysis because of compiler-command
admission. This is an incomplete audit checkpoint, not a successful production
verification: faithful dialect/driver/flag and filename-macro ingestion remains
required. See `docs/reports/cycle19-current-command/README.md`.

Cycle19 explicit Clang audit (2026-09-07):
`python3 -B scripts/prepare_cycle19_clang_audit.py --run` prepares Fluent Bit and
forced-Libevent Monkey with explicit Clang compilers and dialect flags, including
repeat configuration. The example accepts `--compile-database PATH`; a GCC
database rejects once with preparation instructions, without compiler substitution.
`-Wextra` reaches preprocessing/syntax/AST; quoted C macro dollar bytes are
preserved literally without evaluating Make/shell expressions. The verified
Clang rerun at `9f0fa573` analyzes all 186 configurations across 185 source files:
2 Safe / 43 provisional Unsafe / 137 Unknown / 4 data-only, zero gateway failures
and zero compiler-command refusals. Historical source/native evidence remains
unpromoted. All 857 workspace tests and standing gates pass. Full native Fluent
Bit build, GCC equivalence and general filename/build semantics remain outside
this result. Evidence: `docs/reports/cycle19-explicit-clang/README.md`.

Windows Clang discovery and test portability (2026-09-07): discovery orders
vswhere results newest first and falls back to standard VS 2026 (`18`) and
VS 2022 (`2022`) edition directories, including missing/empty locator cases.
The Windows tests use the discovered executable in compile databases, preserve
CRLF mutation checks, and run POSIX LP64 cleanup fixtures in isolated child
tests through that Clang with an explicit Linux target. Native Windows analysis
retains its native target; no checker rule or production cleanup ABI is widened.
See `docs/reports/windows-vs2026/README.md` for local validation and limits.
Windows validation: 828 tests, formatting, Clippy and both reevaluation probes
pass. The Lean gate was not run because `lake` is not installed on this host.

Enforced process limits (2026-09-07): the verifier host gains a target-qualified
`linux-rlimit-v1` execution profile that installs a package's declared limits
with `setrlimit` before `exec` — `RLIMIT_AS`, `RLIMIT_CPU`, `RLIMIT_CORE`,
`RLIMIT_FSIZE`, `RLIMIT_NOFILE`, `RLIMIT_NPROC` — on top of the existing wall
deadline, output caps and process-group kill. A declared limit kind the profile
cannot enforce, and every non-Linux target, are refused before spawning; there
is no downgrade path. The default stays the explicitly labeled
`unenforced-legacy` profile, which now names the kinds it did not enforce.
Executions record the achieved limits, their source and the outcome; every
non-completed outcome remains a scoped `Unknown` under QP006 with no
certificate, never a program `Unsafe`. Adversarial fixtures bound memory growth
(exit 2 at 4.7 ms under an 8 MiB `RLIMIT_AS`, while the same fixture completes
under the legacy profile), output flooding, a wall hang (300 ms), a CPU spin
(SIGKILL at 1.004 s against a 60 s wall clock), a surviving descendant and a
`max_processes` fork rejection, plus a pre-spawn refusal in 51 us. All five
shipped packages answer identically under both profiles across their 11
capabilities. `PluginLimits` gains an optional declared-kind map, making
`qcore-schemas` 3.0.0 under its documented policy; manifests declaring no kind
serialize byte-identically. All 837 workspace tests, 47 provider tests, 43 Python
tests, 20 Lean audits and the standing gates pass. This is limit enforcement,
not an OS sandbox: filesystem/network confinement, per-tree process quotas and
default profile wiring remain open (card P03). Evidence:
`docs/reports/remaining-p02/README.md`.

Diagnostic attribution and ownership (2026-09-07): W7 repairs the two real
attribution defects recorded by V04 and V07. `locate` tested an inclusive bound
against an exclusive `FileRange::end`, so a diagnostic at byte 0 of a non-first
input file was blamed on the previous file at a line past its end; all three of
V07's confirmed occurrences now name the offending file, with its controls
unchanged. The unconditional dimension/axis check no longer reports
`Span { 0, 0 }`: slice provenance carries the offending declaration's own span,
and a parse failure in the reconstructed buffer now reports the caller's byte
offsets (V04-F2, fixed rather than deferred). `StaticEnvironment::check_located`
is additive; `check` delegates and the accepted/rejected set is unchanged.
No diagnostic code changed — attribution is not a code change. `Q700`–`Q703` are
reconciled in the spec against the code, which is the authority; `Q020` (task)
and `Q021` (sequential) stay separate by decision because their rejection
coverage differs. `docs/QCORE_SPEC_r0_3.md` §L21 is the canonical per-code
ownership table and §L21.1 specifies a closed reason-code vocabulary for
uncovered obligations, with `environment-unavailable` separated so no
environment failure can be relabeled a source defect. That vocabulary is
specified only: carrying it on `DerivationIncomplete` would change every task
digest, and carrying it on audit-report units would change all four pinned
185-unit artifacts. Three new tests; 890 workspace tests, standing gates and
20 Lean audits pass. The 185-unit audit was not re-run — no prepared Clang build
tree here — and parity is argued by verified disjointness from the C gateway.
Evidence: `docs/reports/remaining-d01/README.md`.

Configured second source set (2026-09-07): the new `chunkio-core` profile
audits 13 pinned ChunkIO translation units through the existing `qcore-audit`
route from `profile.json` and `audit.json` alone, over CMake's unmodified
Clang compile database and the existing resource-contract manifest. No
production Rust changed: the whole `crates/` diff is one new test and one new
test fixture, and `compiler/` is untouched. The run reports 0 Safe /
2 provisional Unsafe / 11 Unknown across 13 units (1 Safe / 2 Unsafe /
120 Unknown functions, 3 unprojected); no raw unit is promoted. Changed pins,
changed audited digests, unsupported compiler flags and absent provider
packages each reject with no report, against both the real inputs and hermetic
fixtures, while unmodeled source forms stay Unknown with keyed diagnostics.
Report and assumption keys match the CMetrics and synthetic-demo routes
exactly, except the `source_contracts` block only a configured provider route
emits; both canonical CMetrics artifacts are byte-identical. Two generalization
gaps are recorded and deliberately not repaired: a source set whose C sources
sit at the checkout root cannot be expressed (`source_roots[].root` rejects
`.`), and a second real CMake project's unmodified database rejects because
only four frontend flags are transported. All 824 workspace tests, 45 provider
tests, 43 Python tests, 20 Lean audits and the standing gates pass. Evidence:
`docs/reports/remaining-g03/README.md`.
Replay mutation matrix (2026-09-07): the ChunkIO Boolean-field capability now
has a systematic 52-row mutation matrix over source-bound witness replay and
cached summaries. All 52 rows reject; no mutation was admitted, so no gate was
changed and no repair was required. Newly pinned: analyzed function scope and a
widened imported function set retire the task and refuse its witness; task,
obligation, request and source-projection version fields are each bound, and an
unsupported evidence version rejects in both directions; mismatched obligation
identifiers reject with the right count, as a foreign vector, as a permutation
and as another function's real identifier, with the core checker independently
refusing a task it did not derive; one witness never covers two obligations,
whether its coverage set claims both or the same certificate is submitted twice
at composition; a forged record rebound to a real donor task digest is caught by
the outermost binding; a well-formed path set that belongs to another function
and satisfies the same contract is refused while the obligation stays Unknown,
as is a proposal that echoes assertion text, the Lean theorem family and the
verdict label instead of computed paths; oversized and degenerate proposals fail
bounded without panicking. Legacy protocol v1 is preserved as non-certifying:
Unknown flows verbatim with no injected rejection, a complete v1 Safe envelope
still refuses terminally and an Unsafe allegation still downgrades. No summary
or evidence cache exists on this route; that is recorded as the current safe
state and pinned behaviorally. Boundaries recorded, not repaired: witness
admission conveys no provider authority, because acceptance terminates in the
core re-evaluating its own sealed import; and this capability's checker returns
its whole task coverage set, sound only because it derives exactly one
obligation and refuses every other task, now guarded by a test. Eleven new
tests; all 861 workspace tests, 46 provider tests, 43 Python tests, 20 Lean
audits and the standing gates pass, and the configured CMetrics audit still
reports one checked function contract, six Unknown functions and a provisional
Unsafe raw unit. No production source file changed, so the pinned 185-unit
artifacts, contract corpus counts and CT coverage are untouched. Evidence:
`docs/reports/remaining-e02/README.md`. Other capabilities' matrices,
out-of-process transport bounds and E01-B1's enumeration gap remain open; no
verdict changes and no raw unit is promoted.
Source-filename-sensitive transport (2026-09-07): the configured audit now hands
Clang the compilation database's own input word from its own directory, so
`__FILE__`, `__FILE_NAME__`, `__BASE_FILE__` and the includer-directory rule for
quoted includes answer as the recorded compile command does; the synthesized
source-parent quote path is removed. Declared source bytes must equal the file's.
`CSourceInput` makes the input identity explicit: an anonymous text rejects
file-name macros that would expand to `<stdin>`, and `__builtin_FILE`/
`__builtin_FILE_NAME` reject in every declared mode because the syntax and typed
AST phases always read preprocessed standard input. Detection scans preprocessed
output, so header-borne constructs are seen too. A CLI regression reproduces a
`Safe` verdict decided by the audit's working directory, where a shadowing header
replaced the real one; it now reports Unknown from any directory. Path spelling is
preserved, not normalized, and no equivalence with a direct one-shot compilation
is claimed. The pinned Cycle19 compatibility route explicitly retains its archived
standard-input projection and both exemptions; its 185-unit artifacts were not
re-measured. Recorded identities bind the declared input under
`qcore.declared-input.v1:`; standard-input units keep historical identity bytes.
Merged with G01's compile-task identity: `CompileTaskIdentity` now also binds
`source_input`, so the transport is not invisible to the recorded identity, and
G01's synthesized-quote-include pin is replaced by an explicit `-iquote` plus
`source_input` assertions. A pre-existing `PluginLimits::additional` build break
on the integration branch is repaired. All 894 workspace tests, 47 provider
tests, 43 Python tests and 20 Lean audits pass with the standing gates. The CMetrics audit is unchanged: one checked
function, six Unknown, a provisional Unsafe raw unit, byte-identical diagnostics,
with ten identity-digest fields moved because 12 `__FILE__` expansions became the
real source path. Evidence: `docs/reports/remaining-g02/README.md`. Compatibility
route migration, single-invocation frontend transport and raw promotion remain open.
Checked mutex invariants (2026-09-07): T02 implements `CheckedMutexInvariantV1`
as a new `crates/qcore-core/src/mutex_invariant.rs`, beside an unedited
`check_concurrent` whose body, 26 pinned tests and `Lifetime` Lean model are
byte-identical; `TrustedExplicitTrace` retains `GuardFootprintAuthority` and
`SynchronizedCustodyEdges`. Mutex generations are liveness-ledgered under V01's
rules: a refused destroy leaves the generation live and a repeat destroy reports
a dead generation, not an ownership complaint. `CreateMutex` fixes its declared
footprints and consumes the creator's `FULL_ACCESS` share over each, so a
protected footprint has exactly one protector and no fallback authority path;
pointwise share conservation over exact interval atoms is verified before the
first event and after every event, refused events included. Each `Lock` mints a
fresh never-reused guard; reads need that guard, writes need an open invariant,
and unlock or typed-error exit requires a closed invariant with no live
guard-derived borrow, dropping every guard-scoped fact. Custody transfer carries
a checker-verified `SyncWitness` compared against derived happens-before, in the
`weak_memory` style; no `bool` occupies an authority position. Dynamic shapes,
condition variables, try/timed locks, rwlocks, nested protecting mutexes, lock
ranks and recursive acquisition are represented refusals, not omissions.
Diagnostics occupy the free `Q610`-`Q623` range with `Q600` retained for carried
-over structural refusals. Certificates name exact generations, admitted
invariant transitions and derived edges; an independent recheck rederives every
field and refuses either trusted premise, including through the plain-string
route. 48 new tests bring the workspace to 898 passing; 46 provider tests,
43 Python tests, both probes and the standing gates pass, and
`check_lean_audits.py` passes 21 audits after a new `QCore.Concurrency.MutexInvariant`
namespace lands all 18 designed theorems plus 13 supporting ones and 29 decidable
scenarios, with no new axiom. Evidence:
`docs/reports/remaining-t02/README.md`. This is model-level only: no source
lowering, no CT case mapped (11/50 unchanged), no raw unit promoted, and the four
pinned 185-unit artifacts are unchanged by construction rather than re-measured,
the corpus being absent from the worktree. `DeclaredFootprintShape` and
`InvariantPredicateDeclared` are new declared premises; source-to-trace
refinement, C layout, deadlock freedom, source lowering (T03), general recursive
shape preservation and `research_closure` profile recording remain open.

Typed scalar-field access classification (2026-09-07): W4 replaces the lexical
indirect-access gate with a typed premise for one conjunctive whitelist. A
statement is a resource-projection no-op only when it carries no call, no `&`,
`*`, `[` or `]`, every access is a direct `base -> field` or `base . field` on a
declared single-star record pointer or record value, the record is a `struct`
defined exactly once whose every member declaration was read, the field's
declared type is a non-pointer, non-array arithmetic or enumeration type without
`volatile` or `_Atomic`, the access does not continue, every other identifier is
an enumeration constant or a declared scalar variable, and the base is a name
this function never binds to a resource identity. That last premise was added
after an implementation of C00's recommendation turned a read through a released
pointer into `Safe`; the existing fail-closed pin caught it and was not weakened.
Unions, bitfields, nested anonymous aggregates, function-pointer members,
pointer and array fields, chains, subscripts, casts, address-of escapes and
call-bearing statements all keep their existing `Unknown` and diagnostics, and a
declaration the reader cannot read refuses every access in its function. The
premise is read from declarations in the same preprocessed token stream rather
than a separate Clang AST, so W4 derivation, W4 validation and certificate
rederivation repeat the classification from the certificate's own content; the
deviation from C00's recommendation and its cost are recorded. Branch, loop and
return conditions, and C02-P borrowed pointer-field reads, stay out of scope.
A held-out generic fixture family checks two admitted owned lifetimes, three
newly surfaced violations (leak, double release, release after escape), a
caller whose classified callee stops releasing, twelve ambiguous accesses that
each keep the existing diagnostic, and four declaration substitutions the
certificate recheck refuses with their measured reason. The canonical CMetrics
audit changes exactly one function: `cmt_counter_allow_reset` moves Unknown to
Safe in the resource projection and its unsupported diagnostic disappears
(36 to 35). `cmt_counter_create` is unchanged because its `calloc`-bound base is
refused, and the six source-contract Unknown functions remain Unknown on their
other blockers, as C00 predicted. The raw unit remains provisional Unsafe with
one checked function contract and six Unknown; no unit is promoted. The pinned
185-unit corpus cannot be re-audited here — the developer checkout has moved off
`66910c10` and the pin gate refuses — so the four archived artifacts are left
untouched and their re-measurement and re-pinning is a named follow-up, where
any newly surfaced provisional Unsafe unit must be recorded. Thirty-one Lean
theorems cover the neutral instruction and the whitelist, including its
non-vacuity and the released-base refusal; the reader's type extraction and C
effective types remain explicit premises. 937 workspace tests (36 new), 47
provider tests, 43 Python tests, 20 Lean audits and all standing gates pass.
CT coverage remains 11/50 mapped and the contract corpus 59 modules. Evidence:
`docs/reports/remaining-c02/README.md`.

Remaining-work wave accounting (2026-09-07, implementation checkpoint `67b1a9a`):
the twenty-five cards recorded
under `docs/reports/remaining-*/` are reconciled into one traceable matrix,
`docs/reviews/r0_3/acceptance_matrix.json`, with a readable companion at
`docs/reports/remaining-d02/README.md`. This paragraph supersedes nothing above:
each card's own record keeps its checkpoint, counts and boundaries. Milestone
criteria met, at their stated boundaries: the documented checkpoint, submodule
pin, branch integration, CT inventory, contract corpus and report presence all
reconcile, and the recorded 819/45/43/20 baseline reproduced exactly before any
change; the R01–R10 soundness gates were verified adversarially with no
soundness failure found and no production semantics changed by the V cards;
source-derived obligation completeness and evidence identity were checked
against 52 replay/cache mutations and a compile-task identity that retires a
witness when a used header changes, with zero accepted mutations; the six
CMetrics Unknown functions are classified into a ranked, corpus-measured blocker
inventory; one generic blocker family — typed direct scalar-field access — is
implemented as a conjunctive type-only whitelist with 31 Lean theorems and both
precision directions exercised; a second pinned source set is audited from
configuration alone with no analysis-code change; the provider execution profile
now installs the limits a manifest declares on Linux and refuses rather than
silently downgrading, with an accurate non-sandbox trust boundary; the CT ledger
is stage-derived, so no mapped case claims a stage its executed evidence does
not carry. Nothing is promoted: raw Cycle 19 units remain 2 Safe / 42
provisional Unsafe / 137 Unknown plus four data-only units, CT coverage remains
11/50 mapped, the contract corpus remains 59 modules and full language
conformance remains false. `qcore-schemas` is now 3.0.0 (P02's public
`PluginLimits::additional` field); prose revision 0.3, plugin protocol v1,
evidence v2 and program schema/profile v2 are unchanged. Current gate: 997
workspace tests, 47 provider tests, 43 Python tests, 21 Lean audits, both
reevaluation examples exit 0, fmt and clippy clean. Named follow-ups: 185-unit
corpus re-measurement and deliberate re-pinning after C02 and G02; E01-F1
definition enumeration; C02-P; O04; T03; P03; P01-F2; G03-F1/F2; the L21.1
reason-code implementation increment; and the T00 stage-derived CT manifest
schema.

Delegated increment (2026-09-08): five bounded cards landed through isolated
worktrees with per-card reports; this paragraph supersedes nothing above and
each card's record keeps its own boundaries.
- Multi-provider composition (E03): seventeen composition properties verified
  against unchanged code and pinned; two fail-closed repairs in
  `compose_verification` — an unchecked `Unsafe` component outcome now
  downgrades to Unknown under QP010 instead of being republished verbatim, and
  unequal component premise sets refuse under QP012 instead of publishing one
  component's assumptions. Fifteen audited Lean theorems in a new
  `Composition` namespace, no new axiom. Four completeness boundaries stay
  recorded open (E03-B in the acceptance matrix). Evidence:
  `docs/reports/remaining-e03/README.md`.
- Enforced confinement (P03): the verifier host gains a target-qualified
  `linux-confined-v1` profile composing P02's setrlimit plan with Landlock
  filesystem confinement and a `CLONE_NEWUSER|CLONE_NEWNET` network namespace
  installed before exec. No default policy; five fail-closed pre-spawn refusal
  grounds, each a scoped Unknown under QP006 with no downgrade. Twelve
  adversarial fixtures with legacy controls measure a blocked localhost
  connect (0 vs 1 accepted connections), blocked out-of-path read/write and
  working admitted paths. This is confinement of two named resource classes,
  not an OS sandbox: no seccomp, no cgroup, no other namespace, and admitted
  paths stay fully reachable. The default remains `unenforced-legacy`.
  Evidence: `docs/reports/remaining-p03/README.md`.
- ChunkIO counter through the process boundary (P04): the unsigned
  unit-counter contract now runs as package `qcore.chunkio` 0.5.0, computing
  its own typed path projection with no derived result in the request; the
  core evaluates the pinned contract on its sealed import and only then
  requires agreement, so agreement alone discharges nothing. Byte-identical
  certificate parity with the retained in-process route on all four helpers;
  missing-evidence, tampered and 25 mutation/refusal cases fail closed.
  `qcore-schemas` is 4.0.0 (three source-incompatible additions; existing wire
  bytes unchanged and pinned). Thirteen other in-process ChunkIO capabilities
  keep named blockers. Evidence: `docs/reports/remaining-p04/README.md`.
- Source mutex/guard lowering (T03): `source.mutex.guard.v1` lowers a lexical
  `with lock m as g` guard onto the unedited `CheckedMutexInvariantV1`
  checker, with C11 emission and independent reference execution compared
  line-for-line under four fixed ASan/UBSan/LSan runs. Twenty-nine tests pin
  exact diagnostic vectors across Q610–Q622 with Q600/Q013 refusals; guard
  binders stay body-local and no diagnostic code was added or changed. Fourteen
  new public Lean theorems; the Lean event model still lacks a
  resource-destruction constructor, so that half stays Rust-tested. CT26,
  CT27, CT29 and CT30 mappings are proposed with measured stages, but the
  manifest is unchanged pending harness fixture integration: CT stays 11/50.
  Evidence: `docs/reports/remaining-t03/README.md`.
- Recursive-summary design (C03): neutral-frame recursion is specified as the
  next W4 family — a two-point per-function lattice with monotone descent to
  the greatest post-fixed point, an empty alias obligation and a ten-item
  post-fixed-point validation contract — motivated by a 309-file census
  (25 of 45 self-recursive functions in scope are effect-neutral; the
  implemented routes reach at most 3). A demonstrated counterexample shows
  validation-by-reanalysis alone cannot refuse a circular effectful candidate,
  so any effectful route keeps a rank. Design only: no production file
  changed, and heap-link chain teardown is rejected with recorded ASan
  hazards. Evidence: `docs/reports/remaining-c03/README.md` and
  `docs/design/W4_RECURSIVE_SUMMARY_PROFILES.md`.

Nothing is promoted by this increment: raw Cycle 19 units stay 2 Safe /
42 provisional Unsafe / 137 Unknown plus four data-only units, CT coverage
stays 11/50 mapped, the contract corpus stays 59 modules and full language
conformance stays false. The four pinned 185-unit artifacts were not
re-measured (CORPUS-REPIN remains open). Merged gate at this checkpoint:
1080 workspace tests, 51 provider tests, 43 Python tests, 21 Lean audits,
both reevaluation examples exit 0, fmt and clippy clean. A pre-existing P02
`RLIMIT_NPROC` conformance test is load-flaky (it counts every process of the
real UID); recorded in `docs/reports/remaining-p03/README.md` §9.1 with the
assertion deliberately not weakened.

Second delegated increment (2026-09-08): seven bounded cards landed through a
second delegated wave (five isolated worktrees plus two lanes coordinated in
the integration worktree under explicit per-lane file ownership); this
paragraph supersedes nothing above and each card's record keeps its own
boundaries.
- Scoped read loans (T04): `ScopedReadLoansV1` extends the checked finite task
  profile with one parent-owned Buffer lent to at most one scoped child as a
  read-only loan closed by creator join. The ledger adds a `Loaned` owner
  state with shared-read versus exclusive-use gates; a loan is not a value
  (refused as result, entry parameter, copy/move/re-loan source and drop
  operand). Failed creation restores captures; failed join reclaims nothing.
  `borrow<Buffer>` parameters and `read PLACE share FULL` captures lower with
  no parser change; native and reference runs pass the same pointer and
  differ only in what the caller keeps, under fixed ASan/UBSan/LSan runs.
  Seventeen public Lean theorems in a new `QCore.ReadLoan` namespace; 18 new
  tests. ThreadSanitizer does not intercept C11 `thrd_create` (measured
  SEGV), so data-race freedom rests on the checker's no-write-while-loaned
  rule, not on measurement. CT24/CT25 mappings are proposed in the report
  only; Buffer writes, fractional shares (T05), write loans and nested
  workers remain refused. Evidence: `docs/reports/remaining-t04/README.md`.
- Reactor callback capability through the process boundary (P05): the reactor
  callback assignment inventory (registration index, event identity and
  generation, declared callback authority) now runs as package
  `qcore.libevent` 0.2.0, computing its proposal from a transported program
  input that carries no derived result; the sealed core import rebinds the
  proposal to its own program and re-runs `check_event_program` itself, so
  agreement alone discharges nothing. Bufferevent operations, raw socket
  custody, ordering assumptions and loop control have no wire record and stay
  Unknown; an empty accepted-build list is `UnsupportedBackend`, never
  acceptance. `qcore-schemas` is 5.0.0 (two forced source-incompatible
  additions; existing wire bytes pinned byte-identical). The moved capability
  has no production caller yet — it is exercised by the Cycle 18 unit tests,
  not the Cycle 19 audit — and the in-process route is retained (P09).
  Evidence: `docs/reports/remaining-p05/README.md`.
- CTraces capability definition (P06): design only, by measurement — a census
  of all 273 function definitions in the 17 pinned CTraces units finds zero
  functions any implemented checker accepts, and the `ctraces` profile entry
  imports and asserts nothing. `ctraces.span.dropped_counts.v1` is specified
  over the four dropped-count setters with a 19-row malicious-witness matrix
  and six negative controls; four refusal tests drive the real importer over
  real upstream bodies and panic if any CTraces function ever yields a
  summary. Two dangling manifest identities (an unimplemented
  `certificate_checker` name and an `evidence_versions` mismatch) are
  recorded as follow-on P06-I4, not repaired. The contract text stays outside
  `contract/qcore/`, so the corpus count is untouched. Evidence:
  `docs/reports/remaining-p06/README.md`.
- librdkafka capability definition (P07): design only, by measurement — the
  adapter imports nothing, and its six declared-but-unloaded mechanism
  operations are mostly unfaithful to the real source (a function-like macro,
  an absent symbol, a wrong arity, a void return taking the entry as input,
  and one unsound postcondition); these are defects in declared unloaded
  contract text, not upstream defect claims, and nothing derived from them.
  A 96-function census finds zero admissible functions with one single-reason
  near miss sizing follow-on P07-U1.
  `qcore.librdkafka.queue_final_destroy.v1` is specified with two disjoint
  outcomes and seven premises; reclamation, callbacks and concurrency are
  excluded, not assumed. Evidence: `docs/reports/remaining-p07/README.md`.
- Constructor-to-caller custody (O04): on byte-verbatim Fluent Bit `90a0109`
  bodies, the three modeled failure arms of `flb_input_chunk_create` release
  the chunk produced at `cio_chunk_open` exactly once, all naming the same
  acquisition site; the constructor's four guard/allocation-failure exits are
  discharged while its rollback and success exits stay Unknown, so its
  declared contract remains a named premise, not a fact its body discharges.
  The C02-P borrowed-actual admission module records per-exit residuals;
  37 Lean theorems in `QCore.ExitResiduals` include both fail-closed
  directions. The canonical CMetrics audit is byte-identical before and
  after; both functions stay Unknown, and one C03 pin was deliberately
  updated (its named blocker moved to the branch condition, recorded in the
  test and report). Evidence: `docs/reports/remaining-o04/README.md`.
- Checker/model correspondence (F01): `check_concurrent` and
  `QCore.Concurrency.Lifetime.step`/`run` are tied by a seven-conjunct
  representation relation checked without instrumenting the checker, with
  domain and transcription agreement pinned as digests both sides compute
  independently; the whole depth-3 domain is decided in Lean and depths 3-4
  run differentially in Rust, with no divergence found in the claimed
  direction. Eight new theorems depend only on `propext` and `Quot.sound`.
  The claim is bounded at trace length 4 and not inductive; the transcription
  is digest-checked, not mechanically extracted; data races, unauthorized
  access and permission overflow are outside every stated correspondence; the
  `CheckedMutexInvariantV1` pair remains open. Evidence:
  `docs/reports/remaining-f01/README.md`.
- T03 CT integration: CT26, CT27, CT29 and CT30 are mapped with stage-derived
  evidence and exact ordered diagnostic cascades; CT coverage moves from
  11/50 to 15/50 mapped. CT26 is the first harness-gated sanitized-native CT
  case. CT28 and CT31 stay unmapped with recorded reasons; `build_matrix.py`
  is recorded out of date via a `generator_status` field rather than silently
  regenerated, and a stale T00 blocker label is corrected in both acceptance
  records. No semantic or diagnostic-code change. Evidence:
  `docs/reports/remaining-t03-ct-integration/README.md`.

Nothing is promoted by this increment: raw Cycle 19 units stay 2 Safe /
42 provisional Unsafe / 137 Unknown plus four data-only units, the contract
corpus stays 59 modules and full language conformance stays false. CT
coverage advances to 15/50 mapped, by executed-stage evidence only. The four
pinned 185-unit artifacts were not re-measured (CORPUS-REPIN remains open and
now names C02, G02 and O04). `qcore-schemas` is 5.0.0. Merged gate at this
checkpoint: 1141 workspace tests, 56 provider tests, 43 Python tests, 23 Lean
audits, both reevaluation examples exit 0, fmt and clippy clean in both
workspaces. Process note: the shared git stash stack caused one
cross-worktree stash consumption during the wave; the affected work was fully
recovered from dangling commits and nothing was lost. Prefer temporary WIP
commits over `git stash` in this repository.

Third delegated increment (2026-09-08): eleven bounded cards plus a
documentation restructure landed through a third and fourth delegated wave of
isolated worktrees with per-card reports; this paragraph supersedes nothing
above and each card's record keeps its own boundaries. A stricter closing
standard applied from this increment on: a card closes only with a Rust
implementation plus audited Lean modules; Python artifacts are supplementary.
- Counted read shares (T05): `ScopedReadSharesV1` turns T04's whole loan into
  a counted budget (`FULL_READ_AUTHORITY` = 1024, at most two holders);
  exclusive authority, `drop`/`move`/re-lend and the new checked Buffer write
  return only with the last share. 19 Lean theorems
  (`QCore.ReadShare`). Separation-logic fractions, slicing, write shares and a
  third reader stay refused. Evidence: `docs/reports/remaining-t05/README.md`.
- Typed owned error payloads (T06): `ScopedErrorPayloadV1` carries exactly one
  owned Buffer inside a declared error tag through completion and join;
  uniform rows, entry exclusion, one-shot `qcore_error_payload` take, 19 Lean
  theorems (`QCore.ErrorPayload`), six fixed sanitizer runs. Evidence:
  `docs/reports/remaining-t06/README.md`.
- Flat epoch accounting (T08): `FlatEpochBoundV1` checks a residency/work
  bound (shared union once, private peaks summed, declared overhead,
  completed-child results live until transfer; work never becomes wall-clock)
  with diagnostics `Q630`-`Q641` and 20 Lean theorems. Model-level only.
  Evidence: `docs/reports/remaining-t08/README.md`.
- OS failure post-states (O03): the Linux `close(2)` failure post-state
  relation — five descriptor states over three separated identities, EBADF as
  refutation, POSIX-unpinned targets Indeterminate with no permitted retry —
  is specified (`docs/design/OS_FAILURE_POSTSTATE_PROFILE.md`), implemented in
  `qcore-core::os_endpoint` (39 tests) and proved (27 Lean theorems); the
  Python matrix/probes are supplementary. Two declared-model defects
  (`DescriptorCustody.lean` failed-close retention; the `evutil_closesocket`
  error clause) are recorded open as U4/U5, not repaired. Evidence:
  `docs/reports/remaining-o03/README.md`.
- Bounded composition audit (P08): the five-provider registry reproduces each
  implemented capability's single-provider outcome, with absence/mismatch/
  timeout/conflict rows per provider; only four capability routes exist and
  cross-route composition is measured unreachable (QP015), so the full
  five-provider gate is recorded open, not satisfied. Evidence:
  `docs/reports/remaining-p08/README.md`.
- Error-region task credit (T07): `CheckedRegionTaskCreditV1` composes the
  region credit discipline with the unedited task ledger — duties are derived
  from the ledger and never convert into credit; abandonment waits for joins,
  loan expiry and child endowments; rethrow carries remaining duties.
  Diagnostics relocated to `Q670`-`Q677` at integration; adapted to T05's
  counted-share API. 23 Lean theorems. Model-level; CT21-23 need a source
  form. Evidence: `docs/reports/remaining-t07/README.md`.
- Disjoint footprints (T09): `DisjointFootprintsV1` decides byte-interval
  disjointness against one allocation generation with an 11-rule layout
  contract, bit-field storage units as single locations and protocol-cell
  descriptor identity; cross-allocation disjointness deliberately refuses.
  Diagnostics relocated to `Q650`-`Q667` at integration; 28 Lean theorems.
  Evidence: `docs/reports/remaining-t09/README.md`.
- Source-to-native trace relation (F02): `qcore.task-trace-relation.v1`
  relates reference and native renderings of the pre-T06 owned round trip by
  admitted causal constraints (nine conjuncts, 20 violation classes); the
  concurrent domain admits six schedules and the shipped backend renders one;
  whole-domain verdict digests tie the Rust relation to 26 Lean theorems. At
  integration the T05/T06 event forms were made explicit refusals and the two
  pinned emission digests were deliberately re-pinned after the T05/T06/macOS
  emission changes, with the native relation re-verified. Which order the
  native execution actually took remains unestablished. Evidence:
  `docs/reports/remaining-f02/README.md`.
- Route removal (P09): the hardcoded per-plugin first-party selection route is
  deleted after byte-identical re-measured parity with W2 H6 package
  discovery; `qcore-compiler` is versioned 0.2.0. P04's counter route was
  deliberately not chosen (live production callers). Seven remaining legacy
  routes are named follow-ons P09-A..G. Evidence:
  `docs/reports/remaining-p09/README.md`.
- Aggregate composition (O05): a compositional teardown contract for one
  aggregate parent and one wrapper — distinct child identities across parent
  release, callback quiescence and joined tasks before teardown, durability
  separate, status facts refused as evidence — decides `Composed`/`Unknown`
  only; on the real Cycle 18/19 subject it yields `Unknown` with named
  per-member blockers. 39 Lean theorems. Evidence:
  `docs/reports/remaining-o05/README.md`.
- Wave-3 CT integration: CT03/CT04/CT05/CT17/CT18 are mapped with
  stage-derived evidence, moving CT coverage 15/50 to **20/50**; CT03/CT17
  are the second and third harness-gated sanitized-native cases. CT11 stays
  unmapped on T05's alias caveat; CT35/CT36/CT38/CT39 stay unmapped pending
  epoch source syntax. CT03 records `cfg: executed`, matching the task-route
  convention over T05's proposal. Evidence:
  `docs/reports/remaining-wave3-ct-integration/README.md`.
- Documentation split: `README.md` is now a ~110-line human summary and
  `DEVELOPMENT.md` holds implementation/status detail, with every status line
  preserved verbatim; new per-increment paragraphs belong in DEVELOPMENT.md's
  "Increment records".

Integration notes: the concurrency diagnostic bands are now `Q610`-`Q623`
(mutex invariants), `Q630`-`Q641` (epoch accounting), `Q650`-`Q667` (disjoint
footprints) and `Q670`-`Q677` (region task credit), with `Q600` shared as the
carried-over structural refusal — T07 and T09 were relocated at integration
after landing on T08's band from independent worktrees; recording all four
bands in spec §L21 remains a deliberate follow-up edit. A module using both
counted shares and a typed error payload is labeled `ScopedReadSharesV1` and
the combined share+payload(+credit) feature set remains unvalidated. Upstream
macOS native-threading support was merged from origin and its emission changes
are covered by the re-pinned F02 digests.

Nothing is promoted by this increment: raw Cycle 19 units stay 2 Safe /
42 provisional Unsafe / 137 Unknown plus four data-only units, the contract
corpus stays 59 modules and full language conformance stays false. CT coverage
advances to 20/50 mapped, by executed-stage evidence only. The four pinned
185-unit artifacts were not re-measured (CORPUS-REPIN remains open).
`qcore-schemas` is 5.0.0; `qcore-compiler` is 0.2.0. Merged gate at this
checkpoint: 1379 workspace tests, 58 provider tests, 81 Python tests, 31 Lean
audits, both reevaluation examples exit 0, fmt and clippy clean in both
workspaces.

Windows native concurrency (2026-09-08): the shared emitted C11 thread subset
now includes a self-contained Windows implementation adapted from Monkey
winpthreads. Joinable start records preserve signed int results; failed native
creation frees unpublished storage, and failed wait/close retains the record
and result until successful join. Plain non-recursive mutexes use exclusive
SRW locks. Source task and mutex routes admit Windows; MSVC tests cover native
reference agreement, failure recovery, completed-but-unjoined custody and
mutex contention. Unix sanitizer fixtures and CT50's Linux mapping observation
retain their target scope; CT mappings and checker/Lean semantics are unchanged.
This is the emitted subset, not full pthread/C11-threads conformance.
The pinned Lean build and all 31 audits pass; the aggregate workspace gate
retains ten failures in three untouched audit targets.
See `docs/reports/windows-c11-threads/README.md` for provenance, build commands,
validation and platform limits.

Windows second-source-set fixture (2026-09-08): compile databases now name the
gateway-selected Clang and omit Unix-only build flags on Windows. All five
tests pass through Visual Studio discovery and explicit `QCORE_CLANG`; CI also
runs this target with Clang removed from PATH. Failed audit assertions include
status, stdout and stderr. Production checker and proof semantics are unchanged.
Formatting, workspace Clippy, both reevaluation probes and the pinned Lean build
pass. Workspace validation still fails in the untouched `o04_constructor_caller`
and `counter_source` targets.

Windows Clang fixture repair (2026-09-08): O04's reduced prelude uses
Clang target integer types, and the generic consuming sink accepts object
pointers without incompatible C calls. Fixture-local Git attributes preserve
the four upstream excerpts' exact LF bytes and existing hashes. Both the O04
and counter-source negative contract tests normalize only their local mutation
baselines and still assert that substitution occurred. All 14 O04 assertions
pass through native Visual Studio discovery; both affected targets (20 tests)
pass with explicit `QCORE_CLANG`. Windows CI covers both routes. Strict C type checks
also pass for Windows x64 and Linux x64 targets. Checker/Lean semantics,
production verdicts and pinned upstream contents are unchanged. The completed
Windows workspace gate records 1,316 passed, zero failed and one ignored,
superseding the earlier fixture-failure limitation. Formatting, workspace
Clippy, both reevaluation probes, the rebased Lean build and all 31 audits
pass. Validation:
`docs/reports/windows-clang-fixtures/README.md`.

Explicit concurrency imports and core prelude (2026-09-08): source files opt in
with `import std.concurrency;`; the concurrency context is unavailable by default.
The core owns exact bundled-module resolution; the frontend rejects unknown
modules (`Q014`) and missing imports (`Q015`) before checking each file, so imports
do not leak across compilation inputs. The task profile retains its separate
`concurrency checked;` marker. `qcore_core::prelude::concurrency` exposes existing
core models and backend APIs through compatible Rust imports; frontend lowering
and the verification driver consume it. `QCore.ConcurrencyPrelude` exposes 14
existing bounded properties and imports six new module-selection theorems.
Thread examples and current source fixtures opt in explicitly, with a paired
Rust custody example. Arbitrary user/package imports, implicit imports, combined
task/mutex semantics and general native correctness remain outside this change.
Validation: 1387 workspace tests passed, 1 ignored; all 32 Lean audits,
formatting, Clippy, both reevaluation probes and four native examples pass.
Evidence: `docs/reports/core-concurrency-prelude/README.md`.

Final observation increment (2026-09-08): the last two cards of the 49-card
remaining-work plan landed through isolated worktrees with per-card reports;
this paragraph supersedes nothing above and each card's record keeps its own
boundaries.
- Reopen corruption/checksum family (O01): an eight-fixture ChunkIO reopen
  observation family runs the real pinned `cio_chunk_open` path with
  `CIO_CHECKSUM` enabled — the first recorded `CIO_ERR_BAD_CHECKSUM` run in
  this repository — over five content classes and three injection states as
  disjoint axes, with the content class derived from recorded bytes by an
  independent Rust CRC-32, never read out of the report. All eight corrupted
  or clean images are byte-identical before and after; custody accounts
  80/80/0 allocation generations, 6 library closes plus 2 harness
  reclamations, and 7 library mapping epochs. The decision procedure is
  `qcore-core::chunk_integrity` (44 tests, 30-row mutation matrix, all
  refused) with 42 audited Lean theorems
  (`QCore.ChunkIntegrityObservation`). Two deliberate pin updates: the
  contract corpus moves 59 to 60 modules
  (`contract/qcore/chunkio/filesystem_integrity_observation.qcore`, 21
  clauses, no `resource_contract_v1` marker) and the loader's rejected count
  moves 144 to 145. The family compiles pinned `lib/chunkio` sources through
  ChunkIO's standalone CMake because the pinned Fluent Bit build tree is
  unavailable; no object-code equivalence with a Fluent Bit build is claimed,
  no production route consumes the relation, and no corruption recovery was
  observed. Evidence: `docs/reports/remaining-o01/README.md`.
- Capacity grow boundary (O02): the file-backed chunk grow boundary
  (`ftruncate`/`fallocate`/`posix_fallocate`/extending `write` and the
  `ENOSPC`/`EDQUOT`/`EFBIG`-class refusal) is recorded on x86_64 Linux over
  six sanitized native sequences: length-only grants versus reserved grants
  (`st_blocks` measured), refusal with identical-repeat refusal, a
  returned-error-without-`errno` arm, a partial transfer and a live
  mapping-beyond-end-of-file, with 16 epochs acquired and released. The
  decided relation is `qcore-core::capacity_boundary` (42 tests, 27 Rust plus
  14 Lean mutation rows, all refused) with 30 audited Lean theorems
  (`QCore.CapacityBoundary`); refusals carry module-local keys, no Q-code.
  `ENOSPC`/`EDQUOT` are classified from man-pages, not observed; metadata
  boundaries, the pinned-unit route and any production consumer remain open.
  Evidence: `docs/reports/remaining-o02/README.md`.

With these two cards the 49-card execution plan of 2026-09-07 is fully
dispositioned: every card is closed at its stated boundary or recorded as an
explicit named follow-on. Nothing is promoted by this increment: raw Cycle 19
units stay 2 Safe / 42 provisional Unsafe / 137 Unknown plus four data-only
units, CT coverage stays 20/50 mapped and full language conformance stays
false. The contract corpus is now 60 modules by O01's deliberate pin update;
the four pinned 185-unit artifacts were not re-measured (CORPUS-REPIN remains
open). `qcore-schemas` is 5.0.0; `qcore-compiler` is 0.2.0. Merged gate at
this checkpoint: 1473 workspace tests, 58 provider tests, 95 Python tests,
34 Lean audits, both reevaluation examples exit 0, fmt and clippy clean in
both workspaces.

macOS Cycle19 compiler-option transport (2026-09-08): ordered typed frontend
inputs now preserve Apple Clang `-Wundef-prefix=<prefix>` and `-arch
<architecture>` through preprocessing, syntax and typed-AST phases, while
malformed forms, missing explicit dialects and compiler substitution still
reject. The prepared `11651d47` replay completes 180 files / 181 configurations:
2 Safe / 41 provisional Unsafe / 134 Unknown / 4 data-only / 0 gateway failures,
with zero option refusals. This does not replace the pinned 185-unit ledger or
promote raw units. Evidence:
`docs/reports/macos-cycle19-compiler-options/README.md`.

Wave-5 delegated increment (2026-09-08): six bounded follow-on cards plus a CT
integration landed through delegated worktrees with per-card reports; this
paragraph supersedes nothing above and each card's record keeps its own
boundaries.
- Borrowed pointer-field actuals (C02-P): the admission had already landed
  inside O04's `borrowed_actual.rs`; no production line changed. The card
  adversarially re-verified it — fifteen refusal shapes keep their existing
  Unknown and diagnostics — and supplied the missing evidence: 36 held-out
  tests including the real `flb_input_chunk_create` -> `cio_chunk_open` shape
  and a contract-mode substitution the recheck refuses, plus 25 audited Lean
  theorems (`QCore.BorrowedActual`) modeling the `untracked_bases` closure.
  One boundary recorded, not repaired: two distinct declared names that may
  alias are unrelated by the name-based projection, pinned so the field-read
  and no-field-read forms move together. Evidence:
  `docs/reports/remaining-c02-p/README.md`.
- Neutral-frame recursion (C03-NFR): a third W4 recursive route descends a
  two-point per-member lattice to the greatest post-fixed point with all ten
  V1-V10 validation obligations checked; the design's circular-effectful
  counterexample is refused (the caller's leak is reported, not explained
  away) and a declaration-derived record is withheld (U7). Measured corpus
  reach is 0, correcting the design's ~9 estimate — all 12 reachable
  self-recursive functions refuse on other blockers — so the payoff gates on
  C02-P's admission. 34 audited Lean theorems (`QCore.NeutralFrameRecursion`);
  seven pins updated deliberately, each keeping its load-bearing negative
  half; the 185-unit audit re-measured byte-identical before/after at the pin.
  A pre-existing dead-uncontracted-call hazard is recorded, not repaired.
  Evidence: `docs/reports/remaining-c03-nfr/README.md`.
- Composition completeness (E03-B): the published composite is now a real
  merge (premise union, coverage union, per-obligation bounds, tightest task
  envelope, provenance sets) on a composite-only field; a lone component
  certificate is republished only when every component is that certificate.
  E03's QP012 unequal-premise-set refusal is deliberately superseded by the
  premise union (values still refuse); `derive_task` no longer folds the
  semantic profile into `build_digest`; observation obligations are keyed per
  provider and denominator; an unregistered provider refuses inside
  `composed` under existing QP002. `Composition` grows 15 -> 31 audited
  theorems with two pre-repair theorems retired by name;
  `qcore-verifier-host` and `qcore-verification` move to 0.2.0. Residuals
  recorded as E03-B-O. Evidence: `docs/reports/remaining-e03-b/README.md`.
- Guard-profile completions (T03-F): `close g unchanged;` derives the checked
  NoWriteOccurred witness for a decidable read-only region, dropping the
  declared invariant premise there; guard-scoped facts land as
  `let f = fact r;` / `assert f with g;` / `use f;` with cross-section use
  refused at the real Q616; `wait m;` gains a source form; a nested thread
  scope composes one child task in the mutex ledger only (a task created and
  joined strictly inside a guard stays refused by the unedited
  JoinWhileGuardHeld rule — recorded as T03-GUARD-JOIN, not lifted). The Lean
  event model gains the resource-destruction half: 19 new public theorems, no
  new axiom. Five new reserved words are recorded as a compatibility hazard
  and checked against every fixture. Evidence:
  `docs/reports/remaining-t03-f/README.md`.
- Manifest template binding (P01-F2): provider executables embed their
  manifest template; `verify`/`enable`, registry loads and the configured
  audit compare the installed manifest field-for-field against the template
  read from the executable's bytes, without executing it. All eight
  previously accepted digest-intact edits now refuse by field name; envelope
  absence/ambiguity/corruption each refuse distinctly; a template-less
  executable no longer verifies, so previously built packages must be rebuilt
  (all five shipped packages rebuilt and verified in the merged gate).
  28 audited Lean theorems (`QCore.PackageTemplateBinding`). Forged
  executables, TOCTOU and the unverified `from_package` loader stay open as
  P01-F3. Evidence: `docs/reports/remaining-p01-f2/README.md`.
- Bounded extern calls (`source.extern-call.v1`): a source function may call a
  declared extern function only behind `import std.extern;`, with a trusted
  C04 contract selected through `CompileOptions::extern_contracts` (inline
  source contracts stay refused), through a closed bridging table (scalars by
  value, borrowed Buffer, consumed own<Buffer>, nullable owned return) where
  every excluded C04 mode has a named refusal; the mandatory
  `succeeds BOOL else raise ERROR;` adapter lowers null returns onto the
  existing AllocError route, and its error arm is admitted only with no other
  live acquisition. Diagnostics Q022-Q027 under a new ExternCall pass with
  spec §L21 and the implemented grammars updated in the same change;
  Q020/Q021 untouched. Reference execution uses literal outcome controls; C
  emission prints ABI-table prototypes and checks declared nullness at
  runtime; two fixed ASan/UBSan/LSan fixtures link a real C callee, native
  matching reference line for line. 32 audited Lean theorems
  (`QCore.ExternCall`). Symbol resolution/linkage remains the builder's
  premise; the other call modes, sentinel adapters, extents, concurrent and
  error-region contexts stay named follow-ons (EXTERN-CALL-F). The design
  deviations, including the one new reserved word `succeeds`, are recorded.
  Evidence: `docs/reports/extern-call-v1/README.md` and
  `docs/design/EXTERN_CALL_SEMANTICS_V1.md`.
- Wave-5 CT integration: CT28, CT31, CT32 and CT48 map with stage-derived
  evidence and exact ordered cascades re-measured on fixture bytes; CT
  coverage moves 20/50 to 24/50. `cfg` stays unassessed for all four — the
  guard route has no CFG construction, and the harness refuses the overclaim.
  No native or proof evidence is added: sanitized-native stays 3 of 24 and
  proof 0 of 24, so the grown denominator is mapping, not conformance. CT33,
  CT34 and CT46 stay unmapped with unchanged reasons; the `build_matrix.py`
  divergence is recorded as thirteen cases in `generator_status`. Evidence:
  `docs/reports/wave5-ct-integration/README.md`.

Nothing is promoted by this increment: raw Cycle 19 units stay 2 Safe /
42 provisional Unsafe / 137 Unknown plus four data-only units in the archived
artifacts, the contract corpus stays 60 modules and full language conformance
stays false. CT coverage advances to 24/50 mapped, by executed-stage evidence
only. The four pinned 185-unit artifacts were not re-pinned; CORPUS-REPIN
remains open and now records C03-NFR's byte-stable 43-provisional-Unsafe
re-measurement at the pin, to be reconciled with the archived 42 and the
explicit-Clang rerun's 43. `qcore-schemas` stays 5.0.0; `qcore-compiler` stays
0.2.0; `qcore-verifier-host` and `qcore-verification` are 0.2.0. Merged gate
at this checkpoint: 1624 workspace tests (1 ignored), 58 provider tests,
95 Python tests, 37 Lean audits, both reevaluation examples exit 0, fmt and
clippy clean, and all five provider packages rebuilt and verified under the
new embedded-template requirement. Follow-on accounting lives in
`docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`, updated for
this wave).

Wave-6 pin-moving increment (2026-09-08): three coordinated cards landed
through delegated worktrees; this paragraph supersedes nothing above and each
card's record keeps its own boundaries.
- Reason codes (REASON-CODES): the spec §L21.1 closed vocabulary is
  implemented as `qcore_schemas::reason` — seven categories under a three-way
  subject partition where `Environment` has exactly one inhabitant by
  construction, so no environment failure can be typed as a source defect.
  `DerivationIncomplete` carries a required `category` named at all
  construction sites; audit-report units and source-contract functions carry
  a classified `reason_code` that fails closed to null rather than defaulting.
  Measured findings: an empty derivation_incomplete list serializes
  identically, so archived task digests do not move from this field alone
  (proved by three-way attribution on the real CMetrics run), and
  `providers/package.py --locked` had been broken since P09's version bumps
  (repaired; CI coverage stays open as P09-lock). A measured ordering hazard
  — four environment-unavailable reasons contain "unsupported" — is pinned
  refused. `qcore-schemas` is 6.0.0; all five provider packages rebuilt with
  only package_digest moving; 20 audited Lean theorems
  (`QCore.ReasonCategory`). Opaque tool text, the untyped ChunkSourceResult
  reason and the adapter-assembled sidecar vocabularies remain open
  (REASON-CODES-B). Evidence: `docs/reports/remaining-reason-codes/README.md`.
- Stage-derived CT schema (T00-CT-SCHEMA): the CT manifest is schema 2 —
  every stage status is an object carrying its executor, artifact,
  observation count and host gate; the conflated cfg/type_checker split into
  the fifteen real passes under per-route stage vocabularies, with wave 5's
  no-CFG guard-route rule made structural; sanitizer claims are gated (only
  CT03/CT17/CT26 may read executed; the five transcript-only cases read
  recorded_not_gated); acceptance.json's CT numbers are generated by two
  independent derivations compared to the committed file; `build_matrix.py`
  reads the manifest through the summarizer and is verified against it,
  ending the recorded thirteen-case divergence. Seventeen schema mutations
  all refuse; coverage stays exactly 24/50 and the derived numbers reproduce
  the hand-kept ones. Lean judged not applicable with reasoning recorded.
  The generator's non-CT sections remain a stale hand-coded snapshot, adopted
  surgically and recorded open as D02-GEN-REST. Evidence:
  `docs/reports/remaining-t00-ct-schema/README.md`.
- Corpus re-pin (CORPUS-REPIN): the four archived 185-unit artifacts are
  re-measured through the canonical pinned route on freshly prepared pinned
  clones (Fluent Bit 66910c10, explicit Clang) and deliberately re-pinned,
  with the complete diff archived under `docs/reports/corpus-repin/`.
  Headline counts are unchanged: 2 Safe / 42 provisional Unsafe /
  137 Unknown / 4 data-only, with no unit verdict change in either direction
  and no newly surfaced provisional Unsafe unit. The 42-vs-43 delta is
  resolved by measurement as files versus configurations: the current-route
  186-configuration inventory counts `src/flb_input_chunk.c` twice under
  G01's recorded compile-configuration ambiguity, and distinct
  provisional-Unsafe files are 42 under both routes. Twenty-three functions
  move Unknown to Safe at function level, all attributed to C02's typed
  scalar-field whitelist; leak diagnostics are unchanged at 271. The
  reason-code prediction matches 377/377 classified rows; source-effects.json
  is measured to carry no reason_code because the compatibility adapter, not
  qcore-audit, assembles it (recorded under REASON-CODES-B). One itemised
  input substitution is recorded: the archived ad-hoc GCC database is
  unreproducible, so the reproducible explicit-Clang preparation fed both
  routes, with the three bounded consequences named in the report. Evidence:
  `docs/reports/corpus-repin/README.md`.

Nothing is promoted by this increment: raw Cycle 19 units stay 2 Safe /
42 provisional Unsafe / 137 Unknown plus four data-only units — now on
freshly re-measured, deliberately re-pinned artifacts — CT coverage stays
24/50 mapped, the contract corpus stays 60 modules and full language
conformance stays false. `qcore-schemas` is 6.0.0; `qcore-compiler` stays
0.2.0. Merged gate at this checkpoint: 1651 workspace tests (1 ignored),
58 provider tests, 108 Python tests, 38 Lean audits, both reevaluation
examples exit 0, fmt and clippy clean, and all five provider packages
rebuilt and verified. Follow-on accounting lives in
`docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`, updated for
this wave).

Wave-7 delegated increment (2026-09-09): five bounded follow-on cards landed
through delegated worktrees with per-card reports, merged onto main one at a
time under the standing gate; this paragraph supersedes nothing above and each
card's record keeps its own boundaries.
- OS-failure declared-model repairs (O03-U4U5): DescriptorCustody.lean's
  failed-close clause no longer certifies a false retention — the four-field
  Close record keeps its success arm and every re-exported name, its failure
  arm is withdrawn, and failed closes live in a new TargetedClose whose
  validity is decided by closePostState itself, digest-tied to
  qcore-core::os_endpoint over the same 1920-triple domain. The reactor's
  CloseSocket failure branch now poisons the descriptor (retry, error
  inspection, reuse and attachment refuse; strictly more refusing, no code
  change). The evutil_closesocket clause is corrected against the pinned
  source with a deliberate 4-to-8 clause-count pin update. 16 new audited
  theorems; the reactor's missing target/errno vocabulary is recorded as
  O03-U4-REACTOR-TARGET. Evidence: `docs/reports/remaining-o03-u4u5/README.md`.
- Source roots and CMake flags (G03-F): profile-manifest version 2 admits
  exactly "." as a source root (v1 spellings mean the same thing under both
  versions; traversal spellings keep their refusals at both), and the
  transported option table grows 4 to 28 exact spellings, each with a measured
  preservation argument — the seven -O levels are preprocessing-visible, so
  the archived attempt that dropped them analysed different text. The real
  Libevent submodule audits over CMake's unmodified database from
  configuration alone: 21 units, 0 Safe / 4 provisional Unsafe / 17 Unknown,
  zero option refusals, all reason codes classifying. Two-command ambiguity is
  narrowed by compile_target.object_prefix, never merged. 18 audited Lean
  theorems (`QCore.SourceRootAdmission`); CMetrics/chunkio-core artifacts
  byte-identical. Evidence: `docs/reports/remaining-g03-f/README.md`.
- Per-arm constructor discharge (O04-ARMS): the borrowed-actual admission
  widens to bounded typed access chains (every link declared exactly
  `struct P *`, re-entered in the declaration model per link, depth capped at
  4 by refusal). On byte-verbatim flb_input_chunk_create the logging arm
  discharges and four of six exits settle; C5 records its named B08 residual.
  Measured corrections to the card premise: array declarators were already
  satisfied and the borrowed callees are two, with variadic flb_error declared
  at its one call-site arity (O04-ARMS-VARIADIC, no schema change). The
  185-unit corpus was re-measured twice through the pinned route: headline
  counts and every unit verdict unchanged, diagnostics move in 4 units in the
  fail-closed direction, and the current baseline is deliberately re-pinned to
  `docs/reports/remaining-o04-arms/artifacts/` with the wave-6 record kept
  intact. 23 audited Lean theorems (`QCore.ChainedAccess`). Evidence:
  `docs/reports/remaining-o04-arms/README.md`.
- Epoch source syntax (T08-EPOCH-SRC): `source.epoch.bound.v1` lowers a
  lexical `thread scope NAME epoch { ... }` declaration onto the unedited
  FlatEpochBoundV1 checker with independent recheck; the route is check-only
  by stated grounds (an admitted epoch is an accounting statement, so no C
  emission or native run). Profile refusals are driven through the core's
  Q630-Q641; eight new reserved words with one real collision found and
  avoided (`work` became `effort` rather than editing the pinned golden
  surface). Measured schema-2 rows for CT35/CT36/CT38/CT39 are parked with
  three named harness prerequisites; CT stays 24/50. 37 audited Lean theorems
  (`QCore.EpochSourceLowering`); spec §L21's Q6xx row now enumerates the four
  concurrency sub-bands, closing the wave-3 follow-up edit. Evidence:
  `docs/reports/remaining-t08-epoch-src/README.md`.
- Confinement bounds (P03-F): the confined profile gains an enforced per-tree
  child quota — cgroup v2 pids controller under a delegated subtree, joined in
  the first pre_exec closure, with the new `max_execution_processes` declared
  kind deliberately distinct from the per-UID `max_processes`; absent
  delegation or a zero quota refuse pre-spawn. The fork-storm fixture measures
  peak 4 with one refused fork under quota 4. Execution-profile selection is
  explicit configuration with `unenforced-legacy` still the recorded default;
  CI now executes refusal grounds 1 and 4 (the userns half against a genuinely
  denying kernel) and runs the provider package build/verify, closing
  P09-lock. P02-FLAKE's assertion is byte-for-byte unchanged with a
  deterministic companion. 29 audited Lean theorems
  (`QCore.ProcessQuotaPlan`); qcore-schemas stays 6.0.0. Evidence:
  `docs/reports/remaining-p03-f/README.md`.

Nothing is promoted by this increment: raw Cycle 19 units stay 2 Safe /
42 provisional Unsafe / 137 Unknown plus four data-only units — the baseline
artifacts are deliberately re-pinned to O04-ARMS's after-measurement with
zero verdict movement — CT coverage stays 24/50 mapped, the contract corpus
stays 60 modules and full language conformance stays false. `qcore-schemas`
stays 6.0.0. Merged gate at this checkpoint: 1747 workspace tests (3 ignored),
58 provider tests, 108 Python tests, 42 Lean audits, both reevaluation
examples exit 0, fmt and clippy clean, and all five provider packages rebuilt
and verified. Follow-on accounting lives in
`docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`, updated for
this wave).

Wave-8 delegated increment (2026-09-09): four bounded follow-on cards landed
through delegated worktrees with per-card reports, merged onto main one at a
time under the standing gate; this paragraph supersedes nothing above and each
card's record keeps its own boundaries.
- Legacy route removals (P09-ROUTES): P09-G, the zero-helper
  `chunk_source::import_chunk_source` wrapper, is removed after archived
  byte-identical parity over all seven consumer source shapes; the public API
  removal moves `qcore-compiler` to 0.3.0. The six other routes are retained,
  each with its measured missing piece named (P09-A..F gaps), and two of the
  P09 report's own notes are corrected by measurement: C's real blocker is
  the Direct-mode no-selector default, and F is a test-rehoming refactor plus
  multi-path API break, not eleven pure re-exports. Evidence:
  `docs/reports/remaining-p09-routes/README.md`.
- Epoch CT lane (T08-SRC-CT): the schema-2 harness gains the `epoch` route
  and the exhaustive `epoch_bound_figures` verdict mode (the whole published
  figure map is pinned, not a convenient subset); CT35, CT36, CT38 and CT39
  map with every cascade and figure re-measured on fixture bytes — CT
  coverage moves 24/50 to 28/50, sanitized-native stays 3 of 28 and proof
  0 of 28, so the grown denominator is mapping, not conformance. Eighteen
  scratch mutations all refuse. One deliberate deviation from the parked
  rows is recorded (`native_sanitized: not_applicable` — the check-only
  route emits nothing to sanitize, structural under the route rule) and one
  correction found by re-measurement (the parked certificate blocks
  under-enumerated the published figures). Evidence:
  `docs/reports/wave8-epoch-ct/README.md`.
- Function enumeration (E01-F1): the main-source function denominator is
  completed from a second, enumeration-only preprocessing with line markers,
  leaving the published `-E -P` text and every bound digest untouched. The
  185-unit denominator moves 4,775 to 4,921 (+146) with zero verdict
  movement in either direction; the actual cause is a raw-tokenizer defect
  (a `#` on a macro continuation line swallows a closing brace, hiding 146
  of rdkafka.c's 165 definitions), named `absent_from_raw_scan` and recorded
  as follow-on E01-F1-RAW rather than repaired. K&R definitions count with
  explicit Unknown rows and classified reason codes; refused enumeration
  claims withhold whole-unit Safe. The baseline is deliberately re-pinned to
  E01-F1's after-measurement after its before-run reproduced the prior
  baseline byte-identically. 38 audited Lean theorems
  (`QCore.SourceEnumeration`). Evidence:
  `docs/reports/remaining-e01-f1/README.md`.
- ChunkIO teardown capabilities (P04-NEXT): of the thirteen remaining
  in-process capabilities, the list-unlink two-summary relation and the
  memory-backend nullable-field destructor move through the process boundary
  in the P04/P05 pattern — both bodies' inputs travel with no derived
  reading in any request, calls are admitted only against a closed
  per-family callee table, and the sealed core re-evaluates the pinned
  contract before requiring agreement, which discharges nothing. Parity is
  byte-identical with the retained in-process routes; a 92-row mutation
  matrix all refuses; 26 audited Lean theorems (`QCore.TeardownAdmission`).
  The remaining eleven are classified: one movable with a third vocabulary
  (P04-NEXT-A), two dependency-blocked (B), two behind the sealed
  SourceMatch boundary (C), seven compositions. `qcore-schemas` is 7.0.0
  (source-incompatible additions with reasoning recorded; existing wire
  bytes pinned unchanged); ChunkIO package 0.6.0; all five packages rebuilt
  and verified. Evidence: `docs/reports/remaining-p04-next/README.md`.

Nothing is promoted by this increment: raw Cycle 19 units stay 2 Safe /
42 provisional Unsafe / 137 Unknown plus four data-only units — the baseline
artifacts are deliberately re-pinned to E01-F1's after-measurement with the
denominator at 4,921 and zero verdict movement — the contract corpus stays
60 modules and full language conformance stays false. CT coverage advances
to 28/50 mapped, by executed-stage evidence only. `qcore-schemas` is 7.0.0;
`qcore-compiler` is 0.3.0. Merged gate at this checkpoint: 1787 workspace
tests (3 ignored), 67 provider tests, 108 Python tests, 44 Lean audits, both
reevaluation examples exit 0, fmt and clippy clean, and all five provider
packages rebuilt and verified. Follow-on accounting lives in
`docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`, updated for
this wave).

Wave-9 delegated increment (2026-09-09): six bounded follow-on cards landed
through delegated worktrees with per-card reports, merged onto main one at a
time under the standing gate, and closed by one integrator corpus
consolidation; this paragraph supersedes nothing above and each card's record
keeps its own boundaries.
- Raw tokenizer repair (E01-F1-RAW): a `#` that is not the first preprocessing
  token of a logical line is no longer a directive introducer (C11 phase-2/
  6.10 semantics; stringization and pasting survive); the raw scan and the
  wave-8 enumeration now agree, with rdkafka.c's absent_from_raw_scan moving
  146 to 15. The verdict surface does not move. The 15-row residual is a
  second, distinct cause — conditional-arm brace imbalance — named
  E01-F1-RAW-COND and pinned, not repaired. 31 audited Lean theorems
  (`QCore.RawDirectiveLexing`) with an exhaustive two-sided lexing checksum
  tie. Evidence: `docs/reports/remaining-e01-f1-raw/README.md`.
- Aggregate stores (O04-B08): a conjunctive typed admission for storing a
  tracked resource into a field of a live, body-acquired aggregate
  (`ic->chunk = chunk`) as a field-link custody edge — deliberately shallow
  (P-B08-SHALLOW: escape carries live linked children, release does not
  cascade, overwrite never discharges) — plus a bounded null-test branch form.
  C5 discharges: five of six constructor exits settle, and only the
  out-of-scope success exit remains, gated on O04-B08-READ-BOUND-BASE. A
  C02-P fail-closed pin caught and corrected the first implementation rather
  than being weakened. Corpus diagnostics move fail-closed in 38 units with
  zero verdict movement and leak unchanged at 271. 42 audited Lean theorems
  (`QCore.AggregateStore`). Evidence:
  `docs/reports/remaining-o04-b08/README.md`.
- Guard-local join (T03-GUARD-JOIN): the soundness argument closes only for a
  rule narrower than proposed — the joined subtree must name no mutex at all,
  making deadlock structurally impossible rather than analyzed — landed as
  the opt-in `thread scope c under g` admission (eight checked conditions)
  with zero deleted checker lines, no diagnostic change, and 24 new Lean
  theorems. The T03-F asymmetry (a child ending its parent's borrow) is
  refused by G7. CT28/CT32 cascades verified unchanged. Evidence:
  `docs/reports/remaining-t03-guard-join/README.md`.
- C-twin CT lane (CT-CTWIN): the reserved `legacy_c` route becomes real — the
  C twin runs through the actual gateway with the whole verdict pinned
  (unit result, verbatim reason, complete function map, ordered diagnostic
  inventory) and tool-gated stage records. CT43 and CT45 map; CT40/CT41/CT42
  stay unmapped on measured invisibility/vacuity/loader-refusal grounds. CT
  coverage moves 28/50 to 30/50; sanitized-native stays 3 of 30 and proof
  0 of 30. One admitted mutation (M20, the inert declared-entry check) is
  recorded and asserted rather than hidden. Evidence:
  `docs/reports/wave9-ctwin/README.md`.
- Call-result vocabulary (P04-NEXT-AB): file_attempts travels as the third
  transported source vocabulary (`call_use_summary`) — policy-free, so which
  callee the contract counts never leaves the core — with byte-identical
  in-process parity on all five bodies and a 122-row mutation matrix, all
  refused. The dependent pair is corrected by measurement into P04-NEXT-B1
  (no contract-side referent: the vocabulary must be declared first) and
  P04-NEXT-B2 (the symbolic-replay fourth vocabulary);
  PrerequisiteIdentity/revalidate_prerequisite land with no production
  consumer yet. `qcore-schemas` is 8.0.0; ChunkIO package 0.7.0; 28 audited
  Lean theorems (`QCore.CallUseAdmission`). Evidence:
  `docs/reports/remaining-p04-next-ab/README.md`.
- Generator and reason residuals (GEN-REASON): the acceptance-matrix
  generator's every section is now derived, integrator-supplied (via
  `--gate`, else labelled carried) or carried verbatim from the committed
  matrix, with a published section_ownership block — whole-file regeneration
  no longer clobbers later reconciliations. REASON-CODES-B closes on its
  reachable surface: B2 types ChunkSourceResult's Unknown reason
  byte-neutrally, B1 is measured at zero occurrences in the pinned corpus
  with the three reachable conversions named, and B3's sidecar reason_code
  lands with the pinned bytes deliberately left for the re-pin. 15 audited
  Lean theorems (`QCore.ReasonCarrier`). Evidence:
  `docs/reports/wave9-gen-reason/README.md`.
- Corpus consolidation: one integrator measurement on the fully merged tree
  through the canonical pinned route re-pins the 185-unit baseline
  (`docs/reports/wave9-corpus/`): 2 Safe / 42 provisional Unsafe /
  137 Unknown / 4 data-only, zero verdict movement, and every changed leaf
  attributed to the union of the three archived per-card diffs plus the two
  known residue classes (path-literal provenance, package identity). The
  REASON-CODES-B3 tripwire test is flipped per its own design; the matrix is
  regenerated whole-file for the first time under the new generator.

Nothing is promoted by this increment: raw Cycle 19 units stay 2 Safe /
42 provisional Unsafe / 137 Unknown plus four data-only units on the freshly
re-pinned wave-9 baseline, the contract corpus stays 60 modules and full
language conformance stays false. CT coverage advances to 30/50 mapped, by
executed-stage evidence only. `qcore-schemas` is 8.0.0; `qcore-compiler`
stays 0.3.0. Merged gate at this checkpoint: 1888 workspace tests
(4 ignored), 75 provider tests, 125 Python tests, 48 Lean audits, both
reevaluation examples exit 0, fmt and clippy clean, and all five provider
packages rebuilt and verified. Follow-on accounting lives in
`docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`, updated for
this wave).

Wave-10 delegated increment (2026-09-09): seven bounded follow-on cards landed
through delegated worktrees at the doubled-throughput fleet (launched eleven,
trimmed to seven when worktree storage became the binding constraint — the
four stopped cards defer to wave 11 with their briefs unchanged), merged onto
main under the standing gate and closed by one integrator corpus
consolidation; this paragraph supersedes nothing above and each card's record
keeps its own boundaries.
- Reactor close-failure vocabulary (O03-REACTOR): CloseSocket failures may
  carry a declared (target, errno) pair resolved total-or-reject; the reactor
  answers with the os_endpoint relation's classification — POSIX-2024 EINTR
  retains with one retry (the only such row in the 4×8×3 domain, proved),
  releasing Linux classes retire the number, everything unpinned stays
  Poisoned. The two-sided digest grows to 2016 rows with Rust and Lean
  agreeing independently. Nine audited theorems. `qcore-compiler` 0.4.0.
  Evidence: `docs/reports/remaining-o03-reactor/README.md`.
- Cleanup-outcome vocabulary (P04-B1): both cleanup-outcome contracts declare
  the referents their profile binds under `cleanup_vocabulary_v1`, bound
  structurally against pinned source (13 source-binding, 17 profile and 20
  grammar refusals; clause counts 16/16/12 → 26/29/25, corpus stays 60
  modules). 37 audited Lean theorems (`QCore.CleanupVocabulary`). The only
  corpus movement of the wave: 18 contract_source leaves. Evidence:
  `docs/reports/remaining-p04-b1/README.md`.
- Loader and TOCTOU (P01-F3): no unverified provider path remains reachable
  from production (two real holes closed: discover_packages registered a
  package with no executable at all, and the compiler's first-party registry
  built providers from metadata alone; compile-fail pins); the
  verify-to-spawn window narrows from unbounded to a measured 0.5–24 ms via
  held-descriptor re-read plus (device, inode) identity, with the residual
  PROVED open in Lean rather than claimed closed, and a self-caught deadline
  regression repaired. 24 audited theorems (`QCore.PackageIdentity`);
  `qcore-verifier-host` 0.3.0. Evidence:
  `docs/reports/remaining-p01-f3/README.md`.
- Lock ranks (CT33-RANKS): the recorded CT33 blocker was measured incomplete
  — nested `with lock` was itself refused — so both moved together: `mutex
  rank N` declarations, an opt-in strict-order discipline (new Q624/Q625
  through the §L21 process; the mutex band is Q610–Q625) and rank-gated
  nesting, with unranked modules byte-identical. A rank is a declared
  position, not an assumption. 12 audited theorems; CT33's mapping is
  proposed with the measured [Q624,Q612] cascade for the next CT lane.
  Evidence: `docs/reports/remaining-ct33-ranks/README.md`.
- Composition residuals (E03-B-O): an absent provider's obligations now
  derive and refuse on absence (ObservationHooks withholds its vacuous
  inventory — E03-B-O1-HOOKS); composition is grouping-invariant on the
  whole outcome; BoundedPaths composes through the real firewall with a
  census tripwire for the still-unemitted count kinds. `Composition` grows
  to 61 audited theorems; `qcore-verification` 0.3.0. Evidence:
  `docs/reports/remaining-e03-b-o/README.md`.
- Extern out/stored modes (EXTERN-F1): `resource_out(nullable)` acquires on
  the success arm only (an unwritten slot is observed as failure and
  refuted, not believed); `resource_stored` consumes permanently under the
  declared owner with `resource_owned` admitted in exactly that position per
  §C04.2. Measured: the stored mode is checked but unreachable from a single
  source module — the abandonment gate refuses a second live acquisition and
  was deliberately not weakened (EXTERN-F1-STORE-REACH). `QCore.ExternCall`
  grows to 48 audited theorems; two new sanitized native fixtures. Evidence:
  `docs/reports/extern-call-f1/README.md`.
- Bound-base reads (O04-READ-BOUND): a member read through a base bound by an
  admitted aggregate store is admitted as a whole Borrowed actual with the
  linked children's liveness required (the released-child admission and a
  call-index span confusion were caught and closed during implementation);
  the READ-BOUND-BASE blocker falls and C6's residual narrows ten to nine
  refusals — C6 itself stays Unknown. All five corpus artifacts
  byte-identical. 44 audited theorems (`QCore.BoundFieldRead`). Evidence:
  `docs/reports/remaining-o04-read-bound/README.md`.
- Corpus consolidation: one integrator measurement on the merged tree
  (`docs/reports/wave10-corpus/`) re-pins the baseline — 2 Safe /
  42 provisional Unsafe / 137 Unknown / 4 data-only unchanged, and the only
  movement against wave 9 is P04-B1's 18 contract_source leaves, attributed
  pair by pair. The acceptance matrix regenerates whole-file at the merged
  numbers.

Nothing is promoted by this increment: raw Cycle 19 units stay 2 Safe /
42 provisional Unsafe / 137 Unknown plus four data-only units on the freshly
re-pinned wave-10 baseline, CT coverage stays 30/50 mapped, the contract
corpus stays 60 modules and full language conformance stays false.
`qcore-schemas` stays 8.0.0; `qcore-compiler` is 0.4.0; `qcore-verifier-host`
and `qcore-verification` are 0.3.0. Merged gate at this checkpoint: 2034
workspace tests (4 ignored), 76 provider tests, 125 Python tests, 51 Lean
audits, both reevaluation examples exit 0, fmt and clippy clean, and all five
provider packages rebuilt and verified. Follow-on accounting lives in
`docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`, updated for
this wave).
