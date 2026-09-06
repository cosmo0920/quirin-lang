# QCore-C

QCore-C is a research language for **quantitative types**, **owned
resources**, **typed recoverable errors**, **checked cleanup**, and **checked
concurrency**, lowering to portable C11. Alongside the language, the project
builds a conservative, fail-closed checker for *existing* C source — the
"unrestricted-C gateway" — currently exercised against a pinned Fluent Bit
corpus.

- Current specification: [QCore revision 0.3](docs/QCORE_SPEC_r0_3.md), with
  [contract semantics](docs/QCORE_CONTRACT_SPEC_r0_3.md),
  [implemented grammars](docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md), and the
  [implementation acceptance ledger](docs/reviews/r0_3/REVIEW.md).
  Full language conformance is not claimed; revision 0.2 remains historical.
- [Persistent Cycle 19 build inputs](docs/CYCLE19_BUILD_INPUTS.md) replaces
  ephemeral `/tmp` build paths for new audits.
- Working with the repo as an AI agent? Start at [`AGENTS.md`](AGENTS.md) —
  the compressed status ledger, invariants, and per-change gate live there.

## What the type system tracks

QCore-C deliberately keeps separate quantitative domains separate instead of
overloading one number:

- **Usage grades** — how many times a value is demanded, as intervals.
- **Static indices and dimensions** — numeric type parameters with nominal
  units (`Bytes`, `Count<Fd>`, `Steps`, …); cross-unit arithmetic is rejected.
- **Quantitative credit** — linear, spendable capacity.
- **Live resource obligations** — what must be released, rolled back, or
  transferred; never weakened.
- **Cost summaries** — upper bounds, not tokens.
- **Effect rows and per-exit post-states** — what a function may do and what
  remains at each exit, including typed error exits.

The supported executable source subset lowers to C11 with a tagged-error ABI
and explicit cleanup. Translation checks and native differential runs cover
that reduced model; broader structural syntax and separately checked semantic
models do not imply complete source-language support.

## Repository layout

| Path | Contents |
|---|---|
| [`compiler/`](compiler/) | Rust library `qcore-compiler`: parser, checker, reference semantics, C backend, existing-C compatibility API, source driver and compatibility APIs |
| [`crates/qcore-verification/`](crates/qcore-verification/) | Trusted obligation derivation, witness replay and scope dispatch over core/frontend/contract/schemas/host |
| [`crates/qcore-core/`](crates/qcore-core/) | Semantic kernel: quantitative types, ownership, errors, cleanup, CFG, concurrency, translation and chunk lifecycle models |
| [`crates/qcore-frontend/`](crates/qcore-frontend/) | QCore language front end: lexer, parsers, quant-index resolution, elaboration |
| [`crates/qcore-contract/`](crates/qcore-contract/) | Declarative `.qcore` contract loader and observation interpreter |
| [`crates/qcore-schemas/`](crates/qcore-schemas/) | Published, versioned schema surface (serde types crossing the core/provider boundary; no analysis logic) |
| [`crates/qcore-verifier-host/`](crates/qcore-verifier-host/) | Generic registry, source scope resolution, bounded process transport, evidence firewall and composition; no provider dependency |
| [`crates/qcorec/`](crates/qcorec/) | CLI binary `qcorec` over the compiler library |
| [`proof/`](proof/) | Lean 4 executable core and safety proofs; per-cycle axiom audits |
| [`contract/qcore/`](contract/qcore/README.md) | Declarative `.qcore` contract corpus for audited C APIs |
| [`docs/`](docs/) | Specifications, cycle records, validation and audit reports |
| `third_party/libevent` | Submodule pinned to the audited Libevent revision |

Useful entry points: the source front end (`parse_source_program`,
`elaborate_source`), the integrated checker (`check_whole_program`), the file
compiler (`compile_files`), and the CLI binary `qcorec`
(`check`, `emit`, `plugins list|inspect`).

## Project status

Development proceeds in bounded, individually accepted **cycles**; every
record is archived under [`docs/cycles/`](docs/cycles/).

**Complete (Cycles 1–17).** The sequential language end to end — dimension
and quantitative-axis declarations, interval-graded usage demand, ownership,
resource-flow bounds, path-sensitive typed errors, sealed error-region
cleanup, CFG lowering to the tagged-error C11 ABI, an executable small-step
model — plus the structural front end, the whole-program checker, the
multi-file driver, translation certificates, a restricted concurrency
checker (access authority, happens-before, mutex capabilities, lock order,
destruction custody), advanced control (cancellation, fallible LIFO cleanup,
double errors, linear capture), and a combined closure certificate.
[Cycle 17](docs/cycles/CYCLE_17.md) added principal product inference, the
fail-closed unrestricted-C11 gateway, explicit weak-memory candidate
executions, and a native threaded custody backend, with recorded MSVC and
AddressSanitizer runs.

