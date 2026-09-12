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
cargo build --bin quirin
.\target\debug\quirin.exe check examples\cycle12.qcore
.\target\debug\quirin.exe emit -o generated.c examples\cycle12.qcore
```

Windows CI runs a core MSVC job (Clang discovery disabled; checking,
emission, native generated-C execution) and an MSVC-plus-Clang analysis job
(full suite including MSVC AddressSanitizer; Clang removed from PATH and
`VSINSTALLDIR` cleared to exercise `vswhere` discovery). Linux CI runs the
full suite.

## Increment records

Wave 29, card `SLICE-PERMISSIONS-WRITE-LEND` (2026-09-12): the write half of
CT07 the read card named — `let NAME = lend(BUFFER, LOW, HIGH);` on
`source.buffer.v1`, a loan a store can name, so **two parties mutate two
disjoint ranges of one buffer independently** while both loans are live.
Accesses through a loan are decided against the loan's own `[0, HIGH-LOW)` and
rebased into the owner's coordinates; the window runs from the lend to the last
statement naming the loan. The custody shape is the card's head: residual
`STD-BUFFER-SLICE`'s footprint fraction now exists — the cycle-3 ownership IR
gains a `LendRange` borrow node carrying a half-open byte interval plus
`WriteRange`/`ReadRange` footprint-stating accesses, and the **IR itself**
decides the exclusivity rule (a second lend admitted exactly when disjoint,
`OverlappingLends` otherwise; empty lends conflict with nothing; exclusivity is
temporal), the range-wise transfer (owner reads and writes refused inside a
live lend's range, admitted outside every one; an unresolvable index conflicts
with everything, `WRITE-LEND-COMPUTED-INDEX`), the rebasing containment
(`AccessOutsideLendRange`) and the release arm, which is the existing
`DropWhileBorrowed` unchanged — every one arriving at the route through the
inherited `Q054`. A module with no lend builds the custody operation list it
always built, by a structural branch. Zero new codes (`Q051`/`Q052`/`Q053`/
`Q054`/`Q055` and the composed route's `Q077` condition widenings, stated in
§L21) and zero reserved words (`lend` is contextual on the
`len`/`concat`/`borrow_utf8` precedent, censused; `let lend = 5;` is still a
program). Fail-closed boundaries, each a named residual: a `for` beside a lend
(`SLICE-PERMISSIONS-LOOP-WINDOW`), crossed windows — borrow nodes are lexical —
(`SLICE-PERMISSIONS-CROSSED-WINDOWS`), an acquisition inside a window
(`WRITE-LEND-NESTED-ACQUIRE`), a lend bound outside the lowering-visible tier
(`WRITE-LEND-BOUND-FROM-BYTE`), re-lending (`WRITE-LEND-RELEND`), and a lend
beside `borrow_utf8` on the composed string route
(`SLICE-PERMISSIONS-COMPOSED-LEND`). 31 end-to-end tests in
`compiler/tests/write_lend.rs` (one sanitized native run — ASan is
load-bearing: a displaced-pointer write is the construct's worst failure mode —
Linux arm only, `WRITE-LEND-WINDOWS-RUN`), 7 IR-level unit rows,
`QCore.WriteLend` + `AuditWriteLend.lean` (25 declarations audited by name,
115 → **116** Lean audits; exclusivity is the read card's own set-theoretic
`overlaps` imported, every byte has exactly one write authority, disjoint
lends' rebased accesses cannot collide, a confined window preserves every
unlent byte). Two fixture programs a CT lane can point at
(`disjoint_write_halves.qcore`, `overlapping_lends.qcore`); the lane statement,
the proposed 15h amendment and the boundaries are in
`docs/reports/slice-permissions-write-lend/README.md`. Corpus-neutral by
construction: no gateway, contract, audit or provider file in the diff, leak
pin 266 untouched, nothing promoted.

Wave 28 close (2026-09-12): **three cards** merged one at a time under the
failure-gated chain — `EXAMPLES-AGGREGATES`, `REGION-CREDIT-JOINS` and
`CT-WIRE-LANE-2` — reconciled at integration tip `71574edc`. The wave's head is
**CT coverage**: 36/50 → **41/50 mapped** (CT07, CT08, CT09, CT21, CT23), 9
unmapped, sanitized-native 5 of 36 → **7 of 41 by real execution**, proof 0 of
41; a grown denominator is mapping, not conformance (constraint 23). CT22 is
**expressible but deliberately not mapped**: `REGION-CREDIT-JOINS` composes the
quant open onto the epoch task route after every join (three unedited checkers,
a duty-free pack, the region erasing from emission, `AbandonCredit` publishing
the parent-local unused credit), the lane publishes the **narrowed** reason in
its own record, and `CT22-CT-MAPPING` is the named follow-on — a capability is
not a mapping. `examples/records/` lands as the **twelfth** published family
under the suite's directory-scan standard. Zero reserved words, zero new
diagnostic codes (`Q302`/`Q600` condition widenings, §L21); standing constraint
**15m** records the composed region's two premises. **No arbiter ran** — no
gateway, contract, audit or provider file was touched — so the corpus rows are
carried from the wave-27 `gateway-c-location` baseline, every figure unchanged
(leak 266). Lean audits 114 → **115** (`+AuditRegionCreditJoins`). Merged gate
at the close tip: 4,392 workspace tests (8 ignored, `--no-fail-fast`), 122
provider (carried), 193 Python, both probes exit 0, fmt/clippy clean
(`docs/reports/wave28-close/gate.json`); two constraint-16 load rows tripped
under concurrent suites during integration and were re-verified green in
isolation, both facts recorded. The acceptance matrix's derived rows are
regenerated through `build_matrix.py --gate` (hand-kept sections carried, the
derived-fact tripwire green again). Nothing is promoted; full language
conformance stays **false**.

Wave 28, card `CT-WIRE-LANE-2` (2026-09-12): the second CT mapping lane, on the
wave-26 precedent, converting the wave-27 capability cards into manifest rows.
CT coverage moves **36/50 → 41/50 mapped** (unmapped 14 → 9), harness-gated
sanitized-native evidence **5 of 36 → 7 of 41**, proof **0 of 41** — by
**mapping, not conformance** (constraint 23), and with nothing semantic
changed: `git diff` over `crates/` and `compiler/src/` is empty, no diagnostic
code is allocated (the `Q-NAMESPACE-DECISION` ice box needed nothing) and no
word is reserved. Five cases map. **CT07** and **CT09** are the sequential
route's first *accepted* cases, under a new `sequential_sanitized` verdict mode
whose runner builds one binary from the case's own emitted C under
ASan/UBSan (LSan on Linux) and compares both streams byte for byte with the
route's checked observation: CT07 is `CT07-SLICE-PERMISSIONS`'s
disjoint-halves program, mapped with the boundary that card's §8 required
stated — one permission is a **read** loan and the other is the owner's
retained write authority, not two write lends
(`SLICE-PERMISSIONS-WRITE-LEND` stays open) — and CT09 is `AGGREGATES-V1`'s
three-field record program, its access proof checked and its **layout** leg an
executed sanitized byte-comparison *observation*, never a proof
(`AGGREGATE-CT09-LAYOUT` honoured; `proof` stays `unassessed`). **CT08** maps
on the Reject arm of its expected text (the overlap decided at check time,
`["Q078"]` naming the footprint; the analyzer-`unknown` arm is not claimed) and
**CT21** on `QUANT-REGION-SOURCE`'s undischarged cleanup pack (`["Q305"]`, the
discharge control admitted, the unopened control refused `["Q306"]`), retiring
the archived *"no source path emits `RegionOp::QuantOpen`"* reason. **CT23**
lands on a new `composed_epoch` route — grown as a new route rather than by
widening the check-only `epoch` route, so that route's structural-absence
statements stay true — under the `epoch_credit_split` verdict mode
`CT23-CT-MAPPING` proposed: the exact `["Q635"]` cascade of `error(close);`
abandoning a child's granted credit on the creator's own error edge, plus every
admitting control's published split compared wholesale (`restored + spent =
credit`, including `partial.qcore`'s real componentwise division) and the
`["Q636"]` mint refusal; the parked fixtures moved from
`compiler/tests/fixtures/epoch_credit/CT23/` into the CT tree with the card's
own test repointed, and `ct_corpus.rs`'s pinned tables grown deliberately.
**CT22 is deliberately not mapped**: the credit half is re-measured to accept
(`AbandonCredit(3)`, explicit) and the narrowed *"after joins"* reason is
published as this lane's own labelled row per `QUANT-REGION-SOURCE` §5
(`REGION-CREDIT-WITH-JOINS`), without coordinating with the parallel card that
owns the composition. The wave-26 lane's CT07/CT08 rows are retired by
declaration in this lane's record — the earlier record keeps its bytes
(constraint 21) — and two Python tests that pinned tree facts are updated with
no assertion weakened. Checkpoint at the card tip: 4,376 workspace tests
(8 ignored; the count is unchanged from the wave-27 close because the five new
cases run inside `ct_corpus.rs`'s existing test functions — 4,375 passed in
the single `--no-fail-fast` run and the one that did not,
`vp04_vp07_vp08_process_lifecycle_and_payload_budgets` under concurrent suite
load, is a recorded constraint-16 row verified passing in isolation), 193
Python, **114 Lean audits** (unchanged — the lane adds no Lean), both
reevaluation examples exit 0, fmt/clippy clean, provider 122 carried (no
provider file touched). Nothing is promoted: the corpus baseline
stays `docs/reports/gateway-c-location/artifacts/` with leak **266**, the
contract corpus 73 modules, full language conformance **false**. Evidence:
`docs/reports/ct-wire-lane-2/README.md` and its `ct_stage_evidence.json`.

REGION-CREDIT-JOINS (2026-09-12, wave 28): the composition gap two wave-27
reports named from opposite sides — `CT23-T07-SEAM` and
`REGION-CREDIT-WITH-JOINS` — closed as the minimal honest composition, making
CT22's scenario ("quant opening discards parent-local unused credit after
joins", expected Accept) expressible end to end for the first time. A
`source.epoch.tasks.v1` entry may now write, after every spawn and join of its
task scope, one `error region … catch (E) quant open … { discharge …; }` whose
body is narrowed to the parent's own quantities (`credit N;`, `effort N;`, the
decided `qcore_check(false, E);` edge — a region acquisition is refused by
name, `REGION-JOINS-DUTIES`): the sealed pack carries no duty and exactly the
parent's unused credit, which the discharge abandons as the kernel's own
explicit `AbandonCredit`, decided by the **unedited**
`qcore_core::error_region::analyze_error_regions` at its own `Q303`–`Q309`
codes (constraint 15l reached from a second route). "After joins" is derived
from the checked task IR — a spawn or join span following the region refuses at
`Q600` — and the published `abandoned_credit` is cross-checked against an
independent fold before publication. The region is blanked from the task half
exactly as the plan is, so it erases from emission: emitted C, source map and
reference observation measured byte-identical with and without it, and every
pre-existing composed module publishes `region: None` with unchanged
certificates. Zero new codes (two condition widenings stated in §L21: `Q302`'s
shape conditions from this route, `Q600`'s after-joins reconciliation), zero
new reserved words (census vacuous — every word already reserved, the shape
already parsed), and `epoch_accounting`, `epoch_credit_recovery` and
`region_task_credit` all untouched (constraint 15j honored; the declarative
route keeps its refusal). 13 new Rust tests
(`compiler/tests/region_credit_joins.rs` 9, frontend 4), eight fixtures parked
at `compiler/tests/fixtures/epoch_credit/CT22/` outside the CT corpus, and 14
audited Lean theorems in the new `QCore.RegionCreditJoins`
(`AuditRegionCreditJoins.lean`, audits 114 → **115**, no project axiom). CT
manifest untouched — CT22 mapping is the wire lane's; CT25 stays inexpressible
with its gap narrowed to `REGION-JOINS-DUTIES`. Checkpoint and boundaries:
`docs/reports/region-credit-joins/README.md`.

The published records family (EXAMPLES-AGGREGATES, wave 28): the follow-on
card `AGGREGATES-V1` named is discharged — `examples/records/` publishes the
first aggregate type as a family held to `EXAMPLES-SUITE`'s standard, growing
the published family count **11 → 12** with **no crate-source change, no
reserved word and no allocated code**. Two programs, chosen to be each other's
leak control: `point.qcore` is CT09's "distinct ordinary struct fields"
scenario as a published example — three fields, three different values, three
reads on stdout plus two on the logger's stream, record-only, so its emitted C
carries **no buffer prelude** (constraint 15k's conditional-prelude fact,
visible in a published program for the first time) and its sanitized run is a
*negative* allocation control — while `pixel_writes.qcore` puts records beside
a real resource: `Rgb` fields drive three stored bytes, a `Write` record
drives the fourth store's **index and value** both out of field reads, the
buffer owes its `drop` (removing it still refuses at `Q054` — the records
launder nothing) and the four bytes printed back are `QCor`, the loops
family's own payload. Both programs are spelled to the index's rule — every
once-read scalar is piped into its sink, constructions and stores stay
statements as the assembly half — and the page's transcripts, its emitted
struct block and its pinned `Q051` incomplete-construction refusal (span caret
included) are read out of the file by the suite rather than restated.
`compiler/tests/examples_suite.rs` reaches the new directory structurally by
its scan and gains four records-owned tests (38 → 42 in that file): the native
runs, the emitted-C claims (declared-order initialization, the three binding
prefixes' contrast, no `qcore_bytes_new` in the record-only program), the
documented refusal compared with the compiler's real diagnostic, and the four
side-table refusals (`Q052` drop, `Q052` undeclared field, `Q013` undeclared
type, `Q054` lost release) each earned by exactly the statement the table
writes. The six `AGGREGATE-SELECTOR-WORDS` route-selector words are avoided in
every published type and field name and the page says so. The family README
publishes no `quirin` command line, deliberately: `examples_cli.rs` pins the
published check/emit command-line counts at 6/6, and the loops precedent is
`cargo test` lines only. Evidence:
`docs/reports/examples-aggregates/README.md`.

Worktree submodule diagnostic (2026-09-12, post-wave-27 close, `1e1916b4`
merged at `e552e59c`): the two checkout-reading `profile_loader` tests
(`declared_pins_match_the_checkouts_and_the_retained_rust_constant`,
`f9_and_f10_missing_files_and_digest_drift_reject_the_inventory`) gain a shared
precondition requiring the Libevent submodule's own `.git` entry before any git
query runs. In a fresh worktree the submodule directory exists but is empty and
`git -C <dir> rev-parse HEAD` answers from the *superproject*, so the pin check
reported a `SourceRevisionMismatch` against whatever commit the repository was
on — a misleading invariant-5 alarm for a checkout that was merely
uninitialised. The tests still **fail** in that state (the repository's
convention; nothing skips), now naming the uninitialised submodule and the fix
command; both directions were measured (empty directory swapped in → the new
message; initialised checkout → all 18 tests pass), the pin assertion itself is
unedited, and `f7_and_f8_pin_failures_reject_the_checkout` keeps the
`SourceRevisionMismatch` arm exactly as it was. **DECIDES — discharges**
residual `DOCS-COMPACT-3-WORKTREE-SUBMODULE`
(`docs/reports/docs-compact-3/README.md` §11.1, recorded there frozen; the
discharge is recorded here per constraint 21). The same change also corrects
the two stale `CLAUDE.md` attributions of constraint 4's assume-guarantee rule
in `docs/design/W4_RECURSIVE_SUMMARY_PROFILES.md` to `AGENTS.md` (residual
`DOCS-COMPACT-2-W4-DESIGN-POINTER`, likewise **discharged**). Gate at the fix's
own tip (pre-close base `27100414`): 4,349 workspace tests, 8 ignored,
`--no-fail-fast`, fmt/clippy clean, both probes exit 0, "Passed 113 Lean
audits" (the close's 114th, `AuditSourceCoordinate`, was not yet on that base).

Wave 27 close (2026-09-12): **six pieces** merged one at a time under the
failure-gated chain, each under its own green gate — `CT23-CREDIT-ERROR-PATH`,
`DOCS-COMPACT-3` (the recurring three-file compaction *plus* a user-directed
examples style pass), `AGGREGATES-V1`, `QUANT-REGION-SOURCE`,
`CT07-SLICE-PERMISSIONS` and the arbiter `GATEWAY-C-LOCATION`. The wave's
language head is the **first source-level aggregate type**: `source.record.v1`
lands `type NAME { FIELD: i32, … }`, `let p = NAME { FIELD: OPERAND, … };` and
`let a = p.FIELD;` on `source.buffer.v1`, emitting real C structs with one
distinctly named member per field and byte-compared against a sanitized native
run, so CT09's *"distinct ordinary struct fields"* scenario is **executable** —
a lowering that gave two fields one member would print one value twice. Beside
it, `source.region.v1` gives the kernel's quantitative region its first source
path (`error region R { … } catch (E) quant open R as p { discharge p; }`), the
sealed `CleanupPack` bound as a **linear** capability consumed exactly once and
the admitted region **erasing** to byte-identical C with the hand-written
straight-line program it is; `EpochCreditRecoveryV1` checks what a flat epoch's
creator does with a grant on its **own error edges** (`restored + spent =
credit` componentwise, `restored ≤ credit`, and `restored_credits +
spent_credits = granted_credits ≤ allocation`), with abandonment and minting
both **refusals**; and `borrow_utf8(NAME, LOW, HIGH)` lends a **range** of an
owned buffer, so two disjoint permissions over one buffer are expressible and
the disjointness is *decided* (`max low < min high`, proved to be the
set-theoretic question itself rather than an approximation of it). **Three
recorded blockers were measured false, each in a different way, and that is the
wave's shape.** `STD-BUFFER-SLICE`'s blocker sentence — *"needs the ownership
IR's borrow nodes to carry a footprint fraction"* — is about the **write**
direction: a read lend leaves the write authority with the owner, the ownership
IR is untouched (`ownership.rs` does not appear in the diff) and the fraction is
what `SLICE-PERMISSIONS-WRITE-LEND` needs, so `STD-BUFFER-SLICE` is retired **in
part**. `RegionOp::QuantOpen`, the operation *variant* CT21/CT22's recorded gap
named, is **unreachable from a valid program by design** — the admitting
constructor is the `error_region::QuantOpen` *struct* on a catch clause and the
variant is retained so an opening outside a catch has an explicit `Q302`
(`REGION-QUANTOPEN-OP-VARIANT`) — so the card landed the half that matters and
said which. And the obvious aggregate spelling `record` was **rejected on
evidence, not taste**: `docs/reports/remaining-v07/probes/d02_boundary_record.qcore`
is the single line `record R { }` and two archived reports pin it to `Q000`, so
claiming the word would have falsified a frozen archive (constraint 21) — the
syntax was renamed rather than the pin edited, and the head is the
already-reserved `type` claimed contextually at `type NAME {`, censused at zero
bare occurrences. **Zero reserved words and zero new diagnostic codes across the
whole wave**: every refusal is a stated condition extension of an existing band
under the `Q-NAMESPACE-DECISION` ice box, run four times on four different bands
(`Q630`/`Q635`/`Q636`/`Q641`; `Q051`/`Q052`/`Q053`; `Q302` with `Q303`–`Q309`
reached from a second route; `Q077`/`Q078`), including by the language head
itself. `DOCS-COMPACT-3` honoured the cadence three waves on — **39 standing
constraints in, 39 out, zero dropped, zero renumbered**, 37 bodies byte-identical
and the two reworded ones argued clause by clause, `AGENTS.md` −796 bytes while
gaining lines, `README.md` refreshed to the full route surface, one drift
corrected (the entry points said `qcore-audit CONFIG.json`; the binary is
`quirin-audit`, `qcore-audit` being the package and library name) — and carried
the user-directed examples style pass beside it, *pipe the flow, parenthesize
the assembly*, five files respelled one commit each with byte identity measured
rather than predicted, nine examined and skipped with reasons, and the rule's one
open case named `EXAMPLES-STYLE-SLICE-FANOUT` for the user rather than repaired
against the brief's direction. **The wave ran one arbiter and its re-pin is
presentation-only.** `GATEWAY-C-LOCATION` makes a gateway diagnostic name the C a
person wrote — `L19357` is now `flb_input_chunk.c:2267` — by pairing the
analysed `-E -P` text with a second marked `-E` run under a **checked** line
correspondence, with the preprocessed `L`-number kept as the **internal**
coordinate by the user's ruling. No presented file spelling is ever read from a
flagless marker (proved in general as `presented_files_are_entered_files`), a
disagreeing flagless marker **withdraws** the coordinate rather than relabelling
it, and constraint 12's scope rule is untouched. The BEFORE run reproduces the
preceding baseline's `diagnostics.json` and `chunk-contracts.json` byte for byte
and the AFTER run differs in attribution presentation alone: the corpus baseline
moves to `docs/reports/gateway-c-location/artifacts/` with **185 units 2 Safe /
42 provisional Unsafe / 137 Unknown / 4 data-only**, 4,834 projected functions
150/84/4,600, leak **266**, `unknown_alias` **4,344**, `unsupported` **49,773**
and all 54,383 diagnostic rows keeping their `(function, preprocessed line,
category)` — **zero verdict moves, zero leaf-set moves, zero STOP findings**, so
constraint 7 gains no sixth move and every preceding archive stays frozen.
**Nothing is promoted**, no `Unknown` became `Safe`, every provisional `Unsafe`
stays a provisional claim and never an upstream defect claim, and full language
conformance stays **false**. CT coverage is **unchanged at 36/50 mapped**: the
wave mapped nothing and edited no manifest row, and the CT08/CT09/CT21/CT22/CT23
readiness it produced is recorded as capability in the cards' own reports and
`ct_stage_evidence.json` proposals — a capability is not a mapping (constraint
23). Standing constraints: 15h is **amended** on the slice card's own proposal
(the borrow lends a range; three asymmetries — the release arm is not range-wise,
an undecidable index conflicts with everything, two overlapping borrows are
deliberately admitted), and three new rules land in merge order, 15j (the epoch
credit law, with `Q637` deliberately not extended), 15k (a v1 record is a plain
copyable value and the field-custody answer is a measurement) and 15l (a region
erases from emission; the operation variant and the v1 path bound must not be
"repaired"), beside **12a** in the compile-task/source-transport block for the
arbiter's anti-forgery rule. Gate at the integration tip, `$HOME/.elan/bin` on
`PATH`: `cargo test --workspace --no-fail-fast` **4,376 passed, 0 failed, 8
ignored**; fmt clean; clippy clean; both probes exit 0; **114 Lean audits**
(110 → 114: `AuditSourceRecord`, `AuditQuantRegionSource`,
`AuditSlicePermissions`, `AuditSourceCoordinate`; the epoch card added 17
theorems inside an existing audit, which is why five cards produced four files);
193 Python tests OK after the matrix regeneration. Provider 122 is **carried** —
no provider source, manifest template or `qcore-schemas` version changed and
`qcore-schemas` stays 14.0.0, so constraint 18's rebuild-and-verify obligation is
not engaged. Evidence: `docs/reports/ct23-credit-error-path/README.md`,
`docs/reports/docs-compact-3/README.md`, `docs/reports/aggregates-v1/README.md`,
`docs/reports/quant-region-source/README.md`,
`docs/reports/ct07-slice-permissions/README.md`,
`docs/reports/gateway-c-location/README.md`, `docs/reports/wave27-close/gate.json`.

Wave 26 close (2026-09-12): five cards merged one at a time under the
failure-gated chain, each under its own green gate, plus the integrator's own
three per-owner conflict compositions — the spec's §L21 rows, the examples index
rows, and `examples/sockets/README.md`'s boundary list, where the merged HEAD's
socket-event bullet was composed with the inet card's post-address boundaries
rather than either side being taken whole. `CT-WIRE-LANE` answered the user's
"anything else we can wire with the current implemented tools?" with a row for
every one of the seventeen unmapped CT cases and moved the three the answer is
**yes** for: CT06 and CT11 on `STRING-CONCAT`'s `borrow_utf8` bridge plus
`STRING-SLICE`'s slices — the first CT cases mapped on the **sequential** route
at all — and CT49 on `OBS-REORDERING`'s window-and-emitter comparison, through a
new `reordering_partial_order` verdict mode that also carries the wave's one new
sanitized-native row. CT mapping moves **33/50 → 36/50**, unmapped 17 → 14, and
harness-gated sanitized-native **4 of 33 → 5 of 36**; the **proof stage stays 0
of 36**, honestly, because no theorem is bound to any CT fixture (`T00.7`). Of
the seventeen reasons, 3 are retired and mapped, 5 partially retired and still
unmapped, and 9 stand. **A grown denominator is mapping, not conformance**:
nothing is promoted, `git diff` over `crates/` and `compiler/src/` is empty for
that card, and eighteen scratch mutations were each refused. The lane also
**withdrew two labelled rows it had measured** (CT24, CT47) rather than loosen a
test around them — `newly_decidable_not_mapped_reasons` is derived from that
judgment's own key set, so publishing them would have put three Python tests red
against a matrix the card may not regenerate; both cases keep card T00's
DECIDABLE-NOW bucket and the measurements are published in the lane's own record
(constraint 23). `LOOP-UNROLL-BUDGET` turned `source.loop.v1`'s single ceiling
into a three-layer budget — an **absolute ceiling of 4096** no declaration may
raise, a **default budget of 256** an unattributed `for` is held to, and a
**licence** the statement-adjacent `[| #unroll, where #bound < N |]` clause list
declares — closing `LOOP-BUDGET`. **The measurement that chose the numbers found
something the brief did not expect**: the cost of exact bounded unrolling is not
a function of the bound at all. Three of four representative bodies are linear
and cheap at every admissible bound, while a body acquiring and releasing once
per iteration was **quadratic** at **56.4 s** of check time at bound 1024,
because `LOOP-V1`'s peak-live narrowing had fixed the *count* of prefixes scanned
and left each one re-executed from the start; replacing that rescan with one pass
takes the same body to **0.39 s**, a factor of **145**, changes no published
value (three checks say so, including a Lean theorem over any stepper and any
measure), and is what made 4096 defensible rather than 1024 — lowering it would
have withdrawn the full-extent traversal `MAX_ITERATIONS == MAX_EXTENT` exists to
keep expressible. The declaration is **checked, never trusted**, the attribute
**erases** (attributed and unattributed programs of the same admitted bound emit
byte-identical C), and the card allocated **zero** diagnostic codes — `Q097` and
`Q098` condition extensions under the iceboxed `Q-NAMESPACE-DECISION` — and
**zero** reserved words, a first for this family, with one new token `#` and the
`quant[| n where n := 5 |]` alias's continued behaviour re-measured rather than
asserted. One `LOOP-V1` fixture was amended deliberately, 38 of 39 untouched.
`HTTP-V1-COMPUTED-LENGTH` closed the seam `HTTP-QUIRIN-V1` and `STRING-CONCAT`
each recorded from their own side: a header value or a content may now be a value
the entry computed — a concatenation, a `dec`/`hex` rendering, a `len` binding —
with `admit_field` and `admit_framing` **unedited**, because a component is
resolved to bytes *before* those gates run and so the framing gate never learns
which spelling produced them. The `Content-Length` equality therefore stays
**decided** rather than becoming a premise, the `CR`/`LF` exclusion strengthens to
an **iff** (a composition is framing-free exactly when every piece is, so header
injection stays unexpressible however a value was built), and
`examples/http/computed_length.qcore` serves a computed length over live loopback
with every pre-existing program byte-identical against reverted sources. Standing
constraint 15f is **amended** accordingly — its wording pinned "the literal
content's byte count", now "the content's byte count", both sides still
check-time-known — deliberately and stated, not quietly. `EXAMPLES-SUITE-WIRE`
put all **eleven** example families under `examples_suite.rs` by **directory
scan** rather than a hand-kept list, so the gap it closes stays closed
structurally; it published `examples/loops/buffer_copy.qcore` with a sanitized
run, added 12 tests and repaired 5 README pages, closing
`STRING-SLICE-IO-EXAMPLE-SUITE`, `HTTP-V1-EXAMPLE-SUITE` and `LOOP-EXAMPLE` while
explicitly **not** closing `STD-BUFFER-EXAMPLE` (a loop example that runs on the
buffer route is not a buffer family page). It carries no Lean on stated ground —
it moves no semantic surface. It also found the gate defect this close acts on:
`cargo test --workspace` is fail-fast, so one constraint-16 flake hid the entire
`qcorec` crate from a run, which is why the standing gate now documents
**`--no-fail-fast`** (`GATE-NO-FAIL-FAST`, a strengthening). `SOCKET-INET-ADDRESSES`
answered the user's "bind IPv4 and IPv6 addresses with ports for outbound and
inbound transportations" with declared address literals carrying **zero** reserved
words and **zero** lexer changes — a dotted quad is already `Float Dot Float`,
`[::1]` rides the existing brackets and `[|` still wins by longest match — an
address position being told from a name position by the one fact that every
address of this route spells a port. RFC 4291 forms 1–2 are admitted and the
embedded-v4 and zone forms refused **by name**; `IPV6_V6ONLY` is 1 on all three
arms and proved constant in Lean; the IPv6 probe follows constraint 15d's shape
with **no v4 fallback**; `SO_REUSEADDR` is set on the POSIX arms and deliberately
**absent** on Winsock — one property, opposite options, argued rather than
inherited. The external-peer refusal is located and proved
(`an_undeclared_address_reaches_nothing`), `Q084`'s condition is widened and
stated, and standing constraint **15i** is the wave's only new rule, numbered in
merge order after 15h. Merged gate at the integration tip `4989622c`: **4,209
workspace tests passed, 0 failed, 8 ignored**; 122 provider tests **carried**
(no provider source, manifest template or `qcore-schemas` version changed, so
constraint 18's rebuild is not engaged and 14.0.0 stands); 193 Python; **110 Lean
audits** (107 → 110: `AuditLoopBudget`, `AuditHttpComputedLength`,
`AuditSocketInetAddresses`), read from the text line rather than an exit code;
both probes exit 0; `fmt` and `clippy` clean. **Nothing is promoted.** No wave-26
card touched a gateway, contract, audit or provider file, so the Cycle 19 corpus
rows are **carried** from the wave-25 measurement rather than re-measured: raw
units stay **2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only**,
functions 150 / 84 / 4,600 of 4,834 projected, leak diagnostics **266** with
constraint 7 gaining no fifth move, and the contract corpus **73 modules**. Every
provisional `Unsafe` stays a provisional claim and never an upstream defect
claim; full language conformance stays **false**. Constraint 16's load-sensitive
rows tripped during two cards' runs (the confined-fixture write test and the
`RLIMIT_NPROC` row) and were verified in isolation exactly as that constraint
prescribes, with no assertion weakened. Evidence:
`docs/reports/ct-wire-lane/README.md`,
`docs/reports/loop-unroll-budget/README.md`,
`docs/reports/http-v1-computed-length/README.md`,
`docs/reports/examples-suite-wire/README.md`,
`docs/reports/socket-inet-addresses/README.md`,
`docs/reports/wave26-close/gate.json`.

Wave 25 close (2026-09-12): five cards merged one at a time under the
failure-gated chain, each under its own green gate, beside one closure the user
validated on their own Windows host. `WINDOWS-NATIVE-SOCKETS-HARNESS` armed the
wave-24 native suites for MSVC — `std_socket`, `std_http` and the string
rendering runs, whose native modules had been `cfg(any(linux, macos))` and so
did not exist on Windows at all — through the established
`tests/support/windows_c.rs` harness, with the Unix arms byte-untouched in
behaviour and the Windows arms type-checked and linted from Linux against
`x86_64-pc-windows-msvc`, the check proved live by a deliberate error inserted
and removed. The card **discharges nothing by itself**: it makes
`HTTP-V1-WINDOWS-RUN` and `STRING-SLICE-IO-WINDOWS-RUN` dischargeable by exact
named test rows, and its Windows arm claims the **byte-comparison** half only —
MSVC ships no ASan/UBSan in this harness, so leak detection stays a Linux
observation and Apple ASan/UBSan an Apple one. Its socket half was **superseded
at integration**: while it was in flight the user's own host landed `360d9464`
with a validated `native_windows` module and a `windows_c.rs` extension, and
real `cl.exe` runs of both published socket examples against Winsock loopback —
Visual Studio 18 Community, MSVC 19.51.36248, stdout byte-identical to the
reference observation, stderr empty, two runs of one program identical, the
complete 49-test `std_socket` target green — so `SOCKET-V1-WINDOWS-RUN` is
**discharged on the user's validated evidence** (byte comparison; the sanitizer
half stays a Unix observation) and the merged tree carries the user's wiring for
that file, the card standing for `std_http.rs` and for the `string_slice_io`
CRLF transcript repair. New residual `WINDOWS-RUN-TEST-NAME-SANITIZED`: four
tests keep `runs_sanitized` in their names while their Windows arm makes no
sanitizer claim, recorded rather than renamed because two frozen reports cite
those exact paths (constraint 21). `SOCKET-EVENT-COMPOSE` made a checked socket
a reactor-registered event source — the socket card's own named follow-on,
retired — and the custody relation is a **loan** both existing models were
already shaped to give: the projection carries no `CreateSocketPair` and no
`CloseSocket`, `watch E S read;` is an `OwnershipOp::BorrowShared` running to
its `unwatch`, so a close while registered is the unedited ownership checker's
drop rule and "deregister before close" is a **consequence** rather than a rule
the card wrote. **Zero diagnostic codes allocated and zero reserved words
added**, with one deliberate, argued widening — `Q083`'s condition moves from
"the inherited ownership checker refused" to "an inherited, unedited **custody
model** refused, carrying that model's own error value", the
`NATIVE-LIBRARY-HEADER-DECORATION` precedent — and a dispatch turn **drains
nothing**, which is what level-triggered readiness means. The emitted program
has no run-time registry: the reactor it runs *is* the declared registration
set materialised as the `poll` set at each turn. One published example,
`examples/events/socket_server.qcore`, watching two descriptors, sanitized on
Linux with its stdout byte-compared; Windows and Apple stay compile-path-correct
only (`SOCKET-EVENT-WINDOWS-RUN`, `SOCKET-EVENT-APPLE-RUN`), and
`HTTP-V1-EVENT-COMPOSE` is deliberately **not** landed and stays a refusal.
`LOOP-V1` landed `source.loop.v1` — `for NAME in 0..BOUND { … }`, admitted on
`source.buffer.v1` only — whose checking route is **exact bounded unrolling and
deliberately not a fixed point, and the reason is not cost**: a route that
publishes exact values and validates them by byte comparison against a native
run would find a widening over a back edge replacing "byte 3 is 65" with "byte 3
is some byte", which makes the route's evidence unwritable rather than the loop
cheaper. The per-iteration custody discipline was almost entirely already there,
so `Q099` exists to make the rule hold at a bound of **one**; the emitted C is a
**real `for`**, which is what makes the native run a check of the unrolling
rather than a restatement of it. **One reserved word** (`for`, census-clean) and
zero new tokens; **`in` is deliberately not reserved**, the constraint-14 census
having found it as a genuine reference in the pinned contract corpus
(`ensures normal resource_borrowed(in);`) — the `len` shape, taking that card's
narrow contextual answer. `Q097`–`Q099` under a new `Loop` pass, the band
`HTTP-QUIRIN-V1` returned to the pool, and `STD-BUFFER-COPY` is **real**: a
bounded `dst[i] = src[i]` copy with no new byte statement at all, byte-compared
against a sanitized run as a positive leak control. `STRING-CONCAT` landed
computed text — `concat(…)` over check-time-known pieces, `dec(NAME)`/`hex(NAME)`
as a piece, and `borrow_utf8(NAME)` bridging an owned `std.buffer` — and its
headline is that the dependency three earlier documents recorded ("concat needs
owned storage, which is the borrow's question first") is **false and measured
so**: every v1 piece is check-time-known, so the compiler holds the composed
bytes and the result is a new `static const` payload with no allocation, no
copy, no custody and a length that is itself a constant. `Q077`–`Q079`, the
`StandardString` pass's third band and all three used; **zero** reserved words;
one published example, `examples/strings/computed_length.qcore`, rendering a
computed `Content-Length`. The arbiter `AOM-SIBLINGS-BINDING-POISON` repaired the
blanket binding poison with a **bounded allow-list**: a refused statement now
spares only bindings it provably cannot have written — a plain name, an automatic
object of this body whose address is never taken, no write position in the refused
text — while every refused **condition** and control-flow refusal keeps the full
poison (`AOM-POISON-CONDITION-FRAME`). **Nothing is promoted and no pin moves**:
unit verdicts stay 2 `Safe` / 42 provisional `Unsafe` / 137 `Unknown` / 4
data-only with zero status and zero `reason_code` moves, the projected-function
histogram stays 150 / 84 / 4,600 with zero moves across 4,834 functions, and
**leak stays 266** with not one leak leaf added or removed, so constraint 7 gains
no fifth move. What did move is attributed leaf by leaf: `unknown_alias`
4,421 → 4,344 and `unsupported` 49,781 → 49,773, 87 `Unknown`-class leaves across
26 units, 1 added and 86 removed. The L19399 leaf — `CONSTRUCTOR-RESIDUALS-LEAK-271`,
which wave 23 decided by **withdrawal** — is now decided by the **discharging**
arm of `flb_input_chunk_destroy_direct_routes`'s contract, that arm's first
firing anywhere in the pinned corpus, which is a better answer to the same
question and changes no count. Three workspace assertions were re-pinned, each
argued and one strictly more accusing (a `UnknownAlias` becoming a
`DoubleRelease`), and the card's pre-registered prediction record is published
as it stood: six structural claims held, three numeric ranges were wrong with
the reason named. Two integration reconciliations were made in the integrator's
own surface rather than inside any card: the Windows-wiring supersession above,
and `STRING-CONCAT` reconciled with `LOOP-V1` on its own branch after the loop
card merged first — `ValueRun` keeps the walk, `BufferEvaluation::apply` keeps
meaning, a flat prelude-scan fail-**open** trap was caught before it could
silently regress, the loop-window refusal is fail-**closed** at `Q068`, and
`LOOP-V1`'s own 39 and 52 test suites are unchanged on the merged tree, so
`STRING-CONCAT-LOOP-WINDOW` was predicted and then **met** rather than left
open. The grammar's loop section was renumbered §G07.11 → **§G07.12** at
integration, `SOCKET-EVENT-COMPOSE` having merged first, with its
cross-references repaired in the same commit. Gate at the integration tip:
4,089 workspace tests (8 ignored), 122 provider carried, 193 Python, 107 Lean
audits (103 → 107: `AuditSocketEventCompose`, `AuditSourceLoop`,
`AuditStringConcat`, `AuditRefusedWrites`), both probes exit 0, fmt and clippy
clean; `qcore-schemas` stays 14.0.0, so constraint 18's rebuild-and-verify
obligation is not engaged. The corpus baseline stays
`docs/reports/wave23-c-operations/artifacts/` with the attributed AFTER runs
chained beneath it — `aom-destroy-siblings`, now `aom-binding-poison` — and no
frozen archive is edited. Nothing is promoted: every raw Cycle 19 unit stays
unpromoted, a provisional `Unsafe` is never an upstream defect claim, and full
language conformance stays **false**. Evidence:
`docs/reports/windows-native-sockets-harness/README.md`,
`docs/reports/socket-v1-windows-run/README.md`,
`docs/reports/socket-event-compose/README.md`, `docs/reports/loop-v1/README.md`,
`docs/reports/string-concat/README.md`,
`docs/reports/aom-binding-poison/README.md`,
`docs/reports/wave25-close/gate.json`.

SOCKET-V1-WINDOWS-RUN (2026-09-12): the two published `source.socket.v1` examples now compile and execute through the shared Visual Studio infrastructure on Windows, with a harness entry point that refuses unless the `cc` toolchain resolved Microsoft's actual `cl.exe` rather than an MSVC-like substitute. The TCP and UDP programs link Winsock solely through the emitted source's `#pragma comment(lib, "ws2_32.lib")`, run their real loopback implementations, keep stderr empty and reproduce the reference observation byte for byte; a repeated TCP run pins platform-independent determinism. Visual Studio 18 Community MSVC 19.51.36248 passes all three native rows and the complete 49-test `std_socket` target, and the MSVC-core CI lane now names that target explicitly while Clang is unavailable. The standing workspace gate also exposed and repaired one unrelated CRLF-only test-reader defect: `string_slice_io` now normalizes the included README before locating its LF-spelled transcript fence, without changing source or string semantics. This is native execution evidence, not a sanitizer claim; socket semantics, certificates, verdicts, providers, Lean and frozen corpus artifacts are unchanged. Evidence: `docs/reports/socket-v1-windows-run/README.md`.

Wave 24 close (2026-09-12): five cards merged one at a time under the
failure-gated chain, each under its own green gate. `DOCS-COMPACT-2` ran the
recurring compaction of `AGENTS.md`, `CLAUDE.md` and `README.md` before the
wave opened — 33 standing constraints in and 33 out with zero dropped and zero
renumbered (renumbering was ruled out by the card's own census, which found
five Rust files citing constraints by number), 67 evidence pointers in and 67
out, 63 diagnostic codes in and out, and three drifts corrected by re-reading
their sources: constraint 16 said "four rows" and listed five, constraint 25
put the frozen B00 ledger's 59-module checkpoint "beside a live 69" where
`contract/qcore` measures 73, and `docs/GENERALIZATION_PLAN.md` §6.1 still
carried the pre-sync-rule instruction to update `README.md` in the same change.
`README.md` was deliberately *grown* by 21 lines, because six admitted source
profiles had no mention in the human summary at all, and the every-2–3-waves
cadence is now a standing working rule in `CLAUDE.md`; the card changed no fact
about the system and carries the follow-on `DOCS-COMPACT-2-W4-DESIGN-POINTER`.
`QUIRIN-SOCKET-V1` then provided `std.socket` and `source.socket.v1`, a checked
TCP **and** UDP surface whose defining decision is that every peer of every
socket in an admitted program is another socket of the same program, on
loopback, in one thread — not a testing convenience but what makes the route's
two determinism obligations decidable: the model carries each stream's
committed bytes and each datagram queue, a receive the program's own history
does not satisfy is refused at `Q085` rather than deadlocked on at run time,
and what a receive yields is published in the certificate and **compared by the
emitted program**, which exits non-zero on disagreement. Custody is the
unedited `qcore_core::ownership` checker's, the close discipline is read out of
`os_endpoint::close_poststate` — which answered the poisoned `Attempted` where
the card expected `Indeterminate`, recorded as `SOCKET-V1-CLOSE-ERRNO` rather
than argued away — the derived row carries `Alloc` and `Io` and never `Mut`,
and ten reserved words landed after a 412-fixture / 41,529-literal census that
**rejected** `stream` and `socket` by measurement. `STRING-SLICE-IO`
implemented the string amendment's own §7 follow-on: a `source.string.v1` entry
may write `print TEXT;` and `log LEVEL TEXT;` over a quoted literal, a bound
string or one inline slice, with the `|>` spellings the same statements — and
the capability rule did not move, which is the point: `std.io` alone still
grants no string and `std.string` alone still opens no stream, and what changed
is that a file naming **both** can render text. The rendering rule is the
identity on bytes, framed for both line kinds by one function so a rendered
scalar line **is** the text line of its own digits (`rfl` in Lean); a string
takes no rendering selector and one written in front of one keeps refusing at
`Q072`; and an embedded NUL renders as an ordinary byte, which was not free —
every `printf` conversion for a byte range stops at the first `0`, so the
emitted line writes the payload with a length-bounded `fwrite` from a **second
prelude fragment** inserted only into a program that renders text, with the
write's result checked rather than discarded, which is what keeps every other
emitted program byte-identical. Zero reserved words, `driver.rs` untouched, and
one shared-ground edit turning the pipe's closed sink table from selector
counts into ranges, which moves one refusal from the pipe to the route (`x |>
print;` now refuses at `Q030` naming the missing `dec`/`hex`) and creates no
acceptance, on a 126-occurrence census of every piped sink statement on the
base. One real cross-card composition surfaced at integration and was repaired
there rather than inside either card: the socket card's ninth `Imports` field
met the string-io card's eight-field constructors, caught by the Lean `TEXT`
check rather than by a test. `C-EMIT-MAIN-UNIFY` answered the user's
architecture direction with the census first — ten emitters wrote `main` in
four byte spellings, and the collision was measured **not reachable today**,
because the driver's route dispatch is a chain of mutually exclusive arms and
33 emitted units carry exactly one `int main(` each — so the card manufactures
no defect and repairs the architecture the concern correctly names: composition
no longer means writing a new whole-program backend with a new entry point, but
contributing keyed `Fragment`s in declared sections plus one `Entry`, where the
single-entry guarantee is a type rather than a check. **33 emitted translation
units, 33 identical digests, zero differing bytes** against a pristine checkout
of the base, every pin unmoved, with the one section inversion the card found —
the task route's `#include <stdio.h>` sitting among its preludes — reported as
`C-EMIT-TASK-LATE-STDIO` rather than repaired under two F02 emission digests.
`HTTP-QUIRIN-V1`, the wave's milestone, is `merge`'s first consumer and uses
that mechanism as designed: **an executed HTTP/1.1 subset with the boundary
named**, written in pure Quirin over the socket route's unedited checker, with
zero external C libraries. Its move is that the design record's prerequisite
list was the list for *parsing* HTTP and not for constructing and checking it —
every component of a declared message is a quoted literal, so its bytes and
therefore its length are known at check time — and 22 static conformance checks
follow, two of them the card's signature: **no admitted component may carry a
`CR` or an `LF`**, so every CRLF in an admitted head is one the compiler wrote
and header injection is not expressible on this route rather than merely
unintended; and a declared `Content-Length` must **equal** the literal
content's byte count, a compile-time refusal a contracted binding to a C
library could not have offered at all, since the library computes the length at
run time. Two published programs run under ASan/UBSan/LSan with their output
byte-compared, and the second is published for the contrast:
`raw_request.qcore` puts the same kind of bytes on the wire by hand and the
compiler checks only that the receive is satisfiable — a payload a program
*writes* is bytes, a message it *declares* is checked. Four reserved words on a
416-fixture / 43,239-literal census that **rejected `body`**, a binding name in
two shipped fixtures, where RFC 9110 §6.4 had already renamed the concept
`content`. Band discipline held all wave: `Q080`–`Q089` and `Q090`–`Q099` were
pre-assigned by the integrator before their cards started rather than relocated
after a collision, two ranges came back — `Q077`–`Q079` from `STRING-SLICE-IO`
and `Q097`–`Q099` from `HTTP-QUIRIN-V1` — and the returned lower range was
deliberately not backfilled. Beside the merges, `EVENT-V1-APPLE-RUN` is
**discharged** on the user's own macOS environment: the `std_event` suite
including its `cfg(not(windows))` native comparison ran green, scope recorded
exactly as Apple ASan/UBSan and byte comparison with no LSan claim;
`SOCKET-V1-APPLE-RUN` stays open until a macOS run at or after the socket card
covers `std_socket`'s native tests. Measured close gate: **3,950 workspace
tests, 0 failed, 8 ignored**, `fmt` and `clippy` clean, both reevaluation
probes exit 0, **193 Python tests**, **103 Lean audits** (99 → 103:
`AuditStdSocket`, `AuditStringSliceIo`, `AuditCProgram`, `AuditStdHttp`), and
122 provider tests carried unchanged with `qcore-schemas` still 14.0.0 and no
package rebuilt. **The corpus is untouched and unread all wave**: no gateway,
contract, audit or provider file appears in any of the five diffs, so the leak
pin stays **266**, the contract corpus stays 73 modules, unit verdicts stay 2
`Safe` / 42 provisional `Unsafe` / 137 `Unknown` / 4 data-only, the function
histogram stays 150 / 84 / 4,600, and the baseline
`docs/reports/wave23-c-operations/artifacts/` is neither re-measured nor
re-pinned. Nothing is promoted: no raw Cycle 19 unit is promoted, no function
moved status in either direction, every provisional `Unsafe` stays a
provisional claim and never an upstream defect claim, and full language
conformance stays false. Evidence: `docs/reports/docs-compact-2/README.md`,
`docs/reports/quirin-socket-v1/README.md`,
`docs/reports/string-slice-io/README.md`,
`docs/reports/c-emit-main-unify/README.md`,
`docs/reports/http-quirin-v1/README.md`.

Native-library decoration Windows fixture repair (2026-09-12): the manifest and visibility-header mutation helpers now normalize checkout line endings before applying LF-spelled edits, so the four negative rows exercise their intended malformed include and non-derivable decoration cases under CRLF rather than either failing at the test guard or silently compiling the unedited manifest. The header-removal row also asserts that both binding statements were actually removed. The focused `native_library_header_decoration` suite passes all 22 tests; the full workspace test gate, formatting, Clippy, both reevaluation probes and all 99 Lean audits pass. Production reader semantics, verdicts, provider packages and frozen corpus artifacts are unchanged.

Wave 23 close (2026-09-11): six cards merged one at a time under the
failure-gated chain, each under its own green gate. `NATIVE-LIBRARY-SOURCE-BINDING`
gave a source file its own manifest selection — `extern library NAME;` resolves
that file against the one selected manifest declaring the name, an unbound file
is unchanged and still resolves selection-wide, and a binding only ever narrows,
with a refusal owed exactly where it removes a resolution (band `Q065`–`Q067`,
`NativeLibrary`'s fourth range, zero reserved words, the identity still a stated
pin); it ships `docs/design/STD_HTTP_BINDING_V1.md` beside it, design-only, with
no library named as chosen. `PIPE-DROP-STAGE` (with `PIPE-INTRINSIC-EXAMPLE`)
admitted `drop` as `source.pipe.v1`'s third statement-stage head and its only
releasing one, so the allocating chain wave 22 measured as unreleasable is
spellable end to end with the pipeline releasing the temporary it invented;
custody flows through the unedited checkers, no code and no reserved word were
added, and no other keyword-shaped head joined on a two-part census criterion.
`STRING-SLICE` provided `std.string` and `source.string.v1` — immutable UTF-8
`{ptr, len}` views of statically stored literal bytes, an exact O(1) length and
an allocation-free copy-free checked slice, reference-executed and lowered to C11
with per-stream byte agreement under sanitizers — on band `Q068`–`Q073` under a
new `StandardString` pass, relocated at integration from the `Q065`–`Q070` it
allocated. `COMPOSE-EVENT-LINES` put `print`, `log` and `flush` and the one value
form `let NAME = dispatch ready E;` inside an event entry without moving anything
of `Q058`–`Q062`, and declared no reordering because none is needed: the
`OBS-REORDERING` live-set walk, generalised to a dispatch turn, reads its
emitters off the single-threaded reactor profile, so every record is a barrier
and the existing byte comparison is shown to be the right one rather than
replaced — landed as standing constraint 15c, on relocated code `Q074`.
`NATIVE-LIBRARY-HEADER-DECORATION` made a decorated exported header readable by
*deriving* the export macro from declared, hash-identified include texts,
evaluating none of the four conditional arms it is defined in: a manifest says
which texts may be read and the reader says what they mean, Libevent's census
goes 0 → 264 of 423 read on one declared text, declaring `event2/util.h` refuses
seven headers as whole files because it carries `#error`, and a silently ignored
prefix decoration is repaired — with no new diagnostic code, `Q063` extending
instead. `AOM-DESTROY-SIBLINGS` then decided the two undeclared sibling
destructors in opposite directions: `flb_input_chunk_destroy_direct_routes` is
declared with a **nullable** release where its neighbour carries `nonnull`, and
`flb_input_chunk_destroy_corrupted` is read in full and declined at measured
payoff zero. Its payoff moves the leak pin **272 → 266**, constraint 7's fourth
deliberate move and the first of more than one, entirely by withdrawal-by-refusal
at the preceding call sites rather than by discharge, so the declaration's
discharging arm stays unexercised corpus-wide; `CONSTRUCTOR-RESIDUALS-LEAK-271`
is decided at a measured cost of five withdrawn claims rather than the six wave
17 recorded, its sixth leaf L19544 standing on the success path, which is why the
function keeps its provisional `Unsafe` and the pin reads 266 and not 265. The
contract corpus grows to 73 modules (manifest-selected 18, v1 declarations 39);
unit verdicts stay 2 `Safe` / 42 provisional `Unsafe` / 137 `Unknown` / 4
data-only and the function histogram stays 150 / 84 / 4,600 with zero status
moves in either direction, and the corpus baseline
`docs/reports/wave23-c-operations/artifacts/` is deliberately **not** re-pinned.
At the close, spec §L21's duplicated `Q050` and `Q063`–`Q064` rows — which the
wave's own merges made worse, one stale copy each carrying pre-wave-22 text —
were deduplicated to one current row apiece with no row lost, closing
`L21-DUPLICATE-ROWS` as a documentation repair. Measured close gate: **3,776
workspace tests, 0 failed, 8 ignored**, `fmt` and `clippy` clean, both
reevaluation probes exit 0, **193 Python tests**, **99 Lean audits** (95 → 99),
122 provider tests carried unchanged with `qcore-schemas` still 14.0.0 and no
package rebuilt. Nothing is promoted: no raw Cycle 19 unit is promoted, no
function moved status, every provisional `Unsafe` stays a provisional claim and
never an upstream defect claim, and full language conformance stays false.
Evidence: `docs/reports/native-library-source-binding/README.md`,
`docs/reports/pipe-drop-stage/README.md`, `docs/reports/string-slice/README.md`,
`docs/reports/compose-event-lines/README.md`,
`docs/reports/native-library-header-decoration/README.md`,
`docs/reports/aom-destroy-siblings/README.md`.

Native observation repair (2026-09-11): native sequential, extern, task, mutex, pipeline and CLI checks explicitly select typed diagnostic sinks after quiet emission, preserving their reference comparisons; CT03/CT17 retain both quiet and instrumented sanitizer controls, and example tests distinguish source-I/O projections from their explicitly published traces. Quiet extern C now includes `<stddef.h>` for `NULL`; mutex observations retain read endpoints and complete declared growth records; two task-emitter Clippy errors are repaired without changing emitted bytes. The compiler facade exposes the existing diagnostic adapter so the CLI keeps its established dependency boundary. F02's instrumented-C pins are deliberately renewed with fresh native causal-relation evidence beside the frozen archives. The Windows task-observation companion now applies that same explicit sink and the same instrumented-C pins; all four native MSVC thread tests pass, including the owned-error and read-loan reference comparisons. The full workspace run passed 3,579 tests with 8 ignored and one temporary CLI layering failure; removing that dependency and recompiling yielded passing reruns of all 31 bitwise-condition, 11 layering, 4 F02 and 13 CLI tests, covering 3,580 distinct workspace tests with no unresolved failures. Formatting, Clippy, both reevaluation probes, 193 Python tests and 95 Lean audits pass. Providers and the frozen wave-23 corpus are carried unchanged, with no new package or corpus run. Evidence and the explicit full-run/rerun distinction: `docs/reports/native-observation-repair/README.md`.

Windows quiet-event test repair (2026-09-11): the `std.event` null-sink regression fixture now removes its logger import and `log_level` statement without assuming Unix newlines, so a CRLF checkout tests quiet emission instead of accidentally retaining the explicit stdout adapter. The focused `std_event` integration suite passes unchanged on the emitted-program contract; the Windows system-trace harness also spells its optional fixture list as an explicit branch for the current Clippy gate, and the Unix-only extern native-run helper is no longer imported into the Windows test build. No compiler semantics, verdicts, provider packages or frozen corpus artifacts change.

Quiet-emission and explicit-system-trace increment (2026-09-11): the C11 backends now retain sequential, extern, task, mutex and event observations behind one typed six-level diagnostic logger whose ordinary sink is null, while `log_level LEVEL;` (gated by `std.logger`, recorded and rechecked, and carrying `Io`) explicitly installs the maintained hosted stdout adapter; source `log` remains a separately composed stderr stream, `Streams` now models stdout/stderr pointwise composition, and the event, thread, pipeline, pipe, declared-extern and branching examples explicitly select `log_level trace;` and publish native-checked transcripts, while unsafe linkage demonstrates caller-authored trace markers without presenting an unchecked return as a modeled value. The English normative amendment and implementation report are `docs/design/QUIET_EMISSION_STREAMS_AMENDMENT.md` and `docs/reports/quiet-emission-streams/README.md`; Rust/Lean coverage pins the quiet default, six levels, import/effect/recheck rules, reactor-projection neutrality and stream algebra. Targeted validation passed: `cargo check --workspace`, `cargo test -p qcore-compiler --no-run`, `cargo test -p qcore-compiler --test std_event` (27), `cargo test -p qcore-compiler --test system_trace_examples_windows` (5 native MSVC transcript tests), the focused `std_modules` log-level test, `cargo fmt --all -- --check`, `lake build QCore.StdIo QCore.StdEvent`, and direct `lake env lean AuditStdIo.lean` / `AuditStdEvent.lean` under elan's pinned Lean 4.33.1 toolchain; both new logger theorems report no axioms, and all other reported axioms remain within the allowed `propext` / `Classical.choice` / `Quot.sound` surface. At the user's direction the full standing gate, clippy, reevaluation probes and all-audit runner were not run, so this increment makes no full-gate claim; providers and the frozen Cycle 19 corpus were untouched.

Wave 23 fixed scalar C queries (2026-09-11): the existing-C gateway admits
typed scalar `sizeof`/`_Alignof` queries in six expression positions and scalar
assignments/initializers. Operand markers never determine values or CFG edges;
assignments forget old scalar facts, and sibling mutations, VLA/opaque operands
and resource accesses retain their refusals. Six new regression tests and nine
Lean theorems cover admission, erasure and scalar-fact forgetting. The pinned
185-unit BEFORE/AFTER comparison removes 27 unsupported diagnostics across
seven units, with no verdict changes, no added diagnostics, and leaks unchanged
at 272; the other four artifacts are byte-identical. The closing gate passes
3,575 workspace tests (8 ignored), 193 Python tests, 95 Lean audits, both probes,
fmt and clippy; 122 provider tests are carried from the unchanged prior package
checkpoint. The shared branch's `1a7846dd` includes the implementation; the
closing evidence records the recovered workspace gate and measured baseline.
This is a bounded first gateway slice, with fixed objects, scalar RHS/mutation
lowering and the other gateway families still named follow-ons. Evidence and
boundaries: `docs/reports/wave23-c-operations/README.md`.

Windows native-event increment (2026-09-11): `EVENT-V1-WINDOWS-AFUNIX`
replaces `std.event`'s unconditional Windows loopback construction with the
pinned Libevent 2.2.2-alpha shape: compile-time `<afunix.h>` detection,
run-time `AF_UNIX`/`SOCK_STREAM` probing, a UTF-8 pathname-backed
listen/connect/accept pair whose connector is separately bound for peer
validation, attempted rollback on every pre-publication failure, and the
existing AF_INET loopback route only for an unsupported family/type/protocol
probe. Other probe failures refuse. The source surface, reactor/custody models, certificate,
diagnostics, model identities and observation bytes are unchanged. The
26-test `std_event` target now runs on Windows through the shared native-C
harness and forces AF_UNIX, explicit loopback and no-header fallback builds;
all three reproduce the reference trace, while an intentional early failure
checks ordinary-exit pathname cleanup. The same harness repairs
`bitwise_conditions`' Windows native cross-check, which previously tried to
launch an absent Unix `cc`, and the two CRLF-sensitive fixture mutations are
newline-agnostic. Evidence: `docs/reports/windows-std-event-af-unix/README.md`.

Windows branch-arm gate repair (2026-09-11): the three native observation
comparisons in `branch_arms` now use the established MSVC harness on Windows
instead of trying to launch the absent Unix `cc`; non-Windows hosts retain the
existing ASan/UBSan/LSan route. The emitted programs, reference observations,
branch semantics, diagnostics and fixtures are unchanged. The three exact
formerly failing tests and the complete 35-test target pass on Windows.

Windows logical-connective gate repair (2026-09-11): the native observation
comparison in `logical_connectives` now uses the established MSVC harness on
Windows instead of trying to launch the absent Unix `cc`; non-Windows hosts
retain the existing ASan/UBSan/LSan route. The emitted program, reference
observation, connective semantics, diagnostics and fixtures are unchanged.
The exact formerly failing test and the complete 25-test target pass on
Windows.

Windows native-library fixture repair (2026-09-11): the two malformed-manifest
rows that remove the library or manifest declaration now use the existing
checked token mutation instead of matching a Unix newline. This makes the
intended `Q039` refusal newline-agnostic without changing the loader, accepted
manifest surface, diagnostics or evidence. The exact formerly failing test
and the complete 16-test `native_library` target pass on Windows.

Windows workspace newline gate repair (2026-09-11): a complete
`cargo test --workspace --no-fail-fast` sweep exposed four remaining failing
targets. Manifest-header and composed-I/O fixture mutations now match semantic
tokens instead of Unix newlines, README fenced-block extraction normalises
checkout line endings, and the task/I/O emission comparison normalises both
sides before comparing synthesized lines. Unix-only native-evidence imports
are target-gated, removing the two Windows warning clusters found by the same
sweep. No loader, source-language, emission or diagnostic semantics changed;
all four repaired targets and the complete workspace sweep pass on Windows.

Post-IF-ARMS gate repair (2026-09-11): the `bitwise_conditions` suite now pins
the existing `Q044` refusal for an arm-local check-time constant binding on its
concrete ground — the binding has no position after the join — rather than on
the superseded pending-owner label. The same gate run narrowed the
`execution_quota` fixture's script helper from Unix to Linux, matching every
call site and restoring warning-free macOS clippy without changing the
provider assertions. The refusal code, span, accepted language, emitted C,
corpus baseline and headline counts are unchanged; the recorded load-sensitive
`vp04_vp07_vp08_process_lifecycle_and_payload_budgets` row remains unchanged
and passes in isolation.

Wave-22 increment (2026-09-11): six delegated cards merged under the
failure-gated chain — the heaviest composition wave yet (a three-way
diagnostics-band contention resolved by merge-order relocation, an enum
variant the blind union misplaced, nine redundant accessor arms clippy
caught, and one `CompileOptions` field rippling into two newer routes'
literals) — composed with five Windows-validation batches, and the corpus
re-pinned to `docs/reports/wave22-corpus/`.
- The reordering capability (OBS-REORDERING): `qcore.obs-reordering.v1`
  compares observation streams window-by-window and emitter-by-emitter
  where a program declares it, the strict comparison proved to be the
  empty relation; worker-body lines land (`COMPOSE-WORKER-LINES`) and
  constraint 15a's condition is met, not waived. Evidence:
  `docs/reports/wave22-obs-reordering/README.md`.
- The byte-buffer tier (STD-BUFFER): `source.buffer.v1` — `bytes(N)`,
  checked indices, `extent`, `drop` — with the census correcting its own
  design: the extent capability already existed as the cycle-3 ownership
  footprint; custody is inherited from the unedited checker. `buffer`
  measured colliding 555 times and respelled `bytes` (band Q051–Q057).
  Evidence: `docs/reports/wave22-std-buffer/README.md`.
- The event surface (STD-EVENT): `source.event.v1` surfaces the existing
  reactor/descriptor custody models (twelve of nineteen operation
  families; the U3 source-unreachability blocker retires), with a
  three-arm embedded poll shim — POSIX `poll` for Linux and Apple,
  `WSAPoll` for Windows — per the user's three-platform rule; the Linux
  arm runtime-verified, `EVENT-V1-APPLE-RUN`/`EVENT-V1-WINDOWS-RUN` the
  named platform residuals (band relocated to Q058–Q062; Mio/Tokio
  binding declined on recorded grounds). Evidence:
  `docs/reports/wave22-std-event/README.md`.
- Intrinsic pipe stages (PIPE-INTRINSIC-STAGE): the census found the
  grammar needed nothing — call-shaped intrinsics were already stages —
  and the card's one production change removed the sequential driver's
  last raw re-lexing site (standing constraint 15b). Evidence:
  `docs/reports/wave22-pipe-intrinsic-stage/README.md`.
- The exported-header level (NATIVE-LIBRARY-HEADER): a manifest may name
  its header and digest; declarations are read from the header's own
  token stream and disagreement refuses (the weaker reading is accepted);
  the census found one decoration macro accounts for Libevent's entire
  unreadable surface (band relocated to Q063–Q064). Evidence:
  `docs/reports/wave22-native-library-header/README.md`.
- The leak leaf decided (AOM-DESTROY): the faithful
  `flb_input_chunk_destroy` contract lands with measured payoff; the
  wave-20 leaf retires as a **withdrawal** — leak 273 → 272,
  `input_chunk_get` provisional `Unsafe` → `Unknown`, nothing promoted;
  the prediction was committed before the measurement and met. Corpus
  72 modules. Evidence: `docs/reports/wave22-aom-destroy/README.md`.
Merged gate at this checkpoint: 3,569 workspace tests (8 ignored),
122 provider tests, 193 Python tests, 94 Lean audits, both reevaluation
examples exit 0, fmt and clippy clean. Follow-on accounting lives in
`docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`, updated
for this wave).


Wave-21 close increment (2026-09-11): the two remaining cards merged under
the failure-gated chain, which caught one composition seam (the example
suite's rename-proof driver constant failed to compile once only `quirin`
existed — `env!` resolves in the arm a match never takes). Wave 21 is
corpus-neutral by construction; the baseline stays
`docs/reports/wave20-corpus/`.
- Source logging in the thread examples (COMPOSE-CONCURRENCY): the new
  route `source.std-io-tasks.v1` composes the checked task route with
  `std.io`/`std.logger` — the task half is the unedited task lowering and
  checker, the line half is each std-io definition extracted and shared,
  so no second lowering or backend exists. The v1 boundary is measured,
  not chosen: every write an emitted task program performs happens on the
  parent's thread, so any parent-body line is join-ordered and admitted;
  a worker-body line refuses at `Q050` (new `ComposedTasks` pass,
  standing constraint 15a). TRACE-STDIO's impossibility pin is rewritten
  into three tests; one STD-IO-COMPOSE route exclusion is lifted and the
  other two re-pinned; `STD-FMT-BUFFER-READ` lands
  (`value |> log info dec` over `qcore_buffer_read`). All four
  `examples/threads` programs gain source logging, sanitized and
  per-stream byte-compared; no synthesized trace byte moved. Evidence:
  `docs/reports/wave21-compose-concurrency/README.md`.
- The example suite (EXAMPLES-SUITE): five families under `examples/` —
  stdio (both renderings, all three severities with the `fatal` collision
  asserted, `flush`'s no-durability boundary, the composed
  `value |> render |> log` chain), pipes, branching, extern, and
  unsafe-linkage — 14 programs, 32 tests, with refused programs and
  quoted diagnostics extracted from the READMEs and compiled, so no
  documented refusal can drift. Evidence:
  `docs/reports/wave21-examples-suite/README.md`.
Merged gate at this checkpoint: 3,343 workspace tests (8 ignored),
122 provider tests, 193 Python tests, 90 Lean audits, both reevaluation
examples exit 0, fmt and clippy clean. Follow-on accounting lives in
`docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`, updated
for this wave).


