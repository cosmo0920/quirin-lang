# Checker generalization plan

Date: 2026-09-06. Status: proposed execution plan. Audience: implementation
agents (Opus/Sonnet-class) and reviewers. This plan reviews the current checker
architecture, names where it is over-fitted to the pinned source sets, sets a
pluggable-system decomposition target for the monolithic crate, and orders the
corrective workstreams. It does not weaken any Cycle 19/20 safety gate or the
[reevaluation](REEVALUATION_SPEC.md) verdict contract.

## 1. Architecture review: what is generic, what is not

Measured against the current tree (`compiler/src` = 23,804 lines):

**Genuinely generic and load-bearing (~14,000 lines).** The Cycle 5–17
semantic core (usage, ownership, quant indices, typed errors, error regions,
whole-program, CFG, weak memory, concurrency, lowering) has zero third-party
coupling. The unrestricted-C checker `compiler/src/legacy_c.rs` (2,726 lines)
is also fully generic: tokenizer, structured-subset parser, CFG builder,
may-alias resource abstract interpreter, Clang gateway, and independent
certificate recheck, taking contracts through one clean interface,
`ResourceContracts` (`legacy_c.rs:86-111`).

**Generic substrate that exists but is not load-bearing.**

- A real declarative contract language: `contract/qcore/**.qcore` (15 files,
  689 lines) with `resource type`, `extern fn ... effects {...} contract
  {...}`, and two working interpreters
  (`compiler/src/contract_observation.rs`, `compiler/src/chunk_contract.rs`).
  W1's first execution (2026-09-06) added a total-or-reject v1 loader,
  `compiler/src/contract_loader.rs`, and the audit example now loads its
  resource contracts through a manifest
  ([parity record](reports/cycle19-contract-loader/README.md)), and the
  leftover test/example literal sites now load `.qcore` fixtures through the
  loader ([literal-site record](reports/w1-literal-sites/README.md)). Richer
  observation/lifecycle predicates still use separate interpreters and no
  loader feeds `LibrarySummaryProfile`.
- A full Cycle 20 plugin protocol (`compiler/src/verifier_plugin.rs`, 1,567
  lines): versioned manifests, capability matching, process isolation,
  certificate firewall, composition. But `driver.rs:199-211` rejects every
  resolved `verify scope` with `Q701` — obligation generation and dispatch are
  unimplemented (finding VP05), and protocol v1 carries only Boolean
  assertions, so the core rightly refuses all v1 `Safe` proposals.
- A generic mechanism vocabulary `LibraryMechanismSummary`
  (`external_integration.rs:467-514`: ReferenceCount, IntrusiveList,
  TypedQueue, Arena, CascadingDestructor, AtomicPublication, ThreadHandoff,
  Reclamation) and a generic custody algebra (`external_integration.rs:19-456`).
- A generic conformance corpus CT01–CT50 (spec §55) with machine-readable
  tracking (`docs/reevaluation/ct_manifest.json`) — currently **100% unmapped**.

**Over-fitted (~5,300 lines of Cycle 18–20 integration).** Concrete hotspots,
ranked:

| Hotspot | Location | Nature |
|---|---|---|
| Literal source-fragment matcher | `compiler/src/external_source.rs` (whole file) | Pinned upstream paths with versions baked in (`:9-19`), closed `AuditedLibrary` dispatch (`:42-68`), "verification" is `contents.contains(fragment)` over exact upstream substrings (`:114-241`) |
| Closed library enum + per-project arms | `compiler/src/external_integration.rs:458-912` | 3-variant `AuditedLibrary`, per-library `permits` arms (`:479-507`), hardcoded C function-name tables (`:653-679`), compile-time-constant Monkey callback audit results (`:722-764`, `:817-908`) |
| Libevent tables | `compiler/src/verifier_plugins/libevent/event_driven.rs` | 48-entry API name table (`:108-157`), pinned commit + clean-tree + 12-file × ~40-needle substring audit (`:1482-1612`) |
| Monkey trio | `verifier_plugins/libevent/monkey_{source,binding,observation}.rs` (2,080 lines) | Closed 3-callback list, hardcoded record names, callee allowlists, literal type-string tests, ghost-name synthesis table |
| ChunkIO field importer | `verifier_plugins/chunkio/chunk_source.rs:119-124` | Exactly two struct/field pairs supported |
| Audit orchestration as an example | `compiler/examples/cycle19_fluent_bit_libraries_audit.rs` (966 lines) | The only production contract table, compile-db ingest, per-library iteration, `include_str!`-bound contracts, hardcoded 12-path pin check. Pointing the checker at a new codebase today means writing a new ~900-line example, not supplying configuration |
| First-party manifest literals | `compiler/src/verifier_plugin.rs:1436-1521` | Closed manifest/registration list |