**Bounded and recorded (Cycle 18).** A fail-closed
[Libevent event-reactor checker](docs/cycles/CYCLE_18.md): generation-safe
lifecycle and custody tracking, explicit callback authority, priority
schedule exploration, cross-context conflict checks, and source pinning,
with Lean theorems and
[multi-platform build evidence](docs/cycles/CYCLE_18_LIBEVENT_VALIDATION.md).
Sanitizer and deployed observation-hook evidence remain pending.

**In progress (Cycle 19).** The gateway applied to a
[pinned Fluent Bit corpus](docs/cycles/CYCLE_19.md) (CMetrics, CTraces,
librdkafka, ChunkIO, the input-chunk wrapper, Monkey): resource-function
contracts, library custody mechanisms (refcount/list/queue/arena),
a bounded chunk-lifecycle model, and Monkey/Libevent callback composition
with recorded native observations. **Verdicts stay honest**: raw translation
units are not promoted to `Safe` until per-path identity imports and real
observation evidence satisfy the cycle's fail-closed gates — see the
[library audit record](docs/cycles/CYCLE_19_FLUENT_BIT_LIBRARIES_VALIDATION.md)
and the archived [evidence reports](docs/reports/).

The [latest teardown observations](docs/reports/cycle19-native-teardown/README.md)
run the pinned native constructors and complete context → stream → chunk →
backend memory teardown under ASan, UBSan and leak checking. Five executions
bind 82 allocation generations and 13 chunks to reciprocal rings and exact
release order. The source chain also preserves unresolved OS custody through
parent release in 26 of 43 conditional mixed-backend cases. All 407 workspace
tests and the pinned audit pass; aggregate verdicts are unchanged. A further
[nine allocation failures and control](docs/reports/cycle19-constructor-failures/README.md)
account for 55 generations across rollback and teardown, preserving null-field
successes and zero error outputs on backend failure. General caller/failure
provenance, filesystem guarantees, aggregate custody, remaining wrapper lifecycle
and full event composition remain open. The
[unsupported backend close classification](docs/reports/cycle19-unsupported-close/README.md)
checks all other integer backend kinds symbolically: outer chunk release
preserves unresolved backend custody, so the result remains `Unknown`.
[Conditional parent teardown](docs/reports/cycle19-nested-unsupported/README.md)
retains those exact opaque identities through stream and context release.
[Four native unsupported-type constructor cases](docs/reports/cycle19-unsupported-constructors/README.md)
return null chunks and release all 24 allocation generations through rollback
and surviving-stream teardown. General caller reachability remains open.
[Twelve real Linux cleanup observations](docs/reports/cycle19-file-os/README.md)
now bind close, unmap and deletion to OS endpoint probes and inode witnesses in
exclusive temporary fixtures. A failed zero-length unmap retains its mapping.
These helper executions do not establish arbitrary filesystem teardown or
durability. [Six enclosing native filesystem runs](docs/reports/cycle19-file-tree/README.md)
add constructor, ring, release-order and OS endpoint evidence for one-file trees.
All 66 allocation generations are released. Injected unmap/close failures retain
live OS resources after parent release, and sync failure still allows cleanup.
[Eleven filesystem allocation failures and a control](docs/reports/cycle19-file-constructor-failures/README.md)
now account for 81 acquired generations and 73 recorded releases. Failed root
and chunk-name copies trigger two observed UBSan null-argument terminations;
eight generations remain outstanding in those interrupted prefixes. Ten returning
cases complete leak checking. These observations do not establish general caller
or failure guarantees, and no raw unit is promoted.
[Eight constructor OS-failure cases](docs/reports/cycle19-file-os-constructors/README.md)
add 88 fully released allocation generations. A suppressed rollback close retains
the original descriptor/inode after null chunk return and parent release; harness
reclamation is recorded separately. File-size fallback and POSIX allocation
return/errno conventions remain distinct, with general OS guarantees still open.
[Seven existing-file profiles](docs/reports/cycle19-file-existing/README.md)
release all 70 allocation generations. Valid and read-only content, malformed
magic, short headers and read-only empty files retain distinct outcomes and their
original inode/bytes. Suppressed chmod plus rollback-close failures preserve a
descriptor residual after parent release. General caller and lifecycle gates
remain open.
[Five native transition sequences](docs/reports/cycle19-file-transitions/README.md)
release all 50 generations and seven descriptor/mapping epochs. Logical rollback
leaves the mapped header length unchanged, so down/up reloads appended bytes.
Locked begin retries while direct write succeeds. These bounded observations
preserve the native behavior without claiming production reachability or general
transaction/lock guarantees.