Wave-21 partial increment (2026-09-11): QUIRIN-RENAME landed through a
delegated worktree — the language presents as **Quirin**, the CLI binaries
rename to `quirin` and `quirin-audit` via `[[bin]] name` alone, and the
`.qc` extension is retired (385 `.qcore`, zero `.qc`, Emacs registration
dropped with a new negative ERT test). The user rule is load-bearing and
kept: every `qcore-*` crate, package, library and namespace name, the
emitted-C `QCORE_*`/`qcore_*` surface and the spec/doc filenames stay —
the core component remains seated as "qcore". Census: 44 of 170 live
occurrences named a binary; no CI edit was needed (every automated
invocation selects the package, not the binary); no transitional alias,
because a second `[[bin]]` would make `cargo run -p qcorec --` ambiguous.
Behaviour neutrality byte-pinned. Evidence:
`docs/reports/wave21-quirin-rename/README.md`.


Wave-20 increment (2026-09-11): six delegated cards merged under the
failure-gated chain — with three integration-time compositions the chain
caught (a removed flag, the negation-return theorem pair, one Lean program
literal) — and the corpus re-pinned to `docs/reports/wave20-corpus/`.
- The headline: **`flb_input_chunk_create` reaches function-level Safe**
  (ABI-TYPEDEF-ADDRESS-OF-MEMBER). The residual was one level off: the
  gateway's `typed_field_access` has its own typedef reader with the same
  brace-abandonment defect wave 19 fixed in the loader, hidden four cards
  by fixture spellings. The admission is Borrowed-position-only; 6
  diagnostics → 0, six exits Discharged, premise list published. The unit
  stays provisional `Unsafe`; nothing raw is promoted. Corpus: 12 leaves
  removed / 3 added, all attributed; functions 149/84/4,601 →
  150/85/4,599; `input_chunk_get` `Unknown` → provisional `Unsafe`
  (accusing); leak **272 → 273** deliberately (constraint 7);
  `msgpack/packer.qcore` lands with its measured payoff (corpus 71).
  Evidence: `docs/reports/wave20-abi-address-of-member/README.md`.
