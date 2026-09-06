# QCore-C agent briefing

Consolidated, compressed project state for LLM agents. Humans should read
[README.md](README.md). Keep this file, `CLAUDE.md`, and `README.md` in sync
when project status changes; this file is the source of truth for agents.

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
| `crates/qcore-core/` | L1 kernel: quantitative types, ownership/error/cleanup/CFG semantics, concurrency/translation models, chunk lifecycle, generic reactor/descriptor custody, checked task IR/reference execution and C11 task emission. No runtime or workspace dependencies; W5 consolidation remains bounded |
| `crates/qcore-frontend/` | L1 front end (W1 relocation): `lexer`, `parser`, `check`, `quant_index`, `frontend` elaboration and exact scoped copy/owned-buffer source lowering. Depends on `qcore-core` only |
| `crates/qcore-contract/` | L1 contract machinery: total-or-reject loader, observation and declared chunk-path policy evaluation. Depends on core/frontend/schemas; Boolean source imports are opaque in the C gateway; compiler compatibility adapters remain |
| `crates/qcore-schemas/` | L0 published schema surface (W0 executed 2026-09-06): plugin protocol v1 types, resource-contract data model, mechanism vocabulary, source-identity records. Serde types only; a test forbids workspace dependencies |
| `crates/qcore-c-gateway/` | L2 existing-C analyzer, custody summary inference, Clang discovery, typed-AST transport and opaque Boolean-field source imports; depends only on contract/schemas plus serialization |
| `crates/qcorec/` | CLI binary `qcorec` plus its process-level tests |
| `crates/qcore-c-gateway/src/legacy_c.rs` | Generic unrestricted-C checker (Clang gateway, CFG, resource abstract interpretation, certificate recheck); contracts enter via `ResourceContracts` |
| `crates/qcore-c-gateway/src/legacy_summary.rs` | W4 interprocedural custody summary inference for internal C functions: SCC-ordered derivation, validation by re-analysis, declared-contract precedence. Relocated with Clang and typed-AST transport (2026-09-07); compiler compatibility reexports retained |
| `crates/qcore-verifier-host/` | L2 generic registry, source-scope resolution, bounded process transport, v1/v2 firewall and composition; depends only on frontend/schemas workspace crates |
| `crates/qcore-verification/` | Trusted adapter: core-derived obligations, lifecycle/observation witness replay and scope dispatch; depends on core/frontend/contract/schemas/gateway/host, never providers |
| `compiler/src/verifier_plugin.rs` | Compatibility facade for the host and first-party adapters; trusted replay/derivation/dispatch live in `qcore-verification` |
| `compiler/src/verifier_plugins/{libevent,chunkio}/` | In-process first-party providers (Libevent/Monkey, ChunkIO) |
| `compiler/src/external_{integration,source}.rs` | Cycle 19 library custody mechanisms + pinned source-set tables (over-fitted; see plan) |
| `compiler/examples/cycle19_fluent_bit_libraries_audit.rs` | Production audit orchestration for the pinned Fluent Bit corpus |
| `contract/qcore/` | Declarative `.qcore` contract corpus (58 files); counts pinned by `compiler/tests/qcore_contract_corpus.rs` |
| `proof/` | Lean 4 executable core and safety theorems; `AuditCycleN.lean` print axiom surfaces |
| `docs/cycles/` | Per-cycle requirement/acceptance records (`CYCLE_2` … `CYCLE_20`) |
| `docs/reports/` | Archived evidence packages (pinned audits, observations, triage) |
| `docs/reevaluation/` | Raw audit logs, probe JSON, `ct_manifest.json` (CT01–CT50 tracking) |
| `third_party/libevent` | Submodule pinned to the audited revision; a test fails if absent or moved |

## Status ledger (do not overstate)

Cycle completion labels describe their bounded milestone evidence, not full
revision-0.3 source-language conformance. The current workspace has nine crates.

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
  only; CT01–CT50 conformance corpus is **42/50 unmapped**
  (CT01/CT12 execute Linux owned-buffer transfer/creation restoration; CT02
  rejects actual post-move access. CT16/CT37 retain sequential-core fixtures;
  CT19 checks real sibling failure source, CT13 has Linux creation-recovery
  evidence and CT15 has Linux native join-failure evidence).
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
  pipeline and fails closed on manifest/fixture disagreement; CT02, CT12,
  CT16 and CT37 retain Cycle 12 sequential ownership/typed-error/cleanup
  fixtures. CT19 now checks actual sibling-failure source; CT15 executes
  Linux native join-failure and successful reclamation controls. CT13 checks
  explicit creation recovery and retained sibling storage. Shares and
  mutex source paths remain unexercised. Batch 2 (2026-09-06,
  `docs/reports/w6-ct-batch-2/README.md`) adds CT19/CT37 and records the
  full triage of all 47 then-unmapped cases (2 decidable-now, 8 blocked on
  W3/W4, 33 blocked on unimplemented syntax, 4 ambiguous); the later task recovery increment leaves 43 cases
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
`docs/QCORE_SPEC_r0_3.md`, `docs/QCORE_CONTRACT_SPEC_r0_3.md`,
`docs/QCORE_MODULE_MODES.md`,
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
fixed points, arithmetic rankings, aliases and cost summaries remain open.

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
8/50 mapped, 42 unmapped. Lean transfer/typing lemmas and fixed native sanitizer
runs are separate evidence; Rust/Lean/C refinement, loans, general owned graphs,
owned errors, worker allocation, shared memory, mutexes, nested workers and
other native platforms remain open. Evidence:
`docs/reports/w6-owned-source-tasks/README.md`.
