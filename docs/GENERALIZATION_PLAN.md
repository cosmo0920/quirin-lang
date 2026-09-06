# Checker generalization plan

Date: 2026-09-06. Status: proposed execution plan. Audience: implementation
agents (Opus/Sonnet-class) and reviewers. This plan reviews the current checker
architecture, names where it is over-fitted to the pinned source sets, and
orders the corrective workstreams. It does not weaken any Cycle 19/20 safety
gate or the [reevaluation](REEVALUATION_SPEC.md) verdict contract.

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
  But **no loader wires `.qcore` into `ResourceContracts` or
  `LibrarySummaryProfile`** — every contract environment that actually drives
  a verdict is a Rust literal (production table:
  `compiler/examples/cycle19_fluent_bit_libraries_audit.rs:141-207`; ~20 more
  sites in tests).
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

**Interpretation.** The project does not need a new architecture. Cycle 20
already specifies the correct one (core owns obligations, identity, firewall,
composition; plugins own library semantics), and the reevaluation spec already
states the correct trust rule (*"The core MUST derive an obligation inventory
from the normalized program. Plugins may propose witnesses, not the list of
obligations whose absence would make their own proof succeed."*). What is
missing is the connective tissue that makes the generic substrate load-bearing:
a contract loader, data-driven source-set profiles, obligation
generation/dispatch, interprocedural summary inference (LC01's remaining
boundary), and a generic acceptance corpus. That is what this plan orders.

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

## 3. Workstreams

Ordered by leverage and dependency. W1–W3 are independent of each other and
can proceed in parallel; W4 depends on W1; W5 depends on W1–W3; W6/W7 are
continuous.

### W1 — Contract loader: make `.qcore` the single source of contracts

**Goal.** One loader from `contract/qcore` contract modules to the analyzer's
`ResourceContracts`, eliminating every Rust-literal production contract table.

**Why first.** It is the highest-leverage, lowest-risk step: the language, the
parser (`frontend.rs` already elaborates contract clauses), and the consumer
interface all exist; only the bridge is missing. Everything downstream (W2,
W4, W5) wants contracts as data.

**Tasks.**
1. Add `compiler/src/contract_loader.rs`: parse a `module contract;` file via
   the existing front end and map each `extern fn` declaration to a
   `ResourceFunctionContract` (parameter modes Borrowed/Owned/Consumed/
   Released/Out/Stored/Removed × nullability, return Owned/Reallocated, error
   sentinels). Define the mapping from contract-clause vocabulary to
   `ResourceApiKind`/modes explicitly; any clause the mapping cannot express
   → that function is rejected with a diagnostic (fail-closed), not silently
   approximated.
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
1. Define a versioned profile manifest (TOML or JSON under
   `contract/profiles/<source-set>/profile.*`) carrying what is today
   hardcoded: pinned revision(s), audited file paths, expected source
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

**Acceptance.** `rg 'rd_kafka|cio_|flb_|mk_|evutil|cfl_'` in `compiler/src`
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
1. Define the obligation inventory: per capability
   (resource contracts, chunk lifecycle, event traces, concurrency,
   translation-unit projection), what obligations the core derives from a
   normalized translation unit / typed program (e.g. per-function custody
   discharge obligations, per-resource teardown obligations, per-callback
   contract obligations). Follow the SV-LIB direction already recorded:
   version the complete verification task, not just the response envelope.
2. Specify evidence schema v2: capability-specific semantic witnesses the
   core can replay (not Boolean assertions). Start with the two capabilities
   that already have core checkers — chunk lifecycle
   (`chunk_lifecycle::check_chunk_program`) and contract observations
   (`contract_observation`) — so v2 `Safe` promotion is real from the first
   provider. Keep v1 refused, as today.
3. Implement dispatch in `driver.rs::compile_files_with_registry`: resolved
   scope → obligations → registry requests → firewall-checked responses →
   fail-closed aggregation into compile diagnostics. Provider absence,
   timeout, or verdict `Unknown` on any obligation keeps the scope
   unverified with today's diagnostics. Resolve the `Q700/Q701` collision
   with spec §54's Libevent range while touching this (see W7).
4. Tests: nonempty obligations with unavailable/unknown/rejecting/conflicting
   providers; a counting provider proving dispatch happens; a v2 witness
   round-trip through the firewall; regression that v1 `Safe` is still
   refused.
5. Lean: extend `QCore/VerifierPlugin.lean` from Boolean evidence to the v2
   witness relation for the first capability, addressing FM01's narrowness in
   that scope (the theorem should be about the checked relation, not a flag).

**Acceptance.** A source file with a `verify scope` over a supported
capability gets a real, firewall-checked verdict; all VP01–VP08 probe
expectations still pass; the reevaluation §6 evidence-contract sentence is
implemented, not just quoted.

**Agent guidance.** Opus-class. This is the trust-boundary workstream; the
failure mode is designing evidence that restates the request (VP01/VP02's
root cause). Every schema decision should be checked against: "could a
malicious provider make the core's check vacuous?"

### W4 — Interprocedural custody summary inference (LC01's open boundary)

**Goal.** The generic capability whose absence forces per-library hand
contracts: infer and validate parameter/return custody summaries for internal
functions of a translation unit, so wrappers around contracted APIs are
analyzed instead of Unknown.

**Tasks.**
1. In `legacy_c.rs`, add a bottom-up (SCC-ordered) summary pass over the
   already-built per-function CFGs: infer each internal function's parameter
   modes and return ownership from its body given the contract environment;
   validate the summary by re-running the intraprocedural analyzer against
   it. Recursion: require a consistent fixed point or stay Unknown.
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

**Acceptance.** LC01's "remaining boundary" paragraph in
`REEVALUATION_FOLLOWUP.md` can be updated to "implemented for parameter/return
custody with validation"; no new false-Safe on the probe suite; triage deltas
archived under `docs/reports/`.

**Agent guidance.** Opus-class, highest reasoning effort of the plan. This is
real static-analysis design (mode inference + fixed points + aliasing
interaction with `MAX_ALIAS_TARGETS`). Insist on the "validated, not just
inferred" discipline the follow-up demands.

### W5 — Provider extraction and orchestration-as-configuration (Cycle 20C/20D)

**Goal.** Finish moving library semantics behind the plugin boundary and turn
the 966-line audit example into a reusable, configured driver.

**Tasks.**
1. Promote the orchestration in `cycle19_fluent_bit_libraries_audit.rs`
   (compile-db ingest, pin verification, per-unit iteration, verdict
   tallying, JSON report emission) into library code (e.g.
   `compiler/src/audit_driver.rs`) driven by a W2 profile + W1 contracts; the
   example shrinks to argument parsing. A second, different codebase then
   needs only a profile directory — recreate the Cycle 17 CFL adjacent-
   checkout probe through this path as proof.
2. Extract `qcore.cmetrics`/`qcore.ctraces` and then `qcore.librdkafka`
   providers per the Cycle 20C order, using W3's request/witness path rather
   than new typed side-doors; each with its own manifest, diagnostic
   namespace, tests, and parity record.
3. Rename/relocate the Monkey trio and `event_driven` internals so the
   *reactor-generic* parts (callback-summary schema, generation-indexed
   lifecycle, descriptor custody) live in core-adjacent generic modules, and
   only the Monkey/Libevent tables live in the plugin (they largely already
   do — this is separation inside `verifier_plugins/libevent/`).
4. Only after each parity record: delete the corresponding hardcoded route
   (`external_source.rs` arms, `AuditedLibrary`, first-party literals).

**Acceptance.** Cycle 20's acceptance-gate list advances measurably: separate
manifests/tests/namespaces for all five providers; the Fluent Bit composition
profile reproduces Cycle 19's pinned verdicts through the registry; the
monolithic routing is gone.

**Agent guidance.** Sonnet-class for extraction mechanics with the parity
harness from task 1 built first; Opus-class review at each deletion step.

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

**Acceptance.** A recorded, increasing `mapping_status` count with per-stage
evidence; no CT case marked complete without an executed source-level test.

**Agent guidance.** Sonnet-class, batchable (cases are independent — this
fans out well across parallel agents).

### W7 — Hygiene: names, diagnostics, roadmap

Small, continuous, do-with-adjacent-work:

- Resolve the `Q700–Q712` collision: spec §54 names Libevent-specific
  diagnostics in a numbered range that Cycle 20 reuses for `verify scope`
  resolution. Assign plugin-namespaced diagnostics (the `QP***` space already
  exists) for provider-specific conditions and reserve `Q7xx` once,
  recording the decision in the spec changelog.
- Rename library-named wrappers around generic models where cheap:
  `QCore.MonkeyEvent` (a generic descriptor-custody model), the
  `busy_effect_preserves_chunkio_lock` theorem, `MonkeyComposition` (keep
  aliases; Lean renames ripple into the audit files, so batch them).
- Update `docs/ROADMAP_AFTER_CYCLE_9.md` (currently ends at Cycle 18) to
  record Cycles 19–20 and this plan, keeping its "no cycle skipped without
  updating the dependency order" rule.
- CI: add the Lean job the reevaluation flagged (EV01) if still absent.

## 4. Sequencing summary

```
W1 contracts-as-data ──┬─▶ W4 custody inference ──┐
W2 profiles-as-data  ──┤                          ├─▶ W5 provider extraction + parity deletions
W3 obligations+dispatch┘                          │
W6 CT corpus (continuous, consumes W3/W4 output) ─┘
W7 hygiene (continuous)
```

Suggested review checkpoints: after W1 parity diff; after the W2 manifest
schema (before mass migration); after the W3 evidence-schema v2 design
(before implementation); after the first W5 provider parity record.

## 5. What this plan explicitly does not do

- It does not promote any pinned translation unit toward `Safe`; verdict
  movement comes only from W4's validated inference and is reported per
  invariant 4.
- It does not open the Cycle 19 fail-closed gates (per-path identity imports,
  observation evidence) — those remain Cycle 19's own acceptance work.
- It does not design a new plugin ABI; protocol v1 framing stays, evidence
  moves to v2 within the existing negotiation.
- It does not treat contract/profile data files as verified truth; they are
  declared trusted assumptions carried into every certificate.
