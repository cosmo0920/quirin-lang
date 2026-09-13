# Historical guidance: cycles and nested teardown

[Development table of contents](../../../../DEVELOPMENT.md) · [History index](../README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../../AGENTS.md); each card's report owns its evidence.

<!-- migrated-section:start -->
### Status paragraphs relocated from AGENTS.md and CLAUDE.md (2026-09-10, card DOCS-COMPACT)

Card DOCS-COMPACT compacted `AGENTS.md` and `CLAUDE.md` to the agent-facing
minimum (project description, repository map, headline ledger, hard
invariants, standing subsystem constraints, the per-change gate, entry
points). Every per-increment status paragraph those two files carried was
checked against this file paragraph by paragraph. The paragraphs below were
the ones **not** already preserved here; they are reproduced verbatim, in the
order they appeared, with their origin recorded. Where AGENTS.md and CLAUDE.md
carried different wordings of the same increment, both are kept, because each
carried facts the other did not. Nothing in this section is new: no count,
verdict or boundary changed, and each paragraph keeps the checkpoint it was
written at. The per-paragraph disposition ledger is
`docs/reports/wave16-docs-compact/README.md`.

#### Cycle status ledger as carried in AGENTS.md through wave 15

*Moved verbatim from `AGENTS.md` lines 60-231 — the full bullet ledger, including the long Cycle 19 teardown-observation run and the W0-W6 `Active direction` history.*

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

#### Cycle 19 nested-teardown boundary as carried in CLAUDE.md through wave 15

*Moved verbatim from `CLAUDE.md` lines 36-79 — the Claude-side teardown/failure observation bullet.*

- The current Cycle 19 nested-teardown boundary is recorded in
  `docs/reports/cycle19-file-descriptor-failures/README.md` and the AGENTS.md status
  ledger. Its native memory and controlled failure observations bind concrete allocation/
  ring provenance and rollback. General caller/failure, filesystem and event-
  composition gates remain open. Unsupported backend close now has a symbolic
  partial-release classification with unresolved backend custody and `Unknown`;
  exact opaque identities survive conditional stream/context release. Four
  observed unsupported-type inputs retain empty streams and return null chunks
  with complete rollback; this is not a universal caller/reachability proof.
  Twelve real Linux close/unmap/delete calls additionally bind OS endpoints in
  exclusive temporary fixtures. Six native one-file constructor/teardown runs
  bind 66 allocation generations and preserve mapping/descriptor residuals after
  parent free under EIO injection. General filesystem callers, arbitrary OS
  failure behavior, full sync guarantees and durability remain open. Twelve
  filesystem allocation-failure/control processes account for 81 acquired
  generations and 73 recorded releases. Two UBSan null-argument terminations
  retain eight outstanding prefix generations; only the ten returning cases
  complete teardown and leak checking. Failed root/chunk-name copies and a
  stream returned without a directory remain distinct observed outcomes. Eight
  further constructor OS-failure/control processes release all 88 generations;
  a suppressed rollback close retains the original descriptor/inode after null
  chunk return and parent release. POSIX resize return values and errno remain
  separate. Seven existing-file profiles additionally release all 70 generations,
  preserving original inode/content, distinct corrupted/read-only errors and
  descriptor custody after suppressed chmod/rollback-close failures. These
  observations do not establish arbitrary kernel failure rules. Five further
  residency/transaction/lock sequences release 50 generations and seven OS
  epochs. Logical rollback leaves the mapped header length unchanged and down/up
  reloads appended bytes. Locked begin retries while direct write succeeds;
  general transaction, lock and production-caller guarantees remain open.
  Three sync-failure sequences release 30 generations and four OS epochs.
  Failed commit retains the transaction and retry clears it. Down continues
  cleanup after failed sync; unmapped commit succeeds without syncing. These
  injected observations do not establish general OS failure or durability rules.
  Four failed-down sequences release 40 more generations and seven OS epochs.
  Failed unmap preserves mapping/descriptor identity, up ring and counter; prior
  successful sync survives failure. Combined sync/unmap failure stays unsynced,
  with recovery through retry or parent teardown releasing the original epoch.
  Four descriptor-failure sequences release 40 further generations and acquire
  five OS epochs. Failed close moves the chunk down but retains its descriptor;
  down/up retries reject. A second suppressed close retains that descriptor after
  parent free, with harness reclamation recorded separately. Failed reopen creates
  no epoch; successful retry creates a fresh one. General reopen failure paths,
  caller provenance, aggregate custody and full event composition remain open.

<!-- migrated-section:end -->