**Structural finding: the system is one monolithic crate.** The workspace has
a single member; `qcore-compiler` compiles the semantic kernel, the QCore
front end, the C gateway, the plugin host, every first-party provider, the
contract interpreters, the audit orchestration, and the CLI into one library
and one binary. Consequences: providers are "plugins" in protocol only — they
share the core's private types and compile into the trusted binary; the trust
boundary Cycle 20 specifies (core firewall vs. untrusted provider) has no
compilation-unit enforcement; any library-specific change rebuilds and
re-risks the core; and third parties cannot ship a provider without forking
the compiler. Generalization and de-monolithization are therefore the same
project: the plan below treats **pluggability as a requirement of every
workstream**, not a final refactor.

**Interpretation.** The project does not need a new architecture. Cycle 20
already specifies the correct one (core owns obligations, identity, firewall,
composition; plugins own library semantics), and the reevaluation spec already
states the correct trust rule (*"The core MUST derive an obligation inventory
from the normalized program. Plugins may propose witnesses, not the list of
obligations whose absence would make their own proof succeed."*). What is
missing is the connective tissue that makes the generic substrate load-bearing
— a contract loader, data-driven source-set profiles, obligation
generation/dispatch, interprocedural summary inference (LC01's remaining
boundary), a generic acceptance corpus — **plus the physical decomposition
that makes the plugin boundary real**. That is what this plan orders.

## 2. Invariants that every workstream must preserve

1. **Fail-closed everywhere.** Unknown, never optimistic fallback, for:
   missing/unloadable contracts, unmatched profiles, unsupported constructs,
   provider absence/failure/mismatch, and unproven call effects. No workstream
   may convert an `Unknown` to `Safe` as a side effect of refactoring.
2. **Reevaluation gate ordering** (`REEVALUATION_SPEC.md` §9): G1 certificate
   containment before expanding provider coverage; Cycle 19's existing
   analysis routes are removed only after G4 parity is recorded.
3. **Parity before deletion** (Cycle 20C rule): each extraction must reproduce
   the previous pinned verdicts, coverage tables, and diagnostics before the
   hardcoded path is removed, with the comparison archived under
   `docs/reports/`.
4. **Verdict contract**: every result carries `(artifact, property, semantic
   profile, scope, bounds, assumptions, evidence version)`; projection
   percentage is never presented as safety percentage; unknowns stay in the
   coverage denominator.
5. **Standing per-change gate**: `cargo test --workspace`, `cargo fmt --check`,
   Clippy `--all-targets -- -D warnings`, `lake build` (placeholder-free, no
   project axioms in audits), and the existing probe examples
   (`reevaluation_probe`, `reevaluation_followup`) keep passing.
6. **Trusted input stays declared.** Contract/profile files are *trusted
   assumptions*, and certificates must record which ones were assumed. Moving
   a table from Rust into data does not make it verified; it makes it
   declarable, diffable, and per-source-set.
7. **Pluggability direction of dependencies.** New and moved code must respect
   the target layering of §3: providers and profiles depend only on the
   published schema surface, never on core internals; the core never depends
   on any provider; the CLI and audit driver depend on the host, not on
   individual providers. A workstream that would violate this must change the
   schema surface (versioned) rather than reach across the boundary.

## 3. Pluggability target: from monolithic crate to layered workspace

### 3.1 Target crate layout

Decompose the workspace into layers with dependencies pointing strictly
downward. Names are suggestions; the layering is the requirement.

```
crates/qcore-schemas          # L0: the published, versioned surface
  verdict lattice, diagnostics model, obligation inventory,
  evidence/witness schemas (v1 refusal + v2), plugin protocol frames,
  manifest + profile formats, contract data model (ResourceFunctionContract),
  source/build identity records.  serde types only, no analysis logic.
crates/qcore-core             # L1: trusted semantic kernel
  usage, ownership, quant indices, typed errors, error regions, CFG,
  weak memory, concurrency, custody algebra and pure model checkers.
crates/qcore-frontend         # L1: front end, elaboration + whole-program checking
crates/qcore-contract         # L1: .qcore contract parsing, loader,
  contract/observation interpreters (contract_observation, chunk_contract).
crates/qcore-c-gateway        # L2: legacy_c, clang discovery, typed_c_ast,
  interprocedural summary inference (W4), opaque typed Boolean-field source import.
crates/qcore-verifier-host    # L2: registry, discovery, process provider,
  composition and certificate admission orchestration (W3).
crates/qcore-verification     # Trusted adapter above L1 + C gateway + verifier host:
  obligation derivation, witness replay, scope dispatch. No provider dependency.
  Keeps gateway/host/frontend/contract/schema integration out of the dependency-free kernel.
crates/qcore-audit            # L3: orchestration-as-configuration (W5):
  compile-db ingest, pin verification, per-unit iteration, report emission.
crates/qcorec                 # L3: CLI binary.
providers/qcore-provider-*    # out-of-tree layer: one crate per provider
  (libevent+monkey, chunkio, cmetrics, ctraces, librdkafka), each buildable
  as a standalone executable speaking the qcore-schemas protocol; in-process
  adapter kept only during migration.
contract/                     # data layer: .qcore contracts + profile
  manifests, shipped with providers or user-supplied.
proof/                        # unchanged location; theorem families referenced
  by manifests via qcore-schemas identifiers.
```

Rules:

- **`qcore-schemas` is the only crate providers may depend on.** It is
  semver-versioned independently; changing it is a compatibility event with
  the same discipline as protocol v1 negotiation. Everything in it must be
  serializable — if a type cannot cross a process boundary, it does not
  belong there.
- **The trust boundary is a process boundary, eventually.** During migration,
  first-party providers may run in-process behind the same request/response
  types, but they must lose access to core internals at the crate split — the
  compiler enforces what code review currently has to.
  *Decision (2026-09-06): providers remain compile-time crates speaking the
  same schema types in-process; the out-of-process step is deferred, not
  abandoned. Schema types stay process-portable so the deferral costs no
  schema rework.*
- **The core must build and test with zero providers present**, and the full
  first-party set must be reproducible as separate artifacts (Cycle 20D's
  packaging gate).

### 3.2 Migration strategy

Big-bang decomposition of a 23.8k-line crate is high-risk and would freeze the
other workstreams; a pure "refactor at the end" repeats the monolith. The plan
uses a thin early split plus per-workstream placement:

1. **W0 (below) extracts only L0 + the CLI**: `qcore-schemas` and `qcorec`.
   This is mostly type movement and re-exports; the existing `qcore-compiler`
   crate temporarily becomes the L1/L2 grab-bag and re-exports schema types
   for compatibility.
2. Every subsequent workstream **lands new code in its target crate** and
   moves the modules it touches (W1 → `qcore-contract`, W3 →
   `qcore-verifier-host`, W4 → `qcore-c-gateway`, W5 → `providers/*` and
   `qcore-audit`). No workstream adds code to the grab-bag.
3. A closing consolidation step (W5, task 5) renames the emptied
   `qcore-compiler` remainder into `qcore-core`/`qcore-frontend` and deletes
   the compatibility re-exports.

This keeps each move reviewable, keeps `cargo test --workspace` green at every
step, and means the crate graph converges on §3.1 as a by-product of the
functional work rather than a separate risky project.

## 4. Workstreams

Ordered by leverage and dependency. W0 comes first and is short; W1–W3 are
then independent of each other and can proceed in parallel; W4 depends on W1;
W5 depends on W1–W3; W6/W7 are continuous.

### W0 — Workspace layering: extract the schema surface and the CLI

**Status.** Executed (2026-09-06): see the
[W0 schema-crate record](reports/w0-schema-crate/README.md). All four tasks
landed; the standing gate passed unchanged.

**Goal.** Make the plugin boundary compiler-enforced from the start: a
standalone `qcore-schemas` crate holding every type that crosses the
core/provider or process boundary, and a `qcorec` binary crate that depends on
the library rather than living inside it.

**Tasks.**
1. Create `crates/qcore-schemas` and move (with re-exports from
   `qcore-compiler` for compatibility): the protocol v1 frame/manifest/
   request/response types and capability/target declarations from
   `verifier_plugin.rs`; the verdict and diagnostic model types; the
   `ResourceFunctionContract`/`ResourceContracts` data model from
   `legacy_c.rs:86-111` (types only — the analyzer stays); the
   `LibraryMechanismSummary` vocabulary; source/build identity and pinned-
   revision record types. Everything moved must be serde-serializable and
   free of analysis logic.
2. Move `src/main.rs` into `crates/qcorec`; the workspace gains three
   members. No behavior change; CLI output byte-identical.
3. Document the schema crate's compatibility policy in its README: semver,
   what constitutes a breaking change, and the rule that providers depend on
   this crate only.
4. Add a CI/test guard that fails if `qcore-schemas` grows a dependency on
   any other workspace crate (dependencies point downward only).

**Acceptance.** Workspace builds with three members; all tests, probes, and
Clippy pass unchanged; `qcore-schemas` has no path dependencies on the rest of
the workspace.

**Agent guidance.** Sonnet-class; mechanical but wide. Resist scope creep:
W0 moves types, never logic. If a type turns out to entangle logic (e.g.
methods doing analysis), split the type from its impl rather than moving the
logic.

### W1 — Contract loader: make `.qcore` the single source of contracts

**Status.** Executed (2026-09-06). `compiler/src/contract_loader.rs`
implements the total-or-reject v1 vocabulary and the Cycle 19 audit consumes
a contract manifest — see the
[contract-loader parity record](reports/cycle19-contract-loader/README.md).
The relocation into `crates/qcore-contract` is executed (2026-09-06, with
`crates/qcore-core` and `crates/qcore-frontend` extracted beneath it so the
loader sits below the certificate recheck — see the
[W1 relocation record](reports/w1-contract-crate/README.md); `chunk_contract`
follows once the W5 task 3 chunk-summary schema split lands). The leftover
literal sites are migrated (2026-09-06): tests and examples load `.qcore`
fixture modules through the loader, and only the loader's own mapping tests
still construct literals — see the
[literal-site record](reports/w1-literal-sites/README.md). The richer clause
vocabulary is closed (see the
[clause-vocabulary record](reports/w1-clause-vocabulary/README.md)): the
`requires resource_contract_v1` marker is pinned corpus-wide as the
interpreter selector (19 declarations load, 141 are rejected rather than
skipped), the clause-family → owner table is documented in the loader header
and the corpus guide, and the C ABI scalar type table is complete. Unifying
the resource loader with the observation/lifecycle interpreters is
**W5-adjacent** — it depends on the W5 task 3 chunk-summary schema split —
and non-null error sentinels need target-interface work in `qcore-schemas`
plus the analyzer, not loader vocabulary.

**Goal.** One loader from `contract/qcore` contract modules to the analyzer's
`ResourceContracts`, eliminating every Rust-literal production contract table.

**Why first.** It is the highest-leverage, lowest-risk step: the language, the
parser (`frontend.rs` already elaborates contract clauses), and the consumer
interface all exist; only the bridge is missing. Everything downstream (W2,
W4, W5) wants contracts as data.

**Tasks.**
1. Add the loader in `crates/qcore-contract` (new home for
   `contract_observation.rs` and `chunk_contract.rs` too): parse a
   `module contract;` file via the existing front end and map each `extern fn`
   declaration to a `ResourceFunctionContract` (parameter modes
   Borrowed/Owned/Consumed/Released/Out/Stored/Removed × nullability, return
   Owned/Reallocated, error sentinels). Define the mapping from
   contract-clause vocabulary to `ResourceApiKind`/modes explicitly; any
   clause the mapping cannot express → that function is rejected with a
   diagnostic (fail-closed), not silently approximated.
2. Migrate the production table in
   `cycle19_fluent_bit_libraries_audit.rs:141-207` into
   `contract/qcore/` files (e.g. `contract/qcore/libc/memory.qcore`,
   `monkey/memory.qcore`, `librdkafka/memory.qcore`,
   `fluent_bit/memory.qcore` extensions, `chunkio/chunk_open_close.qcore`),
   and load them at runtime instead of `include_str!`/Rust literals.
3. Update `compiler/tests/qcore_contract_corpus.rs` expected-count tables;
   add loader unit tests: round-trip (loaded contract ≡ previous literal),
   unsupported-clause rejection, missing-file behavior, duplicate-function
   conflict behavior.
4. Parity check: rerun the Cycle 19 audit example with loaded contracts and
   diff verdict/coverage JSON against the current archived report
   (`docs/reports/cycle19-resource-triage/`); archive the comparison.

**Pluggability deliverable.** Contract files become a shippable unit: a
provider or user can supply a directory of `.qcore` contracts without
touching Rust. The loader consumes and produces only `qcore-schemas` types,
so the same loader serves in-process use and future out-of-process providers.

**Acceptance.** No production `ResourceContracts` literal remains outside
tests; audit verdicts byte-identical (or every difference explained and
recorded); corpus test still pins per-file clause counts.

**Agent guidance.** Sonnet-class is sufficient; mechanical but detail-heavy.
The one design decision needing care is the clause→mode mapping table — write
it down in the module header and keep it total-or-reject.

### W2 — Source-set profiles as data: replace closed enums and fragment tables

**Goal.** A source-set *profile manifest* format so that adding a library
means adding files, not editing Rust.

**Tasks.**
1. Define a versioned profile manifest (schema types in `qcore-schemas`;
   files under `contract/profiles/<source-set>/profile.*`) carrying what is
   today hardcoded: pinned revision(s), audited file paths, expected source
   fragments/digests, permitted `LibraryMechanismSummary` set, function-name
   → mechanism-summary table (the `required_chunk_summary` names,
   `external_integration.rs:653-679`), API disposition tables (the 48-entry
   Libevent profile, `event_driven.rs:108-157`), and required
   evidence/observation kinds. Prefer **file digests over embedded source
   substrings** for pin checks; keep substring needles only where a digest
   cannot express "this specific construct is present", and record why.
2. Replace `enum AuditedLibrary` and `external_source.rs`'s fragment tables
   with profile loading. `derive_library_summary_profile` takes a manifest,
   not a match arm. Unknown profile / failed pin → Unknown, as today.
3. Replace the first-party manifest literals
   (`verifier_plugin.rs:1436-1521`) with the same on-disk manifests routed
   through the existing package-discovery path, keeping deterministic
   registry ordering.
4. Migrate the Fluent Bit / librdkafka / CFL / ChunkIO / Monkey / Libevent
   data into profiles; parity-diff all pinned audit outputs before deleting
   the Rust tables (invariant 3).

**Pluggability deliverable.** A profile directory plus a contract directory is
the complete definition of a source set. Profiles are distributable alongside
provider packages and are validated by the host against `qcore-schemas`
alone — a third party can define a new source set with zero forked code.

**Acceptance.** `rg 'rd_kafka|cio_|flb_|mk_|evutil|cfl_'` in core crates
matches only: (a) profile-loading code comments, (b) the bounded semantic
models pending W5 extraction. Adding a hypothetical new library touches zero
Rust files in the core.

**Agent guidance.** Sonnet-class for the migration; have an Opus-class pass
review the manifest schema before mass migration (it becomes a compatibility
surface — version it from day one, like protocol v1).

### W3 — Core obligation generation and verify-scope dispatch (closes VP05)

**Goal.** `verify scope` declarations actually drive verification: the core
derives an obligation inventory from the normalized program and dispatches
requests; plugins propose witnesses against *core-chosen* obligations.

**Tasks.**
1. Define the obligation inventory in `qcore-schemas`: per capability
   (resource contracts, chunk lifecycle, event traces, concurrency,
   translation-unit projection), what obligations the core derives from a
   normalized translation unit / typed program (e.g. per-function custody
   discharge obligations, per-resource teardown obligations, per-callback
   contract obligations). Follow the SV-LIB direction already recorded:
   version the complete verification task, not just the response envelope.
2. Specify evidence schema v2 (also in `qcore-schemas`):
   capability-specific semantic witnesses the core can replay (not Boolean
   assertions). Start with the two capabilities that already have core
   checkers — chunk lifecycle (`chunk_lifecycle::check_chunk_program`) and
   contract observations (`contract_observation`) — so v2 `Safe` promotion is
   real from the first provider. Keep v1 refused, as today.
3. Implement dispatch in the verifier host (new crate
   `qcore-verifier-host`, absorbing `verifier_plugin.rs`'s registry/process/
   composition halves; the firewall's per-capability checkers belong to the
   trusted core side): resolved scope → obligations → registry requests →
   firewall-checked responses → fail-closed aggregation into compile
   diagnostics. Provider absence, timeout, or verdict `Unknown` on any
   obligation keeps the scope unverified with today's diagnostics. Resolve
   the `Q700/Q701` collision with spec §54's Libevent range while touching
   this (see W7).
4. Tests: nonempty obligations with unavailable/unknown/rejecting/conflicting
   providers; a counting provider proving dispatch happens; a v2 witness
   round-trip through the firewall; regression that v1 `Safe` is still
   refused. Add one **out-of-process** conformance test: a trivial external
   provider binary (test-only) that speaks v2 over stdio, proving the schema
   crate suffices to implement a provider with no other workspace dependency.
5. Lean: extend `QCore/VerifierPlugin.lean` from Boolean evidence to the v2
   witness relation for the first capability, addressing FM01's narrowness in
   that scope (the theorem should be about the checked relation, not a flag).

**Pluggability deliverable.** The complete provider ABI —
obligations + requests + witnesses + verdicts — lives in `qcore-schemas` and
is exercised by an external binary in CI. From this point, "plugin" means a
process, not a module.

**Acceptance.** A source file with a `verify scope` over a supported
capability gets a real, firewall-checked verdict; all VP01–VP08 probe
expectations still pass; the reevaluation §6 evidence-contract sentence is
implemented, not just quoted; the out-of-process conformance provider passes.

**Agent guidance.** Opus-class. This is the trust-boundary workstream; the
failure mode is designing evidence that restates the request (VP01/VP02's
root cause). Every schema decision should be checked against: "could a
malicious provider make the core's check vacuous?"

### W4 — Interprocedural custody summary inference (LC01's open boundary)

**Status.** Executed for parameter/return custody with validation (2026-09-06):
see the [W4 custody-inference record](reports/w4-custody-inference/README.md).
`crates/qcore-c-gateway/src/legacy_summary.rs` derives SCC-ordered summaries, validates each
by re-running the intraprocedural analyzer against it, publishes them as
`qcore-schemas` `ResourceSummaryRecord`s, and has the certificate recheck
re-derive them independently. Declared contracts always win; a disagreement is
a `SummaryConflict` diagnostic. The [2026-09-07 extension](reports/w4-custody-extensions/README.md)
adds validated direct `Out` slots, `Stored` formals, `Reallocated` results and
possibly-null owned returns. [Direct Removed wrappers](reports/w4-removed-custody/README.md)
now validate contained entry and detached/live exits with matching Lean relations.
[Direct Out forwarding](reports/w4-out-forwarding/README.md) now validates
original slots and invalidates mutation/shadow/alias identity assumptions.
Remaining: general removal aliases and nullable removal, general out-slot
aliases, general recursive components and the Juliet stretch corpus.
[Context-specific recursion](reports/w4-contextual-recursion/README.md) now
validates exhaustive zero/nonzero integer controls using strictly lower-rank
base summaries, including bounded mutual recursion. Arithmetic rankings and
general recursive fixed-point inference remain open. The [gateway relocation](reports/w4-gateway-crate/README.md)
is executed (2026-09-07): `legacy_c`, `legacy_summary`, Clang discovery and
typed-AST transport now live in `qcore-c-gateway`, with compiler compatibility
reexports and pinned full-corpus parity.

**Goal.** The generic capability whose absence forces per-library hand
contracts: infer and validate parameter/return custody summaries for internal
functions of a translation unit, so wrappers around contracted APIs are
analyzed instead of Unknown.

**Tasks.**
1. In the C gateway (target crate `qcore-c-gateway`, absorbing `legacy_c.rs`,
   `clang.rs`, `typed_c_ast.rs`), add a bottom-up (SCC-ordered) summary pass
   over the already-built per-function CFGs: infer each internal function's
   parameter modes and return ownership from its body given the contract
   environment; validate the summary by re-running the intraprocedural
   analyzer against it. Recursion: require a consistent fixed point or stay
   Unknown.
2. Apply inferred summaries at call sites with identity and footprint
   transfer, mirroring what `whole_program.rs` already does for QCore source
   (§22.3's summary shape is the target). Explicit `.qcore` contracts always
   take precedence over inference; a conflict between the two is a
   diagnostic, never a silent override (the LC01 lesson).
3. Keep the certificate recheck honest: `check_legacy_safety_certificate`
   must re-derive the same summaries independently.
4. Tests, from the LC01 repair list: allocation wrapper (the LC01
   counterexample must now be Unsafe-leak or Safe-with-discharge, never
   false-Safe), caller-discard, caller-release, nested wrappers, parameter
   consumption, recursion, out-parameters, realloc wrappers. Add the Juliet
   temporal-safety held-out cases direction from the reevaluation literature
   table as a stretch corpus.
5. Measure on the pinned corpus: report movement in the 2 Safe / 98
   provisional-Unsafe / 81 Unknown triage counts, per invariant 4 (report
   deltas per verdict class; do not advertise a coverage percentage as
   safety).

**Pluggability deliverable.** Inferred summaries are expressed as
`qcore-schemas` contract/summary records, so an inferred summary, a loaded
`.qcore` contract, and a provider-proposed summary are the same data with
different trust labels — providers can consume and refine gateway summaries
through the ordinary schema surface.

**Acceptance.** LC01's "remaining boundary" paragraph in
`REEVALUATION_FOLLOWUP.md` can be updated to "implemented for parameter/return
custody with validation"; no new false-Safe on the probe suite; triage deltas
archived under `docs/reports/`.

**Agent guidance.** Opus-class, highest reasoning effort of the plan. This is
real static-analysis design (mode inference + fixed points + aliasing
interaction with `MAX_ALIAS_TARGETS`). Insist on the "validated, not just
inferred" discipline the follow-up demands.

### W5 — Provider extraction and orchestration-as-configuration (Cycle 20C/20D)

[Source-field provider integration](reports/w3-source-field-provider/README.md)
now derives bounded tasks from opaque Clang imports and dispatches the real
ChunkIO package for three production function contracts. This closes that
source-effect exchange slice; general lifecycle effect import and provider-owned
full source analysis remain open. The [statement execution increment](reports/w5-provider-statement-execution/README.md) moves finite-state path computation into the actual process, without sending expected paths. The selected audit mode has no provider fallback,
and retains direct evaluation until broader composition parity is established.

**Status (2026-09-07).** Configured audit orchestration is extracted with a
second-codebase fixture and pinned parity. The sequential/concurrent kernel,
chunk lifecycle and generic loop/ledger checks reside in core; whole-program
checking resides in frontend. Declared chunk-path evaluation is in contract,
with portable schema0.2.0 records and an opaque source-import boundary.
Five standalone schema-only executables now reproduce the built-in v1 Unknown
responses and reject malformed requests. ChunkIO 0.3.0 now computes bounded
Boolean-field witnesses from typed statement inputs through v2; other v2
capabilities remain Unknown without witnesses. Two clean builds reproduce executable/manifest/
archive hashes. [Provider packaging evidence](reports/w5-provider-packages/README.md)
and [kernel integration](reports/w5-resource-accounting/README.md) record scope.

Task3's generic reactor lifecycle/descriptor model now lives in core with an
explicit profile policy and compatible Libevent facade ([evidence](reports/w5-generic-reactor/README.md)).
Task3's [callback records and declared-model exchange](reports/w5-reactor-callback-records/README.md)
are implemented with a schemas-only process fixture; source binding and callback
protocol witness admission remain open. Task2's specialized semantic extraction,
registry-based Fluent Bit composition, task4 route deletion and remaining
compiler consolidation remain open. [CLI package lifecycle](reports/w5-cli-packages/README.md)
now supports explicit verification and pinned enable/disable selection. Existing
in-process/source routes are retained until semantic parity meets these gates.

**Goal.** Finish moving library semantics behind the plugin boundary — as
separate crates and ultimately separate processes — and turn the 966-line
audit example into a reusable, configured driver.

**Tasks.**
1. Promote the orchestration in `cycle19_fluent_bit_libraries_audit.rs`
   (compile-db ingest, pin verification, per-unit iteration, verdict
   tallying, JSON report emission) into `crates/qcore-audit`, driven by a W2
   profile + W1 contracts; the example shrinks to argument parsing. A second,
   different codebase then needs only a profile directory — recreate the
   Cycle 17 CFL adjacent-checkout probe through this path as proof.
2. Move each provider to its own crate under `providers/`
   (`qcore-provider-libevent` including the Monkey trio,
   `qcore-provider-chunkio`, then new `cmetrics`/`ctraces`/`librdkafka`
   providers per the Cycle 20C order), each depending on `qcore-schemas`
   only, each buildable as a standalone executable speaking W3's v2 protocol,
   each with its own manifest, diagnostic namespace, tests, and parity
   record. Where a provider currently needs a core type that isn't in the
   schema crate, that is a schema-surface decision, not a license to link the
   core.
3. Split reactor-generic parts out before the move: the callback-summary
   schema, generation-indexed lifecycle, and descriptor-custody models go to
   `qcore-schemas`/`qcore-core`; only the Monkey/Libevent tables and
   importers go to the provider crate.
4. Only after each parity record: delete the corresponding hardcoded route
   (`external_source.rs` arms, `AuditedLibrary`, first-party literals, the
   in-process adapters).
5. Consolidation: rename the emptied `qcore-compiler` remainder into
   `qcore-core` + `qcore-frontend`, drop the W0 compatibility re-exports, and
   verify the §3.1 dependency rules hold across the whole workspace
   (including the W0 CI guard extended to every layer). Add the Cycle 20D
   lifecycle tooling: `qcorec plugins` package verification, enable/disable,
   and reproducible first-party provider packages.

**Pluggability deliverable.** This *is* the pluggability workstream's
end-state: core builds and passes its tests with zero providers; the five
first-party providers are separate artifacts; the Fluent Bit result is a
composition of packages plus data, reproduced through the registry.

**Acceptance.** Cycle 20's acceptance-gate list advances measurably: separate
manifests/tests/namespaces for all five providers; the Fluent Bit composition
profile reproduces Cycle 19's pinned verdicts through the registry; the
monolithic routing is gone; the workspace matches §3.1.

**Agent guidance.** Sonnet-class for extraction mechanics with the parity
harness from task 1 built first; Opus-class review at each deletion step and
at every schema-surface addition requested by a provider.

### W6 — Generic acceptance corpus: make CT01–CT50 the yardstick

**Goal.** Shift the test center of gravity from pinned-corpus regression to
the spec's own generic conformance corpus, so "generic" is measured, not
asserted.

**Tasks.**
1. Stand up a CT harness keyed to `docs/reevaluation/ct_manifest.json`:
   each CT case gets QCore source (and, where the case concerns the C path, a
   C twin), expected verdict/diagnostics, and per-stage status updates in the
   manifest. Hand-constructed Rust events do not mark a case complete
   (reevaluation §5.1 rule).
2. Map cases opportunistically per workstream: W4 lands the
   resource/custody CT rows; W3 lands the verdict/composition rows; the
   concurrency rows follow the existing checkers. Track the mapped count in
   the manifest — the number moving off "unmapped" is the plan's genericity
   metric.
3. Keep namespaces separate as specified: CT = language corpus; WM/VP/LC
   probes = regression namespace; per-library cases = provider test suites.
   After W5, per-library cases move out of the core's test tree into their
   provider crates — the core test suite must not name any third-party
   library.

**Acceptance.** A recorded, increasing `mapping_status` count with per-stage
evidence; no CT case marked complete without an executed source-level test;
core test suite free of third-party identifiers post-W5.

**Agent guidance.** Sonnet-class, batchable (cases are independent — this
fans out well across parallel agents).

### W7 — Hygiene: names, diagnostics, roadmap

The [Lean CI gate](reports/w7-lean-ci/README.md) and
[bounded generic model names](reports/w7-generic-lean-names/README.md) are
implemented and locally validated (2026-09-07); hosted CI awaits a push.
Historical model names remain compatibility aliases. Diagnostic renumbering
is a separate work item.

Small, continuous, do-with-adjacent-work:

- Resolve the `Q700–Q712` collision: spec §54 names Libevent-specific
  diagnostics in a numbered range that Cycle 20 reuses for `verify scope`
  resolution. Assign plugin-namespaced diagnostics (the `QP***` space already
  exists) for provider-specific conditions and reserve `Q7xx` once,
  recording the decision in the spec changelog. Provider-owned diagnostic
  namespaces become part of each provider's manifest (they already are in
  Cycle 20's design) — the core allocates no numbers for library-specific
  conditions.
- Rename library-named wrappers around generic models where cheap:
  `QCore.MonkeyEvent` (a generic descriptor-custody model), the
  `busy_effect_preserves_chunkio_lock` theorem, `MonkeyComposition` (keep
  aliases; Lean renames ripple into the audit files, so batch them). The
  generic/renamed Lean modules pair with `qcore-core`; provider-specific
  theorem families are referenced from provider manifests.
- Keep `docs/ROADMAP_AFTER_CYCLE_9.md` current: it now records Cycles 19–20
  and tracks this plan's W1–W7 execution; update its workstream table as
  records land, keeping its "no cycle skipped without updating the dependency
  order" rule.
- CI: add the Lean job the reevaluation flagged (EV01) if still absent, and
  the layering guard from W0 (no upward dependencies; `qcore-schemas`
  dependency-free within the workspace).
- Documentation split (done 2026-09-06, keep in sync): `AGENTS.md` is the
  compressed agent briefing (status ledger, invariants, per-change gate) and
  the source of truth for agents, imported by `CLAUDE.md`; `README.md` is the
  decompressed human document. A workstream that changes project status
  (verdict counts, gate closures, crate layout) updates `AGENTS.md` and
  `README.md` in the same change.

## 5. Sequencing summary

```
W0 schema crate + CLI split
        │
W1 contracts-as-data ──┬─▶ W4 custody inference ──┐
W2 profiles-as-data  ──┤                          ├─▶ W5 provider crates/processes,
W3 obligations+dispatch┘                          │     audit driver, consolidation
W6 CT corpus (continuous, consumes W3/W4 output) ─┘
W7 hygiene (continuous)
```

Suggested review checkpoints: after W0 (schema-surface inventory — what got
published is now a compatibility commitment); after W1 parity diff; after the
W2 manifest schema (before mass migration); after the W3 evidence-schema v2
design (before implementation); after the first W5 provider parity record;
after W5 consolidation (workspace layering audit against §3.1).

## 6. What this plan explicitly does not do

- It does not promote any pinned translation unit toward `Safe`; verdict
  movement comes only from W4's validated inference and is reported per
  invariant 4.
- It does not open the Cycle 19 fail-closed gates (per-path identity imports,
  observation evidence) — those remain Cycle 19's own acceptance work.
- It does not design a new plugin ABI; protocol v1 framing stays, evidence
  moves to v2 within the existing negotiation, and the crate decomposition
  packages the same protocol rather than replacing it.
- It does not treat contract/profile data files as verified truth; they are
  declared trusted assumptions carried into every certificate.
- It does not attempt OS-level sandboxing of providers beyond the existing
  process-group containment; memory/network isolation remains a recorded
  open boundary from the follow-up repair, orthogonal to this decomposition.