- The integer tier and the bitwise singles (BITWISE-CONDITIONS): one
  position (a parenthesized comparison operand), `|`/`^`/`&` bitwise,
  unary `!` by arm swap, check-time constant bindings; precedence declared
  only where it agrees with C, refused unparenthesized elsewhere.
  Evidence: `docs/reports/wave20-bitwise-conditions/README.md`.
- Arm returns and calls (IF-ARMS): a `return` in an arm discharges at its
  own exit (obligation, not position), the join becomes agreement over
  survivors (a shape change, proved not a relaxation), continuations move
  into the surviving arm, never duplicated. Evidence:
  `docs/reports/wave20-if-arms/README.md`.
- Manifest-bound contract slices (NATIVE-LIBRARY-CONTRACT-SLICE): a
  `contract fn` entry binds a symbol to a declared contract by module
  reference; artifact identity stacks beside the contract premise;
  withdrawal only where the weaker admissible reading refuses (band
  Q047–Q049). Evidence:
  `docs/reports/wave20-native-library-contract-slice/README.md`.
- The observation names the declared outcome (EXTERN-OUTCOME-TRACE):
  `-> N` on a `succeeds INTEGER` call, honest because the emitted program
  verifies the number before the write. Evidence:
  `docs/reports/wave20-extern-outcome-trace/README.md`.
