# Claude Code project notes

> **Handoff, 2026-09-07:** The user requested that subagents halt and remaining
> work be documented. All three managed subagents were interrupted; none is
> running. Do not restart subagents without a new user instruction. Current
> implementation checkpoint: `e3e7c2c`; see [progress, remaining tasks and specs](docs/HANDOFF_2026-09-07.md).
> This handoff is not a claim that the overall generalization goal is complete.

@AGENTS.md

Human-facing implementation and status detail lives in `DEVELOPMENT.md`
(split out of `README.md`, which is now the short summary); add new
per-increment status paragraphs there.

Current normative specification: `docs/QCORE_SPEC_r0_3.md` and its contract
companion. Follow `docs/reviews/r0_3/acceptance.json` for current repair status;
the supplied archive review is older than merged W2–W5 implementations.
Persistent audit setup: `docs/CYCLE19_BUILD_INPUTS.md`.
Emacs source/contract highlighting: `editors/emacs/qcore-mode.el`; installation
and ERT checks are documented in `editors/emacs/README.md`. This is lexical
editor support, not a change to language conformance or verification.

Claude-specific additions:

- Native threading examples: `examples/threads/README.md` explains `cl.exe`
  compilation; `build-windows.ps1 -Run` emits, builds and runs copied-worker,
  owned-buffer and shared-reader examples into `target/thread-examples/`.
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
production code changed. Four new `qcore-core` tests pin the full terminal-state
x event-kind matrix with exact diagnostics (and the absence of authority
diagnostics), the refused-join direction with exact diagnostic vectors, and the
`Completed`/`Joined` ledger phases rejecting all ten operations with
`InactiveTask` while a completed child's captures stay in `Custodian::Capsule`.
Nineteen new Lean theorems give the `Lifetime` model per-constructor terminal
rejection, post-join corollaries and refused-join retention, with no new axioms.
All 823 workspace tests, 45 provider tests, 43 Python tests, 20 Lean audits and
the standing gates pass; CT coverage remains 11/50 and Cycle19 artifacts are
untouched. Evidence: `docs/reports/remaining-v02/README.md`. Source-to-trace
refinement, checked mutex invariants, loans and native correspondence remain open.

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
attribution defects recorded by V04 and V07 without changing a single diagnostic
code. `crates/qcore-verification/src/driver.rs` `locate` tested an inclusive
bound against an exclusive `FileRange::end`, so a diagnostic at byte 0 of a
non-first input file was blamed on the previous file; the repair selects the
containing range and keeps a fallback for the separator newline and
end-of-input. `crates/qcore-frontend/src/frontend.rs` no longer emits
`Span { 0, 0 }` for unconditional dimension/axis failures: per-slice provenance
maps back to the offending declaration's original span, and parse failures
report the caller's byte offsets instead of offsets into the reconstructed
buffer. `StaticEnvironment::check_located` is purely additive. When touching
these sites, keep attribution and rejection separate: fixing where a diagnostic
points is not a code change, and changing a code is semantic. `Q020` (task
route) and `Q021` (sequential route) both refuse unenforced source contracts and
must stay separate until a consumer lands — the task gate also refuses an empty
`contract { }` and the sequential gate accepts it, so unifying them would change
rejection coverage. `docs/QCORE_SPEC_r0_3.md` §L21 is now the canonical per-code
ownership table (a semantic pass owns a range; `source_diagnostic` is the
authoritative map) and §L21.1 specifies the reason-code vocabulary, which is
prose only — do not add a category field to `DerivationIncomplete` without
regenerating every task digest, provider package and witness in the same change.
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
Source-filename-sensitive transport (2026-09-07): the configured audit hands
Clang the compilation database's own input word from its own directory, so
file-name macros and the includer-directory rule for quoted includes answer as
the recorded compile command does; the synthesized source-parent quote path is
removed and declared source bytes must equal the file's. `CSourceInput` makes the
input identity explicit; anonymous texts reject `<stdin>` file-name expansions and
`__builtin_FILE`/`__builtin_FILE_NAME` reject in every declared mode. When
touching this transport, do not normalize the spelled input path (that changes
`__FILE__`), do not let a declared identity survive onto derived text, and do not
move the pinned Cycle19 compatibility route off `HistoricalStdin` without
re-measuring its 185-unit artifacts. A CLI regression pins that a shadowing header
in the audit's working directory can no longer buy a `Safe` verdict. G01's
`CompileTaskIdentity` binds this transport as `source_input`: keep it there, or a
standard-input run and a declared-file run record the same identity for different
preprocessing. All 894 workspace tests, 47 provider tests, 43 Python tests and 20
Lean audits pass; the CMetrics audit is unchanged at one checked function/six
Unknown with byte-identical diagnostics. Evidence:
`docs/reports/remaining-g02/README.md`.
Checked mutex invariants (2026-09-07): T02 implements `CheckedMutexInvariantV1`
in a new `crates/qcore-core/src/mutex_invariant.rs` with its own entry point,
`check_mutex_invariant_program`. `check_concurrent`'s body, its 26 pinned tests
and `proof/QCore/Concurrency.lean`'s `Lifetime` namespace are byte-identical;
`TrustedExplicitTrace` still issues `GuardFootprintAuthority` and
`SynchronizedCustodyEdges`, and the two premises are discharged only inside the
new profile, over supplied traces. Generation-indexed mutexes follow V01's
destruction rules; `CreateMutex` fixes its declared footprints and parks the
creator's `FULL_ACCESS` share, so protected footprints have one protector and no
fallback authority. Pointwise conservation over exact interval atoms is checked
at every step. Fresh guards key all protected access, writes require an open
invariant, and unlock/error exit require a closed one with no live borrow,
dropping every guard-scoped fact. `SyncWitness` edges are derived and compared,
never believed. New diagnostics occupy `Q610`-`Q623`; carried-over structural
refusals keep `Q600`. Certificates name exact generations and admitted invariant
transitions, and an independent recheck rederives them and refuses either
trusted premise through both the typed set and the plain-string route. 48 new
tests (898 workspace total), 46 provider tests, 43 Python tests, both probes and
all standing gates pass; a new `QCore.Concurrency.MutexInvariant` Lean namespace
and `AuditMutexInvariant.lean` bring `check_lean_audits.py` to 21 audits with all
18 designed theorems proved, no new axiom and no placeholder. Two obligations are
narrowed and named open: general `step_preserves_declared_shape` and the
pointwise-atom refinement of the abstract conservation total. Evidence:
`docs/reports/remaining-t02/README.md`. Source lowering (T03), CT mapping
(11/50 unchanged), lock-order analysis, condition variables, rwlocks, try/timed
locks and any raw promotion remain out of scope and open; the four pinned
185-unit artifacts are unchanged by construction, not re-measured.

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

Remaining-work wave accounting (2026-09-07): the wave's twenty-five
`docs/reports/remaining-*/` cards are reconciled into
`docs/reviews/r0_3/acceptance_matrix.json` (17 requirements x 4 independently
marked evidence classes, plus stage-derived CT rows) with a readable companion
at `docs/reports/remaining-d02/README.md`. AGENTS.md carries the single
consolidated status paragraph; read it there rather than duplicating it. The
implementation checkpoint is now `67b1a9a`; the halt handoff's standing
instruction not to restart subagents is unchanged. Nothing is promoted: raw
Cycle 19 units stay 2 Safe / 42 provisional Unsafe / 137 Unknown plus four
data-only units, CT coverage stays 11/50 mapped, the contract corpus stays 59
modules and full language conformance stays false. `qcore-schemas` is 3.0.0;
prose revision 0.3, protocol v1 and evidence v2 are unchanged.

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