[Three sync-failure sequences](docs/reports/cycle19-file-sync-failures/README.md)
release 30 more generations and four OS epochs. Failed commit preserves the
transaction and retry clears it. Down continues cleanup after failed sync;
commit while unmapped succeeds without syncing. These fixed injected observations
do not establish general OS failure behavior or durability.

[Four failed-down sequences](docs/reports/cycle19-file-unmap-failures/README.md)
release 40 more generations and seven OS epochs. Failed unmap retains the
original mapping, descriptor, up ring and counter; preceding successful sync
remains visible. Retry or parent teardown then releases that original epoch.
[Four descriptor-failure sequences](docs/reports/cycle19-file-descriptor-failures/README.md)
release 40 further generations and acquire five OS epochs. Failed close moves
the chunk down but retains its descriptor; down/up retries reject. A second
suppressed close leaves that descriptor live after parent release, with harness
reclamation recorded separately. Failed reopen creates no epoch; a successful
retry creates a fresh one. General reopen failures, production caller provenance,
aggregate custody and full event composition remain open.

**Active (Cycle 20).** Extraction of library-specific verification into a
[versioned verifier-plugin ecosystem](docs/cycles/CYCLE_20.md): a
length-delimited JSON protocol, manifests, a fail-closed registry, bounded
process providers, and a certificate firewall. Protocol v1 carries only
Boolean assertions, so the core currently refuses all v1 `Safe` proposals
and treats unchecked `Unsafe` proposals as `Unknown`; source-level
`verify scope` declarations now derive obligations and dispatch providers.
The [bounded evidence-v2 increment](docs/reports/w3-evidence-v2/README.md)
replays observation predicates against pinned contracts and records agreement
for supplied runs under an explicit observation-trust assumption. A standalone
provider builds against `qcore-schemas` alone. Source lifecycle effects remain
unimported, empty inventories remain unverified, and first-party providers
remain unpromoted. The [generic verifier host](docs/reports/w3-verifier-host/README.md)
is extracted into `qcore-verifier-host`, buildable without provider crates;
production profile import and remaining kernel consolidation are still open.

**Trust and audits.** An independent
[reevaluation](docs/REEVALUATION_SPEC.md) (2026-09-05) reproduced six
certificate/weak-memory discrepancies, followed by a
[seven-finding follow-up](docs/REEVALUATION_FOLLOWUP.md); all thirteen are
repaired, with the remaining boundaries (production semantic-witness imports, OS-level
provider sandboxing, and the custody modes W4 still does not infer) documented
rather than papered over.

**Direction.** The [generalization plan](docs/GENERALIZATION_PLAN.md)
reviews the architecture and orders the work to (a) make the checker generic
over source sets — contracts and profiles as data, core-derived obligations,
interprocedural summary inference — and (b) decompose the monolithic crate
into a pluggable contract/verification system with a versioned schema
surface and out-of-process providers. W0 is executed
([record](docs/reports/w0-schema-crate/README.md)): the schema surface and
the CLI are standalone crates, with a test guarding the downward-only
dependency rule; providers stay compile-time crates for now (decision
recorded in the W0 report). The W1 relocation is executed
([record](docs/reports/w1-contract-crate/README.md)): the contract loader
and observation interpreter live in `crates/qcore-contract`, above the new
`qcore-core` kernel seed and `qcore-frontend` front-end crates. The W1
literal-site migration is executed
([record](docs/reports/w1-literal-sites/README.md)): the remaining Rust
contract-table literals in tests and examples now load `.qcore` fixture
modules through the loader. W1 is executed
([record](docs/reports/w1-clause-vocabulary/README.md)): the clause-family
boundary between the loader and the observation/lifecycle interpreters is
documented and pinned corpus-wide, and the C ABI scalar table is complete;
unifying the interpreters is a W5-adjacent item and error sentinels need
target-interface work. The
W6 CT conformance harness is stood up
([record](docs/reports/w6-ct-harness/README.md)): `compiler/tests/ct_corpus.rs`
executes the manifest-mapped spec §55 cases through the real source pipeline;
CT02/CT12/CT16 are mapped as sequential-core fixtures. Batch 2
([record](docs/reports/w6-ct-batch-2/README.md)) adds CT19 and CT37 and
records the full triage of all 47 then-unmapped cases, so 45 of 50 cases
remain unmapped, counted by a pinned test. W4 is
executed for parameter/return custody with validation
([record](docs/reports/w4-custody-inference/README.md)): the C gateway now
infers each internal function's parameter and return custody summary bottom-up
over the call graph and publishes it only after re-running the intraprocedural
analyzer against it, so a wrapper around a contracted API is analyzed instead
of being `Unknown`. Declared contracts always take precedence, a disagreement
is reported rather than resolved silently, and the safety certificate's recheck
re-derives every summary independently. The [bounded W4 extension](docs/reports/w4-custody-extensions/README.md)
adds direct out-parameters, stores of formal resources into formal owners,
reallocation wrappers and possibly-null owned returns. Recursive functions,
aggregate removal and unsupported out-slot aliases keep their callers `Unknown`.
The [gateway relocation](docs/reports/w4-gateway-crate/README.md) extracts this
analysis, Clang discovery and typed-AST transport into `qcore-c-gateway`, retaining
the compiler API and exact 185-unit diagnostic/verdict/projection parity.