- The borrowed render (STD-FMT-BORROW): `print RENDERING borrow NAME`
  renders the model identity, admitted only where the extern ledger holds
  it live; every larger render names its missing capability. Evidence:
  `docs/reports/wave20-std-fmt-borrow/README.md`.
Merged gate at this checkpoint: 3,277 workspace tests (8 ignored),
122 provider tests, 193 Python tests, 89 Lean audits, both reevaluation
examples exit 0, fmt and clippy clean. Follow-on accounting lives in
`docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`, updated
for this wave).


Wave-19 remainder increment (2026-09-10): the six-card spine closed through
delegated worktrees under the failure-gated chain, composed with three
Windows-validation batches from the user's host by ordinary merges. Five
cards are corpus-neutral by construction; the sixth measured the corpus in
both directions and moved exactly two attributed leaves, so the 185-unit
baseline stays `docs/reports/wave17-corpus/` unmoved: 2 Safe / 42
provisional Unsafe / 137 Unknown / 4 data-only, leak 272, CT 33/50.
Contract corpus 70 modules. Nothing is promoted.
- Scalar results (PIPE-SCALAR-RESULT): `let n = f(x);` binds a
  non-acquiring call's custody-free scalar result and a scalar local
  passes as a `Data` actual, in a `ValueId` namespace beside `ResourceId`
  that nothing converts; the acquisition adapter stays the one spelling of
  an acquisition. Evidence: `docs/reports/wave19-pipe-scalar-result/README.md`.
