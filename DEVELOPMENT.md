# QCore-C development notes

This file holds the implementation and status detail that used to live in
`README.md`: cycle records, workstream and evidence narratives, corpus and
audit specifics, the repository layout, and the full build/validation
instructions. [`README.md`](README.md) is the short human summary and
[`AGENTS.md`](AGENTS.md) is the compressed ledger for AI agents.

## Specifications and records

- Current specification: [QCore revision 0.3](docs/QCORE_SPEC_r0_3.md), with
  [contract semantics](docs/QCORE_CONTRACT_SPEC_r0_3.md),
  [implemented grammars](docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md), and the
  [implementation acceptance ledger](docs/reviews/r0_3/REVIEW.md).
  Full language conformance is not claimed; revision 0.2 remains historical.
- [Persistent Cycle 19 build inputs](docs/CYCLE19_BUILD_INPUTS.md) replaces
  ephemeral `/tmp` build paths for new audits.
- [Emacs syntax highlighting](editors/emacs/README.md) covers current QCore
  source and contract syntax with the standalone `qcore-mode`.
- Working with the repo as an AI agent? Start at [`AGENTS.md`](AGENTS.md) —
  the compressed status ledger, invariants, and per-change gate live there.

## Repository layout

| Path | Contents |
|---|---|
| [`compiler/`](compiler/) | Rust library `qcore-compiler`: parser, checker, reference semantics, C backend, existing-C compatibility API, source driver and compatibility APIs |
| [`crates/qcore-verification/`](crates/qcore-verification/) | Trusted explicit-registry source driver, generic model/source imports, obligation derivation, witness/source-effect replay and scope dispatch over core/frontend/contract/schemas/gateway/host |
| [`crates/qcore-core/`](crates/qcore-core/) | Semantic kernel: quantitative types, ownership, errors, cleanup, CFG, concurrency, translation, library custody and chunk lifecycle models |
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
CT02/CT12/CT16 initially mapped sequential-core fixtures. Batch 2
([record](docs/reports/w6-ct-batch-2/README.md)) adds CT19 and CT37 and
records the full triage of all 47 then-unmapped cases. The later
[native task increment](docs/reports/w6-native-source-tasks/README.md) maps CT15
and upgrades CT19 to actual task source; [explicit recovery](docs/reports/w6-task-recovery/README.md)
adds CT13. [Owned-task mappings](docs/reports/w6-owned-source-tasks/README.md)
and [handler rethrow](docs/reports/w6-handler-rethrow/README.md) now leave
39 of 50 unmapped after the [reused-address mapping](docs/reports/w6-reused-address/README.md). [Worker allocation](docs/reports/w6-worker-allocation/README.md)
upgrades CT16 to real child allocation/error cleanup and retains its sequential
implicit-cleanup control. W4 is
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
python3 scripts/check_lean_audits.py
```

The Rust suite verifies that `third_party/libevent` is at the audited
revision and still contains the source constructs the Cycle 18 model relies
on; CI initializes the submodule before this gate. The Lean toolchain is
selected through `proof/lean-toolchain` (install elan first). The audit runner
starts Lake inside `proof`, builds incrementally, then runs every `Audit*.lean`
with warnings treated as failures. It reports elapsed time every 15 seconds,
limits the build to 600 seconds and each audit to 120 seconds, and stops the
active process tree on timeout or Ctrl-C. Limits can be set with
`--build-timeout`, `--audit-timeout`, and `--heartbeat`; `--verbose` shows Lake
build commands. Use `--skip-build` only after a successful build of the current
sources.

For a build without audits, use `(cd proof && lake build)`. From the repository
root, `lake -d proof ...` can make elan query the default `stable` release before
Lake applies `-d`; selecting the working directory first uses the pinned
toolchain directly. `QCore.lean` only imports the proof modules: checking that
file alone does not run the axiom audits. Keep `proof/.lake` for incremental
reuse. To profile the trace relation independently:

```sh
(cd proof && lake env lean -Dprofiler=true QCore/TaskTraceRelation.lean)
```

### Windows

Install Rust's MSVC toolchain and Visual Studio or Build Tools with
**Desktop development with C++** (MSVC and the Windows SDK). Use an **x64
Native Tools Command Prompt** or **Developer PowerShell** when compiling
native C so the SDK, linker, and runtime DLLs are available. Generated-C
tests discover MSVC through the `cc` crate; no `cc` or `clang` command on
PATH is required.

Scoped task and checked mutex emission embeds the required C11 thread subset
on Windows, adapted from Monkey winpthreads. In the developer shell:

```powershell
cargo run -p qcorec -- emit examples/threads/two_workers.qcore -o generated.c
cl /nologo /std:c11 /W4 /WX generated.c /Fe:generated.exe
.\generated.exe
```

The [threading examples](examples/threads/README.md) cover copied inputs,
owned-buffer transfer and shared readers. Build and run all three with
`.\examples\threads\build-windows.ps1 -Run`; the script puts C, object and
executable files in `target\thread-examples\`. The MSVC core CI job runs the
same script with Clang discovery disabled.

No extra header, object file or pthread library is needed. The native test
harness uses MSVC's `/experimental:c11atomics` for its synchronization observers;
the emitted task runtime itself does not require that switch. The supported
surface is joinable creation/join, equality/yield and plain non-recursive
mutexes, not the full C11 header. See
[the Windows thread report](docs/reports/windows-c11-threads/README.md).

For existing-C analysis and the full test suite, also install Visual
Studio's optional **C++ Clang tools for Windows** or a standalone LLVM
installation (see
[Microsoft's Clang setup instructions](https://learn.microsoft.com/en-us/cpp/build/clang-support-msbuild)).
QCore locates the analysis compiler in this order:

1. `QCORE_CLANG`, when set, names a single executable (no extra arguments).
2. `clang` on PATH.
3. On Windows, LLVM under `VSINSTALLDIR`, then Visual Studio installations
   found via `vswhere`, newest first (including Build Tools and previews).
4. Standard Visual Studio 2026 (`Microsoft Visual Studio\18`) and 2022
   (`Microsoft Visual Studio\2022`) edition directories under Program Files,
   including when `vswhere` is missing or returns no installations.

Both `clang.exe` and `clang-cl.exe` work, using Clang's GCC driver mode for
analysis. For example, VS 2026 Community installs its x64 compiler at
`C:\Program Files\Microsoft Visual Studio\18\Community\VC\Tools\Llvm\x64\bin\clang.exe`.
Discovery tries subsequent installations when an earlier one lacks Clang.

The Windows suite also runs syntax-only POSIX LP64 cleanup fixtures in isolated
child tests, forwarding to the discovered Clang with an explicit Linux target.
These exercise the existing POSIX models, not native Windows cleanup behavior;
ordinary Windows analysis keeps its native target.
Local VS 2026 validation passed 828 tests, formatting, Clippy and both
reevaluation probes; Lean was unavailable. See the
[Windows validation report](docs/reports/windows-vs2026/README.md).

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

## Increment records

### Quant delimiter alias `[| |]` (2026-09-09)

`quant[| ... |]` is now an accepted delimiter alias for `quant< ... >` on
closed quantitative index types, over the byte-identical interior grammar.
`[|` and `|]` are new single lexer symbols occupying previously rejected
syntax space (a bare `|` was and remains a lexer error, so no existing
program changes meaning); the opener fixes the required closer, and a mixed
pair such as `quant[| ... >` is refused, never repaired. Both spellings
resolve to the same `SourceQuantIndexType`/`ResolvedQuantIndexType`, source
spellings round-trip verbatim, and the `quant<access ...>` exclusion applies
symmetrically to both openers. The alias exists because the angle form's
closing `>` sits beside where-clause comparison operators; it adds no type,
no diagnostic code and no semantic surface, so no Lean change is required
(the resolver and everything downstream are untouched). Five new tests pin
lexing beside plain brackets, identical elaboration of both spellings, the
bracket twin of the pinned `quantified_indices.qcore` fixture in parameter,
return and contract-clause position (the corpus file itself is untouched),
and mixed/unterminated-delimiter refusals. The implemented-grammars G04
entry, spec prose and the Emacs mode's `|` punctuation class are updated in
the same change. Validation: 1752 workspace tests passed (5 new),
3 ignored, 0 failed; formatting, Clippy, both reevaluation probes, all
42 Lean audits and the 9 Emacs ERT checks pass.

### Sequential C emission indentation (2026-09-08)

`qcore_core::c_emit::emit_c11`'s runtime helper bodies (`q_trace`,
`q_acquire`, `q_release`) and `main`'s result/trace report block were written
with Rust string-literal line continuations, which strip the following line's
leading whitespace, so those regions of every emitted sequential translation
unit landed flush left with visually mis-nested braces. The literals now carry
the intended four-space-per-level indentation; emitted bytes change only in
leading whitespace, the token stream is identical, and a re-emitted unit
compiles under `-std=c11 -Wall -Wextra -Werror` with byte-identical program
output. A new unit test pins the repaired lines and refuses any column-zero
statement outside preprocessor lines, file-scope declarations, braces and goto
labels. The task, mutex and extern emitters already placed indentation before
their continuations and are untouched, so the two pinned F02 emission digests
do not move (re-emission of all four thread examples verified byte-identical).
This is a formatting repair of generated C, not a semantic, verdict or
coverage change. Validation: 1652 workspace tests passed (one new), 1 ignored;
formatting, Clippy and both reevaluation probes pass; the Lean gate could not
run on this host because `lake` is not installed, and no Lean-relevant
semantics changed.

### Core concurrency prelude (2026-09-08)

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

Windows second-source-set fixture (2026-09-08): compile databases now name the
gateway-selected Clang and omit Unix-only build flags on Windows. All five
tests pass through Visual Studio discovery and explicit `QCORE_CLANG`; CI also
runs this target with Clang removed from PATH. Failed audit assertions include
status, stdout and stderr. Production checker and proof semantics are unchanged.
Formatting, workspace Clippy, both reevaluation probes and the pinned Lean build
pass. Workspace validation still fails in the untouched `o04_constructor_caller`
and `counter_source` targets.

Windows executable examples (2026-09-08): `examples/threads/` adds two copied
workers, an owned-buffer round trip with independent parent work, and two
shared readers followed by an exclusive parent write. `build-windows.ps1`
emits C and builds executables under `target/thread-examples/`; `-Run` executes
them. All three run under MSVC with `/std:c11 /W4 /WX` and an unavailable
`QCORE_CLANG`, producing main results 7, 42 and 7 respectively. The Windows
MSVC core CI job runs the same script. The guide explains developer-shell
setup, compile/link commands, process exit codes and the buffered trace output.
Compiler, runtime and proof semantics are unchanged. Formatting, workspace
Clippy, both reevaluation probes, the pinned Lean build and all 31 audits pass.
The aggregate Windows workspace run records 1,306 passed, ten existing fixture
failures and one ignored test. The failures are two in `second_source_set`,
seven in `o04_constructor_caller` and one in `counter_source`. These failures
are outside the example changes; the run started before the concurrent
audit-fixture repair was finished.

Lean build performance (2026-09-08): TaskTraceRelation's finite-domain proofs
expand D3's generated inputs once before kernel evaluation; theorem statements,
domains, digests and allowed axioms are unchanged. The affected-module rebuild
passes in 47.5 seconds locally; the next incremental build takes 0.13 seconds.
`python3 scripts/check_lean_audits.py` now builds from the pinned proof directory,
reports progress, enforces finite time limits and cleans up workers on timeout
or cancellation. All 31 audits, eight runner tests and the standing gates pass.
Evidence: `docs/reports/lean-build-performance/README.md`.

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


The dated entries below are archived in the order they landed. Each one keeps
its own checkpoint, counts and boundaries; a later entry does not supersede an
earlier one unless it says so.

### Reopen failure evidence (2026-09-07)

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
tasks, typed owned-error payload tasks and checked mutex guards now check, emit,
compile and execute on macOS through an emitted pthread-backed implementation
of the C11 thread subset used by QCore. The macOS CT harness executes the
threaded native and fault-injection cases; CT50's separate
`MAP_FIXED_NOREPLACE` address-reuse observation remains Linux-only. Apple
ASan/UBSan runs remain enabled, while leak detection stays a Linux-only
observation because Apple's AddressSanitizer does not provide LSan. Configured
audit compile inputs compare canonical existing paths so the macOS `/var` to
`/private/var` alias cannot spuriously invalidate a pinned task, and archived
Linux trace replay pins Linux errno values instead of reading the host Python
errno table.

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

macOS Cycle19 compiler-option transport (2026-09-08): the configured compile
database gateway now retains Apple Clang `-Wundef-prefix=<prefix>` and the
two-word `-arch <architecture>` as ordered typed frontend inputs, forwarding
them through preprocessing, syntax checking and typed-AST import. Malformed
forms, absent explicit dialects and compiler substitution remain fail-closed;
the complete command and typed target selection stay bound into the compile-task
identity. The user's prepared audit at Fluent Bit `11651d47` now completes all
181 configurations across 180 source files with 2 Safe / 41 provisional Unsafe /
134 Unknown / 4 data-only and zero gateway failures or option refusals.
Historical evidence stays Unknown and this current-revision result neither
replaces the pinned 185-unit ledger nor promotes a raw unit. Evidence:
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

Wave-11 delegated increment (2026-09-09): the four fleet-trimmed cards
relaunched plus the CT lane landed through delegated worktrees at the
seven-agent cap, merged onto main under the failure-gated chain discipline
and closed by one integrator corpus consolidation; this paragraph supersedes
nothing above and each card's record keeps its own boundaries.
- CT33 mapped (wave-11 CT lane): coverage 30/50 to 31/50 with the sharper
  one-statement accepted control (Q624 attributed to the descending edge
  alone), the trailing Q612 pinned, native_sanitized deliberately
  unassessed, 19 scratch mutations refused, and the stale-blocker
  publication made structural (blocker_current beside the frozen archive).
  Evidence: `docs/reports/wave11-ct33/README.md`.
- Epoch/task composition (T08-COMPOSE): `source.epoch.tasks.v1` places a
  flat-epoch plan before a real checked scoped-task region by blanking the
  plan's bytes — the task half is byte-for-byte what the plain route
  compiles, pinned — and requiring the derived shape to equal the plan in
  both directions at existing codes. The card's extent-derivation example is
  refuted by measurement (this backend's Buffer has no extent); disposal is
  the one derived figure; guard/rank composition is refused deliberately
  with controls. 30 new audited theorems. Evidence:
  `docs/reports/remaining-t08-compose/README.md`.
- Conditional-arm raw scan (E01-RAW-COND): the recovered wave-10 commit
  applied cleanly; a #if group's file-scope depth is the greatest depth its
  arms reach, so the scan can only hide, never invent, definitions —
  absent_from_raw_scan is now zero in every unit, stated plainly as
  mechanism agreement rather than analysis coverage, and a latent
  phantom-definition hazard in the replaced rule is closed in passing.
  24 audited theorems. Evidence:
  `docs/reports/remaining-e01-raw-cond/README.md`.
- Recursion reach (C03-REACH): reach is still 0 — now known over the true
  population, 8x the recorded one (46 components / 96 members across 21
  units, 17 of them mutual), with the dominant indirect-access blocker
  measured to be a decoy (admitting it completely admits zero components)
  and the real bottleneck the conjunction of expression-position families —
  recorded as `docs/design/W4_NEUTRAL_EXPRESSION_POSITIONS.md` with
  digits10 as the minimal validation case. The census is route-derived and
  the corpus byte-identical. Evidence:
  `docs/reports/remaining-c03-reach/README.md`.
- May-alias refinement (C02-P-ALIAS): a modeled read is unresolved, never
  Safe, when another same-tag name was released on the reaching path and
  neither discharge (body-acquired provenance; pointer-inequality branch)
  applies — formals-only in v1. Implementation surfaced a pre-existing
  scheduling sensitivity (report_leaks on intermediate iterates), bisected
  and made scheduling-neutral via a pre-pass with AbstractState
  byte-identical, the hazard recorded as C02-P-ALIAS-LEAK-ORDER rather than
  disturbed. One corpus obligation fires (rd_kafka_new); the C02-P agreement
  pin flips per its own design. 41 audited theorems. Evidence:
  `docs/reports/remaining-c02-p-alias/README.md`.
- Corpus consolidation: one integrator measurement
  (`docs/reports/wave11-corpus/`) re-pins the baseline — 2 Safe /
  42 provisional Unsafe / 137 Unknown / 4 data-only unchanged, and the only
  changed unit is rdkafka.c, carrying both gateway cards' predicted effects
  at once; all other artifacts byte-identical with no residue class
  appearing. One semantic merge conflict (C03-REACH's census against
  E01-RAW-COND's conditional-aware scan API) was caught by the
  failure-gated chain, reconciled to the analyzer's own call pattern and
  re-gated before pushing.

Nothing is promoted by this increment: raw Cycle 19 units stay 2 Safe /
42 provisional Unsafe / 137 Unknown plus four data-only units on the freshly
re-pinned wave-11 baseline, the contract corpus stays 60 modules and full
language conformance stays false. CT coverage advances to 31/50 mapped, by
executed-stage evidence only. `qcore-schemas` stays 8.0.0; `qcore-compiler`
stays 0.4.0. Merged gate at this checkpoint: 2119 workspace tests
(4 ignored), 76 provider tests, 125 Python tests, 53 Lean audits, both
reevaluation examples exit 0, fmt and clippy clean, and all five provider
packages rebuilt and verified. Follow-on accounting lives in
`docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`, updated for
this wave).

Wave-12 delegated increment (2026-09-09): seven bounded follow-on cards landed
through delegated worktrees at the seven-agent cap, merged onto main under the
failure-gated chain discipline and closed by one integrator corpus
consolidation with exact triangulated attribution; this paragraph supersedes
nothing above and each card's record keeps its own boundaries.
- Expression positions (W4-EXPR-POS): the design note's admission lands as
  three conjunctive typed families threaded so the classification has the
  contract environment, with the proved sentence that an admitted expression
  contains no pointer-valued operand. A pre-existing fail-open is repaired
  (a for-condition reached the classifier stripped of structure and answered
  Safe with no diagnostic). 28 attributed function-level Unknown-to-Safe
  discharges; digits10 passes V1-V10 end-to-end and neutral-frame recursion
  reach moves 0 to 1 of 46. 30 audited theorems. Evidence:
  `docs/reports/remaining-w4-expr-pos/README.md`.
- Endpoint identities (O04-B09B10): member-address actuals (sub-object
  disjointness load-bearing) and custody-taking bound reads land; C6 narrows
  nine refusals to ONE — the flb_trace statement plus its proved shadow — and
  a third pre-existing false-Safe (nested-call custody silently dropped)
  is repaired and fires on the real corpus. 35+18 audited theorems.
  Evidence: `docs/reports/remaining-o04-b09b10/README.md`.
- Locals may-alias (ALIAS-LOCALS): v1's formal bound was measured
  launderable and a projection false-Safe (copy, release, probe) repaired;
  the definite-alias relation refines without widening, the verdict stays
  UnknownAlias on four recorded grounds, and the census closes the prior
  card's reach item. 51 audited theorems total. Evidence:
  `docs/reports/remaining-alias-locals/README.md`.
- Fourth transport vocabulary (P04-B2): file_outcomes and file_delete move
  through the process boundary with the replay profile contract-derived,
  never wire-trusted, and the prerequisite mechanism gains its first
  production consumer. `qcore-schemas` 9.0.0; ChunkIO 0.8.0; 40 audited
  theorems. Evidence: `docs/reports/remaining-p04-b2/README.md`.
- Extern error-arm release (EXTERN-ERR-REL): the else arm renders the
  cleanup edge the core already computes (same identities, once, reverse
  order); STORE-REACH closes at four levels; "carried into the error path"
  is measured unimplementable on this route and recorded, and
  AdapterDepartingArgument is named. 19 new audited theorems. Evidence:
  `docs/reports/remaining-extern-err-rel/README.md`.
- Protection growth (CT34-GROWTH): sound under five conditions — one
  narrower than proposed (creator-only, because DestroyMutex unparks to the
  creator); Q626/Q627 via the §L21 process (band Q610-Q627); the check-only
  backend behavior converted from omission to asserted decision; CT34's
  mapping proposed with a one-token control. 18 new audited theorems.
  Evidence: `docs/reports/remaining-ct34-growth/README.md`.
- Transferability (CT46-SENDSYNC): a closed transferable(T) judgment decides
  both matrices whole, with the checked-redundant composition proven unable
  to diverge; CT46 stays honestly unmapped — the recorded blocker is retired
  and replaced by the measured CT46-MUTEX-CAPTURE (the accept half is
  unwritable). 27 audited theorems. Evidence:
  `docs/reports/remaining-ct46-sendsync/README.md`.
- Corpus consolidation: one integrator measurement
  (`docs/reports/wave12-corpus/`) re-pins the baseline with the 59-unit diff
  TRIANGULATED exactly — the measured tree differs from each gateway card's
  archived AFTER in precisely the other card's units (55 + 9 − 5 shared),
  function moves measured directly at 28 Unknown-to-Safe equal to
  EXPR-POS's list, and the headline unchanged at every checkpoint.

Nothing is promoted by this increment: raw Cycle 19 units stay 2 Safe /
42 provisional Unsafe / 137 Unknown plus four data-only units on the freshly
re-pinned wave-12 baseline, CT coverage stays 31/50 mapped (CT34's mapping
awaits its lane), the contract corpus stays 60 modules and full language
conformance stays false. `qcore-schemas` is 9.0.0; `qcore-compiler` stays
0.4.0. Merged gate at this checkpoint: 2150+ workspace tests across the
per-merge gates with the final tree at 57 Lean audits, 87 provider tests,
125 Python tests, both reevaluation examples exit 0, fmt and clippy clean,
and all five provider packages rebuilt and verified at schemas 9.0.0.
Follow-on accounting lives in `docs/reviews/r0_3/acceptance_matrix.json`
(`named_followups`, updated for this wave).

Wave-13 delegated increment (2026-09-09): six bounded follow-on cards landed
through delegated worktrees, merged onto main under the failure-gated chain
discipline and closed by one integrator corpus consolidation with exact
attribution; this paragraph supersedes nothing above and each card's record
keeps its own boundaries.
- CT34 mapped (wave-13 CT lane): coverage 31/50 to 32/50, with a new
  accept_check_only harness capability asserting the core's admission AND
  the pinned backend refusal together (strictly more assertion); CT46's
  blocker text updated to the measured CT46-MUTEX-CAPTURE; the hand-kept
  bucket staleness recorded as D02-BUCKETS-STALE. Evidence:
  `docs/reports/wave13-ct34/README.md`.
- C6 discharges (O04-TRACE): `flb_input_chunk_create` reaches function-level
  Safe on the byte-verbatim upstream body — all six exits Discharged, every
  premise named as the assumption it is — via the total-or-reject
  variadic-data-tail vocabulary (rederived from retained sources, no schema
  change) and nine-premise one-level nested composition whose load-bearing
  pair makes argument evaluation order unobservable. Two silent gaps
  repaired along the way, including the N9 nested-argument escape-exemption
  fail-open. The declarations deliberately live as gateway fixtures, so the
  production corpus is untouched and nothing is promoted; production reach
  is measured zero (O04-TRACE-REACH). Evidence:
  `docs/reports/remaining-o04-trace/README.md`.
- Recursion reach closed at the frontier (REACH-GATE): family F4 admits
  typed chains in expression positions under the refined proved sentence
  (binds, releases, escapes and addresses nothing the projection tracks),
  adding 2 attributed function discharges — and the design note's
  11-component payoff is DISPROVED: every component at that threshold needs
  what fail-closed must refuse, named per component. Reach stays 1 of 46 by
  measurement; further reach requires new model capability. Evidence:
  `docs/reports/remaining-reach-gate/README.md`.
- The plainest use-after-free refused (BORROW-LIVENESS): a single-name
  release-then-borrow — no aliasing involved — was measured Safe with no
  diagnostics at base; the liveness precondition now refuses it on settled
  denotations, with the verdict class decided first on three measured
  grounds and the must/may distinction proved both directions. LEAK-ORDER
  untouched by construction; corpus byte-identical with the census naming
  the binding conjunct. Evidence:
  `docs/reports/remaining-borrow-liveness/README.md`.
- Hash-table contract (HASH-CONTRACTS): the upstream body read; the
  borrowed-mode contract lands with the size-0 boundary enforced by a
  machine-parsed total-or-reject premise directive in the contract's own
  retained text (the deliberate HASH-CONTRACTS-CLAUSE deviation); corpus
  60 to 61; resource_stored refused on two grounds, one executed; B09B10's
  VALUE-DEPENDENT note corrected by measurement (retention, not custody, is
  the arm-dependent fact). Rows 5-8 decide on the fixture tree; production
  awaits the neighboring contract. Evidence:
  `docs/reports/remaining-hash-contracts/README.md`.
- Fifth transported vocabulary (P04-COMPOSE): the unmap_wrapper composition
  moves with no composed result or prerequisite conclusion on the wire; the
  core re-derives and re-evaluates all three prerequisites itself under the
  E03-B premise union, and verify_v2 gains its missing QP001 binding for
  the new projection. Census: file_close is composable-now because this
  landed (P04-COMPOSE-A); the frame helper's lack of a verification task is
  the structural blocker (P04-COMPOSE-FRAME). `qcore-schemas` 10.0.0;
  host/verification 0.4.0; ChunkIO 0.9.0. Evidence:
  `docs/reports/remaining-p04-compose/README.md`.
- Corpus consolidation: one integrator measurement
  (`docs/reports/wave13-corpus/`) re-pins the baseline — 4 changed units
  and 2 function discharges, exactly REACH-GATE's attributed set with
  HASH-CONTRACTS' single premise leaf riding inside one unit's counts, the
  3 evidence rows the schemas-10.0.0 package residue, and the byte-identity
  claims of the other three cards all holding on the composed tree.

Nothing is promoted by this increment: raw Cycle 19 units stay 2 Safe /
42 provisional Unsafe / 137 Unknown plus four data-only units on the freshly
re-pinned wave-13 baseline, the contract corpus is 61 modules by
HASH-CONTRACTS' deliberate pin update and full language conformance stays
false. CT coverage advances to 32/50 mapped, by executed-stage evidence
only. `qcore-schemas` is 10.0.0; `qcore-compiler` stays 0.4.0;
`qcore-verifier-host` and `qcore-verification` are 0.4.0. Merged gate at
this checkpoint: 2343+ workspace tests across the per-merge gates with the
final tree at 62 Lean audits, 94 provider tests, 125 Python tests, both
reevaluation examples exit 0, fmt and clippy clean, and all five provider
packages rebuilt and verified at schemas 10.0.0. Follow-on accounting lives
in `docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`, updated
for this wave).
