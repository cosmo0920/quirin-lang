# Quirin language specification — revision 0.3

> **Name.** The language is **Quirin**; its command is `quirin`. The `QCore`
> spelling is retained for the core component — Rust namespaces, the emitted-C
> surface, the diagnostics codes, the legacy `.qcore` alias and this file's name —
> and those component identifiers are unchanged by the branding. Current source
> names use `.qv` for contract mode and `.qr` for executable/library mode;
> `.qrlib` names native ABI manifests and `.qo` names binary checkpoints. Legacy
> spellings remain warned compatibility aliases.

> **Repository integration (2026-09-07).** Normative requirements are adopted
> from the supplied review; implementation status is reconciled with `main` in
> the [acceptance ledger](reviews/r0_3/REVIEW.md). The original archive findings
> are historical observations, not a claim that later merged repairs are absent.
> Full revision-0.3 target-language conformance is not claimed; each implemented
> capability is bounded by its named semantic profile and evidence level (§L02).

Historical `.qcore`/`.qc` fixture census counts and measured probe outcomes
cited in provenance notes retain their original-checkpoint names and
populations. Current corresponding source paths use the `.qr`, `.qv`, and
`.qrlib` suffixes above.

> **Normative amendment (2026-09-11).** Generated programs are quiet unless
> source code explicitly performs I/O; stream observations compose pointwise;
> and `std.logger` provides `trace`, `debug`, `info`, `warn`, `error`, and
> `fatal` plus entry-level `log_level` system-trace configuration. The complete
> requirements and migration gates are in
> [Quiet C11 Emission, Composable Streams, and Logger Levels](design/QUIET_EMISSION_STREAMS_AMENDMENT.md).

**Revision date:** 2026-09-07, Asia/Tokyo (with reconciled incremental
amendments tracked in the acceptance ledger and card reports).  
**Status:** Source-reviewed specification and implementation requirements. This
is not a declaration of full target-language compiler conformance.  
**Reviewed input:** `qcore-c.tar.gz`, SHA-256 `fd3e844ec975c74e47c7113037c334dd7a4f78fbb8a7e1543a5e59f58da5760b`.

---

## How to Read This Specification & Companion Documents

Quirin is a resource-, effect-, and concurrency-aware systems and verification
language whose toolchain serves two distinct but connected roles:

1. **Checked `.qr` source compilation and C11 emission**, organized as a family
   of explicit, fail-closed **executable and check-only source profiles**
   dispatched in a deterministic table order; and
2. **Contract-driven verification of existing C11 codebases** (`.qv` contracts
   and `.qrlib` native library manifests), organized around separate resource,
   bounds, null-dereference, outer-contract, and observation consumers.

Earlier drafts (revision 0.2) could be misread as describing one monolithic,
uniformly implemented compiler. Revision 0.3 eliminates that ambiguity. Every
section of this specification distinguishes **target language design** (what the
broader Quirin calculus means and forbids) from **implemented profile
boundaries** (exact syntax, static checks, lowered IR, C11 emission, and open
named follow-ons).

| Document | Normative role | What to consult it for |
|---|---|---|
| **Language specification** (this file) | Core semantic domains, quantitative type system, effects/cleanup, executable `.qr` profiles, C interoperability, concurrency/weak-memory rules, verdicts, reason codes, and diagnostics | Meaning of types, ownership, usage grades, resource flows, `.qr` language constructs, profile boundaries, and verdict/diagnostic invariants |
| **[Contract specification](QCORE_CONTRACT_SPEC_r0_3.md)** | External C custody contracts, bounds/index/null-deref contracts, outer contracts, observation policies, and verifier-plugin evidence | `.qv` contract semantics (`resource_contract_v1`, `bounds_contract_v1`, `outer_contract_v1`), contract pinning/review holds, and C-gateway trust boundaries |
| **[Implemented grammars](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md)** | Exact EBNF grammars and consumer limits (`G01`–`G11`) | Token-by-token input shapes accepted by each specific source route, contract loader, manifest reader, and service configuration |
| **[Design BNF](QCORE_DESIGN_GRAMMAR_r0_3.bnf)** | Extended target-language surface syntax | Prospective syntax target; structural acceptance of a BNF form **never** implies semantic checking or C lowering |
| **[Acceptance ledger](reviews/r0_3/REVIEW.md)** & **[VALIDATION.md](VALIDATION.md)** | Reconciled status of review findings (`R01`–`R07`), tests, and audited proofs | Verification checkpoint evidence, repaired findings, and open conformance items |

---

## Table of Contents

- **[Part I — Foundations, Evidence, and Semantic Domains](#part-i--foundations-evidence-and-semantic-domains)**
  - [L01. Authority, scope, and reading order](#l01-authority-scope-and-reading-order)
  - [L02. Evidence vocabulary and profile selection](#l02-evidence-vocabulary-and-profile-selection)
  - [L03. Semantic domains and the typing judgment](#l03-semantic-domains-and-judgments)
- **[Part II — Lexical Structure, Modules, and the Quantitative Type System](#part-ii--lexical-structure-modules-and-the-quantitative-type-system)**
  - [L04. Lexical structure, module modes, and imports](#l04-lexical-and-module-rules)
    - [L04.1. Module modes and multi-file compilation](#l041-module-modes-and-multi-file-compilation)
    - [L04.2. File-local bundled capability imports (`#import std.*;`)](#l042-file-local-bundled-capability-imports)
    - [L04.3. Relative runtime source imports](#runtime-source-imports)
  - [L05. Structural declarations versus complete typing](#l05-structural-declarations-versus-complete-typing)
  - [L06. Nominal dimensions and the closed quantitative index profile](#l06-static-dimensions-and-the-closed-index-profile)
  - [L07. Usage grades and demand intervals](#l07-usage-grades)
    - [L07.4. Existing-C dereference demand (`quant null-deref`) and the three-part dereference decomposition](#l074-existing-c-dereference-demand-and-the-three-part-dereference-decomposition)
  - [L08. Credit, live obligations, cost, and access quantities](#l08-credit-live-obligations-cost-and-access-quantities)
  - [L09. Ownership, lifetime generations, aggregate custody, and borrowing](#l09-ownership-identity-and-borrowing)
    - [L09.1. Quant variants (`source.quant-variant.v1`)](#l091-quant-variants)
  - [L10. Resource-flow algebra and loop/recursion bounds](#l10-resource-flow-algebra-and-loop-bounds)
- **[Part III — Effects, Recoverable Errors, and Fallible Cleanup](#part-iii--effects-recoverable-errors-and-fallible-cleanup)**
  - [L11. Effects and exit maps](#l11-effects-and-exit-maps)
  - [L12. Error regions and checked quantitative opening](#l12-error-regions-and-checked-quantitative-opening)
  - [L13. Fallible cleanup policies and cancellation](#l13-fallible-cleanup-policies-and-cancellation)
- **[Part IV — Executable Source Language and Native C Interoperability](#part-iv--executable-source-language-and-native-c-interoperability)**
  - [L14. Executable source architecture and base sequential profile](#l14-the-current-executable-source-profile)
  - [L14.0. Scalar source calls, fixed-width integers, and IEEE floating point](#l140-scalar-source-calls-and-explicit-standard-output)
    - [L14.0.5. IEEE 754 floating point (`f32` and `f64`) and normative limitations](#l147-floating-point-and-its-limitations)
  - [L14.B. Strings, owned byte buffers, range lends, and records](#l14b-strings-owned-byte-buffers-range-lends-and-records)
  - [L14.C. Pipelines, decidable branching, and bounded counted loops](#l14c-pipelines-decidable-branching-and-bounded-counted-loops)
  - [L14.D. Standard I/O, event reactors, loopback sockets, and HTTP/1.1](#l14d-standard-io-event-reactors-loopback-sockets-and-http11)
  - [L14.E. External C calls, unchecked linkage, `.qrlib` manifests, and C struct FFI](#l141-unchecked-linkage)
    - [L14.1. Unchecked linkage](#l141-unchecked-linkage) · [L14.2. Manifest-resident unchecked linkage](#l142-manifest-resident-unchecked-linkage) · [L14.3. Manifest contract slices](#l143-manifest-contract-slices) · [L14.4. Exported headers & derived decorations](#l144-the-exported-header-a-manifest-binds) · [L14.5. Source library bindings](#l145-the-source-files-own-library-binding) · [L14.6. Imported C structs (`source.ffi.v1`)](#l146-imported-c-structs-and-the-file-wide-trust-mark) · [L14.8. Generated `.qrlib` v2 linkage](#l148-c-linkage-through-generated-manifests-and-its-limitations)
  - [L15. Lowering, observations, and translation certificates](#l15-lowering-observations-and-translation-certificates)
- **[Part V — Concurrency, Weak Memory, and Existing-C Analysis](#l16-checked-concurrency-required-rules-and-current-limitations)**
  - [L16. Checked concurrency: required rules and current limitations](#l16-checked-concurrency-required-rules-and-current-limitations)
  - [L17. Explicit C11 execution candidates](#l17-explicit-c11-execution-candidates)
  - [L18. Existing C and interprocedural summaries](#l18-existing-c-and-interprocedural-summaries)
  - [L19. Contract clauses and verifier scopes](#l19-contract-clauses-and-verifier-scopes)
    - [L19.1. Formal contract hypotheses and quant effects (required extension)](#l191-formal-contract-hypotheses-and-quant-effects-required-extension)
- **[Part VI — Verdicts, Reason Categories, Diagnostics, and Conformance](#l20-verdicts-and-claim-boundaries)**
  - [L20. Verdicts and claim boundaries](#l20-verdicts-and-claim-boundaries)
  - [L21.2. The four-value verdict and its composition](#l212-the-four-value-verdict-and-its-composition)
  - [L21.1. Reason categories for uncovered obligations](#l211-reason-categories-for-uncovered-obligations)
  - [L21. Diagnostics and compatibility](#l21-diagnostics-and-compatibility)
  - [L22. Required conformance and regression evidence](#l22-required-conformance-and-regression-evidence)
  - [L23. Change record](#l23-change-record)

---

<a id="part-i--foundations-evidence-and-semantic-domains"></a>

# Part I — Foundations, Evidence, and Semantic Domains

<a id="l01-authority-scope-and-reading-order"></a>

## L01. Authority, scope, and reading order

### Conceptual Motivation

A verification-oriented language specification fails its readers if it blurs
what the language intends to mean, what a standalone core checker proves over a
supplied IR, what the source-to-C compiler currently lowers, and what an
external contract loader assumes about unanalyzed C binaries. Revision 0.3
establishes a strict separation across four architectural layers:

1. **The Quirin (`QCore`) semantic language** — its orthogonal quantitative,
   ownership, effect, and concurrency domains;
2. **Implemented `.qr` source profiles** — explicitly bounded, fail-closed
   compilation routes from source syntax to checked IR and C11;
3. **Independently callable semantic models** — Rust and Lean checkers over
   explicitly supplied IRs, event traces, state graphs, or weak-memory
   execution candidates; and
4. **Contract consumers and existing-C analyzers** — strict loaders and
   Clang-gated projection pipelines analyzing external C translation units
   relative to pinned `.qv` contracts.

This specification replaces any reading of revision 0.2 as a single uniformly
implemented compiler. Adopting revision 0.3 does not rename the Rust compiler
package version (`qcorec` / `qcore-*`) or alter any wire or serialized protocol
version (`PLUGIN_PROTOCOL_VERSION = 1`, `PLUGIN_EVIDENCE_VERSION = 1`, resource
manifest `schema_version = 1`).

### Normative Precedence and Modal Keywords

This document and the companion [contract specification](QCORE_CONTRACT_SPEC_r0_3.md)
take normative precedence over conflicting descriptions in
`obsolete/QCORE_SPEC_r0_2.md`, `obsolete/qcore_quantitative_types_spec.md`,
`obsolete/concurrency_revision.md`, and historical cycle reports:

- **Companion authority.** The [contract specification](QCORE_CONTRACT_SPEC_r0_3.md)
  governs external custody contracts, bounds/null-deref contracts, outer
  contracts, observation policies, source-bound contracts, and verifier
  evidence. The [implemented grammars](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md)
  specify the deliberately narrow input languages of current consumers. The
  [design BNF](QCORE_DESIGN_GRAMMAR_r0_3.bnf) records the extended target
  syntax; acceptance by the structural parser **never** establishes conformance
  to that BNF or to its runtime semantics.
- **Unchanged design obligations.** Requirements from earlier design documents
  that are not modified here remain **target design requirements**, never claims
  that the current compiler already implements them. Historical cycle and wave
  reports remain immutable records of their own artifacts and execution dates.
- **Requirement keywords.** `MUST`, `MUST NOT`, `SHOULD`, and `MAY` express
  normative requirements on any implementation claiming the relevant semantic
  profile. A requirement stated here can be unmet by the originally reviewed
  archive snapshot; stating a requirement is not an assertion of existing
  enforcement. Observed boundaries, merged repairs, and open gaps are tracked in
  the [acceptance ledger](reviews/r0_3/REVIEW.md).

### Source Anchors and Service Orchestration Boundary

Original line-number citations from the initial revision-0.3 review refer to the
reviewed archive snapshot and may shift as files evolve;
`reviews/r0_3/source_inventory.json` binds those historical review references to
repository source anchors, while the acceptance ledger links current regression
tests and proof modules. Neither the original tarball nor its external probe
bundle is stored inside this checkout.

The configured source-contract audit service (`quirin-audit`,
[configuration grammar G08](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g08-source-contract-service-configuration-v1))
selects explicit provider packages and snapshotted `.qv` policy bundles using
sealed import identities. **Configuration selection supplies zero evidence
authority**: the bounded Boolean-field verification task and independent trusted
witness replay remain mandatory. Service orchestration adds no `.qr` language
syntax, no new semantic profile, and no general lifecycle coverage.

> **Implementation & Provenance.** Repository integration baseline (`2026-09-07`),
> `reviews/r0_3/source_inventory.json`, `crates/qcore-audit/src/source_contract.rs`
> (cards `W5-CONFIGURED-SOURCE-CONTRACTS`, `EXT-01`).

---

<a id="l02-evidence-vocabulary-and-profile-selection"></a>

## L02. Evidence vocabulary and profile selection

### Conceptual Motivation

In verification systems that combine static type checking, C AST abstract
interpretation, dynamic execution replay, and Lean mechanization, labeling a
feature "supported" or "verified" without stating *how* it was checked invites
unsound composition. Every published capability or verification claim in Quirin
`MUST` identify **both** a named **semantic profile** and an **evidence level**.

### The Seven Evidence Levels

| Evidence level | Meaning | What it does **not** establish |
|---|---|---|
| **Structural** | Lexical tokens, balanced delimiters, declaration headers, contract clause kinds, or identifier bindings were accepted and retained in an AST. | Complete ordinary type checking, contract truth, custody safety, effect soundness, or C emission support. |
| **Source-checked** | A named, bounded `.qr` or `.qv` source subset was lowered into its specified semantic passes and passed all static obligations of that profile. | Semantic or executable support for syntactic forms outside that named subset, or whole-language conformance. |
| **Model-checked** | A core semantic checker accepted an explicitly supplied IR program, event trace, state graph, or weak-memory execution candidate. | That any source elaborator faithfully extracts that IR or trace from arbitrary `.qr` or C source code. |
| **Source-bound** | A checked model or interprocedural summary is cryptographically or structurally bound to identified source/build inputs and explicit import coverage. | Behavior of omitted callers, uninspected translation units, dynamic `dlopen` extensions without outer contracts, or unmodeled build configurations. |
| **Observed** | Formal predicates or reference-vs-native stream comparisons were evaluated and agreed on identified recorded executions. | Universal behavior across all inputs, schedules, target platforms, or unobserved failure branches. |
| **Mechanized** | A mathematical theorem about an identified formal model in `proof/QCore/` has placeholder-free, axiom-audited Lean 4 machine-checked evidence. | Correspondence between the Rust implementation and the Lean model, or C compiler refinement, unless those bridges are separately proved. |
| **Proposed** | A syntax form, schema, contract judgment, or subsystem integration is normatively specified for revision 0.3, but has no trusted production consumer yet. | Any completed acceptance gate or executable verification capability. |

### Why Evidence Levels Do Not Form a Linear Hierarchy

The seven evidence levels above are **incomparable epistemic kinds**, not rungs
on a single ladder from weak to strong:

- **Mechanized ≠ Source-bound or Observed.** A machine-checked Lean theorem
  (`Mechanized`) establishes an exact mathematical implication over a formal
  model for *all* model instances, with zero schedule or input sampling error —
  yet by itself it proves nothing about whether a real C library's exported
  symbol matches a header or whether Clang on `x86_64` evaluates an expression
  without UB.
- **Observed ≠ Universal.** Conversely, an AddressSanitizer/UBSan-clean native C
  run whose `stdout`/`stderr` matches the reference interpreter byte-for-byte
  (`Observed`) exercises the actual compiler, OS kernel, and loopback or thread
  runtime on one concrete execution — yet proves nothing about untaken error
  branches, other thread interleavings, or untested inputs.
- **Model-checked ≠ Source-checked.** A concurrency checker accepting an
  explicit event trace (`Model-checked`) relies on a `TrustedExplicitTrace`
  premise until a source lowering (`Source-checked`) derives every event and
  synchronization witness from `.qr` syntax.

Combining two claims across evidence levels `MUST` always state the explicit
bridge relating their subjects (for example, a translation-validation
certificate binding a `Source-checked` lowering, a `Model-checked` CFG, and an
`Observed` native execution under named target and compiler assumptions).

No consumer, report, or user interface `MAY` infer a stronger semantic profile
or higher evidence level from a file extension (`.qr`, `.qv`, `.qrlib`),
successful structural parsing, the mere existence of a `QCore.*` Lean theorem
name, or a historical cycle-completion label.

### Catalog of Semantic Profiles

The profile names below are normative specification labels identifying exact
implemented boundaries (they are not CLI flags unless a command explicitly
documents a selector). They divide into four families according to their input
artifact and checking boundary.

#### 1. `.qr` Source Execution and Source-Checking Profiles

| Profile label | Entry point / pass owner | Implemented capability & boundary |
|---|---|---|
| `source.structural` | `parse_source_program`, `elaborate_source` | Broad structural declaration/statement AST, delimiter matching, lexical name resolution, parameter usage-grade parsing, and unconditional `quant` dimension/axis validation (`Q000`–`Q016`). |
| `source.exec12` | `compile_files`, `quirin check`, `quirin emit` (`driver`) | Base sequential cleanup route: `qcore_acquire`, `qcore_check`, `drop`, `move`, `i32`/`unit` literal returns, and pure zero-argument direct calls (`§L14`). |
| `source.scalar-calls.v1` | `source_scalar_calls` (`OrdinaryTypes`, `Q020`) | Fixed-width integers (`bool`, `i8..u64`, `isize`, `usize`), IEEE `f32`/`f64`, non-wrapping checked arithmetic, `ArithmeticError` (`?`, `match`, `raise`), acyclic value calls, copyable scalar records across calls, `print(expr)`, and `std.io` lines (`§L14.0`). |
| `source.resource-calls.v1` | `source_resource_calls` (`Q020`, `Q051`–`Q057`) | Closed acyclic helper functions carrying `own<bytes> use one`, whole-call `borrow<bytes>` / `lend<bytes>`, and `i32` scalars; derived minimum extents and zero-stranded-owner allocation-failure checks (`§L14.B.4`). |
| `source.string.v1` | `source_std_string` (`StandardString`, `Q068`–`Q073`, `Q075`–`Q079`) | Opt-in `std.string` immutable UTF-8 `{ ptr, len }` views: checked UTF-8 literals and code-point slicing, raw byte output with `std.io`/`std.logger`, static `concat(...)`, and range-wise `borrow_utf8(b, low, high)` over `std.buffer` with loop-window checking (`§L14.B.1`). |
| `source.buffer.v1` | `source_std_buffer` (`StandardBuffer`, `Q051`–`Q057`) | Opt-in `std.buffer` owned `bytes(N)` storage (`1..=4096`), `extent(b)`, statically checked byte reads/writes, disjoint `lend(b, low, high)` write leases with coordinate rebasing, check-time scalar/float value statements, and bounded `for` loops (`§L14.B.2`). |
| `source.record.v1` | `source_record` (widening admitting routes' bands) | Nominal `type NAME { ... }` aggregates: copyable scalar/float records, nested records (depth ≤ 8, leaves ≤ 256), field writes (`r.path = v`), owned `bytes` / `own<Buffer<T>>` fields with partial-move holes, refills, `drop`, and `let y = x;` copy binds (`§L14.B.3`). |
| `source.pipe.v1` | `pipe::desugar_pipes` (`Pipe`, `Q036`–`Q038`) | Parse-time `\|>` pipeline chain desugaring into hygienic `qcore__pipeN` temporaries over call stages, call-shaped model intrinsics, `print`/`log` sinks, and terminal `drop`, plus the per-file positional operator gate (`§L14.C.1`). |
| `source.branch.v1` | `cleanup_lowering` (`Branch`, `Q044`–`Q046`) | Sequential `if (COND) { ... } else { ... }` over a two-sorted decidable condition tier (`\|\|`, `&&`, `!`, comparisons, parenthesized `\|`/`^`/`&`) with C-compatible precedence, arm `return` continuation hoisting, neutral calls, and strict custody-join agreement (`§L14.C.2`). |
| `source.loop.v1` | `source_loop` (`Loop`, `Q097`–`Q099`) | Bounded counted `for i in 0..BOUND { ... }` loops on buffer and composed string/buffer routes: exact bounded unrolling at check time, real C `for` emission, per-iteration custody identity, and the 256 / `[#unroll]` / 4096 / 8192 budget (`§L14.C.3`). |
| `source.std-io.v1` | `source_std_io` (`StandardIo`, `Q028`–`Q031`; composed `Q043`, `Q050`) | Opt-in `std.fmt`, `std.io`, `std.logger` line statements (`print`, `log`, `flush`, `log_level`), plus checked compositions `source.std-io-extern.v1` (with `borrow NAME` live-identity rendering) and `source.std-io-tasks.v1` (with `qcore.obs-reordering.v1` worker windows) (`§L14.D.1`). |
| `source.event.v1` | `source_event` (`Event`, `Q058`–`Q062`, `Q074`) | Opt-in `std.event` single-threaded reactor (`event open`/`close`, `endpoints`, `watch`/`unwatch`, `wake` with optional byte, `dispatch ready`/`idle`), single-ready determinism guard, and drained-byte rendering composition (`§L14.D.2`). |
| `source.socket.v1` | `source_socket` (`StandardSocket`, `Q080`–`Q089`, `Q664`) | Opt-in `std.socket` loopback TCP/UDP state machine, ephemeral or declared IPv4/IPv6 ports (`IPV6_V6ONLY = 1`), `watch`/`unwatch` reactor loan composition, and `disjoint A B;` derived protocol-cell checks (`§L14.D.3`). |
| `source.http.v1` | `source_http` (`StandardHttp`, `Q090`–`Q096`, `Q664`) | Opt-in `std.http` checked RFC 9110/9112 HTTP/1.1 subset: validated start lines and headers, compiler-emitted CRLF framing (excluding header injection), static and `concat`/`len`-computed `Content-Length` equality over `std.socket` (`§L14.D.4`). |
| `source.region.v1` | `source_region` (`ErrorRegion`, `Q302`–`Q309`) | Sequential `error region NAME { ... } catch (E) [quant open NAME as B] { ... }` with `discharge B;`, plus composed `source.epoch.tasks.v1` epilogue regions supporting `qcore_buffer_new` pack duties and `move NAME;` owner adoption (`§L12`). |
| `source.tasks.copy.v1` | `source_tasks` (`Concurrency`, `Q600`) | Checked scoped leaf tasks (`concurrency checked;`) emitted to C11 threads (Linux native, macOS pthread adapter, Windows winpthreads subset): scalar/float `copy` captures, closures (`\|...\|`), explicit `join`, `Result` matching, and `?` propagation (`§L14`, `§L16`). |
| `ScopedOwnedTasksV1` | `source_tasks`, `task_execution` (`Q600`) | Extends scoped tasks (with `ScopedReadLoansV1`, `ScopedReadSharesV1`, `ScopedErrorPayloadV1`) to affine `own<Buffer<T>>` moves/results, counted read shares (`FULL = 1024`), disjoint `lend<Buffer<T>>` byte write leases, tagged error payloads, and record leaf captures (`§L14`, `§L16`). |
| `source.mutex.guard.v1` | `source_mutex` (`Concurrency`, `Q600`, `Q610`–`Q627`) | Lexical `with lock m as g` critical sections over `protected N`/`TYPE` footprints, `open`/`close`/`close g unchanged`, guard-scoped `fact`, opt-in `rank` ordering, permanent `grow`, `thread scope ... under g`, and `share PLACE under MUTEX` crossings with two-thread C11 emission (`§L16`). |
| `source.epoch.bound.v1` | `source_epoch` (`Concurrency`, `Q600`, `Q630`–`Q641`) | Check-only quantitative `thread scope s epoch { ... }` accounting over declared memory bounds, runtime overheads, and worker credits (`FlatEpochBoundV1`; refuses latency claims at `Q639`) (`§L16`). |
| `source.epoch.tasks.v1` | `source_epoch_tasks` (`Concurrency`, `Q600`, `Q630`–`Q641`, `Q302`–`Q309`) | Composes an epoch credit/recovery plan (`FlatEpochBoundV1` + `EpochCreditRecoveryV1`) and optional `error region` epilogue over a real checked scoped-task region (`§L12`, `§L16`). |
| `source.bitfield-concurrency.v1` | `source_bitfield_concurrency` (`Concurrency`, `Q600`, `Q650`–`Q667`) | Check-only concurrent worker field-write disjointness from explicit source layouts via `DisjointFootprintsV1`; refuses concurrent writes to bit-fields sharing a storage unit (`Q661`) (`§L16`). |
| `source.extern-call.v1` | `source_extern` (`ExternCall` `Q022`–`Q027`, `UncheckedLinkage` `Q032`–`Q035`) | Flat sequential calls to declared `extern fn`s under explicitly selected C04 resource-v1 contracts (with mandatory nullable-acquisition/out/reallocation adapters and typed value statements) or scalar-only `unsafe extern fn` waivers (`§L14`, `§L14.1`). |
| `source.native-library.v1` | `native_library`, `linkage` (`NativeLibrary`, `Q039`–`Q042`, `Q047`–`Q049`, `Q063`–`Q067`) | `.qrlib` v1 declarative and v2 build-generated library manifests: artifact/ABI pinning, total contract slices, verified `.h` headers and derived export decorations, `extern library NAME;` source bindings, and ELF/Mach-O/COFF symbol-table verification (`§L14.2`–`§L14.5`, `§L14.8`). |
| `source.ffi.v1` | `source_ffi`, `ffi_layout` (`Q032`–`Q035`, `Q063`–`Q067`, `Q022`–`Q030`, `Q051`–`Q053`) | `#import std.ffi.unsafe;` C struct imports (`struct!`) with Clang-verified layouts, `[| quant #c_ffi |]` contract calls, by-value `.qr` helpers, mandatory v2 `.qrlib` linkage, and the file-wide unsafe-FFI assumption (`§L14.6`, `§L14.8`). |
| `source.quant-variant.v1` | `compile_files`, `quirin check`, `quirin emit` | Library calls and delayed consuming `match`es adapting a separately selected C04.13 signed `i32` return-code contract into a generation-indexed `quant variant` custody package ([G02d.4](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02d4-delayed-custody-results-sourcequant-variantv1), `§L09.1`). |
| `source.nonexec` | `compile_files` with `module library;` or `module contract;` | Entry-free checking of declarations and supported function bodies; emits no standalone C artifact or public library ABI (`§L04.1`). |

#### 2. Pure and Model IR Profiles

| Profile label | Entry point or core module | Model scope & explicit boundary |
|---|---|---|
| `index.closed` | `qcore_frontend::quant_index` | Closed, pure, nominally dimensioned natural-number index expressions and `where` constraints evaluated in checked `u128` inside `credit` and `live` quantitative types (`§L06`). |
| `model.sequential` | `usage`, `ownership`, `resource_flow`, `typed_errors`, `error_region`, `advanced_control`, `cleanup_lowering` | Standalone Rust and Lean semantic checkers operating on explicitly supplied sequential IR programs and CFGs; not a general `.qr` source elaborator (`§L07`–`§L13`). |
| `model.concurrent14` | `qcore_core::concurrency::check_concurrent` (plus `DisjointFootprintsV1`, `FlatEpochBoundV1`, `EpochCreditRecoveryV1`, `CheckedRegionTaskCreditV1`) | Explicit concurrent event-trace, vector-clock, interval-footprint, and epoch-credit checkers carrying `TrustedExplicitTrace` and initial-authority premises (`§L16`, review findings `R01`–`R03`). |
| `CheckedMutexInvariantV1` | `qcore_core::mutex_invariant` (`Q610`–`Q627`) | Bounded concurrency-trace checker discharging `GuardFootprintAuthority` and `SynchronizedCustodyEdges` via guard-instance capabilities, derived happens-before witnesses, opt-in lock ranks, and permanent protection growth (`§L16`). |
| `model.c11-candidate` | `check_c11_execution`, atomic-publication API | Axiomatic C11 weak-memory candidate execution checker (`rf`, `mo`, `sw`, `sc`) and bounded single-publisher/single-consumer release-acquire publication checker (`§L17`). |

#### 3. Existing-C Analysis Profiles

| Profile label | Entry point or gateway | Analysis scope & explicit boundary |
|---|---|---|
| `c.resource` | `analyze_unrestricted_c11` (`qcore-c-gateway`) | Clang-gated conservative custody, indexed-collection, member-chain, callback, and bounded W4 summary inference over C11 translation units relative to pinned `.qv` resource contracts; unmodeled constructs stay `Unknown` (`§L18`). |
| `c.bounds.v1` | `qcore-c-gateway` bounds pass + `bounds_contract`, `bounds_quant` | Reviewed pointer extent, index, and frame checking (`C09.1`) plus `quant checks P, Idx #i: ...` quantified index verification (`C09.2`) over C translation units (`§L06`, [contract §C09.1–§C09.2](QCORE_CONTRACT_SPEC_r0_3.md#c091-reviewed-index-and-extent-contracts-planned)). |
| `c.null.deref.v1` | `qcore-c-gateway` null-dereference pass + `deref_parts` | Null-dereference verification (`C09.3`) using `quant null-deref #B;` (`§L07.4`), `quant deref #B;`, `nonnull_member` (`[[stable]]` / `[[scope]]`), `nonnull_result`, and `result_set` (`C09.4`) contracts ([contract §C09.3](QCORE_CONTRACT_SPEC_r0_3.md#c093-null-dereference-the-property-cnullderefv1-and-quant-null-deref)). |

#### 4. Contract and Verifier-Plugin Profiles

| Profile label | Entry point or host module | Validation scope & explicit boundary |
|---|---|---|
| `contract.resource.v1` | `load_resource_contract_*` (`qcore-contract`; alongside `bounds_contract_v1` and `outer_contract_v1` loaders) | Total-or-reject external `.qv` custody, bounds, and outer-contract loaders, `#include` / `contract inherit` resolution (`QI001`–`QI020`), symbolic premise checking (`QF001`–`QF004`), and `contract-inventory.lock` pin validation (`§L19`). |
| `contract.observation` | `check_contract_observation`, `check_typed_contract_facts` | Checked `i128`, Boolean, and UTF-8 text predicate evaluation (`G06`, `G06b`) over an explicitly supplied fact environment; manufactures no unobserved runtime or C source facts (`§L19`). |
| `verifier.v1` | `VerifierRegistry`, protocol-v1 certificate firewall (`Q700`–`Q703`, `QP001`–`QP016`) | Declarative `verify scope` resolution and external plugin proposal transport; permanently accepts **zero** protocol-v1 `Safe` proposals and demotes unvalidated `Unsafe` allegations to `Unknown` (`§L19`). |
| `verifier.v2.observed` | `verify_dispatch`, independent v2 observation/witness replay | Core obligation derivation, provider dispatch, and independent replay of recorded execution observations under explicit `observations_trusted = true` and observed-run bounds; source lifecycle derivation remains incomplete (`§L19`). |

> **Implementation & Provenance.** `crates/qcore-verification/src/driver/source_routes.rs`
> (`SOURCE_ROUTES` 22-entry dispatch table, card `ROUTE-SELECTION-ORDER-RESTATED`),
> `proof/QCore/RouteSelection.lean`, `proof/QCore/SourceProfileWitness.lean`.

---

<a id="l03-semantic-domains-and-judgments"></a>

## L03. Semantic domains and the typing judgment

### Conceptual Motivation

Systems bugs and unsound static analyses routinely arise from **conflating
distinct resource questions**: treating a reference count or usage count as
proof of exclusive mutation access, treating a pointer's non-null numeric value
as proof that its allocation is still live, or treating a budget of allowed
allocations as proof that an allocated handle was closed. Quirin prevents these
confusions by partitioning program reasoning into **eight orthogonal semantic
domains**, coupled through a single design judgment whose invariants forbid
discharging an obligation in one domain by altering state in another.

### Summary of the Eight Semantic Domains

| Domain | Judgment symbol | Core question answered | What it tracks | What it does **not** establish |
|---|---|---|---|---|
| **1. Usage demand** | `U` | *How many times is a binding demanded along execution paths?* | Closed intervals `[lower, upper]` in `ℕ × (ℕ ∪ {∞})` (`zero`, `one`, `affine`, `relevant`, `many`). | Object lifetime, destruction point, exclusive aliasing, or memory read/write permission. |
| **2. Static dimensions & indices** | `Σ` (index env) | *What static, unit-safe natural quantity describes a capacity or bound?* | Nominal dimensions (`Bytes`, `Count<T>`) and pure closed `u128` linear expressions (`index.closed`). | Runtime values, live heap identities, pointer extents in C, or credit conservation by itself. |
| **3. Spendable credit** | `C` | *How much pre-authorized capacity remains available to spend?* | Conservatively split, transferred, and consumed `quant<credit, q>` balances per dimension. | That any live allocation has been freed; abandoning unused credit never releases custody. |
| **4. Live custody** | `L` | *Which specific resource lifetimes currently owe destruction or handoff?* | Identity- and generation-indexed live resources, aggregate field paths, and outstanding duties (`quant<live, q>`). | Memory bounds, pointer non-nullness of untracked C pointers, or thread-sharing safety alone. |
| **5. Cost upper bounds** | *(flow `P`, `T`)* | *What is the upper bound on cumulative work or peak occupancy?* | Monotone `cost.total` accumulation and `cost.peak` high-water marks across sequential/parallel composition. | Spendable tokens, transferable authority, or wall-clock latency without a separate schedule model. |
| **6. Effects** | `E` | *What side effects or typed error exits may this computation perform?* | Permitted capability tokens (`io`, `alloc`, `mut`, `unsafe`, `spawn`, `join`, `may_block`, `partial_eq`, `partial_ord`) and `throws<Error>`. | Custody cleanup on its own, or C error-sentinel mapping without an explicit contract adapter. |
| **7. Footprint access authority** | `A` | *Which party currently holds read or write permission over which bytes or cells?* | Fractional read shares (out of `FULL = 1024`), disjoint byte write leases `[low, high)`, mutex guard tokens, and protocol cells. | Destruction authority (`drop`/`close` remains the owner's duty) or infinite lifetime. |
| **8. Per-exit post-states & protocol context** | `X`, `K` | *What obligations and protocol states hold on each distinct control exit, and under what enclosing scope?* | Exit-indexed post-states (`normal`, `return`, `error(E)`, `cancel`) and active task/mutex/cleanup-pack protocol context `K`. | That incompatible branch resource states can be averaged or merged into an "optional" leak. |

### Dedicated Domain Specifications

#### L03.1. Usage Demand (`U`)

Usage demand tracks how many times a lexical binding's value is *demanded* by an
expression or function body (§L07). A parameter declared `x: T use one` demands
exact interval `[1, 1]`; `use affine` permits `[0, 1]`; `use relevant` demands
`[1, ∞]`. Usage demand is purely a **binding-occurrence accounting discipline**:
demanding a binding once (`one`) does not identify which heap generation backs
it, does not prove no alias exists in external C code, and does not itself run a
destructor when `0` uses occur.

#### L03.2. Static Dimensions and Closed Indices (`Σ`)

Static indices (§L06) are pure, side-effect-free mathematical natural numbers
tagged with **nominal dimensions** such as `Bytes` or `Count<Chunk>`. Dimensions
prevent unit confusion (`5 Bytes` cannot be added to `5 Steps`), and closed
`where` clauses allow local definitions and checked comparisons without letting
mutable runtime state, pointer arithmetic, or arbitrary calls leak into type
elaboration.

#### L03.3. Spendable Credit (`C`)

Credit (`quant<credit, q>`, §L08) models **pre-paid capacity** — such as an
allocation quota granted to a task epoch or a strict-credit allowance inside an
error region. Credit can be split additively across dimensions, transferred to a
child task, spent by admitting operations, or (where a profile explicitly
permits) abandoned on recovery. Because credit represents *permission to
acquire* rather than an *already acquired object*, discarding or restoring
credit never frees a live resource.

#### L03.4. Identity-Indexed Live Custody (`L`)

Live custody (`L`, summarized quantitatively by `quant<live, q>`, §L08–§L09)
records **active responsibility for concrete resource identities** (each carrying
a unique lifetime generation, such as a heap buffer, socket descriptor, reactor
context, or aggregate field path `RECORD.FIELD`). Every identity in `L` `MUST`
eventually be destroyed (`drop` / `close`), rolled back through a checked
cleanup pack, or transferred (`move`) to an identified custodian. Equal numeric
live counts never make two distinct identities interchangeable: leaking resource
`r1` while freeing an extra resource `r2` is a custody violation even when net
`Δ = 0`.

#### L03.5. Cost Upper Bounds (`cost.total` and `cost.peak`)

Cost quantities (§L08, §L10) bound operational behavior: `cost.total` sums
cumulative work or effort along an execution, whereas `cost.peak` tracks the
maximum simultaneous resource occupancy above entry. Unlike credit, cost is an
**observation bound**, not a conserved currency that can be transferred back to
unlock new operations. In concurrent profiles (`FlatEpochBoundV1`), total work
across workers is never equated with elapsed wall-clock latency (`Q639`).

#### L03.6. Effects (`E`)

An effect row `E` (§L11) bounds the observable interactions and non-normal
control transfers a computation may perform — such as writing to streams (`io`),
allocating (`alloc`), mutating state or byte ranges (`mut`), spawning or joining
tasks (`spawn`, `join`, `may_block`), performing partial IEEE float comparisons
(`partial_eq`, `partial_ord`), or raising typed errors (`throws<E>`). An omitted
or empty effect row is a restrictive semantic claim, never a wildcard.

#### L03.7. Footprint Access Authority (`A`)

Access authority (§L08, §L09, §L16) governs **who may read or mutate a specific
identity's memory range or protocol cell right now**, independent of who
ultimately owns the duty to destroy it. Non-atomic mutation requires exclusive
authority (`FULL = 1024` over the touched footprint, or a disjoint byte write
lease `[low, high)`), whereas concurrent or borrowed reads split positive
conserved shares or non-overlapping read windows (`borrow_utf8`). Passing or
holding a raw C pointer conveys an address representation, **never** access
authority or custody unless backed by a checked loan or contract.

#### L03.8. Protocol Context (`K`) and Per-Exit Post-States (`X`)

Protocol context `K` tracks structured lexical obligations enclosing `e` — active
`thread scope` handles awaiting `join`, open mutex guards whose invariants must
be restored via `close` before scope exit, and sealed error-region
`CleanupPack`s awaiting `quant open` and `discharge`. The post-state family `X`
maps **each reachable exit** (normal fallthrough, `return`, each `error(Variant)`
edge, and modeled cancellation safe points) to its own resulting `(C', L', A', K')`
state, ensuring that error paths cannot silently drop cleanup obligations.

### The General Design Judgment

Across the language design, typing and verification of an expression or
statement `e` take the judgmental shape:

```text
Σ ; Γ ; U ; C ; L ; A ; K ⊢ e : T ! E ▷ X
```

where the components carry the following exact responsibilities:

- `Σ`: static environment of nominal dimensions, quantitative axes, record/error
  type declarations, and explicitly selected trusted external contracts or
  `.qrlib` manifests;
- `Γ`: lexical runtime value environment mapping immutable/typed bindings to
  their ordinary types;
- `U`: synthesized or checked usage-demand environment over bindings in `Γ`;
- `C`: available spendable credit vector on entry to `e`;
- `L`: identity- and generation-indexed live-custody ledger on entry to `e`;
- `A`: active footprint access shares, range leases, and borrow windows on entry
  to `e`;
- `K`: enclosing task-scope, mutex-invariant, and error-region cleanup protocol
  state;
- `T`: value result type produced on the normal continuation exit;
- `E`: derived or checked effect and typed-error row of `e`; and
- `X`: exit-indexed family of post-states `(C_exit, L_exit, A_exit, K_exit)` for
  every normal, `return`, and `error` exit admitted by `E`, and every
  cancellation safe point the profile models (`§L13`).

> **Architectural note.** This judgmental notation specifies **semantic
> responsibilities and composition invariants**, not an assertion that one
> monolithic Rust struct holds all ten components simultaneously. In the
> implementation, specialized passes (`usage`, `quant_index`, `ownership`,
> `resource_flow`, `typed_errors`, `error_region`, `task_execution`,
> `mutex_invariant`, and route-specific lowerings) check their respective
> projections and refuse whenever a cross-domain precondition is unmet.

### Non-Conflation Invariants (Normative)

An implementation `MUST NOT` discharge an obligation in one semantic domain by
mutating or appealing to another:

1. **Credit ≠ Live Custody.** Spending or abandoning `credit` (`C`) `MUST NOT`
   release or excuse a live allocation in `L`.
2. **Usage Demand ≠ Exclusive Access or Destruction.** Consuming a binding with
   grade `one` (`U`) `MUST NOT` be treated as proof of exclusive memory access
   (`A`) or automatic resource destruction (`L`).
3. **Typed Error Exit ≠ Cleanup Erasure.** Exiting via `raise` or `?` (`E`)
   `MUST NOT` erase live custody duties in `L` or open guard/scope duties in `K`;
   every error exit in `X` `MUST` account for every live identity.
4. **Cost Allowance ≠ Resource Creation or Time.** Increasing a `cost.total` or
   `cost.peak` bound `MUST NOT` mint spendable credit (`C`), manufacture a
   resource identity (`L`), or be reported as a wall-clock latency bound.
5. **Representation ≠ Ownership Witness.** Holding a raw pointer value, integer
   address, or OS file-descriptor number in `Γ` `MUST NOT` be treated as an
   ownership witness in `L` or an access share in `A`.

> **Implementation & Provenance.** `crates/qcore-core/src/{usage,ownership,resource_flow,typed_errors,error_region,concurrency,mutex_invariant}.rs`,
> `proof/QCore/{Usage,Ownership,Flow,TypedErrors,ErrorRegion,Concurrency,MutexInvariant,Transferability}.lean`.

---

<a id="part-ii--lexical-structure-modules-and-the-quantitative-type-system"></a>

# Part II — Lexical Structure, Modules, and the Quantitative Type System

<a id="l04-lexical-and-module-rules"></a>

## L04. Lexical structure, module modes, and imports

### Conceptual Motivation

Quirin's front end enforces two foundational hygiene properties before any
semantic route runs: **byte-exact lexical attribution** and **explicit,
non-transitive file-local capability gating**. No ambient I/O, threading, byte
buffer, network socket, string view, or foreign C call vocabulary is implicitly
in scope, and combining multiple files into one compilation unit never silently
transfers capabilities or build-trust waivers from one file to another.

### Lexical Structure, Comments, and Spans

The lexer (`crates/qcore-frontend/src/lexer.rs`) scans UTF-8 source text into
tokens carrying half-open UTF-8 byte spans `Span { start, end }`:

- **Identifiers and words:** ASCII identifiers beginning with an ASCII letter or
  `_`, followed by zero or more ASCII letters, digits, or `_`. Contextual clause
  words (such as `len`, `concat`, `borrow_utf8`, `lend`, and `in`) remain
  ordinary `Word` tokens rather than globally reserved keywords whenever a
  global reservation would collide with pinned `.qv` contract identifiers.
- **Numeric literals:** unsigned decimal integer tokens (`NAT`, with unary `-`
  handled structurally or range-checked in wider precision so two's-complement
  minima such as `-2147483648` for `i32` and `i128::MIN` in `G06` are exact) and
  decimal floating-point tokens (`DIGITS.DIGITS`, `DIGITS.DIGITS e[+|-]DIGITS`,
  or `DIGITS e[+|-]DIGITS`, matched only when a digit follows the exponent).
- **Text literals:** double-quoted string tokens and single-quoted character
  tokens. Inside declaration metadata (such as `verify scope` version strings)
  quoted tokens remain structural; inside executable bodies, string literals
  require `#import std.string;` or route-specific syntax (`std.http`, `struct!`).
- **Punctuation and multi-byte operators:** balanced delimiters `( )`, `{ }`,
  `[ ]`; quantitative index brackets `[|` and `|]`; the three-byte contract
  maps-to arrow `|->` (matched first); two-byte symbols `|>`, `||`, `&&`, `::`,
  `->`, `=>`, `==`, `!=`, `<=`, `>=`; and single-byte punctuation including
  `|`, `&`, `^`, `!`, `?`, `#`, `@`, `,`, `;`, `:`, `.`, `+`, `-`, `*`, `/`,
  `%`, `<`, `>`, `=`.
- **Trivia:** ASCII whitespace, `//` end-of-line comments, and non-nested
  `/* ... */` block comments are skipped while preserving exact byte offsets on
  all surviving tokens.

Lexical recognition of a float literal, string literal, or operator token `MUST
NOT` be interpreted as proof that `source.exec12` or another profile evaluates
it; each semantic pass validates its own admitted token subset.

<a id="l041-the-string-view-amendment"></a>

> **Cross-reference — The revision-0.3 string-view amendment (`§L04.1` → `§L14.B.1`).**
> Earlier printings placed the `2026-09-11` immutable UTF-8 string-view
> amendment (`std.string`, `source.string.v1`, cards `STRING-SLICE`,
> `STRING-SLICE-IO`, `STRING-CONCAT`, `CT07-SLICE-PERMISSIONS`, and
> `LOOPS-ON-COMPOSED-ROUTES`) under `§L04.1`. To keep all `.qr` data-type
> profiles together, the complete normative specification of `std.string` now
> resides in **[§L14.B.1 (Immutable UTF-8 String Views)](#l14b1-immutable-utf-8-string-views)**
> alongside owned byte buffers and records. This anchor is preserved for link
> compatibility.

<a id="l041-module-modes-and-multi-file-compilation"></a>

### L04.1. Module modes and multi-file compilation

A source file `MAY` begin with at most one optional module-mode header, placed
before all imports and declarations (surrounded only by whitespace or comments):

```qcore
module executable;
module library;
module contract;
```

The three declarations above are **mutually exclusive alternatives** (at most
one header per file; repeated, misplaced, unknown, or semicolon-missing headers
reject at parse time):

1. **`module executable;` (default when omitted):** selects executable
   compilation. The driver honors an explicit `--entry NAME` flag, otherwise
   selects `main`, or falls back to its documented single-function rule. The
   selected entry `MUST` declare zero parameters (`Q020`); missing or ambiguous
   entry selection rejects (`Q401`). All supported function bodies in the
   module, **including uncalled helper definitions**, `MUST` undergo full
   semantic checking.
2. **`module library;` (`source.nonexec`):** checks all declarations and
   supported function bodies without requiring, selecting, or synthesizing a
   process entry. A function named `main` in library mode is an ordinary
   checked function, never an implicit entry point. Current library mode
   produces a checked `CompiledModule` (with `entry`, `c_source`, and
   `reference_observation` set to `None`), **not** a compiled C `.o`/`.a`
   object or a public C ABI; `quirin emit` reports that boundary before writing
   output.
3. **`module contract;` (`source.nonexec`):** checks declarations, unconditional
   dimension/axis rules, and structured contract envelopes without selecting an
   entry or emitting C. Retaining a `contract { ... }` block in contract mode
   does **not** prove arbitrary predicates or automatically install external C
   trust; external consumers (`resource_contract_v1`, `bounds_contract_v1`,
   `outer_contract_v1`) validate and load contracts explicitly.

#### Multi-File Mode Resolution and Suffix Guards (`EXT-01`)

When multiple files are compiled together in one invocation:

- **Mode precedence:** the combined compilation is `executable` if **any** root
  file is executable; otherwise `library` if **any** root file is a library;
  otherwise `contract`. Passing `--entry` when no input root is executable
  rejects with `Q403`.
- **Shared declaration namespace:** declarations across combined files share a
  single flat global namespace; duplicate global declaration names reject with
  `Q010`. Combining files introduces no module-qualified namespace hierarchy.
- **Filename suffix guards:** `.qv` requires parsed `contract` mode; `.qr`
  requires `executable` or `library` mode; `.qrlib` names native ABI manifests;
  and `.qo` names binary checkpoints. A filename suffix **never** overrides the
  parsed `module ...;` header — a mismatch rejects at `Q403`. Legacy `.qcore`
  and `.qcorelib` extensions remain accepted with `legacy-source-extension`
  warnings (warnings are naming diagnostics, never safety verdicts), and
  supplying both legacy and current aliases of the same raw path stem is
  ambiguous and refuses (`Q403`; `.qr` and `.qv` of one stem remain distinct
  runtime vs. contract inputs).

The installed CLI binary is `quirin`, owned by the `qcorec` Cargo package:

```sh
cargo run -p qcorec -- check path/to/module.qv
cargo run -p qcorec -- emit -o output.c path/to/program.qr
cargo run -p qcorec -- plugins list
```

*(The historical `cargo run -p qcore-compiler --bin qcorec` invocation is
obsolete following the CLI crate split.)*

<a id="l042-file-local-bundled-capability-imports"></a>

### L04.2. File-local bundled capability imports (`#import std.*;`)

Every capability beyond core declarations and pure scalar/control syntax lives
in an opt-in **bundled semantic module** that `MUST` be imported explicitly at
top level in **each** `.qr` or `.qv` file that uses its vocabulary:

```qcore
#import std.concurrency;
concurrency checked;
```

```qcore
#import std.fmt;
#import std.io;
#import std.logger;
```

Canonical syntax is `#import std.NAME;` (dot-separated identifiers terminated by
`;`, placed after the optional `module` header and before or between top-level
declarations). Bare `import std.NAME;` remains accepted as a parser
compatibility alias for bundled modules; maintained sources use `#import`.
Repeated imports of the same bundled module are idempotent. Unknown bundled
module names reject with `Q014`.

#### Complete Bundled Module Registry

The current registry (`qcore_core::module_imports`) contains **twelve**
independent bundled modules:

| Bundled module | Primary profile(s) | Exact capability gated | Missing-import code | What it does **not** grant |
|---|---|---|---|---|
| `std.concurrency` | `source.tasks.copy.v1`, `ScopedOwnedTasksV1`, `source.mutex.guard.v1`, `source.epoch.*` | `concurrency checked;`, `thread scope`, `spawn`, `join`, `with lock`, `mutex`, `protected`, `epoch` | `Q015` | Stream output (`std.io`/`std.logger`), `std.buffer`, or race-freedom proof from the marker alone. |
| `std.fmt` | `source.std-io.v1` and composed I/O routes | Scalar/float rendering selectors `dec` and `hex` | `Q015` | Any output stream (`print` or `log`); `flush` and raw `std.string` text lines need no `std.fmt`. |
| `std.io` | `source.std-io.v1`, `source.scalar-calls.v1`, composed routes | `print RENDERING OPERAND;`, `print TEXT;`, and scalar-route `print(EXPR);` on `stdout` | `Q015` (`Q020` on `print(EXPR)` without it) | `stderr` logging, `dec`/`hex` selectors (except `print(i32)`), `std.string`, or `std.buffer`. |
| `std.logger` | `source.std-io.v1` and hosted routes | `log LEVEL ...;` on `stderr`, `flush;` sink-handoff barrier, and entry `log_level LEVEL;` threshold | `Q015` | `stdout` `print` lines or `dec`/`hex` formatting selectors. |
| `std.buffer` | `source.buffer.v1`, `source.resource-calls.v1`, composed string bridge | Owned `bytes(N)` acquisition, `extent(b)`, indexed byte loads/stores `b[i]`, and `lend(b, low, high)` | `Q015` | Text literals, UTF-8 string views (`std.string`), stream output, or extern C pointer transport. |
| `std.string` | `source.string.v1`, `source.http.v1` bound views | `string` type ascriptions, body string literals, `len(s)`, `s[low:high]`, `concat(...)`, `borrow_utf8(...)` | `Q015` | Mutable buffers (`std.buffer`), stream output without `std.io`/`std.logger`, or C ABI string passing. |
| `std.event` | `source.event.v1`, `source.socket.v1` reactor composition | Single-threaded reactor `event open`/`close`, `endpoints`, `watch`/`unwatch`, `wake`, `dispatch` | `Q015` | Multi-threading (`std.concurrency`), sockets (`std.socket`), or output lines without `std.io`/`std.logger`. |
| `std.socket` | `source.socket.v1`, `source.http.v1` transport | Loopback `tcp`/`udp`, `bind`, `listen`, `connect`, `accept`, `send`/`recv`, `sendto`/`recvfrom`, `close`, `disjoint` | `Q015` | External network peers, DNS, TLS, HTTP message framing (`std.http`), or reactor context (`std.event`). |
| `std.http` | `source.http.v1` | Checked HTTP/1.1 `request`, `response`, `header`, and `content` message construction | `Q015` | Wire transport without `std.socket`, bound string views without `std.string`, or output lines. |
| `std.extern` | `source.extern-call.v1`, `source.ffi.v1` | Calling a declared `extern fn` (contracted or unchecked) inside an entry body | `Q022` | Uncontracted `unsafe extern fn` declarations (`std.extern.unsafe`) or `struct!` FFI (`std.ffi.unsafe`). |
| `std.extern.unsafe` | `source.extern-call.v1` (`§L14.1`) | Declaring `unsafe extern fn` without a contract (scalars by value only; gate is on the *declaration*) | `Q032` | Calling the function without `std.extern`, pointer parameters/results (`Q033`), or C struct imports. |
| `std.ffi.unsafe` | `source.ffi.v1` (`§L14.6`) | Declaring `struct!` C struct imports, calling `[#c_ffi]` contracts, and marking the file with the unsafe-FFI assumption | `Q032` | Calling `extern fn`s without `std.extern`, plain `Safe` verdicts (`Q035`), or contaminating other files in a bundle. |

> **Why `drop NAME;` is ungated.** The statement `drop NAME;` is an established
> core ownership operation (`OwnershipOp::Drop`) shared across sequential,
> task-buffer, mutex-borrow, and byte-buffer profiles. Gating `drop` behind
> `std.buffer` would force non-buffer modules to import an unrelated module; it
> therefore requires no capability import.

#### Per-File Capability Isolation Invariants

1. **Checked per file before combining.** `check_source_imports` runs on every
   individual file before multi-file or bundle merging, including uncalled
   functions and non-executable (`library`/`contract`) modules.
2. **Zero transitive or cross-file leakage.** Importing a capability in file `A`
   `MUST NOT` grant that capability to file `B` in the same compilation. In
   particular, calling an `extern fn` declared in another file requires
   `#import std.extern;` in the **calling** file (`Q022`), and declaring
   `unsafe extern fn` or `struct!` requires `std.extern.unsafe` /
   `std.ffi.unsafe` in the **declaring** file (`Q032`). Likewise, the
   import-adjacent `extern library NAME;` build binding (`§L14.5`) applies
   strictly to the file that writes it.
3. **Vocabulary ≠ verification.** Importing a `std.*` module exposes syntax to
   its semantic pass; it never waives `concurrency checked;`, effect-row
   obligations, or unsupported-profile refusals. Wildcards (`std.*`), import
   aliases, selective symbol imports, and re-exports are not supported.

<a id="runtime-source-imports"></a>

### L04.3. Relative runtime source imports

Runtime `.qr` modules `MAY` include helper `.qr` files from relative paths using
a quoted `#import` directive at top level:

```qcore
#import std.io;
#import "./helper";
#import "../shared/other.qr";

fn main() -> i32 {
    return helper();
}
```

#### Syntax and Path Hygiene

- **Canonical form only:** runtime file imports `MUST` be written
  `#import "RELATIVE_PATH";` at top level, terminated by `;`. Bare `import "..."`
  without `#` is refused.
- **Extension rule:** an omitted file extension appends `.qr` exactly; an
  explicit extension `MUST` be `.qr`. There is no `.qcore` legacy fallback, no
  directory `index`/`mod` convention, no package registry, and no environment
  variable search path.
- **Lexical path constraints:** the quoted path `MUST` be nonempty and `MUST
  NOT` contain escape sequences, backslashes (`\`), control characters, or
  Windows/URI drive prefixes, ensuring identical interpretation across lexer and
  host OS.
- **Strict separation from `.qv` contract `#include`:** contract inclusion uses
  `#include "..."` (`QI001`–`QI020`), resolved exclusively by `qcore-contract`
  against a published contract load path. Contract `.qv` files `MUST NOT` import
  runtime `.qr` files, and runtime `#import` never resolves `.qv` contracts or
  `.qrlib` manifests (`Q014`).

#### Snapshot-Only vs. Filesystem Resolution and Symlink Identity

Quirin separates pure in-memory compilation from filesystem loading
(`crates/qcore-verification/src/source_imports.rs`):

- **Snapshot-only API (`SourceLoading::SnapshotsOnly`, default for library
  calls):** resolves imports strictly over caller-supplied virtual paths, never
  reading the host disk or consulting the process working directory. Duplicate
  supplied snapshot identities refuse (`Q014`) even when their UTF-8 contents
  are byte-identical.
- **Explicit filesystem API (`SourceLoading::FileSystem`, used by CLI `quirin
  check` and `quirin emit`):** loads dependencies absent from the supplied
  snapshot map relative to the importing file's directory, reading each disk
  file at most once per resolution. Supplied in-memory snapshots take precedence
  over disk at the same lookup identity.
- **Physical identity before `..` collapse:** in filesystem mode, existing
  directory/file physical identities `MUST` be resolved **before** simplifying
  parent `..` components — the resolver `MUST NOT` lexically collapse
  `symlink/..` prior to filesystem lookup. An unresolved parent identity with
  `..` refuses in filesystem mode. Canonicalized lookup keys never overwrite the
  caller's original diagnostic file paths. Missing files, unreadable or
  non-UTF-8 files, directories, import cycles, and ambiguous identities all
  refuse at `Q014`.

#### Bundle Graph Ordering, Modes, Resource Bounds, and Retained Evidence

- **Roots, pool, and deterministic topological order:**
  `compile_source_bundle_with_registry` separates explicit **roots** from an
  available snapshot pool (unreachable pool entries are ignored; legacy
  `compile_files` treats all supplied files as roots). Roots preserve caller
  order; transitive dependencies are appended in source-order depth-first
  first-discovery order. Repeated and diamond imports include a dependency once
  while validating **every** import edge (including edges targeting roots).
  Completed-node topological order is independently rechecked: every edge `MUST`
  target a dependency completed prior to its importer.
- **Dependency modes:** unmarked files and explicit `module library;` files are
  permitted as imported dependencies; `module contract;` and explicit
  `module executable;` dependencies refuse (`Q014`). **Only root files determine
  the combined module mode**, so importing an unmarked helper into a `module
  library;` root never turns that library into an executable.
- **Hard resource bounds:** bundle resolution enforces defaults of **1,024**
  supplied/selected modules, **16 MiB** retained UTF-8 source bytes, and
  recursive traversal depth **128**. Callers may tighten these bounds; exceeding
  any bound refuses (`Q014`) rather than truncating the module graph.
- **Standalone parser vs. staged checking projection:** standalone
  `parse_source_program` refuses unresolved runtime `#import` directives
  (`Q000`), while `inspect_source_imports` extracts structural header/import
  metadata without producing a `SourceProgram`. In the verification driver,
  `CompiledModule::source_bundle` retains the exact original snapshots, resolved
  edges, directive spans/spellings, and completed-node order; its internal
  checking projection blanks only resolved runtime `#import` directives and mode
  headers with equal-length ASCII spaces (preserving CR/LF bytes and every
  declaration/statement byte span). Snapshot retention binds checked source
  bytes; it is not host-filesystem or runtime-execution attestation.

> **Implementation & Provenance.** `crates/qcore-frontend/src/{lexer,frontend,module_imports}.rs`,
> `crates/qcore-core/src/module_imports.rs`, `crates/qcore-verification/src/source_imports.rs`;
> Lean: `QCore.ModuleImports`, `QCore.RuntimeImports`, `QCore.SourceExtensions`;
> cards `STD-MODULES`, `STD-BUFFER`, `STRING-SLICE`, `STD-EVENT`, `QUIRIN-SOCKET-V1`,
> `HTTP-QUIRIN-V1`, `W48-QR-C-STRUCT-IMPORT`, `EXT-01`.

---

<a id="l05-structural-declarations-versus-complete-typing"></a>

## L05. Structural declarations versus complete typing

### Conceptual Motivation

Because Quirin's parser serves structural contract tools, gradual source routes,
and full executable backends simultaneously, the front end parses a **broad
structural envelope** (`G01`) whose syntactic reach intentionally exceeds any
single execution route. Treating "the file parsed without `Q000`" as "the file
is type-safe" would violate Hard Invariant 1 (fail-closed). Section `L05`
draws a hard line between **structural retention** and **semantic validation**.

### What the Structural Front End Recognizes and Retains

`parse_source_program` and `elaborate_source` (`crates/qcore-frontend/src/frontend.rs`)
recognize thirteen `DeclarationKind` forms:

1. `concurrency checked;` profile markers (`ConcurrencyProfile`);
2. `verify scope NAME { ... }` provider configurations (`VerifierScope`);
3. `dimension D : Idx + Ordered;` nominal index dimensions (`Dimension`);
4. `quant axis A : MODE<D>;` quantitative axes (`Axis`);
5. `quant variant Name(subject) { Arm carries ...; ... }` delayed custody
   result declarations (`QuantVariant`, `§L09.1`);
6. `type Alias = ...;` type aliases (`TypeAlias`);
7. `type Name { field: Type, ... }` aggregate record declarations (`Record`);
8. `struct! { type Name = c"struct Tag" from "header.h"; fields { ... } }` C FFI
   struct imports (`ImportedStruct`);
9. `resource type Name { ... }` nominal resource declarations (`ResourceType`);
10. `error Name { Variant; ... }` typed error declarations (`ErrorType`);
11. `const NAME ...;` constant declarations (`Constant`);
12. `[unsafe] extern fn ...;` external function declarations, optionally
    preceded by `[| quant #c_ffi |]` (`ExternFunction`); and
13. `fn name(...) -> Result [effects { ... }] [contract { ... }] { ... }` source
    functions (`Function`), plus lifted closure workers (`SourceClosure`).

For each declaration, the AST retains parameter lists (`SourceParameter`), exact
result-type token slices and spans (`SourceResultType`), parsed quantitative
index types (`SourceQuantIndexType`), structured contract-clause envelopes
(`SourceFunctionContract`), and statement block trees (`SourceBlock`).

### Why Structural Classification Is Not Complete Type Elaboration

Classifying a parameter or return token sequence into a coarse `SourceTypeKind`
tag (`Owned`, `Borrow`, `Quant`, `AccessQuant`, `Result`, `Task`, `Mutex`,
`Pointer`, `Slice`, etc.) is **syntactic bucketing**, not type checking:

- **Result and payload separation (`R04`).** In `CompiledModule.result_profile`
  (`SourceResultProfile`), the base sequential driver admits explicit `i32` or
  `unit` scalar return types, checks return statement shapes, and tracks `move`d
  resources in a separate `CustodyPayloadProfile::SeparateIdentityLedger` — an
  `i32` return annotation does not type owned payloads as `own<T>`. By contrast,
  `ScopedOwnedTasksV1` explicitly validates `own<Buffer<T>>` worker results,
  `source.resource-calls.v1` validates `own<bytes>` helper results, and
  `source.scalar-calls.v1` / `source.tasks.copy.v1` / `source.ffi.v1` validate
  nominal record results. General arbitrary `own<T>` and per-exit dependent
  result elaboration across unrestricted `.qr` syntax remain open (`R04`).
- **Unconditional declaration validation (`R05` and Wave 48).** Axis and
  dimension declarations are validated unconditionally in `elaborate_source`
  (`Q016`), even in modules where no function references a `quant<...>` type
  (`R05` repaired). Likewise, across executable and library modules:
  - Sequential parameter annotations are checked against the fixed-width scalar
    table (`Q020`, card `W48-QR-SCALAR-TYPES-R2`);
  - Every `extern fn` declaration is checked against the `§C04.1` ABI vocabulary
    whether or not any body calls it (`Q020`, card `W48-QR-SCALAR-TYPES-R3`);
    and
  - Every entry-only source route (`source.std-io.v1` `Q030`, `source.buffer.v1`
    `Q051`, `source.event.v1` `Q058`, `source.string.v1` `Q068`,
    `source.socket.v1` `Q080`, `source.http.v1` `Q090`) explicitly refuses
    additional unmodeled source functions rather than silently ignoring their
    signatures and bodies (`W48-QR-SCALAR-TYPES-R2`).

### Normative Conformance Rules

1. A conforming source checker `MUST` validate every declared type and every
   `dimension` / `quant axis` declaration in scope, even when unused by any
   function body.
2. It `MUST` preserve and validate parameter types, result types, declared
   `effects { ... }` rows, and every contract clause that its profile claims to
   discharge. Unknown type names, invalid dimension references, unsupported
   semantic annotations, and result-type mismatches `MUST NOT` receive a claim
   of complete type correctness.
3. Structural inspection tools `MAY` retain declarations or clauses they do not
   semantically interpret, provided their output explicitly distinguishes
   **retention** from **validation**. Any verification consumer asked to prove a
   property over a construct that was only structurally retained `MUST` reject
   the input or return `Unknown`.

> **Implementation & Provenance.** `crates/qcore-frontend/src/{frontend,source_annotation}.rs`,
> `crates/qcore-verification/src/driver.rs`, `proof/QCore/{Frontend,SourceAnnotation}.lean`;
> review repairs `R04`, `R05`, cards `SOURCE-ANNOTATION-ENFORCEMENT`,
> `W48-QR-SCALAR-TYPES-R2`, `W48-QR-SCALAR-TYPES-R3`.

---

<a id="l06-static-dimensions-and-the-closed-index-profile"></a>

## L06. Nominal dimensions and the closed quantitative index profile

### Conceptual Motivation

Quantitative resource capacities and live-duty summaries require static integer
indices that are **unit-safe** and **decidable without executing user code**.
Quirin achieves this through **nominal dimensions** paired with the closed
linear natural-number evaluator `index.closed` (`G04`).

### Nominal Dimensions and Quantitative Axes

A dimension is nominal: `Bytes` and `Steps` are distinct types even when both
evaluate to `5`. The parameterized form `Count<T>` names a nominal count
dimension tagged by `T`; declaring `Count<File>` does **not** by itself prove
that `File` is a live runtime resource or heap allocation kind.

```qcore
dimension Bytes : Idx + Ordered;
quant axis memory_budget : credit<Bytes>;
quant axis open_files    : live<Count<File>>;
quant axis allocations   : cost.total<Count<Allocation>>;
quant axis maximum_bytes : cost.peak<Bytes>;
```

- **Pure static bounds (`Idx + Ordered`):** the bound `Idx + Ordered` signifies
  that index addition, scaling, `min`/`max`/`monus`, and ordering comparisons
  are pure, total/checked, dimension-preserving operations owned by the compiler.
  It is **not** a user-implementable runtime trait: no mutable field read,
  pointer dereference, allocator query, or runtime function call `MAY` ever
  enter index evaluation.
- **Declared axis modes vs. resolved `quant` types:** `quant axis` declarations
  grammatically and Environment-wise admit all four axis modes (`credit`, `live`,
  `cost.total`, `cost.peak`) and are validated unconditionally (`Q016`).
  However, the implemented `index.closed` quantitative *type* resolver admits
  only **`credit`** and **`live`** vectors/scalars (`quant<cost, ...>` is
  rejected as an unimplemented cost-type profile, and `quant<access, ...>` is
  skipped by `quant_index` for separate handling without claiming source-level
  access verification).

### The `index.closed` Type and Expression Grammar (`G04`)

Quantitative index types are written with angle brackets `quant< ... >` or with
the equivalent readable bracket-bar alias `quant[| ... |]`:

```qcore
module contract;
quant axis chunks : credit<Count<Chunk>>;

extern fn reserve(q: quant<n where n := 5> use one)
    -> quant<credit, { chunks = total }
             where total := base + 2, base := 3, total <= 5>;

extern fn hold(q: quant[|n where n := 5|] use one) -> unit;
```

*(Note: this contract signature illustrates closed index elaboration to `5`
`Count<Chunk>` credits; declaring `extern fn reserve` does not prove that an
external C implementation conserves or manufactures credit.)*

#### Syntax, Delimiters, and Shorthand Rules

- **Delimiter pairing:** `quant[| ... |]` has the exact same interior grammar
  and semantics as `quant< ... >` (introduced so `<` and `>` in `where`
  comparisons do not collide visually with type brackets). The opening delimiter
  fixes the required closer; mixed pairs such as `quant[| ... >` are rejected.
- **Scalar shorthand vs. named vectors:** scalar `quant<EXPR ...>` selects the
  unique `credit` axis in the module's static environment; if zero or two or
  more `credit` axes are declared, scalar shorthand is ambiguous and rejects
  (`Q016`). Vector syntax `quant<MODE, { axis_1 = e_1, ... } ...>` requires a
  nonempty set of unique field names whose declared axes all match `MODE`
  (`credit` or `live`). Axes omitted from a vector are absent from that type,
  not implicitly zero-initialized resources.
- **Expression vocabulary:** decimal natural literals (`NAT`), type-local
  `where`-bound names, addition (`+`), multiplication (`*`) with **at least one
  syntactic literal factor per product**, parenthesized subexpressions, and
  `min(a, b)`, `max(a, b)`, and `monus(a, b)`.
- **Constraint vocabulary:** `==`, `!=`, `<`, `<=`, `>`, `>=`, and `true`,
  separated by `,` or `and`. Every constraint `MUST` evaluate to `true` (`Q016`).

#### Local `where` Bindings, `u128` Checked Arithmetic, and Operational Limits

1. **Type-local `where` scope:** each `where name := expr` binding is pure and
   scoped strictly to its enclosing `quant` type (never creating a runtime local
   or global constant). Forward references to later definitions in the same
   `where` clause are permitted, provided the dependency graph is acyclic.
   Duplicate bindings, dependency cycles, shadowing of forbidden visible names,
   unbound or runtime variable references, and mixed-dimension additions/comparisons
   all reject (`Q016`). Every local binding `MUST` acquire a concrete dimension
   from its axis context; an unreferenced dimensionless binding is rejected,
   never silently defaulted.
2. **Checked `u128` natural arithmetic and strict `monus`:** all evaluation uses
   checked `u128` arithmetic; overflow rejects. `monus(a, b)` requires `b <= a`
   and evaluates to `a - b`; when `b > a`, `monus(a, b)` **rejects** rather than
   silently saturating a negative difference to `0`. Nonlinear products (`x * y`
   where both operands are names) are rejected syntactically even if both names
   resolve to constants.
3. **Explicit evaluator bounds:** `quant_index` enforces maximum atom nesting
   depth **32**, additive chain length **64**, and bounded dependency/evaluation
   recursion (`crates/qcore-frontend/src/quant_index.rs`). Exhausting any bound
   is a hard rejection (`Q016`), never permission to approximate an expression
   as zero. Runtime-to-index reflection, symbolic inequality solving, dependent
   parameter substitution, and C pointer extents are outside `index.closed`.

### Relationship to Existing-C Extent and Quantified Index Contracts

Existing-C bounds analysis (`c.bounds.v1`) uses two distinct contract constructs
that relate carefully to this section:

- **Reviewed extent and index contracts ([contract §C09.1](QCORE_CONTRACT_SPEC_r0_3.md#c091-reviewed-index-and-extent-contracts-planned), grammar `G05b`):**
  clauses such as `bounds_extent(p, n, bytes)` and `bounds_index(i, n)` are
  loaded by the separate `bounds_contract_v1` consumer (`crates/qcore-contract/src/bounds_contract.rs`,
  partly implemented by cards `CG16-INDEX-AND-OFFSET-EXTENTS` and
  `CG24-ALIAS-EXTENT-FRAME-VOCABULARY`). Their conceptual `Idx<N>` relation is
  an external C precondition/postcondition vocabulary, not `index.closed` type
  syntax; custody safety (`c.resource`) alone proves zero access bounds.
- **Quantified index checks ([contract §C09.2](QCORE_CONTRACT_SPEC_r0_3.md#c092-quantified-index-checks-quant-checks), grammar `G05c`):**
  the collocated contract check `quant checks P, Idx #i: PREDICATE;` (`qcore_contract::bounds_quant`,
  `qcore_core::index_quant`) belongs to this quantitative index family. Within a
  pointer dereference `*B` or `B->f`, `quant checks B, Idx #i: #i == 0;` is the
  **extent projection `(b)`** of the three-part dereference decomposition
  (`§L07.4`): it entails existence (`quant null-deref #B;`, since `NULL` has no
  in-bounds extent), whereas `quant null-deref #B;` itself has no index,
  dimension, or predicate and belongs to `§L07` usage demand.

> **Implementation & Provenance.** `crates/qcore-frontend/src/{parser,check,quant_index}.rs`,
> `crates/qcore-contract/src/{bounds_contract,bounds_quant}.rs`,
> `crates/qcore-core/src/index_quant.rs`; Lean: `QCore.Index`,
> `QCore.QuantIndexBinding`, `QCore.IndexQuant`, `QCore.IndexExtent`;
> cards `CG16-INDEX-AND-OFFSET-EXTENTS`, `CG24-ALIAS-EXTENT-FRAME-VOCABULARY`,
> `QUANT-INDEX-CHECKS`, repair `R05`.

---

<a id="l07-usage-grades"></a>

## L07. Usage grades and demand intervals

### Conceptual Motivation

Following Quantitative Type Theory (QTT) — adapted to closed nonnegative
intervals rather than a coarse `{0, 1, ω}` semiring (see
[`QTT_ALIGNMENT.md`](QTT_ALIGNMENT.md)) — Quirin tracks **usage demand**: a
static interval bounding how many times a bound parameter or variable is
demanded along execution paths. Interval grades distinguish strict linear
consumption (`one = [1, 1]`) from optional/affine omission (`affine = [0, 1]`)
and guaranteed-at-least-once use (`relevant = [1, inf]`), without conflating
binding demand with heap lifetime or memory aliasing.

### The Interval Grade Algebra (`G03`)

A usage grade is a closed interval `[lower, upper]` over nonnegative demands,
where `lower ∈ ℕ` and `upper ∈ ℕ ∪ {inf}`. Finite endpoints are represented as
checked `u64` integers (`crates/qcore-core/src/usage.rs`):

| Source grade syntax | Interval meaning | Intuition |
|---|---|---|
| `zero` | `[0, 0]` | Erased / never demanded at runtime |
| `one` | `[1, 1]` | Demanded **exactly once** on every path |
| `affine` | `[0, 1]` | Demanded **at most once** (omission permitted) |
| `relevant` | `[1, inf]` | Demanded **at least once** on every path |
| `many` | `[0, inf]` | Unrestricted demand |
| Natural literal `n` | `[n, n]` | Demanded **exactly `n` times** (`n` fits `u64`) |
| `[a, b]` or `[a, inf]` | `[a, b]` / `[a, inf]` | Explicit closed interval (`a <= b` required for finite `b`) |

#### Algebraic Operations and Admission Order

- **Sequential addition (`+`):** sequential demands on the same binding add
  endpoint-wise: `[a_1, b_1] + [a_2, b_2] = [a_1 + a_2, b_1 + b_2]`, with
  `x + inf = inf`. Checked `u64` overflow rejects (`Q100`).
- **Demand substitution / scaling (`*`):** substituting an argument or scaling
  captured variables under an enclosing usage multiplies intervals:
  `[a_1, b_1] * [a_2, b_2] = [a_1 * a_2, b_1 * b_2]`. **Zero absorbs infinity:**
  `0 * inf = 0` and `inf * 0 = 0`, so zero outer demand yields `[0, 0]` even
  against an unbounded upper endpoint. Checked `u64` overflow rejects (`Q100`).
- **Mutually exclusive branch join (`⊔`):** joining two mutually exclusive
  control-flow arms takes the **least enclosing interval (hull)**
  `[min(a_1, a_2), max(b_1, b_2)]` — **never** the sum of the two branches.
- **Grade admission (`⊇`):** a declared grade `D = [d_lo, d_hi]` admits an
  actual synthesized demand `A = [a_lo, a_hi]` if and only if `D` contains `A`
  (`d_lo <= a_lo` and `a_hi <= d_hi`).

### Lambda Binder Validation Before Capture Scaling

In higher-order usage checking (`UsageExpr::Lambda` and `synthesize_demands_at`),
an implementation `MUST` check a lambda body for **one single invocation**,
validate its bound parameter's actual demand against that parameter's declared
grade, remove the bound parameter from the environment, and **only then** scale
the remaining free-variable capture demands by the enclosing outer multiplicity:

- **Zero outer demand never hides a broken binder:** wrapping `(λ (x : T use one). 0)`
  in a `zero`-graded context `MUST` still reject `x`'s actual `[0, 0]` demand
  against `[1, 1]` (`QCore.UsageDemand.lambda_parameter_covers`).
- **Closure replication scales captures:** duplicating a closure under `[2, 2]`
  or `many` multiplies its free-variable demands (`scaled_captures_cover`), so an
  `affine` (`[0, 1]`) outer binding cannot be captured and demanded inside a
  replicated closure.

> **Profile boundary.** Lambda demand synthesis is implemented and mechanized at
> the reduced usage IR boundary (`crates/qcore-core/src/usage.rs`,
> `proof/QCore/UsageDemand.lean`). First-class closure values and general source
> lambda expressions are **not** part of `source.exec12` (scoped-task spawn
> closures `spawn s |...| -> T { ... }` in `G02v` desugar to hidden first-order
> worker functions with explicit capture lists; see `§L16`).

### Source Parameter Enforcement in the Compiler Driver

In the `.qr` compiler driver (`crates/qcore-verification/src/driver.rs`):

- Unannotated source parameters default to grade `many` (`[0, inf]`). Explicit
  parameter annotations (`x: i32 use one`, `buf: own<bytes> use one`, etc.) are
  preserved through elaboration and checked on **every** function in the module,
  including uncalled functions (`Q100`).
- Supported explicit resource operations (`drop`, `move`) and zero-argument
  direct calls contribute demand. Local `let` bindings receive distinct internal
  usage symbols so that lexical variable shadowing **cannot** falsely satisfy an
  enclosing parameter's `use one` obligation.
- Statically known failing model operations (`qcore_acquire(..., false, ...)` or
  `qcore_check(false, ...)`) truncate subsequent demands on that dead tail, while
  call continuations conservatively admit early exit (`0` lower continuation
  demand) when normal return is not guaranteed during demand synthesis.
- Local `let` usage annotations (`let x use one = ...;`) and indirect calls
  through shadowed locals are explicitly rejected (`Q100` / `Q020`) in the
  executable subset rather than silently ignored.
- **Non-conflation:** `use one` constrains binding demand only; heap destruction,
  implicit error-path cleanup, pointer aliasing, and cross-thread transferability
  remain separate checks (`§L08`–`§L09`, `§L16`).

<a id="l074-existing-c-dereference-demand-and-the-three-part-dereference-decomposition"></a>

### L07.4. Existing-C dereference demand (`quant null-deref`) and the three-part dereference decomposition

The existing-C contract clause `quant null-deref #B;`
([contract §C09.3](QCORE_CONTRACT_SPEC_r0_3.md#c093-null-dereference-the-property-cnullderefv1-and-quant-null-deref),
grammar [G05d](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g05d-null-dereference-checks))
is the contract-level projection of a **`§L07` usage grade**, interpreted jointly
with the access shares of `§L08` and the object identities of `§L09`. Despite
beginning with the `quant` introducer, it is **not** a `§L06` index or
dimension, introduces no `credit`/`live`/`cost` axis, and defines no new type:
`#B` names a pointer formal of the C declaration just as `Idx #i` names an index
binder in `G05c`.

#### 1. Path Demand, Body Join, and Clause Obligations (Normative)

1. **Demand of an execution path.** Each dereference (`*B`, `B->f`, `B[i]`) of
   formal `B`'s entry value counts as one use (`[1, 1]`) of its pointee, and
   uses along a single execution path add sequentially (`[n, n]`). Accounting on
   that path closes immediately at the **first `NULL` test** of `B`'s entry
   value or at the **first reassignment** of `B`. Pure pointer-offset evaluation
   `B ± n` without dereferencing adds `[0, 0]`. Passing `B`'s entry value to a
   callee parameter governed by an active `quant null-deref` clause performs
   demand substitution (`relevant * one`, adding at least one use).
2. **Demand of a function body.** A function body's dereference demand for `B`
   is the **branch join** (least enclosing interval) of the demands of all its
   terminating paths. A body with zero terminating paths has no demand interval.
3. **Meaning and inference of `quant null-deref #B;`.** Dereferencing `B` on
   *every* terminating path before any `NULL` test corresponds exactly to a body
   demand lower bound of at least `1` — i.e., admission by the grade `relevant`
   (`[1, inf]`). Dereferencing `B` on only *some* paths yields lower bound `0`
   (`many = [0, inf]`), which imposes no unconditional caller precondition and
   from which no `quant null-deref` clause is inferred.
4. **The moved question (identity inheritance).** Once a path has incurred a
   demand of at least `1` on `B` without reassigning `B`, subsequent operations
   along that path (including sub-object address formation `&B->F`) add
   sequentially to a demand whose lower bound already requires `B`'s pointee
   identity; they inherit the existence guarantee required by the first use.
5. **Caller obligation.** At a call site, a declared `relevant` dereference
   demand admits an actual pointer argument only if that actual's target
   **identity exists** — that is, only if the pointer is non-`NULL` (`§L08`–`§L09`:
   `NULL` denotes no identity, so no footprint access share exists for `NULL`).
6. **Callee body validation (`necessary` vs. `stronger-than-necessary`).**
   Validating a declared `quant null-deref #B;` clause against its C body reads
   the body's demand **lower bound alone**, classifying the clause as
   `necessary` (lower bound `≥ 1`) or `stronger-than-necessary` (lower bound `0`)
   and **never rejecting** the body. This is an explicit, sound exception to
   ordinary interval containment: because `quant null-deref #B;` states a pure
   existence precondition on callers and consumes no conserved share, requiring
   non-`NULL` from callers when a body only conditionally dereferences `B`
   weakens no safety property.

#### 2. The Three-Part Decomposition of a Pointer Dereference

A pointer dereference `*B` or `B->f` in C is memory-safe only when **three
distinct projections** of that single access hold simultaneously, each owned by
an existing Quirin subsystem:

- **(a) Existence:** `quant null-deref #B;` (`§L07` usage grade + `§L09`
  identity — the pointer designates an identity rather than `NULL`);
- **(b) Extent:** `quant checks B, Idx #i: #i == 0;` (`§L06` quantified index
  check — offset `0` lies within the target object's valid extent); and
- **(c) Liveness:** `quant live #B;` (`§L08` live obligation + `c.resource`
  custody — the designated identity's lifetime generation has not been released).

In `QCore.DerefDecomposition`, the logical relationships among `(a)`, `(b)`, and
`(c)` are machine-checked theorems derived directly from the existing semantic
models with zero new axioms:

- **Extent `(b)` entails existence `(a)`** (`checks_zero_entails_null_deref`):
  `NULL` has zero extent, so any pointer with in-bounds index `0` is non-`NULL`.
  *(Current Existing-C passes still evaluate `(a)` and `(b)` independently
  without cross-discharging via this entailment; see [contract §C09.3](QCORE_CONTRACT_SPEC_r0_3.md#c093-null-dereference-the-property-cnullderefv1-and-quant-null-deref).)*
- **Existence `(a)` does NOT entail extent `(b)`**
  (`null_deref_does_not_entail_checks_zero`,
  `an_unknown_extent_is_never_in_bounds`): a one-past-the-end pointer or a
  non-`NULL` pointer with unknown extent satisfies `(a)` while failing `(b)`, so
  `quant null-deref #B;` remains a strictly weaker, separate projection.
- **Grade multiplication across parts** (`relevant_obligates_a_stated_part`,
  `zero_lower_obligates_no_part`): a caller's obligation for each stated part is
  the use's demand grade multiplied by that part's per-use demand (`one`), so a
  lower bound `≥ 1` obligates `(a)` and every stated part, whereas lower bound
  `0` obligates none.
- **Existence and extent never discharge liveness**
  (`existence_and_extent_do_not_discharge_liveness`): neither `(a)` nor `(b)`
  implies `(c)`, because a dangling pointer to a freed allocation still names a
  historical identity and size while violating temporal liveness.

#### 3. Two-Level Contract Surface (`quant deref #B;`)

To avoid repetitive boilerplate without hiding orthogonal obligations, `.qv`
bounds contracts support a **two-level surface** (`crates/qcore-contract/src/deref_parts.rs`):

| Written `.qv` clause | Canonical elaborated part(s) |
|---|---|
| `quant deref #B;` | `quant null-deref #B;` + `quant checks B, Idx #i: #i == 0;` + `quant live #B;` |
| `quant null-deref #B;` | Existence `(a)` alone (`quant null-deref #B;`) |
| `quant checks B, Idx #i: P;` | Extent `(b)` alone (`G05c`) |
| `quant live #B;` | Liveness `(c)` alone (checked against `resource_owned(B, nonnull)` on audit routes) |

Before any analysis consumer inspects a declaration, the `bounds_contract_v1`
loader elaborates every `quant deref #B;` shorthand into its three canonical
parts and idempotently merges agreeing explicit overlaps
(`elaboration_preserves_meaning`, `merging_an_agreeing_overlap_is_idempotent`),
while rejecting conflicting extent predicates (`quant-deref-conflict`).

> **Implementation & Provenance.** `crates/qcore-core/src/usage.rs`,
> `crates/qcore-contract/src/{bounds_contract,deref_parts}.rs`,
> [`QTT_ALIGNMENT.md`](QTT_ALIGNMENT.md); Lean: `QCore.Usage`,
> `QCore.UsageDemand`, `QCore.NullDerefGrade`
> (`relevant_accepts_body_iff_inferred`, `zero_lower_imposes_nothing`,
> `moved_question_is_sequential_addition`, `branch_join_matches`,
> `caller_obligation_is_grade_admission`), `QCore.DerefDecomposition`;
> user decisions `2026-10-10` / `2026-10-11`, cards `NULL-SAFETY-PROPERTY`,
> `NULL-DEREF-QTT-GROUNDING`.

---

<a id="l08-credit-live-obligations-cost-and-access-quantities"></a>

## L08. Credit, live obligations, cost, and access quantities

### Conceptual Motivation

Quantitative types in Quirin govern four distinct operational concepts that look
numerically similar — nonnegative numbers attached to dimensions or identities —
but obey fundamentally incompatible conservation and destruction laws: **credit**,
**live obligations**, **cost bounds**, and **footprint access shares**.

### 1. Spendable Credit (`quant<credit, q>`)

`quant<credit, q>` denotes pre-authorized capacity along dimensions in `q`
(such as byte allocation quota or work units) that `MAY` be split, transferred
between scopes, and spent by operations whose contracts consume credit:

- **Dimension conservation on split:** splitting a credit bundle into `q_1` and
  `q_2` `MUST` conserve each dimension (`q_1 + q_2 == q` without overflow).
- **Abandonment boundary:** specific profiles (such as `error region` pack
  discharge in `§L12` or `EpochCreditRecoveryV1` `error(drop)` in `§L16`) `MAY`
  explicitly account for unused strict credit abandoned on an error edge, but
  **abandoning credit `MUST NOT` discharge or forgive any live custody
  obligation**.

### 2. Live Duty Summaries (`quant<live, q>`)

`quant<live, q>` summarizes the count or volume of outstanding live duties along
dimensions in `q`:

- **Summary, never identity replacement:** `quant<live, q>` is a quantitative
  projection of the live-custody ledger `L`, **never** a substitute for tracking
  individual resource identities. Equal numeric totals (`live_after == 1`) `MUST
  NOT` permit leaked allocation `r_1` to be masked by destroying unrelated
  allocation `r_2`.
- **Mandatory disposition:** every live duty `MUST` be explicitly released,
  rolled back through a validated cleanup action, or transferred to an
  identified custodian (`§L09`).
- **Dereference liveness (`(c)`):** in Existing-C contracts, `quant live #B;`
  names this section's live-custody obligation for pointer formal `B` (`§L07.4`);
  neither pointer non-nullness (`(a)`) nor in-bounds extent (`(b)`) discharges it.

### 3. Cost Upper Bounds (`cost.total` and `cost.peak`)

Cost quantities state upper bounds on execution behavior, **never** spendable
tokens or transferable capabilities:

- **`cost.total`** accumulates monotone operational effort or event counts
  additively across sequential and parallel execution.
- **`cost.peak`** bounds maximum simultaneous occupancy (or another declared
  high-water metric) relative to entry.
- **No wall-clock conflation:** a parallel work or effort bound (`effort` in
  `FlatEpochBoundV1`) `MUST NOT` be presented as elapsed wall-clock time or
  latency without a separate verified scheduling model (`bound effort as latency;`
  is unconditionally refused at `Q639`).

### 4. Footprint Access Shares (`FULL = 1024`)

Access shares authorize reads and writes on a **specific live identity and
byte/protocol footprint**:

- **Unit and exclusivity:** the design fixes full exclusive authority at
  `FULL = 1024` (implemented as `FULL_ACCESS` in `model.concurrent14` and `FULL`
  in `ScopedReadSharesV1`). Mutating ordinary non-atomic memory requires
  reconstructed `FULL` exclusive authority over the written footprint (or an
  exclusive disjoint byte write lease `[low, high)`; see `§L14.B.2` and `§L16`).
- **Conserved read sharing:** multiple concurrent or overlapping readers `MAY`
  hold positive conserved shares summing to at most `FULL` (or overlapping
  immutable `borrow_utf8` byte windows while excluding overlapping writes).
- **Null has no share:** dereferencing a pointer requires an access share over
  its target identity's footprint; because `NULL` designates no identity, no
  share can exist for `NULL` (`quant null-deref #B;`, `§L07.4`).
- **Atomicity ≠ lifetime or publication:** atomic operations require their own
  memory-order and lifetime protocols (`§L16`–`§L17`). That two memory accesses
  are individually atomic `MUST NOT` be treated as proof of safe publication,
  safe reclamation, or pointee liveness.

### Implementation Representation Boundary

The closed `index.closed` type resolver (`§L06`) is not a general credit-flow
solver over arbitrary `.qr` function bodies. In the compiler and core crates,
the usage checker (`u64`), resource-flow checker (`u64` / `i128`), closed index
evaluator (`u128`), ownership ledger (`ResourceId` / `Place`), and concurrency
checkers (`u16` / `u32` shares, `u32` / `u64` identifiers and generations)
maintain distinct data structures and APIs; any conversion across their
numeric ranges `MUST` be explicit and checked for overflow.

> **Implementation & Provenance.** `crates/qcore-core/src/{resource_flow,concurrency,epoch_accounting,epoch_credit_recovery}.rs`,
> `proof/QCore/{Quant,Flow,Ct03ReadShares,EpochAccounting}.lean`.

---

<a id="l09-ownership-identity-and-borrowing"></a>

## L09. Ownership, lifetime generations, aggregate custody, and borrowing

### Conceptual Motivation

Memory and handle safety in C-interoperable systems requires separating
**address representation** (a raw pointer or integer fd), **unique destruction
responsibility** (ownership), **temporary access constraints** (borrows and
range leases), and **temporal identity** (lifetime generations). Without
generations and explicit parent-child custody rules, allocator address reuse or
container teardown silently corrupts static tracking.

### Raw Pointers, Owned Resources, and Borrows

Quirin's extended type and IR model distinguishes three categories of references
to resources:

1. **Raw pointers (`ptr<const, T>`, `ptr<mut, T>`):** convey a machine address
   representation only, **never** custody or automatic lifetime guarantees. A
   dereference through a raw C pointer is modeled as a temporary use under a
   borrow of an existing live identity, never an exercise of ownership (`§L07.4`).
   Because a destroyed allocation's historical identity still "existed" prior to
   release, existence (`quant null-deref`) never implies liveness (`quant live`).
2. **Owned resources (`own<T>`, `bytes(N)`, `tcp`/`udp`, `endpoints`):** carry
   unique responsibility to dispose of a live identity.
   - **Move (`move x`):** transfers unique custody to a new binding, aggregate
     field, return slot, or task capture, immediately leaving the source binding
     moved/uninitialized (`UseAfterMove`).
   - **Destruction (`drop x` / `close s`):** consumes unique destruction
     authority and requires **quiescence** — zero incompatible active shared
     borrows, mutable borrows, read shares, or byte write leases
     (`DropWhileBorrowed`, `LoanOutstanding`, `LeaseOutstanding`).
3. **Shared borrows and range write leases (`borrow<T>`, `borrow_utf8`,
   `lend<T>`, `lend(b, low, high)`):** grant temporary read or disjoint-slice
   write authority without transferring the duty to destroy the underlying
   allocation. Ending a borrow or lease restores suppressed authority to the
   owner; a borrow or lease itself can never be `drop`ped as an allocation.

### Lifetime Generations (`CT50`)

Every resource identity in the semantic design is paired with a **unique
lifetime generation**:

- **Address or descriptor reuse never revives an identity.** If heap memory is
  freed and `malloc`/`mmap` later returns the exact same numeric virtual address
  (`CT50`), or if `close(fd)` is followed by `socket()` returning the same file
  descriptor integer, or if a source local name is rebound, the new allocation
  receives a **fresh generation** distinct from the retired one. Stale bindings
  or pointers targeting the old generation remain invalid (`UseAfterDrop` /
  destroyed-generation access).
- **Bounded integer-ID models:** any checker or IR that represents resources by
  plain integers (the `u64` alias `ResourceId`, the `u32` aliases
  `SharedResourceId`, `AdvancedResourceId`, `ExternalResourceId` and
  `NativeResourceId`, or `(task_id, alloc_ordinal)` in `ScopedOwnedTasksV1`) `MUST` either enforce single-assignment uniqueness per
  ID or import an explicitly verified generation mapping; it `MUST NOT` equate
  numeric pointer equality with generation equality.

### Aggregate Custody, Footprints, and Parent-Child Cascades

A resource's footprint `MAY` encompass heap byte intervals, OS file/socket
descriptors, shared kernel protocol cells (`CT42`), or nested child resource
identities (`Place` paths `RECORD.FIELD...`):

- **No implicit child erasure:** releasing or moving a parent struct or array in
  general C or IR models **does not**, by itself, release child allocations,
  close embedded OS descriptors, or cancel registered reactor callbacks.
- **Explicit aggregate rules:** cascading destruction or partial movement is
  admitted only where a checked model or contract accounts for every child
  explicitly — for example, `.qr` source records (`§L14.B.3`), where
  `qcore_core::ownership` tracks per-leaf paths (`Compose`, `MoveOut`,
  `WriteField` refill into a hole, and `Drop` releasing live owned leaves in
  declaration order), or Existing-C indexed resource collections and teardown
  contracts (`§L18`, [contract §C04.8 / §C04.19](QCORE_CONTRACT_SPEC_r0_3.md)).

### Core Ownership IR Boundary

`qcore_core::ownership` provides checked operations for `Acquire`, `Move`,
`Copy`, `BorrowShared`, `BorrowMut`, `LendRange`, `EndBorrow`, `Read`,
`ReadRange`, `Write`, `WriteRange`, `Compose`, `ReadField`, `WriteField`,
`MoveOut`, `DropField`, `Drop`, `TransferOut`, and `Branch` over supplied
`ResourceId`, `Place`, and `Footprint` inputs. Calling that core API does **not**
by itself provide general source-level borrow lifetime inference, unrestricted C
pointer alias analysis, C struct ABI layout discovery, or automatic `Send`/`Sync`
derivation (`§L16`).

> **Implementation & Provenance.** `crates/qcore-core/src/{ownership,transfer,socket_footprint}.rs`,
> `proof/QCore/{Ownership,AggregateOwnedField,OwnedFieldCopyReach,WriteLend,Ct50LifetimeIdentity}.lean`;
> cards `AGGREGATE-OWNED-FIELD`, `OWNED-FIELD-ROUTE-REACH`, `SLICE-PERMISSIONS-WRITE-LEND`, `CT50`.

<a id="l091-quant-variants"></a>

### L09.1. Quant variants

#### Concept & Motivation

Many systems APIs return a scalar status code whose runtime value determines whether ownership or protocol state changed — for example, a function taking `p` that consumes `p` on `0` and leaves caller custody intact on non-zero error codes, or a socket `bind(s)` transitioning `s` from `Raw` to `Named` on success while keeping `Raw` on failure. Rather than forcing an immediate syntactically fused branch at the call site, a **`quant variant`** seals an exit-specific fragment of resource or protocol authority inside a linear, move-only result value indexed by the subject's runtime generation $\rho$.

- **Implemented vs. Proposed boundary:** The **custody / empty** fragment (`custody(p)` vs. `none`, optionally carrying `code: i32`) is source-checked and compiled to C11 on **`source.quant-variant.v1`** ([G02d.4](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02d4-delayed-custody-results-sourcequant-variantv1), paired with [contract §C04.13.2](QCORE_CONTRACT_SPEC_r0_3.md#c04132-packing-a-return-code-disposition)). Protocol-state fragments (`protocol(s, State)`) and general source-level variant constructors remain **Proposed** target design ([design BNF](QCORE_DESIGN_GRAMMAR_r0_3.bnf); neither `G01e` effect clauses nor `G02bc` error payloads establish `protocol` arm checking).

```qcore
quant variant BindResult(s) {
    Bound carries protocol(s, Named);
    Failed(code: i32) carries protocol(s, Raw);
}
```

#### Generation Indexing & Partition Judgment (`pack` / `move` / `open`)

In `fn bind(s: Socket) -> quant variant BindResult(s)`, the formal parameter `s` denotes the **entry lifetime generation** $\rho$ of the actual argument. Every call and move substitutes $\rho$; rebinding a local variable or reusing a numeric descriptor/pointer address cannot alter $\rho$. Each declared arm specifies one authority fragment $\Phi_i(\rho)$:
- `none`: empty fragment;
- `custody(s)`: unique live destruction responsibility in domain $L$;
- `protocol(s, S)` *(Proposed)*: exclusive authority in domain $K$ to rely on and transition protocol state `S` for generation $\rho$ (granting neither raw byte access shares nor destruction rights).

Writing $D = F \oplus P$ for a compatible disjoint partition of the current resource/protocol state, introduction, transfer, and exhaustive elimination follow three linear state transitions:

```text
pack i   F ⊕ Φᵢ(ρ)                 → F,  v : V(ρ) sealed with (i, Φᵢ(ρ))
move v   F,  v : V(ρ)               → F,  w : V(ρ) with the same seal
open v   F', v : V(ρ), observed i   → F' ⊕ Φᵢ(ρ), arm i's payload data
```

1. **Construct (`pack`):** Partitions $D = F \oplus \Phi_i(\rho)$ at the exact construction point, leaving residual frame $F$ available and sealing $\Phi_i(\rho)$ into `v` (for `G01e`-justified protocol transitions, entry/exit states, guards, reads/writes, and frame preservation must discharge on every path producing tag $i$).
2. **Move and alias protection (`move`):** Every `quant variant` value — including an arm carrying `none` prior to matching — is **strictly move-only** (copying, implicit discard, overwrite, or raw byte reconstruction refuses). While sealed, its subject $\rho$ must outlive the package, and **any** operation through any alias that requires or could invalidate the sealed authority (including opaque calls with unproven effects) refuses; operations on disjoint generations proceed normally.
3. **Exhaustive elimination (`open`):** An exhaustive consuming `match v` (one explicit arm per declared tag) removes the package and checks each arm under current residual frame $F' \oplus \Phi_i(\rho)$ (never restoring a stale pre-call snapshot). Joins across paths require a common instantiated type $V(\rho)$ and correlated `(tag, payload, fragment)` state — summing scalar live counts or joining tag and authority independently is unsound.
4. **C11 lowering & conformance boundary:** Emits a private discriminant tag plus optional `int32_t` payload (evaluating match scrutinees once, reading only active payloads, erasing generation indices after static verification, and exporting no public C struct ABI by default; the integer return-code adapter in [contract §C04.13.2](QCORE_CONTRACT_SPEC_r0_3.md#c04132-packing-a-return-code-disposition) invokes the underlying C symbol once without extra ABI wrappers).

> **Implementation & Provenance.** `compiler/tests/quant_variant.rs`, `proof/QCore/QuantVariant.lean`; grammar [G02d.4](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02d4-delayed-custody-results-sourcequant-variantv1), contract [§C04.13.2](QCORE_CONTRACT_SPEC_r0_3.md#c04132-packing-a-return-code-disposition).

---

<a id="l10-resource-flow-algebra-and-loop-bounds"></a>

## L10. Resource-flow algebra and loop/recursion bounds

### Conceptual Motivation

Even when individual resource identities are valid, interprocedural summaries
and loops must track **net live resource delta**, **minimum entry capacity**,
**peak transient occupancy**, and **cumulative cost** across sequential blocks,
branches, and iterations — without ever confusing an upper bound ("at most `n`
iterations") with an exact live allocation count ("exactly `n` live objects").

### The Four-Component Flow Algebra

A resource flow summary is a four-tuple `(Δ, R, P, T)` evaluated componentwise
across tracked resource/cost dimensions (`crates/qcore-core/src/resource_flow.rs`),
where `[x]+ = max(0, x)` componentwise:

- `Δ` (`i128` signed integer): **exact net live change** from entry to exit;
- `R` (`u64` natural): **minimum entry live quantity** required so intermediate
  live holdings never become negative;
- `P` (`u64` natural): **peak live increase** above the entry level reached at
  any intermediate point; and
- `T` (`u64` natural): **cumulative operational cost** (`cost.total`).

#### Primitive Flows and Sequential Composition

```text
Acquire(q, c) = (  q,  0,  q,  c )
Release(q, c) = ( -q,  q,  0,  c )

F1 ; F2       = ( Δ1 + Δ2,
                  max(R1, [R2 - Δ1]+),
                  max(P1, [Δ1 + P2]+),
                  T1 + T2 )
```

All component operations use checked `u64` and `i128` arithmetic; any arithmetic
overflow rejects the flow check (`Q200`).

#### Mutually Exclusive Branch Joins

- **Equal net change (`Δ1 == Δ2`):** `Flow::join_branches` joins two mutually
  exclusive branches with identical exact net change `Δ` by taking componentwise
  maxima of entry requirements, peaks, and total costs:
  `(Δ, max(R1, R2), max(P1, P2), max(T1, T2))`.
- **Unequal net change (`Δ1 != Δ2`):** a branch join whose surviving arms have
  different net live changes `MUST` either preserve separate exit post-states or
  reject (`Q200`). An implementation `MUST NOT` invent an "optional" live count
  by averaging or taking `max(Δ1, Δ2)`. Furthermore, numeric flow balance
  (`Δ == 0`) **never** proves identity-level custody agreement (`Q046` / `Q103`):
  dropping `r1` in one arm and `r2` in the other has equal `Δ = -1` yet leaves
  incompatible live-identity ledgers.

### Loop Bounds: Exact Iteration vs. At-Most Iteration (`R06`)

Repeating a non-neutral loop body `F` with `Δ != 0` exposes a critical soundness
distinction: `F^n` describes a loop that executes **every single one of `n`
iterations**, whereas a loop with an early break or runtime condition that runs
*between `0` and `n` times* does **not** exit with exact net change `n * Δ`.

`summarize_general_loop` enforces this separation explicitly (`R06` repaired):

- **`Exact(n)` (`Flow::repeat_exact(n)`, legacy alias `bounded_loop(n)`):**
  computes the exact `n`-fold sequential composition `F ; F ; ... ; F`.
- **`AtMost(n)` (including historical `Terminating(n)` and `Widened(n)`):**
  represents an upper bound on iteration count (`0 ..= n` executions).
  - When `n == 0`, `AtMost(0)` yields the zero identity flow `(0, 0, 0, 0)`.
  - When `Δ == 0` (a **zero-net** loop body that releases everything it acquires
    each turn), `AtMost(n)` conservatively retains one iteration's requirement
    `R` and peak `P` while scaling total cost `T` by `n`.
  - When `n > 0` and `Δ != 0`, `AtMost(n)` **rejects** (`Q200`).

#### Normative Loop and Recursion Summary Rules

1. **Exact-count evidence (`Exact(n)`):** `MAY` produce `F^n` only after both
   the exact count `n` and absence of early loop exits (`break` / `return` /
   unmodeled `raise`) are established (as in `source.loop.v1`, `§L14.C.3`). Even
   then, numeric flow checking `MUST` be paired with identity custody checking
   (`Q099` / `Q054`).
2. **Upper-bound evidence (`AtMost(n)`):** for any body with `Δ != 0`, an
   implementation `MUST` either represent the family of possible post-states or
   reject / return `Unknown`; it `MUST NOT` promote an upper bound `n` into
   exact live custody `n * Δ`.
3. **Conservative cost/peak widening:** widening `P` and `T` over admitted
   iteration counts `MUST NOT` erase distinctions between possible live-resource
   exit states.
4. **Recursion and termination:** summarizing recursive calls requires a checked
   fixed-point condition under explicit assumptions. The base `.qr` sequential
   driver synthesizes restricted non-emitted recursive flow summaries and
   **refuses recursive C emission**, while `source.scalar-calls.v1`,
   `source.resource-calls.v1`, and `source.ffi.v1` require acyclic call graphs.
   Any termination claim is distinct from partial-correctness resource safety
   and `MUST` carry its own explicit evidence.

> **Implementation & Provenance.** `crates/qcore-core/src/resource_flow.rs`,
> `crates/qcore-frontend/src/whole_program.rs`, `proof/QCore/{Flow,SourceLoop,CountedLoopMeasure}.lean`;
> review repair `R06`, card `LOOP-V1`.

---

<a id="part-iii--effects-recoverable-errors-and-fallible-cleanup"></a>

# Part III — Effects, Recoverable Errors, and Fallible Cleanup

<a id="l11-effects-and-exit-maps"></a>

## L11. Effects and exit maps

### Conceptual Motivation

In resource-aware systems code, side effects and recoverable errors are
inseparable from custody accounting: an operation that throws or returns an
error code often leaves a different set of live allocations than its normal
continuation, and assuming an operation cannot fail silently erases cleanup
paths. Quirin models **qualitative capabilities**, **typed error exits**, and
**per-exit custody transitions** explicitly rather than treating error exits as
unmodeled jumps.

### Effect Rows and Capability Admission

An effect row `E` (`crates/qcore-core/src/typed_errors.rs` and route-specific
checkers) declares the set of observable effects and typed error exits a
function or block is permitted to perform:

- **Modeled effect tokens:** `io` (`Effect::Io`), `alloc` (`Effect::Alloc`),
  `mut` (`Effect::Mut`), `unsafe` (`Effect::Unsafe`), `throws<ErrorType>`
  (`Effect::Throws`), plus profile-specific concurrency and floating-point
  comparison tokens (`spawn`, `join`, `may_block`, `partial_eq`, `partial_ord`).
- **Subset admission (`derived ⊆ declared`):** whenever a profile checks
  user-written `effects { ... }` rows, every direct or transitively derived
  effect of the body `MUST` be admitted by the declared row. An explicit empty
  row `effects {}` (or absence of `throws<E>`, `partial_eq`, `partial_ord` where
  inference is forbidden) is a **restrictive semantic claim** that requires
  static discharge, **never** an analysis convenience or wildcard.
- **`source.exec12` synthesis boundary:** the base sequential profile
  (`source.exec12`, `§L14`) synthesizes an allocation effect allowance and
  controlled model-failure operations (`qcore_acquire`, `qcore_check`), and
  refuses nonempty user `contract { ... }` blocks (`Q021`). It does **not**
  derive general user-declared effect rows and arbitrary result contracts from
  unrestricted source annotations; verification consumers `MUST NOT` report
  structurally retained annotations on that route as proved.

### Per-Exit Ledger Maps and Owned Error Payloads

Every call or region boundary distinguishes its **normal continuation** from
each reachable **non-normal exit** (`return`, `error(ErrorName)`, and any
cancellation safe point admitted by the profile):

1. **Exit-indexed transitions:** in `typed_errors`, a call summary carries an
   explicit normal `LedgerTransition` (`released`, `acquired` over `ErrorLedger`)
   and an explicit per-error-name map of `LedgerTransition`s and derived effects.
   Omitting an error branch from a summary asserts that the error exit is
   statically unreachable under the profile's contract.
2. **Path-sensitive merge discipline:** when two control-flow paths merge at the
   same exit kind, quantitative upper bounds (`cost.total`, `cost.peak`) `MAY`
   be joined conservatively, but **incompatible live-resource ledgers `MUST` be
   preserved as separate post-states or rejected** (`Q300` / `Q103` / `Q046`).
   No merge `MAY` average or intersect live identities.
3. **Owned error payloads remain live duties:** an error variant carrying a
   resource payload (`MoveToErrorPayload` in `typed_errors` / `error_region`, or
   `V(own<Buffer>)` in `ScopedErrorPayloadV1`) transfers unique ownership of
   that resource into the active error value. Matching an error arm, logging the
   error, or retagging/replacing the error variant **never** implicitly destroys
   or excuses its owned payload (`Q300` / `Q309` / `Q600`).
4. **C calling-convention boundary:** a foreign C function returning a negative
   integer (`-1`), `NULL`, or another sentinel value has **returned normally**
   in the C calling convention unless an explicit checked adapter (`succeeds`,
   mandatory nullable-acquisition branching in `source.extern-call.v1`, or a
   `c.resource` postcondition relation) maps that C return outcome onto a Quirin
   typed error exit. External contract consumers — **never** integer sign or
   pointer nullability by heuristic — define which C outcomes correspond to
   normal vs. error post-states ([contract §C04](QCORE_CONTRACT_SPEC_r0_3.md)).

> **Implementation & Provenance.** `crates/qcore-core/src/typed_errors.rs`,
> `crates/qcore-frontend/src/whole_program.rs`, `proof/QCore/TypedErrors.lean`.

---

<a id="l12-error-regions-and-checked-quantitative-opening"></a>

## L12. Error regions and checked quantitative opening

### Conceptual Motivation

Structured `try / catch` blocks in systems languages fail to prevent leaks when
different failure points inside a region have acquired different subsets of
resources, or when a handler catches an error and forgets to run rollback
actions. Quirin solves this with **path-sensitive sealed cleanup packs** and
**linear quantitative opening (`quant open`)**: catching an error seals the exact
outstanding duties created inside the region along that failure path into a
move-only capability that the handler `MUST` explicitly open and `discharge`
before exiting.

### Core `CleanupPack` and `RegionOp` Calculus

In the core error-region model (`crates/qcore-core/src/error_region.rs`,
`proof/QCore/ErrorRegion.lean`), entering `error region R` snapshots the
enclosing entry `ErrorLedger` (`baseline`) and tracks an ordered sequence of
region-local responsibilities:

- **Tracked region actions (`CleanupAction`):**
  - `Destroy { resource: ResourceId, noexcept: bool }` — registered when the
    region acquires or adopts a live resource identity (`Drop(id)` or
    `MoveToErrorPayload(id)` on a region-local resource removes its pending
    destruction action before an error edge is taken);
  - `Rollback { name: String, noexcept: bool }` — registered by `Defer`; and
  - `credit_remainder: u64` — unspent strict region credit remaining from
    `AddStrictCredit(n)` minus `SpendStrictCredit(k)`.
- **Sealing on an error edge:** when `Raise(E)` transfers control out of region
  `R` to a matching `catch (E)` clause, the region seals a path-specific
  `CleanupPack { region: R, error: E, baseline, actions, credit_remainder }`.
  Different error paths inside `R` (for example across `RegionOp::Branch`)
  `MAY` reach `catch (E)` with different sealed packs; **packs `MUST NOT` be
  merged quantitatively in any way that loses which concrete resource identities
  exist on each path**. An error payload (`MoveToErrorPayload`) remains separate
  from the sealed `CleanupPack` unless an explicit transfer relation connects
  them (`Q309`).
- **Checked opening (`quant open`) and linear `discharge`:**
  - Opening a sealed pack via `quant open R as B` creates a **linear cleanup
    capability** bound to `B`; opening **never** discards or runs the pack's
    contents by itself.
  - `quant open` is valid **only** in the matching `catch` header of region `R`:
    it `MUST NOT` appear outside a catch context (`Q302`), select a different
    region's pack (`Q303`), open the same pack twice (`Q304`), or authorize
    destruction of an outer resource absent from `R`'s region authority (`Q302`).
  - `discharge B;` validates that every `CleanupAction` in `B`'s pack is total
    and `noexcept` (`Q308`), executes the actions in reverse acquisition (LIFO)
    order (`CleanupEvent::Destroy`, `CleanupEvent::Rollback`), explicitly
    accounts for any abandoned strict credit (`CleanupEvent::AbandonCredit`),
    restores the region's `baseline` ledger, and consumes the linear capability
    `B` **exactly once**.
  - Before the handler exits (normally or via rethrow/error), every opened or
    nonempty cleanup obligation `MUST` be discharged (`Q305`, `Q306`, `Q307`),
    and all active borrows and child-task authorities (`CheckedRegionTaskCreditV1`,
    `Q670`–`Q677`) `MUST` be quiescent before destructive cleanup runs. A
    handler that fails any of these obligations `MUST NOT` receive a
    resource-safe exit certificate.

### Diagnostic Ownership (`Q302`–`Q309`)

The `ErrorRegion` pass (`qcore-frontend`, `qcore-verification::driver`, and
`qcore_core::region_task_credit`) enforces eight pinned diagnostic conditions:

| Code | Normative refusal condition |
|---|---|
| `Q302` | Core error-region model violation (e.g., `quant open` outside catch, releasing an outer resource outside region authority, strict-credit underflow) **or** a `.qr` source-region / composed-epilogue shape boundary listed below. |
| `Q303` | `quant open` names a region identifier other than the originating region of the sealed `CleanupPack`. |
| `Q304` | A single cleanup pack / linear cleanup binding is opened or discharged twice. |
| `Q305` | A `catch` handler returns or exits normally while its opened cleanup binding remains undischarged. |
| `Q306` | A **nonempty** sealed `CleanupPack` (having pending cleanup actions or nonzero strict credit remainder) reaches a `catch` clause that omits `quant open`. |
| `Q307` | An error exit leaves a `catch` handler while an opened cleanup pack remains undischarged. |
| `Q308` | A cleanup action sealed into or discharged from a pack is not total and `noexcept`. |
| `Q309` | A single resource identity is held simultaneously in an error payload and in a sealed cleanup pack. |

### `.qr` Sequential Source Error Regions (`source.region.v1`)

While the base `source.exec12` body driver rejects `error region` statements,
the dedicated sequential source route `source.region.v1` (card
`QUANT-REGION-SOURCE`, grammar `G02r`) lowers source-written error regions
directly into `qcore_core::error_region` with zero new reserved words:

```qcore
error Failure { Failed; }

fn main() -> i32 {
    error region setup {
        let h = qcore_acquire(1, true, Failure);
        credit 8;
        effort 3;
        qcore_check(false, Failure);
    } catch (Failure) quant open setup as pack {
        discharge pack;
    }
    return 0;
}
```

- **Admitted sequential body statements:** `let NAME = qcore_acquire(ID, true, ERROR);`,
  `drop NAME;` (for region-acquired resources), `credit N;`, `effort N;`, and
  the decided error edge `qcore_check(false, ERROR);`.
- **Admitted handler statements:** `discharge BINDING;` (plus an empty handler
  body when the sealed pack is empty and `quant open` is omitted).
- **Named `Q302` boundaries on `source.region.v1`:** every shape outside the v1
  envelope refuses explicitly by name at `Q302` rather than falling through
  silently:
  - `REGION-NORMAL-EXIT` — region body with no decided error edge;
  - `REGION-MULTIPLE-CATCH` — more than one `catch` clause on a region;
  - `REGION-MULTIPLE` — more than one `error region` in a single entry;
  - `REGION-PROPAGATE` — an uncaught error edge escaping the region (`Q300` owns
    undeclared error names);
  - `REGION-FAILING-ACQUIRE` — using `qcore_acquire(..., false, ...)` as the
    region's error edge;
  - `REGION-IN-ARM` — nesting an `error region` inside an `if / else` branch arm;
  - `REGION-HANDLER-BODY` — any handler statement other than `discharge BINDING;`;
    and
  - `REGION-CATCH-PAYLOAD-BINDING` — a `catch (...)` pattern other than a single
    declared error type name.

### Composed Epoch-Task Epilogue Regions (`source.epoch.tasks.v1`)

On the composed epoch-and-scoped-task route `source.epoch.tasks.v1` (`§L16`,
grammar `G02f`), a creator entry `MAY` write one `error region` epilogue inside
its `thread scope` to reconcile quantitative credit, task-allocated buffers, and
adopted parent resources after concurrent work completes (cards
`REGION-CREDIT-JOINS`, `CT24-CLEANUP-TASK-COMPOSITION`, `REGION-JOINS-DUTIES`,
and `REGION-JOINS-LOAN`):

1. **Post-join placement and `CT24` composition:** by default, the creator's
   `error region` `MUST` follow every `spawn` and `join` in the task scope —
   an ordering fact derived from checked task IR spans and enforced at
   `Q600` / `Q302` (placing a region inside a worker, outside the creator's
   `thread scope`, or writing a second region refuses at `Q302`). Card
   `CT24-CLEANUP-TASK-COMPOSITION` narrows placement refusals solely for a
   direct handled `error region` inside the exact `Result::Ok` arm of a direct
   creator-scope spawn `match`; attempting cleanup while a child task or borrowed
   share remains unjoined still refuses at the task ledger's `Q600` (`ScopeBusy`
   / `LoanOutstanding`, naming both the preempted owner and unjoined borrower
   under `CT24-CLEANUP-PREEMPTED`).
2. **Task-buffer pack duties (`REGION-JOINS-DUTIES`):** besides `credit N;`,
   `effort N;`, and `qcore_check(false, ERROR);`, the composed region body
   admits the task route's real heap allocation `let NAME = qcore_buffer_new(INT)?;`
   (sealing a real `Destroy` cleanup duty into the pack) and `drop NAME;` of a
   region-acquired buffer (removing that duty), erasing the handled region into
   its straight-line twin in the task half (`REGION-JOINS-ERASURE-FIT` checks
   span fit at `Q302`). Sequential `qcore_acquire` stays refused (`Q302`),
   dropping an outer binding not acquired by the region refuses at `Q302`
   (`REGION-OUTER-LIVE-RESOURCES`), colliding with an entry binding or acquiring
   one name twice refuses at `Q302`, and holding two simultaneously live owners
   inside the region refuses at the task checker's `Q600`
   (`REGION-JOINS-MULTI-OWNER`, because the second `?` failure edge would leak
   the first allocation).
3. **Owner adoption via `move NAME;` (`REGION-JOINS-LOAN` / `CT25`):** the
   composed region body also admits `move NAME;`, which **adopts** a live
   creator-scope `own<Buffer<T>>` binding's destruction duty into the region's
   cleanup pack. Combined with worker `read ... share ...` loans during the
   preceding task scope (`read;` in the epoch plan, discharging
   `T08-COMPOSE-LOAN`), this expresses `CT25` end-to-end: workers read a parent
   buffer concurrently, `join` expires every read share, and the epilogue
   `error region` opens and discharges the adopted owner on recovery. Adopting a
   name not bound by the entry or taking the same owner twice refuses at `Q302`;
   adopting a non-buffer refuses at `Q500`; discharging while a share remains
   active refuses at `Q600` (`LoanOutstanding`); and pairing an acquisition with
   an adopted owner retains `REGION-JOINS-MULTI-OWNER` (`Q600`).

> **Implementation & Provenance.** `crates/qcore-core/src/{error_region,region_task_credit}.rs`,
> `crates/qcore-frontend/src/{source_region,source_epoch_tasks}.rs`,
> `crates/qcore-verification/src/driver.rs`; Lean: `QCore.ErrorRegion`,
> `QCore.SourceRegion`, `QCore.RegionTaskCredit`, `QCore.RegionJoinsDuties`,
> `QCore.RegionJoinsLoan`; cards `QUANT-REGION-SOURCE`, `REGION-CREDIT-JOINS`,
> `CT24-CLEANUP-TASK-COMPOSITION`, `CT24-CLEANUP-PREEMPTED`,
> `REGION-JOINS-DUTIES`, `REGION-JOINS-LOAN`.

---

<a id="l13-fallible-cleanup-policies-and-cancellation"></a>
<a id="l13-fallible-cleanup-and-cancellation"></a>

## L13. Fallible cleanup and cancellation

### Conceptual Motivation

Unlike in-memory heap deallocation (`free`), operating-system and protocol
release primitives (`close`, `fclose`, `fsync`, TLS shutdown) can fail at
runtime, and different APIs attach completely different custody meanings to a
failed close. Simultaneously, asynchronous instruction-level thread cancellation
destroys structured invariants unless cancellation is restricted to explicit
safe points.

### The Five Fallible-Release Custody Policies

Every fallible release contract or cleanup model `MUST` state what happens to
resource custody both on normal completion and on each modeled failure class.
The language and contract design distinguishes five semantic policy categories:

| Cleanup policy | Custody meaning on success vs. failure |
|---|---|
| **Infallible** | Checked preconditions exclude cleanup failure under the profile; destruction succeeds and consumes custody. |
| **Consumes on attempt** | Once the release call is attempted, the caller's custody obligation is consumed even if the OS/callee reports a teardown error (e.g., POSIX `close(fd)` where retrying the descriptor is unsafe). |
| **Returns on failure** | A failed release attempt preserves or returns an identified live custody obligation to the caller for retry or fallback disposal. |
| **Poisons on failure** | A failed release transitions the resource into an explicit poisoned or residual obligation whose remaining permitted operations are separately specified. |
| **Indeterminate external outcome** | The profile cannot establish whether the underlying external endpoint was released; an unresolved custody obligation is retained and **complete cleanup `MUST NOT` be claimed**. |

*(These five categories are semantic contract/model classifications — see
[contract §C04](QCORE_CONTRACT_SPEC_r0_3.md) — not standalone `.qr` surface
keywords.)*

### Advanced-Control Model, Failure Preservation, and Safe-Point Cancellation

In `crates/qcore-core/src/advanced_control.rs` (`model.sequential` Cycle 15 IR):

1. **Attempt-consuming IR boundary:** `AdvancedOp::Drop` removes a resource from
   its local live sequence when attempting its `DestructorSpec` and records any
   `DestructorOutcome::Error`. That model `MUST` therefore be read strictly as
   an **attempt-consuming** abstract destructor model — **never** as proof that
   arbitrary failing C or OS teardown functions consume their obligations;
   `Returns on failure` and `Poisons on failure` require explicit state tracking
   and contract validation.
2. **Primary error vs. secondary cleanup errors (`DoubleError`):** a primary
   `Raise` error or `Cancelled` outcome `MUST` remain distinguishable from any
   subsequent destructor failures triggered during unwinding. `advanced_control`
   records secondary cleanup errors alongside `PrimaryFailure` (`AdvancedOutcome::DoubleError`)
   rather than overwriting the original cause. Cleanup runs in **LIFO**
   (reverse-acquisition) order unless a profile proves a different dependency
   order.
3. **Explicit safe-point cancellation:** `RequestCancellation` never interrupts
   execution asynchronously at arbitrary instructions; cancellation is observed
   **exclusively** at explicit `SafePoint` operations and at the modeled
   `Return` boundary, triggering deterministic LIFO unwinding of live resources.
4. **One-shot vs. multi-shot handler captures:** `HandlerKind::OneShot` handlers
   `MAY` capture `CaptureMode::Owned` resources under their linear (at-most-once)
   invocation discipline, whereas `HandlerKind::Resumable` and
   `HandlerKind::MultiShot` handlers `MUST NOT` capture owned resources because
   repeated resumption would duplicate unique custody. This rule defines a core
   IR capability boundary, **not** general `.qr` algebraic-effect handler or
   multi-shot closure syntax (`§L07`, `§L16`).

> **Implementation & Provenance.** `crates/qcore-core/src/advanced_control.rs`,
> `proof/QCore/AdvancedControl.lean`.

---

<a id="part-iv--executable-source-language-and-native-c-interoperability"></a>

# Part IV — Executable Source Language and Native C Interoperability

<a id="l14-the-current-executable-source-profile"></a>

## L14. Executable source architecture and base sequential profile

### Conceptual Motivation

Rather than presenting a partially implemented monolithic compiler that silently
drops checks when two features meet, Quirin organizes `.qr` compilation around a
**single, deterministic, fail-closed route-selection table** (`SOURCE_ROUTES` in
`crates/qcore-verification/src/driver/source_routes.rs`, mechanized in
`QCore.RouteSelection`). Each route lowers a precisely specified `.qr` grammar
into checked semantic passes, an executable reference observation (for emitted
executable profiles), and C11 output — refusing any unmodeled cross-feature
combination at the owning pass's diagnostic code.

### Deterministic Route-Selection Dispatch Order (`SOURCE_ROUTES`)

`compile_elaborated_source` evaluates pure syntactic selectors over `SOURCE_ROUTES`
with **first-match semantics** (23 entries in pinned order, card
`ROUTE-SELECTION-ORDER-RESTATED`):

1. **Contract-module guard by construction:** every table entry except the final
   sequential route carries `source_only: true`, ensuring `module contract;`
   files are rejected on all specialized executable routes and lowered only on
   the final sequential route (`QCore.RouteSelection`).
2. **Closure hosting gate (`W48-QR-CLOSURES`):** only the two scoped-task
   entries (`source.std-io-tasks.v1` and the scoped-task route) carry
   `ClosureRule::Hosts`; spelling a `|...|` closure on any other route refuses
   immediately at `Q600` naming the route's exact boundary residual.
3. **Pinned priority tiers (first match wins):**
   - **Tier 1 — Quantitative protocol variants and unsafe C struct FFI:**
     (1) `source.quant-variant.v1` (`quant variant` declarations/clauses, `§L09`),
     followed by (2) `source.ffi.v1` (`#import std.ffi.unsafe;` or `struct!`,
     `§L14.6`) — placed ahead of all general routes so a file importing C can
     never reach a route that would omit the file-wide unsafe-FFI assumption.
   - **Tier 2 — Checked concurrency profiles (most specific first, `§L16`):**
     (3) `source.bitfield-concurrency.v1` (`write obj.field` layout tasks) →
     (4) `source.epoch.tasks.v1` (epoch scope + `concurrency checked;`) →
     (5) `source.epoch.bound.v1` (check-only `epoch` scope) →
     (6) `source.mutex.guard.v1` (`mutex` / `with lock` / `protected`) →
     (7) `source.std-io-tasks.v1` (`has_source_tasks ∧ has_source_std_io ∧ ¬has_source_extern`,
     `§L14.D.1`) →
     (8) the scoped-task route (`source.tasks.copy.v1` / `ScopedOwnedTasksV1`).
     Placing concurrency above sequential I/O, extern, buffer, and socket routes
     guarantees that combining `spawn` with uncomposed features refuses at the
     concurrency checker (`Q600`).
   - **Tier 3 — Declared external C calls (`§L14.E`):**
     (9) `source.std-io-extern.v1` (`extern` calls + `std.io` lines, `§L14.D.1`) →
     (10) `source.extern-call.v1` (`extern` calls alone). Placing extern routes
     above buffer, string, and socket routes ensures extent-carrying `.qr`
     buffers, UTF-8 string views, and sockets cannot silently decay into
     extentless C04 `u8` pointers.
   - **Tier 4 — Event reactor, string bridge, resource helpers, buffers, records, HTTP, strings, and sockets:**
     (11) `source.socket.v1` composed with `std.event` (`event` + `socket`
     without buffer/string/HTTP, `§L14.D.3`) →
     (12) `source.event.v1` (`std.event`, including `COMPOSE-EVENT-LINES`,
     `§L14.D.2`) →
     (13) `source.string.v1` buffer bridge (`borrow_utf8(...)`, `§L14.B.1`) →
     (14) `source.resource-calls.v1` (`own<bytes>` / `borrow<bytes>` /
     `lend<bytes>` helper functions, `§L14.B.4`) →
     (15) `source.scalar-calls.v1` multi-function copyable records (`record` +
     no buffer + `> 1` function, `§L14.B.3`) →
     (16) `source.buffer.v1` (`std.buffer` or single-function `record`,
     `§L14.B.2`–`§L14.B.3`) →
     (17) `source.http.v1` (`std.http` over `std.socket` and bound views,
     `§L14.D.4`) →
     (18) `source.string.v1` (`std.string` literals, slices, `concat`, text I/O,
     `§L14.B.1`) →
     (19) `source.socket.v1` (`std.socket` TCP/UDP + `std.io` scalar lines,
     `§L14.D.3`).
   - **Tier 5 — Scalar calls, formatted standard I/O, and base sequential cleanup:**
     (20) `source.scalar-calls.v1` on statement-head `print(` (`§L14.0`) →
     (21) `source.std-io.v1` (`std.io` / `std.logger` lines, delegating to
     `source.scalar-calls.v1` when scalar-call syntax causes `std.io` entry-only
     lowering to refuse, `§L14.D.1`) →
     (22) `source.scalar-calls.v1` (general scalar functions/expressions, `§L14.0`) →
     (23) the base sequential cleanup route (`source.exec12`, fallback catch-all).

### Base Sequential Cleanup Profile (`source.exec12`, Grammar `G02`)

The fallback sequential source-to-C route (`source.exec12`) intentionally checks
and lowers a minimal canonical body vocabulary focused on resource acquisition,
cleanup, and sequential control (`§L14.C.2` extends it with decidable `if / else`):

```qcore
module executable;
error Failure { Failed; }
fn main() -> i32 {
    let buffer = qcore_acquire(1, true, Failure);
    qcore_check(true, Failure);
    drop buffer;
    return 0;
}
```

1. **Controlled model intrinsics:** `qcore_acquire(RESOURCE_ID, BOOL, ERROR_NAME)`
   takes a `u32` literal resource ID, a literal Boolean success flag, and a
   declared error-type name (defaulting to `Failure` when none is declared);
   `qcore_check(BOOL, ERROR_NAME)` takes a literal Boolean flag and error-type
   name. Both are controlled verification-model operations lowered to generated
   C instrumentation and tagged outcomes — **never** claims to model a complete
   production heap allocator. Because `qcore_acquire` publishes a custody
   identity without a nominal payload type, explicit local type annotations on
   its binding (including `own<T>`) refuse at `Q020` (`SOURCE-ANNOTATION-ENFORCEMENT`),
   while integer literal `let` bindings admit an optional `i32` annotation.
2. **Destruction, abstract result transfer, and return values (`R07`):**
   - `drop name;` destroys the live modeled resource identity (`OwnershipOp::Drop`).
   - `move name;` transfers custody to the abstract result payload
     (`OwnershipOp::TransferOut`, tracked in `CompiledModule.result_profile` via
     `CustodyPayloadProfile::SeparateIdentityLedger`), **not** general variable
     reassignment or raw C pointer movement.
   - `i32` functions require an explicit signed `i32` integer literal return
     checked in wider precision so the full two's-complement range
     `-2147483648 ..= 2147483647` is accepted (`R07` repaired; out-of-range
     literals reject); `unit` functions require bare `return;` (represented as
     `0` in the reduced C ABI).
3. **Direct zero-argument calls, purity gate, and declaration hygiene:**
   - Direct calls inside `source.exec12` take zero arguments, resolve to a
     declared source function, and discard the callee's scalar result.
   - For C emission, every reachable callee `MUST` be **observationally pure**
     under the reduced model: empty resource trace, normal exit outcome, and
     empty transferred custody payload. Recursive callees and effectful callees
     are rejected for C emission (`Q500`).
   - Every parameter annotation on `source.exec12` is validated against the
     `§G02t` fixed-width scalar table (`Q020`, `W48-QR-SCALAR-TYPES-R2`), and
     every uncalled `extern fn` declaration in an executable or library module
     is validated against the `§C04.1` ABI vocabulary (`Q020`,
     `W48-QR-SCALAR-TYPES-R3`).
4. **Explicit exclusions from `source.exec12`:** runtime-varying conditions,
   `match`, loops, arbitrary mutation/assignment, general arithmetic, nonliteral
   returns, `error region` bodies, source borrowing, concurrency, and foreign
   calls are **not** executable on `source.exec12`. Structural parsing of those
   constructs `MUST NOT` be reported as implemented lowering.

### Scoped Task Execution Profiles Summary (`source.tasks.copy.v1` & `ScopedOwnedTasksV1`)

Modules declaring `#import std.concurrency;` and `concurrency checked;` lower
through the dedicated checked task IR (`crates/qcore-frontend/src/source_tasks.rs`,
`crates/qcore-core/src/task_execution.rs`) and emit real native threads on
Linux (C11 `<threads.h>`), macOS (emitted `pthread` adapter), and Windows
(emitted `winpthreads`-derived C11 thread subset):

- **`source.tasks.copy.v1` ([grammar `G02a`](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02a-scoped-copy-task-execution), [recovery report](reports/w6-task-recovery/README.md)):**
  supports immutable fixed-width scalar and float `copy` captures, first-order
  named leaf workers and `|...| -> T { ... }` spawn closures (`W48-QR-CLOSURES`),
  scalar/record/`unit` worker results, dynamic thread-creation failure
  (`SpawnError`), explicit creator `join` (native OS join failure aborts before
  frame reclamation), two-arm `Result::Ok` / `Result::Err` handlers that
  preserve prior sibling handles until their explicit `join` (with arm-local
  bindings confined to their arm), and `?` propagation (infallible `?` consumes
  an empty-error `Result` without synthesizing a dead error branch; nonempty
  error rows retain both normal and error exits).
- **`ScopedOwnedTasksV1` ([grammar `G02b`](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02b-owned-buffer-task-execution), [owned-task report](reports/w6-owned-source-tasks/README.md), [worker-allocation report](reports/w6-worker-allocation/README.md)):**
  extends scoped tasks (alongside `ScopedReadLoansV1`, `ScopedReadSharesV1`, and
  `ScopedErrorPayloadV1`) to affine `own<Buffer<T>>` heap allocations holding
  one `§G02t` scalar or float payload (`own<Buffer>` defaults to `own<Buffer<i32>>`).
  Root and worker `qcore_buffer_new` calls may fail with `AllocError` using
  schedule-independent `(32-bit task_id, nonzero 32-bit alloc_ordinal)`
  generations. Successful `move` capture consumes the parent binding, whereas
  failed `spawn` restores original parent custody and sibling state. Checked
  worker exits return at most one owned input or fresh local allocation while
  explicitly dropping all others (owned `main` entry results have no external
  custodian and reject). Conserved read shares (`FULL = 1024`), disjoint byte
  write leases `lend PLACE LOW..HIGH` (`WRITE-LEND-TASKS` /
  `WRITE-LEND-TASKS-EXERCISE`), tagged `own<Buffer>` error payloads, and
  per-leaf record captures (`W48-QR-TASK-RECORD-CAPTURES`) are governed in full
  by **[§L16 (Checked Concurrency)](#l16-checked-concurrency-required-rules-and-current-limitations)**.

### Bounded External C Calls Summary (`source.extern-call.v1`)

Modules opting in with `#import std.extern;` (`Q022` when absent) execute flat
sequential foreign calls through `source.extern-call.v1`
([grammar `G02d`](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02d-bounded-extern-calls),
[v1 report](reports/archive/language-and-tooling/extern-call-v1/README.md),
[out/stored report](reports/archive/language-and-tooling/extern-call-f1/README.md)):

- **Explicit contract selection only:** every contracted callee `MUST` resolve in
  an explicit `CompileOptions` bundle loaded by `resource_contract_v1` (`Q023`
  when absent; inline `contract { ... }` blocks on called `extern fn`s refuse at
  `Q027` rather than being trusted).
- **Closed C04 bridging table (`Q024`–`Q025`):** admits fixed-width scalars and
  IEEE floats by value; `resource_borrowed` on admitted pointers;
  `resource_consumed(p, nonnull)`; `resource_return_none()` with `unit` or
  scalar/float results; `resource_stored(p, owner, nonnull)` on `ptr<mut, u8>`
  paired with a distinct `resource_owned(owner, nonnull)` container parameter
  (permanently removing `p` from the caller's ledger under a recorded certificate
  premise); and **at most one** fresh custody acquisition per call via the
  mandatory null-branching error adapter on `resource_return_owned(nullable)`,
  `resource_out(p, nullable)` (`ptr<mut, ptr<mut, u8>>`), or
  `resource_return_reallocated(p, nullable)` (`EXTERN-MODES`), with optional
  reverse-acquisition-order error-arm dispositions (`EXTERN-ERR-REL`). C04
  pointers carry extentless `u8` projections, recorded explicitly on every
  certificate.
- **Derived effects, custody-free scalar bindings, and trust boundary:** a call
  derives `{Io}`, plus `Alloc` on fresh acquisition and `Mut` when any parameter
  is `ptr<mut, ...>`. Non-acquiring scalar/float calls `MAY` bind
  `let NAME = CALLEE(ARGS) [succeeds LITERAL];` (`PIPE-SCALAR-RESULT`, emitted C
  exiting `70` if a declared `succeeds` control disagrees at runtime) in a
  value namespace strictly disjoint from resource identities, and feed typed
  single-operation value statements (`§L14.0.4`). Every verdict reached via an
  external contract is conditional on `ContractTrust::Declared`, never a proof
  of the C library's implementation. Unchecked waivers (`§L14.1`), `.qrlib`
  manifests, verified headers, `extern library` bindings, and `struct!` FFI are
  specified in **`§L14.E` (`§L14.1`–`§L14.8`)**.

> **Implementation & Provenance.** `crates/qcore-verification/src/driver/source_routes.rs`,
> `crates/qcore-verification/src/driver.rs`,
> `crates/qcore-frontend/src/{source_tasks,source_extern}.rs`; Lean:
> `QCore.RouteSelection`, `QCore.TaskExecution`, `QCore.ExternCall`; repair
> `R07`, cards `ROUTE-SELECTION-ORDER-RESTATED`, `EXTERN-MODES`, `EXTERN-ERR-REL`,
> `PIPE-SCALAR-RESULT`, `W48-QR-CLOSURES`.

---

<a id="l140-scalar-source-calls-and-explicit-standard-output"></a>

### L14.0. Scalar source calls, fixed-width integers, and IEEE floating point

#### L14.0.1. Scalar source functions, explicit `print(expr)`, effect inference, and C assembly (`source.scalar-calls.v1`)

`source.scalar-calls.v1` ([grammar `G02s`](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02s-pure-scalar-functions-sourcescalar-callsv1),
[report](reports/source-scalar-calls-v1/README.md)) checks and lowers
multi-function `.qr` programs over fixed-width scalars, IEEE floats, copyable
scalar records (`§L14.B.3`), immutable non-shadowing `let` bindings, direct
acyclic calls, checked left-to-right expressions, and explicit stream output:

1. **Output forms and capability gating:**
   - The legacy function-style statement `print(EXPRESSION);` requires
     file-local `#import std.io;`, evaluates one `i32` expression, and writes
     its signed decimal representation followed by `\n` to `stdout`. It neither
     requires nor consults `std.fmt` because it has no rendering selector, and
     it is restricted to pure `i32` modules (refused at `Q020` in any module
     using `§G02t` non-`i32` types, comparisons, conversions, `bool`, or line
     statements).
   - Every module on this route also admits the `source.std-io.v1` line
     statements `print dec|hex X;`, `log LEVEL dec|hex X;`, and `flush;`
     (`§L14.D.1`, plus `|>` sinks desugaring to them), parsed by `source.std-io.v1`'s
     own reader (`Q030` on shape violation, `Q028` on `borrow NAME` because this
     route carries no custody model, `Q030` on `log_level`).
2. **Total body checking and `io` effect inference (`Q020`):**
   - **Every function definition `MUST` be checked**, including uncalled helper
     functions and `module library;` definitions; uncalled bodies `MUST NOT`
     bypass type, annotation, literal-fold, effect, or acyclicity checks.
   - An **omitted** `effects` clause infers `io` whenever a function performs
     direct output (`print(...)` or a line statement) or calls an `io`-bearing
     callee transitively. Writing `effects { io }` explicitly permits derived
     `io`, whereas writing `effects {}` explicitly forbids both direct and
     transitive `io` (`Q020`). By contrast, `throws<ArithmeticError>`,
     `partial_eq`, and `partial_ord` are **never inferred** (`§L14.0.3`,
     `§L14.7`). Nonempty source `contract { ... }` blocks refuse at `Q021`
     (empty `contract {}` remains admissible).
3. **Closed-entry observation vs. library mode and hard limits:**
   - Evaluating an executable `main` entry (zero parameters, `i32` or `unit`
     result) publishes only *that closed invocation's* scalar/error outcome and
     ordered `stdout`/`stderr` lines — **never** a universal proof that every
     helper function is overflow-free on arbitrary caller arguments.
   - `module library;` checks all definitions without publishing a closed
     observation or standalone C.
   - Enforced structural/evaluation limits are **256** functions, expression/call/binary-chain
     depth **64**, **16,384** IR nodes, **16,384** tokens per function, and
     **100,000** evaluation steps (`Q020` on exhaustion, never truncated).
4. **C11 emission and `c_program::assemble`:**
   - Emitted C preserves call structure and left-to-right evaluation while
     guarding every arithmetic operation **before** C evaluation so signed
     overflow and `MIN / -1` or `MIN % -1` undefined behavior are never executed.
   - Programs without output statements emit zero bytes on `stdout`/`stderr`
     (quiet emission); an uncaught arithmetic guard fault exits immediately with
     status **`86`** without printing diagnostic text.
   - The scalar backend contributes file-scope function/struct fragments and an
     entry body to `c_program::assemble`, deduplicating common I/O includes and
     prelude helpers by fragment key and emitting exactly one C `main`. Fragment
     assembly is an emission mechanism only — it **does not** permit cross-route
     calls into functions owned by other execution profiles (`Q020`).

#### L14.0.2. Fixed-width scalar types, checked non-wrapping arithmetic, and recorded targets

Card `W48-QR-SCALAR-TYPES` ([grammar `G02t`](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02t-fixed-width-scalar-types),
`crates/qcore-core/src/scalar_types.rs`, Lean `QCore.ScalarTypes`) promotes the
eleven `§G05` `scalar_abi` types to first-class `.qr` scalar types. Their rules
are **normative across every `.qr` route that admits them**:

| `.qr` type | Semantic domain | Emitted C11 type | `std.fmt` `dec` | `std.fmt` `hex` |
|---|---|---|---|---|
| `bool` | `{ false, true }` | `bool` (`<stdbool.h>`) | `false` / `true` | Refused (`Q020`/`Q050`/`Q052`) |
| `i8`, `i16`, `i32`, `i64` | Signed mathematical integers `-2^(w-1) ..= 2^(w-1) - 1` | `int8_t` .. `int64_t` | Signed decimal | `0x` + `2`/`4`/`8`/`16` uppercase two's-complement hex digits |
| `u8`, `u16`, `u32`, `u64` | Unsigned mathematical integers `0 ..= 2^w - 1` | `uint8_t` .. `uint64_t` | Unsigned decimal | `0x` + `2`/`4`/`8`/`16` uppercase zero-padded hex digits |
| `isize` | Signed range of recorded target width `W ∈ {32, 64}` | `ptrdiff_t` | Signed decimal | `0x` + `W/4` uppercase hex digits |
| `usize` | Unsigned range `0 ..= 2^W - 1` of recorded width `W` | `size_t` | Unsigned decimal | `0x` + `W/4` uppercase hex digits |

1. **Exact mathematical arithmetic — never wrapping, never C UB:**
   - `+`, `-`, `*`, `/`, `%` (truncating division toward zero) and unary `-`
     (`0 - x`, which on unsigned types succeeds only at `0`) compute the exact
     mathematical integer result over the ten integer types and `MUST` fault
     whenever the exact result lies outside the operand type's range, when a `/`
     or `%` divisor is `0`, or on signed `MIN / -1` and `MIN % -1`.
   - Emitted C `MUST` decide every fault from the operand values **prior to**
     executing the operation using explicit range checks (`q_scalar_*`), never
     evaluating C signed overflow or shift/division undefined behavior.
   - Any fault decidable from constants — including every operation along a
     closed executable entry and every literal-only subexpression inside any
     called or uncalled function — `MUST` refuse at **check time** (`Q020` /
     `Q024` / `Q053`). Where an operand is known only at runtime, the fault is
     governed by `ArithmeticError` (`§L14.0.3`).
2. **Zero implicit conversions and `EXPR as TYPE`:**
   - Binary arithmetic and comparison operands `MUST` have the exact same type;
     bindings, call arguments, and `return` expressions `MUST` match their
     declared target type without implicit widening or narrowing.
   - Integer literals adopt their context's integer type, default to `i32` when
     unconstrained, and `MUST` fit in that type's range (including exact signed
     minima such as `-2147483648` for `i32`).
   - `EXPR as TYPE` (binding tighter than binary operators and looser than unary
     `-`) is the **only** conversion construct: it is **total** when every
     source value fits in `TYPE` (widening), and **checked** otherwise —
     faulting (`OutOfRange`) whenever the runtime or constant value lies outside
     `TYPE`, never truncating bits or reinterpreting sign. Nothing converts to
     or from `bool`.
3. **Strict separation of `bool`:** comparisons (`==`, `!=`, `<`, `<=`, `>`,
   `>=`) produce `bool`. `bool` supports `==` and `!=` only; ordering
   comparisons, arithmetic, unary `-`, `as` conversions, and `hex` rendering on
   `bool` all refuse.
4. **`isize`, `usize`, and the builder's `recorded target`:** `isize` and
   `usize` take the pointer width of the builder's explicit recorded target
   (`quirin --recorded-target FILE`, a total-or-reject record specifying
   `pointer_width` `32` or `64`, `triple`, and `compiler`) and **no other**.
   Naming `isize` or `usize` without a recorded target `MUST` refuse (`Q020` /
   `Q051` / `Q098` / `Q600`); with one, that target is published in the verdict's
   declared assumptions (`recorded target: ...`, `§L20`) and emitted C
   `_Static_assert`s `SIZE_MAX`, `PTRDIFF_MIN`, and `PTRDIFF_MAX` so building
   under any other pointer width fails compilation.

#### L14.0.3. Typed `ArithmeticError` (`?`, `match`, and `raise`)

Card `W48-QR-SCALAR-TYPES-R2` (`§G02t`, Lean `QCore.ScalarTypedErrors`) provides
the builtin payload-free error type `ArithmeticError` with three variants:

- `ArithmeticError::Overflow` (tag `1`) — exact `+`, `-`, `*`, `/`, `%`, or
  unary `-` result outside the type's range, including signed `MIN / -1` and
  `MIN % -1`;
- `ArithmeticError::DivisionByZero` (tag `2`) — zero divisor in `/` or `%`; and
- `ArithmeticError::OutOfRange` (tag `3`) — checked `as` conversion (including
  `float as INT`, `§L14.7`) whose source value is not representable in the
  target integer type.

`ArithmeticError` obeys the explicit typed-error row discipline of `§L11`:

1. **Explicit `throws<ArithmeticError>` permission:** a function `MAY` raise
   `ArithmeticError` only when its signature declares
   `effects { ... throws<ArithmeticError> ... }`; this permission is **never**
   inferred. Inside a permitted function, any potentially faulting operation
   with at least one non-constant operand raises its classified variant instead
   of aborting. Operations over literal-only operands are still folded and
   refused at **check time** regardless of the effect row
   (`QCore.ScalarTypedErrors.static_fault_is_dynamic_raise`), and runtime-faulting
   operations in functions without `throws<ArithmeticError>` refuse at check time.
2. **Mandatory call consumption (`?` and `match`):** calling a function whose
   row declares `throws<ArithmeticError>` `MUST` be explicitly consumed (`Q020`
   otherwise; applying `?` or `match` to an infallible callee also refuses):
   - **Propagation (`CALL?` / `CALL?;`):** requires `throws<ArithmeticError>` in
     the caller's own row and immediately exits the caller with the unchanged
     error variant; or
   - **Handling (`match CALL { Result::Ok { value: v } => { ... } Result::Err { error: e } => { ... } }`):**
     requires one `Ok` arm and one `Err` arm (in either order) over a value-returning
     callee, binding `v` or `e` strictly within its arm, with every block ending
     in `return`, `raise`, or `match`.
3. **Explicit `raise`:** `raise e;` re-raises an enclosing `Err`-arm binding and
   `raise ArithmeticError::VARIANT;` raises a variant directly; both require
   `throws<ArithmeticError>` (`Q020`).
4. **C11 lowering and exit codes (`81`–`83`):** emitted functions with
   `throws<ArithmeticError>` use `q_scalar_try_*` helpers that classify faults
   from operands prior to evaluation and return an `int` error tag (`0` on `Ok`,
   `1..=3` on `Err`) while writing normal results through an out-pointer. When
   `main` propagates `ArithmeticError`, it publishes that variant as the entry's
   error outcome and exits with status **`80 + tag`** (`81` `Overflow`, `82`
   `DivisionByZero`, `83` `OutOfRange`) preserving all lines already written.

#### L14.0.4. Fixed-width scalars across buffer, extern, task, loop, mutex, epoch, and string routes

Cards `W48-QR-SCALAR-TYPES-R2` through `W48-QR-SCALAR-TYPES-R6` enforce the exact
same scalar typing, checked conversion, and `recorded target` laws everywhere
other `.qr` routes admit scalar values:

- **Declaration hygiene across all routes (`R2`, `R3`):** the six entry-only
  routes (`source.std-io.v1` `Q030`, `source.buffer.v1` `Q051`, `source.event.v1`
  `Q058`, `source.string.v1` `Q068`, `source.socket.v1` `Q080`, `source.http.v1`
  `Q090`) refuse any additional unmodeled source function rather than leaving
  its annotations unchecked; and every executable or library module validates
  all `extern fn` declarations (called or uncalled) against the `§C04.1` ABI
  table (`Q020`).
- **Byte-buffer route (`source.buffer.v1`, `R3`, `R5`):** record fields
  (`§L14.B.3`) admit any `§G02t` type; every scalar binding carries a single
  static type (buffer indices, stored bytes `0..=255`, and `lend` bounds require
  `i32` and reject other types at `Q052`); and the route admits both
  `let NAME = OPERAND as TYPE;` and single-operation value statements
  (`let NAME = A OP B;`, `A CMP B`, `-NAME`). Because every value on
  `source.buffer.v1` is known at check time (per iteration inside `for`), all
  faults decide and refuse at check time (`Q051`–`Q053`).
- **Extern and composed extern + I/O routes (`source.extern-call.v1`,
  `source.std-io-extern.v1`, `R3`):** bound extern results take their declared
  `§G02t` type; `succeeds` controls and scalar call arguments are checked at
  exact parameter/result types (`Q024`); single-operation value statements fold
  check-time-known operands (`Q024` on fault) and require `throws<ArithmeticError>`
  on runtime-only extern results (exiting `80 + tag` on fault); and composed
  `std.io` renders determined values of every `§G02t` type (`Q043`).
- **Scoped tasks and composed task I/O (`source.tasks.copy.v1`,
  `source.std-io-tasks.v1`, `R4`, `R6`):** worker parameters, worker results,
  and `copy` captures admit any `§G02t` type (`TransferKind::FixedScalar`, exact
  type match with zero conversion at `spawn`; task `main` results remain `i32`,
  `unit`, or error). Both parent and workers admit single-operation value
  statements: runtime-operand faults require `throws<ArithmeticError>` (`Q300`)
  and traverse the task route's checked error exit, requiring zero leaked owned
  buffers (`Q600`). Worker `ArithmeticError`s cross `join` unchanged for `?` or
  `match` consumption (`QCore.ScalarTasks.worker_error_reaches_creator_unchanged`;
  user redeclaration of `ArithmeticError` refuses at `Q013`), composed I/O
  renders every type (`Q050`), and the F02 relational trace alphabet encodes
  fixed-width worker outcomes (`QCORE_TASK_RESULT TASK TYPE NUMBER`,
  `QCore.TaskTraceFixedOutcomes`).
- **Typed counted loops (`source.loop.v1`, `R5`):** `for i in 0..BOUND` admits a
  check-time `BOUND` of any integer type (`bool`, floats, and unrecorded
  `isize`/`usize` refuse at `Q098`), typing `i` at `BOUND`'s type; because
  `1 <= BOUND <= 4096` and `BOUND` fits the type, `i += 1` never overflows and
  executes `BOUND` exact iterations (`QCore.ScalarLoopBounds.the_loop_runs_exactly_count_times`).
- **Typed `Buffer<T>` payloads, `protected TYPE`, epoch tasks, and string/buffer
  composition (`R6`):**
  - `own<Buffer<T>>`, `borrow<Buffer<T>>`, and `lend<Buffer<T>>` (`own<Buffer>`
    = `own<Buffer<i32>>`) fix payload type `T` from `qcore_buffer_new(VALUE)`.
    Byte extents of range write leases and byte reads equal `T`'s exact byte
    width (`1`, `2`, `4`, `8`, or recorded `isize`/`usize` width; `QCore.ScalarPayloads`).
    Byte writes into `Buffer<bool>` refuse (`Q600`) so non-`{0, 1}` bytes cannot
    forge invalid `bool` representations; error-variant payloads remain `i32`
    `own<Buffer>` (`Q300`); and F02 refuses traces carrying typed buffer-payload
    events rather than narrowing them (`W48-QR-F02-TYPED-PAYLOADS`).
  - `protected TYPE` on `source.mutex.guard.v1` protects `sizeof(TYPE)` bytes
    (`protected i32` = `protected 4`), masking writes into `protected bool` to
    the low bit.
  - `source.epoch.tasks.v1`'s task half admits the full fixed-width task tier
    while keeping epoch byte declarations explicit and routing worker
    `ArithmeticError`s onto join-failure recovery edges (`QCore.ScalarEpochTasks`).
  - Composed `source.string.v1` + `std.buffer` (`§L14.B.1`) admits typed
    conversions, value statements, and typed-index `for` loops under `Q077`
    without altering `borrow_utf8` mutation-exclusion windows
    (`QCore.ScalarComposedString`).

<a id="l147-floating-point-and-its-limitations"></a>

#### L14.7. IEEE 754 floating point and its limitations

Cards `W48-QR-FLOATING-POINT`, `W48-QR-FLOAT-IEEE-LEAN-MODEL`, and
`W48-QR-FLOAT-BUFFER-PAYLOADS` (user decisions `2026-09-30`, grammar `§G02t`)
add `f32` (IEEE 754 binary32) and `f64` (IEEE 754 binary64) across the scalar
route (including copyable records), extern and composed extern + I/O routes,
unchecked `unsafe extern fn` declarations, `.qrlib` v1/v2 manifests, `source.ffi.v1`
(`struct!` fields, `#c_ffi` functions, helpers), scoped tasks (parameters,
results, `copy` captures, record leaves, closures, and `source.epoch.tasks.v1`'s
task half), `source.buffer.v1` (records, conversions, value statements, loop
bodies), composed `source.string.v1`, `Buffer<f32>` / `Buffer<f64>`, and
`protected f32` / `protected f64`.

##### Surface Typing, Operations, Effect-Gated Comparisons, and Renderings

1. **Literals and strict type separation:** decimal float literals (`D.D`,
   `D.DeX`, `DeX`) adopt their context's float type (`f64` when unconstrained)
   and `MUST` be parsed and rounded **once** directly at that target format; any
   literal that overflows to `±inf` or any nonzero literal that underflows to
   `±0.0` `MUST` be refused (`Q020` / `Q024` / `Q053`). Integer literals are
   never floats, float literals are never integers, and `f32` and `f64` never
   mix without explicit `as`.
2. **Total IEEE arithmetic (no `%`):** `+`, `-`, `*`, `/`, and unary `-`
   compute IEEE 754 round-to-nearest, ties-to-even results and **never fault**:
   `NaN` and `±inf` are ordinary values (`x / 0.0` yields `±inf` or `NaN`).
   Floating-point `%` is not supported and `MUST` be refused.
3. **Explicit `partial_eq` and `partial_ord` comparison effects:** every float
   comparison `MUST` be refused unless the enclosing function's declared
   `effects { ... }` row explicitly lists **`partial_eq`** (for `==`, `!=`) or
   **`partial_ord`** (for `<`, `<=`, `>`, `>=`), neither implying the other and
   neither ever inferred. On `source.scalar-calls.v1`, comparison effects
   propagate transitively across calls so callers `MUST` declare every
   comparison effect derived by their callees; on task routes each function's
   row governs its own body; and on buffer/string routes the entry's row governs
   all top-level and loop-body statements.
4. **Conversions (`as`) and renderings (`dec`, `hex`):**
   - `INT as f32|f64` and `f64 as f32` round to nearest, ties to even, and never
     fault (`f64` beyond `f32` range becomes `±inf`); `f32 as f64` is exact; and
     `f32|f64 as INT` truncates toward zero and `MUST` fault — refused at check
     time when known, otherwise raising `ArithmeticError::OutOfRange` under
     `throws<ArithmeticError>` — whenever the truncated value lies outside `INT`
     (`NaN` and `±inf` included).
   - `dec` renders the shortest round-trip decimal in ECMAScript
     `Number::toString` layout (`-0` for `-0.0`), `hex` renders normalized C
     hexadecimal floating point (`0x1.8p+1`, `0x1p-1074`, `-0x0p+0`), and `NaN`,
     `+inf`, `-inf` render as `nan`, `inf`, `-inf` under both, using Quirin's
     own self-contained C helpers (`q_float_dec_*`, `q_float_hex_*`) without C
     `printf`/`snprintf` locale or library formatting.
5. **Named IEEE verdict assumption:** every checked result over a program
   holding an `f32` or `f64` value records the **IEEE floating-point premise**
   in its declared assumptions (`§L20`), and emitted C refuses to compile if the
   build-checkable parts of that premise fail.

##### The Bit-Pattern Model and What Lean Proves

- **The model:** an `f32` or `f64` value is its IEEE 754 bit pattern (every
  32-bit or 64-bit pattern is a value). The checker, reference interpreters, and
  constant folders evaluate with host IEEE round-to-nearest-even operations, and
  emitted C executes the same operations at runtime; their agreement rests on
  the IEEE premise and is **observed** across boundary tables bit-for-bit (`NaN`
  for `NaN`).
- **Mechanized Lean 4 theory (`W48-QR-FLOAT-IEEE-LEAN-MODEL`):**
  - `QCore.Ieee754` formalizes binary32/binary64 bit-pattern encoding/decoding
    (sign, biased exponent, trailing significand, subnormals, `±0`, `±inf`, and
    payload-unspecified `NaN`), proving non-`NaN` round-trips and the five-class
    partition;
  - `QCore.Ieee754Round` formalizes exact-rational round-to-nearest-even
    (including subnormals and overflow to `±inf`), proving nearestness,
    even-tie selection, monotonicity, exactness on representable rationals, and
    the exact `0.5 ulp` overflow threshold above `MAX_FINITE`;
  - `QCore.Ieee754Ops` (building on `QCore.FloatTypes`) defines `+`, `-`, `*`,
    `/`, unary `-`, the six comparisons, `int -> float`, `f64 -> f32`,
    `f32 -> f64`, and `float -> int` as single-rounding IEEE operations and
    proves the complete special-case table (`NaN` propagation, `inf - inf`,
    `0 * inf`, `0 / 0`, `x / 0`, zero-result signs), `-0 == +0`, `NaN`
    unorderedness, exact correspondence of `partial_eq`/`partial_ord` gates,
    equivalence of `int -> float` with `QCore.FloatTypes.roundNat`, and
    exactness of `f32 -> f64`; and
  - `QCore.Ieee754Vectors32` and `QCore.Ieee754Vectors64` prove by kernel
    computation that all **4,006 rows / 19,171 results** produced by the Rust
    kernel (`compiler/tests/float_ieee_vectors.rs`, which fails on drift) match
    the Lean IEEE model bit-for-bit (`NaN` as `NaN`). Behavior on unenumerated
    operands, emitted C helpers, and the target C compiler remains **observed**
    (the Lean model is a vector-bound specification, not code extraction or C
    compiler refinement).

##### Normative Limitations of Floating-Point Verdicts

Every item below is a **normative boundary** on every verdict over a program
holding an `f32` or `f64` value:

1. **`NaN` and `±inf` are ordinary values; ordering is partial.** Every
   comparison involving `NaN` evaluates to `false` except `!=` (`true`), so
   `x == x` can be `false` and `x < y`, `x >= y` can both be `false`. Functions
   without `partial_eq` / `partial_ord` perform (and on `source.scalar-calls.v1`
   transitively reach) zero float comparisons of that kind
   (`no_undeclared_float_comparison`); no total-order primitive over `NaN` is
   provided.
2. **`NaN` payload and sign bits are neither preserved nor specified.** All
   `NaN` bit patterns denote one semantic `NaN` across `dec`, `hex`, task
   traces, and reference-vs-native stream comparisons; verdicts claim nothing
   about produced `NaN` payload/sign bits, and no source literal spells `NaN`.
3. **Signed zero (`-0.0` vs. `+0.0`).** `-0.0 == 0.0` is `true`, yet the two
   values differ in bit pattern, in `dec` (`-0` vs. `0`), in `hex` (`-0x0p+0`
   vs. `0x0p+0`), and in declared extern/FFI `succeeds` controls (which compare
   bit-for-bit, so `succeeds 0.0` rejects `-0.0`).
4. **Silent rounding precision loss.** `int -> float` and `f64 -> f32` round to
   nearest-even without faulting (`9007199254740993 as f64` yields
   `9007199254740992`, and out-of-range `f64 as f32` yields `±inf`). Float
   literals and literal-only `as` operands round once directly at their target
   type (never double-rounding via `f64`), and literal-only arithmetic folds at
   the operand type.
5. **`float -> int` faults before cast.** `NaN`, `±inf`, and any float whose
   toward-zero truncation falls outside the target integer range fault with
   `ArithmeticError::OutOfRange` (refused at check time when known); emitted C
   tests operands against exact hexadecimal float thresholds prior to casting so
   C float-to-int undefined behavior is never evaluated.
6. **Subnormals are preserved; FTZ/DAZ modes invalidate verdicts.** Flush-to-zero
   and denormals-are-zero processor states cannot be detected by C macros and
   lie outside every verdict; `-ffast-math` is rejected at C compile time.
7. **IEEE binary32/binary64 targets only.** Emitted C `_Static_assert`s
   `FLT_RADIX == 2` together with binary32/binary64 significand, exponent, and
   `sizeof` parameters; non-IEEE targets fail to compile.
8. **Rounding mode fixed to nearest-even.** Dynamic rounding-mode changes
   (`fesetround`, `#pragma STDC FENV_ACCESS`) are unmodeled and place the run
   outside every verdict.
9. **Zero excess intermediate precision (`FLT_EVAL_METHOD == 0`).** Asserted at
   C compile time so x87 80-bit extended evaluation (`-ffp-eval-method=extended`)
   refuses to build rather than computing divergent values.
10. **No fast-math and no FMA contraction.** `__FAST_MATH__` and
    `__FINITE_MATH_ONLY__` fail C compilation (`#error`). Each float operation
    executes in its own helper with `#pragma STDC FP_CONTRACT OFF` under Clang
    under the build premise `-ffp-contract=off` (MSVC `/fp:strict`); builds that
    fuse operations lie outside every verdict.
11. **No cross-platform `libm` ULP identity claim.** External C math functions
    may differ by `<= 1 ulp` across glibc, Apple `libm`, and UCRT; platform
    runs compare only against their own host reference, and `succeeds` controls
    remain call-site runtime checks rather than `libm` proofs.
12. **No float `%`, no `NaN`/`inf` keyword literals, and no hex float literals.**
    `%` on floats refuses; `NaN` and `±inf` are constructed via quotients such
    as `0.0 / 0.0` and `1.0 / 0.0`.
13. **Language-owned `dec` / `hex` renderers (unproved round-trip).** Emitted
    Dragon4 `dec` helpers are validated natively against Rust's shortest
    ECMAScript-layout formatter on boundary tables and seeded samples (observed,
    not mechanized; `W48-QR-FLOAT-RENDER-ROUNDTRIP-PROOF`), and `hex` normalizes
    subnormals (`0x1p-1074`).
14. **Loop bounds and native platform scope.** Float loop bounds are refused on
    every route (`Q098`), both directly and through constant-tier bindings,
    before reading bits. Native float execution gates currently cover
    Linux/Clang (`W48-QR-FLOAT-MSVC-NATIVE` remains a named follow-on).
15. **Check-time evaluation on buffer and string routes.** All values on
    `source.buffer.v1` and composed `source.string.v1` are check-time constants:
    host-evaluated check-time floats and runtime C floats agree by observed
    boundary-vector comparison, and faulting `float -> int` casts refuse at
    check time (`Q053` / `Q077`) with no runtime `OutOfRange` path.
16. **Byte views of `Buffer<f32>` / `Buffer<f64>` payloads.** Every byte pattern
    of width `4` or `8` is a valid IEEE float (admitting byte writes, unlike
    `Buffer<bool>`), in host endianness, with **`NaN` sign/payload bytes
    unspecified** (e.g., `0.0 / 0.0` on `x86_64` differs in sign bit between
    runtime division and Clang `-O2` constant folding; whole-payload traces
    normalize every `NaN` to `nan`).
17. **Task traces and F02 boundary.** Float buffer payloads log as `F32 BITS` /
    `F64 BITS` (`nan` for `NaN`), and F02 encoding refuses runs carrying typed
    buffer-payload events rather than dropping them (`W48-QR-F02-TYPED-PAYLOADS`).
18. **`protected f32` and `protected f64` hold raw bytes.** `source.mutex.guard.v1`
    writes fixed fill bytes without reading cells as floats; IEEE assertions
    there fix only the `4`- and `8`-byte protected footprint sizes.
19. **Entry-row comparison gating on buffer and string routes.** Because those
    routes model only the entry function (including unrolled loop iterations),
    `partial_eq` and `partial_ord` are checked solely against the entry's
    declared `effects` row and recorded on the certificate.

> **Implementation & Provenance.** `crates/qcore-core/src/{scalar_types,scalar_values}.rs`,
> `crates/qcore-frontend/src/source_scalar_calls.rs`,
> `compiler/tests/float_ieee_vectors.rs`; Lean: `QCore.ScalarTypes`,
> `QCore.ScalarTypedErrors`, `QCore.ScalarTasks`, `QCore.ScalarLoopBounds`,
> `QCore.ScalarPayloads`, `QCore.ScalarEpochTasks`, `QCore.ScalarComposedString`,
> `QCore.TaskTraceFixedOutcomes`, `QCore.FloatTypes`, `QCore.Ieee754`,
> `QCore.Ieee754Round`, `QCore.Ieee754Ops`, `QCore.Ieee754Vectors32`,
> `QCore.Ieee754Vectors64`; cards `SOURCE-SCALAR-CALLS-V1`, `W48-QR-SCALAR-TYPES`,
> `W48-QR-SCALAR-TYPES-R2`..`R6`, `W48-QR-FLOATING-POINT`,
> `W48-QR-FLOAT-IEEE-LEAN-MODEL`, `W48-QR-FLOAT-BUFFER-PAYLOADS`.

---

<a id="l14b-strings-owned-byte-buffers-range-lends-and-records"></a>

### L14.B. Strings, owned byte buffers, range lends, and records

<a id="l14b1-immutable-utf-8-string-views"></a>

#### L14.B.1. Immutable UTF-8 string views (`std.string`, `source.string.v1`)

String-view capabilities are gated per file behind `#import std.string;` (`Q015`
when missing; ninth bundled module in `qcore_core::module_imports`). Importing
`std.string` applies strictly to the importing file and grants no mutable
buffer, stream output, formatting, socket, or concurrency capability; quoted
tokens inside structural declaration metadata continue to need no import. String
operations `MUST NOT` be implicitly available as un-imported built-ins.

##### Semantic Representation `{ ptr, len }`, Literals, and Code-Point Slicing

Revision 0.3 models `string` as an **immutable, non-owning UTF-8 byte view**
with semantic representation `{ ptr, len }` (card `STRING-SLICE`, design
[STRING_SLICE_V1](design/STRING_SLICE_V1.md), [report](reports/archive/language-and-tooling/string-slice/README.md)):

- **Zero custody, zero NUL-termination assumptions:** string literals denote
  `static const` UTF-8 bytes; `len(s)` returns byte length in `O(1)`; and a
  view carries zero ownership duty and can never mutate or outlive its backing
  storage. Views are **not** NUL-terminated, and **no implementation `MAY` call
  `strlen` to recover a string's length**. The three-word `{ ptr, len, cap }`
  layout is reserved for future mutable byte slices.
- **Zero globally reserved keywords:** `len` is skipped by lexical reference
  collection only immediately before `(` (preserving pinned `.qv` contracts that
  name parameters `len`), `concat` and `borrow_utf8` are likewise contextual
  before `(`, and `string` is recognized only in `let NAME: string = ...;` type
  ascriptions.
- **Static literal decoding (`Q070`) and static code-point slicing (`Q069`, `Q071`):**
  - Quoted body tokens admit the closed escape set `\\`, `\"`, `\n`, `\t`, `\r`,
    `\0`, and `\xHH`; unknown or truncated escapes, decoded byte sequences that
    are not well-formed UTF-8, or payloads exceeding **`4096` bytes** refuse at
    check time (`Q070`, never truncated).
  - Slice expressions `s[low:high]` count **bytes** and require check-time
    constant bounds (`Q069`) satisfying both `0 <= low <= high <= len(s)` and
    **UTF-8 code-point boundary alignment** at `low` and `high` (`Q071`).
    Boundary violations are hard compile-time refusals — **never** rounded to
    adjacent code points and never deferred to runtime traps (emitted C range
    guards are provably untaken witnesses).
  - Using a string at a scalar/slice-bound/selected-rendering position refuses
    at `Q072`, while passing a string to a C04 `extern fn` refuses via route
    dispatch order (`STRING-SLICE-ABI`). String operations derive **neither
    `Alloc` nor `Mut`** (`Q073` validates the entry's declared `effects` row;
    `Q068` validates flat entry-only shape and certificate rederivation).

##### Stream Output Composition and NUL-Safe Emission (`STRING-SLICE-IO`)

Modules importing `std.string` together with `std.io` and/or `std.logger`
([report](reports/archive/language-and-tooling/string-slice-io/README.md), diagnostics `Q075`–`Q076`) admit
raw UTF-8 text lines without importing `std.fmt`:

```qcore
#import std.string;
#import std.io;
#import std.logger;

fn main() -> i32 {
    let s: string = "hello\0world";
    print s[0:5];
    s |> log info;
    return 0;
}
```

- **Exact byte identity:** `print TEXT;` and `log LEVEL TEXT;` (plus pipeline
  forms `TEXT |> print;` and `TEXT |> log LEVEL;`), where `TEXT` is a literal,
  bound `string`, or inline slice `s[low:high]`, emit the view's exact UTF-8
  bytes with no re-encoding, no escaping, and no extra terminators beyond the
  line's trailing `\n` (and `[LEVEL] ` prefix on `stderr`,
  `QCore.StringSliceIo.the_two_line_kinds_share_one_framing`). Placing a
  `dec`/`hex` selector before a string still refuses at `Q072`; malformed text
  lines, integers at text operands, or `borrow NAME` markers refuse at `Q075`;
  and non-string bindings refuse at `Q076` (while `print bogus;` in a module
  without constructed strings stays on `source.std-io.v1` and refuses at `Q030`).
- **Embedded NUL preservation:** because C `printf("%.*s", ...)` truncates at
  `\0`, emitted C writes text payloads via a length-bounded **`fwrite`** prelude
  helper (aborting on short write rather than truncating output) and derives
  only `{Io}` (`a_program_with_no_text_is_its_own_erasure` when no text is
  rendered).

##### Static `concat` and Range-Wise `borrow_utf8` over `std.buffer`

Cards `STRING-CONCAT` ([report](reports/archive/language-and-tooling/string-concat/README.md)),
`CT07-SLICE-PERMISSIONS` ([report](reports/archive/language-and-tooling/ct07-slice-permissions/README.md)),
and `LOOPS-ON-COMPOSED-ROUTES` (`Q077`–`Q079`) add compile-time string
composition and checked range borrows over owned `std.buffer` storage:

1. **Zero-allocation `concat` (`Q077`):** `let s = concat(PIECE_1, PIECE_2, ...);`
   combines two or more check-time-known pieces — string literals, bound strings,
   inline slices, or `dec(NAME)` / `hex(NAME)` of check-time integers — into a
   single `static const` payload (`<= 4096` bytes) whose length `len(s)` is the
   check-time sum of its pieces' lengths (enabling statically verified HTTP
   `Content-Length` headers, `§L14.D.4`), deriving zero `Alloc` or `Mut` effects.
2. **Range-wise `borrow_utf8` and loop-window exclusion (`Q077`–`Q079`):**
   - `let s = borrow_utf8(b, low, high);` borrows immutable UTF-8 view access
     over byte range `[low, high)` of an owned `std.buffer` binding `b`
     (`0 <= low <= high <= extent(b)` checked at `Q077`; single-argument
     `borrow_utf8(b)` is identical to `borrow_utf8(b, 0, extent(b))`,
     `the_whole_extent_decision_is_the_shipped_one`).
   - **Static UTF-8 check (`Q079`):** the checker reconstructs `[low, high)`
     from `b`'s constant write history and refuses at `Q079` if those bytes are
     unwritten or ill-formed UTF-8 (including range endpoints that split a
     multi-byte code point).
   - **Range-wise mutation exclusion (`Q078`):** `s`'s borrow window runs from
     `borrow_utf8` to the last statement reading `s` or any slice derived from
     `s` (slicing narrows the borrowed subrange; overlapping immutable
     `borrow_utf8` views on `b` are permitted). Within that window, any store
     into `b` overlapping `[low, high)` (decided exactly by `max low < min high`,
     `overlaps_iff_a_shared_byte`) or any `drop b;` (even under an empty
     `[k, k)` loan) refuses at `Q078`, alongside inherited `qcore_core::ownership`
     refusals (`UseAfterDrop`, `NormalPathLeak`, `DoubleDrop`). Writing
     `lend(...)` in a composed string/buffer entry refuses by name at `Q077`
     (`SLICE-PERMISSIONS-COMPOSED-LEND`).
   - **Counted `for` loops across borrow windows (`LOOPS-ON-COMPOSED-ROUTES`,
     `QCore.LoopWindows`):** composed entries holding a buffer admit `for` loops
     (including `§L14.0.4` typed integer bounds, conversions, and float/scalar
     value statements under `Q077`). Loans read after a `for` remain live across
     all iterations; every loop-body store `[i, i + 1)` is checked per iteration
     against live loans (indices outside the lowering-visible tier fail closed
     as conflicting, `loop_body_conflict`), while per-iteration loans
     (`LOOP-PER-ITERATION-LOAN`), body acquire/drop (`LOOP-WINDOW-BODY-CUSTODY`),
     and string/text operations inside loop bodies
     (`LOOP-ON-COMPOSED-ROUTE-BODY-VOCABULARY`) refuse by name at `Q077` (loop
     header/bound/identity errors retain `Q097`–`Q099`).
3. **Named non-goals:** `source.string.v1` does not freeze buffers into owned
   strings (`STRING-SLICE-FREEZE`), concatenate runtime-varying values
   (`STRING-SLICE-CONCAT-RUNTIME`), borrow runtime-varying buffers
   (`STRING-SLICE-BORROW-VARYING`), transport strings across C04 / `.qrlib`
   boundaries (`STRING-SLICE-ABI`), or define string `==`, `<`, hashing, or
   substring search.

<a id="l14b2-owned-byte-buffers-and-disjoint-write-lends"></a>

#### L14.B.2. Owned byte buffers and disjoint write lends (`std.buffer`, `source.buffer.v1`)

`#import std.buffer;` (`Q015` when missing, design
[STD_BUFFER_V1](design/STD_BUFFER_V1.md)) gates `source.buffer.v1`
(`StandardBuffer`, `Q051`–`Q057`), providing **owned, extent-carrying byte
allocations** and **disjoint range write leases**:

```qcore
#import std.buffer;

fn main() -> i32 effects { alloc, mut } {
    let b = bytes(8);
    let left  = lend(b, 0, 4);
    let right = lend(b, 4, 8);
    left[0] = 65;
    right[0] = 66;
    let n = extent(b);
    drop b;
    return 0;
}
```

1. **Acquisition, check-time bounds, and custody (`Q051`–`Q057`):**
   - `let b = bytes(N);` acquires an owned byte buffer of check-time constant
     extent `N ∈ 1..=4096` (`Q053` outside range; spelled `bytes` rather than
     `buffer` so existing fixtures binding `buffer` remain valid; `drop b;` stays
     ungated).
   - `let n = extent(b);` binds `b`'s `i32` extent; `b[i] = v;` stores byte `v`
     (`0..=255` as `i32`) at `0 <= i < extent(b)`; and `let x = b[i];` reads the
     initialized/zero-initialized `i32` byte.
   - **Check-time decidability (`Q052`, `Q053`):** every index, byte value, and
     extent on `source.buffer.v1` `MUST` resolve to a check-time constant (`Q052`
     otherwise, `STD-BUFFER-VARYING-INDEX`), making all out-of-range accesses
     compile-time refusals (`Q053`) and emitted C bounds checks provably untaken
     witnesses.
   - **Custody, crossings, and effects:** `qcore_core::ownership` enforces
     `NormalPathLeak`, `DoubleDrop`, `UseAfterDrop`, and `BindingAlreadyExists`
     at `Q054`. Passing a buffer or write loan to `print`/`log` (`borrow` or
     bare) refuses at `Q055`; passing a buffer to an `extern fn` refuses via
     route order (`STD-BUFFER-EXTERN`). Acquisitions derive `Alloc`, stores
     derive `Mut`, output lines derive `Io`, and float comparisons derive
     `partial_eq` / `partial_ord`, all checked against `effects { ... }` at
     `Q057` with independent certificate rederivation at `Q056`.
2. **Disjoint range write lends (`SLICE-PERMISSIONS-WRITE-LEND`, Lean `QCore.WriteLend`):**
   - `let s = lend(b, low, high);` (`lend` contextual before `(`, zero new
     reserved words) lends exclusive write authority over `[low, high)` of owned
     buffer `b` (`0 <= low <= high <= extent(b)` in the lowering-visible
     constant tier — literal, literal `let`, or `extent`; byte-derived bounds
     refuse by name at `Q053` as `WRITE-LEND-BOUND-FROM-BYTE`).
   - **Coordinate rebasing:** `s[i] = v;`, `let x = s[i];`, and `extent(s)`
     check `0 <= i < high - low` (`Q053` / `AccessOutsideLendRange` at `Q054`)
     and rebase `s[i]` to owner offset `low + i` (`LendRecord`, lowered to a
     displaced `const` pointer in C).
   - **Range-wise exclusivity in `qcore_core::ownership` (`Q054`):** projecting
     onto `OwnershipOp::LendRange`, `WriteRange`, and `ReadRange` guarantees
     that at every step **every byte has at most one write authority**
     (`QCore.WriteLend`): overlapping live lends refuse (`OverlappingLends`;
     empty `[k, k)` lends conflict with nothing, and closed windows release
     their range), owner reads or writes overlapping a live lend refuse while
     disjoint owner accesses succeed, and `drop b;` while any lend is live
     refuses (`DropWhileBorrowed`; `drop s;` and re-lending `lend(s, ...)`
     refuse at `Q052`, `WRITE-LEND-RELEND`).
   - **Iteration-aware loan windows (`LOOPS-ON-COMPOSED-ROUTES`,
     `COMPOSED-WINDOW-REACH`, Lean `QCore.LoopWindows`):** top-level lends
     compose with nested `for` loops; `last_uses` extends a loan's window across
     every iteration of any loop whose body names it, unrolling loop bodies
     under active loans to check every footprint. Crossed lexical loan windows
     (`SLICE-PERMISSIONS-CROSSED-WINDOWS`), acquisitions inside a loan window
     (`WRITE-LEND-NESTED-ACQUIRE`), per-iteration `lend` inside `for`
     (`LOOP-PER-ITERATION-LOAN`), loop-body acquire/drop in a lending entry
     (`LOOP-WINDOW-BODY-CUSTODY`), and unnameable `RECORD.FIELD` custody events
     in `lend_windows` (`WINDOW-FIELD-PATH-REFUSED-NOT-WINDOWED`) all refuse at
     `Q051`.

<a id="l14b3-source-records"></a>

#### L14.B.3. Source records (`source.record.v1`)

Cards `AGGREGATES-V1`, `AGGREGATE-OWNED-FIELD`, `OWNED-FIELD-ROUTE-REACH`,
`W48-QR-USABLE-RECORDS` ([report](reports/w48-qr-usable-records/README.md)), and
`W48-QR-TASK-RECORD-CAPTURES` ([grammar `G07.13`](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md))
provide nominal aggregate records without adding any reserved keyword (`type`
followed by `NAME {` introduces a record declaration; `record` was rejected by
pinned fixture `d02_boundary_record` at `Q000`, and `Word Dot Word` field paths
never collide with `127.0.0.1` float tokens):

```qcore
#import std.buffer;

type Header { tag: u16, flags: bool }
type Packet { head: Header, payload: bytes }

fn main() -> i32 effects { alloc, mut } {
    let raw = bytes(4);
    let pkt = Packet { head: Header { tag: 7, flags: true }, payload: raw };
    pkt.head.tag = 9;
    pkt.payload[0] = 42;
    let moved_out = pkt.payload;
    let refill = bytes(2);
    pkt.payload = refill;
    drop pkt;
    drop moved_out;
    return 0;
}
```

1. **Declarations, nesting bounds, and leaf types:**
   - `type NAME { FIELD: FTYPE, ... }` declares a nominal struct with `1..=32`
     direct fields, maximum nesting depth **`8`**, and at most **`256`** total
     scalar/resource leaves (`Q053` / `Q020`; direct or mutual self-containment
     refuses at `Q051`).
   - Admitted `FTYPE`s are `§G02t` fixed-width scalars and IEEE floats (`f32`,
     `f64`), nested nominal record types, owned `bytes` (on `source.buffer.v1`),
     and `own<Buffer<T>>` (on scoped-task routes). String-view fields refuse by
     name (`AGGREGATE-VIEW-FIELD`).
2. **Construction, field paths, partial moves, refills, `drop`, and `let y = x;`:**
   - **Total construction:** `let r = R { f1: v1, ... };` supplies every
     declared field once in any order; supplying an owned `bytes` /
     `own<Buffer<T>>` binding moves ownership into `r.f` (`OwnershipOp::Compose`),
     consuming the source binding.
   - **Field reads vs. partial moves (`r.path`):** `let x = r.f1.f2;` copies a
     scalar/float or purely copyable sub-record, whereas at an owned leaf it
     performs a **partial move** (`OwnershipOp::MoveOut`), leaving a hole at
     `r.f1.f2` whose second read refuses (`UseAfterMove`); reading or moving a
     whole sub-record that owns any leaf refuses (`Q051` / `Q600`).
   - **Field writes and hole refills (`r.path = v;`):** writes to scalar/float
     paths update the leaf (deriving `Mut` where tracked), and writes to a
     moved-out owned-field hole **refill** custody (`OwnershipOp::WriteField`),
     whereas overwriting a still-live owned leaf refuses (`Q054` / `Q600`
     `OverwriteOfLiveField`), whole sub-record writes refuse (`Q051`,
     `W48-QR-RECORD-FIELD-WHOLE-WRITE`), and reading or writing even a scalar
     field of an owning record after `drop r;` or whole `move` refuses (`Q054`).
   - **Aggregate `drop` and `let y = x;` copy bind:** `r.path[i] = v;`,
     `let x = r.path[i];`, `extent(r.path)`, and `drop r.path;` operate directly
     on live `bytes` leaves; `drop r;` releases every currently live owned leaf
     in declaration order without double-dropping moved-out holes and reports
     normal-exit leaks by exact `r.path` (`drop` on a pure scalar record refuses
     at `Q052`). The copy bind `let y = x;` (`QCore.OwnedFieldCopyReach`)
     duplicates scalars and pure scalar/float records while emitting
     `OwnershipOp::Copy` for any owned buffer, write loan, or record type
     declaring an owned leaf (hollow or full), refusing at `Q054` (`CopyOfOwned`
     / `MoveOfBorrow`).
3. **Route-specific record boundaries:**
   - **`source.buffer.v1`:** full single-entry support above; combining owning
     records with `lend` (`OWNED-FIELD-LEND-WINDOW`) or `for` loop bodies
     (`OWNED-FIELD-LOOP-BODY`) refuses by name at `Q051`.
   - **`source.scalar-calls.v1` (`W48-QR-USABLE-RECORDS`):** modules declaring
     copyable scalar/float records with no buffer and `> 1` function pass and
     return records **by value** (`struct qcore_record_NAME`); owning `bytes`
     fields (`W48-QR-OWNED-RECORD-ACROSS-CALL`), field writes
     (`W48-QR-SCALAR-CALLS-FIELD-WRITE`), and record results on
     `throws<ArithmeticError>` functions (`W48-QR-RECORD-THROWS-RESULT`) refuse
     at `Q020`.
   - **Scoped tasks (`W48-QR-TASK-RECORD-CAPTURES`, `§L16`):** records of
     scalars, floats, nested records, and `own<Buffer<T>>` leaves cross `spawn`
     leaf-by-leaf (`copy` for pure scalar/float records, `move` for owning
     records; distinct `own<Buffer<T>>` leaves may be `lend`-leased to sibling
     workers simultaneously; field writes require `mut` at `Q300`; `bytes`
     fields and records on `source.epoch.tasks.v1` refuse at `Q500` as
     `WRITE-LEND-TASKS-EXTENT` / `W48-QR-TASK-RECORD-EPOCH`).
   - **C Struct FFI (`source.ffi.v1`, `§L14.6`):** `struct!` imports support
     listed-field reads, listed-field writes (`Mut`), `let q = r;` copies, and
     by-value `.qr` helpers under the file-wide unsafe-FFI assumption.

<a id="l14b4-resource-aware-byte-helpers"></a>

#### L14.B.4. Resource-aware byte helpers (`source.resource-calls.v1`)

Card `RESOURCE-HELPER-FUNCTIONS` (`crates/qcore-frontend/src/source_resource_calls.rs`,
`crates/qcore-core/src/resource_calls.rs`) checks and lowers closed, acyclic
multi-function `.qr` programs over `#import std.buffer;` where helper functions
operate on owned and borrowed `bytes` allocations:

- **Signature and call-site modes (`Q020`, `Q100`, `Q021`):** parameters admit
  `i32`, linear `own<bytes> use one` (passed with `move name`), whole-call
  read-only `borrow<bytes>` (passed with `borrow name`), and whole-call mutable
  `lend<bytes>` (passed with `lend(name)`); results admit `unit`, `i32`, or
  `own<bytes>` (`return move name;`). Every helper `MUST` be reached acyclically
  from `main` within bounded expansion limits (**256** functions, **256**
  parameters, call depth **64**, **16,384** statements, **32,768** flattened
  events; nonempty `contract` blocks retain `Q021`, invalid explicit usage
  grades retain `Q100`).
- **Derived minimum-extent summaries (`Q053`):** literal byte reads and writes
  `p[k]` inside helpers synthesize per-parameter `minimum_extents` (`k + 1`) and
  `returned_extent` provenance, checked against exact caller extents at every
  call site before value storage allocation (`Q053`).
- **Zero-stranded-owner allocation-failure discipline (`Q054`–`Q057`):**
  acquiring `bytes(N)` inside a function while **any** `own<bytes>` binding is
  currently live refuses at `Q054`, guaranteeing that an allocation failure can
  never strand an already-owned buffer without cleanup. Flattened call trees
  also pass `qcore_core::ownership` alias/custody verification (`Q054`),
  derived `{alloc, mut}` effect-row checking (`Q057`, inferred when `effects`
  is omitted and enforced when written), and independent `BufferCertificate`
  rederivation (`Q056`).

> **Implementation & Provenance.** `crates/qcore-frontend/src/{source_std_string,source_std_buffer,source_record,source_resource_calls}.rs`,
> `crates/qcore-core/src/{std_string,std_buffer,source_record,resource_calls,ownership}.rs`;
> Lean: `QCore.StringSlice`, `QCore.StringSliceIo`, `QCore.StringConcat`,
> `QCore.WriteLend`, `QCore.LoopWindows`, `QCore.AggregateOwnedField`,
> `QCore.OwnedFieldCopyReach`, `QCore.ResourceCalls`; cards `STRING-SLICE`,
> `STRING-SLICE-IO`, `STRING-CONCAT`, `CT07-SLICE-PERMISSIONS`, `STD-BUFFER`,
> `SLICE-PERMISSIONS-WRITE-LEND`, `LOOPS-ON-COMPOSED-ROUTES`,
> `COMPOSED-WINDOW-REACH`, `AGGREGATES-V1`, `AGGREGATE-OWNED-FIELD`,
> `OWNED-FIELD-ROUTE-REACH`, `W48-QR-USABLE-RECORDS`, `RESOURCE-HELPER-FUNCTIONS`.

---

<a id="l14c-pipelines-decidable-branching-and-bounded-counted-loops"></a>

### L14.C. Pipelines, decidable branching, and bounded counted loops

<a id="l14c1-pipe-operator-and-operator-token-gate"></a>

#### L14.C.1. Pipe operator (`|>`) and operator token gate (`source.pipe.v1`)

The `Pipe` pass (`crates/qcore-frontend/src/pipe.rs`, diagnostics `Q036`–`Q038`,
[grammar `G02h`](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md)) runs **before every
route-specific semantic pass**, desugaring well-formed `|>` pipelines over the
parsed token stream (`PIPE-INTRINSIC-STAGE` ensures downstream passes read the
post-desugaring token stream rather than re-lexing raw source bytes) and
enforcing the per-file positional operator gate:

1. **Multi-stage pipeline desugaring and hygiene (`PIPE-OPERATOR`, `PIPE-CHAIN`, `Q036`):**
   - A pipeline of `n` stages desugars into `n` sequential statements: each
     non-final stage binds its result to a compiler-generated temporary
     `qcore__pipe0`, `qcore__pipe1`, ... passed as the leading/sole operand of
     the next stage, while the final stage preserves the outer statement wrapper
     (`let NAME = ...;` or statement sink).
   - **Refusal-enforced hygiene (`Q036`):** if a file requires pipeline
     temporaries and also writes any identifier beginning with the reserved
     `qcore__pipe` prefix, compilation refuses at `Q036` rather than silently
     renaming user bindings. Empty stages/operands, unbalanced call parentheses,
     malformed `let` heads, assignment pipelines (`PIPE-ASSIGN`), `move` on the
     initial value, or bare `move` on a non-binding value also refuse at `Q036`.
2. **Call stages, model intrinsics, statement sinks, and terminal `drop` (`Q037`):**
   - **Call stages** (admitted at any stage position, binding a value when
     non-final, `PIPE-SCALAR-RESULT`): ordinary source/extern calls `f(args...)`
     (inserting the piped value, optionally prefixed by `move`) and call-shaped
     model intrinsics (`qcore_buffer_new`, `qcore_buffer_read`,
     `qcore_buffer_write`, `qcore_error_payload`, `qcore_acquire`,
     `qcore_check`). Whether a specific call's return type may be bound is
     decided by the selected route (e.g., a `unit` extern call in non-final
     position refuses at `Q024`). Keyword-shaped concurrency/mutex intrinsics
     (`spawn`, `join`, `share`, `grow`, `open`, `close`) are not call stages and
     refuse (`Q036`).
   - **Terminal statement sinks (`Q037` if non-final):** `print [dec|hex]`,
     `log LEVEL [dec|hex]` (0 or 1 rendering selector, supporting both scalar and
     `std.string` text sinks), and **`drop`** (`b |> drop;` desugars to
     `drop b;`, card `PIPE-DROP-STAGE`). Because statement sinks produce no
     value (`flush` takes zero operands and always refuses in a pipe at `Q037`),
     placing `print`, `log`, or `drop` before a later stage refuses at `Q037`,
     and binding `let x = ... |> drop;` refuses at `Q036`.
3. **Per-file positional operator token gate (`Q038`):**
   - Outside parser-recorded `if (...)` condition extents — and outside the two
     `|...|` parameter/capture bars of a scoped-task spawn closure
     (`W48-QR-CLOSURES`; capture-list bars elsewhere refuse at `Q038` as
     `W48-QR-CLOSURE-VALUES`) — **all five** bitwise and logical operator tokens
     `|`, `&`, `^`, `||`, and `&&` are unconditionally refused at **`Q038`**
     (cards `OR-AND-LOWERING`, `BITWISE-CONDITIONS`; named follow-ons
     `BITWISE-LET-INITIALIZER` and `BITWISE-STD-IO-OPERAND`). Inside `if (...)`
     conditions, validity is decided by `source.branch.v1` at `Q045`.

<a id="l14c2-two-sorted-decidable-conditions-and-if-else-branching"></a>

#### L14.C.2. Two-sorted decidable conditions and `if / else` branching (`source.branch.v1`)

`source.branch.v1` (`Branch`, diagnostics `Q044`–`Q046`,
[grammar `G02i`](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md), cards `IF-STATEMENT`,
`OR-AND-LOWERING`, `BITWISE-CONDITIONS`, `IF-ARMS`) lowers sequential
`if (COND) { ... } else { ... }` statements onto `CfgTerminator::Branch` in
`cleanup_lowering`:

1. **Admitted arm statements and early-`return` continuation hoisting (`Q044`):**
   - Branch arms admit `qcore_check`, `drop`, `move`, nested `if / else`, empty
     blocks, **`return`**, and **neutral-flow zero-argument source calls**.
   - An arm ending in `return` discharges its empty-live-ledger obligation at
     its own exit span (`Q202` on leak) and does not reach the post-branch join.
     When one arm returns and its sibling falls through, statements written
     after the `if / else` are hoisted **once** into the surviving arm; shapes
     that cannot be lowered by single-move hoisting — a mixed branch with no
     continuation or a continuation lacking `return`, unreachable statements
     after a branch where both arms return, or an early return inside one arm of
     a nested branch whose outer sibling also falls through
     (`IF-ARM-RETURN-NESTED`) — refuse at `Q044`.
   - Conditional source calls require a **neutral** resource-flow summary (`Δ = 0`,
     zero requirements/peaks) so whole-program call-chain composition remains
     exact (`IF-ARM-CALL-FLOW` at `Q044` otherwise). Arm-local `let` bindings
     refuse at `Q044`; arm-local `qcore_acquire` refuses at `Q103`
     (`IF-ARM-ACQUIRE`).
2. **Two-sorted decidable condition tier and C11 precedence agreement (`Q045`):**
   - **Boolean sort:** `true`, `false`, prefix `!C`, `A && B`, `A || B`, and
     non-associative comparisons (`==`, `!=`, `<`, `<=`, `>`, `>=`) over integer
     operands.
   - **Integer sort:** signed `i32` literals, top-level sequential `let NAME = INTEGER;`
     constant bindings (resource names refuse at `Q045`; unbound names refuse at
     `Q013`), and parenthesized bitwise `&`, `^`, `|`.
   - **C11-agreement precedence rule:** operator precedence is defined **only
     where it agrees with C11** (`&` tighter than `^` tighter than `|`; `!`
     tighter than `&&` tighter than `||`, left-associative). Wherever two-sorted
     parsing would invert C11's ranking — specifically unparenthesized `!A < B`
     or `A | B == C` — **the unparenthesized spelling refuses at `Q045`**,
     requiring `!(A < B)` and `(A | B) == C`. Calls, `|>`, shifts, `~`,
     arithmetic, chained comparisons, and runtime-varying conditions
     (`IF-RUNTIME-CONDITION`) refuse at `Q045`.
   - `!`, `&&`, and `||` eliminate prior to core checking via arm-swapping
     (`if (!C) T else E` ≡ `if (C) E else T`) and equivalent nested `if / else`
     trees, enforcing the custody join at every branch level.
3. **Surviving-arm custody agreement (`Q046`):** whenever both arms fall through
   to the join, their **ordered live-identity lists** and **transferred sets**
   `MUST` be equal (`Q046` refuses conditional leaks, conditional double drops,
   and move-vs-drop mismatches; incompatible arms are **never** widened).

<a id="l14c3-bounded-counted-for-loops-and-unroll-budgets"></a>

#### L14.C.3. Bounded counted `for` loops and unroll budgets (`source.loop.v1`)

`source.loop.v1` (`Loop`, diagnostics `Q097`–`Q099`,
[grammar `G07.12`](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md), cards `LOOP-V1`,
`LOOP-UNROLL-BUDGET`, `LOOPS-ON-COMPOSED-ROUTES`, `W48-QR-SCALAR-TYPES-R5`)
provides checked counted loops on `source.buffer.v1` and composed
`source.string.v1` + `std.buffer` (`LOOP-ROUTE-WIDENING` pins refusals on other
routes):

```qcore
#import std.buffer;

fn main() -> i32 effects { alloc, mut } {
    let b = bytes(512);
    let n = extent(b);
    [| #unroll, where #bound < 513 |]
    for i in 0..n {
        b[i] = 1;
    }
    drop b;
    return 0;
}
```

1. **Header syntax, contextual `in`, and licence-attribute shape (`Q097`):**
   - `for` is reserved; `..` is read positionally from two `Dot` tokens; and
     `in` remains **contextual** (skipped by reference collection only in a
     `for` header, preserving pinned Fluent Bit `.qv` contracts with parameter
     `in`).
   - The lower bound `MUST` be written `0`. Early loop exits (`break`/`continue`
     `LOOP-BREAK`, `return` `LOOP-BODY-RETURN`), body bindings shadowing outer
     names, and malformed/misplaced `[| #unroll, where #bound < N |]` attributes
     (wrong clause word, missing `where`, non-`<` operator, computed `N`,
     non-`for` target, consecutive or trailing attributes; unterminated `[|` is
     `Q000`) all refuse at `Q097`.
2. **Decidable integer bound tier and three-layer unroll budget (`Q098`):**
   - `BOUND` `MUST` belong to the lowering-visible integer constant tier
     (integer literal, literal `let`, `extent(NAME)`, or check-time integer
     conversion/value statement over that tier; byte-read bounds refuse as
     `LOOP-BOUND-FROM-BYTE`, `bool` and float bounds refuse unconditionally,
     `isize`/`usize` require a `recorded target`, and `BOUND <= 0` refuses so
     zero-trip bodies can never enter a program unchecked).
   - **Three-layer budget (`Q098`):**
     1. **Default unattributed budget (`256`):** an unattributed `for` admits
        `1 <= BOUND <= 256`; exceeding `256` refuses with a diagnostic showing
        the `[| #unroll, where #bound < N |]` attribute.
     2. **Checked `#unroll` licence:** `[| #unroll, where #bound < N |]` is
        **verified, never trusted** — requiring `2 <= N <= 4097` (checked even
        before reading `BOUND`) and `BOUND < N`, and erasing completely from
        emitted C.
     3. **Hard ceilings (`4096` / `8192`):** no single loop `MAY` exceed
        **`4096`** iterations, and total unrolled events across nested loops in
        one entry `MUST NOT` exceed **`8192`**.
3. **Exact check-time unrolling, real C `for` emission, and custody identity (`Q099`):**
   - Check time unrolls all `BOUND` iterations exactly so byte values remain
     exact for stream/observation verification, whereas emitted C outputs a
     **real `for (T i = 0; i < BOUND; i += 1)` loop** (`QCore.ScalarLoopBounds`).
   - Every single iteration `MUST` be the **identity on custody** (`Q099`,
     enforced even when `BOUND == 1`), exiting with the exact same live
     resources, names, and footprints with which it entered.

> **Implementation & Provenance.** `crates/qcore-frontend/src/{pipe,source_loop}.rs`,
> `crates/qcore-core/src/{cleanup_lowering,source_loop}.rs`; Lean: `QCore.Pipe`,
> `QCore.Branch`, `QCore.SourceLoop`, `QCore.LoopWindows`,
> `QCore.ScalarLoopBounds`, `QCore.CountedLoopMeasure`; cards `PIPE-OPERATOR`,
> `PIPE-CHAIN`, `PIPE-SCALAR-RESULT`, `PIPE-INTRINSIC-STAGE`, `PIPE-DROP-STAGE`,
> `IF-STATEMENT`, `OR-AND-LOWERING`, `BITWISE-CONDITIONS`, `IF-ARMS`, `LOOP-V1`,
> `LOOP-UNROLL-BUDGET`.

---

<a id="l14d-standard-io-event-reactors-loopback-sockets-and-http11"></a>

### L14.D. Standard I/O, event reactors, loopback sockets, and HTTP/1.1

<a id="l14d1-formatted-output-and-logging"></a>

#### L14.D.1. Formatted output and logging (`std.io`, `std.fmt`, `std.logger`, `source.std-io.v1`)

Cards `STD-MODULES`, the Quiet C11 Emission amendment
([design/QUIET_EMISSION_STREAMS_AMENDMENT.md](design/QUIET_EMISSION_STREAMS_AMENDMENT.md)),
`STD-IO-COMPOSE`, `STD-FMT-BORROW`, `COMPOSE-CONCURRENCY`, and `OBS-REORDERING`
(`StandardIo` `Q028`–`Q031`, `ComposedStdIo` `Q043`, `ComposedTasks` `Q050`,
grammars `G02g`, `G02i`, `G02j`) partition stream output across three
independent per-file capability imports (`Q015` when missing):

- **`#import std.fmt;`** — grants scalar/float rendering selectors `dec` and
  `hex` (`§L14.0.2`, `§L14.7`) and opens zero streams;
- **`#import std.io;`** — grants `print RENDERING OPERAND;` on `stdout`; and
- **`#import std.logger;`** — grants `log LEVEL RENDERING OPERAND;` on `stderr`,
  the zero-operand `flush;` sink-handoff barrier (needing no `std.fmt`), and
  entry-level `log_level LEVEL;`.

1. **Logger levels, quiet emission, and `log_level LEVEL;` (`Q028`–`Q031`):**
   - The six ordered logger levels are `trace`, `debug`, `info`, `warn`, `error`
     (contextual immediately after `log`, remaining the error-declaration
     keyword elsewhere), and `fatal`, emitted on `stderr` with canonical tags
     `[TRACE]` .. `[FATAL]`. **Source-authored `log` lines are never filtered.**
   - **Quiet C11 emission by default:** compiler-generated internal trace events
     use a separate default-null system logger and produce **zero** output
     unless the entry explicitly writes `log_level LEVEL;` (at most once per
     entry, requiring `std.logger`, deriving `Io`, recorded and independently
     rechecked, and projecting to zero custody or reactor steps), which enables
     hosted `stdout` rendering of admitted system trace events at or above
     `LEVEL` across sequential, `std-io`, `event` (before, inside, or after
     `event open`..`close`), `std-io-extern`, and `std-io-tasks` profiles.
   - On standalone `source.std-io.v1`, non-scalar operands and `borrow NAME`
     refuse at `Q028`, undeclared `Io` refuses at `Q029`, entry-only/statement
     shape violations refuse at `Q030`, and certificate recheck mismatches
     refuse at `Q031`.
2. **Extern composition and `borrow NAME` live-identity rendering
   (`source.std-io-extern.v1`, `Q043`):**
   - Composes unedited `ExternCall` (`Q022`–`Q027`) and `StandardIo`
     (`Q028`–`Q031`) projections in one flat entry under four `Q043` join rules:
     (1) exact order-preserving interleaving ending in one `return`; (2) scalar
     operands resolve in the extern value namespace and `MUST` be
     source-determined (literals, `succeeds` outcomes, or folded value
     statements — undetermined C returns refuse at `Q043`); (3) derived effect
     union admitted by `effects { ... }`; and (4) **`print|log ... RENDERING borrow NAME;`**
     resolves `NAME` in the extern **custody** namespace and succeeds iff the
     extern certificate's `live_after` holds that exact resource identity live
     at that step (refusing unestablished, null-arm, consumed, released, stored,
     or reallocated identities). A borrowed render emits **only the deterministic
     model identity integer** — never pointer addresses, pointee bytes, or
     extents — leaving custody unchanged and introducing no scalar `ValueId`.
3. **Scoped-task composition and window-wise stream reordering
   (`source.std-io-tasks.v1`, `Q050`):**
   - Selected on `has_source_tasks ∧ has_source_std_io ∧ ¬has_source_extern`,
     joining `check_task_module` (`Q600`) with `source.std-io.v1` line parsing
     (`Q030`) under five `Q050` rules: (1) lines may stand in `main` (always
     join-ordered relative to parent-replayed worker traces, keeping exact
     byte-for-byte stream comparison when no worker writes) or inside spawned
     leaf workers — which declares **`admits_reordering`** under
     `qcore.obs-reordering.v1` ([design/OBS_REORDERING_V1.md](design/OBS_REORDERING_V1.md)),
     comparing `stdout`/`stderr` window-by-window across quiescent
     zero-live-worker barriers and emitter-thread-by-emitter-thread; (1a) every
     writing worker `MUST` declare `io` in its own `effects` row alongside
     `main`; (2) intra-block statement order is preserved; (3) scalar/float
     operands are validated for all-path readability by inserting `BindScalar`
     at the rendering site and re-running `check_task_module` (`borrow NAME`
     rendering is refused on this route); and (4) `Io` admission and composed
     certificate rederivation are enforced at `Q050`.

<a id="l14d2-deterministic-event-reactor"></a>

#### L14.D.2. Deterministic event reactor (`std.event`, `source.event.v1`)

`#import std.event;` (`Q015` when missing, design
[STD_EVENT_V1](design/STD_EVENT_V1.md)) gates `source.event.v1` (`Event`,
diagnostics `Q058`–`Q062` and `Q074`, cards `STD-EVENT` and
`COMPOSE-EVENT-LINES`), projecting single-threaded `.qr` reactor programs onto
the unmodified `qcore_core::event_reactor::check_event_program` checker:

1. **Lifecycle, custody, and `write` check-only boundary (`Q058`–`Q060`, `Q062`):**
   - An entry opens at most one reactor context (`event open;` before all other
     reactor statements and `event close;` after all of them, `Q058`), creates
     loopback descriptor pairs (`endpoints a b;`), registers/deregisters
     interest (`watch reg a read|write;` / `unwatch reg;`), signals readiness
     (`wake b [BYTE];`), dispatches (`dispatch ready reg;` / `dispatch idle;`),
     and closes endpoints (`close a;`, `close b;`).
   - Unknown or non-live endpoint/registration names refuse at `Q059`; core
     reactor model violations (`L00`–`L12`) report verbatim at **`Q060`**; and
     opening a context derives `{Io}` (`Q062`). Registering `write` interest is
     **accepted as check-only** (no C or reference observation emitted), because
     kernel send-buffer writability is not induced by `wake`.
2. **Single-ready determinism guard (`Q061`):**
   - `dispatch ready reg;` and `dispatch idle;` `MUST` match the exact readiness
     induced by prior `wake` statements, and emitted C verifies `poll(2)` /
     `WSAPoll` results at runtime (exiting non-zero if platform premise
     `EVENT-V1-LEVEL-TRIGGERED-READINESS` is violated).
   - Any dispatch turn at which **two or more** registrations are simultaneously
     ready refuses by name at **`Q061`**, because equal-priority (`0`) callback
     activation order in `poll(2)` is implementation-defined
     (`explore_activation_orders`).
3. **Drained-byte binding and line composition (`COMPOSE-EVENT-LINES`, `Q074`):**
   - Entries may interleave `print`, `log`, and `flush` with reactor statements
     and bind `let x = dispatch ready reg;` to capture the `i32` byte delivered
     by `wake ep BYTE;` (verified at runtime under named premise
     `EVENT-V1-STREAM-BYTE-ORDER`; `QCore.ComposeEventLines.erasing_the_lines_leaves_the_reactor_projection`).
     Rendering an endpoint/registration name or `borrow NAME` (`STD-FMT-DESCRIPTOR`)
     refuses at `Q074`.

<a id="l14d3-loopback-tcp-udp-sockets-and-protocol-cell-disjointness"></a>

#### L14.D.3. Loopback TCP/UDP sockets and protocol-cell disjointness (`std.socket`, `source.socket.v1`)

`#import std.socket;` (`Q015` when missing) gates `source.socket.v1`
(`StandardSocket`, diagnostics `Q080`–`Q089` plus `Q664`, cards
`QUIRIN-SOCKET-V1`, `SOCKET-EVENT-COMPOSE`, `SOCKET-INET-ADDRESSES`, and
`CT42-CELL-ROUTE`), providing deterministic single-threaded loopback TCP and UDP
transport:

1. **Transport state machine, ownership, and loopback determinism (`Q080`–`Q089`):**
   - Ten reserved clause words (`tcp`, `udp`, `bind`, `listen`, `connect`,
     `accept`, `send`, `recv`, `sendto`, `recvfrom`) pair with `let`, `close`,
     `return`, and scalar `std.io` lines (`Q080` flat entry shape; `Q081`
     check-time constant operand / socket lookup).
   - **`Q082` (protocol state machine):** refuses `listen`/`connect` on UDP,
     `listen` before `bind`, `connect` to non-listening sockets, duplicate
     `bind`, `send`/`recv` on unconnected TCP or UDP, `sendto`/`recvfrom` on
     TCP or unbound UDP, closed-socket operations, and IPv4/IPv6 family
     mismatches.
   - **`Q083` (custody) & `Q084` (ranges):** `qcore_core::ownership` enforces
     `tcp`/`udp`/`accept` acquisition and `close` release (`Q083`); payload
     bytes `0..=255`, transfer counts `1..=1024`, exact whole-datagram length on
     `recvfrom` (never truncated), and port numbers `0..=65535` check at `Q084`.
     Socket crossings refuse at `Q086`; `{Alloc, Io}` (never `Mut`) checks at
     `Q087`; certificate rederivation and backend checks use `Q088`–`Q089`.
   - **`Q085` (determinism boundary):** every peer is a socket of the same
     program on loopback. Unsatisfied `recv`/`recvfrom`, `accept` on an empty
     queue, `> 1` unaccepted connection on the depth-1 backlog, `> 4096` unread
     stream bytes, datagram queue overflow, **TCP `close` with unread bytes**
     (which emits `RST`), and listener `close` with unaccepted connections all
     refuse at check time (`Q085`; UDP `close` with queued datagrams is admitted
     because local drop emits zero packets). Emitted C verifies received bytes
     (`SOCKET-V1-STREAM-BYTE-ORDER`) and bounds every wait with a finite timeout
     (`SOCKET-V1-LOOPBACK-BUFFERED-SEND`).
2. **Declared IPv4/IPv6 loopback addresses (`SOCKET-INET-ADDRESSES`):**
   - `bind s 127.0.0.1:8080;` and `bind s [::1]:8080;` (plus `connect`/`sendto`
     targeting lexically adjacent `ADDR:PORT`) parse addresses once at check
     time without calling `inet_pton` at runtime. Targets not bound by the same
     program (`SOCKET-EXTERNAL-PEER`) or targeting ephemeral ports refuse at
     `Q085`; hostnames (`SOCKET-DNS`), unbracketed/embedded-v4/zone IPv6, and
     leading-zero IPv4 octets refuse at `Q080`. Every IPv6 socket explicitly
     sets **`IPV6_V6ONLY = 1`** across Linux, macOS, and Windows with zero IPv4
     fallback.
3. **Reactor loan composition (`SOCKET-EVENT-COMPOSE`) and protocol-cell
   disjointness (`CT42-CELL-ROUTE`, `Q664`):**
   - Entries importing both `std.socket` and `std.event` model
     `watch E S read|write;` .. `unwatch E;` as `OwnershipOp::BorrowShared` of
     socket `S`: `close` while watched (`DropWhileBorrowed`), `bind`/`listen`/`connect`
     while watched (`OwnerMutationWhileBorrowed`), and reactor `L00`–`L12`
     errors report at `Q083`, while `send`/`recv`/`accept` on watched sockets
     are permitted and level-triggered `dispatch` readiness checks at `Q085`.
   - The query **`disjoint A B;`** (`disjoint` reserved) projects derived
     connection cells (`qcore_core::socket_footprint`) onto `DisjointFootprintsV1`
     with zero `DeclaredProtocolCellIdentity` premise
     (`QCore.SocketCellRoute.a_derived_projection_stamps_no_declared_identity`).
     The two ends of one accepted TCP connection share one kernel protocol cell
     and **refuse at `Q664` (`SharedProtocolCell`)**, whereas a listener vs.
     accepted end, separate TCP connections, and UDP endpoints
     (`CT42-CELL-UDP-QUEUE`) are admitted; `disjoint A B;` derives zero effects
     and erases completely from emitted C.

<a id="l14d4-checked-http11-message-subset"></a>

#### L14.D.4. Checked HTTP/1.1 message subset (`std.http`, `source.http.v1`)

`#import std.http;` (`Q015` when missing) gates `source.http.v1`
(`StandardHttp`, diagnostics `Q090`–`Q096` plus `Q664`, cards `HTTP-QUIRIN-V1`,
`HTTP-V1-COMPUTED-LENGTH`, and `CT42-CELL-ROUTE`), composing checked RFC
9110/9112 message construction (`request`, `response`, `header`, `content` —
using RFC 9110 §6.4's `content` rather than fixture-colliding `body`) over
`source.socket.v1` loopback TCP and static `std.string` composition:

1. **Start-line, header, and CRLF-injection guarantees (`Q090`–`Q092`):**
   - `Q090` validates flat entry shape, string literal escapes, static
     `concat`/`len` (`<= 4096` bytes), post-transmission immutability, single
     `content` assignment, and certificate rederivation.
   - `Q091` validates RFC 9112 start lines: exact `HTTP/1.1` version,
     case-sensitive `token` methods (`HTTP-V1-METHOD-CASE` refuses lowercase
     registered spellings such as `"get"`), origin-form request targets, status
     codes in `100..=599`, and valid reason-phrase bytes.
   - `Q092` validates RFC 9110/9112 headers: `token` field names, printable
     ASCII field values excluding control bytes, `obs-text` (`HTTP-V1-OBS-TEXT`),
     and leading/trailing whitespace, **exactly one `Host` header** per request,
     and no unsupported `Connection` options (`HTTP-V1-PERSISTENT`). **CRLF
     header injection is structurally unrepresentable:** user components forbid
     `CR` and `LF`, and every `CRLF` delimiter in an emitted message head is
     synthesized exclusively by the compiler.
2. **Decided `Content-Length` framing (`Q093`) and transport composition (`Q094`–`Q096`, `Q664`):**
   - Every message carrying `content` `MUST` declare a single `1*DIGIT`
     `Content-Length` header **statically equal** to the content's exact byte
     length (`Q093`) — whether written literally or computed via `concat(...)`,
     `len(...)`, and `dec(NAME)` (`HTTP-V1-COMPUTED-LENGTH`; `Transfer-Encoding`
     refuses as `HTTP-V1-CHUNKED`).
   - Message/text/scalar kind mismatches refuse at `Q094`; unedited
     `source.socket.v1` custody, transport, determinism, and cell-derivation
     refusals report at `Q095`; `{Alloc, Io}` transport/output effects check at
     `Q096` (HTTP message assembly derives zero effects); and `disjoint A B;`
     refuses shared accepted-TCP cells at **`Q664`** while erasing from C.
   - **Named boundaries:** `source.http.v1` is a bounded executed HTTP/1.1
     subset, not full protocol conformance — runtime string assembly
     (`STRING-SLICE-CONCAT-RUNTIME`), wire HTTP parsing (`HTTP-V1-PARSE`),
     chunked transfer coding (`HTTP-V1-CHUNKED`), persistent connections
     (`HTTP-V1-PERSISTENT`), `100 Continue` (`HTTP-V1-EXPECT-CONTINUE`),
     computed header names (`HTTP-V1-COMPUTED-FIELD-NAME`), `std.buffer` bodies
     (`HTTP-V1-BUFFER-CONTENT`), reactor composition (`HTTP-V1-EVENT-COMPOSE`),
     external hosts (`SOCKET-EXTERNAL-PEER`), and TLS (`HTTP-V1-TLS`) remain
     explicit named follow-ons.

> **Implementation & Provenance.** `crates/qcore-frontend/src/{source_std_io,source_composed,source_composed_tasks,source_event,source_socket,source_http}.rs`,
> `crates/qcore-core/src/{std_io,obs_reordering,event_reactor,std_socket,socket_footprint,std_http}.rs`;
> Lean: `QCore.QuietEmission`, `QCore.ObsReordering`, `QCore.ComposeEventLines`,
> `QCore.SocketFootprint`, `QCore.SocketCellRoute`, `QCore.HttpV1`; cards
> `STD-MODULES`, `STD-IO-COMPOSE`, `STD-FMT-BORROW`, `COMPOSE-CONCURRENCY`,
> `OBS-REORDERING`, `STD-EVENT`, `COMPOSE-EVENT-LINES`, `QUIRIN-SOCKET-V1`,
> `SOCKET-EVENT-COMPOSE`, `SOCKET-INET-ADDRESSES`, `CT42-CELL-ROUTE`,
> `HTTP-QUIRIN-V1`, `HTTP-V1-COMPUTED-LENGTH`.

---

### L14.E. External C calls, native library manifests (.qrlib), and C struct FFI

Quirin separates what a program *checks* across its own boundary from what it *assumes* about linked native code. Calling into C is never treated as a proof about the C implementation: every boundary records explicit, auditable premises on the published certificate, and every mechanism states both **what crosses** (ABI types and custody modes) and **which artifact or contract vouches for it**.

#### The Linkage and FFI Trust Spectrum

Rather than offering a single binary toggle between "verified" and "unsafe", Quirin organizes native C interop along an explicit **Linkage and FFI Trust Spectrum**. Each step either widens the admitted value/custody surface only where a contract vocabulary states how capabilities behave, or sharpens *which* header, source binding, and binary symbol table the certificate names:

| Mechanism / Level | Source & Build Opt-In | Admitted Boundary Surface | Derived Effects | Certificate Premise & Identity | Primary Diagnostics |
|---|---|---|---|---|---|
| **Bounded Contracted Extern Calls** (`source.extern-call.v1`, §L14.E.1) | `#import std.extern;` + explicit `CompileOptions` resource-v1 selection | §C04 scalars by value; extentless `ptr<const\|mut, u8>` under `borrowed`, `consumed`, `owned` (store-owner loan), `return_none`, `return_owned`, `out`, `stored`, `return_reallocated` | `{Io}`, plus `Alloc` (fresh custody) and `Mut` (`mut` pointer) | Declared contract (`ContractTrust::Declared`), extentless pointer premise, builder linkage premise | `Q022`–`Q027` |
| **Composed Extern + Standard I/O** (`source.std-io-extern.v1`, §L21 `Q043`) | `#import std.extern;` + `#import std.io;` / `std.logger` / `std.fmt` | Interleaves extern calls with deterministic line rendering of literal scalars, `succeeds`-pinned scalar results, and live `borrow NAME` custody IDs | Union of extern and I/O derived rows | Sub-certificates of both projections plus composed interleaving recheck | `Q043` (+ `Q022`–`Q031`) |
| **Unchecked Scalar Linkage** (`unsafe extern fn`, §L14.1, level L0) | `#import std.extern.unsafe;` + `unsafe extern fn NAME(…) -> ABI;` | §C04.1 scalars by value in, scalar or `unit` out; **every pointer refused** (`ptr<const, u8>` included) | Whole modeled row `{Io, Alloc, Mut}` unconditionally | Per-callee unchecked-linkage waiver by symbol name and ABI signature (no build named) | `Q032`–`Q035` |
| **Manifest-Resident Waivers** (`.qrlib` v1 `unchecked fn`, §L14.2, level L1) | `#import std.extern.unsafe;` + selected `source.native-library.v1` manifest | Unchanged §L14.1 scalar-only table (manifest never widens the waived surface) | `{Io, Alloc, Mut}` | Replaces bare waiver with sharpened artifact pin (library, version, file, `sha256` pin) + 4 ABI premises | `Q039`–`Q042` |
| **Manifest Contract Slices** (`.qrlib` v1 `contract fn … from MODULE`, §L14.3, level L3) | `#import std.extern;` + contract selection + `.qrlib` manifest | Unchanged §C04 pointer and scalar bridging table (where pointer symbols belong) | Derived from contract per §L14.E.1 | **Stacks** declared-contract premise with artifact + module premise (neither replaces the other) | `Q047`–`Q049` |
| **Verified Exported Headers** (`.qrlib` header + include texts, §L14.4, §L14.4.1) | Optional `header` + `sha256` pair (and optional declared `include` texts) in `.qrlib` | Unchanged entry surface; checks every bound symbol against real `.h` token stream and derived erasable decorations | Unchanged | Adds **verified** `sha256` digests of header and declared include texts beside stated artifact pin | `Q063`–`Q064` |
| **Source-File Library Binding** (`extern library NAME;`, §L14.5) | File-local `extern library NAME;` declaration | Unchanged; restricts symbol resolution to manifests explicitly named by the declaring file | Unchanged | **Stacks** per-file source-binding premise naming spelling, manifest, artifact pin, and ABI fields | `Q065`–`Q067` |
| **Imported C Structs & Usable Records** (`source.ffi.v1`, §L14.6) | `#import std.ffi.unsafe;` + `struct! { type T = c"struct TAG" from "HEADER"; … }` | Scalars and Clang-layout-verified imported C records by value or non-retained `const` pointer through `[| quant #c_ffi |]` contracts | `{Io}`, plus `Mut` on record field writes and helper effects | **File-wide unsafe-FFI assumption** (never plain `Safe`), recorded compiler, header digests, `#c_ffi` callees | `Q032`–`Q035`, `Q051`–`Q053`, `Q063`–`Q064` |
| **Symbol-Generated `.qrlib` v2 Linkage** (`W48-QRLIB-LINKAGE`, §L14.8) | `extern library NAME;` bound to build-generated `.qrlib` v2 manifest | FFI C functions verified exported and defined in ELF, Mach-O, or COFF/PE symbol tables (or `.tbd` / import stubs) | As governed by §L14.6 | Per-callee `linked:NAME` premise: symbol, re-hashed static artifact(s) or dynamic stub + named runtime premise | `Q039`, `Q041`, `Q047`, `Q063`–`Q067` |

#### The Unified Contradiction and Withdrawal Rule

Across all manifest, header, contract, and source-binding levels, disagreements between two visible descriptions of a symbol obey a single monotonicity law:

> **Withdraw only where the weaker admissible reading refuses.**

A contradiction between two inputs MUST never be resolved by falling back to a weaker premise that *admits* the program while stating less about the build or contract—doing so would reward a contradiction with a weaker obligation on the certificate. Specifically:

* **Outright refusal when the weaker reading would still admit (`Q041`, `Q064`, `Q066`):**
  * At §L14.2 (`Q041`), if an `unsafe extern fn` signature disagrees with its `.qrlib` manifest's `unchecked fn` entry, withdrawing the manifest entry would fall back to the bare unmanifested §L14.1 waiver—which is accepted and omits the artifact identity. The symbol is therefore **refused outright**.
  * At §L14.4 (`Q064`), if a manifest's transcribed declaration disagrees with the exported `.h` text (or, at §L14.6, if a `struct!` field listing disagrees with the recorded Clang layout), withdrawing the header check would fall back to a manifest without a verified header—which is accepted and says less about the build. Both inputs are load-time artifacts, so the manifest or struct import is **refused outright**.
  * At §L14.5 (`Q066`), if a file declares `extern library A;` and also declares a symbol bound by another selected manifest `B` that the file did *not* bind (or, at §L14.8, calls an FFI symbol bound by no bound manifest), falling through to an unmanifested reading would let a file strip build attribution from its certificate by naming a different library. The module is **refused outright**.
* **Per-unit contract withdrawal when the weaker reading itself refuses (`Q048`):**
  * At §L14.3 (`Q048`), if a declared contract's §C04 ABI projection disagrees with the manifest's `contract fn` header declaration, the weaker reading below a contracted callee is an *uncontracted* callee—which `source.extern-call.v1` already refuses at every call site (`Q023`). Following `LOADER-ABI` (standing constraint 8), the disagreed contract is **withdrawn for the analyzed unit**, `Q048` is reported rather than performed silently, the symbol is omitted from the admitted contract environment, and every call site refuses at `Q023`. Withdrawal here is strictly monotone in the refusing direction: it can only turn an admission into a refusal.

---

#### L14.E.1. Bounded contracted extern calls (`source.extern-call.v1`)

The `source.extern-call.v1` profile makes a **call to a declared extern function** executable at one bounded sequential boundary.

* **Opt-in and explicit contract selection.** A source file MUST opt in locally with `#import std.extern;` (`Q022` if omitted, including when calling an `extern fn` declaration supplied by another file in the bundle; `Q014` for an unknown module name). Every called extern function's contract MUST come from an explicit `CompileOptions` selection loaded by the total-or-reject resource-contract v1 loader (`Q023` if no selection is supplied, the selection fails to load, or the callee name is absent). There is no default contract environment, no implicit filesystem search, and no fallback. An inline source `contract { … }` block on a called extern declaration—or a source `extern fn` signature that disagrees with its selected contract—is refused (`Q027`) rather than enforced or overridden.
* **Admitted §C04 parameter and return modes.** Out of the companion contract specification's §C04 vocabulary, this route bridges a closed table (`Q024` outside it):
  * **Scalars and `unit`:** §C04.1 scalars passed by value (widened across the §G02t fixed-width integer/boolean table by `W48-QR-SCALAR-TYPES-R3` and IEEE `f32`/`f64` by `W48-QR-FLOATING-POINT`), and `resource_return_none()` with a `unit` or custody-free scalar result.
  * **Borrowed and consumed inputs:** `resource_borrowed` on an admitted pointer (`ptr<const, u8>` or `ptr<mut, u8>`), and `resource_consumed(p, nonnull)` on a nonnull owned resource binding.
  * **Permanent store into an owner (`resource_stored` and `resource_owned`):** `resource_stored(p, owner, nonnull)` on `ptr<mut, u8>` removes the passed identity permanently from the caller's live ledger, with no owned return permitted in the same call; that the declared `owner` retains and eventually releases the stored resource is recorded as a named certificate premise, discharged nowhere in the caller. Because contract specification §C04.2 requires a stored parameter's `owner` to be a **distinct `Owned` parameter**, `resource_owned` is admitted in *exactly* that owner position—nonnull, on `ptr<mut, u8>`, acting as a loan the caller keeps—and refused everywhere else (including when the stored argument names the same identity as its declared owner).
  * **Three nullable acquisition/reallocation sources:**
    1. `resource_return_owned(nullable)` returning a fresh owned pointer on success;
    2. `resource_out(p, nullable)` on `ptr<mut, ptr<mut, u8>>`, materializing a fresh owned identity in the out slot only on the success arm; and
    3. `resource_return_reallocated(p, nullable)` (card `EXTERN-MODES`), which reallocates an existing live resource `p`: on success the old identity is retired and replaced by the fresh returned identity, whereas on failure §C04.3 specifies that the original subject `p` remains unconsumed and live on the error arm.
  * **Excluded §C04 combinations.** A single declaration may establish at most one fresh identity: two out slots, an out slot beside an owned or reallocated return, and a `resource_stored` parameter beside any acquisition source are each refused by name (`Q024`). Also refused are `resource_released`, `resource_removed`, a nonnull owned return, a nonnull out slot, a nonnull reallocated return, a nullable stored parameter, and a nullable consumed parameter. An acquiring call that simultaneously moves custody out is refused because a contract constrains custody transfer only on its normal exit—with the sole exception of the `resource_return_reallocated` subject noted above, whose failure-arm preservation §C04.3 explicitly defines.
* **Mandatory acquisition adapter and error-arm cleanup (`EXTERN-MODES`, `EXTERN-ERR-REL`).** Every call that establishes fresh custody through any of the three nullable acquisition sources (`resource_return_owned`, `resource_out`, or `resource_return_reallocated`) MUST be written with the mandatory acquisition adapter (`Q025` otherwise), branching the null/failure arm at the call site into a typed-error exit and carrying the literal fixture outcome control `succeeds BOOL`. Conversely, applying the adapter to a non-acquiring callee or naming an undeclared error refuses at `Q025`. On the adapter's error arm, every resource identity live at the raise—including the preserved input subject of a failed `resource_return_reallocated` call—MUST be discharged before exiting (`Q025` on abandonment). Under card `EXTERN-ERR-REL`, the error arm MAY carry explicit cleanup disposition calls, subject to strict rules enforced at `Q025`:
  * every live identity at the raise MUST be released by a disposition;
  * no disposition call may itself acquire or store custody;
  * each disposition MUST discharge **exactly one** identity that is currently live on the error arm;
  * dispositions MUST execute in **strict reverse acquisition order** (LIFO); and
  * every disposition's derived effect row MUST be admitted by the caller's declared `effects { … }` row.
* **Custody-free scalar bindings, literal bindings, and `succeeds` outcome controls (`PIPE-SCALAR-RESULT`, `EXTERN-SCALAR-LITERAL`, `STD-IO-COMPOSE`).** Values and resource identities inhabit disjoint namespaces:
  * A call whose declared result is a **custody-free scalar** (`unit` and every pointer result—even one declared `resource_return_none()`—are excluded) and which establishes no fresh custody MAY bind its result to a local variable: `let NAME = CALLEE(ARGS) [succeeds LITERAL];`.
  * When the optional `succeeds LITERAL` tail is present (`STD-IO-COMPOSE`, extended to typed integer/float literals with bitwise float equality where `-0.0` does not match `0.0`), the emitted C compares the runtime return value against `LITERAL` and exits with status `70` on disagreement, pinning a source-determined value that composed standard-I/O routes (`Q043`) may deterministically render. The `succeeds BOOL` adapter tail and `succeeds LITERAL` scalar-result tail introduce no reserved word and are distinguished by literal/callee kind alone.
  * Plain scalar literal bindings (`let NAME = LITERAL;`, card `EXTERN-SCALAR-LITERAL`) and single-operation checked scalar value statements (`W48-QR-SCALAR-TYPES-R3` / `W48-QR-FLOATING-POINT`) are likewise admitted in the value namespace; a literal binding calls nothing, derives no effect, and opens no custody obligation.
  * A bound scalar is admitted at a declared scalar parameter and nowhere else; a resource binding is refused at a scalar parameter (`Q024`); an unread scalar binding removes nothing from and adds nothing to the custody ledger (so it is never a leak); and an acquiring callee's result can never use the plain scalar-binding form (`Q025`).
* **Extentless pointer premise, derived effects, and flat-entry scope.** Length and byte extent are **not** transported across §C04 pointers: every C04 pointer is an extentless `u8` projection, so no declaration on this path states how many bytes a callee reads or writes, and every certificate records that premise alongside each callee's declared contract and linkage premise. The derived effect row of an extern call is `{Io}`, plus `Alloc` iff the call establishes fresh custody through one of the acquisition sources, plus `Mut` iff at least one parameter is a `mut` pointer (and `partial_eq` / `partial_ord` / `throws<ArithmeticError>` where scalar float comparisons or runtime-faulting conversions appear); the caller's declared `effects { … }` row MUST admit every derived effect. Extern calls are admitted in a **flat sequential parameterless entry body only** (`Q026` in nested blocks, non-entry functions, parameterized entries, task workers, `with lock` bodies, or error regions; `source.exec12` retains its own non-source-callee refusal).
* **Relative soundness.** Every verdict reached through `source.extern-call.v1` holds only *relative to* the selected declarations (`ContractTrust::Declared`); it is never a proof about the C library's body. See [implemented grammar G02d](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02d-bounded-extern-calls), the [extern-call v1 report](reports/archive/language-and-tooling/extern-call-v1/README.md), and the [out/stored extension report](reports/archive/language-and-tooling/extern-call-f1/README.md).

---

<a id="l141-unchecked-linkage"></a>
#### L14.1. Unchecked scalar linkage (`unsafe extern fn`)

An external library that has **no contract at all** MAY be linked under an explicit, per-declaration recorded waiver and never otherwise (card `UNSAFE-LINKAGE`, level L0 of `docs/design/UNSAFE_LINKAGE_V1.md`). Without the waiver, an uncontracted callee retains its `Q023` refusal.

* **Dual file-local opt-in on the declaration.** Each waived symbol requires two independent file-local acts:
  1. `#import std.extern.unsafe;` in the declaring file (`Q032` otherwise); and
  2. the explicit declaration form `unsafe extern fn NAME(…) -> ABI;`.
  Neither `#import std.extern;` nor `#import std.extern.unsafe;` implies the other, so a file that both declares and calls an uncontracted symbol writes both imports. The gate sits on the *declaration* rather than on call sites because declaring an uncontracted symbol is itself the trust act.
* **The contract is waived; the ABI is not (`Q033`).** Every parameter and result spelling MUST belong to the §C04.1 scalar ABI vocabulary (including `f32` and `f64` under `W48-QR-FLOATING-POINT`): scalars by value in, and a scalar or `unit` out. **Every pointer, in every spelling and every position, is refused (`Q033`)**, `ptr<const, u8>` deliberately included, as is any `unit` parameter or unknown type spelling. Handing a pointer to uncontracted code grants an unbounded capability—nothing bounds the extent read or written, `const` constrains only the C declaration rather than the implementation, nothing terminates the loan at return, and nothing prevents the callee from retaining or publishing the address—so no honest premise text could be stated for it. Restricting unchecked linkage to custody-free scalars ensures an uncontracted call cannot alter the caller's custody ledger regardless of what the callee executes.
* **No coexistence with checkable contracts (`Q034`).** A waiver MUST NOT stand beside a checkable contract: both an inline `contract { … }` block on an `unsafe extern fn` declaration and a waived symbol name that the selected contract environment also declares are refused (`Q034`), regardless of whether their signatures agree.
* **Unconditional full effect row.** Because nothing is modeled about an uncontracted callee's internal behavior, its derived effect row is unconditionally the **entire** modeled sequential row `{Io, Alloc, Mut}`. The caller's declared `effects { … }` row MUST admit all three effects, making the waiver visible in the entry function's signature as well as at the declaration.
* **Mandatory certificate premise and publication tripwire (`Q035`).** Every certificate for a program declaring a called uncontracted symbol MUST carry that waiver—by symbol name and declared ABI signature—on the same declared-assumption channel as trusted contracts. The premise is derived directly from the program's callee inventory and checked again by independent certificate rederivation; any certificate missing a required waiver premise is refused at publication (`Q035`). A resulting verdict is `Safe` **only relative to that waiver** and MUST be rendered as such; it makes no claim about the C callee. See [implemented grammar G02d.1](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02d1-unchecked-linkage), `docs/design/UNSAFE_LINKAGE_V1.md`, and the [unchecked-linkage evidence](reports/archive/early-waves/wave18-unsafe-linkage/README.md).

---

<a id="l142-manifest-resident-unchecked-linkage"></a>
#### L14.2. Manifest-resident unchecked linkage

A §L14.1 waiver MAY be made **resident in a native library manifest** (`.qrlib`), sharpening its certificate premise from a bare symbol name to a named library build artifact (card `UNSAFE-LINKAGE-ARTIFACT`; level L1 of `docs/design/UNSAFE_LINKAGE_V1.md` §1.1).

* **Manifest structure and total-or-reject loading (`Q039`, `Q040`).** A `source.native-library.v1` manifest binds:
  * one **artifact identity**: `library` name, `version`, `artifact` file name, and `sha256` content digest;
  * four **ABI premise fields**: data model, unwinding posture, allocator custody, and thread posture; and
  * a non-empty list of exported symbol entries, each explicitly tagged by keyword either as **contract-less** (`unchecked fn`, governed here) or as a **contract slice** (`contract fn … from MODULE`, governed by §L14.3).
  Entry kinds are distinguished strictly by keyword and never by signature shape, ensuring a dropped contract can never be misread as an unchecked waiver. Loading is **total or reject** (`Q039`): a missing format marker, an unknown/repeated/missing field, an unknown field value, a malformed hash or statement, an entry with mismatched keyword/tail syntax, a symbol bound twice across either entry kind, a type outside §C04.1, a `unit` parameter, a contract-less entry declaring a pointer, an empty entry list, or (under `EXT-01`) a `.qr`/`.qv`/legacy source suffix passed as a manifest or conflicting legacy/current manifest aliases rejects the entire manifest. Selecting two manifests that bind the same symbol is refused (`Q040`), so symbol resolution across a selection is a partial function.
* **Optionality and sharpened waiver rules (`Q041`, `Q042`).** Selecting manifests is optional: a program selecting none is byte-identical to §L14.1, and the absence of an artifact field in its bare waiver explicitly records that **no build was named**. When a selected manifest names a waived symbol:
  * **Signature agreement (`Q041`):** The source `unsafe extern fn` signature and the manifest's `unchecked fn` declaration MUST agree in arity, parameter types, and return type, or the symbol is **refused outright** (`Q041`)—never withdrawn to the bare unmanifested waiver (per the unified contradiction rule above).
  * **Unchanged scalar-only surface:** The closed §L14.1 scalar table still decides admission first; naming an artifact states *which binary code* is linked, not what that code would do with a pointer capability.
  * **Sharpened premise and downgrade guard (`Q035`):** The certificate replaces the bare §L14.1 premise with the sharpened artifact-resident premise, and the independent publication recheck refuses any certificate that omits it or downgrades it back to a bare waiver.
  * **No contract/waiver collision (`Q042`):** A symbol declared `unchecked fn` in a selected manifest MUST NOT also be reached through a declared contract in the contract environment (`Q042`, the manifest-side mirror of `Q034`).
* **Pin, not probe (at v1).** In `source.native-library.v1`, the artifact identity and `sha256` digest act as a **stated pin, not a check-time file probe**: the checker validates the manifest's syntax and consistency without reading or hashing the binary library on disk, and makes no claim about what the external linker ultimately consumed (contrast `.qrlib` v2 in §L14.8, which *does* read symbol tables and re-hash pinned files during `quirin build`). Recording the pin makes link-time build substitution an explicit violation of a named certificate assumption (§L20). See `docs/design/NATIVE_LIBRARY_LINKAGE_V1.md` and the [manifest evidence](reports/archive/early-waves/wave19-unsafe-linkage-artifact/README.md).

---

<a id="l143-manifest-contract-slices"></a>
#### L14.3. Manifest contract slices

A `.qrlib` manifest entry MAY instead bind an exported symbol to a **declared resource contract** via `contract fn NAME(…) -> ABI from MODULE;` (card `NATIVE-LIBRARY-CONTRACT-SLICE`; level L3 of `docs/design/UNSAFE_LINKAGE_V1.md` §1.1 and `docs/design/NATIVE_LIBRARY_LINKAGE_V1.md` §4.1–§4.2). Because §L14.1 waivers forbid all pointers, manifest contract slices are the mechanism through which a native library's **pointer-bearing functions** receive build-attributed linkage alongside §C04 custody modes.

* **Selection and attribution without new contract semantics.** A manifest creates no contract clauses of its own: each `contract fn` entry names a symbol, transcribes the artifact's exported C declaration, and gives an exact, unnormalized **contract module reference** (`MODULE`) into the selected contract environment. The contract itself is still loaded, validated, and bridged exclusively by the `contract.resource.v1` and `source.extern-call.v1` machinery.
* **Bidirectional slice totality (`Q047`).** Binding between a manifest's contract slice and the selected contract environment MUST be **total in both directions** (`Q047`):
  1. every `contract fn` entry in a selected manifest MUST resolve to a declared contract inside the exact `MODULE` it names within the selected contract environment (selecting a contract-slice manifest with no contract environment selected therefore refuses at `Q047`); and
  2. every contract declaration inside any `MODULE` named by the slice MUST be bound by an entry in that manifest.
  Because a contract module in a library's slice is authored specifically for that library, an unbound declaration in that module indicates an incomplete or drifted binding.
* **Per-unit withdrawal on ABI disagreement (`Q048`) and stacked premises (`Q049`).** Once the slice resolves:
  * **ABI agreement and monotone withdrawal (`Q048`):** The declared contract's §C04 ABI projection and the manifest entry's transcribed signature MUST agree. On disagreement, the contract is **withdrawn for the analyzed unit** (`Q048`): the symbol is withheld from the admitted contract environment, `Q048` is emitted explicitly, and every call site in the unit falls to `Q023` as an uncontracted callee (per the unified contradiction rule).
  * **Stacked premises (`Q049`):** Every published certificate calling a manifest-bound contracted function MUST carry **both** premises: (1) the §L14.E.1 declared-contract premise (*what the code does*) and (2) the manifest residence premise naming the contract module, artifact pin, and four ABI premises (*which build supplies it*). Neither premise may replace the other, and a certificate missing either is refused (`Q049`).
  * **Unchanged §C04 surface and automatic waiver exclusion:** The §C04 bridging table alone governs what crosses the call boundary, and writing `unsafe extern fn` for a slice-bound symbol is automatically refused by `Q034` because `Q047` guarantees the symbol is present in the selected contract environment.

See [implemented grammar G02d.2](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02d2-the-native-library-manifest) and the [contract slice evidence](reports/archive/waves-20-39/wave20-native-library-contract-slice/README.md).

---

<a id="l144-the-exported-header-a-manifest-binds"></a>
#### L14.4. The exported C header a manifest binds

A `source.native-library.v1` manifest MAY bind the actual **exported C header** (`.h`) shipped by the library alongside its `sha256` content digest (card `NATIVE-LIBRARY-HEADER`; `docs/design/NATIVE_LIBRARY_HEADER_V1.md`).

* **All-or-nothing field pair.** The header reference and header `sha256` digest are optional **as a pair**: a manifest specifies both or neither (`Q039` if only one is written). Naming neither leaves §L14.2 and §L14.3 completely unchanged.
* **Token-stream header reader and bounded C subset (`Q063`).** When a header is bound, the declaration of **every** symbol the manifest binds (`unchecked fn` and `contract fn` alike) MUST be read directly from the supplied header's text and compared against the manifest entry's transcribed signature. To ensure that check-time derivation, manifest validation, and standalone certificate rechecking reproduce identical decisions without host-compiler drift (the reader discipline of `docs/reports/archive/early-waves/remaining-loader-abi/README.md`), the v1 header reader operates directly on the C token stream rather than invoking a full C AST frontend. It admits comments, backslash-newline splicing, a whole-file include guard, and depth-zero function declarations; **every shape outside that bounded subset refuses the manifest by name at `Q063`**, including:
  * an unsupplied header or a header whose measured `sha256` disagrees with the manifest's declared digest;
  * preprocessing directives outside `if`, `ifdef`, `ifndef`, `elif`, `else`, `endif`, `define`, `undef`, and `include`, or unbalanced conditional groups;
  * a bound symbol missing from the header, declared only inside a conditional region (no `#if` condition is evaluated), or defined as a preprocessor macro;
  * a `static` declaration (internal linkage cannot be an artifact's exported symbol);
  * an empty `()` parameter list (unspecified parameters under C11 6.7.6.3p14) or a variadic `(…, ...)` declaration;
  * two depth-zero declarations of the same symbol whose ABI projections disagree; and
  * a C type outside the reader's committed subset, or a C type the reader *does* recognize for which the manifest's ABI vocabulary has no spelling (e.g., function pointers, or struct parameters on the non-FFI extern route—reported distinctly from unreadable syntax because a vocabulary boundary and a reader boundary have different remedies).
* **One-directional totality.** Unlike §L14.3's bidirectional contract-slice totality, header totality is deliberately **one-directional**: every symbol bound by the manifest MUST appear at depth zero in the header (`Q063`), whereas extra declarations in the header that the manifest does not mention are ignored (a public C header describes an entire library surface authored independently of any Quirin slice).
* **Refusal on manifest-versus-header disagreement (`Q064`).** If a manifest entry's transcribed signature and the exported header's declaration disagree in arity, parameter type, or return type, loading **refuses outright** (`Q064`), never withdrawing to an unverified-header manifest.
* **Verified header digest versus stated artifact pin.** Because the checker reads the header bytes at check time to decide admission, the header's `sha256` digest is **verified**, whereas the binary artifact's `sha256` in v1 remains a **stated pin**. Published residence premises carry both and MUST distinguish verified digests from declared pins in rendered output. `sha256` is the sole admitted hash algorithm.

<a id="l1441-declared-include-texts-and-derived-export-decorations"></a>
##### L14.4.1. Declared include texts and derived export decorations

Real C library headers commonly place export-visibility and calling-convention macros (such as `FOO_API` or `__attribute__((visibility("default")))`) in companion headers included from the main `.h`. A manifest that binds a header under §L14.4 MAY additionally declare the exact companion texts that the header's `#include` directives are permitted to splice (card `NATIVE-LIBRARY-HEADER-DECORATION`).

* **Closed, digest-verified include graph (`Q063`).** Each declared include entry specifies three fields: (1) the exact `#include` operand spelling written in the header, (2) the reference from which the companion text is supplied, and (3) its `sha256` digest. Declaring none is the default and preserves §L14.4 unchanged.
  * Any `#include` operand in a header that the manifest does **not** declare is never followed (preserving offline determinism).
  * Every declared include text MUST be supplied, MUST match its declared `sha256` digest *before* any token is read, MUST conform to the same bounded §L14.4 token subset, MUST be reached by a matching `#include` directive (spliced at most once), and no undeclared extra include text may be supplied on the side—violating any of these conditions refuses at `Q063`.
  * The verified `sha256` identities of all declared include texts are published alongside the main header's digest inside the residence premise.
* **Text-derived erasable declaration decorations (never manifest-asserted).** A manifest is **forbidden** from listing macro names to ignore or erase: allowing a manifest to assert that `FOO_EXPORT` is harmless would turn a lexical reader boundary into an unverified claim about C preprocessing. Instead, the header reader **derives** that an identifier is an *erasable declaration decoration* purely from the declared texts' own tokens, erasing it iff **all four** conditions hold:
  1. **Object-like only:** every `#define` of the identifier across the declared texts is object-like (never function-like);
  2. **Closed erasure vocabulary:** every definition's replacement list consists solely of tokens in the reader's published ABI-neutral decoration vocabulary—tokens that alter neither the C type, the parameter vector, nor the linkage symbol name—or identifiers already proved erasable by earlier derivations;
  3. **Never undefined:** the identifier is never the target of `#undef`; and
  4. **Path-universal definition without evaluating `#if` conditions:** the identifier is defined on **every syntactic path** through the conditional preprocessing structure preceding its first declaration use, where a conditional group establishes definedness iff it includes an explicit `#else` branch and *every* arm of the group establishes definedness.

Because condition 4 holds across all arms without evaluating preprocessor expressions, deleting the identifier is sound under every consumer configuration. Non-derivable macros are left in the token stream and trigger the existing §L14.4 `Q063` refusal; ABI-neutral decorations written inline in prefix or suffix declaration positions are erased by the same closed vocabulary, while unknown attributes/decorations refuse by name.

---

<a id="l145-the-source-files-own-library-binding"></a>
#### L14.5. The source file's own library binding (`extern library`)

Sections §L14.2–§L14.4 allow *manifests* selected by the builder to claim exported symbols across the compilation's entire selection. Card `NATIVE-LIBRARY-SOURCE-BINDING` lets the **Quirin source file itself** state which manifest-bound libraries its extern declarations resolve against, via the file-scope declaration:

```qcore
extern library NAME;
```

* **Import-adjacent, exact-match syntax.** `extern library NAME;` declares no value/type identifier, reserves no new keyword (`extern` and `library` were already source words), and grants no capability of its own. `NAME` is compared byte-for-byte against a selected manifest's `library` field and is **never normalized**: two spellings that would normalize together are distinct selections.
* **Opt-in selection narrowing and totality (`Q065`–`Q067`).** A file with no `extern library` declaration resolves extern symbols selection-wide across all selected manifests exactly as in §L14.2–§L14.4, and does **not** force manifest loading if the file triggers no manifest-reading route. A file writing one or more `extern library NAME;` declarations resolves its declared extern surface **only** against the manifests it named, under four rules:
  1. **Total binding resolution (`Q065`):** Every `extern library NAME;` written in a module MUST resolve to **exactly one** selected manifest—even if the file declares no `extern fn` at all. An unknown library name, a library name declared by two or more selected manifests, a duplicate `extern library NAME;` in the same file, or a binding written with no manifests selected refuses the module (`Q065`).
  2. **Foreign-manifest symbol refusal (`Q066`):** If the file declares an extern symbol (`extern fn` or `unsafe extern fn`) that is bound by some selected manifest the file did **not** bind via `extern library`, the module is **refused outright** (`Q066`, checked prior to `Q042`). It never falls back to an unmanifested reading.
  3. **Unclaimed symbols unaffected:** A symbol that **no** selected manifest claims is unaffected by `extern library` on the `std.extern` / `std.extern.unsafe` routes (absence of any manifest claim is distinct from a claim by an unbound manifest; contrast `.qrlib` v2 FFI linkage in §L14.8, where every FFI C callee must be bound).
  4. **Stacked source-binding premise and tripwire (`Q067`):** Every published certificate for a module writing `extern library NAME;` MUST carry a source-binding premise for each bound library—naming the written spelling, the resolved manifest, its stated artifact pin, and its four ABI premises—even if the module calls zero functions from that library (`Q067` if omitted). Source-binding premises stack beside per-callee premises and replace none.
* **Strict file locality.** Following §L12's file-local `#import` discipline, `extern library` resolution is **per input file**, governed by the file that *declares* the extern symbol: a binding in one file of a multi-file bundle neither grants resolution to nor restricts another file. Because every symbol resolved by a bound file would also have resolved selection-wide in an unbound file, adding `extern library` is strictly monotone in the refusing direction.

---

<a id="l146-imported-c-structs-and-the-file-wide-trust-mark"></a>
#### L14.6. Imported C structs and the file-wide unsafe-FFI assumption

Card `W48-QR-C-STRUCT-IMPORT` (`source.ffi.v1`; [implemented grammar G02u](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02u-imported-c-structs-sourceffiv1); contract specification §C04.12) enables Quirin programs to import real C `struct` layouts from C headers and pass them across marked C contract functions.

* **File-local opt-in (`Q032`).** A source file enables C struct FFI with `#import std.ffi.unsafe;` and imports a C struct using:
  ```qcore
  struct! { type T = c"struct TAG" from "HEADER"; fields { … } }
  ```
  Writing `struct!`—or, in a multi-file FFI bundle, defining a helper function—in any file that does not itself write `#import std.ffi.unsafe;` is refused (`Q032`). Mixing `unsafe extern fn` into an FFI file is refused (`Q034`).
* **The file-wide unsafe-FFI assumption (not the `Unsafe` verdict).** Importing C marks **every verification result from the entire file**, not merely individual statements touching C. This mark is a **named conditional assumption** (§L20)—never the `Unsafe` verdict, which remains reserved exclusively for validated reachable violations:
  * It states that the recorded compiler's imported C struct layouts match the linked C code and that every called `[| quant #c_ffi |]` contract function behaves as declared.
  * Every certificate from the file MUST list every imported struct (with its verified header digest), every called `#c_ffi` function, and the recorded compiler; `quirin check` renders the best possible outcome as `"Safe under the … unsafe-ffi assumption(s)"`, never plain `Safe` (`Q035` if a certificate attempts to publish plain `Safe`).
  * Under §L21.2 verdict composition, verdicts join and assumption sets union, so the unsafe-FFI assumption propagates to every composed result.
* **Compiler-read, build-asserted layouts (`P-RECORDED-COMPILER`, `Q063`, `Q064`).** Field offsets and struct byte sizes are **never user-declared or guessed**:
  * The source `fields { … }` block lists only direct member names and their §G02t scalar/float types, in layout order.
  * Offsets and total object size are queried from the **recorded Clang compiler** for the builder's recorded target (`P-RECORDED-COMPILER`); if no target is recorded or the running Clang is not the recorded compiler for that target, loading refuses (`Q063`).
  * Unlisted struct members are preserved as **opaque padding/member bytes** that Quirin initializes/copies as part of the aggregate representation and never reads or writes directly (`Q051`).
  * Unresolvable headers/compile contexts, spellings other than `c"struct TAG"`, incomplete structs, unions, bit-fields, flexible array members, anonymous members, layout attributes, and listed members with no §G02t type equivalent refuse at `Q063`. A field listing whose member existence, canonical §G02t type, or layout order contradicts Clang's AST refuses at `Q064`.
  * Emitted C `#include`s the real C header and emits compile-time `_Static_assert` checks for `sizeof(struct TAG)`, every listed `offsetof`, and every listed member type—turning any post-check header drift into a C compile error rather than silent ABI corruption.
* **Boundary crossings and usable imported records (`W48-QR-USABLE-RECORDS`).**
  * Imported C structs behave as §G07.13 records. They cross a C call **as pure data**—passed/returned by value or passed via a `const` pointer that the callee reads without retaining—exclusively through contract declarations carrying `[| quant #c_ffi |]` (`Q033` if `[| quant #c_ffi |]` appears in a runtime/library module or if an imported struct crosses an unmarked contract). Where a C result is rendered or depended upon as a concrete scalar/float value, the call site writes `succeeds LITERAL`, which the emitted C checks at runtime before reading (`Q043` if rendered without `succeeds`).
  * Listed fields may be mutated in place via `RECORD.FIELD = VALUE;` (requiring exact field type agreement and deriving `Mut`; opaque members may never be written), and whole records copy via `let q = r;`.
  * Non-entry `.qr` helper functions in the FFI bundle may accept and return scalars and imported records **by value** (acyclic call graph bounded to depth 16, every helper reachable from the entry, checked at each call site with concrete values, no `succeeds` tail on `.qr` helper calls; `Q025`/`Q026`), operating under the same file-wide unsafe-FFI assumption aslong as each helper's own file imports `std.ffi.unsafe` (`Q032`).

---

<a id="l148-c-linkage-through-generated-manifests-and-its-limitations"></a>
#### L14.8. C linkage through generated manifests (.qrlib v2) and platform boundaries

Card `W48-QRLIB-LINKAGE` (user decisions 2026-09-30; implemented grammars §G02d.2.1, §G02d.3, §G02u; contract specification §C04.12) closes the gap between source-level C FFI declarations and actual binary link inputs. Every C function called by a file that imports C (`#import std.ffi.unsafe;`) MUST resolve through a **generated `.qrlib` v2 manifest** produced from the binary artifact's bytes prior to linking, and the calling file MUST explicitly bind that library with `extern library NAME;` (`Q065` if an FFI file calling C binds no library or binds a v1 manifest; `Q066` if an FFI callee is not exported by a bound manifest; `Q041` if the generated entry's contract module or C signature disagrees with the callee; `Q064` if the manifest's header digest disagrees with the `struct!` header of a struct carried by the callee).

Here **linkage means the exported binary symbol table**: every bound symbol MUST be exported and defined in the pinned artifact—decided directly from binary object/archive/stub bytes by Quirin's ELF, Mach-O, and COFF/PE symbol-table readers—and `quirin build` re-hashes every pinned file at link time and passes **exactly** those pinned files to the native linker. Every published result then carries, per called C function (`Q067` if missing), a `linked:NAME` premise naming:
* for **static linkage**: symbol `X` from artifact `Y@sha256` (plus every file named by a GNU `ld` script), under ABI premises `Z`; or
* for **dynamic linkage**: symbol `X` from link-time stub `Y@sha256` **plus** the named runtime library premise `R`, under ABI premises `Z` (§L20).

##### What a generated linkage premise does not claim (normative)

Every item below is a mandatory boundary on every verdict carrying a `linked:NAME` premise:

1. **Artifact identity is not code verification.** A `.qrlib` v2 manifest proves *which binary symbol table* was linked, never that the linked C implementation is memory-safe or satisfies its contract. Every result retains the file-wide unsafe-FFI assumption of §L14.6; each `linked:NAME` premise stacks beside it and never replaces it.
2. **Dynamic runtime libraries remain unpinned premises.** A dynamic manifest hashes and pins the *link-time stub* (an ELF `.so`, an Apple `.tbd` text stub or `.dylib`, or a Windows COFF `.lib` import library) and records the *runtime library identity*—ELF `DT_SONAME`, Mach-O install name, or Windows DLL name plus OS build—as a named premise without hashing the runtime image loaded at execution time, because the operating system controls dynamic loading (running against a different runtime image is outside the premise). On Windows, the generator inspects the runtime DLL's export directory at generation time to refuse missing or forwarded exports, without hashing the DLL. On macOS, `.tbd` stubs are read for the recorded target's architecture slice; for an `arm64-macos` target whose SDK `.tbd` lists `arm64e-macos` and no `arm64-macos` slice (as in the macOS 27 SDK's `libSystem.tbd`), the reader selects `arm64e-macos` matching Apple `ld`—a deterministic function of the pinned `.tbd` bytes, with no other cross-slice substitution permitted.
3. **Weak-symbol interposition.** Defined weak symbols are admitted and explicitly tagged `weak` in the manifest. However, a strong definition of the same symbol elsewhere in the static link (another object or the main executable), or an earlier-loaded shared object at runtime, can interpose over a weak symbol; a verdict does not claim the pinned weak definition is the one executed.
4. **ELF symbol versioning.** Versioned ELF symbols bind their default (`@@`) version, recorded in the manifest and verified by staleness checks; symbols offering only hidden/non-default (`@`) versions are refused. Dynamic loader version resolution at run time falls under boundary 2.
5. **GNU indirect functions (`ifunc`).** `STT_GNU_IFUNC` definitions are admitted and tagged `ifunc` in the manifest: because an `ifunc` resolver selects an implementation variant at load time, the manifest pins the containing artifact rather than the specific CPU-dispatched variant executed. (Where glibc's static `ifunc` resolvers reference symbols defined only by a fully static `libc.a`, linking static `libm.a` `floor` into a dynamically linked executable fails at link time rather than producing an unsound run.)
6. **Transitive undefined references inside static archives.** Static linkage pins the archive/object containing the called symbol, but any undefined symbols referenced *by* that archive member (e.g., `libm.a` referencing `libc`'s `errno` or CPU-feature tables, or a Rust `staticlib` referencing platform runtime symbols) are resolved from whatever else the link line supplies; the manifest neither pins those transitive providers nor claims which code satisfies them.
7. **Bounded one-level GNU `ld` scripts.** When a library path names a GNU linker script (such as `libm.a` or `libm.so` on glibc systems), the generator follows it **at most one level**: both the script itself and every file path it names are `sha256`-pinned in order, preserving `AS_NEEDED` annotations (an `AS_NEEDED` file is still hashed and pinned even if the linker ultimately omits it). Any script containing `-lNAME` search directives, relative paths, nested linker scripts, directives outside `GROUP`, `INPUT`, `AS_NEEDED`, and `OUTPUT_FORMAT`, or a mix of static and shared inputs is refused.
8. **Windows ordinal imports, export forwarders, and API sets.** Windows imports MUST resolve by symbol name; ordinal-only imports are refused, as are absent runtime exports and export forwarders (`DLL.NAME`). Imports targeting Windows API-set contract DLLs (`api-ms-win-*`), which the OS loader redirects to host DLLs at runtime, are checked against the API-set import stub's exports as read, with loader redirection covered by boundary 2.
9. **Archive symbol indexes and excluded object formats.** An archive member satisfies a symbol only when the archive's own symbol table index lists that symbol for that member (supporting GNU `/` and `/SYM64/`, BSD `__.SYMDEF`, and COFF linker members); indexless archives satisfy nothing. Thin archives, universal (fat) Mach-O binaries, `/bigobj` and LTCG/LTO COFF objects, and section-header-less ELF binaries are refused unread.
10. **Cross-platform `libm` last-ULP variation.** Per §L14.7 limitation 11, C math library implementations (glibc, Apple `libm`, Windows UCRT) may differ by up to the last unit in the last place on non-exact operations; cross-platform bit-identity of `libm` calls is never claimed, each platform is compared only against its own reference, and the card's native `libm` test rows exercise exact IEEE operations.
11. **External linker bypass.** Because a `.qrlib` v2 manifest is a pure function of the artifact bytes and declared inputs, any drift between a stored manifest and fresh regeneration is flagged stale, and `quirin build` / `LinkPlan` re-hashes and links the exact pinned paths (verified on Linux via linker `--trace`). Invoking an external C compiler or linker manually outside `quirin build` / `LinkPlan` steps outside every verdict, just as substituting an unrecorded C compiler does.
12. **Platform execution evidence scope.** Native end-to-end execution runs cover Linux/Clang and arm64 macOS (card `W48-QRLIB-MACOS`, green on macOS 27.0.1 / Apple clang 21.0.0 across SDK `libm.tbd` under three build modes, a Mach-O C archive, and a Rust `staticlib`, wired into the `macos` CI job). Windows MSVC static-library and `ntdll.dll` import-library checks are wired into `msvc-core` CI without native execution on Linux hosts, and Intel macOS is out of scope (current macOS no longer supports Intel Macs; user decision 2026-09-30).

---

<a id="l15-lowering-observations-and-translation-certificates"></a>
## L15. Lowering, observations, and translation certificates

Sequential lowering from source or semantic IR into the typed reduced-C11 CFG and emitted C11 code MUST preserve:
* checked resource identities and generations;
* ordered acquisition, transfer, loan, and release lifecycle events; and
* modeled normal outcomes and typed-error payloads, ensuring that the tagged C error representation never bypasses required cleanup via unmodeled jumps or control transfers.

#### Observation equivalence and translation certificates

Quirin's translation-validation pipeline compares identified execution observations across source/reference interpreters, CFG interpretation, structural checks on generated C, and recorded native binary runs:

* **Mandatory certificate identity.** Every `TranslationCertificate` MUST bind the exact source model, lowered CFG, emitted C artifact (where applicable), observation semantics identity, and supplied execution environment. A translation certificate produced for one lowering phase MUST NOT be reused as proof of another phase without an explicitly checked composition relation, and a recorded native run is an *observed execution witness* for one input/schedule—never a substitute for static all-path verification, universal compiler correctness, arbitrary C undefined-behavior absence, or uninspected target triples.
* **Byte-for-byte stream comparison (single-writer degenerate case).** On every single-threaded route—and on multi-threaded task programs whose output lines are emitted solely by the parent thread (including buffered worker trace events replayed deterministically by the parent inside `join`)—each output stream (`stdout`, logger/trace stream) has a single writer thread, so reference and native observations are compared **byte for byte, per stream**.
* **Windowed partial-order stream comparison (`qcore.obs-reordering.v1`).** Where a concurrency backend permits independent threads to write observation records concurrently (such as worker-body `print` / `log` lines on `source.std-io-tasks.v1`, card `OBS-REORDERING`, `crates/qcore-core/src/obs_reordering.rs`, normative design `docs/design/OBS_REORDERING_V1.md`), validation MUST NOT demand one incidental total instrumentation order, nor may it erase required lifecycle, fork/join, synchronization, or cleanup edges:
  * **Declared reference relation, never inferred from native output:** An `AdmittedReordering` is derived strictly from the checked model's structure (`AdmittedReordering::declare`) and stamped on the certificate (`admits_reordering` under relation identity `qcore.obs-reordering.v1`). Native runs are only ever checked *against* the declared relation, never used to construct or relax it.
  * **Window-by-window, emitter-by-emitter comparison:** Each stream is partitioned into monotone **concurrency windows** bounded by quiescent synchronization barriers (points where no worker thread is live). Within each window, records are attributed to the **emitter thread that physically performed the write** (`Emitter::Parent` vs `Emitter::Worker(id)`—deliberately the opposite of F02's causal task-state attribution, because byte streams are ordered by writers). Two streams match iff they have the same window count and, within each window, every emitter's projected subsequence matches byte for byte while distinct emitters' records within the same window may interleave arbitrarily. If two distinct emitters in the same window could emit identical line text, attribution of native lines to writers would be ambiguous and the declaration itself is refused fail-closed.
  * **Stream-lock premise and non-claims:** Whole-line atomicity rests on C11 stream locking (`C11StreamLock`, C11 7.21.2p7–p8); `qcore.obs-reordering.v1` is an observation-stream relation, not a memory-consistency or data-race proof, makes no cross-stream ordering claim, and when every window is a single-emitter barrier (as in single-threaded `source.event.v1` and `source.socket.v1`) it reduces exactly to strict byte-for-byte equality.

---

# Part V — Concurrency, Weak Memory, and Existing-C Analysis

<a id="l16-checked-concurrency-required-rules-and-current-limitations"></a>
## L16. Checked concurrency: models, tasks, closures, mutexes, and epochs

### L16.1. Core concurrency invariants and non-claims

Quirin's concurrency architecture strictly separates **ordinary memory-race freedom**, **resource and protocol-conflict freedom**, and **lifetime/generation safety**. Every concurrency checker MUST state explicitly which of these properties its certificates establish:

* **Non-claims on deadlocks and atomics.** Lock-order checks establish only acyclic or rank-increasing acquisition order within their modeled primitives; they do **not** by themselves establish freedom from all deadlocks. Atomic memory operations do **not** establish pointee lifetime, publication safety, scheduler fairness, or liveness/progress.
* **Pointwise conservation of access authority.** In any full access-capability profile, access authority is indexed by **lifetime generation identity** and **byte/protocol footprint**. At every step, pointwise permission shares across all active task custodians, parked mutex invariants, and completed-but-unjoined child-task capsules MUST be conserved. Any unassigned complement is either explicitly outside the claim or accounted for as an owned reserve; it is never implicitly readable or writable by arbitrary tasks.
* **Task spawn, completion capsules, and join.** Spawning a child task partitions or transfers the parent's authority and mints an explicit task handle. When a child finishes execution, its **completion capsule** retains its return value, typed error, and any surviving authority until an explicit `join` (or another modeled collection protocol) consumes the handle and transfers authority back to the caller according to the child's outcome. Task completion, handle join, and authority return MUST NEVER be conflated with mere index order in a flat event array.
* **Mutex footprints and mutable fact invalidation.** Every mutex protects a declared or derived footprint and invariant. Holding an unrelated mutex grants **zero** access authority over another footprint. Unlocking returns protected authority to the parked mutex state, and any refinement facts asserted about mutable protected memory MUST be invalidated—or freshly re-established—across release and reacquisition.
* **Quiescent destruction and post-join finality.** Destroying a shared resource or mutex requires unique release custody, a live generation identity, and complete **quiescence** with respect to all outstanding accesses, borrows, leases, callbacks, and unjoined tasks capable of reaching that generation. No access or custody transfer may act on a destroyed generation, and a joined task may never execute subsequent events.
* **The `concurrency checked;` marker is not verification evidence.** Writing `concurrency checked;` in a module header is a structural mode marker, not proof of race freedom. The scoped-task source routes check their own lowered typed task IR, while the explicit event-trace checkers remain separate model APIs. Every source-concurrency profile MUST derive and validate its own events, task transitions, access shares, and synchronization witnesses rather than trusting producer-supplied boolean flags.

---

### L16.2. Explicit event-trace checkers (`model.concurrent14` and `CheckedMutexInvariantV1`)

Two model-level checkers validate supplied concurrency traces under distinct assumption boundaries:

#### 1. `model.concurrent14` (`check_concurrent`)

The baseline `model.concurrent14` checker analyzes explicit event traces using interval footprints, fixed initial permission tables, vector clocks, spawn/join happens-before edges, mutex release/acquire edges, and a live custody table:

* Cross-task ordering in the input event slice creates **no** happens-before edge; synchronization arises solely from spawn, join, and mutex unlock/lock edges.
* It rejects conflicting unsynchronized accesses, lock-order inversions, dead-generation accesses or transfers, post-join task events, and non-quiescent destruction (requiring unique custody, absence of other modeled authority, and happens-before ordering after all prior accesses).
* Joining a child task terminalizes the child only after all locks held by the child and all of its own descendants have resolved; retained vector-clock history confers no surviving execution authority. R01/R02 regression suites and reduced Lean lifetime models pin this boundary.
* Certificates stamp `TrustedExplicitTrace` and the initial-authority / unique-lifetime-ID premises, plus `GuardFootprintAuthority` and `SynchronizedCustodyEdges` whenever held guards or synchronized custody handoffs are used. They do not prove faithful trace extraction from arbitrary source code.

#### 2. `CheckedMutexInvariantV1` (`Q610`–`Q627`)

`CheckedMutexInvariantV1` is a sibling trace profile that **discharges** `GuardFootprintAuthority` and `SynchronizedCustodyEdges` inside its own bounded model—its certificates MUST NOT carry either premise, and independent rechecking rejects any certificate that includes them:

* **Single-protector parked shares, fresh guard instances, and derived sync witnesses:**
  * Creating a mutex generation declares its protected interval footprint and consumes the creator's full `FULL_ACCESS` share over every atom in that footprint (`Q617` if already protected, `Q621` if authority is not conserved), guaranteeing each protected interval has **exactly one protector** and zero fallback authority (`Q611` on unguarded access, `Q610` on guard/footprint mismatch).
  * Each `Lock` mints a globally fresh **`GuardId`** (never keyed merely to a `MutexId`), scoping invariant phases (`Open` vs `Closed`, `Q613`–`Q614`), guard-derived borrows (`Q615`), and refinement facts (`Q616`) to that single critical section (`Q612` on wrong task affinity or dead guard; `Q619` on non-quiescent mutex destruction; `Q623` on dead mutex generation).
  * Every cross-task custody handoff supplies a `SyncWitness` that the checker **derives and verifies** against its own vector-clock happens-before history rather than believing a caller flag (`Q620` on forged witness).
  * Certificates retain `TrustedExplicitTrace`, `DeclaredFootprintShape`, and—whenever a `CloseInvariant` transition supplies a declared predicate—`InvariantPredicateDeclared`.
  * Condition variables, try/timed locks, reader/writer locks, upgradeable guards, nested protecting mutexes, recursive re-locking, and task-completion capsules are **explicit represented refusals** (`Q622`), never silent omissions. Joining a child while holding a guard refuses (`JoinWhileGuardHeld` at `Q600`) except via the opt-in `GuardLocalJoin` event (`T03-GUARD-JOIN`), which structurally requires a single closed borrow-free guard (`G1`–`G6`) and a child subtree that never names any mutex, guard, fact, waiter, share, or resource anywhere in the trace (`G7`–`G10`).
* **Opt-in strict lock-rank discipline (`CT33-RANKS`, `Q624`–`Q625`):**
  * Lock ranks are off by default (empty `lock_ranks` map), leaving unranked traces byte-identical. When at least one rank is declared, the discipline activates module-wide:
    1. **No mixed modules (`Q625`):** Every mutex generation created in the trace MUST have a declared rank, and every declared rank MUST name a generation created in the trace.
    2. **Strictly increasing acquisition (`Q624`):** Acquiring a mutex of rank `r` while the task holds any live guard of rank `held >= r` is refused (equal ranks may never nest).
  * The rank check runs *last* among `Lock` preconditions so it never masks stronger rank-free refusals (such as re-locking an already-held generation at `Q600`). Ranks are recorded verbatim on the certificate, re-derived during rechecking, and add **zero assumptions** because a rank is a declared order position—**not** a deadlock-freedom proof (this non-blocking trace model represents neither general wait-for graphs nor waits on joins, I/O, or condition variables). Dynamic, call-propagated, or undecidable rank relations remain represented refusals at `Q622`.
* **Opt-in monotonic protection growth (`CT34-GROWTH`, `Q626`–`Q627`):**
  * Unconditional mid-lifetime shape mutation (`ExtendProtection`) remains an unconditional refusal at `Q618`. Alongside it, explicit `GrowProtection` widens a mutex generation's protected footprint monotonically when **all five** preconditions hold:
    1. **Live guard ownership:** The growing task holds a live `guard`, and the target mutex generation is read directly off `guard` (never named separately).
    2. **Open invariant phase (`Q626`):** `guard`'s invariant is currently `Open`, placing the shape expansion inside the window where the old predicate need not hold and forcing an explicit `CloseInvariant` before unlock or error exit.
    3. **Fresh invariant identity (`Q627`):** The growth transitions the generation to a new `InvariantId` that this generation has **never previously carried** (a wider region is a distinct proposition), and counts as a mutation so the checked `NoWriteOccurred` witness cannot be used to close that critical section.
    4. **Disjoint unprotected memory (`Q617`):** The added interval overlaps no live mutex generation's effective footprint (including its own).
    5. **Creator full-share conservation (`Q621`):** The growing task is the mutex generation's original creator and holds `FULL_ACCESS` over the entire added interval, preserving symmetric return of all parked shares to the creator at `DestroyMutex`.
  * Protection growth is **permanent** (there is no shrink or replacement), recorded in `admitted_protection_growths` and `protected_shape`, re-derived on certificate rechecking, and introduces no new assumption kind (`InvariantPredicateDeclared` attaches to the new `InvariantId`, and `DeclaredFootprintShape` covers the intervals).

---

### L16.3. Guarded source mutex profile (`source.mutex.guard.v1`)

The source compiler does not lower arbitrary concurrent syntax; for mutex synchronization it lowers the bounded `source.mutex.guard.v1` profile ([implemented grammar G02c](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02c-guarded-fixed-footprint-mutex)), translating lexical critical sections, ranked mutex declarations, and explicit `grow` statements into `CheckedMutexInvariantV1` events and independently re-deriving the resulting certificate before publication. Source-to-trace refinement, real C memory layout of declared intervals, truth of declared invariant predicates, deadlock freedom, runtime contention, and general native refinement remain outside the claim (fixed Windows native execution via the emitted C11 thread compatibility layer is documented in [the Windows thread report](reports/archive/language-and-tooling/windows-c11-threads/README.md)).

Within `source.mutex.guard.v1`, four decidable extensions govern invariant witnesses, facts, and child threads ([T03-F record](reports/archive/early-waves/remaining-t03-f/README.md); cards `MUTEX-SHARE-CROSS` and `MUTEX-SHARE-CROSS-EMISSION`):

* **Checked `NoWriteOccurred` close witnesses:** A lexical `close` MAY request the checked no-write witness; lowering admits it **only** if no statement since the matching `open` can write or grow protection, and refuses otherwise (never silently downgrading to a declared predicate witness).
* **Guard-scoped refinement facts:** Any asserted refinement fact mints a fresh `FactId` scoped strictly to the active guard instance; referencing it after that guard releases refuses as a stale fact (`Q616`).
* **Plain lexical child tasks:** A lexical child-task block without a sharing clause receives neither share permissions nor resource custody, so any access to a declared protected resource must be mutex-mediated (`Q611` otherwise), and joining while holding a guard refuses (`Q600`).
* **Mutex-mediated child-task sharing and C emission boundaries (`share PLACE under MUTEX`):**
  * Writing `thread scope NAME share PLACE under MUTEX { … }` (`MUTEX-SHARE-CROSS`) lets a `protected T` payload—Send-but-not-Sync—cross into a child task exclusively under `CheckedMutex` protection: `Q600` refuses if `MUTEX` is not a mutex generation of the module (*absent*) or protects a different resource (*mismatched*), the declaring child may touch no declared resource other than `PLACE`, and any unguarded child access refuses inside `CheckedMutexInvariantV1` at `Q611` (`QCore.MutexShareCross.aSharedPayloadIsReachableOnlyThroughTheProtocol`, `everyReachAlongAnAdmittedTraceHoldsTheProtocol`).
  * **Emitted C vs check-only traces (`MUTEX-SHARE-CROSS-EMISSION`):** The mutex C backend (`mutex_c_emit::classify_trace`) emits executable C11 `thrd_t` and `mtx_t` code for single-task traces and for **non-nested fork/join traces whose child event segments are contiguous** between `Spawn` and `Join` (establishing a real OS thread crossing, while explicitly **not** claiming runtime lock contention because the parent performs no trace events while its child runs: `QCore.MutexShareCrossEmission.the_creator_performs_no_event_while_its_child_runs`). Traces using guard-local joins, nested spawns, interleaved parent/child events, custody transfers, waiter registrations, or child typed-error exits have no faithful single/contiguous C rendering and MUST be published **without emitted C** rather than with unverified C. Scoped-task owned-`Buffer` captures and completion capsules remain separate (`MUTEX-SHARE-CROSS-TASK-BUFFER`).

---

### L16.4. Scoped tasks, record-leaf captures, range write leases, and explicit-capture spawn closures

#### 1. Copy and owned-buffer scoped tasks (`source.tasks.copy.v1` and `ScopedOwnedTasksV1`)

The scoped-task source routes lower `concurrency checked;` modules through an opaque checked task IR and emit native OS threads across Linux (C11 `<threads.h>`), macOS (emitted `pthread` adapter), and Windows (emitted `winpthreads`-derived C11 subset):

* **`source.tasks.copy.v1`** ([G02a](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02a-scoped-copy-task-execution), [task-recovery evidence](reports/w6-task-recovery/README.md)) supports immutable scalar value copies (widened across §G02t fixed-width scalars by `W48-QR-SCALAR-TYPES-R4` and IEEE `f32`/`f64` by `W48-QR-FLOATING-POINT`), non-recursive leaf workers returning scalars or `unit`, dynamic thread-creation failure, explicit creator `join`, two-arm `match` `Result` handlers (preserving earlier live siblings across later spawn/worker errors without allowing arm-local bindings to escape), and `?` error propagation (infallible `?` unwraps empty-error `Result`s without synthesizing a dead error edge; non-empty error rows check both arms). Native OS join failure aborts immediately before environment reclamation.
* **`ScopedOwnedTasksV1`** ([G02b](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02b-owned-buffer-task-execution), [owned-tasks evidence](reports/w6-owned-source-tasks/README.md), [worker-allocation evidence](reports/w6-worker-allocation/README.md)) adds affine `own<Buffer>` / `own<Buffer<T>>` (`W48-QR-SCALAR-TYPES-R6`) heap allocations and counted immutable `borrow<Buffer<T>>` read shares (`T04`). Allocation in root or worker frames may fail with `AllocError`; a committed `move` consumes the parent's binding, whereas a failed `spawn` restores all parent bindings and prior sibling states intact. Every worker normal exit returns at most one original input or fresh local allocation and explicitly drops all others; scalar, `unit`, and error exits must explicitly drop all owned allocations (entry functions may never return an owned `Buffer`, as no external custodian exists). Worker-local allocations combine checked 32-bit task IDs with nonzero 32-bit per-task allocation ordinals so resource identities remain schedule-invariant.

#### 2. Scoped range write leases (`lend<Buffer>`)

Cards `WRITE-LEND-TASKS` and `WRITE-LEND-TASKS-EXERCISE` extend the scoped-task transferability judgment and `ScopedTaskLedger` with disjoint byte-range write leases (`ShareProtocol::DisjointWriteLeases`):

* A `spawn` capture `lend PLACE LOW..HIGH` delivers an ungraded `lend<Buffer>` / `lend<Buffer<T>>` parameter (at most one write-lease parameter per worker, `Q020`). Bounds `LOW` and `HIGH` MUST be check-time integer literals satisfying `0 <= LOW < HIGH <= payload_bytes` (`Q600`).
* Multiple workers may simultaneously hold write leases over the same parent `Buffer` iff their byte intervals `[LOW, HIGH)` are **pairwise disjoint** (`OverlappingLeases` at `Q600`); while any write lease is outstanding, parent whole-buffer reads, writes, moves, drops, and read-share mixes are refused (`LeaseOutstanding`, `Q600`).
* Inside a worker body, `qcore_buffer_write(LEASE, INDEX, VALUE)` and `qcore_buffer_read(LEASE, INDEX)` access constant byte offsets relative to `LOW` (byte writes into `bool` payloads are refused so invalid `bool` bit patterns cannot be written); the worker checker derives the exclusive touched bound and the spawn site verifies `LOW + bound <= HIGH` (`QCore.WriteLendTasks.no_byte_has_two_write_authorities`, `QCore.WriteLendTasksExercise.the_footprint_fit_is_exactly_the_rebasing_law`, `no_byte_is_written_by_two_parties`).

#### 3. Records across tasks (`W48-QR-TASK-RECORD-CAPTURES`)

On the scoped-task route ([implemented grammar G02a](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02a-scoped-copy-task-execution)), nominal records cross `spawn` and `join` decomposed into their **leaves**—one task slot per scalar or `own<Buffer<T>>` leaf:

* Whether a record may cross under `copy` or `move` is decided by the transferability judgment's aggregate rule as the exact conjunction of its leaves (`QCore.Transferability.theAggregateJudgmentIsTheConjunctionOfItsLeaves`): `copy` refuses if any leaf owns a `Buffer`, `move` refuses on a purely scalar record or sub-record, whole-record `write` and whole-record sharing modes refuse by name, and continuing `match` arms must leave records in identical per-leaf custody states.
* Leaf custody is strictly conserved across reservation, commit, completion, and join, and restored identically on spawn failure (`QCore.TaskRecordCaptures.custody_is_conserved_across_spawn_and_join_for_records`, `spawn_failure_restores_the_record_exactly`).
* Borrowing or leasing an individual record leaf (`r.a`, `r.b`) operates per underlying resource identity, so two distinct `Buffer` fields of one record MAY be write-leased to two concurrent workers simultaneously (`no_byte_has_two_write_authorities_per_leaf`). Record-valued entry outcomes lie outside the F02 trace alphabet and are refused by the reference trace encoder rather than silently flattened.

#### 4. Explicit-capture spawn closures (`W48-QR-CLOSURES`)

A spawn closure ([implemented grammar G02v](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02v-closures-with-explicit-capture-lists)):

```qcore
spawn SCOPE |CAPTURES| -> TYPE [effects { … }] { BODY }
```

is pure syntactic desugaring over a named leaf worker and introduces **no new concurrency semantics**:

* The parser lifts each closure into a hidden worker function declared immediately before the spawning function, whose parameter list is the explicit capture binder list.
* The task route uniquely determines each parameter's type from the captured place and mode at the spawn site (`QCore.Closures.theClosureParameterIsTheOnlyAdmissibleOne`), producing the exact same worker IR, transfer checks, certificate, and byte-for-byte emitted C as a hand-written named worker (`desugaringPreservesTheTaskIR`).
* **Zero implicit capture:** The closure body's lexical scope contains only its explicit capture binders and module-level globals; referencing any uncaptured outer local refuses at name resolution (`Q013`, `theBodysFreeOuterNamesAreExactlyTheCaptureBinders`).
* **Scoped-task spawn targets only:** Closures are refused by name as first-class values and on the epoch, mutex, bit-field footprint, and FFI routes (`Q038` / `Q600`).

#### 5. Quantitative flat-epoch bounds and error-region task credit

Two additional `Concurrency`-pass profiles check resource residency and error-path credit conservation over flat task scopes:
* **`FlatEpochBoundV1` and `EpochCreditRecoveryV1` (`Q630`–`Q641`, `source.epoch.bound.v1` and `source.epoch.tasks.v1`):** Compute exact quantitative residency and work bounds over a fixed-count flat epoch—counting shared entry cells once as a union, summing concurrent private peaks across independent tasks, retaining completed results until creator `join`, stamping `DeclaredRuntimeOverhead` and `NoScheduleModel` (never claiming wall-clock latency, `Q639`), and checking error-edge credit recovery/abandonment.
* **`CheckedRegionTaskCreditV1` (`Q670`–`Q677`):** Composes error-region allowable credit with the unedited `ScopedTaskLedger`, enforcing exact credit conservation (`initial = Σ frames + Σ child-held + spent + abandoned`, `Q673`) and forbidding pack discharge or scope error escape while any child join, read loan, or child-endowed credit remains outstanding (`Q670`–`Q672`).

---

### L16.5. Concurrent bit-field footprint checks (`source.bitfield-concurrency.v1`)

The `source.bitfield-concurrency.v1` profile ([implemented grammar G09](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g09-concurrent-field-footprint-questions)) derives record-field and bit-field access footprints from explicit source layout declarations and simultaneous worker write captures, then checks and independently re-derives a `DisjointFootprintsV1` certificate (`Q650`–`Q667`):

* **C11 bit-field storage-unit rule (`Q661`):** Two bit-fields allocated within the **same storage unit** occupy a single C11 memory location and are **never** disjoint (`SharedBitfieldUnit`, `Q661`), even when their declared bit offsets and widths do not overlap.
* **Restricted claim boundary:** A passing check establishes **field footprint independence only**, conditional on the recorded `SpawnAndJoinSucceed` and `DeclaredRecordLayout` premises. It establishes **no** spawn/join failure-path custody, task-lifetime safety, reference execution, or C code emission/refinement, and `quirin check` MUST report this restricted property explicitly.

---

<a id="l17-explicit-c11-execution-candidates"></a>
## L17. Explicit C11 execution candidates and bounded atomic publication

### Explicit weak-memory candidate checking (`model.c11-candidate`)

The weak-memory kernel API (`check_c11_execution`) validates an **explicitly supplied C11 execution candidate** consisting of memory/fence/mutex events and candidate relations—reads-from (`rf`), modification order (`mo`), synchronizes-with (`sw`), sequential-consistency order (`sc`), and mutex order—against its implemented C11 memory-model axioms:

* **Single candidate versus whole-program semantics:** Accepting a single candidate execution proves only that *that candidate* is consistent and race-free; it does **not** establish that every reachable execution of a C program is race-free. Conversely, rejecting one inconsistent or racy candidate does **not** prove the C program is safe. Any whole-program weak-memory claim requires a sound all-execution argument or complete candidate-set enumeration, together with pointee-lifetime, publication, and reclamation obligations.
* **No manufactured synchronization and no `volatile` synchronization:** Neither a source importer nor a verifier provider may invent synchronization edges merely to make a candidate pass. Every relation edge MUST either be derived from real atomic/mutex operations or recorded explicitly as an unverified assumption in the evidence record. Under this specification, a C `volatile` qualifier is **never** an independently established synchronization witness.

### Bounded atomic-publication API (`check_atomic_publication`)

Separately, `check_atomic_publication` checks one conditional C11 release/acquire publication idiom over a restricted, fully typed C translation unit ([CT41 synchronization model report](reports/archive/language-and-tooling/ct41-sync-model/README.md)):

* **Conditional property:** A successful `memory_order_acquire` observation of a unique `memory_order_release` flag value happens-before—and therefore orders—one payload write before one payload read, verified via the C11 execution checker.
* **Declared invocation bounds and non-claims:** Exactly **one** publisher invocation and **one** successful consumer observation are **declared analysis bounds**, not source-proved call counts. Plain non-atomic flags, `volatile` flags, and `memory_order_relaxed` operations supply zero synchronization proof and are refused. Each result binds both raw and preprocessed C source digests plus analysis configuration, publishes its exact `(profile, property, scope, bounds, assumptions)` tuple, proves neither termination nor general C safety, and does **not** promote legacy-C resource verdicts or CT41 whole-program coverage.

---

<a id="l18-existing-c-and-interprocedural-summaries"></a>
## L18. Existing C and interprocedural summaries

### L18.1. Clang frontend, dialect/scope discipline, and conservative projection

The unrestricted-C gateway (`c.resource`, `analyze_unrestricted_c11`) first runs Clang with an explicit C dialect, include paths, and preprocessor definitions, and then projects supported C constructs into a conservative resource-custody abstract model. The word `unrestricted` describes what C input can be parsed and type-checked by the production Clang frontend—**not** what the resource analyzer can completely verify:

* **Fail-closed `Unknown` preservation:** Unsupported control flow, unresolved pointer/alias identities, missing callee contracts, and incomplete function-body projection MUST preserve `Unknown` (never silently treating unmodeled paths as custody-neutral).
* **Explicit definition-scope tracking:** Analyzing only definitions originating in the main `.c` file establishes a narrower claim than analyzing every definition brought in by preprocessing headers; that scope choice MUST travel explicitly with every published verdict. Library-specific C source importers may model a subset distinct from the generic gateway and MUST report their own projection and safety denominators separately.

---

### L18.2. Unevaluated `sizeof` and `_Alignof` scalar and pointer-dereference queries

Two token-derived rules admit unevaluated C type/size queries as custody-free scalar markers without treating them as numerical bounds:

1. **Scalar type-name and scalar-variable queries ([wave23-c-operations report](reports/archive/waves-20-39/wave23-c-operations/README.md)):**
   * Admits `sizeof(TYPE)` and `_Alignof(TYPE)` over a readable scalar type name, plus `sizeof` over a single declared scalar variable, as unevaluated scalar data in the generic gateway's six expression positions and in scalar assignments.
   * Modeling such a scalar assignment forgets the destination variable's previous scalar fact; expression-position side effects alongside a query, variable-length arrays (VLAs), opaque/incomplete types, and compound operands remain outside this rule.
   * The query's operand-class marker MUST NEVER be promoted into a numerical byte size, alignment value, or branch condition fact. Summary validation and standalone certificate rechecking reproduce the identical token-derived decision.
2. **Unevaluated pointer-dereference `sizeof` in allocation/call statements ([w44-collection-allocation report](reports/archive/waves-40-47/w44-collection-allocation/README.md)):**
   * Under C11 6.5.3.4p2, `sizeof(*NAME)`, `sizeof(**NAME)`, or `sizeof(NAME[LITERAL])` is unevaluated when *every* declaration of `NAME` in the function is a pointer declarator carrying at least that many `*` indirections, no array dimension, and a type-specifier list resolving to a non-variably-modified type.
   * In a statement whose first call targets an ordinary callee, the lexical access gate treats such an operand as a scalar marker naming **no** resource passed to the callee; contract lookup and result custody for the statement proceed unchanged.
   * Variably modified or unresolved types, array declarators, and all other statement positions retain their existing refusals, and the marker never becomes a numerical extent.

---

### L18.3. Typed scalar accesses, pointer index-traversal (`derived` interior pointers), and 2–4 link member chains

Three fallback rules—consulted **only** on statements or conditions that the generic gateway would otherwise refuse—track pointer-base custody across indirect scalar reads/writes, pointer stepping, and multi-link struct chains:

#### 1. Typed scalar accesses ([w43-indirect-access-general report](reports/archive/waves-40-47/w43-indirect-access-general/README.md))

Consulted only when the generic gateway refused a statement for its memory access, destination, or increment, or flagged it as an unsanctioned `*p` store:
* **Admitted shape:** Every memory access in the statement is `*p` or `p[i]` over a declared `SCALAR *p` or `SCALAR p[N]`, `p->f` over a declared record pointer and scalar member, or `s.f` over a declared record object; both destination and value are scalar; and the statement contains **no** call, unary `&`, comma operator, nested write, or pointer increment.
* **Dereference custody obligation:** Such a statement is custody-neutral (a write through `*p` or `p[i]` preserves the binding frame), provided every dereferenced base `p` is on every reaching path either: (a) unbound, (b) a non-null binding with no tracked target, or (c) bound to **exactly one** known, non-null, live resource identity. Otherwise the function remains `Unknown`, and dereferencing a base released on any reaching path records a `use_after_release` violation candidate.
* **Custody-only boundary:** Bounds, alignment, array extent (subscripts `p[i]` prove nothing about allocation length), and the liveness/nullness of untracked objects lie strictly outside this property.

#### 2. Pointer index-traversal and `derived` interior pointers ([w43-index-traversal report](reports/w43-index-traversal/README.md))

Extends typed scalar access to loops and conditions walking buffers or NUL-terminated strings:
* **Call-free controlling expressions:** A refused controlling expression (`if`, `while`, `for`, `do`-`while`, or `switch`) containing no call, no write, and only typed scalar memory accesses lowers to a read imposing the same dereference obligation on *every* base—even operands reachable only past a short-circuit operator.
* **Interior `derived` pointers:** Stepping operations `p++`, `p += e`, `p = q ± e`, and the post-increment of `*p++` over declared single-level pointers produce a pointer **derived** from `q`'s base binding with an interior offset:
  * A derived pointer owns **nothing** and can never discharge its base allocation's release obligation.
  * Passing a derived pointer that stepped forward by a positive literal to a release function records an `invalid_release` candidate; passing one that could still sit at offset zero (via a backward/variable step or a loop-header join with the base identity) refuses (`Unknown`).
  * Dereferencing or stepping a derived pointer after its base identity was released records a `use_after_release` candidate.
  * Stepping **in place** (`p++`, `p += e`) the *sole* live binding holding an owned object's identity MUST refuse so custody of the base address is not lost.
* **Finite fixed-point convergence and no-bounds scope:** Joining a declared scalar's branch fact with "no fact" yields "no fact", and copying or returning an unfacted scalar carries no custody. The worklist reaches a finite fixed point over loop headers (`Unknown` if unrepresentable). **No bounds claim is made**: offsets and indices are never evaluated beyond testing whether a step is a positive literal, and the published scope string states this explicitly.

#### 3. Bounded 2–4 link struct member chains ([w44-indirect-chain-access report](reports/archive/waves-40-47/w44-indirect-chain-access/README.md))

Both the typed scalar-access route and the index-traversal route additionally admit **2-to-4-selection member chains**—`r->l1->…->f`, `(*r).l1->…->f`, or `s.l1->…->f`—under strict structural and tracking constraints:
* **Admitted chain types:** Every intermediate member selected and dereferenced MUST be declared `struct T *` over a complete, unambiguous, non-union C `struct`, and the final member `f` MUST be scalar. Pointer-valued final members, nested value-aggregate sub-objects, array subscripts inside the chain, and `void *` or non-struct pointer links are excluded.
* **Root and link obligations:** Every pointer dereferenced along the chain carries a custody obligation. The root identifier MUST never be rebound inside the function body and must be unbound, non-null untracked, or bound to one known non-null live identity. Any link read from an untracked object is itself untracked; a link read from a *tracked* root object MUST match a member place bound by an admitted aggregate store either to one live identity equal to its recorded field link or to no target with no field link. Tracked objects deeper than the first link—and all other link shapes—refuse (`Unknown`).
* **Custody-only boundary:** Evaluating the chain mutates no bindings and establishes **custody safety of tracked bases only**; nullness and liveness of untracked intermediate pointers remain outside the claim.

---

### L18.4. Indexed heap resource collections (`T **v` segment partitions)

When the generic gateway refuses a statement or `if` condition operating on a body-acquired pointer array declared `T **v` whose elements own heap resources, the indexed-collection rule ([w43-indexed-resource-collections report](reports/archive/waves-40-47/w43-indexed-resource-collections/README.md)) partitions the non-negative index line into symbolic half-open segments `[lo, hi)`:

* **Symbolic bounds and segment states:** Segment endpoints are integer literals or `max(0, x + k)` for a declared integer variable `x` designated by no other lvalue; each segment is tracked as `unfilled`, `null`, `live`, or `released`.
* **Admitted element operations (over index `I` = literal, declared integer, or `n++`):**
  * `v[I] = ACQUIRE(…)` (owned-return heap contract with all-borrowed parameters): moves fresh custody into index `I`; storing over an already `live` non-null segment records a **leak** candidate.
  * `v[I] = NULL`: marks index `I` `null`.
  * `RELEASE(v[I])` (single-parameter heap release contract returning `void`): discharges one `live` element; releasing an already `released` segment records a **`double_release`** candidate.
  * Null tests on `v[I]`, plus canonical counted loops `for (i = 0; i < n; i++)` whose body mutates neither `i` nor `n` and has no non-header entries or early exits (carrying `i < n` into the body and rewriting `i` to `n` on loop exit).
* **Fail-closed fallback and exit rules:**
  * If `v` does not resolve to **one known, live, non-null heap allocation** acquired by the current body, the statement retains its generic-gateway refusal byte for byte.
  * Reading or operating through a released array `v` or released element `v[I]` records a `use_after_release` candidate; any operation whose index cannot be proved on the segment partition records an unknown alias and yields `Unknown`.
  * Releasing, retaining, or refuting `v` at function exit while any `[lo, hi)` segment remains `live` leaks those elements; returning or consuming `v` transfers its elements out, and no single-identity W4 summary is published for it.
  * **Heap kind and custody-only scope:** Both `v`'s elements and their acquire/release contracts MUST be of heap kind (streams, `DIR *`, and file descriptors—or statements reading tracked descriptors—keep their refusals). Segment tracking decides **which element indices owe a release**, never whether `I` lies within `v`'s allocated byte capacity.

---

### L18.5. Typed reads in contracted call actuals

When a simple call statement was refused by the generic gateway solely for an indirect read inside its argument list, the call-argument rule ([w44-indirect-call-argument-access report](reports/w44-indirect-call-argument-access/README.md)) admits that read iff **all** of the following hold:

* **Position and statement re-classification:** The indirect actual stands only at a declared `Borrowed` parameter position or inside a declared variadic data tail, and re-classifying the statement with those actuals admitted yields a valid contracted call whose result destination is its exact left-hand side (or an admitted indexed-collection element store). Custody-consuming positions, uncontracted callees, and statements reading tracked file descriptors retain their original refusals byte for byte.
* **Admitted actual forms:** (1) Scalar expressions over §L18.3 typed scalar accesses; (2) single-step pointer reads `B[I]` / `*B` over a declared `T **B`, or `B->F` / `S.F` over a pointer struct member; (3) sub-object address-of `&B->F`, `&S.F`, or `&B[I]` over a non-pointer member/element; and (4) `sizeof(TYPE)` constants.
* **Dual pre-call obligations:** Before applying the call's contract transfer, the analyzer enforces:
  1. **Base dereference check:** Every base dereferenced by the actuals must be unbound, non-null untracked, or one known non-null live identity (else unknown alias → `Unknown`; released base → `use_after_release` candidate).
  2. **Alias/containment exclusion:** Refuses any identifier carrying an integer-encoded resource identity, any pointer read through a base whose tracked target owns a field link, contained child, or indexed collection, and any locally bound pointer place.
* **Custody-only scope:** As stated in the published scope string, this rule checks custody of the **dereferenced base object** only—never the liveness or validity of the pointee named by a loaded pointer member/element.

---

### L18.6. B-series expression-position calls and desugared branch-condition calls

Two rules lower contracted calls appearing inside `return` expressions and branch conditions while preserving exact C evaluation and short-circuit paths:

1. **Bounded B-series return and condition calls ([b-series-integration report](reports/b-series-integration/README.md), [gateway constraints 5d–5f](agents/gateway.md#constraint-5d)):**
   * Models one **unconditional, externally declared contracted call** (having no local definition in the translation unit) inside an otherwise custody-neutral `return` or controlling expression, preserving its ordinary resource transfer and CFG successors.
   * The scalar-return call extension likewise requires an external callee with no local definition and zero custody return; direct opaque-member return reads establish no escaping resource identity.
   * Multiple one-level nested sibling calls are admitted **only** when every inner callee contract is resource-neutral and all argument, liveness, and alias obligations pass. Deeper nesting and short-circuit effectful calls not covered below remain `Unknown`.
2. **Boolean-tree desugaring of call-bearing branch conditions ([w44-branch-condition-calls report](reports/archive/waves-40-47/w44-branch-condition-calls/README.md), [gateway constraint 5h](agents/gateway.md#constraint-5h)):**
   * Consulted only when a controlling expression containing calls was refused by every earlier rule, and admitted only when **every call in the expression is modeled**:
   * Reads the condition as a Boolean tree over `&&`, `||`, `!`, and top-level `?:` (treating `__builtin_expect(E, K)` with literal `K` as `E`).
   * Lowers each operand's call—in exact C evaluation order along CFG edges so short-circuited operands run **only** on paths where C evaluates them—to the ordinary statement classifier's contracted call requiring a **resource-free result** (`none` or `derived`), followed by the operand's branch decision (a neutral residual, or for call-free leaves an existing null, validity, distinctness, or typed-read test).
   * Admits at most one inline assignment-call `(NAME = CALL(…))` per operand into a declared scalar or single-level pointer local `NAME` (neither a formal parameter nor a file descriptor), lowered as the call statement followed by testing `NAME`.
   * **Strict byte-for-byte refusal preservation:** Any uncontracted or refused call, custody-producing return (`owned`, `reallocated`, or descriptor), local callee in the function's own recursive SCC, operand containing two effectful calls, type operator (`sizeof`/cast), compound statement/braces, increment, indirect call expression, or undecided leaf MUST preserve the controlling expression's prior refusal byte for byte, including diagnostic site numbering.

---

### L18.7. W4 interprocedural summary inference and bounded indexed child/next lookup

#### 1. W4 interprocedural custody summary inference

External `.qv` resource declarations remain trusted assumptions, whereas internal C helper functions without declared contracts may have custody summaries inferred by **W4** (`legacy_summary.rs`) under strict validation and precedence rules:

* **Validation by full body re-analysis and certificate rederivation:** No candidate summary is ever published from a syntactic scan alone; every candidate is validated by re-analyzing the callee's body under its entry assumptions across every modeled exit, checking formal-parameter dispositions, return custody, and absence of unmodeled alias escape. Standalone certificate checkers **re-derive** inferred summaries from the retained original C source rather than trusting serialized summary labels.
* **Declared-contract precedence and recursive SCCs:** Explicitly declared contracts always take precedence over inference; if a function's analyzed body contradicts its declared contract, the gateway reports a `SummaryConflict` (or withdraws a refuted contract under C06.4) rather than silently overriding the declaration. Unresolved recursive strongly connected components never acquire self-justifying inferred summaries; context-specific zero/nonzero recursive specialization requires calls to strictly lower-rank validated instances.
* **Supported W4 summary shapes:**
  * Direct `Out`, `Stored`, `Removed`, `Reallocated`, and nullable-owned-return inference.
  * **Immutable entry-prefix output aliases:** Flat, explicitly typed immutable pointer locals initialized at function entry (`T **slot = out_param;`) may forward modeled `Out` calls or direct stores, with both the original sanctioned slot and every exit independently validated (casts, pointer mutation, variable shadowing, escaping aliases, and branch-local alias bindings remain unsupported).
  * **Null-tolerant `Removed` wrappers:** Admitted only after re-validating every newly admitted null path—confirming a null child carries no custody while a nonnull child starts contained and exits detached/live with its original parent owner retained.
  * General removal/out-slot aliasing, relational numerical branch analysis, arbitrary recursive fixed points, and quantitative cost summaries remain outside W4 (see the [contract specification](QCORE_CONTRACT_SPEC_r0_3.md) for mode and trust semantics).

#### 2. Bounded read-only indexed child/next lookup ([grammar G11](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g11-existing-c-indexed-childnext-lookup), [detach-lookup-membership report](reports/detach-lookup-membership/README.md))

* A bounded, complete, read-only indexed child/next lookup helper may infer a **nullable `Borrowed` return** under fresh-run validation.
* Refining caller identity across such a lookup requires **both** source-derived must-held link facts and independent unique parent custody, with exact matching complete canonical `struct` tags across the callee formal, caller actual, and stored-link owner (unknown or mismatched tags refuse identity refinement).
* `Borrowed` provenance alone NEVER establishes container membership or a fresh allocation site, and missing physical field order remains `Unknown`. The current source bridge admits exact stored-head and explicit empty-head shapes; whole-return/nested `Removed` composition and general successor-chain closure remain outside it.

---

<a id="l19-contract-clauses-and-verifier-scopes"></a>
## L19. Contract clauses and verifier scopes

Quirin source declarations can carry `requires`, `ensures normal`, `ensures error(Type)`, `ensures any`, and `quant` contract clauses:

* **Structural retention vs consumer interpretation:** The structural parser (`source.structural`) retains clause expression text and byte spans rather than a universally type-checked predicate AST; exact semantic interpretation belongs to the selected contract consumer ([contract specification](QCORE_CONTRACT_SPEC_r0_3.md)). Writing an inline `contract { … }` on a declaration does **not** automatically install a C-gateway resource summary—installation requires the total-or-reject `contract.resource.v1` loader and an explicit contract selection—nor does a fact-policy checker prove caller preconditions or whole-program reachability.
* **Verifier scopes (`verify scope`) and core obligation dispatch (`Q700`–`Q703`):**
  * A `verify scope` declaration selects external verifier-plugin configurations; it grants **zero** ownership, memory-access, or proof authority.
  * Registry resolution (`VerifierRegistry::resolve_source_scopes`) validates plugin IDs, enabled state, version requirements, target profiles, and declared capabilities before normalized facts leave the compiler, reporting invalid scope configurations as `Q700` (with dispatch outcomes in `Q701`–`Q703`).
  * Admitted source scopes enter core obligation derivation and evidence-v2 dispatch (`verifier.v2.observed`), while incomplete source imports or unsupported effects leave their obligation inventories incomplete (`Unknown`, never `Safe`). Source lifecycle derivation remains intentionally incomplete until a full effect/ABI importer lands.
* **Permanent protocol-v1 `Safe` refusal and evidence-v2 replay firewall:**
  * Protocol v1 carries only untrusted Boolean assertions and **permanently refuses every `Safe` proposal** (`QP009`)—even when claims are empty or match requested facts 1-for-1—and downgrades every unverified `Unsafe` allegation to `Unknown` (`QP010`).
  * Evidence-v2 witness replay requires trusted-core `WitnessChecker` verification against the core's own obligation denominator; observation-replay witnesses additionally require explicit `observations_trusted = true` and `observed_runs_only` bounds (an explicit `false` is never overwritten). No first-party source safety claim is promoted without core replay.

---

<a id="l191-formal-contract-hypotheses-and-quant-effects-required-extension"></a>
### L19.1. Formal contract hypotheses, conditions, theorems, and quant effects

> **User requirement & decision (2026-10-26); Proposed evidence level (§L02) with bounded increments noted below.** Contract hypotheses across all providers MUST support decomposed mathematical representation rather than opaque English prose strings (companion contract specification §C03.1, §C04.18). Specifying this revision-0.3 extension does not claim full C-effect valuation across all existing provider corpora until each consumer grammar is registered in [implemented grammars](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md).

#### 1. `quant condition` and epistemic discipline (G01c)

`quant condition` carries explicit position selectors—`entry`, `normal`, `error(E)`, and `any` ([implemented grammar G01c](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md)):
* **`entry` conditions** are caller obligations; **`normal` / `error(E)` / `any` exit conditions** must hold on every applicable terminating exit, supply no entry assumption, and prove no termination.
* **Meaning-preserving decomposition:** Sentence-shaped natural-language postconditions MUST be decomposed by semantic role: pure state predicates become `quant condition`s, state transitions and frame preservation become `quant effect`s, and conditional proof steps become `quant theorem` applications retaining all hypotheses.
* **No conflation of ignorance with negation:** Epistemic absence statements (e.g., *"on error, initialization is not guaranteed"*) MUST NOT be mistranslated into negative state assertions (*"the buffer is uninitialized"*); non-binding explanatory possibilities belong in comments or separately scoped witness claims.

#### 2. The three formal `quant` judgments

1. **Proposition (`quant proposition`, G01p):** Denotes a typed mathematical predicate `P(s, x)` over an explicit state `s` and typed value, relation, or function parameters `x`, with standard first-order quantifiers, integer arithmetic terms, equality, set/domain membership, conjunction, disjunction, implication, and negation (`QCore.QuantProposition`). Sentence-shaped identifiers and quoted prose strings are **not** valid decompositions (enforced by premise/hypothesis prose bans `QF001`–`QF005`).
2. **Theorem application (`quant theorem`, G01t):** Instantiates a registered, audited formal theorem `∀ x, H₁(x) → … → Hₙ(x) → C(x)`. Every premise `Hᵢ(t)`—after capture-avoiding, sort-checked substitution of `t` for `x`—remains an explicit open obligation until discharged; naming a theorem, provider crate, or file digest NEVER discharges its premises on its own.
3. **State-transition effect (`quant effect`, G01e):** Denotes a typed two-state relation `R(s, s′, x)` between identified entry state `s` and exit state `s′`, naming the applicable exit/guard, modified footprint, and preserved frame. A state predicate guarding an effect is a prerequisite, never proof that the transition occurred.

*Current implementation boundary:* `quant proposition` (G01p) is implemented as a typed hypothesis representation with mathematical semantics in `QCore.QuantProposition` (paired with conditional decomposition theorems in the libc and CMetrics Lean models, which are model proofs rather than C AST extractions), and `quant theorem` / `quant effect` (G01t/G01e) have bounded structural/model representations while trusted C-effect valuation remains Proposed. Interpreted C-summary loaders explicitly reject standalone unvalued model prerequisites rather than erasing or trusting them, and existing `quant live`, `quant deref`, `quant checks`, condition-binding, and `#outer` contract forms retain their semantics unchanged.

#### 3. Soundness, plugin-role instantiation, atomics, and corpus migration

* **Model-to-source obligations:** Every formalization MUST expose its model primitives, source/build identities, environmental assumptions, and source-to-model correspondence obligations. Unproved source facts remain open obligations (`Unknown`, never `Safe`); absence of a fact never proves its negation; bounded runtime observations never discharge universal quantifiers.
* **Plugin interface instantiation:** Applying `quant theorem` to a plugin architecture MUST bind the exact plugin identity, descriptor/interface struct, configuration, and callback table. Input, output, filter, processor, and custom plugin roles carry distinct lifecycle-ordering, ABI, custody, effect, and environment obligations; mere plugin registration proves none of them, and custom interfaces require explicit contracts rather than inheriting built-in role assumptions.
* **Atomic `quant effect` declarations:** Must distinguish backend implementation, operation kind, returned value, read/write footprint, and C11 memory order—separating `compare_exchange` success from failure, retaining mutex-fallback/init failures and platform specifics, never treating a call's local frame as an assertion that concurrent threads leave memory untouched, and never conflating atomicity with happens-before synchronization.
* **All-provider migration accounting:** Migrating provider contracts from legacy prose premises to formal `quant` clauses MUST preserve every guard, branch alternative, quantifier scope, outer assumption, and effect order, pairing each migrated clause with an explicit correspondence proof or retained open correspondence obligation (migration completeness is tracked separately from proof discharge; see contract specification §C03.1).

> **Implementation & Provenance.** `crates/qcore-frontend/src/quant_proposition.rs`,
> `crates/qcore-schemas/src/quant_proposition.rs`, `proof/QCore/QuantProposition.lean`;
> implemented grammars `G01c`, `G01p`, `G01t`, `G01e`, and companion [contract specification](QCORE_CONTRACT_SPEC_r0_3.md) §C03.1.

---

<a id="part-vi--verdicts-reason-categories-diagnostics-and-conformance"></a>

# Part VI — Verdicts, Reason Categories, Diagnostics, and Conformance

<a id="l20-verdicts-and-claim-boundaries"></a>

## L20. Verdicts, declared assumptions, and claim boundaries

### 1. The Eight-Tuple Verification Claim Contract

Every published verification claim MUST identify all eight components of the claim boundary:

```text
(artifact, property, semantic profile, scope, bounds, assumptions,
 evidence version, evidence provenance)
```

| Component | Normative Meaning |
|---|---|
| `artifact` | Exact subject identity under check — source file/module digests, raw and preprocessed C translation-unit digests, function/row key, and build/target snapshot. |
| `property` | Specific semantic property judged (e.g., sequential resource custody, byte-buffer bounds, race/authority conservation, field footprint independence, null-dereference safety, or translation-observation equivalence). |
| `semantic profile` | Named bounded calculus or route under which the input was interpreted (e.g., `source.buffer.v1`, `ScopedOwnedTasksV1`, `CheckedMutexInvariantV1`, `c.resource`, `verifier.v2.observed`). |
| `scope` | Which definitions, entry points, call chains, or runtime-loaded extensions lie inside versus outside the analyzed unit (e.g., main-file C definitions only vs. header definitions; custody-only vs. extent-checked). |
| `bounds` | Explicit quantitative and exploration limits governing the decision — loop unroll ceilings, call-chain depth, path/schedule bounds (`SinglePath` vs. `BoundedPaths`), or `observed_runs_only` replay bounds. |
| `assumptions` | Complete set of undischarged conditional premises on which the verdict rests — target pointer width, IEEE 754 evaluation rules, file-wide unsafe-FFI marks, `.qrlib` linkage premises, and consulted `.qv` contract dependencies. |
| `evidence version` | Wire/certificate schema version under which obligations and witnesses were evaluated (`PLUGIN_EVIDENCE_VERSION = 1`, `PLUGIN_EVIDENCE_VERSION_V2 = 2`, or internal core certificate revision). |
| `evidence provenance` | Producer identity — core analyzer crate/revision, active rule set and off-list, contract-bundle digests, and (where applicable) verified external plugin ID, package version, and package digest. |

> **Reporting vs. single-struct representation.** This eight-tuple is a mandatory **reporting contract** across every user-facing publication (`quirin check`, audit JSON reports, and certificate checkers); it is not yet unified into one single Rust struct across every internal library API.

---

### 2. Conditional Declared Assumption Families

Assumptions are stamped **conditionally and minimally**: an assumption appears on a published verdict if and only if the analyzed unit uses the construct, type, or external boundary that relies on it, and verdict composition (§L21.2) takes the union of all component assumptions so no premise is ever silently dropped.

| Assumption Family | Trigger Condition | Rendered / Recorded Premise Contents | Admitting Routes & Cards | Omit-When-Unused & Tripwire Rules |
|---|---|---|---|---|
| **Recorded target (`recorded target: …`)** | Program names target-sized integers `isize` or `usize` (§L14.0). | Builder's recorded pointer width, target triple, and C compiler in `quirin check`'s declared-assumption list. | • **R3** (`W48-QR-SCALAR-TYPES-R3`): `source.scalar-calls.v1`, `source.buffer.v1`, `source.extern-call.v1`, `source.std-io-extern.v1`<br>• **R4** (`-R4`): `source.tasks.copy.v1` / `ScopedOwnedTasksV1`, `source.std-io-tasks.v1`<br>• **R5** (`-R5`): `source.loop.v1` byte-buffer loops with `isize`/`usize` index<br>• **R6** (`-R6`): `Buffer<isize\|usize>`, `protected isize\|usize` (`source.mutex.guard.v1`), `source.epoch.tasks.v1` task half, composed `source.string.v1` + `std.buffer` | Omitted whenever neither `isize` nor `usize` appears. Using `isize`/`usize` with no recorded target refuses (`Q020`, `Q024`, `Q043`, `Q051`, `Q098`, `Q600`). |
| **IEEE 754 floating point (`IEEE floating point: …`)** | Program holds an `f32` or `f64` value (§L14.0, §L14.0.5). | `binary32` and `binary64` evaluated at their own type width (no excess precision), round-to-nearest-ties-to-even, subnormals preserved, no fast-math, and no FMA contraction. | `W48-QR-FLOATING-POINT` (and float buffer payloads): scalar, extern, composed extern + I/O, C-FFI (`source.ffi.v1`), scoped-task, composed task + I/O, and composed epoch routes. | Compiling the emitted C discharges the compiler-flag subset; contraction and CPU rounding/denormal control-register modes remain runtime premises. Omitted when no float value is held. |
| **File-wide unsafe FFI (`unsafe-ffi`)** | Source file writes `#import std.ffi.unsafe;` (§L14.6). | Every imported `struct!` type (with its verified C header `sha256` digest), every called `[| quant #c_ffi |]` contract function, and the recorded Clang compiler used to query field layouts. | `W48-QR-C-STRUCT-IMPORT`, `W48-QR-USABLE-RECORDS` (`source.ffi.v1`). | `quirin check` prints `"Safe under the … unsafe-ffi assumption(s)"` plus the assumption list; **never publishes plain `Safe`** (`Q035` tripwire). Preserved across all §L21.2 joins. |
| **Generated binary linkage (`linked:NAME`)** | FFI module calls an external C function bound through a `.qrlib` v2 manifest (§L14.8). | One `linked:NAME` entry per called C function: exported binary symbol, pinned static archive/object file(s) + `sha256` (including every file named by a one-level GNU `ld` script), library bound via `extern library NAME;`, the four ABI premises, and—for dynamic linkage—the unpinned runtime shared library (`DT_SONAME`, Mach-O install name, or Windows DLL + OS build). | `W48-QRLIB-LINKAGE` (`source.ffi.v1` × `.qrlib` v2). | Replaces the generic *"the builder links the library that supplies each callee"* premise (leaving only header include paths to the builder). Missing `linked:NAME` premises refuse publication (`Q067`). |
| **Per-verdict C contract attribution (`contract_dependencies`)** | Legacy-C gateway (`c.resource` / bounds / null-deref) derivation consults declared `.qv` contracts. | Per-row `contract_dependencies` list naming each consulted contract's digest, mode, status, body standing (`validated`, `declared-assumption`, or `held`), and premises in the run's dependency table. | `VERDICT-CONTRACT-DEPENDENCIES` ([contract specification](QCORE_CONTRACT_SPEC_r0_3.md) §C06.5). | Refines attribution from run-wide to per-verdict without changing the 8-tuple shape; the field records provenance and decides no verdict on its own. |

*(For `.qr` programs using §L14.1–§L14.5 non-FFI native interop, certificates analogously record bare `unsafe extern fn` waivers (`Q035`), `.qrlib` v1 stated artifact pins, stacked manifest contract-slice premises (`Q049`), verified `.h` / `include` digests, and per-file `extern library` source-binding premises (`Q067`).)*

---

### 3. The Four Verdict Values and Claim Integrity Rules

Every verification judgment evaluates to one of four mutually exclusive values:

* **`Safe`:** The named property is established for the identified artifact, scope, and declared assumptions with **complete obligation coverage** under the selected semantic profile.
* **`Pending`** *(card `W45-USER-CONTRACT-PLUGINS`, approved 2026-09-26; extended by `OUTER-CONTRACT`)*: The property is not decided by the tool and awaits **one of four named human acts**:
  1. completing a deferred human contract review (`awaiting-human-review`);
  2. re-pinning a contract lock after auditing upstream source changes (`contract-pin-mismatch`);
  3. supplying an independently checkable certificate for a contract-attached checker plugin's claim (`plugin-claim-uncertified`); or
  4. reviewing dynamically loaded runtime code against an outer contract (`outer-contract`, emitted in the runtime projection only).
  `Pending` is **not `Safe`**, remains in every safety/coverage denominator identically to `Unknown`, and can be constructed **only** from those four closed reasons (§L21.2).
* **`Unknown`:** The requested property was **not established** — covering unsupported syntax or control flow, incomplete source imports or obligation derivations, missing or refused premises, invalid or stale evidence, provider failure, conflicting contract bundles or composition accounts, or exhausted analysis bounds.
* **`Unsafe`:** A **validated, reachable defect or counterexample** under the selected semantic profile.

#### Mandatory Non-Repurposing and Aggregation Rules

1. **No weak-signal inflation:** The words `Safe`, `Pending`, `Unknown`, and `Unsafe` MUST NOT be repurposed to mean successful parsing, a green compiler build, clean `.qv` contract loading, or passing a handful of runtime fixtures. Rejecting ill-formed source code is a compile-time refusal, not necessarily a semantic counterexample to a well-formed program.
2. **Provisional local defects vs. aggregate uncertainty:** A partial Existing-C report MAY retain local violation candidates alongside an aggregate `Unknown` verdict for developer triage, but MUST NOT erase uncertainty by presenting provisional warnings under unresolved call/alias effects as whole-program `Unsafe` certificates or confirmed upstream bugs.
3. **Anti-vacuousness in aggregate composition:** Aggregate composition MUST account for every obligation in the core-derived denominator; an empty provider coverage set or uninspected translation unit can NEVER certify `Safe`.

> **Implementation & Provenance.** `crates/qcore-schemas/src/{verdict,obligation,plugin}.rs`,
> `crates/qcore-verification/src/{driver,audit}.rs`; cards `W48-QR-SCALAR-TYPES-R3`–`R6`,
> `W48-QR-FLOATING-POINT`, `W48-QR-C-STRUCT-IMPORT`, `W48-QRLIB-LINKAGE`,
> `VERDICT-CONTRACT-DEPENDENCIES`, `W45-USER-CONTRACT-PLUGINS`, `OUTER-CONTRACT`.

---

<a id="l212-the-four-value-verdict-and-its-composition"></a>

### L21.2. The four-value verdict and its composition

> **Normative status.** Approved by user decision on 2026-09-26 (card `W45-USER-CONTRACT-PLUGINS`) as an amendment to §L20 and `AGENTS.md` Hard Invariant 2, with cross-result audit composition approved on 2026-10-24 (card `AUDIT-COMPOSITION`).

#### 1. Total Lattice Order and Reasoned Semigroup (`QCore.VerdictSemigroup`)

The four verdict values form a finite total lattice ordered by increasing epistemic/defect severity:

```text
Safe < Pending < Unknown < Unsafe
```

* **Value join (`⊔`):** Composition along a call chain, across obligations in a function, across functions in a translation unit, or across a checkpoint group is the **join** (maximum in the order above). It is associative, commutative, and idempotent, with `Safe` as its identity element.
* **Reasoned join (`ReasonedVerdict`):** Every verdict carries a finite multiset/sequence of structured reasons. Composing two reasoned verdicts takes the join of their `Verdict` values and the union of their reason collections — associative with empty-reason `Safe` as identity, and commutative up to reason ordering.
* **Closed `Pending` construction (`PendingReason`):** In the shared schema algebra (`qcore_schemas::verdict`: `Verdict`, `ReasonedVerdict`, `PendingReason`, `Chain`), `ReasonedVerdict::pending` accepts **only** a `PendingReason` value (`AwaitingHumanReview`, `ContractPinMismatch`, `PluginClaimUncertified`, or `OuterContract`). No other code path can construct `Pending`. Because `Safe < Pending`, `Pending` is strictly fail-closed: no join or chain extension can ever lower `Pending` (or `Unknown` / `Unsafe`) back to `Safe`.

#### 2. Intra-Unit Gateway Fold, Anti-Masking (`QCore.ReviewHoldStanding`), and Chains

* **Legacy-C function and unit fold:** Within the Existing-C gateway, a function's status is the join fold of its statement/obligation diagnostics:
  * a validated reachable violation contributes `Unsafe`;
  * an unmodeled construct or unresolved alias/effect contributes `Unknown`; and
  * a call **held** for a withheld contract — awaiting human review ([contract specification](QCORE_CONTRACT_SPEC_r0_3.md) §C06.1) or failing its source lock pin (§C06.2) — contributes `Pending`.
  A translation unit's status is the join fold of its functions' statuses, joined with `Unknown` for every function whose body could not be projected. Contract-attached checker plugins (§C11.1) enter as additional chain links on every function applying that contract.
* **Hold-preemption anti-masking (`W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL`):** At an individual C statement, holding a call for review preempts downstream statement-level refusals at that site. To prevent a review hold from hiding a real `Unknown` elsewhere in the same function, any candidate `Pending` function is re-evaluated under the tentative counterfactual in which every held declaration is **in force as written** (counting any violation under that unaccepted declaration as `Unknown`, never `Unsafe`), and joined with every diagnostic strictly above `Pending` produced by that pass (`QCore.ReviewHoldStanding`). Consequently, a function publishes `Pending` **if and only if** accepting all held declarations as written would render the function `Safe`.
* **Three-valued backward equivalence:** Under the coarsening map that sends `Pending ↦ Unknown`, the four-valued gateway fold produces the exact three-valued decision made prior to `W45`; on translation units with zero held calls, published verdicts are byte-for-byte identical.
* **Chains (`Chain`):** An ordered sequence of contract steps — a caller-to-callee contract chain, an obligation pipeline, a multi-unit subsystem, or a checkpoint group — evaluates via the same associative join fold; appending a link is monotone and never lowers a chain's verdict.

#### 3. Provenance-Keyed Cross-Result Composition `⊕` (`QCore.AuditComposition`)

Integrating verification results produced by **separate producers** — distinct audit runs, rule subsets, contract bundles, or external checkers — uses a dedicated provenance-keyed semigroup (`qcore_schemas::composition`, card `AUDIT-COMPOSITION`):

* **Contributions and components:** A producer's output is a *contribution* consisting of a finite set of *components*. Each component is one producer's complete account of a single published row:
  1. **Artifact key:** `(unit, row_key, property, semantic_profile)`;
  2. **Provenance key:** `(analyzer_revision, rule_set_with_off_list, contract_bundle_digests, optional_external_checker_identity)`;
  3. **Verdict payload:** `(verdict, scope, bounds, assumptions, evidence_version)`; and
  4. **Reason leaves:** classified reason records attached to that account.
* **The `⊕` operator:** `⊕` is provenance-keyed component union. It is associative, commutative, and idempotent on byte-identical components, has the empty contribution as its identity, and **erases nothing**.
* **Derived audit rows (never stored as mutable ground truth):** Published unit rows in an audit report are a **pure derived view** computed over per-row components persisted in the `<report>.components.json` sidecar:
  * When one or more distinct-provenance components exist for a row, the published row joins their verdicts and takes the union of their `assumptions` and `reasons` — never lowering a verdict and never dropping an assumption.
  * **Empty-set non-promotion:** A row with **zero** components derives **no row at all** — never the lattice join's algebraic identity `Safe`.
  * **Same-provenance byte conflict (`composition-conflict`):** If two components for the same row share an identical provenance key yet differ byte for byte, **neither is chosen and both are retained**; the derived row is joined with `Unknown` under reason category `composition-conflict` (§L21.1), yielding `Unknown` (or `Unsafe` if any component is itself `Unsafe`).
  * **Clean withdrawal by re-join:** Switching off a rule, withdrawing a contract bundle, or dropping an external checker simply re-evaluates the derived view without that provenance's contribution; historical components are never rewritten.
* **Distinction from intra-run route dispatch:** Cross-result composition `⊕` is **commutative**, whereas the ordered first-answer fallback sequence of compiler routes or C-gateway fallback packages *within* a single analyzer invocation (§L18, §L21) is deliberately **non-commutative**.

> **Implementation & Provenance.** `crates/qcore-schemas/src/{verdict,composition}.rs`,
> `proof/QCore/{VerdictSemigroup,ReviewHoldStanding,AuditComposition}.lean`;
> cards `W45-USER-CONTRACT-PLUGINS`, `W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL`, `AUDIT-COMPOSITION`.

---

<a id="l211-reason-categories-for-uncovered-obligations"></a>

### L21.1. Reason categories for uncovered obligations

Historically, uncovered obligations carried only free-text English strings (`DerivationIncomplete.reason` on `VerificationTask`, `ChunkSourceResult::Unknown { reason }` in source importers, and per-unit `"reason"` fields in audit JSON). Free text is neither machine-queryable nor capable of cleanly separating a limitation in modeling the analyzed source from an analyzer hold, missing compiler environment, or broken provider pipe.

Card `REASON-CODES` (2026-09-08, extended by `W45-*`, `OUTER-CONTRACT`, `AUDIT-COMPOSITION`, and `CONTRACT-SUPERSESSION`) establishes the **closed 13-category reason vocabulary** in `qcore_schemas::reason::ReasonCategory`:

| Category | Subject | Standalone Verdict | Normative Meaning & Precedence | Representative Reasons in Force |
|---|---|---|---|---|
| `unsupported-syntax` | Source / model boundary | `Unknown` | Input is well-formed C or Quirin source, but uses a construct outside the selected profile's grammar; the enclosing function stays `Unknown`. | `unmodeled expression CallExpr`; `unmodeled statement DeclStmt`; `unsupported assignment expression`; `unmodeled call, aggregate or indirect access in return expression`; `callback parameter type outside selected Unix profile` |
| `missing-premise` | Declared input / trust gate | `Unknown` | A required declared assumption or trusted contract input was refused by the caller or omitted from the selection. | `caller refuses the observations_trusted assumption required for recorded observation replay`; `required observation hook … has no extern fn contract in the unit's trusted contract sources` |
| `incomplete-obligation-coverage` | Core capability derivation | `Unknown` | The requested capability has no core obligation derivation (or only a partial inventory) in the current increment, so it cannot reach `Safe`. | `no core derivation for this capability in evidence v2.0`; `per-allocation-site custody derivation runs in the unrestricted-C gateway`; `source lifecycle effect/ABI import is unavailable` |
| `stale-evidence` | Evidence / contract identity | `Unknown` *(never `Pending`)* | Existing evidence or contract metadata is bound to an outdated task digest, source digest, build/profile snapshot, or a body that refutes its own declared contract (`W45-REFUTED-CONTRACT-WITHDRAWAL`, §C06.4). | Witness rejected against a post-deletion task; changed preprocessing input retiring source-effect witnesses; ``call to `k_close` whose declared contract its own analysed body refutes (contract-refuted-by-body)`` |
| `provider-failure` | External verifier host | `Unknown` | Selected verifier plugin was absent, disabled, timed out, crashed, failed request consumption, returned malformed framing, or failed firewall validation. | Any `QP001`–`QP016` host or certificate-firewall rejection path |
| `residual-custody` | Abstract model post-state | `Unknown` | Analysis ran to completion within its model, leaving an unresolved custody, containment, alias, or leak obligation. | Ambiguous call effects retaining unresolved custody; residual leaks contingent on unmodeled paths |
| `environment-unavailable` | Host toolchain / build | `Unknown` *(never source defect)* | Analysis could not execute because a configured compiler, build directory, compilation database, or submodule input was absent or mismatched. | `audit requires the configured Clang executable …; compilation database selects /usr/bin/cc` |
| `awaiting-human-review` | Analyzer hold (human-resolvable) | `Pending` | A consulted `.qv` contract carries `awaiting-human-review` (§C06.1) and is withheld — discharging nothing and accusing nothing (`W45-CONTRACT-DEFERRED-REVIEW`). **Takes highest summary precedence ahead of every other category**; never contributes to provisional `Unsafe`. | ``call to `k_close` whose declared contract awaits human review (awaiting-human-review)``; `resource projection unresolved: …, 1 call(s) held awaiting human review` |
| `contract-pin-mismatch` | Analyzer hold (human-resolvable) | `Pending` | A pinned `.qv` contract's audited C source no longer matches its recorded lock digest (§C06.2) and is withheld (`W45-CONTRACT-REVISION-MODE`). **Takes summary precedence ahead of all source categories, immediately after `awaiting-human-review`**; never contributes to provisional `Unsafe`. | ``call to `k_close` whose pinned contract is not in force: the audited source does not match its lock (contract-pin-mismatch)``; `resource projection unresolved: …, 1 call(s) held for a contract pin mismatch` |
| `plugin-claim-uncertified` | Analyzer hold (human-resolvable) | `Pending` | A checker plugin attached to a declared contract (§C11.1) proposed `Safe` or `Unsafe` without an independently core-checkable witness (`W45-USER-CONTRACT-PLUGINS`). Recorded without deciding; never contributes to provisional `Unsafe`. | ``plugin claim uncertified: `acme.pool_check` proposed Safe without a certificate the audit checks independently (QP016: …)`` |
| `outer-contract` | Analyzer hold (human-resolvable, runtime projection) | `Pending` | In the **runtime projection only** (§C04.20, gateway rule 31, card `OUTER-CONTRACT`), a closed-world fact rests on dynamically loadable plugin/extension code vouched for by an active `#outer` contract (`NAME@DIGEST`). Never contributes to provisional `Unsafe`. | ``rests on outer contract `flb_external_plugin@…` through stable:flb_output_instance.p (runtime projection, rule 31; outer-contract)`` |
| `composition-conflict` | Analyzer provenance merge | `Unknown` *(not human-resolvable)* | Two cross-result components (§L21.2 `⊕`) for the same row under the **same** provenance key differ byte for byte (`AUDIT-COMPOSITION`). Both are retained; row is at least `Unknown`, never contributes to provisional `Unsafe`. | ``composition-conflict: components of row `unit#0` under one provenance (analyzer `qcore-audit@0.1.0:unrecorded-build`) differ; all are kept and none is chosen`` |
| `contract-conflict` | Analyzer contract selection | `Unknown` *(not human-resolvable)* | Multiple contract bundles declare a called function and their supersession pin stamps (§C06.6) fail to select a unique winner — equal maximal stamps on differing texts, missing/malformed/stale stamps, or unpinned declarations (`CONTRACT-SUPERSESSION`). Every candidate is retained; never contributes to provisional `Unsafe`. | ``call to `k_close` whose contract bundles conflict: their pin stamps do not decide which declaration is in force (contract-conflict)`` |

#### Human-Resolvable Subset (`ReasonCategory::is_human_resolvable`)

Exactly **four** categories represent holds resolvable by a single named human act:
1. `awaiting-human-review`
2. `contract-pin-mismatch`
3. `plugin-claim-uncertified`
4. `outer-contract`

A row withheld solely for reasons in this four-element subset evaluates to `Pending` (§L21.2); any row carrying even one reason from the remaining nine categories joins to `Unknown` (or `Unsafe` on a validated violation). Notice that `stale-evidence` (`contract-refuted-by-body`), `composition-conflict`, and `contract-conflict` are deliberately **outside** the human-resolvable subset and always produce at least `Unknown`.

#### Binding Implementation and Digest-Stability Invariants

1. **No relabeling of `environment-unavailable` as a source defect:** Missing Clang binaries, absent build trees, mismatched `compile_commands.json` entries, or missing submodules MUST classify exclusively as `environment-unavailable` — never folded into `unsupported-syntax` or `residual-custody`, and never allowed to contribute to a provisional `Unsafe`.
2. **Additive refinement of the verdict contract:** Reason categories accompany and classify the existing human-readable `reason` text; adding or refining `reason_code` MUST NOT alter source spans, `property`, semantic profile, scope, bounds, assumptions, or obligation IDs. Audit-report units and source-contract functions carry `reason_code` failing closed to `null` rather than guessing an unverified default.
3. **Canonical `VerificationTask` digest lockstep ([diagnostics constraint 3](agents/diagnostics.md#constraint-3)):** `DerivationIncomplete` is embedded inside `VerificationTask`, which uses `#[serde(deny_unknown_fields)]` and whose canonical-JSON SHA-256 `task_digest` is bound into every evidence-v2 request, provider package, and recorded semantic witness. While `DerivationIncomplete` carries `category` at construction in current schemas, any wire-serialized schema modification to `VerificationTask` alters every `task_digest` and therefore MUST regenerate all shipped provider packages and witnesses in the same atomic change; likewise, the four historical pre-`REASON-CODES` 185-unit corpus snapshots remain frozen byte-for-byte until an explicit `CORPUS-REPIN`.

> **Implementation & Provenance.** `crates/qcore-schemas/src/{reason,obligation}.rs`,
> `crates/qcore-verification/src/audit.rs`; cards `REASON-CODES`, `W45-CONTRACT-DEFERRED-REVIEW`,
> `W45-CONTRACT-REVISION-MODE`, `W45-REFUTED-CONTRACT-WITHDRAWAL`, `W45-USER-CONTRACT-PLUGINS`,
> `OUTER-CONTRACT`, `AUDIT-COMPOSITION`, `CONTRACT-SUPERSESSION`.

---

<a id="l21-diagnostics-and-compatibility"></a>

## L21. Diagnostics and compatibility

### 1. Pass-Ownership Allocation Principles, `Q-NAMESPACE-DECISION`, and Wire-Version Stability

The Quirin diagnostic architecture enforces strict, pass-scoped ownership established by the W3/W7 diagnostic-attribution migration ([diagnostics constraint 1](agents/diagnostics.md#constraint-1); [attribution report](reports/w7-diagnostic-attribution/README.md)):

* **Ownership by semantic pass, never by crate:** Every `Q` diagnostic range belongs to one **semantic pass**, mapped authoritatively by `qcore_frontend::frontend::source_diagnostic`. Any workspace crate that executes a semantic pass (`qcore-core`, `qcore-frontend`, `qcore-verification`) emits from that pass's owned codes, and **no code may ever be reused by a second semantic pass** — even when two passes invoke the identical underlying checker (for example, `Ownership` owns `Q103`, whereas `StandardBuffer`, `StandardString`, and `StandardSocket` inherit `qcore_core::ownership` under their own `Q054`, `Q078`, and `Q083`).
* **Condition stability vs. span/message attribution:** Within a pass's range, each code identifies one specific semantic refusal condition. Changing *which* inputs a code rejects or renumbering a shipped code is a semantic change governed by §L21; refining *where* an existing condition is reported (source file, line, byte span) or naming a more specific boundary token in its message without altering the refused program set is an attribution improvement, not a code change.
* **Non-contiguous multi-band passes and integration relocation:** Shipped diagnostic numbers are **never** renumbered merely for visual contiguity. When a pass grows across later waves after sibling passes have claimed intervening ranges, the pass owns multiple explicit bands (such as `NativeLibrary`'s four ranges `Q039`–`Q042`, `Q047`–`Q049`, `Q063`–`Q064`, and `Q065`–`Q067`, `Event`'s `Q058`–`Q062` and `Q074`, and `StandardString`'s `Q068`–`Q073` and `Q075`–`Q079`). Under the wave-3 `T07`/`T09` precedent, if two concurrent feature branches collide on an unshipped range at merge time, only the still-unshipped band relocates while preserving its internal code-to-condition order; unused pre-assigned pool codes (`Q077`–`Q079` from `STRING-SLICE-IO`, `Q097`–`Q099` from `HTTP-QUIRIN-V1`) were returned to the allocator before shipping rather than repurposed post-ship.
* **Route-code exhaustion and the `Q-NAMESPACE-DECISION` icebox:** All two-digit route bands `Q000`–`Q099` are allocated (`Q077`–`Q079` and `Q097`–`Q099` spent the final pools). Opening a new route-code range above `Q099` or under a new prefix was placed in the **`Q-NAMESPACE-DECISION` icebox** on 2026-09-12 (and re-affirmed across subsequent waves) to prevent ad-hoc numbering collisions. Under that operative rule:
  * New `.qr` language increments allocate **zero new `Q` codes**; instead, each new refusal reaches an existing pass-owned condition by a new route (**condition widening**) or lifts an earlier boundary (**deliberate condition narrowing**), documented explicitly in the owning pass's reference entry below.
  * Subsystem-owned non-route namespaces — `QP***` (verifier host/firewall), `QI***` (`.qv` contract inclusion loader), and `QF***` (symbolic premise/hypothesis format readers) — operate outside route `Q` bands on the `QP***` precedent without resolving `Q-NAMESPACE-DECISION`.
* **Superseded revision-0.2 `Q700`–`Q712` Libevent table:** The Libevent-specific `Q700`–`Q712` table in revision 0.2 was a superseded design draft, never a second active owner of `Q7xx`. Core `Q700`–`Q703` belong exclusively to `PluginVerification` (`verify_dispatch.rs`), while Libevent reactor diagnostics use provider-qualified codes `qcore.libevent.L00`–`L12` (or carry `L00`–`L12` inside `Event` `Q060` / `StandardSocket` `Q083`). Both plugin response versions reject empty, nested (multi-dot), or non-ASCII local components with `QP007` prior to verdict evaluation, and provider diagnostic prose is always treated as untrusted text.
* **Wire, manifest, and package version stability:** Updating this prose specification does **not** alter `PLUGIN_PROTOCOL_VERSION = 1`, `PLUGIN_EVIDENCE_VERSION = 1` (with `PLUGIN_EVIDENCE_VERSION_V2 = 2` negotiated explicitly), the resource-contract manifest `schema_version = 1`, the `quirin` / `qcorec` package version, or any stored observation schema. Any wire-incompatible change requires an explicit protocol or schema version bump.

---

### 2. Master Pass Ownership Summary Table

| Code Band(s) | Owning Semantic Pass | Emitting Crate(s) | Primary Responsibility |
|---|---|---|---|
| `Q000`, `Q010`–`Q015` | `NameResolution` | `qcore-frontend`, `qcore-verification` | Per-file structural parsing (`Q000`), duplicate bindings/declarations (`Q010`–`Q012`), lexical reference resolution (`Q013`), `#import` & runtime module bundling (`Q014`), and file-local capability import gates (`Q015`). |
| `Q001`, `Q016` | `Index` | `qcore-frontend` | Generic quantitative index/`quant` presentation (`Q001`), unconditional nominal dimension/axis validity, and quant-index type resolution (`Q016`). |
| `Q020`, `Q021` | `OrdinaryTypes` | `qcore-frontend`, `qcore-verification::driver` | Ordinary expression/type/annotation/effect admission, scalar/float/record/helper/task-parameter typing, task source-contract gate (`Q020`), and sequential nonempty source-contract gate (`Q021`). |
| `Q022`–`Q027` | `ExternCall` | `qcore-frontend`, `qcore-verification::driver` | Admission, §C04 mode bridging, mandatory acquisition adapters, LIFO error-arm cleanup, scalar/outcome bindings, and certificate recheck for `source.extern-call.v1` (and `source.ffi.v1` calls). |
| `Q028`–`Q031` | `StandardIo` | `qcore-frontend`, `qcore-verification::driver` | Admission, operand/effect rules, flat entry shape, and certificate recheck for `std.io`, `std.logger`, and `std.fmt` (`source.std-io.v1`). |
| `Q032`–`Q035` | `UncheckedLinkage` | `qcore-frontend`, `qcore-verification::driver` | File-local opt-in, closed scalar ABI surface, contract coexistence exclusion, and certificate waiver/unsafe-FFI premise tripwires for `unsafe extern fn` (§L14.1) and `source.ffi.v1` (§L14.6). |
| `Q036`–`Q038` | `Pipe` | `qcore-frontend`, `qcore-verification::driver` | Parse-time `\|>` pipeline desugaring (`Q036`), stage value-flow arity/terminality (`Q037`), and file-wide positional gate for `\|`, `&`, `^`, `\|\|`, `&&`, and closure bars (`Q038`). |
| `Q039`–`Q042`, `Q047`–`Q049`, `Q063`–`Q067` | `NativeLibrary` | `qcore-frontend`, `qcore-verification::driver` | `.qrlib` v1/v2 manifest loading & waivers (`Q039`–`Q042`), bidirectional contract slices & withdrawal (`Q047`–`Q049`), verified `.h`/`include` & Clang FFI struct layouts (`Q063`–`Q064`), and `extern library` / `linked:NAME` bindings (`Q065`–`Q067`). |
| `Q043` | `ComposedStdIo` | `qcore-frontend`, `qcore-verification::driver` | Checked interleaving join of `source.extern-call.v1` × `source.std-io.v1` (`source.std-io-extern.v1`), `succeeds`-determined scalar rendering, and live `borrow NAME` custody-ID rendering. |
| `Q044`–`Q046` | `Branch` | `qcore-frontend`, `qcore-verification::driver` | `if`/`else` arm subset & continuation hoisting (`Q044`), C-compatible two-sorted decidable condition tier (`Q045`), and exact custody agreement across surviving arms (`Q046`). |
| `Q050` | `ComposedTasks` | `qcore-frontend`, `qcore-verification::driver` | Checked join of scoped tasks × `source.std-io.v1` (`source.std-io-tasks.v1`), worker `io` effects, task-ledger scalar liveness, and `qcore.obs-reordering.v1` windowed emitter comparison. |
| `Q051`–`Q057` | `StandardBuffer` | `qcore-frontend`, `qcore-verification::driver` | Owned extent-carrying byte buffers (`source.buffer.v1`), scalar/nested/owning records (`source.record.v1`), range write lends (`LendRange`), copy binds, `source.resource-calls.v1` helpers, and inherited `ownership` custody (`Q054`). |
| `Q058`–`Q062`, `Q074` | `Event` | `qcore-frontend`, `qcore-verification::driver` | Single-threaded event reactor (`source.event.v1`), inherited `L00`–`L12` descriptor custody (`Q060`), `wake`/`dispatch` determinism (`Q061`), `log_level` configuration (`Q058`), and composed event lines + drained bytes (`Q074`). |
| `Q068`–`Q073`, `Q075`–`Q079` | `StandardString` | `qcore-frontend`, `qcore-verification::driver` | Immutable UTF-8 `{ptr, len}` string views (`Q068`–`Q073`), length-bounded `fwrite` text output (`Q075`–`Q076`), static `concat`, range-wise `borrow_utf8`, loop windows, and static UTF-8 buffer-history proof (`Q077`–`Q079`). |
| `Q080`–`Q089` | `StandardSocket` | `qcore-frontend`, `qcore-verification::driver` | Checked loopback TCP/UDP state machine, inherited custody & reactor loans (`Q083`), port/payload ranges (`Q084`), history-satisfied `recv`/`accept` determinism (`Q085`), declared IPv4/IPv6 addresses, and `disjoint` protocol-cell setup. |
| `Q090`–`Q096` | `StandardHttp` | `qcore-frontend`, `qcore-verification::driver` | Checked HTTP/1.1 start-line (`Q091`), CRLF-injection-free header fields (`Q092`), statically decided `Content-Length` framing (`Q093`), computed message components (`Q090`/`Q094`), and inherited `StandardSocket` transport/cell checks (`Q095`). |
| `Q097`–`Q099` | `Loop` | `qcore-frontend`, `qcore-core`, `qcore-verification::driver` | Counted `for NAME in 0..BOUND` shape & `[| #unroll, where #bound < N |]` attribute syntax (`Q097`), three-layer iteration/event unroll budget (`Q098`), and per-iteration custody identity (`Q099`). |
| `Q100`, `Q103` | `Usage`, `Ownership` | `qcore-frontend` | Quantitative usage-grade intervals (`Q100`) and core sequential resource ownership/borrowing (`Q103`). |
| `Q200`, `Q202` | `ResourceFlow`, `CleanupLowering` | `qcore-frontend`, `qcore-verification::driver` | Four-component `(Δ, R, P, T)` resource-flow algebra (`Q200`) and sequential cleanup lowering / reference interpretation (`Q202`). |
| `Q300`, `Q302`–`Q309` | `TypedErrors`, `ErrorRegion` | `qcore-frontend`, `qcore-verification::driver` | Typed error/effect row checks (`Q300`) and `error region` / `quant open` linear cleanup-pack discipline across sequential and composed task routes (`Q302`–`Q309`). |
| `Q400`, `Q401`, `Q403` | Driver Invocation | `qcore-verification::driver` | Empty input list (`Q400`), missing/ambiguous entry point (`Q401`), and `--entry` against library/contract modules or conflicting `EXT-01` suffix/mode selections (`Q403`). |
| `Q500` | `CEmission` | `qcore-frontend`, `qcore-verification::driver` | Constructs outside the reduced C11 CFG or task C backend encoding capabilities (and fail-closed capture backstop). |
| `Q600`, `Q610`–`Q627`, `Q630`–`Q641`, `Q650`–`Q667`, `Q670`–`Q677` | `Concurrency` | `qcore-frontend`, `qcore-verification::driver` | Scoped tasks, transferability, leases, records & closures (`Q600`); `CheckedMutexInvariantV1`, lock ranks & growth (`Q610`–`Q627`); `FlatEpochBoundV1` & error-credit recovery (`Q630`–`Q641`); `DisjointFootprintsV1` & `Q664` protocol cells (`Q650`–`Q667`); `CheckedRegionTaskCreditV1` (`Q670`–`Q677`). |
| `Q700`–`Q703` | `PluginVerification` | `qcore-verification` | Source `verify scope` resolution (`Q700`), undischarged core obligations (`Q701`), conflicting provider evidence/premises (`Q702`), and unvalidatable provider allegations (`Q703`). |
| `QP001`–`QP016` | Provider Host | `qcore-verifier-host`, `qcore-verification` | Plugin protocol framing, process confinement/quotas, request-consumption checks, certificate firewall, and multi-certificate composition. |
| `QI001`–`QI020` | Contract Inclusion | `qcore-contract` | `#include` and `contract inherit` resolution, provider load-path manifests, signature/capture compatibility, and inclusion-evidence replay (§C15). |
| `QF001`–`QF005` | Premise Formula | `qcore-frontend`, `qcore-contract` | Retirement bans on English prose strings in `resource_premise`, `resource_hypothesis`, guarded decompositions, `#outer` contracts, and JSON formal-binding manifests (§C04.18, §C04.20). |
| `<plugin-id>.<local-code>` | External Providers | Registered providers | Provider-owned diagnostics validated at `QP007` (exact `plugin_id` + single 1–128-byte ASCII `[A-Za-z0-9_-]` code, e.g., `qcore.libevent.L00`–`L12`). |

---

### 3. Structured Per-Pass Diagnostic Reference

#### 3.1. `NameResolution` (`Q000`, `Q010`–`Q015`)

Emitted per input file and across linked source bundles by `qcore-frontend` and the runtime bundle resolver in `qcore-verification`:

* **`Q000` — Per-file structural parse refusal:**
  * Input file fails to parse as a structural QCore declaration — including malformed declaration or `#import` syntax, omitting the final `;` of a block's last statement, unbalanced `(` / `)`, or an unterminated `[| … |]` clause list (`LOOP-UNROLL-BUDGET`).
  * Standalone frontend invocation receives an unresolved relative runtime `#import` without bundle resolution.
  * **Spawn-closure syntax shapes (`W48-QR-CLOSURES`):** A closure capture list lexically adjacent to `[` or `]`, a missing `-> TYPE` return annotation, a missing `{ … }` body, an empty capture list written with interior whitespace `| |` rather than `||`, or passing a closure as an argument to a worker call.
  * **Compiler-route wrapper for contract-format/inclusion readers:** Unresolved `#include` / `contract inherit` syntax reaching the structural parser (`CONTRACT-INHERIT`), and symbolic-premise reader rejections `QF001`–`QF004` (`SP5`, `SP6`, `SP8`), surface on the compiler route as `Q000` carrying the underlying `QF00*` code prefix in their message.
* **`Q010` — Duplicate top-level declaration:** Two declarations in the module/bundle bind the same global identifier.
* **`Q011` — Duplicate parameter or closure capture binder:**
  * Function header declares two parameters with the same name.
  * **Spawn closures (`W48-QR-CLOSURES`):** A closure capture list captures the same outer place twice, or binds two captures to the same parameter name.
* **`Q012` — Duplicate local variable binding:** A function body introduces a second `let` (or local binder) with a name already bound in scope. *(Reachability note: Because `Q012` runs ahead of route lowerings, route-local rebinding/collision guards in `ExternCall` `Q026`, `Event` `Q074`, and `StandardHttp` `Q090` are unreachable from single-module source programs and retained strictly as fail-closed model backstops.)*
* **`Q013` — Unknown referenced name or reserved-identifier misuse:**
  * Identifier in a value, type, callee, error, or clause reference position has no visible declaration — including unreserved words written in `std.fmt` rendering or `std.logger` level positions (`STD-FMT-DIAGNOSTIC`), unbound names inside `if` conditions (`BITWISE-CONDITIONS`) or string/socket/HTTP operands, undeclared record type names in `let p = T { … };` (`AGGREGATES-V1`), and unbound place or mutex names in `share PLACE under MUTEX` (`MUTEX-SHARE-CROSS`).
  * **Explicit-only closure capture (`W48-QR-CLOSURES`):** Referencing an outer local variable inside a spawn closure's body without listing it in `|CAPTURES|`.
  * **Reserved built-in error name (`W48-QR-SCALAR-TYPES-R4`):** Declaring a user error type named `ArithmeticError`.
  * *Contextual non-reference skips:* `frontend::lexical_references` deliberately skips contextual clause words only in their exact grammatical slots — `len(`, `concat(`, `borrow_utf8(`, `lend(` and `lend<Buffer>`, `for … in`, record field labels `FIELD:` and `.FIELD`, `: string` type ascriptions, and fixed-width type names inside conversions (`W48-QR-SCALAR-TYPES`, allowing non-scalar routes to reject misplaced conversions at their own route codes rather than `Q013`) — so none of those words is globally reserved against user variables or `.qv` contract parameters.
* **`Q014` — Unknown imported module or refused runtime-source bundle:** Unknown `std.*` capability name, or a relative runtime `#import` refused for a missing/unreadable target file, unsupported path grammar, ambiguous file/alias identity, import cycle, incompatible module dependency mode, or exhausted bundle-size bound.
* **`Q015` — Capability used without its file-local `#import`:**
  * Concurrency construct used in a file lacking its concurrency import; `print` without `#import std.io;` (including on `source.scalar-calls.v1`); `log`, `log_level`, or `flush` without `#import std.logger;`; scalar formatting selector (`dec`, `hex`) without `#import std.fmt;`; or constructs of `std.event`, `std.buffer`, `std.string`, `std.socket`, or `std.http` without their respective `#import` (the diagnostic always names the missing module).
  * *Formatting-gate precision (`STRING-SLICE-IO`, `HTTP-V1-COMPUTED-LENGTH`):* Plain UTF-8 text output (`print s;`, `log info "ok";`) names no formatting selector and requires `std.string` + `std.io`/`std.logger` **without** `std.fmt`. On `source.http.v1`, a `let`-bound `concat(…)` constructs a `string` value and requires `std.string`, whereas an inline message-component `dec(NAME)` constructs no `string` and requires neither `std.string` nor `std.fmt`.

*Provenance:* Core W3/W7 map; cards `STD-MODULES`, `STRING-SLICE`, `STRING-SLICE-IO`, `STRING-CONCAT`, `LOOP-V1`, `LOOP-UNROLL-BUDGET`, `AGGREGATES-V1`, `MUTEX-SHARE-CROSS`, `HTTP-V1-COMPUTED-LENGTH`, `W48-QR-SCALAR-TYPES`, `W48-QR-SCALAR-TYPES-R4`, `W48-QR-CLOSURES`, `CONTRACT-INHERIT`, `SP5`–`SP8`.

---

#### 3.2. `Index` (`Q001`, `Q016`)

Emitted by `qcore-frontend` over quantitative index and dimension declarations (§L06):

* **`Q001` — Generic index / `quant` presentation refusal:** Malformed or unsupported quantitative index clause outside `Q016`'s declaration/resolution checks.
* **`Q016` — Nominal dimension/axis validity and quant-index type resolution:** Invalid `dimension` or `axis` declaration — checked **unconditionally** on every module regardless of whether any function body instantiates the axis (revision-0.3 conformance item `R05`) — or ill-typed `quant[| … |]` index expression/constraint.

*Provenance:* Core W3/W7 map; revision-0.3 `R05` unconditional axis repair.

---

#### 3.3. `OrdinaryTypes` (`Q020`, `Q021`)

Emitted by `qcore-frontend` and `qcore-verification::driver`:

> **Intentional route-specific split between `Q020` and `Q021` ([diagnostics constraint 2](agents/diagnostics.md#constraint-2)).** Both `Q020` (on the scoped-task route) and `Q021` (on the sequential whole-program route) refuse unenforced inline source `contract { … }` blocks, and they MUST NOT be unified until a real source-contract consumer lands: the task gate (`Q020`) rejects *every* source contract **including empty `contract { }`** (as does `source.resource-calls.v1`), whereas the sequential gate (`Q021`) and `source.scalar-calls.v1` accept empty `contract { }` and reject only nonempty `contract { … }`. Collapsing them would silently alter rejection coverage. `ExternCall` (`source.extern-call.v1`) consumes explicitly selected `.qv` contract files rather than inline source `contract { … }` blocks and is therefore separate from both.

* **`Q020` — Unsupported or ill-typed ordinary construct, annotation, scalar/float/record operation, or task signature:**
  * **Base ordinary & declaration rules:** Unsupported ordinary AST forms; invalid entry-function parameter signatures; `effects { … }` rows naming effects unmodeled on the active profile (e.g., `io` on an uncomposed task module); task-route inline source `contract { … }` blocks (empty or nonempty); and any `extern fn` declaration in an executable or library module whose parameter or return type falls outside the §C04.1 ABI table (`W48-QR-SCALAR-TYPES-R3`, widened to include `f32` and `f64` by `W48-QR-FLOATING-POINT` — affecting message text on four pinned survey/probe files at identical code, exit, and span).
  * **Local type annotations (`SOURCE-ANNOTATION-ENFORCEMENT`):** Incompatible explicit local type annotations on executable routes, unsupported explicit local type spellings, malformed local binder heads before `=`, and any explicit result annotation (including `own<T>`) on `qcore_acquire` (which establishes untyped custody without a nominal payload type; legacy sequential integer literal bindings admit `: i32`, while local usage-grade annotations stay at `Q100`).
  * **Scalar calls, fixed-width types, `ArithmeticError`, and IEEE floats (`SOURCE-SCALAR-CALLS-V1`, `W48-QR-SCALAR-TYPES`, `-R2`, `W48-QR-FLOATING-POINT`):**
    * Scalar-call arity/type mismatches, unsupported statements, recursive call cycles, AST/step bounds, missing `std.io` or undeclared `Io` effect on `source.scalar-calls.v1`, legacy `print(…)` builtin mixed with fixed-width forms, non-`i32`/`unit` entry return types, and sequential parameter annotations outside §G02t.
    * Unknown type names; integer or float literals out of range (or float literals underflowing to zero); integer literals at float types or vice versa; mixed-type binary operands (including mixing `f32` with `f64`); `bool` arithmetic, ordering, conversion, or `hex` rendering; `%` on floats; failing value conversions; check-time or closed-entry arithmetic faults; `isize`/`usize` without a recorded target; and missing, unpropagated, or duplicate `partial_eq` / `partial_ord` float-comparison effects.
    * `ArithmeticError` typed-error discipline (`-R2`): unconsumed call to a fallible function, `?` or `raise` without `throws<ArithmeticError>`, `?` or `match` on an infallible callee, `match` lacking exact `Ok` + `Err` arms or applied to a `unit` callee, error rows naming anything other than `ArithmeticError`, and unknown variants.
  * **Scalar-call records (`W48-QR-USABLE-RECORDS`, §G02s):** Record parameters passed other than by value; records in arithmetic, comparisons, conversions, or output lines; non-total record constructions; undeclared field paths; field writes; records declaring owned fields; and fallible functions returning a record.
  * **Bounded resource helper functions (`RESOURCE-HELPER-FUNCTIONS`, `source.resource-calls.v1`):** Unsupported helper shapes, incomplete header/type annotations, exceeded finite call-expansion bounds, and empty inline `contract { }` blocks.
  * **Scoped-task worker parameters, write leases, typed `Buffer<T>` payloads, and task records:**
    * Contextual `lend<Buffer>` / `lend<Buffer<T>>` worker parameter agreement with `lend PLACE LOW..HIGH` spawn captures (`WRITE-LEND-TASKS`), enforcing **at most one** `lend<Buffer>` parameter per worker (`WRITE-LEND-TASKS-EXERCISE`).
    * Arity-selected `qcore_buffer_write(LEASE, INDEX, VALUE)` and `qcore_buffer_read(NAME, INDEX)` binding-kind checks: writing a byte through anything other than a write lease, or reading a single byte through a counted `borrow<Buffer>` read share (`WRITE-LEND-TASKS-EXERCISE`).
    * Scoped-task fixed-width and float typing (`W48-QR-SCALAR-TYPES-R4`, `W48-QR-FLOATING-POINT`): literal ranges, mixed types, `bool` restrictions, check-time faults, missing recorded target for `isize`/`usize`, missing worker/entry float-comparison effects, and fixed-width task entry return types.
    * Typed `own<Buffer<T>>` / `borrow<Buffer<T>>` / `lend<Buffer<T>>` payload validation (`W48-QR-SCALAR-TYPES-R6`): malformed payload types, `isize`/`usize` payloads without a recorded target, and payload-type mismatches across local annotations or spawn captures.
    * Scoped-task records (`W48-QR-TASK-RECORD-CAPTURES`): unsupported field types in task-route record declarations, wrong-type field values or field writes, undeclared field paths, and invalid record parameter spellings.
* **`Q021` — Unenforced nonempty source `contract { … }` gate:** Refuses any nonempty inline source `contract { … }` block on the base sequential whole-program route, `source.scalar-calls.v1`, and `source.resource-calls.v1`.

*Provenance:* Core W3/W7 map (`remaining-d01`); cards `WRITE-LEND-TASKS`, `WRITE-LEND-TASKS-EXERCISE`, `SOURCE-SCALAR-CALLS-V1`, `SOURCE-ANNOTATION-ENFORCEMENT`, `RESOURCE-HELPER-FUNCTIONS`, `W48-QR-SCALAR-TYPES` (`R1`–`R4`, `R6`), `W48-QR-FLOATING-POINT`, `W48-QR-USABLE-RECORDS`, `W48-QR-TASK-RECORD-CAPTURES`.

---

#### 3.4. `ExternCall` (`Q022`–`Q027`)

Emitted by `qcore-frontend` and `qcore-verification::driver` for calls to declared external C functions on `source.extern-call.v1` (§L14.E.1) and `source.ffi.v1` (§L14.6):

* **`Q022` — Missing file-local `#import std.extern;`:** An extern call appears in a source file that does not itself write `#import std.extern;`, including when the `extern fn` declaration was supplied by another file in a multi-file bundle.
* **`Q023` — Missing admitted declared contract:** The called extern function has no admitted `.qv` contract in the explicit `CompileOptions` selection — no selection supplied, selection failed to load, symbol absent, or contract withdrawn for the unit by `NativeLibrary` `Q048`.
* **`Q024` — Closed §C04 bridging or value-namespace refusal:**
  * Contract mode, ABI type, argument spelling, or argument count falls outside the closed v1 bridging table — including `resource_stored` naming the same identity as its declared `owner`, or an error-arm cleanup disposition with invalid arity or argument form.
  * **Value vs. custody separation (`PIPE-SCALAR-RESULT`):** Binding a call result whose declared return type is not a custody-free scalar (`unit`, or *any* pointer — including `ptr<…, u8>` declared `resource_return_none()`), passing a resource binding at a declared scalar parameter, or referencing an unbound scalar.
  * **Fixed-width and IEEE float value checks (`W48-QR-SCALAR-TYPES-R3`, `W48-QR-FLOATING-POINT`):** Mixed operand types; `bool` arithmetic, ordering, or conversion; integer/float literal mismatch; out-of-range parameter or `succeeds` outcome literals (or a float outcome literal rounding to `±∞`); `isize`/`usize` without a recorded target; check-time arithmetic faults; runtime-faulting operations without `throws<ArithmeticError>`; scalar argument type mismatch; and `source.ffi.v1` types/arguments outside the FFI bridging table.
* **`Q025` — Mandatory acquisition adapter, LIFO error-arm cleanup, and `let` / `succeeds` forms:**
  * **Mandatory adapter (`EXTERN-MODES`, `PIPE-SCALAR-RESULT`):** Required for all three nullable fresh-custody sources — `resource_return_owned(nullable)`, `resource_out(_, nullable)`, and `resource_return_reallocated(_, nullable)`. Refuses if the adapter is omitted, if an acquiring callee's result is written as a plain scalar `let`, if the adapter wraps a non-acquiring callee, or if it names an undeclared error type.
  * **Simultaneous acquire + move-out exclusion:** Refuses any acquiring call that also moves custody out, *except* the `resource_return_reallocated` input subject, whose failure-arm preservation is explicitly stated by contract specification §C04.3 and which therefore remains live on the error arm and must be cleaned up there.
  * **Explicit error-arm cleanup dispositions (`EXTERN-ERR-REL`):** Refuses if (1) any resource live at the raise is left unreleased (abandonment), (2) a disposition itself acquires or stores custody, (3) a disposition discharges anything other than **exactly one** live identity on the arm, (4) dispositions are written in any order other than **strict reverse acquisition order (LIFO)**, or (5) a disposition's derived effect row is not admitted by the entry's declared `effects { … }` row.
  * **Local `let` and `succeeds` outcome forms (`EXTERN-SCALAR-LITERAL`, `STD-IO-COMPOSE`, `W48-QR-FLOATING-POINT`, `W48-QR-USABLE-RECORDS`):** Refuses any `let` that is neither an acquisition adapter, a custody-free scalar/record call binding (whose optional `succeeds LITERAL` tail causes emitted C to compare the runtime return value and exit `70` on mismatch; integer vs. float literal kind must match), nor a literal/value binding (`let NAME = LITERAL;` derives zero effects and opens zero obligations). On `source.ffi.v1`, also refuses writing `succeeds` on a `.qr` helper call or returning a mismatched type from a `.qr` helper.
* **`Q026` — Unsupported call context, `.qr` FFI helper limits, or local-kind collision backstop:**
  * Extern call written inside a nested block, a non-entry function, or a parameterized entry; or an unmodeled declaration/statement on `source.ffi.v1`.
  * **`.qr` FFI helper rules (`W48-QR-USABLE-RECORDS`):** Uncalled helper function, recursive helper call cycle, call chain deeper than 16, or helper parameter other than a scalar or record passed by value.
  * *Fail-closed backstop (`PIPE-SCALAR-RESULT`):* Local `let` shadowing a live resource binding with a scalar or vice versa (unreachable from source because `NameResolution` `Q012` rejects duplicate local bindings first).
* **`Q027` — Disagreement between declared input and checked artifact:** Source `extern fn` signature disagrees with its selected `.qv` contract, called `extern fn` carries an inline `contract { … }` block, contract selection no longer reproduces its source digests, or published certificate fails independent rederivation.

*Provenance:* Cards `EXTERN-CALL-V1`, `EXTERN-F1`, `EXTERN-MODES`, `EXTERN-ERR-REL`, `EXTERN-SCALAR-LITERAL`, `PIPE-SCALAR-RESULT`, `STD-IO-COMPOSE`, `W48-QR-SCALAR-TYPES-R3`, `W48-QR-FLOATING-POINT`, `W48-QR-C-STRUCT-IMPORT`, `W48-QR-USABLE-RECORDS`.

---

#### 3.5. `StandardIo` (`Q028`–`Q031`), `ComposedStdIo` (`Q043`), and `ComposedTasks` (`Q050`)

##### `StandardIo` (`Q028`–`Q031`, `source.std-io.v1`)

Emitted by `qcore-frontend` and `qcore-verification::driver` for deterministic line output (`std.io`, `std.logger`, `std.fmt`):

* **`Q028` — Invalid bare or borrowed rendering operand:**
  * Bare rendering operand is not an integer binding of the entry — structurally preventing any resource from being rendered bare.
  * **Borrowed custody rendering without a custody model (`STD-FMT-BORROW`):** `borrow NAME` operand where no checked custody model published a live identity — which on standalone `source.std-io.v1` and `source.scalar-calls.v1` refuses *every* `borrow NAME` because their custody identity environments are empty (value and custody lookups are strictly disjoint; `borrow` reuses the existing mutex-guard keyword).
* **`Q029` — Undeclared derived effect:** Entry performs `Io` (or, on `source.ffi.v1` under `W48-QR-USABLE-RECORDS`, field mutation `Mut` or a `.qr` helper's effects) omitted from its declared `effects { … }` row.
* **`Q030` — Flat entry shape or statement-form violation:** Unmodeled statement, non-entry function (`W48-QR-SCALAR-TYPES-R2`), nested block, parameterized entry, `print`/`log`/`flush` outside the entry, unknown *reserved* rendering/level word (unreserved words fail earlier at `Q013`), `flush` given an operand, missing or duplicate `return`, malformed `effects` row (including `partial_eq`/`partial_ord` on standalone `source.std-io.v1`), or typed operands outside `source.std-io.v1`'s scalar renderer (`W48-QR-SCALAR-TYPES-R3`). *(On `source.scalar-calls.v1`, `std.io` line shape errors likewise report `Q030`.)*
* **`Q031` — Certificate rederivation failure:** Published `source.std-io.v1` certificate disagrees with independent rederivation.

##### `ComposedStdIo` (`Q043`, `source.std-io-extern.v1`)

Single join code for the checked composition of `source.extern-call.v1` × `source.std-io.v1` (selected **after** all concurrent routes and **before** standalone extern and standard-I/O routes; sub-projections run unedited and keep their own `Q022`–`Q027` and `Q028`–`Q031` codes):

* **`Q043` — Extern × Standard-I/O join or composite recheck refusal:**
  1. **Order-preserving interleaving:** Every event of both projections must appear **exactly once** in its projection's order, terminating in the single shared `return;` (no dropped, duplicated, or reordered steps).
  2. **Source-determined scalar operands:** A bare rendering operand resolves in the extern model's **value namespace**, must be bound by an earlier step at an admitted ABI width (requiring a recorded target for `isize`/`usize`, `W48-QR-SCALAR-TYPES-R3`), and must have a **source-determined value** — a literal binding or a call carrying `succeeds LITERAL`. Rendering an undetermined extern or `#c_ffi` call result refuses here (`W48-QR-C-STRUCT-IMPORT`). *(Bare resource names fail at `Q028`.)*
  3. **Effect-row union:** Entry's declared `effects { … }` row must admit the union of both projections' derived rows (`partial_eq`/`partial_ord` governed by the extern half).
  4. **Live borrowed custody-ID rendering (`STD-FMT-BORROW`):** `print/log … borrow NAME;` resolves `NAME` in the extern model's **custody namespace** and requires the extern certificate's `live_after` ledger to hold that exact identity live at the rendering step (unbound, null-arm, consumed, released, stored, or reallocated-away identities refuse). Renders **only the numeric model identity** (never object bytes, extent, or runtime pointer address), alters no custody state, binds no `ValueId`, and never constitutes address publication (standing constraint 10).
  5. **Composite recheck:** Rechecks all three certificates (extern, standard-I/O, and composed join).

##### `ComposedTasks` (`Q050`, `source.std-io-tasks.v1`)

Single join code for the checked composition of scoped tasks × `source.std-io.v1` (selected via `has_source_tasks ∧ has_source_std_io ∧ ¬has_source_extern`, **after** epoch/mutex routes and **before** the standalone task route; task errors keep `Q600` and line-shape errors keep `Q030`):

* **`Q050` — Task × Standard-I/O join, worker effect, operand, or reordering-recheck refusal:**
  1. **Placement and stream comparison (`OBS-REORDERING`):** Output lines may stand **only** in the entry body or in functions spawned as leaf workers (lines in unspawned functions refuse). Parent-only lines execute on context `0` (worker trace records buffer privately and replay inside parent `join`) and use strict **byte-for-byte** stream comparison. Placing any worker line stamps `admits_reordering` (`qcore.obs-reordering.v1`, §L15), comparing streams window-by-window across quiescent barriers and emitter-by-emitter per OS writer thread.
  2. **Per-function worker `io` effect (rule 1a):** Any worker body that writes or flushes MUST declare `io` in its own `effects { … }` row in addition to the entry's.
  3. **Intra-block ordering:** Lines occupy exact lowered block positions and execute in source order within each block.
  4. **Task-checked scalar operands:** Operands must be literals or task-bound scalars (copied parameters, `let` literals, `qcore_buffer_read` results, or `join` outcomes) verified readable on all paths by injecting a synthetic `BindScalar` copy at the line's position and re-running `check_task_module`. Refuses owned `Buffer`s, read loans, task handles, `borrow NAME` operands, `hex` of `bool` (`W48-QR-SCALAR-TYPES-R4`), and whole records, sub-records, or owned record fields (`W48-QR-TASK-RECORD-CAPTURES`).
  5. **Entry `Io` effect & composite recheck:** Enforces entry `Io` declaration and independent composed-certificate rederivation.

*Provenance:* Cards `STD-MODULES`, `STD-IO-COMPOSE`, `STD-FMT-BORROW`, `COMPOSE-CONCURRENCY`, `OBS-REORDERING`, `W48-QR-SCALAR-TYPES-R2`–`R4`, `W48-QR-FLOATING-POINT`, `W48-QR-C-STRUCT-IMPORT`, `W48-QR-USABLE-RECORDS`, `W48-QR-TASK-RECORD-CAPTURES`.

---

#### 3.6. `UncheckedLinkage` (`Q032`–`Q035`) and `NativeLibrary` (`Q039`–`Q042`, `Q047`–`Q049`, `Q063`–`Q067`)

##### `UncheckedLinkage` (`Q032`–`Q035`)

Governs explicit contract-free scalar waivers (`unsafe extern fn`, §L14.1) and file-wide C-struct FFI trust gates (`source.ffi.v1`, §L14.6):

* **`Q032` — Missing file-local unsafe-interop import:** `unsafe extern fn` declared in a file lacking `#import std.extern.unsafe;` (gated on the declaration itself), OR `struct!` / an FFI-bundle `.qr` helper function written in a file lacking `#import std.ffi.unsafe;` (`W48-QR-C-STRUCT-IMPORT`, `W48-QR-USABLE-RECORDS`).
* **`Q033` — Signature or FFI-marker boundary violation:**
  * `unsafe extern fn` parameter or return type outside the §C04.1 scalar ABI table — **every pointer spelling (`ptr<const, u8>` included) and any `unit` parameter is refused**, whereas IEEE `f32`/`f64` are admitted (`W48-QR-FLOATING-POINT` deliberate narrowing).
  * `[| quant #c_ffi |]` written in a runtime or library module, or passing an imported C struct across a contract lacking `[| quant #c_ffi |]` (`W48-QR-C-STRUCT-IMPORT`).
* **`Q034` — Coexistence of waiver with checkable contract or FFI mode:** Inline `contract { … }` on an `unsafe extern fn`, waived symbol name also declared in the selected `.qv` contract environment, or `unsafe extern fn` written inside a `std.ffi.unsafe` file.
* **`Q035` — Certificate waiver / `unsafe-ffi` premise tripwire:** Certificate omits (or downgrades from manifest-sharpened to bare) a required unchecked-linkage waiver premise, or an FFI certificate attempts to publish plain `Safe` without the `unsafe-ffi` assumption naming every imported struct and `#c_ffi` callee.

##### `NativeLibrary` (`Q039`–`Q042`, `Q047`–`Q049`, `Q063`–`Q064`, `Q065`–`Q067`)

Governs `.qrlib` v1/v2 native library manifests, contract slices, verified C headers, Clang FFI struct layouts, and `extern library` bindings across four historical allocation bands (§L14.2–§L14.8). Disagreements follow §L14.E's unified monotonicity law (*withdraw only where the weaker admissible reading refuses*):

* **Band 1 — Manifest loading and manifest-resident waivers (`Q039`–`Q042`, §L14.2, §L14.8):**
  * **`Q039` — Total-or-reject `.qrlib` manifest load failure:** Missing format marker; unterminated statement; unknown, duplicate, or missing field (including unpaired `header`/`sha256`); invalid field value or `sha256` hash; malformed `unchecked fn` (carrying `from MODULE` or declaring a pointer) or `contract fn` (missing `from MODULE`); invalid module reference; non-§C04.1 ABI type or `unit` parameter; symbol bound twice; empty entry list; `EXT-01` source/checkpoint suffix passed as a manifest or simultaneous legacy+current manifest aliases; or `.qrlib` v2 grammar violations (`W48-QRLIB-LINKAGE`: v2 statement in v1, `include` or `unchecked fn` in v2, missing/malformed `export`/`input`/`runtime`, `runtime` on a static manifest, or missing `runtime` on a dynamic manifest).
  * **`Q040` — Ambiguous multi-manifest symbol:** Two selected `.qrlib` manifests bind the same exported symbol.
  * **`Q041` — Manifest vs. declaration signature disagreement (outright refusal):** Waived `unsafe extern fn` disagrees in arity, parameter, or return type with its manifest's `unchecked fn` entry — or a `.qrlib` v2 generated entry disagrees in contract module or C signature with its FFI callee (`W48-QRLIB-LINKAGE`) — **refused outright**, never withdrawn to an unmanifested premise.
  * **`Q042` — Manifest waiver / contract collision:** Symbol bound as `unchecked fn` in a selected manifest is also present in the selected `.qv` contract environment.
* **Band 2 — Manifest contract slices (`Q047`–`Q049`, §L14.3):**
  * **`Q047` — Bidirectional contract-slice resolution failure:** `contract fn … from MODULE` symbol absent from the exact named `MODULE`, a declaration in a sliced `MODULE` left unbound by the manifest, or contract slice selected with no contract environment (reached unwidened by `.qrlib` v2). *(Writing `unsafe extern fn` for a slice-bound symbol needs no extra code here: `Q047` guarantees environment membership, which `Q034` already refuses.)*
  * **`Q048` — Contract vs. manifest ABI disagreement (per-unit withdrawal):** Declared `.qv` contract's §C04 ABI projection disagrees with the manifest's `contract fn` entry — **withdrawn for the analyzed unit** (`Q048` emitted, symbol omitted from admitted contract environment, and every call site refuses at `Q023`).
  * **`Q049` — Stacked slice-premise tripwire:** Certificate for a manifest-bound contracted call omits either its declared-contract premise or its manifest artifact/module residence premise.
* **Band 3 — Verified exported C headers and Clang FFI struct layouts (`Q063`–`Q064`, §L14.4, §L14.6, §L14.8):**
  * **`Q063` — Header, `include`-decoration, or FFI struct-layout load failure:**
    * *`.qrlib` header & `include` reader (`NATIVE-LIBRARY-HEADER`, `-DECORATION`, `W48-QRLIB-LINKAGE`):* Missing header or declared `include` text; measured `sha256` mismatch; unreached or undeclared extra `include` text; unsupported preprocessor directive or unbracketed conditional group; bound symbol missing at depth zero, inside `#if`, macro-defined, `static`, empty-`()` (C11 6.7.6.3p14), variadic, doubly declared with conflicting types, unreadable, or using a C type outside the manifest ABI vocabulary (e.g., `fnptr`, or `struct` under an extern-route bounded reader); or non-derivable macro decoration.
    * *FFI Clang struct reader (`W48-QR-C-STRUCT-IMPORT`):* Unreadable header, missing recorded target, running compiler different from recorded Clang, spelling other than `c"struct TAG"`, incomplete `struct TAG`, union, bit-field, flexible array member, anonymous member, layout attribute, or listed member with no §G02t type equivalent.
  * **`Q064` — Header or Clang-layout contradiction (outright refusal):** Manifest entry disagrees with exported `.h` text; FFI `struct!` field listing disagrees with recorded Clang layout (non-member, wrong type, or out of layout order); or `.qrlib` v2 header digest disagrees with the `struct!` header digest of a struct carried by the callee (`W48-QRLIB-LINKAGE`). *(No separate publication tripwire code is needed for `.h` digests because header identity sits inside the residence record already enforced by `Q049`.)*
* **Band 4 — Source-file `extern library` and `.qrlib` v2 linkage bindings (`Q065`–`Q067`, §L14.5, §L14.8):**
  * **`Q065` — `extern library` binding resolution failure:** Unknown library name, library name claimed by multiple selected manifests, duplicate `extern library NAME;` in one file, binding with zero manifests selected, or (`W48-QRLIB-LINKAGE`) an FFI file calling C that binds no library or resolves to a v1 manifest.
  * **`Q066` — Foreign-manifest or unbound-FFI symbol refusal (outright refusal):** File declares a symbol claimed by a selected manifest the file did **not** bind via `extern library` (checked before `Q042`), or (`W48-QRLIB-LINKAGE`) calls an FFI C function exported by no manifest the file bound.
  * **`Q067` — Source-binding / `linked:NAME` certificate tripwire:** Published certificate omits a required `extern library` source-binding premise or per-callee `linked:NAME` FFI linkage premise.

*Provenance:* Cards `UNSAFE-LINKAGE`, `UNSAFE-LINKAGE-ARTIFACT`, `NATIVE-LIBRARY-CONTRACT-SLICE`, `NATIVE-LIBRARY-HEADER`, `NATIVE-LIBRARY-HEADER-DECORATION`, `NATIVE-LIBRARY-SOURCE-BINDING`, `EXT-01`, `W48-QR-C-STRUCT-IMPORT`, `W48-QR-USABLE-RECORDS`, `W48-QR-FLOATING-POINT`, `W48-QRLIB-LINKAGE`.

---

#### 3.7. `Pipe` (`Q036`–`Q038`), `Branch` (`Q044`–`Q046`), and `Loop` (`Q097`–`Q099`)

##### `Pipe` (`Q036`–`Q038`, `source.pipe.v1`)

Runs at parse time **before every downstream semantic pass**: well-formed `n`-stage `|>` pipelines desugar into `n` ordinary statements connected by hygienic `qcore__pipe*` local bindings, with all routes reading the post-desugaring token stream (`PIPE-INTRINSIC-STAGE`):

* **`Q036` — Unsupported pipeline shape or hygiene collision:** Empty operand or stage; stage head neither a closed statement head nor a callee name; sink stage carrying an invalid selector count (widened to a selector range by `STRING-SLICE-IO` so text `print`/`log` desugars while missing `dec`/`hex` on scalar `source.std-io.v1` reports `Q030`); unbalanced call parentheses; malformed or valueless `let NAME =` binder; assignment pipeline (`PIPE-ASSIGN`); `let`-bound pipeline ending in a releasing `drop` stage; `move` written on the first stage or bare `move` over a non-binding; or user code writing the reserved `qcore__pipe` identifier prefix when temporaries are generated (hygiene enforced by refusal, never silent renaming).
* **`Q037` — Non-value-producing or zero-operand pipeline stage:**
  * `flush` used as a pipeline stage (takes zero operands).
  * Statement stage — `print`, `log`, or `drop` (`PIPE-DROP-STAGE`) — placed in a **non-final** pipeline position (binds no result to feed a successor). Call-shaped model intrinsics (`qcore_buffer_new`, `qcore_buffer_read`, `qcore_buffer_write`, `qcore_error_payload`, `qcore_acquire`, `qcore_check`) and extern/source calls are valid non-final stages by stage kind (`PIPE-SCALAR-RESULT`, `PIPE-INTRINSIC-STAGE`; `unit`-returning extern calls fail downstream at `Q024`, while keyword-shaped `spawn`, `join`, `share`, `grow`, `open`, and `close` are unreachable as call stages, §G02h).
* **`Q038` — File-wide positional operator gate:**
  * Refuses all five operator tokens `|`, `&`, `^`, `||`, and `&&` **everywhere outside a parser-recorded `if (…)` condition extent** (`OR-AND-LOWERING`, `BITWISE-CONDITIONS`), with the sole exception of the two `|CAPTURES|` bars of a `spawn` closure (`W48-QR-CLOSURES`).
  * Any closure capture-list shape written outside `spawn` refuses here by name (`W48-QR-CLOSURE-VALUES`). Occurrences *inside* an `if` condition are judged by `Branch` `Q045`, never `Q038`.

##### `Branch` (`Q044`–`Q046`, `source.branch.v1`)

Governs `if (COND) { … } else { … }` over `CfgTerminator::Branch` (separate from `CEmission` `Q500` because the typed CFG natively encodes two-way branches):

* **`Q044` — Unsupported branch/arm shape or continuation-hoisting boundary (`IF-STATEMENT`, `IF-ARMS`, `BITWISE-CONDITIONS`):**
  * Admits `qcore_check`, `drop`, `move`, nested `if`, empty arms, early `return` (discharging at its own exit; post-branch continuation statements move once into the surviving arm), and custody-neutral source calls.
  * Refuses: (1) mixed early-return/fall-through branches lacking a continuation or whose continuation reaches no `return`, early returns inside one arm of a nested branch whose sibling falls through (`IF-ARM-RETURN-NESTED`), and unreachable statements following an `if` where *both* arms return *(leaks at an arm's `return` report `Q202` at that exit)*; (2) conditional source calls whose callee summary flow is non-neutral (`IF-ARM-CALL-FLOW`); (3) arm-local `let` bindings; and (4) inconsistent condition/block counts or generic branch presentation errors.
* **`Q045` — Condition outside the check-time decidable two-sorted tier (§G02i, `OR-AND-LOWERING`, `BITWISE-CONDITIONS`):**
  * Admits `true`, `false`, signed 32-bit integer literals and top-level `let NAME = INTEGER;` constant bindings compared via non-associative `==`, `!=`, `<`, `<=`, `>`, `>=`, bitwise `&` > `^` > `|` inside integer comparison operands, prefix `!` (eliminated by swapping `then`/`else` arms), and `&&` > `||` (desugared into nested `if`s before core models run).
  * **C-compatible precedence law:** Precedence is defined **only where it agrees with C**. Because C ranks `!` tighter and `|`/`^`/`&` looser than comparisons, unparenthesized `!A < B` and `A | B == C` are **refused** rather than silently re-ranked (`!(A < B)` and `(A | B) == C` are required).
  * Refuses calls, `|>`, shifts, `~`, arithmetic, chained comparisons, resource bindings in conditions, sort mismatches, unbalanced parentheses, and empty conditions *(unbound names fail earlier at `Q013`)*.
* **`Q046` — Custody disagreement across surviving branch arms:** Two fall-through arms reach the join with unequal ordered live resource lists or unequal transferred sets (conditional leak, conditional double-drop, or move-vs-drop). Returning arms discharge at their own exit and do not participate in the join; arm-local acquisitions fail at `Ownership` `Q103` attributed directly to the acquisition span (`IF-ARMS`, follow-on `IF-ARM-ACQUIRE`).

##### `Loop` (`Q097`–`Q099`, `source.loop.v1`)

Governs counted bounded loops `for NAME in 0..BOUND { … }` (using exact check-time unrolling paired with real C `for`-loop emission; contextual `in`, reserved `for`, allocated from the `Q097`–`Q099` pool returned by `HTTP-QUIRIN-V1`):

* **`Q097` — Loop header, body-control, or unroll-attribute shape (`LOOP-V1`, `LOOP-UNROLL-BUDGET`):**
  * Header outside `for NAME in 0..BOUND` (low bound must be literal `0`), `break` or `continue` (`LOOP-BREAK`), `return` (`LOOP-BODY-RETURN`), or a body binding shadowing an outer entry binding.
  * Statement-adjacent unroll licence outside `[| #unroll, where #bound < N |]` — unknown clause word, missing `where`, comparator other than strict `<`, non-literal `N`, trailing clauses, or a licence placed before a non-`for` statement, doubled in a row, or dangling at block end *(unterminated `[|` fails at `Q000`; a `Q097` attribute syntax error suppresses `Q098` so one mistake emits one diagnostic)*.
* **`Q098` — Undecidable bound or three-layer unroll budget violation (`LOOP-V1`, `LOOP-UNROLL-BUDGET`, `W48-QR-SCALAR-TYPES-R5`, `W48-QR-FLOAT-BUFFER-PAYLOADS`):**
  * Bound outside the lowering-decidable constant tier (literal, literal `let`, or `extent(NAME)`; `LOOP-BOUND-FROM-BYTE`), zero or negative bound (zero-trip bodies would go unchecked), `bool` or `f32`/`f64` bound, `isize`/`usize` bound without a recorded target, or count overflowing/disagreeing with the index type.
  * **Three-layer unroll budget** (erased completely from emitted C):
    1. **Default unattributed budget (`256`):** Unattributed loop with `BOUND > 256` refuses with the exact `[| #unroll, where #bound < N |]` licence syntax in the diagnostic message.
    2. **Checked statement licence (`#bound < N`):** Refuses if `BOUND >= N`, if `N > 4097` (checked on the attribute alone before reading `BOUND`), or if `N <= 1` (`#bound < 1` / `< 0` licenses zero valid loops).
    3. **Absolute ceilings (`4096` iterations / `8192` unrolled events):** Single loop `BOUND > 4096` or nested-loop unrolled event product `> 8192`.
* **`Q099` — Non-identity per-iteration custody:** A single iteration of the loop body leaves the ownership ledger with different live resources, bindings, or footprints than on entry (enforcing custody neutrality even at `BOUND = 1`, complementing multi-iteration `Q054` / `Q078` unrolled checks).

*Provenance:* Cards `PIPE-OPERATOR`, `PIPE-CHAIN`, `PIPE-SCALAR-RESULT`, `PIPE-INTRINSIC-STAGE`, `PIPE-DROP-STAGE`, `IF-STATEMENT`, `IF-ARMS`, `OR-AND-LOWERING`, `BITWISE-CONDITIONS`, `LOOP-V1`, `LOOP-UNROLL-BUDGET`, `W48-QR-SCALAR-TYPES-R5`, `W48-QR-FLOAT-BUFFER-PAYLOADS`, `W48-QR-CLOSURES`.

---

#### 3.8. `StandardBuffer` (`Q051`–`Q057`)

Emitted by `qcore-frontend` and `qcore-verification::driver` for owned extent-carrying byte buffers (`source.buffer.v1`), scalar/nested/owning records (`source.record.v1`), range write lends (`LendRange`), bounded resource helpers (`source.resource-calls.v1`), and record/member checks on `source.ffi.v1` (selected after concurrent and extern routes and before `source.string.v1` / `source.std-io.v1`):

* **`Q051` — Entry, statement, record, or lend-window shape refusal:**
  * Violation of flat parameterless entry shape (including non-entry functions on `source.buffer.v1`, `W48-QR-SCALAR-TYPES-R2`); malformed `bytes(…)`, `extent(…)`, indexing, `lend(…)`, field-path, conversion, or value statement (including multi-operation value statements, `-R5`); `isize`/`usize` field or conversion without a recorded target (`-R3`, `-R5`); missing/duplicate `return` or malformed `effects` row.
  * **Record declarations & constructions (`AGGREGATES-V1`, `AGGREGATE-OWNED-FIELD`, `OWNED-FIELD-ROUTE-REACH`, `W48-QR-USABLE-RECORDS`):** Malformed `type NAME { … }` or `NAME { FIELD: VALUE, … }`; recursive self-containing record; duplicate, missing, or undeclared field; unsupported field type (`string` names `AGGREGATE-VIEW-FIELD`; other unmodeled types name `AGGREGATE-FIELD-TYPES`, whereas nested record fields are **admitted** since `W48-QR-USABLE-RECORDS` lifted `AGGREGATE-NESTED`); numeric field path; whole write of a sub-record or whole read of an owning sub-record; or reading/writing an unlisted opaque C struct member on `source.ffi.v1`. *(Plain copy bind `let a = b;` is **admitted** in shape since `OWNED-FIELD-ROUTE-REACH` and judged at `Q054`; route selection separates it from `source.socket.v1`'s `let listener = tcp;`.)*
  * **Lend-window structural exclusions (`SLICE-PERMISSIONS-WRITE-LEND`, `LOOPS-ON-COMPOSED-ROUTES`, `COMPOSED-WINDOW-REACH`):** While top-level `lend(BUFFER, LOW, HIGH)` across nested `for` loops is **admitted** via iteration-wide `last_uses` (`QCore.LoopWindows`), `Q051` refuses crossed non-nested lend windows (`SLICE-PERMISSIONS-CROSSED-WINDOWS`), `lend` shadowing a live name, buffer acquisition/release inside a lend window or lending loop body (`WRITE-LEND-NESTED-ACQUIRE`, `LOOP-WINDOW-BODY-CUSTODY`), per-iteration `lend` inside a `for` body (`LOOP-PER-ITERATION-LOAN`), `lend` in an entry composing an owning record (`OWNED-FIELD-LEND-WINDOW`), owned-field statements inside a `for` body (`OWNED-FIELD-LOOP-BODY`), and the independent fail-closed `window_unnameable_events` gate refusing field-path events inside `lend_windows`.
* **`Q052` — Check-time constant decidability or place/type kind mismatch:**
  * Non-check-time-constant index or value operand; custody position naming a non-buffer (or scalar field, `FieldIsNotABuffer`); `lend(…)` naming anything other than an entry-acquired owned buffer (including re-lending a loan, `WRITE-LEND-RELEND`); `drop` of a write loan or scalar-only record; non-buffer supplied at a `bytes` field (`FieldTakesABuffer`); field path continuing through a non-record field; wrong-type record/field value or field write (`W48-QR-USABLE-RECORDS`, `-R3`, `source.ffi.v1`); mixed binary operand types; or `bool` arithmetic, negation, ordering, conversion, or `hex` rendering (`-R3`, `-R5`). *(Constructing an undeclared record type fails earlier at `Q013`.)*
* **`Q053` — Bounds, byte/extent/nesting limits, and constant-tier arithmetic:**
  * Byte index outside `[0, extent)` (on an owned buffer, a rebased `[0, high - low)` write loan, or `RECORD.FIELD`); stored byte outside `0..=255`; declared buffer extent outside `1..=4096` (including `source.resource-calls.v1` helper acquisition bounds checked prior to value storage allocation); record field count outside `1..=32`, record nesting depth `> 8`, or total leaf count `> 256` (`W48-QR-USABLE-RECORDS`); `lend` range violating `0 <= low <= high <= extent` or `lend` bound outside the lowering-visible constant tier (`WRITE-LEND-BOUND-FROM-BYTE`, including bounds spelled through copy binds); out-of-range typed/float field or value literal (`-R3`, `W48-QR-FLOATING-POINT`); non-fitting conversion (`-R3`); or value statement faulting on known operands (`-R5`).
* **`Q054` — Inherited `qcore_core::ownership` custody, range-lend, path, and copy refusals:**
  * Leak at normal exit (`NormalPathLeak`, including `RECORD.FIELD`), `DoubleDrop` (buffer, record, or field path), `UseAfterDrop` / `UseAfterMove`, and `BindingAlreadyExists`.
  * **Range write lends (`SLICE-PERMISSIONS-WRITE-LEND`, `QCore.WriteLend`):** `OverlappingLends` (non-disjoint concurrent lends), owner `ReadRange`/`WriteRange` overlapping a live `LendRange` (or unresolvable rangeless index `WRITE-LEND-COMPUTED-INDEX`), `AccessOutsideLendRange`, and `DropWhileBorrowed` on releasing an owner with any live lend.
  * **Copy binds & owning records (`AGGREGATE-OWNED-FIELD`, `OWNED-FIELD-ROUTE-REACH`, `W48-QR-USABLE-RECORDS`, `QCore.OwnedFieldCopyReach`):** `CopyOfOwned` on `let a = b;` for owned buffers and records declaring owned fields (hollow or populated), `MoveOfBorrow` on copying a write loan, refill store over a still-live owned field, scalar field read/write on an owning record after move or `drop`, `source.resource-calls.v1` custody/aliasing refusals, plus the IR's `AggregateAtOwnerPosition` and `NotAnAggregate` backstops (unreachable from `source.buffer.v1` source behind `Q051`/`Q052`, retained for future direct IR consumers).
* **`Q055` — Boundary-crossing refusal:** Owned buffer or write loan used as a rendering operand (bare or `borrow`), array index, stored byte, or scalar `i32` record field, and any write loan placed in *any* record field (`AGGREGATE-OWNED-FIELD` deliberately narrowed `Q055` so owned buffers *are* admitted at `bytes` record fields; passing buffers to extern callees fails on the earlier extern route).
* **`Q056` — Certificate construction or rederivation failure:** Published `source.buffer.v1` or `source.resource-calls.v1` certificate fails construction or independent recheck.
* **`Q057` — Undeclared derived effect:** Entry or resource helper derives `Alloc`, `Mut`, or `Io` omitted from its declared `effects { … }` row (pass-distinct from `StandardIo` `Q029`; scalar-only records derive zero effects and emit no buffer C prelude).

*Provenance:* Cards `STD-BUFFER`, `AGGREGATES-V1`, `SLICE-PERMISSIONS-WRITE-LEND`, `AGGREGATE-OWNED-FIELD`, `LOOPS-ON-COMPOSED-ROUTES`, `OWNED-FIELD-ROUTE-REACH`, `COMPOSED-WINDOW-REACH`, `RESOURCE-HELPER-FUNCTIONS`, `W48-QR-SCALAR-TYPES-R2`/`R3`/`R5`, `W48-QR-FLOATING-POINT`, `W48-QR-C-STRUCT-IMPORT`, `W48-QR-USABLE-RECORDS`.

---

#### 3.9. `Event` (`Q058`–`Q062`, `Q074`)

Emitted by `qcore-frontend` and `qcore-verification::driver` for single-threaded event-reactor programs (`source.event.v1`, selected after concurrent and extern routes and before `source.std-io.v1`):

* **`Q058` — Flat reactor entry shape, `log_level` configuration, and certificate recheck:**
  * Entry must open exactly one reactor context before all other reactor statements and close it after all of them, contain no nested blocks or non-entry functions (`W48-QR-SCALAR-TYPES-R2`), and end with a single `return INTEGER;`; also covers entry-free modules and certificate recheck failures.
  * **Quiet-emission `log_level LEVEL;` refinement:** An event entry MAY contain at most one `log_level LEVEL;` before, inside, or after its live event context (requiring `std.logger`, deriving `Io`, projecting to zero reactor operations, independently rechecked, and selecting the hosted stdout trace adapter; likewise admitted across sequential, standard-I/O, extern-composed, and task-composed profiles under their own shape/import/effect codes).
  * *Check-only `write` interest:* Registering `write` interest is checked identically and sets `accept_check_only` (emitting no C and no reference observation, because kernel send-buffer writability is not program-induced).
* **`Q059` — Unknown endpoint or registration name:** Identifier is not a live endpoint or registration of the entry (base `source.event.v1` has no scalar/buffer value namespace).
* **`Q060` — Inherited reactor custody/lifecycle refusal:** Unmodified `qcore_core::event_reactor::check_event_program` rejected the projected program, carrying its `L00`–`L12` diagnostic verbatim (retiring the `U3` source-unreachability blocker).
* **`Q061` — Reactor determinism boundary:**
  * Declared `dispatch ready E;` / `dispatch idle;` outcome disagrees with the readiness induced by earlier `wake` operations (verified at runtime by the emitted binary against `poll(2)` / `WSAPoll`, turning any violation of platform premise `EVENT-V1-LEVEL-TRIGGERED-READINESS` into a non-zero exit rather than an unsound verdict).
  * **Multi-ready turn refusal:** Any dispatch turn at which **two or more registrations are simultaneously ready** is refused by name (`event_reactor::explore_activation_orders` treats equal-priority `0` callbacks as nondeterministic).
* **`Q062` — Undeclared derived `{Io}` effect:** Entry opens a reactor context (or configures `log_level`) without declaring `io` in `effects { … }` (`Alloc` and `Mut` are unreachable on this route).
* **`Q074` — Composed event lines and drained-byte binding (`COMPOSE-EVENT-LINES`, relocated from `Q065` at integration):**
  * Admits `source.std-io.v1` statements (`print`, `log`, `flush`), `wake D BYTE;`, and `let NAME = dispatch ready E;` (draining and binding the delivered byte under runtime-checked platform premise `EVENT-V1-STREAM-BYTE-ORDER`; line erasure preserves reactor projections and runs per `QCore.ComposeEventLines`, and single-threaded execution keeps strict byte-for-byte stream comparison).
  * Refuses non-value rendering operands (endpoints or registrations), `borrow NAME` operands (follow-on `STD-FMT-DESCRIPTOR`), any `let` other than `let NAME = dispatch ready E;`, malformed `std.io` line syntax, and the unreachable value/custody name-collision backstop behind `Q012`.

*Provenance:* Cards `STD-EVENT`, `QUIET-EMISSION-STREAMS`, `COMPOSE-EVENT-LINES`, `W48-QR-SCALAR-TYPES-R2`.

---

#### 3.10. `StandardString` (`Q068`–`Q073`, `Q075`–`Q076`, `Q077`–`Q079`)

Emitted by `qcore-frontend` and `qcore-verification::driver` for immutable, custody-free UTF-8 `{ptr, len}` string views (`source.string.v1`, selected after concurrent, extern, and standalone buffer routes and before `source.std-io.v1`; contextual `len(`, `concat(`, `borrow_utf8(`, and `: string` reserve zero global keywords):

* **Band 1 — Core UTF-8 string views and slicing (`Q068`–`Q073`, relocated from `Q065`–`Q070` at integration):**
  * **`Q068` — Entry/statement shape and certificate recheck:** Violation of flat parameterless entry shape, non-entry functions (`W48-QR-SCALAR-TYPES-R2`), nested blocks (**narrowed** by `LOOPS-ON-COMPOSED-ROUTES` to admit `for` loops in composed `borrow_utf8` string+buffer entries while keeping `for` refused in bufferless string modules), malformed literals/`len(…)`/slices, type ascriptions other than `: string`, missing/duplicate `return`, malformed `effects` row, entry-free module, or failed certificate rederivation.
  * **`Q069` — Undecidable slice/borrow bound or non-string view operand:** Slice or `borrow_utf8` bound is not a check-time constant integer bound in the entry *(unreachable from single-module source behind `Q013`, kept as fail-closed backstop `STRING-SLICE-NOTASCALAR-REACH`)*, or a view position (`len`, slice base, `concat` string piece) names a non-string binding.
  * **`Q070` — String literal decode or length failure:** Escape outside `\\`, `\"`, `\n`, `\t`, `\r`, `\0`, and `\xHH`; truncated `\x` or trailing `\`; decoded bytes not well-formed UTF-8; or decoded payload `> 4096` bytes (refused, never truncated).
  * **`Q071` — Slice out of range or off a UTF-8 code-point boundary:** Slice violates `0 <= low <= high <= len(s)` or splits a multi-byte UTF-8 code point (decided statically; never rounded).
  * **`Q072` — View/buffer crossing into scalar or formatted-rendering position:** String view (or buffer) supplied as a slice/`borrow_utf8` bound, `borrow` operand, or `std.fmt` scalar-formatted operand (`print dec s;`). *(Passing strings to extern callees fails on the earlier extern route.)*
  * **`Q073` — Undeclared derived `{Io}` effect:** Entry writes output without `io` in `effects { … }` (string operations themselves derive **neither `Alloc` nor `Mut`**).
* **Band 2 — UTF-8 text stream output (`Q075`–`Q076`, `STRING-SLICE-IO`):**
  * Renders exact UTF-8 bytes via `print TEXT;` and `log LEVEL TEXT;` (including `|>` desugarings) using length-bounded `fwrite` so embedded `\0` bytes are preserved verbatim and short writes abort (`QCore.StringSliceIo.the_two_line_kinds_share_one_framing`). Route selection triggers on constructed text operands (quoted literals or inline slices), leaving bare `print bogus;` in `source.std-io.v1` at `Q030`.
  * **`Q075` — Text-line shape refusal:** Missing operand, trailing tokens, malformed inline slice, integer scalar written without a `dec`/`hex` selector, or `borrow NAME` marker on a text line.
  * **`Q076` — Non-string binding at text output operand:** Bare name in `print NAME;` / `log LEVEL NAME;` is not a string bound by the entry (distinct from `Q069` view-internal positions so scalar bindings receive a targeted formatting hint).
* **Band 3 — Static string concatenation and range-wise `std.buffer` → `std.string` borrowing (`Q077`–`Q079`, returned pool from `STRING-SLICE-IO`):**
  * Admits static check-time `let NAME = concat(PIECE, …);` (2+ pieces over literals, strings, inline slices, and constant `dec(NAME)`/`hex(NAME)`, emitted as zero-allocation `static const` C payloads) and range-wise `let NAME = borrow_utf8(BUFFER [, LOW, HIGH]);` (arity **1 or 3**, whole-extent equivalent to `[0, extent)` per `QCore.StringConcat`).
  * **`Q077` — Computed-string shape, range bound, loop-body vocabulary, or embedded buffer refusal:** Malformed `concat` (`< 2` pieces or `> 4096` bytes), `borrow_utf8` (2-argument spelling or range violating `0 <= low <= high <= extent`), or `dec(NAME)`/`hex(NAME)`; non-buffer at `borrow_utf8`; `lend(…)` in a composed string/buffer entry (`SLICE-PERMISSIONS-COMPOSED-LEND`); per-iteration `borrow_utf8` or `lend` inside a `for` body (`LOOP-PER-ITERATION-LOAN`), loop body acquiring/releasing (`LOOP-WINDOW-BODY-CUSTODY`), or string/text constructs inside a loop body (`LOOP-ON-COMPOSED-ROUTE-BODY-VOCABULARY`); typed non-`i32` binding at an `i32`-only string position; and any embedded `source.buffer.v1` refusal (carrying the buffer model's own message under `Q077`). *(`W48-QR-SCALAR-TYPES-R6` **narrows** `-R5`'s earlier `Q077` ban so conversions, value statements, and typed integer loops are **admitted** on the composed route; loop header/bound/custody errors surface at `Q097`–`Q099`.)*
  * **`Q078` — Range-wise borrow exclusion and inherited buffer custody (`CT07-SLICE-PERMISSIONS`, `LOOPS-ON-COMPOSED-ROUTES`):** Refuses any byte store overlapping `[low, high)` of a live `borrow_utf8` loan (across both flat statements and unrolled `for` iterations via `loop_body_conflict`, `QCore.LoopWindows`; unresolvable indices conservatively conflict with all live loans), **any** buffer `drop` while *any* loan (even empty) is live, and any `qcore_core::ownership` leak, double-drop, use-after-drop, or rebind. *(Two overlapping immutable `borrow_utf8` views — including slices of a borrow — conflict on nothing and are deliberately **admitted**.)*
  * **`Q079` — Static UTF-8 failure on borrowed buffer write history:** The constant byte sequence held by `BUFFER` across `[low, high)` at the `borrow_utf8` point is not well-formed UTF-8 (including range endpoints that split a code point).

*Provenance:* Cards `STRING-SLICE`, `STRING-SLICE-IO`, `STRING-CONCAT`, `CT07-SLICE-PERMISSIONS`, `SLICE-PERMISSIONS-WRITE-LEND`, `LOOPS-ON-COMPOSED-ROUTES`, `W48-QR-SCALAR-TYPES-R2`/`R5`/`R6`.

---

#### 3.11. `StandardSocket` (`Q080`–`Q089`) and `StandardHttp` (`Q090`–`Q096`)

##### `StandardSocket` (`Q080`–`Q089`, `source.socket.v1`)

Emitted by `qcore-frontend` and `qcore-verification::driver` for checked single-thread loopback TCP/UDP sockets, composed reactor watches (`SOCKET-EVENT-COMPOSE`), declared IPv4/IPv6 addresses (`SOCKET-INET-ADDRESSES`), and protocol-cell disjointness queries (`CT42-CELL-ROUTE`), under runtime-checked `SOCKET-V1-STREAM-BYTE-ORDER` and timeout-bounded `SOCKET-V1-LOOPBACK-BUFFERED-SEND` premises (11 reserved clause words: `tcp`, `udp`, `bind`, `listen`, `connect`, `accept`, `send`, `recv`, `sendto`, `recvfrom`, `disjoint`):

* **`Q080` — Entry, socket, reactor-watch, IP-address, or `disjoint` syntax shape:** Violation of flat parameterless entry shape or non-entry functions (`W48-QR-SCALAR-TYPES-R2`); malformed socket statements; missing/doubled `event open`/`close`, unwatched registration at `return`, crossed watch spans, or poll-set bound overflow; malformed `ADDR:PORT` syntax — hostnames (`SOCKET-DNS`), unbracketed IPv6, embedded-IPv4 IPv6 (`SOCKET-INET-V6-EMBEDDED-V4`), IPv6 zone IDs (`SOCKET-INET-V6-ZONE`), leading-zero IPv4 octets, non-adjacent address tokens, or malformed port; and `disjoint` written with other than two names.
* **`Q081` — Non-constant integer operand or non-socket/non-registration target:** Payload byte, count, or port operand is not a check-time constant integer bound in the entry (including registration names used in value positions), or a socket/watch/`disjoint` target names no socket/registration of the entry.
* **`Q082` — Transport state-machine or address-family violation:** `listen`/`connect` on UDP; `listen` before `bind`; `connect` to a non-listening target; second `bind` on one socket or one declared address; `send`/`recv` on UDP or `sendto`/`recvfrom` on TCP; send on unconnected TCP or datagram to unbound UDP; operation on a closed socket; `watch` on a socket state the program cannot make ready; or IPv4/IPv6 family mismatch (every IPv6 socket explicitly sets `IPV6_V6ONLY = 1`, with host IPv6 absence probed cleanly and **never** falling back to IPv4).
* **`Q083` — Inherited custody or reactor model refusal:** Carries unmodified `qcore_core::ownership` refusals (`NormalPathLeak`, `DoubleDrop`, `UseAfterDrop` including `disjoint` on a closed socket, `DropWhileBorrowed` on closing a watched socket, and `OwnerMutationWhileBorrowed` on `bind`/`listen`/`connect` while watched — whereas `send`, `recv`, `accept`, and `disjoint` on a watched socket are admitted) and unmodified `event_reactor` `L00`–`L12` diagnostics (`watch` projects as `BorrowShared` without reactor-owned `CreateSocketPair`/`CloseSocket`).
* **`Q084` — Byte, length, exact-datagram, or port range violation:** Payload byte outside `0..=255`, payload or `recv` count outside `1..=1024`, `recvfrom` count unequal to the head datagram's exact byte length (never truncated), or port outside `0..=65535`.
* **`Q085` — Transport/reactor determinism or external-peer boundary:** `recv`/`recvfrom` not satisfied by committed program history; `accept` on an empty queue or second unaccepted connection on a depth-1 queue; `> 4096` unread stream bytes or exceeded UDP queue bound; **TCP stream** `close` with unread bytes or listener `close` with an unaccepted connection *(UDP `close` with queued datagrams is admitted because discarding local datagrams emits no packet)*; `connect`/`sendto` targeting an undeclared external or ephemeral address (`SOCKET-EXTERNAL-PEER`); reactor `dispatch` mismatch or 2+-member ready set; and fail-closed `socket_footprint` cell-derivation backstops on unqueued `accept` or listener-less `connect` (`CT42-CELL-FALLBACK-ARM-DIRECTION`, `QCore.SocketFootprint`). *(When `disjoint A B;` compares two ends of one accepted TCP connection, `DisjointFootprintsV1` refuses under **`Q664`**, erasing from C and stamping zero `DeclaredProtocolCellIdentity` via `DeriveDescriptorAlias`; `write` watches set check-only mode.)*
* **`Q086` — Socket boundary-crossing refusal:** Socket binding placed at a payload byte, port, or bare/`borrow` rendering operand.
* **`Q087` — Undeclared derived `{Alloc, Io}` effect:** Entry omits required `Alloc` or `Io` (`Mut` is never derived; reactor `dispatch` derives `Io`, while watch bookkeeping and `disjoint` derive nothing).
* **`Q088` — Certificate rederivation failure:** Published `source.socket.v1` certificate fails independent recheck.
* **`Q089` — Reference or native backend refusal:** Reference runner or platform backend rejects an admitted socket trace.

##### `StandardHttp` (`Q090`–`Q096`, `source.http.v1`)

Emitted by `qcore-frontend` and `qcore-verification::driver` for checked HTTP/1.1 messages composed over `source.socket.v1` (selected after concurrent, extern, event, and buffer routes and before standalone string and socket routes; 4 reserved words: `request`, `response`, `header`, `content`, with `Q097`–`Q099` returned to the pool for `Loop`):

* **`Q090` — Entry, message, computed-component, or recheck failure:** Flat entry shape violation or non-entry functions (`W48-QR-SCALAR-TYPES-R2`); undecodable string literal; extending a message after wire transmission or setting `content` twice; malformed `concat(…)` or `len(…)`, `< 2` concat pieces, composed payload `> 4096` bytes, bare `dec`/`hex` without `(NAME)`, or unreachable local value rebind backstop (`HTTP-COMPUTED-REBIND-REACH` behind `Q012`); entry-free module; or certificate-recheck / backend refusal.
* **`Q091` — Invalid RFC 9112 start line:** Version other than exact `HTTP/1.1`, non-`token` method, lowercase/miscased registered method (`HTTP-V1-METHOD-CASE`, RFC 9110 §9.1), non-origin-form request target, status code outside `100..=599`, or invalid reason-phrase byte.
* **`Q092` — Invalid RFC 9110/9112 header field (CRLF-injection exclusion):** Non-`token` field name (field names remain written tokens, `HTTP-V1-COMPUTED-FIELD-NAME`), control byte (`CR`/`LF` included) or non-ASCII `obs-text` byte (`HTTP-V1-OBS-TEXT`) in a field value, leading/trailing field-value whitespace, request without **exactly one** `Host` header, or unsupported `Connection` option (`HTTP-V1-PERSISTENT`). *Because every source component forbids `CR` and `LF`, every CRLF in an admitted message head is compiler-emitted and header injection is structurally unexpressible.*
* **`Q093` — RFC 9112 §6 message framing violation:** Declared `Content-Length` unequal to the content's exact byte length — **decided statically** after resolving literals, text bindings, `concat(…)`, and `dec(NAME)` to bytes (`HTTP-V1-COMPUTED-LENGTH`) — content with no `Content-Length`, non-`1*DIGIT` `Content-Length` (such as `hex(…)` or negative `dec(…)`), duplicate `Content-Length`, or any `Transfer-Encoding` (`HTTP-V1-CHUNKED`).
* **`Q094` — Wrong binding kind at message, text, or scalar position:** Socket or scalar at a message/view position, or wrong-kind binding in a computed component, `concat` piece, or `len(…)` operand *(unbound names fail at `Q013`)*.
* **`Q095` — Inherited `source.socket.v1` transport/custody/cell-derivation refusal:** Carries unmodified `StandardSocket` errors (custody, state machine, determinism) plus `socket_footprint` cell-derivation backstops (`CT42-CELL-ROUTE`; `disjoint client server;` on an HTTP exchange refuses at **`Q664`**).
* **`Q096` — Undeclared derived transport/output effect:** Entry omits `Alloc` or `Io` derived by its socket/output operations (HTTP message construction itself is purely static and derives zero effects).

*Provenance:* Cards `QUIRIN-SOCKET-V1`, `SOCKET-EVENT-COMPOSE`, `SOCKET-INET-ADDRESSES`, `HTTP-QUIRIN-V1`, `HTTP-V1-COMPUTED-LENGTH`, `CT42-CELL-ROUTE`, `W48-QR-SCALAR-TYPES-R2`.

---

#### 3.12. Core Sequential Passes (`Q1xx`–`Q5xx`)

* **`Usage` (`Q100`) and `Ownership` (`Q103`)** *(emitted by `qcore-frontend`)*:
  * **`Q100` — Quantitative usage-grade violation:** Declared or inferred `[low, high]` usage interval violated (including explicit local usage annotations inside error regions and on `source.resource-calls.v1`; copying an owning record is `CopyOfOwned` at `Q054`/`Q103`, never `Q100`).
  * **`Q103` — Core sequential ownership/borrowing violation:** Refusal from `qcore_core::ownership` on the sequential whole-program route — including branch-arm resource acquisitions leaving unequal identity ledgers, attributed directly to the acquisition span (`IF-ARMS`). *(Reachability note `OWNED-FIELD-SEQUENTIAL-UNREACHED`: Although `qcore_core::ownership` includes aggregate/path, range-lend, borrow, and `Copy` operations covered by `Q103` in principle, the sequential `whole_program` builder constructs only `Acquire`, `TransferOut`, `Drop`, and `Branch`; source programs exercise the extended vocabulary through `StandardBuffer` `Q054` and `StandardString` `Q078`.)*
* **`ResourceFlow` (`Q200`) and `CleanupLowering` (`Q202`)** *(emitted by `qcore-frontend`, `qcore-verification::driver`)*:
  * **`Q200` — Resource-flow summary or loop-bound error:** `(Δ, R, P, T)` flow mismatch, branch `Δ` disagreement, `AtMost(n)` loop over a non-zero-`Δ` body (`R06`), or checked `u64`/`i128` flow arithmetic overflow (§L10).
  * **`Q202` — Cleanup lowering and reference interpretation error:** Lowering or reference-execution failure, including `LiveResourcesAtNormalReturn` (attributed to the exact leaking `return` statement inside a branch arm rather than the enclosing function header, `IF-ARMS`).
* **`TypedErrors` (`Q300`) and `ErrorRegion` (`Q302`–`Q309`)** *(emitted by `qcore-frontend`, `qcore-verification::driver`)*:
  * **`Q300` — Undeclared or unhandled typed error / effect:** Raising or propagating an error absent from the function's signature — including task-route runtime-faulting value statements or runtime float-to-integer conversions without `throws<ArithmeticError>` (`W48-QR-SCALAR-TYPES-R4`, `W48-QR-FLOATING-POINT`), task-route record field writes without `mut` in `effects { … }` (`W48-QR-TASK-RECORD-CAPTURES`), and declaring a typed `Buffer<T>` payload inside an error variant (`W48-QR-SCALAR-TYPES-R6`, `W48-QR-TYPES-ERROR-PAYLOAD`).
  * **`Q302` — Error-region structure and `source.region.v1` / composed-task region shapes:**
    * Core `error_region` structural errors (`DuplicateActiveRegion`, `DuplicateCatchClause`, `DuplicateResourceIdentity`, `ResourceHasNoCleanupAction`, `ResourceHeldByCleanupPack`, `PayloadResourceNotOwned`, `NestedPackNotDischarged`, `UnknownCleanupCapability`).
    * `source.region.v1` v1 path/shape boundaries (`QUANT-REGION-SOURCE`), each refused by name: `REGION-NORMAL-EXIT` (region lacking a decided error edge), `REGION-MULTIPLE-CATCH`, `REGION-MULTIPLE`, `REGION-PROPAGATE`, `REGION-FAILING-ACQUIRE`, `REGION-IN-ARM`, `REGION-HANDLER-BODY` (handler statement other than `discharge BINDING;`), and `REGION-CATCH-PAYLOAD-BINDING`.
    * Composed task-scope error regions on `source.epoch.tasks.v1` (`REGION-CREDIT-JOINS`, `REGION-JOINS-DUTIES`, `REGION-JOINS-LOAN`, `CT24-CLEANUP-TASK-COMPOSITION`): admits a parent `error region` placed after all spawns and joins — or directly inside the `Ok` arm of a direct creator-scope `spawn` `match` (`CT24-CLEANUP-TASK-COMPOSITION`) — carrying `credit N;`, `effort N;`, `qcore_check(false, ERROR);`, task-buffer duties `let NAME = qcore_buffer_new(INT)?;` / `drop NAME;`, and live entry-owner adoptions `move NAME;` alongside worker read shares (erasing into its straight-line task twin). Refuses regions in workers or outside the task scope, releasing unacquired outer resources (`REGION-OUTER-LIVE-RESOURCES`), owner/entry name collisions, taking one owner twice, adopting an unbound entry name, non-fitting erasures (`REGION-JOINS-ERASURE-FIT`), and sequential `qcore_acquire` inside task regions *(simultaneous multi-owner leaks and `LoanOutstanding` destructions fail on the erased task program at `Q600`, and `move` of a non-buffer fails at `Q500`)*.
  * **`Q303`:** `quant open` names a region other than the sealed cleanup pack's (`WrongRegionOpened`).
  * **`Q304`:** Single linear cleanup-pack capability consumed twice (`CleanupPackDuplicated`).
  * **`Q305`:** Catch handler returns normally while its opened cleanup pack remains undischarged (`CleanupPackNotDischarged`).
  * **`Q306`:** Nonempty sealed cleanup pack reaches a `catch` clause that omits `quant open` (`CatchWithoutRequiredQuantOpen`).
  * **`Q307`:** Handler exits via error/rethrow with an opened cleanup pack still undischarged (`RethrowWithUndischargedCleanup`).
  * **`Q308`:** Registered cleanup action is not total and `noexcept`.
  * **`Q309`:** Same resource identity appears simultaneously in both the error payload and a sealed cleanup pack (`ResourceInPayloadAndCleanupPack`).
* **Driver Invocation (`Q400`, `Q401`, `Q403`)** *(emitted by `qcore-verification::driver`)*:
  * **`Q400`:** No input files supplied to the driver.
  * **`Q401`:** Missing or ambiguous entry function.
  * **`Q403`:** `--entry` passed against a `module library;` or `module contract;` unit, incompatible `EXT-01` source file suffix vs. declared module mode, or ambiguous simultaneous legacy (`.qcore`/`.qc`) and current (`.qr`/`.qv`) source aliases.
* **`CEmission` (`Q500`)** *(emitted by `qcore-frontend`, `qcore-verification::driver`)*:
  * **`Q500` — Construct outside reduced C11 CFG or task C backend encoding:** Unencodable AST/CFG forms — including multi-operation value statements on the scoped-task route (`W48-QR-SCALAR-TYPES-R4`), `Buffer<T>` write of a value mismatched with its payload type (`-R6`), `move` adoption of a non-buffer in a composed task error region (`REGION-JOINS-LOAN`), a `bytes` field inside a task-route record (`WRITE-LEND-TASKS-EXTENT`), any record declaration on the composed epoch route (`W48-QR-TASK-RECORD-EPOCH`, after `W48-QR-TASK-RECORD-CAPTURES` lifted `AGGREGATE-TASK-CAPTURE` on the scoped-task route), and the unedited fail-closed task-capture backstop behind `Q600`.

*Provenance:* Core Cycle 3–7 & W3/W7 map; cards `IF-ARMS`, `QUANT-REGION-SOURCE`, `REGION-CREDIT-JOINS`, `REGION-JOINS-DUTIES`, `REGION-JOINS-LOAN`, `CT24-CLEANUP-TASK-COMPOSITION`, `AGGREGATE-OWNED-FIELD`, `OWNED-FIELD-ROUTE-REACH`, `EXT-01`, `W48-QR-SCALAR-TYPES-R4`/`R6`, `W48-QR-FLOATING-POINT`, `W48-QR-TASK-RECORD-CAPTURES`.

---

#### 3.13. `Concurrency` (`Q600`, `Q610`–`Q627`, `Q630`–`Q641`, `Q650`–`Q667`, `Q670`–`Q677`)

Emitted by `qcore-frontend` and `qcore-verification::driver` across `Q600` and four disjoint model-profile sub-bands:

##### `Q600` — Scoped tasks, transferability, leases, records, closures, and structural concurrency refusals

* **Base concurrent & structural profile refusals:** Scoped-task ledger errors (`ScopedTaskLedger`), structural concurrency-trace errors (`JoinWhileGuardHeld`, `DataRace`, `DuplicateLockCapability`, `NonquiescentDestruction`, `DeadResource`), `source.bitfield-concurrency.v1` source-shape/worker/capture/join/window grammar refusals (`CT10-SOURCE-LAYOUT`), and `mutex_c_emit` backend refusals reached from single-task and contiguous non-nested fork/join traces (`MUTEX-SHARE-CROSS-EMISSION`; guard-local joins, nested spawns, interleaved parent/child events, custody transfers, waiter registrations, and child typed-error exits remain check-only without emitted C).
* **Transferability judgment (`CT47-COPY-CAPTURE`, `qcore_core::transfer`, `QCore.Transferability`):** Task-creation captures refused by the transferability matrix are reported at `Q600` at the crossing site naming the missing transfer/share contract and the kind's admitted contracts (`TransferRefusal`: uncontracted mode, `write` capture, delivered-kind mismatch, non-crossing kind, or unavailable protocol), ahead of `Q500`'s backstop.
* **Task-region ordering, read-loan narrowing, and `ScopeBusy` attribution (`REGION-CREDIT-JOINS`, `REGION-JOINS-LOAN`, `CT24-CLEANUP-PREEMPTED`, `CT24-CLEANUP-TASK-COMPOSITION`):**
  * Refuses any `spawn` or `join` ordered after the parent's epilogue `error region` (while `CT24-CLEANUP-TASK-COMPOSITION` narrows placement to admit a direct handled region in the exact `Ok` arm of a direct creator-scope `spawn` `match`; cleanup-before-join remains `Q600`).
  * **Deliberately narrows `Q600`** to admit `ScalarOperand::Loan` read-share captures (`read;` clause) on `source.epoch.tasks.v1` (`T08-COMPOSE-LOAN` discharged), while keeping `LoanOutstanding` and `REGION-JOINS-MULTI-OWNER` at `Q600`.
  * Renders early-exit `ScopeBusy(..)` from the factored exit-loan backstop naming the preempted owner and unjoined borrower (`QCore.TaskExecution.attribution_never_moves_the_refusal`).
* **Scoped-task range write leases (`WRITE-LEND-TASKS`, `WRITE-LEND-TASKS-EXERCISE`):**
  * `TransferMode::LendWrite` (`lend PLACE LOW..HIGH`), delivered `lend<Buffer>` / `lend<Buffer<T>>`, and `ShareProtocol::DisjointWriteLeases`; check-time literal range validation (`0 <= low < high <= payload_bytes`, refusing variable bounds `WRITE-LEND-TASKS-VARYING-BOUND`, empty ranges, and out-of-bounds ranges).
  * `ScopedTaskLedger` lease checks: `OverlappingLeases` (via `ownership::axes_overlap`), `LeaseOutstanding` (blocking parent read, write, drop, move, or read-share mixing), `InvalidLease`, and `TooManyWriters` (`QCore.WriteLendTasks.no_byte_has_two_write_authorities`).
  * Worker-body byte access (`WRITE-LEND-TASKS-EXERCISE`): spawn-site footprint fit (`low + worker_bound <= high`, `QCore.WriteLendTasksExercise`), constant-index tier (refusing variable/negative/out-of-bounds indices), and authority tier (requiring a write lease for byte writes, forbidding byte reads on counted read shares, and gating owner byte reads on zero outstanding leases).
* **Mutex-mediated child-task sharing gates (`MUTEX-SHARE-CROSS`):** Adds `ProtectedResource`, `MutexShare`, and `ShareUnderMutex` (each sharing mode requesting one exact protocol: `CheckedMutex` vs. `CountedReadShares` vs. `DisjointWriteLeases`). At `thread scope NAME share PLACE under MUTEX { … }`, `Q600` refuses if `MUTEX` is **absent** (not a mutex generation) or **mismatched** (guards a different resource), and restricts the declaring child exclusively to `PLACE` *(unguarded child accesses report `Q611` in the mutex sub-band)*.
* **Scoped-task records (`W48-QR-TASK-RECORD-CAPTURES`):** Aggregate transferability refusals (`copy` of an owning record naming the non-copyable leaf, `move` of a purely scalar record, `write` or whole-record sharing, nominal record type mismatch); post-move whole-record use; `match` arm record-state mismatch (`W48-QR-TASK-RECORD-ARM-MERGE`); whole move of an owning sub-record; invalid record return custody; `OverwriteOfLiveField`; unpacked collected record at exit; per-leaf custody rules; and **deliberately narrows `Q600`** so the single-active-lease limit applies *per underlying resource*, admitting simultaneous write leases on two distinct `Buffer` fields of one record.
* **Fixed-width task scalars & typed `Buffer<T>` payloads (`W48-QR-SCALAR-TYPES-R4`, `-R6`):** Checker re-decision of task value statements and raising error-exit edges; lease or byte read past `sizeof(T)`; byte write into a `bool` payload; `protected isize`/`usize` without a recorded target on `source.mutex.guard.v1`; and **deliberately narrows `Q600`** to discharge `source.epoch.tasks.v1`'s blanket refusal of fixed-width task halves.
* **Spawn closures (`W48-QR-CLOSURES`):** Capturing non-values (`handle`, `Result`, `scope`, error, `unit`), writing closures on unsupported routes (`W48-QR-CLOSURE-EPOCH`, `-MUTEX`, `-ROUTES`, `W48-QR-TASK-FFI-STRUCT-WORKER`, or non-task modules), or unlowered closure spawn sites.

##### Sub-Band `Q610`–`Q627` — `CheckedMutexInvariantV1` (`source.mutex.guard.v1`)

* **`Q610` (`UnprotectedFootprint`):** Guard's mutex does not protect the accessed interval footprint.
* **`Q611` (`UnguardedProtectedAccess`):** Protected footprint accessed without holding its protecting guard — including unguarded child-task reaches under `thread scope NAME share PLACE under MUTEX` (`MUTEX-SHARE-CROSS`, `QCore.MutexShareCross`).
* **`Q612` (`WrongGuardAffinity` / `DeadGuard`):** Guard used by a task other than its acquirer, or after unlock/release.
* **`Q613` (`InvariantOpenAtUnlock` / `DoubleOpenInvariant` / `WriteWithClosedInvariant` / `InvalidInvariantWitness`):** Critical-section invariant phase or `NoWriteOccurred` witness violation.
* **`Q614` (`InvariantOpenAtErrorExit`):** Typed-error exit from a critical section with its invariant still `Open`.
* **`Q615` (`GuardBorrowEscape`):** Guard-derived borrow outlives its enclosing critical section.
* **`Q616` (`StaleProtectedFact`):** Guard-scoped refinement fact referenced after its guard released.
* **`Q617` (`FootprintAlreadyProtected`):** `CreateMutex` or `GrowProtection` interval overlaps an already protected footprint.
* **`Q618` (`DynamicProtectedShape`):** Unconditional mid-lifetime `ExtendProtection` shape mutation.
* **`Q619` (`NonquiescentMutexDestruction`):** Destroying a mutex while held or with registered waiters.
* **`Q620` (`ForgedSynchronizationWitness`):** Cross-task handoff `SyncWitness` absent from derived vector-clock happens-before history.
* **`Q621` (`MutexAuthorityNotConserved`):** Pointwise share conservation violated at creation, destruction, or `GrowProtection` (including growth by a non-creator task).
* **`Q622` (`UnsupportedSynchronizationConstruct`):** Condition variables, try/timed/RW/upgradeable/nested/recursive locks, dynamic lock ranks, or completion-capsule composition.
* **`Q623` (`DeadMutexGeneration`):** Operation on a destroyed mutex generation.
* **`Q624` (`LockRankViolation`, `CT33-RANKS`):** Acquiring a mutex at `rank` while holding a guard at `held_rank >= rank`.
* **`Q625` (`MixedLockRankDeclaration`, `CT33-RANKS`):** Rank discipline active on only a strict subset of created/declared mutex generations.
* **`Q626` (`ProtectionGrowthOutsideOpenInvariant`, `CT34-GROWTH`):** `grow` attempted while the guard's invariant is `Closed`.
* **`Q627` (`ProtectionGrowthInvariantUnchanged`, `CT34-GROWTH`):** `grow` reuses a previously carried `InvariantId` instead of advancing to a fresh predicate identity.

##### Sub-Band `Q630`–`Q641` — `FlatEpochBoundV1` & `EpochCreditRecoveryV1` (`source.epoch.bound.v1`, `source.epoch.tasks.v1`)

* **`Q630`:** Structural epoch or recovery-plan shape error (`EmptyEpoch`, `DuplicateTask`, `DuplicateSharedCell`, `DuplicateSharedEntry`, `TaskCountMismatch`, `EmptyRetainedResult`, `PhaseVectorMismatch`, or `CT23-CREDIT-ERROR-PATH` plan mismatch / error-recovery clause written on the declarative error-free route).
* **`Q631` (`UnknownSharedCell`):** Task references an undeclared shared storage cell.
* **`Q632` (`UnsupportedEpochShape`):** Nested tasks, dynamic task count, dynamic/mutated shared storage, inter-task communication, recursive workers, latency/stack claims, or communicating dynamic heaps.
* **`Q633` (`RetainedResultExceedsTaskPeak`):** Retained task result exceeds the task's accounted `private_peak`.
* **`Q634` (`ResultDiscardedAtCompletion`):** Retained task result silently discarded at worker completion.
* **`Q635`:** `UntransferredResultAtEpochExit`, `UnjoinedTask`, or (`CT23-CREDIT-ERROR-PATH`) **credit abandoned at the creator's error edge**.
* **`Q636`:** `TaskExceedsItsCredit`, or (`CT23-CREDIT-ERROR-PATH`) **error-path credit restoration larger than its original grant**.
* **`Q637` (`CreditsExceedAllocation`):** Sum of granted task credits exceeds epoch allocation *(deliberately not extended by `CT23-CREDIT-ERROR-PATH`)*.
* **`Q638` (`EpochExceedsAllocation`):** Computed `epoch_peak` residency exceeds epoch allocation.
* **`Q639` (`WallClockUnavailable`):** Wall-clock, makespan, or latency bound requested (`NoScheduleModel`).
* **`Q640` (`MeasuredOverheadIsNotADeclaredBound`):** Empirical native-run observation offered in place of `OverheadBasis::Declared`.
* **`Q641` (`Arithmetic`):** Checked flow or error-recovery (`CT23-CREDIT-ERROR-PATH`) arithmetic overflow.

##### Sub-Band `Q650`–`Q667` — `DisjointFootprintsV1` (`source.bitfield-concurrency.v1`, `CT42-CELL-ROUTE`, `CG18-FOOTPRINT-CONNECTION`)

* **`Q650` (`InvalidExtent`):** Zero element size, zero count, zero-length slice, overflowing byte interval, or **zero-byte `memcpy`/`memmove`/`memset` transfer** (`CG18-FOOTPRINT-CONNECTION`).
* **`Q651` (`UnknownAllocation`):** Event names an unknown `AllocationId`.
* **`Q652` (`StaleGeneration`):** Slice, view, or byte transfer (`CG18`) names a retired/stale allocation generation.
* **`Q653` (`SliceOutOfBounds`):** Slice or **`memcpy`/`memmove`/`memset` byte count** (`CG18`) exceeds its allocation/handle extent.
* **`Q654` (`IdentityNotFresh`):** `HandleId` reused across events (retired handles included).
* **`Q655` (`UnknownHandle`):** Field derivation, disjointness check, view, or byte transfer (`CG18`) names an unknown handle *(unreachable from `source.socket.v1` `disjoint` behind `Q081`)*.
* **`Q656` (`LayoutContractMissing`):** Record field derivation lacks a `LayoutContract`.
* **`Q657` (`InconsistentLayoutContract`):** Whole-record layout contract rejected (`ZeroSize`, `NoFields`, duplicates, out-of-bounds/overlapping addressable fields, or inconsistent/overlapping bit-field storage units).
* **`Q658` (`UnknownField`):** `FieldId` absent from the record's `LayoutContract`.
* **`Q659` (`ElementShapeMismatch`):** Field derivation over a non-single-record handle, or **view / byte transfer over a bit-field storage unit or descriptor** (`CG18`).
* **`Q660` (`OverlappingFootprints`):** `AssertDisjoint` over overlapping byte ranges of one generation.
* **`Q661` (`SharedBitfieldUnit`, `CT10-SOURCE-LAYOUT`):** Two bit-fields share the same C11 storage unit — reachable from source on `source.bitfield-concurrency.v1` simultaneous worker write captures regardless of non-overlapping bit offsets/widths.
* **`Q662` (`CrossAllocationDisjointnessUnsupported`, `CG18`):** Disjointness assertion, `memcpy`, or composed mutex-invariant span across two distinct allocations whose live generations were not **both** declared fresh under `P-DISTINCT-ALLOCATIONS-DISJOINT`.
* **`Q663` (`IncomparableFootprintKinds`):** Comparing a memory byte footprint against a protocol-descriptor footprint.
* **`Q664` (`SharedProtocolCell`, `CT42-CELL-ROUTE`):** Two descriptors advance the same shared `ProtocolCellId` — reached from source via `disjoint NAME NAME;` on `source.socket.v1`, socket+event, and `source.http.v1` when comparing the two ends of one accepted TCP connection (`DeriveDescriptorAlias` stamps zero `DeclaredProtocolCellIdentity`, `QCore.SocketCellRoute`).
* **`Q665` (`DerivedFootprintMismatch`, `CG18`):** Declared byte interval disagrees with derived slice interval, or a composed mutex-invariant trace names an interval exported by no live derived handle.
* **`Q666` (`DestroyedAllocation`):** Operation on a destroyed allocation.
* **`Q667` (`UnsupportedFootprintConstruct`, `CG18`):** Unions, FAMs, VLAs, computed indices, out-of-extent pointer arithmetic, nested anonymous aggregates, cross-allocation alias proofs, atomic reclamation, **type-punned views (`TypePunnedAlias`)**, and **overlapping `memcpy` ranges (`OverlappingCopy`)**.

##### Sub-Band `Q670`–`Q677` — `CheckedRegionTaskCreditV1`

*(Error-region conditions reuse `Q300` / `Q302`–`Q309`; task-ledger conditions reuse `Q600`.)*
* **`Q670` (`PendingLifetimeObligation`):** Cleanup pack discharged or region exited while a child `join` or read loan remains outstanding.
* **`Q671` (`ChildCreditOutstanding`):** Pack discharged or region exited while allowable credit remains lent to an unjoined child.
* **`Q672` (`ObligationEscapesScope`):** Error escapes the flat thread scope carrying an unresolved join, read-loan, or child-credit duty.
* **`Q673` (`CreditNotConserved`):** Conservation equation `initial = Σ frames + Σ child-held + spent + abandoned` violated.
* **`Q674` (`UnusedCredit`):** Normal exit from a region leaves unspent allowable credit (credit may be abandoned only on checked error discharge).
* **`Q675` (`InsufficientCredit`):** Region endowment, `Spend`, or child credit loan exceeds available frame credit.
* **`Q676` (`ObligationOutsideRegion`):** Duty-opening or credit-moving operation executed outside any active region.
* **`Q677` (`PayloadObligationDropped`):** Owned error-payload resource still held at an exit that cannot carry it.

*Provenance:* Core Cycle 14–18 & W6 concurrency models; cards `CT33-RANKS`, `CT34-GROWTH`, `CT23-CREDIT-ERROR-PATH`, `REGION-CREDIT-JOINS`, `REGION-JOINS-LOAN`, `CT24-CLEANUP-PREEMPTED`, `CT24-CLEANUP-TASK-COMPOSITION`, `CT47-COPY-CAPTURE`, `WRITE-LEND-TASKS`, `WRITE-LEND-TASKS-EXERCISE`, `MUTEX-SHARE-CROSS`, `MUTEX-SHARE-CROSS-EMISSION`, `CT10-SOURCE-LAYOUT`, `CT42-CELL-ROUTE`, `CG18-FOOTPRINT-CONNECTION`, `W48-QR-SCALAR-TYPES-R4`/`R6`, `W48-QR-TASK-RECORD-CAPTURES`, `W48-QR-CLOSURES`.

---

#### 3.14. Verification, Provider Host, Contract Loader, and Premise Reader Namespaces

##### `PluginVerification` (`Q700`–`Q703`, `qcore-verification`)

* **`Q700`:** Source `verify scope` resolution failure (duplicate scope name, malformed/uninstalled/disabled plugin ID, version mismatch, or undeclared capability/target profile).
* **`Q701`:** Core-derived obligations remain undischarged after provider dispatch.
* **`Q702`:** Conflicting provider evidence, bounds, or assumptions across dispatched providers (`verify_dispatch.rs`).
* **`Q703`:** Provider allegation that the trusted core cannot independently validate (`verify_dispatch.rs`).

##### Provider Host & Certificate Firewall (`QP001`–`QP016`, `qcore-verifier-host`, `qcore-verification`)

* **`QP001`:** Malformed verification request, invalid `VerificationTask`, or request `task_digest` / bound source-projection digest mismatch.
* **`QP002`:** Requested verifier plugin is not installed in the registry.
* **`QP003`:** Requested verifier plugin is disabled.
* **`QP004`:** Plugin manifest does not support the negotiated `protocol_version` or `evidence_version`.
* **`QP005`:** Plugin manifest does not declare the requested `VerifierCapability` and `TargetProfile`.
* **`QP006`:** Provider execution/transport failure — pre-spawn profile/confinement/quota/identity refusal (`P01-F3`, `QUOTA-PROCS-DESCRIPTOR-DUP-FAIL-OPEN`), spawn/I/O error, wall-clock timeout, output cap overflow, non-zero exit/signal, unconsumed request bytes (`HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING`), or request pipe held by a detached descendant (`HOST-REQUEST-READ-BY-DETACHED-DESCENDANT`).
* **`QP007`:** Invalid plugin response envelope (mismatched `protocol_version`, `request_id`, or `plugin_id`, or malformed `<plugin-id>.<local-code>` diagnostic code).
* **`QP008`:** Provider proposed `Safe` with `evidence: None`.
* **`QP009`:** Trusted-core certificate-firewall rejection — **permanent refusal of every protocol-v1 `Safe` proposal** (including empty or 1-for-1 matching Boolean claims), mismatched v2 `evidence_version` / unit / capability / assumptions / reproducibility / `task_digest`, failed `WitnessChecker::check` replay, or discharged set unequal to the core's required obligation inventory.
* **`QP010`:** Unverified provider `Unsafe` allegation downgraded to `Unknown` (neither protocol v1 nor evidence v2 ships a counterexample checker), or `compose_verification` invoked with zero outcomes.
* **`QP011`:** Multi-provider composition includes a required `Unknown` outcome.
* **`QP012`:** Composed certificates bind the same assumption key to conflicting Boolean values.
* **`QP013`:** Composed certificates make conflicting predicate claims or discharge the same obligation ID under incomparable `TaskBounds`.
* **`QP014`:** `Safe` outcome passed to composition lacks both a `CoreCertificate` and a `CompositeCertificate`.
* **`QP015`:** Composed certificates certify different `UnitIdentity` values.
* **`QP016`:** Incomplete core obligation derivation for the requested capability, no registered trusted-core `WitnessChecker` for the capability, or empty core obligation inventory lacking an independently checked derivation-coverage witness.

##### Contract Inclusion Loader (`QI001`–`QI020`, `qcore-contract`, [diagnostics page](agents/diagnostics.md#contract-inclusion-namespace-qi), [contract specification](QCORE_CONTRACT_SPEC_r0_3.md) §C15)

Loader-owned namespace for `.qv` `#include` and `contract inherit` (allocates zero `Q` route codes; unresolved inclusion reaching the structural parser stays `Q000`):
* **`QI001`:** Malformed `#include` / `contract inherit` syntax or inheriting declaration.
* **`QI002`:** Contract clauses written after `inherit`.
* **`QI003`:** Inclusion target not spelled `crate:module`.
* **`QI004`:** Target crate absent or neither own crate nor a declared dependency.
* **`QI005`:** Target contract module not published.
* **`QI006`:** `#include` dependency cycle.
* **`QI007`:** Duplicate `#include` of the same target.
* **`QI008`:** Inherited parent declaration not in scope.
* **`QI009`:** Ambiguous parent declaration (two parents in scope).
* **`QI010`:** Child vs. parent arity, parameter type, or return type disagreement.
* **`QI011`:** `contract inherit` cycle.
* **`QI012`:** Capture or consumer interpretation differs between parent and child.
* **`QI013`:** Contract inclusion used with no load path supplied.
* **`QI014`:** Invalid load path, or malformed/mismatched/stale load-path manifest (`CONTRACT-INHERIT-ADOPTION`).
* **`QI015`:** Parent declaration is not inheritable.
* **`QI016`:** Inclusion-evidence replay disagrees with recorded derivation.
* **`QI017`:** Provider crate name matches neither `quirin-provider-*` nor `qcore-provider-*` (`CONTRACT-INHERIT-QCORE-PROVIDER-NAMING`), namespace not derived from crate name, or stem shared across both prefixes.
* **`QI018`:** Unqualified parent name declared only in an included module.
* **`QI019`:** Qualified parent namespace names no directly included crate.
* **`QI020`:** Inheriting child carries the same name as its qualified parent.

##### Premise & Symbolic Formula Readers (`QF001`–`QF005`, `qcore-frontend`, `qcore-contract`, [diagnostics page](agents/diagnostics.md#premise-formula-namespace-qf))

Reader-owned namespace enforcing symbolic mathematical contract formulas over retired English prose strings ([contract specification](QCORE_CONTRACT_SPEC_r0_3.md) §C04.18, §C04.20; `QF001`–`QF004` surface on the compiler route as `Q000` prefixed with `QF00*`):
* **`QF001` (`SP5-GLOBAL-PROSE-PREMISE-BAN`):** `resource_premise(NAME, …)` whose statement is or contains a string literal (`premise_formula::premise_clause`).
* **`QF002` (`SP6-SYMBOLIC-HYPOTHESIS-ATOMS`):** Retired `resource_hypothesis(NAME, "STATEMENT")` clause (`premise_formula::hypothesis_clause`) or declared hypothesis-graph atom with an opaque string statement (only generated `legacy_atom_*` erasures retain opaque statements).
* **`QF003` (`SP6-GUARDED-SYMBOL-PROSE`):** String literal in `resource_guarded_symbol_v1` or `resource_guarded_decomposition_v1`, or a guarded binding whose proposition is not the canonical rendering of a §C04.18.9 typed formula (`guarded_decomposition`).
* **`QF004` (`SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS`):** `quant #outer [[contract]] NAME, …` definition whose statement is or contains a string literal rather than a §C04.18.9 formula (`premise_formula::outer_header`, `outer_contract`).
* **`QF005` (`SP10-FORMAL-BINDING-DESCRIPTIONS`):** JSON binding-manifest or premise-flow environment value / formal-binding field containing untyped prose instead of a canonical G01p typed parameter/term reference or content digest (`formal_binding`).

##### External Provider-Owned Diagnostics (`<plugin-id>.<local-code>`)

* Every provider-emitted diagnostic code MUST equal `<selected-plugin-id>.<local-code>`, where `<local-code>` is a single **1–128-byte ASCII identifier** composed exclusively of `[A-Za-z0-9_-]` (total code length `<= 257` bytes; e.g., `qcore.libevent.L00`–`L12`). Multi-dot suffixes, empty suffixes, mismatched plugin prefixes, and control/non-ASCII bytes are rejected at the host firewall with `QP007`.

*Provenance:* Cycle 20 host & W3/W7 dispatch; cards `P01-F3`, `P03-F`, `HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING`, `HOST-REQUEST-READ-BY-DETACHED-DESCENDANT`, `CONTRACT-INHERIT`, `CONTRACT-INHERIT-ADOPTION`, `CONTRACT-INHERIT-QCORE-PROVIDER-NAMING`, `SP5`, `SP6`, `SP8`, `SP10`.

---

#### 3.15. Cross-Pass Route Reuse and Horizontal Widenings Summary

Under `Q-NAMESPACE-DECISION`, multi-pass composed profiles and horizontal Wave-48 type, record, helper, and closure extensions allocate **zero new `Q` codes**, routing each condition to the semantic pass that owns that kind of obligation:

| Horizontal Feature / Route | Reused & Widened Pass Codes | Deliberate Narrowings (Lifts) & Key Invariants |
|---|---|---|
| **Imported C structs & linkage (`source.ffi.v1`)**<br>`W48-QR-C-STRUCT-IMPORT`, `W48-QR-USABLE-RECORDS`, `W48-QRLIB-LINKAGE` | • `UncheckedLinkage`: `Q032` (`std.ffi.unsafe`), `Q033` (`#c_ffi` marker), `Q034` (no `unsafe extern fn`), `Q035` (`unsafe-ffi` premise tripwire)<br>• `NativeLibrary`: `Q039`/`Q041`/`Q047` (`.qrlib` v2), `Q063`–`Q064` (Clang layout, recorded compiler, header digests), `Q065`–`Q067` (`extern library` v2 & `linked:NAME`)<br>• `StandardBuffer`: `Q051`–`Q053` (§G07.13 record construction, opaque members, `1..=32` fields, type/range)<br>• `ExternCall` / `StandardIo` / `ComposedStdIo`: `Q022`–`Q027` (bridging & `.qr` helpers), `Q028`–`Q030` (lines & `Mut`/helper effects), `Q043` (undetermined C result rendering) | Never publishes plain `Safe`; `unsafe-ffi` and per-callee `linked:NAME` premises stack and propagate across all §L21.2 joins. |
| **Usable nested & mutable records**<br>`W48-QR-USABLE-RECORDS` | • `StandardBuffer` (`source.buffer.v1`): `Q051`–`Q054` (non-recursive records, sub-record read/write rules, depth `<= 8` / leaves `<= 256`, no live-field refill, no post-move/post-drop scalar access on owning records)<br>• `OrdinaryTypes` (`source.scalar-calls.v1`): `Q020` (§G02s pure value records)<br>• `source.ffi.v1`: `Q025`–`Q026`, `Q029`, `Q032`, `Q051`–`Q053` (§G02u field mutation & `.qr` helpers) | **Lifts** `AGGREGATE-NESTED`'s by-name refusal on `source.buffer.v1`; tightens post-`drop` scalar field access on owning records in the refusing direction (`Q054`). |
| **Bounded resource helpers (`source.resource-calls.v1`)**<br>`RESOURCE-HELPER-FUNCTIONS` | • `OrdinaryTypes`: `Q020` (helper shapes, complete headers, finite expansion bounds, empty contracts), `Q021` (nonempty contracts)<br>• `StandardBuffer`: `Q053` (acquisition extents before value storage), `Q054` (custody/aliasing), `Q056` (certificate build/recheck), `Q057` (derived effects)<br>• `Usage`: `Q100` (explicit usage grades) | Refusal categories are carried as typed Rust variants — never inferred from English message substrings. `Q058` remains exclusively `Event`-owned. |
| **Fixed-width scalars & IEEE 754 floats**<br>`W48-QR-SCALAR-TYPES` (`R1`–`R6`), `W48-QR-FLOATING-POINT`, `W48-QR-FLOAT-BUFFER-PAYLOADS` | • `NameResolution`: `Q013` (reserved `ArithmeticError`; conversion type names skipped so downstream routes decide)<br>• `OrdinaryTypes`: `Q020` (fixed-width & float typing, conversions, `isize`/`usize` target gate, `partial_eq`/`partial_ord`, `extern fn` ABI vocabulary)<br>• `ExternCall` / I/O: `Q024`–`Q025`, `Q028`, `Q030`, `Q043`, `Q050`<br>• Entry-only non-entry function rejection (`-R2`): `Q030`, `Q051`, `Q058`, `Q068`, `Q080`, `Q090`<br>• `StandardBuffer` / `StandardString` / `Loop`: `Q051`–`Q053`, `Q077`, `Q098` (`bool`, float & target-width loop bounds)<br>• `TypedErrors` / `CEmission` / `Concurrency`: `Q300` (`throws<ArithmeticError>`, `Buffer<T>` error payloads), `Q500` (multi-op task values, mismatched `Buffer<T>` writes), `Q600` (task value re-decision, `Buffer<T>` bounds, `protected isize\|usize`) | **Four deliberate narrowings:** (1) `Q033` & `Q020` admit `f32`/`f64` in extern/unchecked ABI signatures (updating message text only on 4 pinned survey/probe fixtures); (2) `Q077` (`-R6`) admits conversions, value statements, and typed integer loops on composed string+buffer; (3) `Q600` (`-R6`) discharges `source.epoch.tasks.v1`'s blanket refusal of fixed-width task halves; and (4) F02 parser admits fixed-width task outcomes. |
| **Scoped-task records & spawn closures**<br>`W48-QR-TASK-RECORD-CAPTURES`, `W48-QR-CLOSURES` | • Task records: `Q020` (typing), `Q300` (`mut` effect), `Q050` (no record rendering), `Q500` (`bytes` fields & epoch-route records), `Q600` (leaf transferability, arm merge, `OverwriteOfLiveField`, unpacked return)<br>• Closures: `Q000` (syntax), `Q011` (duplicate capture/binder), `Q013` (uncaptured outer local), `Q038` (closure bars outside `spawn`), `Q600` (non-value captures & unsupported routes) | **Lifts** `AGGREGATE-TASK-CAPTURE` (`Q500`) on scoped tasks and **narrows** `Q600` single-lease tracking to per-resource so distinct `Buffer` fields of one record lease concurrently. Closures leave every existing program's diagnostic byte-identical. |

> **Implementation & Provenance.** `crates/qcore-frontend/src/frontend.rs` (`source_diagnostic`),
> `crates/qcore-verification/src/{driver,verify_dispatch}.rs`, `crates/qcore-verifier-host/src/lib.rs`,
> `crates/qcore-contract/src/{contract_inclusion,formal_binding}.rs`;
> [diagnostics rules](agents/diagnostics.md) and [W7 attribution report](reports/w7-diagnostic-attribution/README.md).

---

<a id="l22-required-conformance-and-regression-evidence"></a>

## L22. Required conformance and regression evidence

### 1. Independent Per-Stage Conformance Tracking

The Quirin implementation matrix MUST track every verification and compilation stage independently:

1. **Parsing** (structural vs. consumer-specific EBNF);
2. **Static semantic checking** (type, grade, custody, flow, effect, and concurrency checkers);
3. **Reference execution** (source/model reference interpreters);
4. **CFG and IR lowering**;
5. **Native C11 compilation and execution** (target- and OS-qualified); and
6. **Mechanized Lean 4 proof** (model correspondence and soundness theorems).

Passing a standalone IR unit test NEVER completes a source-language concurrency-test (`CT`) case, and passing a native execution test on one host OS or C toolchain NEVER covers another target triple or backend.

---

### 2. Original Integration Checkpoint Baseline vs. Live Repository Validation

* **Original revision-0.3 integration checkpoint (`CT` baseline):** At this specification's initial integration checkpoint, the `CT` manifest recorded **11 mapped cases** — `CT01`, `CT02`, `CT12`, `CT13`, `CT14`, `CT15`, `CT16`, `CT19`, `CT20`, `CT37`, and `CT50` — alongside **39 unmapped cases**, over a 58-module `.qcore` corpus and a 5-module `resource-v1` contract selection (compared against 56 modules in the originally reviewed archive):
  * **`CT01` & `CT02`:** `CT01` executes owned `Buffer` transfer, worker return, parent `join`, and `drop`; `CT02` rejects parent post-`move` access.
  * **`CT12`, `CT13`, `CT15`, `CT19`, & `CT20`:** `CT12` executes spawn failure restoring the parent's original `Buffer`; `CT13` provides Linux native spawn-recovery and retained-sibling evidence; `CT15` provides Linux native join-failure evidence; `CT19` checks sibling-failure source programs; and `CT20` enforces sibling join and result discharge prior to bound-error rethrow.
  * **`CT16`, `CT14`, & `CT37`:** `CT16` executes native worker-local allocation and error cleanup; `CT14` and `CT37` force worker-body completion before `spawn` returns, verify retained completion-capsule storage, require explicit `join` and owned-result consumption, and reject both pre-join parent access and discarded owned results (with earlier sequential `CT16`/`CT37` controls retained).
  * **`CT50`:** Executes real OS memory unmap/remap at an identical virtual address with distinct `Buffer` lifetime generations and rejects stale source bindings.
  *(Individual per-stage and per-target rows in the current matrix remain authoritative; mapping a `CT` scenario never implies full end-to-end target-language proof.)*
* **Archive review vs. live repository execution:** Whereas the externally supplied review archive contained only 39 Python evidence-validator tests, an empty Libevent directory, and zero Rust, Lean, or native binary runs, this repository maintains complete pinned third-party inputs and executes separate Rust, Lean, Python, and configured C-audit gates. Live status and checkpoint evidence are maintained in [VALIDATION.md](VALIDATION.md) and the [acceptance ledger](reviews/r0_3/REVIEW.md); historical runs are never substituted for fresh gate execution.

---

### 3. Mandatory Revision-0.3 Regression Suite

Before claiming revision-0.3 conformance, the acceptance ledger requires dedicated regression coverage across all nine core boundaries:

1. **Destroyed-generation access** (`R01`);
2. **Post-join task events** (`R02`);
3. **Mutex-bound footprint authority** (`CheckedMutexInvariantV1`);
4. **Ordinary source `Result` / typed-error return types**;
5. **Unconditional nominal dimension/axis validity checking** (`Q016`, `R05`);
6. **Exact (`Exact(n)`) versus upper-bound (`AtMost(n)`) loop custody** (`Q200`, `R06`);
7. **Signed-minimum 32-bit integer literal handling** (`-2147483648` / `i32::MIN`);
8. **Explicit, total contract-consumer selection**; and
9. **Protocol and evidence-version mismatch rejection** (`QP004`, `QP009`).

Every subsequent change MUST preserve the fail-closed verifier-plugin certificate firewall (`QP001`–`QP016`) and MUST NOT silently promote Cycle 19 raw C units without audited evidence.

> **Implementation & Provenance.** [VALIDATION.md](VALIDATION.md), [acceptance ledger](reviews/r0_3/REVIEW.md),
> `docs/STATUS.md`, `crates/qcore-verification/tests/`, `proof/QCore/`.

---

<a id="l23-change-record"></a>

## L23. Change record

* **Revision 0.3 baseline (2026-09-07):** Introduces explicit capability and evidence levels (§L02); documents bounded `.qr` source-to-C profiles rather than claiming a monolithic target compiler (§L14); specifies separate consumer and interpreter grammars ([implemented grammars](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md)); clarifies multi-file module composition and closed quantitative indices (§L04, §L06); restores the full four-component `(Δ, R, P, T)` resource-flow tuple including minimum entry requirements `R` (§L10); separates exact (`Exact(n)`) from upper-bound (`AtMost(n)`) loop counts (`R06`); formalizes residual custody on fallible cleanup (§L13); narrows concurrency claims to implemented trace, mutex, and scoped-task models (§L16); records review gaps in the [acceptance ledger](reviews/r0_3/REVIEW.md); preserves permanent protocol-v1 `Safe` refusal (`QP009`); and assigns every diagnostic range to a single owning semantic pass (§L21).
* **String-view and quiet-stream amendments (2026-09-11):**
  * **Quiet C11 emission & composable logger streams (`QUIET-EMISSION-STREAMS`):** Generated C11 binaries emit nothing unless source code explicitly performs I/O; stdout and logger streams compose pointwise; and `std.logger` provides six severity levels plus entry-level `log_level` trace configuration (§L14.D, §L21 `Q058`).
  * **Immutable UTF-8 string views (`STRING-SLICE`, `source.string.v1`):** Adds allocation-free, custody-free UTF-8 `{ ptr, len }` views and code-point-checked slicing (§L04.1, §L14.B, `Q068`–`Q073`), keeping mutable `{ ptr, len, cap }` byte slices, runtime string materialization, and C04/native-library ABI crossings as explicit follow-ons (with subsequent static `STRING-SLICE-IO` and `STRING-CONCAT` / `borrow_utf8` increments documented in §L14.B and §L21 `Q075`–`Q079`).
* **Source/manifest suffix hygiene (`EXT-01`) and subsequent wave amendments:** `EXT-01` introduces `.qr`, `.qv`, `.qrlib`, and `.qo` canonical suffixes while retaining warned legacy aliases, extending `Q039` and `Q403` to reject source/checkpoint suffixes passed as `.qrlib` manifests or simultaneous legacy+current alias selections without rewriting exact manifest references/hashes or falling back across suffixes. Later incremental waves — covering native library manifests and C-struct FFI (§L14.1–§L14.8), fixed-width scalars and IEEE 754 floats (§L14.0), records, range write lends, loops, sockets, HTTP/1.1 (§L14.B–§L14.D), spawn closures (§L16.4), Existing-C fallback packages (§L18), decomposed `quant` hypotheses (§L19.1), and the four-value `Safe < Pending < Unknown < Unsafe` verdict lattice and `⊕` audit composition (§L20–§L21.2) — are recorded inline in their respective normative sections and in `docs/reports/`.
* **Archive reconciliation provenance:** The supplied external review archive modified documentation only and omitted its advertised probe files and validation bundle; no execution of absent archive artifacts is claimed, historical checkpoint evidence is preserved intact, and all merged implementation repairs are independently tested in this repository.

> **Implementation & Provenance.** [acceptance ledger](reviews/r0_3/REVIEW.md), [VALIDATION.md](VALIDATION.md),
> `docs/design/QUIET_EMISSION_STREAMS_AMENDMENT.md`, `docs/design/STRING_SLICE_V1.md`, `docs/reports/`.