- Branching (IF-STATEMENT): `source.branch.v1` — `if`/`else`, a decidable
  condition tier, custody-join agreement (band Q044–Q046, sequential route
  only, zero new reserved words). The census corrected wave 18's premise:
  five of six semantic models already branched; only the cleanup lowering
  lacked a branch constructor, so both arms flow through the unedited
  checkers. Evidence: `docs/reports/wave19-if-statement/README.md`.
- Named waivers (UNSAFE-LINKAGE-ARTIFACT): level L1 of the linkage
  spectrum — a manifest-resident waiver publishes "unchecked X at ABI S
  from artifact Y@hash under stated ABI premises" over a total-or-reject
  manifest; the identity is a pin, not a probe; header disagreement
  refuses rather than withdraws (band Q039–Q042, no schemas bump).
  Evidence: `docs/reports/wave19-unsafe-linkage-artifact/README.md`.
- Typedef reader (CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF): the declared-ABI
  reader commits to function-pointer and aggregate-definition typedefs
  (`fnptr` normative in §C04.1, outside every custody admission), and the
  card's own premise is REFUTED by its measurement: the constructor stays
  `Unknown` — the ABI withdrawal was stacked on the importer's
  address-of-member refusal (residual `ABI-TYPEDEF-ADDRESS-OF-MEMBER`).
  BEFORE reproduced the wave-17 verdict artifact byte-identically; the
  msgpack contract measured zero payoff and is deliberately not landed.
  Evidence: `docs/reports/wave19-abi-typedef/README.md`.
- Operators (OR-AND-LOWERING): the operator table is superseded by user
  direction — logical or is `||`, logical and `&&`, both short-circuit,
  lowered onto the branch construct with byte-identical C against
  hand-lowered branches; single `|`/`&`/`^` are reserved for bitwise with
  actionable refusals, resolving the open bitwise-OR-symbol note. No new
  diagnostic code. Evidence: `docs/reports/wave19-or-and-lowering/README.md`.
- Route composition (STD-IO-COMPOSE): the extern and std-io routes compose
  as a checked model union selected after every concurrent route (Q043,
  `ComposedStdIo`); `EXTERN-SCALAR-LITERAL` lands, and a declared
  `succeeds INTEGER` outcome is verified by the emitted program rather
  than believed. The literal `value |> render |> log` chain compiles,
  emits its hand-written twin byte-identically and byte-compares per
  stream sanitized. Evidence: `docs/reports/wave19-std-io-compose/README.md`.
Merged gate at this checkpoint: 3,134 workspace tests (8 ignored),
122 provider tests, 193 Python tests, 85 Lean audits, both reevaluation
examples exit 0, fmt and clippy clean. Follow-on accounting lives in
`docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`, updated for
this wave).


Wave-19 partial increment (2026-09-10): two user-directed cards landed
through delegated worktrees and were composed with two Windows-validation
batches pushed concurrently from the maintainer's Windows host (the
non-fast-forward chain merged them; one fmt/clippy hygiene repair on a
Windows fixture was applied at integration with test behavior unchanged).
Corpus-neutral by construction; the baseline stays
`docs/reports/wave17-corpus/`. Nothing is promoted.
- Stream pipelines (PIPE-CHAIN): multi-stage `|>` lands as
  statement-sequence desugaring with hygienic per-stage temporaries —
  custody flows through the unedited checkers, so pipelines inherit
  linearity, leak and use-after-move checking; `PIPE-LET` closes; the
  wave-18 span blocker is retired by measurement (consecutive stage extents
  provably disjoint). `x |> f(a) |> g(b)` composes with real custody; sinks
  are final-stage-only; an unsafe extern stage deposits its
  UncheckedLinkage assumption through a pipeline. The literal
  `value |> render |> log` stays gated on two pre-existing measured
  boundaries recorded as PIPE-SCALAR-RESULT and STD-IO-COMPOSE.
  21 new audited theorems (`QCore.PipeDesugaring` 35 → 56). Evidence:
  `docs/reports/wave19-pipe-chain/README.md`.
- One printing route (TRACE-STDIO): all 46 compiler-synthesized print sites
  across the five C backends migrate onto the single
  `QCORE_IO_WRITE`/`QCORE_IO_FLUSH` route `std.io` lowers to, with no
  printed byte moved — every line-for-line reader keeps its evidence, the
  F02 emission digests are deliberately re-pinned with the relation
  re-verified, and 23 audited theorems (`QCore.SingleEmissionRoute`)
  characterize the single-route property both ways. The census corrected
  the premise: `examples/threads` verified nothing before and now carries
  five tests (compiled under -Werror, run sanitized, byte-compared against
  reference execution, README transcript checked against a real run); a
  source-level `print` in a threaded module is measured impossible and
  pinned. Windows residuals recorded (TRACE-STDIO-WIN-DIGEST/-WIN-RUN).
  Evidence: `docs/reports/wave19-trace-stdio/README.md`.
Merged gate at this checkpoint: 2,986 workspace tests (8 ignored),
122 provider tests, 193 Python tests, 80 Lean audits, both reevaluation
examples exit 0, fmt and clippy clean. The remaining wave-19 spine —
a source branching construct, the or/and lowerings onto it, and the
scalar-result binding gap — is recorded in the follow-on ledger.


Wave-18 delegated increment (2026-09-10): three user-directed cards landed
through delegated worktrees, merged onto main under the failure-gated chain
with one integration-time diagnostic-band relocation; this paragraph
supersedes nothing above and each card's record keeps its own boundaries.
Wave 18 is corpus-neutral by construction — the gateway, audit, contract and
provider surfaces are byte-untouched against the wave-17 tree, so the
185-unit baseline stays `docs/reports/wave17-corpus/` unmoved: 2 Safe /
42 provisional Unsafe / 137 Unknown / 4 data-only, leak 272, corpus 69
modules, CT 33/50, `qcore-schemas` 14.0.0. Nothing is promoted.
- Unchecked external linkage (UNSAFE-LINKAGE): `source.extern-call.unsafe.v1`
  lands — an external library with no contract at all links end to end behind
  `import std.extern.unsafe;` and `unsafe extern fn`, scalars by value in and
  a scalar or unit out, every pointer refused by name (an assumption that
  cannot be stated cannot be carried, so no custody crosses the boundary).
  `ExternAssumption::UncheckedLinkage { callee, signature }` rides the
  existing declared-assumption channel with three independent omission guards
  and a Lean totality theorem; `qcorec check` prints the premises, so a Safe
  obtained this way is never displayed bare. The Q023 default is pinned
  unmoved; the new `UncheckedLinkage` pass owns Q032–Q035 (spec §L21). The
  prior workflow's `NATIVE_LIBRARY_LINKAGE_V1.md` is adopted with provenance
  and reconciled as one rule: pointers cross exactly where a contract exists
  to state their premise; the manifest-resident artifact-identity entry is
  staged as UNSAFE-LINKAGE-ARTIFACT. 33 audited theorems, a fixed sanitizer
  run linking real uncontracted C. Evidence:
  `docs/reports/wave18-unsafe-linkage/README.md`.
- Proof-phase shortening (LEAN-PERF): the audit gate drops cold 121.4 s to
  61.0 s and incremental 36.5 s to 4.2 s with nothing proved changed — no
  theorem statement, domain, digest or pinned number moved, `native_decide`
  not introduced, the axiom closure pinned before/after. TaskTraceRelation
  shares one `cbv` expansion of its 3,025-trace domain (its residual ~27 s
  named the cost of the D3 claim); CapacityBoundary decides one fused
  predicate; `decide +kernel` was applied only where measured to help and
  reverted where it regressed. The audit phase parallelises behind a
  still-serialised build; there is deliberately no cross-worktree cache and
  CI stays cold, which is the recorded trust argument. Evidence:
  `docs/reports/wave18-lean-perf/README.md`.