Further reading: [module modes and index syntax](docs/QCORE_MODULE_MODES.md),
[QTT alignment](docs/QTT_ALIGNMENT.md),
[post-Cycle-9 roadmap](docs/ROADMAP_AFTER_CYCLE_9.md).

## Build and test

The Rust compiler and `qcorec check` / `qcorec emit` do not require Clang.
Compiling emitted C requires a C11 compiler (MSVC on Windows via the Rust
MSVC toolchain, a native compiler elsewhere). The full test suite also
exercises existing-C analysis, which needs Clang for preprocessing, syntax
validation, and its JSON AST; `cl.exe` cannot replace that dependency.

```sh
git submodule update --init --recursive
cargo test --workspace
cd proof
lake build
```

The Rust suite verifies that `third_party/libevent` is at the audited
revision and still contains the source constructs the Cycle 18 model relies
on; CI initializes the submodule before this gate. The Lean toolchain is
selected through `proof/lean-toolchain` (install a Lean 4 stable toolchain
via elan first).

### Windows

Install Rust's MSVC toolchain and Visual Studio or Build Tools with
**Desktop development with C++** (MSVC and the Windows SDK). Use an **x64
Native Tools Command Prompt** or **Developer PowerShell** when compiling
native C so the SDK, linker, and runtime DLLs are available. Generated-C
tests discover MSVC through the `cc` crate; no `cc` or `clang` command on
PATH is required.

For existing-C analysis and the full test suite, also install Visual
Studio's optional **C++ Clang tools for Windows** or a standalone LLVM
installation (see
[Microsoft's Clang setup instructions](https://learn.microsoft.com/en-us/cpp/build/clang-support-msbuild)).
QCore locates the analysis compiler in this order:

1. `QCORE_CLANG`, when set, names a single executable (no extra arguments).
2. `clang` on PATH.
3. On Windows, LLVM under `VSINSTALLDIR` and Visual Studio installations
   found via `vswhere` (including Build Tools). Both `clang.exe` and
   `clang-cl.exe` work, using Clang's GCC driver mode for analysis.

An explicit path works without changing PATH, including paths with spaces:

```powershell
$env:QCORE_CLANG = 'C:\Program Files\LLVM\bin\clang.exe'
cargo test --workspace
```

An invalid or empty `QCORE_CLANG` is an error; it never silently selects
another compiler. If no Clang is found, existing-C analysis reports an
unavailable parser with installation guidance and does not issue a safety
certificate; ordinary QCore checking and C emission remain available:

```powershell
cargo build --bin qcorec
.\target\debug\qcorec.exe check examples\cycle12.qc
.\target\debug\qcorec.exe emit -o generated.c examples\cycle12.qc
```

Windows CI runs a core MSVC job (Clang discovery disabled; checking,
emission, native generated-C execution) and an MSVC-plus-Clang analysis job
(full suite including MSVC AddressSanitizer; Clang removed from PATH and
`VSINSTALLDIR` cleared to exercise `vswhere` discovery). Linux CI runs the
full suite.

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
only inside their bodies; nested concurrency lowering still rejects with Q500.
This closes the two W6 batch-2 frontend defects without claiming contract or
concurrency semantics are implemented. CT mapping remains 5/50.

W2 H6 provider metadata discovery is implemented (2026-09-07;
`docs/reports/w2-provider-discovery/README.md`). First-party manifest literals
are removed after field-for-field parity; registry metadata is loaded atomically
from profile packages with stable plugin-id ordering and no compiled fallback.
All shipped providers remain v1 built-ins returning Unknown; process extraction
is still W5 work. Full 185-TU diagnostic/source parity is unchanged.

The [Lean CI hygiene](docs/reports/w7-lean-ci/README.md) adds the pinned Lean
build and every theorem audit to CI, including a compiled QCore axiom check.
Both reevaluation probes also run in Linux and Clang-enabled Windows CI.
Local checks passed; the hosted workflow has not yet run.

[Generic Lean naming](docs/reports/w7-generic-lean-names/README.md) makes the
descriptor-custody model, event composition and busy/lock preservation theorem
available under library-independent names while keeping historical aliases.

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
