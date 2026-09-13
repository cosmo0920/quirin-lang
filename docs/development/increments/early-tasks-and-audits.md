# Early integration records: tasks, custody and configured audits

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

<!-- migrated-section:start -->
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

<!-- migrated-section:end -->