- Pipe and the operator table (PIPE-OPERATOR): `|>` lands as a pure
  token-stream desugaring (first-argument insertion, left-associative) with
  byte-identical emitted C against the hand-written twin and linearity
  through pipelines inherited from the unedited checkers; a pre-existing
  divergent re-tokenization path in `source_extern` was found and repaired.
  Two measurements bound the rest: QCore has no `if` statement (Q500, zero
  occurrences in 294 source files) and no boolean expression tier, so the
  logical operators land as claimed tokens with actionable refusals — by
  decision, or is single `|` and and is `&&`, with `||` and a logical-position
  lone `&` redirecting, bare `&` reserved for a future binary operation, and
  bitwise-or's missing symbol flagged open, not answered. The Pipe pass owns
  Q036–Q038 after an integration-time band relocation (UncheckedLinkage
  shipped Q032–Q035 first; the T07/T09 precedent), and the adopted
  native-library design note moves its reservation to Q039+, closing a latent
  third collision. Multi-stage chains (PIPE-CHAIN) and the or/and lowerings
  wait on a source branching construct — the named wave-19 prerequisite.
  35 audited theorems. Evidence: `docs/reports/wave18-pipe-operator/README.md`.
Merged gate at this checkpoint: 2,959 workspace tests (8 ignored),
122 provider tests, 193 Python tests, 79 Lean audits, both reevaluation
examples exit 0, fmt and clippy clean. Follow-on accounting lives in
`docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`, updated for
this wave).


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

Wave-14 delegated increment (2026-09-09): six bounded follow-on cards landed
through delegated worktrees, merged onto main under the verified failure-gated
chain discipline and closed by one integrator corpus consolidation; this
paragraph supersedes nothing above and each card's record keeps its own
boundaries.
- Sanitized protection growth (CT34-SANITIZED): GrowProtection renders in the
  C backend admission-gated against the certificate's own record, with the
  original refusal's stated ground corrected (a C mutex protects by
  discipline for every generation); CT34 joins the harness-gated sanitized
  set — 4 of 32 — with the artifact rule made derived and a new
  accept_growth expectation. 25 audited theorems. Evidence:
  `docs/reports/remaining-ct34-sanitized/README.md`.
- Production logging contracts (PROD-CONTRACTS): three measured corrections
  to the brief — flb_trace/flb_error are macros the corpus never names (the
  real callee flb_log_print is contracted with the first tail written
  against a read body), the site count is 21 in one unit, and C6's
  discharge does NOT reach production, its sole remaining blocker isolated
  by a one-withdrawal experiment to six named uncontracted neighbours.
  Corpus 61 to 64; the watched provisional-Unsafe row did not move; four
  functions read and deliberately left undeclared. Evidence:
  `docs/reports/remaining-prod-contracts/README.md`.
- The literal-argument clause (HASH-CLAUSE): `resource_literal_argument` is
  a real §C04.5 clause (name-keyed, requires-positioned, both deviations
  recorded); the predicted schema change avoided by measurement
  (byte-identical source-effects — no digest moves); the comment carrier
  retired loudly at three layers with parity measured while both carriers
  lived and pinned to survive the directive's removal. Ten new theorems.
  Evidence: `docs/reports/remaining-hash-clause/README.md`.
- Buckets and mixed joins (HYGIENE-14): the unmapped bucket counts derive
  from the manifest with sum-refusals in both derivations (a duplicate
  triage-row bug found and repaired in passing), labels honestly frozen as
  T00's archive; the mixed-join liveness condition is exactly {RELEASED} and
  {RELEASED, ESCAPED} over the modelled bits, enumerated and decided, three
  theorems retired by name, LEAK-ORDER untouched. Evidence:
  `docs/reports/remaining-hygiene-14/README.md`.
- Frame verification task (P04-FRAME): the frame helper — and by census
  correction the list-unlink and field-destructor checks too — gain
  first-class verification tasks wrapping unedited evaluators in full
  compile-task identity; the shared obligation-id is proved sound only via
  the identity triple (P04-FRAME-OBLIGATION-ID); memory_close's move is
  honestly sized out as P04-COMPOSE-MEMCLOSE. 27 audited theorems;
  `qcore-schemas` to 11.0.0. Evidence:
  `docs/reports/remaining-p04-frame/README.md`.
- file_close compositions (P04-COMPOSE-A): both entry points travel with
  structural recursion (a boxed nested family input; measured depth 2
  compositions / 4 task levels; 171 mutation rows including the forged
  leaf under an honest middle), the fifth vocabulary's projection moving to
  v2 with both directions refused; a fail-closed gate its own change nearly
  weakened was repaired at the original site, and a missing pin its module
  doc claimed was written rather than the claim lowered. 15 new audited
  theorems. Evidence: `docs/reports/remaining-p04-compose-a/README.md`.
- Corpus consolidation: one integrator measurement
  (`docs/reports/wave14-corpus/`) re-pins the baseline — one attributed
  diagnostics unit (PROD-CONTRACTS' −58/+17 leaves), the schemas-11.0.0
  package residue, and the deliberately grown contract inventory verified
  by direct comparison past a recorded diff-tool blind spot
  (DIFF-TOOL-CONTRACTS). Zero function status moves. Spec §C05's stale
  selection sentence is corrected to the pinned authority.

Nothing is promoted by this increment: raw Cycle 19 units stay 2 Safe /
42 provisional Unsafe / 137 Unknown plus four data-only units on the freshly
re-pinned wave-14 baseline, the contract corpus is 64 modules by deliberate
pin updates and full language conformance stays false. CT coverage stays
32/50 mapped with gated sanitized-native at 4 of 32. `qcore-schemas` is
11.0.0; `qcore-compiler` stays 0.4.0. Merged gate at this checkpoint: 2482+
workspace tests across the per-merge gates with the final tree at 64 Lean
audits, 102 provider tests, 131 Python tests, both reevaluation examples
exit 0, fmt and clippy clean, and all five provider packages rebuilt and
verified at schemas 11.0.0. Follow-on accounting lives in
`docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`, updated for
this wave).

Wave-15 delegated increment (2026-09-10): six bounded follow-on cards landed
through delegated worktrees, merged onto main under the verified failure-gated
chain discipline and closed by one integrator corpus consolidation; this
paragraph supersedes nothing above and each card's record keeps its own
boundaries.
- Label provenance (CT-LABELS): unmapped bucket labels are dual-published
  with their source — 5 of 18 measured by later lanes, 13 honestly
  t00-archive — via a forward-applied evidence-record field with five
  verbatim-cited migrations; the frozen archive's in-place mutation by an
  earlier lane is recorded as pending hygiene. Evidence:
  `docs/reports/wave15-ct-labels/README.md`.
- Six callee contracts (NEIGHBOURS): the production blocker closes by
  declaration, not discharge — all six read against pinned bodies,
  flb_input_chunk_create moves 36 to 21 diagnostics with every
  callee-naming refusal gone, and production Safe stays out on 21 residuals
  in two named checker families. flb_time_now could not be declared
  faithfully (double), exposing that the loader never compares a fixture's
  declared ABI type against the C prelude; floats join the ABI table
  writable-only. Corpus 64 to 67. Evidence:
  `docs/reports/remaining-neighbours/README.md`.
- Enumeration leftovers (E01-TAIL): two live defects repaired — a #line
  directive in analysed source could move the audited scope in both
  directions (one fail-open), now attributed by include-depth flags no
  directive can move; and the partial analyzer silently dropped selected
  definitions, so Safe now requires full analysed coverage at the
  certificate gate. The cost residual measured at 53 ms and the cache
  declined, with the two genuinely larger redundancies named. 31 audited
  theorems. Evidence: `docs/reports/remaining-e01-tail/README.md`.
- Derived-text capability (MEMCLOSE): memory_close moves through the
  process boundary with the derived text never travelling — the core
  derives from its own recipe and compares field-for-field, with the
  equal-bytes/collapsed-identity behavior pinned and four refusal
  directions; the extension is additive at the wire. filesystem_chunk_close
  becomes composable-now. `qcore-schemas` 12.0.0; ChunkIO 0.11.0;
  32 audited theorems. Evidence:
  `docs/reports/remaining-memclose/README.md`.
- Literal-premise boundaries (HASH-REFINE): TYPE admitted on a closed C11
  argument (integer-constant 0 is the null pointer constant); LITERAL
  widened only structurally (matched parens, the 23-entry suffix table)
  with octal's refusal load-bearing on both sides and the declared space
  made canonical; zero corpus reach measured across all 50 sites. 28 new
  audited theorems. Evidence:
  `docs/reports/remaining-hash-refine/README.md`.
