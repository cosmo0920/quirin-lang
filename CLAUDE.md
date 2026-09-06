# Claude Code project notes

@AGENTS.md

Current normative specification: `docs/QCORE_SPEC_r0_3.md` and its contract
companion. Follow `docs/reviews/r0_3/acceptance.json` for current repair status;
the supplied archive review is older than merged W2–W5 implementations.
Persistent audit setup: `docs/CYCLE19_BUILD_INPUTS.md`.

Claude-specific additions:

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

- Before starting checker/architecture work, read `docs/GENERALIZATION_PLAN.md`
  and land code according to its target crate layering (§3) and invariants
  (§2); do not add new code to the monolithic grab-bag once a target crate
  exists.
- W4 is executed for parameter/return custody with validation
  (`crates/qcore-c-gateway/src/legacy_summary.rs`,
  `docs/reports/w4-custody-inference/README.md`). A call into an internal C
  function is modeled only through a declared `.qcore` contract or a summary
  that `observations_match_contract` validated against a fresh analyzer run;
  everything else stays `Unknown`. When touching that pass, do not let a
  summary be published without validation, do not seed a formal parameter with
  entry custody the body did not demand (that asserts non-nullness and deletes
  failure paths), and do not accept an assume-guarantee fixed point for a
  recursive component. The 2026-09-07 extension validates direct `Out` slots,
  `Stored` formals, `Reallocated` results and possibly-null owned returns
  (`docs/reports/w4-custody-extensions/README.md`). Reassigned out pointers,
  Direct `Removed` wrappers are inferred and validated; general removal aliases and general recursive components remain unsupported; the
  `qcore-c-gateway` relocation is executed with compiler compatibility reexports
  (`docs/reports/w4-gateway-crate/README.md`).
- Never weaken a fail-closed gate, delete a pinned expectation, or downgrade
  a diagnostic to make a test pass. If a pinned table (contract corpus counts,
  submodule revision, archived report) conflicts with your change, the change
  must update the pin deliberately and say so.
- Status language matters in this project: keep the Safe/Unsafe/Unknown
  verdict vocabulary and the "unpromoted" phrasing exactly as defined in
  AGENTS.md when editing docs, reports, or commit messages.
- Run the full standing per-change gate from AGENTS.md before reporting a
  task complete; report probe/example exit codes honestly (a nonzero probe
  exit is a finding, not a build failure).

## Reopen failure evidence (2026-09-07)

Five native reopen size-lookup/mapping failure sequences release all 50
allocation generations and six mapping epochs. Ten descriptor epochs yield eight
library closes and two residuals after parent release, reclaimed separately by
the harness. Failed size lookup retains an unmapped descriptor; failed mapping
stores MAP_FAILED, reports up-ring membership without incrementing the counter,
and retains the new descriptor. Fixed retries and already-synced behavior remain
observations, not general OS failure or durability guarantees. Evidence:
`docs/reports/cycle19-reopen-failures/README.md`. Raw units remain unpromoted.

- W2's bounded profile migration and full 185-TU parity are recorded in
  `docs/reports/w2-profile-manifests/README.md`; remaining H4/H5/H6 and H7
  orchestration work is explicit there. Profile data remains declared trust.

Frontend gates repaired (2026-09-07,
`docs/reports/w6-frontend-gates/README.md`): nonempty source function contracts
reject centrally with Q021 until enforced. Guard/thread-scope header names bind
only inside their bodies; at that checkpoint nested concurrency lowering rejected with Q500.
This closes the two W6 batch-2 frontend defects without claiming contract or
concurrency semantics were implemented by that increment. CT mapping was then 5/50;
the native source-task increment below advances it.

- W3 bounded evidence-v2 replay/dispatch is recorded in
  `docs/reports/w3-evidence-v2/README.md`. Positive observation certificates
  describe recorded runs under `observations_trusted`; lifecycle source
  derivation stays incomplete until real effect/ABI import exists. The generic
  host is extracted in `crates/qcore-verifier-host` (record:
  `docs/reports/w3-verifier-host/README.md`); trusted derivation/replay/dispatch now reside
  in `qcore-verification` above the dependency-free kernel. V1 Safe refusal is unchanged.

- W2 H6 provider metadata now loads from profile packages; see
  `docs/reports/w2-provider-discovery/README.md`. This does not extract providers
  into processes or change the v1 Safe refusal.

Lean CI hygiene is implemented (2026-09-07): the pinned proof build, all theorem
audits and the complete QCore axiom closure are checked, with both reevaluation
examples in Linux/Windows CI. Local gates pass; hosted CI awaits a push.
See `docs/reports/w7-lean-ci/README.md`.

W7 generic Lean naming is complete for the bounded targets: DescriptorCustody,
EventComposition and busy_effect_preserves_lock, with historical aliases and
unchanged models (`docs/reports/w7-generic-lean-names/README.md`).

Boolean field profile routing landed (2026-09-07,
`docs/reports/w2-boolean-field-profiles/README.md`): typed C record/field and
Boolean operation routes come from loaded profile data, with exact declared
assumptions on summaries. An unrelated record fixture exercises both supported
models. Full 185-TU resource diagnostics and source projections are unchanged;
chunk reports add only three assumption maps. Frame-helper ABI generalization
and source-to-model identity premises remain open.

- The generic `qcore-audit` driver and retained Cycle19 compatibility adapter
  are documented in `docs/reports/w5-configured-audit/README.md`. No provider
  module is linked by the new driver; production evidence routes remain.

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
Coverage is 7/50 mapped, 43 unmapped. Moves/loans, owned results, shared memory,
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
import, owned task execution and broader W5 consolidation remain open.

W7 diagnostic attribution now requires the exact selected plugin ID plus one
1–128-byte ASCII local code (letters, digits, underscore or hyphen), in both
protocol response checks. Nested-provider spoofing, empty suffixes and control
characters reject with QP007 before verdict admission. Valid provider codes and
all existing evidence gates remain unchanged. This repairs attribution, not a
previously demonstrated Safe-admission bypass. Evidence:
`docs/reports/w7-diagnostic-attribution/README.md`.