- Liveness quartet (BORROW-RESIDUALS): PRESTATE lands (a call releasing and
  borrowing one settled identity in its own declared effects was
  Safe-with-no-diagnostics — refused, on the measured ground that no
  ordering exists between a callee's own declared effects); PLACES split by
  measurement; OUT and ASSUMPTION are reasoned refusals with every ground
  measured; corpus byte-identity proved by a toggled-rule control. 21 new
  audited theorems. Evidence:
  `docs/reports/remaining-borrow-residuals/README.md`.
- Corpus consolidation: one integrator measurement
  (`docs/reports/wave15-corpus/`) re-pins the baseline — three diagnostics
  units, exactly NEIGHBOURS' attributed set, zero function status moves,
  the package residue, and the grown contract inventory; the
  Released+Borrowed census re-checked on the 31-declaration tree. Process
  note adopted: agents share the session scratchpad, so measurement outputs
  now belong in worktree-private directories (a mid-run overwrite was
  caught and re-measured by HASH-REFINE).

Nothing is promoted by this increment: raw Cycle 19 units stay 2 Safe /
42 provisional Unsafe / 137 Unknown plus four data-only units on the freshly
re-pinned wave-15 baseline, the contract corpus is 67 modules by deliberate
pin updates and full language conformance stays false. CT coverage stays
32/50 mapped. `qcore-schemas` is 12.0.0; `qcore-compiler` stays 0.4.0.
Merged gate at this checkpoint: 2567+ workspace tests across the per-merge
gates with the final tree at 67 Lean audits, 108 provider tests, 146 Python
tests, both reevaluation examples exit 0, fmt and clippy clean, and all five
provider packages rebuilt and verified at schemas 12.0.0. Follow-on
accounting lives in `docs/reviews/r0_3/acceptance_matrix.json`
(`named_followups`, updated for this wave).

Wave-16 delegated increment (2026-09-10): seven bounded cards landed through
delegated worktrees, merged onto main under the verified failure-gated chain
discipline — which caught one predicted semantic conflict and forced an
explicit composed-tree reconciliation — and closed by one integrator corpus
consolidation. This paragraph supersedes nothing above and each card's record
keeps its own boundaries. It is also the first increment written under the
compacted convention: `AGENTS.md` and `CLAUDE.md` now carry only the ledger,
the invariants, the standing constraints and the gate, and this file is the
only place per-increment history lives.
- Agent-doc compaction (DOCS-COMPACT): `AGENTS.md` 2,423 lines to 361 and
  `CLAUDE.md` 2,150 to 86, with every unpreserved paragraph moved here
  verbatim by line range and 28 of 28 moved segments byte-verified present.
  It changed **no fact**, but re-read every headline number from its source
  rather than carrying the prose, correcting three that had drifted (the
  repository map still said 60 contract modules and schemas 3.0.0; the
  reevaluation bullet still said CT 39/50 unmapped). The central obligation
  was the constraint sweep: 87 candidate imperative sentences deduplicated
  into 25 numbered standing subsystem constraints in `AGENTS.md`, each
  carrying the card that established it, with six working-practice rules left
  in `CLAUDE.md`. The sweep is regex-driven plus a manual pass — thorough,
  not provably exhaustive — and the complete original text remains in git
  history and, for every unpreserved paragraph, verbatim here. Evidence:
  `docs/reports/wave16-docs-compact/README.md`.
- Ledger and tooling hygiene (HYGIENE-16): three tally-neutral follow-ons
  close and none moves a number. `D02-BUCKETS-STALE` was measured **already
  repaired** — HYGIENE-14 made the counted set structural and CT-LABELS made
  the label derived, so the committed acceptance record is a fixed point of
  its own writer — and the card supplied the regression a tree-reading test
  cannot show: the literal sum-22 shape re-inflated into a scratch record and
  refused by field name. Extending the same check to the companion
  `newly_decidable_not_mapped_reasons` map, in both derivations, immediately
  found one live staleness (the matrix still explained why CT39 was unmapped,
  four waves after wave 8 mapped it), dropped here.
  `CT-LABELS-T00-REVERT` restores card T00's frozen `coverage.json` to its
  pre-CTWIN bytes byte-exactly, including the original missing final newline;
  CT40/CT41/CT42's current labels come from the wave-9 lane through
  CT-LABELS' dual publication and T00's original text is visible beside them
  instead of overwritten — verified neutral three ways, one of them a proof.
  `DIFF-TOOL-CONTRACTS` was measured **worse than recorded**: `contracts.json`
  was not diffed at all, in any archived diff document. The fifth artifact now
  itemizes contract-module adds, removals and content changes, both embedded
  JSON documents and every remaining top-level key; both recorded blind-spot
  cases reproduce on committed bytes and the four pre-existing entries are
  byte-identical to the two archived documents the pre-change tool produced.
  14 audited theorems (`QCore.BucketTally`); Lean judged not applicable for
  the diff tool with the reasoning recorded. One further finding fell out of
  regenerating: `build_matrix.py`'s writer had never emitted the committed
  matrix's byte form, so every "adopt the regenerated matrix" commit
  re-serialised its output by hand; the writer now emits the committed form.
  Evidence: `docs/reports/wave16-hygiene/README.md`.
- Chained tail actuals (NEIGHBOURS-CHAINED-TAIL): the one refusal wave 15
  isolated falls. A tail actual at a **declared**
  `resource_variadic_data_tail()` position that is exactly one admitted
  bounded typed access chain is data (clause T5) — the same predicate, over
  the same declaration facts, that already decides a declared `Borrowed`
  actual, so the position is the only difference and the position is read
  from the callee, never from the actual. Three measured corrections to the
  card premise: the cause was **not** a pointer-typed final field (a
  depth-one *scalar* tail actual refused identically) but premise T2 plus the
  lexical gate firing ahead of it; the requested closure over the derived
  value is structurally unnecessary because a tail actual binds no name,
  while the closure over the **base** is what refuses a released or
  body-acquired root; and the pinned NEIGHBOURS headline moved in the
  *admitting* direction, re-pinned deliberately with all three load-bearing
  negative halves kept and a fourth added. Corpus effect: exactly one leaf
  removed, `flb_input_chunk_create` L19581, the line NEIGHBOURS predicted.
  28 audited theorems (`QCore.ChainedTailActual`); CHAINED-TAIL-NO-LIVENESS
  is recorded as inherited, not created. Evidence:
  `docs/reports/remaining-neighbours-chained-tail/README.md`.
- Opaque aggregate members (ALIAS-AGGREGATE): the twelve `unresolved
  resource identity` residuals NEIGHBOURS left on production
  `flb_input_chunk_create` are measured to have **no aliasing cause at all**
  — each is the reaching state of an earlier refusal, and the twelve have
  four roots, attributed by withdrawal experiments rather than argued. The
  root of eight of them **corrects card O04-B08's own record**: production
  declares `void *chunk`, not the reduced fixture prelude's `struct cio_chunk
  *chunk`, so that card's flagship store never reached production at all —
  `NEIGHBOURS-FIXTURE-PROXY` caught on the statement it was named for. A
  third member class lands, admitted as a store destination and **refused as
  a bound read in every position**, with the exact-tag value premise replaced
  by "a declared record pointer"; the asymmetry is the whole soundness
  argument. `legacy_c.rs` is byte-unchanged, so `C02-P-ALIAS-LEAK-ORDER` is
  undisturbed by construction. The constructor moves 21 to 12 diagnostics;
  corpus reach is five statements in four units, with **all four added leaves
  refusals replacing refusals**. Fourteen new audited theorems across two
  existing namespaces, none retired. Evidence:
  `docs/reports/remaining-alias-aggregate/README.md`.
- Declared/C ABI agreement (LOADER-ABI): a declared contract whose parameter
  or result ABI projections the analysed translation unit's own C declaration
  contradicts is now **withdrawn for that unit** — every call site refuses
  with one of eight named keys and the name leaves the environment the
  derivation seeds — closing the fail-open NEIGHBOURS recorded, where the
  loader checked those spellings against its own table, against nothing else,
  and then discarded them while its admissibility rules were keyed on exactly
  what it discarded. Both sides are read from what a certificate already
  holds, so W4 derivation, W4 validation and certificate rechecking reproduce
  the decision; withdrawal is proved monotone in the refusing direction. The
  sweep found **eight unfaithful declarations**: `flb_sds_create_len`'s `len`
  declared `usize` against C's `int` (the only production finding, corrected
  in `contract/qcore`) and seven fixture declarations, the last of which
  (`flb_error` at a call site's arity against `(const char *, ...)`) closes
  `O04-ARMS-VARIADIC` and was load-bearing for five cards' published fixture
  results. 38 audited theorems (`QCore.DeclaredAbiAgreement`); spec §C04.1
  gains the agreement requirement. **The composed-tree reconciliation is part
  of the record**: the failure-gated chain caught eleven pins, and on the
  composed tree all eleven revert to their pre-LOADER-ABI values — now
  carried by a *faithful* declaration, because CHAINED-TAIL's clause T5
  admits the same statement honestly at the declared data tail the correction
  creates. Neither ground is weakened by the other, four further theorems tie
  the two namespaces, and the two probes composition made undiscriminating
  are retired **by name** rather than left to pass vacuously.
  `O04-TRACE-REACH` still stands: no production claim is restored. Evidence:
  `docs/reports/remaining-loader-abi/README.md` (§10 for the reconciliation).
- Publication custody (CTWIN-PUBLISH): the wave-9 `CT-CTWIN-PUBLISH` hazard
  is reproduced and closed — a store of an owned pointer into a file-scope
  object was `Safe` with no diagnostic, and so was publishing an
  already-released one, because the importer modelled the file-scope object
  as one more local name. The repair **refuses rather than escapes**, and the
  brief's escape proposal is **disproved by measurement**: escape discharges
  the residual, so the publish-without-release body — measured `Unsafe` at
  base — would have been *promoted* to `Safe`. Both directions are withdrawn
  instead, the leak claim retired through the existing unresolved-effect
  mechanism, which is a downgrade of a provisional `Unsafe` and never a
  promotion. Destinations are file-scope objects the body does not shadow
  plus its own `static` locals (added by measurement, not in the brief); the
  value half fires only on an identity the body itself holds, so borrowed
  publication, null stores and scalar stores stay admitted, each argued.
  Corpus reach is **zero**, with a toggled-rule probe measuring *which*
  conjunct makes it so: with the value conjunct disabled, six librdkafka
  units move at real static-storage destinations, so the destination scan is
  not vacuous. `CT-CTWIN-SYNC` is converted from an omission into an asserted
  decision — C11 `volatile` is not a synchronization primitive, so a
  projection deriving no synchronization fact must answer identically, pinned
  on the whole verdict in both directions. 30 audited theorems
  (`QCore.PublicationCustody`); CT40 candidate rows recorded without touching
  the manifest, with `_Atomic` measured *not* load-bearing. Evidence:
  `docs/reports/remaining-ctwin-publish/README.md`.
- Filesystem-close composition (P04-COMPOSE-FS): the fourth composition
  module moves through the process boundary — the first with **two derived
  units in one question** and the first whose **every** prerequisite is
  itself a composition, checked over two different translation units. The
  measured consequence is recorded rather than papered over: this family
  cannot carry the memory family's single-unit base tie, so the unit tie is
  pinned in *both* directions and the backend prerequisite is tied by
  identity, revision and prototype with linker resolution published as a
  premise (`P04-COMPOSE-FS-CROSS-UNIT`). No composed result and no
  prerequisite conclusion travels at any of the three levels; parity with the
  retained in-process route is byte-identical on the serialized certificate,
  and the two routes now call one decision procedure so they cannot drift. A
  118-row mutation matrix, an 18-row provider degeneracy matrix and 9
  request-gate rows all refuse. `nested()` returns a vector because one slot
  could not publish two nested questions honestly; that and three enum
  variants make `qcore-schemas` **13.0.0** with existing wire bytes and every
  existing task digest unchanged. ChunkIO package 0.12.0; 35 audited theorems
  (`QCore.TwoDerivationAdmission`). `unsupported_close` is now composable-now
  and honestly sized out as `P04-COMPOSE-UNSUPPORTED` — a fifth proposal
  record carrying the symbolic other-backend class, not an extension of this
  one. Evidence: `docs/reports/remaining-p04-compose-fs/README.md`.
- Corpus consolidation: one integrator measurement
  (`docs/reports/wave16-corpus/`) re-pins the baseline. The diagnostics diff
  is 4 added / 14 removed across four units and is **triangulated in both
  directions** against the two gateway cards' archived AFTER measurements —
  each was taken without the other's diff — with zero unattributed leaves:
  the union of CHAINED-TAIL's one leaf and ALIAS-AGGREGATE's seventeen equals
  the composed measurement exactly, and the composed set minus either card's
  is the other card's. The composed `flb_input_chunk_create` count is
  **measured at 11** (ALIAS-AGGREGATE's 12 minus CHAINED-TAIL's L19581), and
  it stays `Unknown`. `chunk-contracts.json` is byte-identical; the one
  changed contract module is LOADER-ABI's correction, verified through
  HYGIENE-16's new itemizer rather than by hand; the 36 `source-effects`
  leaves are LOADER-ABI's 33 identity digests plus P04-COMPOSE-FS's three
  `provider_version` leaves, with no verdict, coverage shape or bounds value
  moving; and the `source.json` residue is 14 `__FILE__` literals and their
  derived extents. CTWIN-PUBLISH's, HYGIENE-16's and DOCS-COMPACT's
  byte-identity claims all hold on the composed tree. `named_followups` moves
  25 to 26 and `AGENTS.md`'s standing constraints 25 to 28 (the LOADER-ABI
  withdrawal rule, the fixture-prelude rule ALIAS-AGGREGATE's correction
  forces, and the publication-store withdrawal rule), with constraints 21 and
  22 rewritten because HYGIENE-16 closed what they described. One recorded
  generator defect fired at the regeneration and was handled rather than
  absorbed: `build_matrix.py` drops `newly_decidable_not_mapped_reasons` on
  every run (`D02-GEN-CTREASONS`), so the map is carried forward by a script
  that **refuses** a row describing a case the manifest reports as mapped —
  the staleness HYGIENE-16 found by hand, now caught mechanically at the one
  point a regeneration would lose it. The follow-on stays open: that is a
  guard at the carry, not a derivation.

Nothing is promoted by this increment: raw Cycle 19 units stay 2 Safe /
42 provisional Unsafe / 137 Unknown plus four data-only units on the freshly
re-pinned wave-16 baseline, with zero unit and zero function status moves and
leak unchanged at 271; the contract corpus stays 67 modules and full language
conformance stays false. CT coverage stays 32/50 mapped. `qcore-schemas` is
13.0.0; `qcore-compiler`, `qcore-verifier-host` and `qcore-verification` stay
0.4.0. Merged gate at this checkpoint: 2,746 workspace tests (8 ignored),
115 provider tests, 161 Python tests, 72 Lean audits, both reevaluation
examples exit 0, fmt and clippy clean in both workspaces, and all five
provider packages rebuilt and verified at schemas 13.0.0 / ChunkIO 0.12.0.
Neither recorded load-flake (`vp04_vp07_vp08`, the confined-fixture write
test) was hit at the consolidation gate; both assertions stay unweakened.
Follow-on accounting lives in
`docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`, updated for
this wave).

Wave-17 delegated increment (2026-09-10): eight bounded follow-on cards landed
through delegated worktrees, merged onto main under the verified failure-gated
chain discipline and closed by one integrator corpus consolidation. This
paragraph supersedes nothing above and each card's record keeps its own
boundaries.
- The constructor's last two roots (CONSTRUCTOR-RESIDUALS): card
  ALIAS-AGGREGATE's two reasoned refusals are dispositioned, one by
  implementation and one by correcting its own premise. **ERRNO**: a
  dereference of a neutral call's result — `errno`, spelled
  `(*__errno_location ())` after preprocessing — is admitted as one outer
  argument under a **refined** proved sentence rather than a violated one,
  because card W4-EXPR-POS's *no pointer-valued operand* cannot reach a
  pointer-valued call; the replacement is stated over what the projection
  tracks (an admitted dereference *binds, releases, escapes and addresses
  nothing*), its four conjuncts discharged by N6, N7, N9 and a new
  scalar-pointee N10. `__errno_location` and `flb_errno_print` are declared
  against read bodies; measured reach is 18 sites for the admission and 24
  more for the declaration alone, across 13 units. **PUBLICATION**: measured,
  `mk_list_add` needs no publication mode at all — card HASH-CONTRACTS had
  already settled the question and the read body frees nothing and moves no
  obligation, so plain `resource_borrowed` is faithful and the retention stays
  unmodeled (B03/R13). The real blocker on the two statements the card could
  not admit is card LOADER-ABI's declared-type reader (a function-pointer
  typedef; a union typedef), named `CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF`, and a
  withdrawal experiment pins what remains: without those three statements the
  constructor reports **0** diagnostics and discharges all six exits. It moves
  **11 → 6** diagnostics with **5 of 6** exits discharging and stays
  `Unknown`. One **pre-existing fail-open** was found and repaired: a released
  identity at an admitted *nested* call's borrowed actual was `Safe` with no
  diagnostic, because card BORROW-LIVENESS's precondition read only the
  enclosing call's actuals; the repair is a set inclusion and
  `C02-P-ALIAS-LEAK-ORDER` is undisturbed by construction. 29 audited theorems
  (`QCore.DereferencedResult`); contract corpus **67 → 69** by two deliberate
  additions. Evidence:
  `docs/reports/remaining-constructor-residuals/README.md`.
- Declared/derived custody-mode agreement (LOADER-ABI-MODES): a declared
  contract whose custody **duty** the validated W4 summary of the callee's own
  body in the analysed unit *contradicts* is now withdrawn for that unit —
  every call site refusing with `mode-parameter-contradiction` or
  `mode-result-contradiction` — closing the fail-open card LOADER-ABI
  recorded, where types were compared against the source and modes against
  nothing. The evidence was already computed and already reported
  (`SummaryConflict` makes the *defining* function `Unknown`) while the
  declaration went on governing every call site; **three `Safe` callers**
  resting on that — a double release, a hidden leak and a discarded
  acquisition — are reproduced and closed. The relation is proved to *be* the
  duty inequality rather than sampled, and deliberately stops short of
  refinements inside one duty: that boundary is not academic, because the
  census measures **every** derivable comparison in the pinned corpus as a
  nullability refinement in three production families (113 rows), so
  withdrawing on inequality would have withdrawn a release contract in every
  unit including those headers. Withdrawal never installs the derived summary
  in the declaration's place, an inferred summary being possibly weaker than
  the declaration it convicts; a fail-closed pin from card C03-NFR caught the
  first implementation destroying its own evidence and was repaired rather
  than weakened. Census: 598 undefined / 484 no-summary / 113 disagreed /
  **0 contradicted** over all 185 units, of which 8 rows are inside the
  audit's analysed function set and all 8 read `no-summary` — so the corpus
  was measured in **both** directions and all five artifacts are byte-identical
  between them. 36 audited theorems (`QCore.DeclaredModeAgreement`); contract
  spec §C04.1 gains the mode-agreement requirement. Evidence:
  `docs/reports/remaining-loader-abi-modes/README.md`.
- Publication residuals (CTWIN-RESIDUALS): the four shapes card CTWIN-PUBLISH
  left open are measured and closed, and the brief's soundness ordering is
  **refuted by measurement** — INDIRECT, named the likeliest launder, is the
  one shape with *no* false `Safe` at all (nine indirect shapes probed, all
  already refused), while three others each carried a live publish-then-free
  `Safe` with no diagnostic. The sharpest, `CTWIN-PUBLISH-BORROWED`, is not the
  lifetime question wave 16 supposed but a custody launder across the
  **summary** boundary: a helper publishing a borrowed formal certified, so W4
  published `Borrowed` for it and a caller that acquired, called the helper and
  released certified too. The exclusion is withdrawn, on the argument that
  `Borrowed` asserts the caller's obligations are unchanged and a publication
  falsifies exactly that, with both faithful alternatives proved to promote in
  the mirror direction. `CTWIN-PUBLISH-OUT` is withdrawn the same way — a
  declared `Out` slot naming such an object is the `Acquire` arm seen from the
  callee's side — and a latent false `Unsafe` closes with it.
  `CTWIN-PUBLISH-INDIRECT` has no launder to close, so its residual
  `violation` **claim** is withdrawn instead, each statement keeping the
  refusal it already had. `CTWIN-PUBLISH-SCAN` narrows: `_Atomic ( type-name )`
  is a specifier, not a declarator, and the parenthesised spelling — CT40's
  literal one — *was* hiding its destination, correcting CTWIN-PUBLISH's
  "`_Atomic` is not load-bearing" note, which had measured only the postfix
  spelling; monotonicity is preserved and proved. Two theorems and one test
  are retired by name and one C03 pin moved earlier, each keeping its
  load-bearing half. `QCore.PublicationCustody` grows 30 → **50** audited
  theorems. Corpus reach is **zero**, measured twice. Evidence:
  `docs/reports/remaining-ctwin-residuals/README.md`.
- Extern released and reallocated modes (EXTERN-MODES): two of the five open
  `source.extern-call.v1` call modes become callable, each at a measured
  boundary. `resource_released(p, nonnull)` is the mode the error arm was
  already describing — §C04.2 separates discharging an obligation from
  transferring it, and card EXTERN-ERR-REL recorded that the profile could
  spell only the transfer — so the caller-side ledger delta is unchanged and
  nothing about leak accounting moves, but a release is now published as a
  release and premised by name. `resource_return_reallocated(p, nullable)`
  over an `Owned` subject is the one departing argument an acquiring call may
  carry, because §C04.3 states its failure arm where ordinary declarations are
  silent; the subject therefore stays live on the error arm and must be
  dispositioned there, which is the unweakened abandonment rule.
  `resource_removed` stays refused on a **measured ground** — a removal names a
  child the profile stopped tracking when it was stored, and card EXTERN-F1
  premised that residual rather than modelling an index, so admitting it would
  mint a caller identity out of a premise — and the sentinel/errno adapter is
  recorded as a design boundary with its three constraints. No diagnostic code
  and **no reserved word** were added: `move NAME` spells all four departing
  modes and the declaration decides which ran. Two Lean theorems retired by
  name with their load-bearing halves kept; three new fixed ASan/UBSan/LSan
  fixtures link a real C `free` and a real `realloc`, including the preserving
  arm where a double free would be reported. Evidence:
  `docs/reports/remaining-extern-modes/README.md`.
- ChunkIO symbolic other-backend classification (P04-COMPOSE-UNSUPPORTED): the
  fifth and **last reachable** composition module moves through the process
  boundary, closing the ladder. It is the first family that imports **no body
  of its own** — its whole question is the nested filesystem question, verbatim
  — the deepest question the vocabulary carries (four composition levels, all
  re-established by the core), and the only one whose published conclusion is
  an explicit `Unknown` that no proposal can move, proved as such. Its reading
  is over a **class**, not a value: the excluded class is fixed jointly by the
  two probes *and the wrapper's own guards*, and a claimed class that leaves a
  guard undecided is refused rather than sampled — a measured correction to the
  card's own first draft, caught by a failing row and kept. Parity with the
  retained in-process route is byte-identical on the serialized certificate
  across three excluded-constant pairs; a 69-row core mutation matrix, a 31-row
  provider matrix and 12 route-boundary refusal rows all refuse, including both
  masquerade directions and a falsified prerequisite contract *two* levels
  down. Three enum variants make `qcore-schemas` **14.0.0** with existing wire
  bytes and every existing task digest unchanged; ChunkIO package **0.13.0**;
  37 audited theorems (`QCore.SymbolicClassAdmission`). Two modules stay behind
  the sealed `SourceMatch` boundary, re-measured rather than assumed: they need
  a reviewed-body attestation the core can re-derive, not a bigger vocabulary.
  Its headline boundary is named rather than blurred
  (`P04-COMPOSE-UNSUPPORTED-REACHABILITY`): the route establishes what happens
  *if* a chunk's stream carries a discriminant outside both constants, and
  **nothing** about whether any caller can produce one. Evidence:
  `docs/reports/remaining-p04-compose-unsupported/README.md`.
- Standard output modules (STD-MODULES): the census contradicts the card's
  premise and is the result — QCore source has **no** printing, logging,
  formatting or IO construct and never had one (21 statement kinds, none
  writing; no string or char type; zero string and character literals outside
  comments across 290 source-side files), so the migrated implicit-use count is
  **0**. What looked implicit is two other things, both recorded:
  compiler-synthesized trace instrumentation in four C backends, read back for
  translation validation, and an `Effect::Io` label stamped on every extern
  call regardless of what the callee does — a capability *name* with nothing
  gating it. With nothing to migrate, `std.fmt`, `std.io` and `std.logger` are
  **provided** instead, as `source.std-io.v1`: two renderings, a line to
  stdout, a level-tagged line to stderr and a `flush` barrier, each gated by
  its own module through the existing `Q014`/`Q015` and refused without it,
  with `Q028`–`Q031` under a new `StandardIo` pass through the §L21 process.
  The v1 effect discipline is **structural rather than checked**: the operand
  type has two constructors and neither can denote a resource, so a rendering
  cannot consume, release, store or retain custody, and the derived row is
  exactly `{Io}`. Two sanitized native runs match the reference **per stream**;
  no interleaving between the two streams is modeled or claimed. The
  amendment's asynchronous logger is refused on a **measured** blocker rather
  than faked — the task ledger's only parent-to-child custody edge is at
  creation and the checked mutex backend emits one thread, so a bounded queue
  cannot be modeled honestly — and only the barrier half landed, as sink
  handoff (`fflush`), explicitly not durability. `error` was measured to
  collide with the reserved error-type keyword, so the severity is spelled
  `fatal` (the `work` → `effort` precedent). 45 audited theorems
  (`QCore.StdIo`), no new axiom. Evidence:
  `docs/reports/wave17-std-modules/README.md`.
- Wave-17 CT lane (CT40-LANE): **CT40 maps on the `legacy_c` route, moving CT
  coverage 32/50 → 33/50**, after card CTWIN-PUBLISH closed the gateway hazard
  that had left it unmapped. Every parked candidate row was re-measured on the
  current tree's fixture bytes and two corrections found: the parked diagnostic
  string used `unknown_alias` where the harness renders `unknown-alias`, and
  one control was not enough. The mapping rests on the construct CT40's
  `expected` text names — the publication protocol, which the gateway names
  verbatim in its own diagnostic — and explicitly **not** on the two its title
  names: `_Atomic` is pinned non-load-bearing by a control byte-identical to
  the case, and "prematurely freed" by a control that publishes with no release
  and is refused just the same. `native_sanitized` is the structural
  `not_applicable` its route prescribes; sanitized-native stays 4 and proof 0,
  so the grown denominator is mapping, not conformance. CT41 and CT42 stay
  unmapped with blockers **re-measured** rather than inherited. One mechanism
  change: mapping a case whose bucket an earlier lane recorded no longer
  requires editing that lane's archived record — the mapping lane declares a
  `retired_unmapped_reasons` row in its own, checked total-or-reject in both
  derivations across seven refusal directions, so the wave-9 record stays
  byte-identical. 24 scratch mutations, all refused. Evidence:
  `docs/reports/wave17-ct40/README.md`.
- Ledger and archive hygiene (GEN-CTREASONS): the CT ledger's
  `newly_decidable_not_mapped_reasons` map is **derived** rather than carried,
  closing `D02-GEN-CTREASONS`, which HYGIENE-16 had recorded as *not*
  derivable. Measured, the map is the companion prose of a list the generator
  already derives, so its keys are that list's ids by construction — which
  makes both staleness classes structurally impossible rather than guarded, and
  the second was found by doing it: beside HYGIENE-16's CT39 row the committed
  map carried a **CT11** row explaining a case its own published list does not
  name, invisible to the wave-16 carry guard. The carry step
  `readd_ct_reasons.py` now retires itself loudly through its own tripwire,
  asserted by name, with its archived bytes untouched. 15 audited theorems
  (`QCore.CtReasons`). `DIFF-TOOL-CONTRACTS-BACKFILL` closes over the whole
  schema-1 consolidation chain: seven archived diff documents gain a
  `contracts` section regenerated from their own committed artifact pairs,
  append-only behind a `backfilled_sections` record and verified three ways,
  including against the pre-backfill blobs in git. Each document is its own
  **oracle** — the four entries it already carried must reproduce
  byte-identically or nothing is written — and that refusal is exercised. The
  card premise is corrected by measurement (the wave-16 diff already carried
  the section), and the backfill supplies what the hand comparisons could not:
  HASH-CLAUSE's *content* change in wave 13 → 14, and a measured zero contract
  movement in waves 9 → 13. Evidence:
  `docs/reports/wave17-gen-ctreasons/README.md`.

- Corpus consolidation: one integrator measurement
  (`docs/reports/wave17-corpus/`) re-pins the baseline. Three of the eight
  cards touched the gateway's decision surface and **only one moved a leaf**,
  so the attribution is by identity rather than by argument: the composed
  `diagnostics.json` is **byte-identical** to CONSTRUCTOR-RESIDUALS' archived
  AFTER, and `triangulate.py` asserts on committed bytes that the composed diff
  (12 added / 60 removed across 21 units) equals that card's exactly, with zero
  unattributed leaves — which is also what makes CTWIN-RESIDUALS' and
  LOADER-ABI-MODES' zero-reach claims hold on the composed tree. The composed
  `flb_input_chunk_create` count is **measured at 6** with 5 of 6 exits
  discharging, and it stays `Unknown`. **One pinned number moves deliberately:
  leak 271 → 272**, at `flb_input_chunk_get_direct_routes` L19399 — a movement
  in the *accusing* direction on a function and a unit that were already
  `Unsafe`, the seventh instance of a claim the same function already published
  six times, surfaced because a neighbouring refusal that hid it was admitted.
  It is not a promotion, it stays provisional in the documented sense, and it
  is recorded as `CONSTRUCTOR-RESIDUALS-LEAK-271`; standing constraint 7 is
  updated to 272 with that record named. `chunk-contracts.json` is
  byte-identical; `contracts.json` moves by exactly the two added modules and
  `logging.qcore`'s changed text, verified through HYGIENE-16's itemizer; the
  48 `source-effects` leaves and 17 `source.json` leaves are the two recorded
  residue classes, checked leaf by leaf by a new `residues.py` that reproduces
  its transcript from committed bytes, with every per-result status equal and
  the sidecar split unchanged at 3 checked / 93 Unknown. `named_followups`
  moves 26 → 27; `AGENTS.md`'s standing constraints stay 28, six of them
  extended in place (the leak count, the declared-contract agreement rule now
  covering modes as well as types, the publication rule's withdrawn borrowed
  and `Out` admissions, the reserved-word inventory, the archive-diff and CT
  rules, and the generator's now-derived reasons map). One
  pre-existing divergence is recorded rather than repaired
  (`D02-B00-LEDGER-SNAPSHOT`): the matrix's `contract_modules` derives from
  disk at 69 while `contract_modules_per_directory` and `contract_clause_pin`
  derive verbatim from card B00's **frozen** ledger and still describe the
  59-module checkpoint — constraint 21 working as designed, but a snapshot
  published beside a live count with nothing saying so.

Nothing is promoted by this increment: raw Cycle 19 units stay 2 Safe /
42 provisional Unsafe / 137 Unknown plus four data-only units on the freshly
re-pinned wave-17 baseline, with zero unit and zero function status moves in
either direction and no newly surfaced provisional `Unsafe`; the single
diagnostic-level movement is the deliberate leak 271 → 272 above, in the
refusing direction. The contract corpus is **69 modules** by
CONSTRUCTOR-RESIDUALS' two deliberate additions and full language conformance
stays false. CT coverage advances to **33/50 mapped**, by executed-stage
evidence only, with sanitized-native 4 of 33 and proof 0 of 33 — a grown
denominator is mapping, not conformance. `qcore-schemas` is **14.0.0** and the
ChunkIO provider package **0.13.0**; `qcore-compiler`, `qcore-verifier-host`
and `qcore-verification` stay 0.4.0. Three fail-opens were found and repaired
this wave, each measured at base before it was closed: a released identity at
an admitted *nested* call's borrowed actual (CONSTRUCTOR-RESIDUALS), three
`Safe` callers resting on a declared mode the callee's own body contradicts
(LOADER-ABI-MODES), and the publication launder across the summary boundary
that `Borrowed` and declared `Out` slots were carrying (CTWIN-RESIDUALS).
Merged gate at this checkpoint: 2,902 workspace tests (8 ignored), 122 provider
tests, 186 Python tests, 77 Lean audits, both reevaluation examples exit 0, fmt
and clippy clean in both workspaces, and all five provider packages rebuilt and
verified at schemas 14.0.0 / ChunkIO 0.13.0. One environment-class failure is
reported honestly: the first providers run failed
`real_packages_preserve_unknown_and_reject_mutations` with
`ExecutableFileBusy` — a sibling test in the same binary executing the file it
mutates — and it passes in isolation and on the rerun; no assertion is
weakened, and it joins the three recorded load-flakes in `P02-FLAKE`. Follow-on
accounting lives in `docs/reviews/r0_3/acceptance_matrix.json`
(`named_followups`, updated for this wave).

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

#### R02 terminal task state, adversarial verification (2026-09-07)

*Moved verbatim from `AGENTS.md` lines 841-860 — fuller variant.*

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

*Moved verbatim from `CLAUDE.md` lines 615-627 — condensed variant, subsumed by the above but kept verbatim.*

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

#### Compile-task identity (2026-09-07)

*Moved verbatim from `AGENTS.md` lines 862-887 — byte-identical in CLAUDE.md 629-654.*

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

#### Windows Clang discovery and test portability (2026-09-07)

*Moved verbatim from `AGENTS.md` lines 929-938 — byte-identical in CLAUDE.md 696-705.*

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

#### Diagnostic attribution and ownership (2026-09-07)

*Moved verbatim from `AGENTS.md` lines 964-986 — variant carrying the V04/V07 occurrence detail and the spec reconciliation.*

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

*Moved verbatim from `CLAUDE.md` lines 731-752 — variant carrying the file paths, the empty-`contract { }` asymmetry and the `DerivationIncomplete` constraint.*

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

#### Configured second source set (2026-09-07)

*Moved verbatim from `AGENTS.md` lines 988-1007 — byte-identical in CLAUDE.md 754-773.*

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

#### Replay mutation matrix (2026-09-07)

*Moved verbatim from `AGENTS.md` lines 1008-1040 — byte-identical in CLAUDE.md 774-806.*

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

#### Source-filename-sensitive transport (2026-09-07)

*Moved verbatim from `AGENTS.md` lines 1041-1067 — variant carrying the macro/detection detail and the G01 merge record.*

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

*Moved verbatim from `CLAUDE.md` lines 807-824 — variant carrying the `CSourceInput` do-not rules and the `HistoricalStdin` re-measurement rule.*

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

#### Checked mutex invariants (2026-09-07)

*Moved verbatim from `AGENTS.md` lines 1068-1101 — fuller variant.*

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

*Moved verbatim from `CLAUDE.md` lines 825-851 — variant with the T02 module/entry-point framing.*

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

#### Typed scalar-field access classification (2026-09-07)

*Moved verbatim from `AGENTS.md` lines 1103-1144 — byte-identical in CLAUDE.md 853-894.*

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

#### Remaining-work wave accounting (2026-09-07)

*Moved verbatim from `AGENTS.md` lines 1146-1179 — fuller variant with the milestone-criteria list and named follow-ups.*

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

*Moved verbatim from `CLAUDE.md` lines 896-906 — condensed variant.*

Remaining-work wave accounting (2026-09-07): the wave's twenty-five
`docs/reports/remaining-*/` cards are reconciled into
`docs/reviews/r0_3/acceptance_matrix.json` (17 requirements x 4 independently
marked evidence classes, plus stage-derived CT rows) with a readable companion
at `docs/reports/remaining-d02/README.md`. AGENTS.md carries the single
consolidated status paragraph; read it there rather than duplicating it. The
implementation checkpoint for that wave is `67b1a9a`. Nothing is promoted: raw
Cycle 19 units stay 2 Safe / 42 provisional Unsafe / 137 Unknown plus four
data-only units, CT coverage stays 11/50 mapped, the contract corpus stays 59
modules and full language conformance stays false. `qcore-schemas` is 3.0.0;
prose revision 0.3, protocol v1 and evidence v2 are unchanged.

#### Delegated increment (2026-09-08)

*Moved verbatim from `AGENTS.md` lines 1181-1246 — the five-card wave and its `Nothing is promoted` closing paragraph; byte-identical in CLAUDE.md 908-973.*

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

#### Second delegated increment (2026-09-08)

*Moved verbatim from `AGENTS.md` lines 1248-1349 — the seven-card wave and its `Nothing is promoted` closing paragraph; byte-identical in CLAUDE.md 975-1076.*

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

#### Short pointer bullets carried in CLAUDE.md through wave 15

*Moved verbatim from `CLAUDE.md` lines 122-124 — W2 profile migration pointer.*

- W2's bounded profile migration and full 185-TU parity are recorded in
  `docs/reports/w2-profile-manifests/README.md`; remaining H4/H5/H6 and H7
  orchestration work is explicit there. Profile data remains declared trust.

*Moved verbatim from `CLAUDE.md` lines 134-140 — W3 evidence-v2 / verifier-host pointer.*

- W3 bounded evidence-v2 replay/dispatch is recorded in
  `docs/reports/w3-evidence-v2/README.md`. Positive observation certificates
  describe recorded runs under `observations_trusted`; lifecycle source
  derivation stays incomplete until real effect/ABI import exists. The generic
  host is extracted in `crates/qcore-verifier-host` (record:
  `docs/reports/w3-verifier-host/README.md`); trusted derivation/replay/dispatch now reside
  in `qcore-verification` above the dependency-free kernel. V1 Safe refusal is unchanged.

*Moved verbatim from `CLAUDE.md` lines 142-144 — W2 H6 provider discovery pointer.*

- W2 H6 provider metadata now loads from profile packages; see
  `docs/reports/w2-provider-discovery/README.md`. This does not extract providers
  into processes or change the v1 Safe refusal.

*Moved verbatim from `CLAUDE.md` lines 146-149 — Lean CI hygiene pointer.*

Lean CI hygiene is implemented (2026-09-07): the pinned proof build, all theorem
audits and the complete QCore axiom closure are checked, with both reevaluation
examples in Linux/Windows CI. Local gates pass; hosted CI awaits a push.
See `docs/reports/w7-lean-ci/README.md`.

*Moved verbatim from `CLAUDE.md` lines 151-153 — W7 generic Lean naming pointer.*

W7 generic Lean naming is complete for the bounded targets: DescriptorCustody,
EventComposition and busy_effect_preserves_lock, with historical aliases and
unchanged models (`docs/reports/w7-generic-lean-names/README.md`).

*Moved verbatim from `CLAUDE.md` lines 163-165 — qcore-audit driver pointer.*

- The generic `qcore-audit` driver and retained Cycle19 compatibility adapter
  are documented in `docs/reports/w5-configured-audit/README.md`. No provider
  module is linked by the new driver; production evidence routes remain.

#### Claude-specific working bullets carried in CLAUDE.md through wave 15

*Moved verbatim from `CLAUDE.md` lines 81-109 — the crate-layering rule, the W4 custody-pass record and its do-not rules, and the three working rules. The imperative halves now live in `AGENTS.md` (constraint 4) and in the rewritten `CLAUDE.md`; the W4 execution record is kept here.*

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
