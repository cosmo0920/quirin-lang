# QCore-C source review and specification revision 0.3

**Review date:** 2026-09-07, Asia/Tokyo.  
**Reviewed artifact:** uploaded `qcore-c.tar.gz`.  
**SHA-256:** `fd3e844ec975c74e47c7113037c334dd7a4f78fbb8a7e1543a5e59f58da5760b`.  
**Change scope:** specifications, documentation navigation, validation records and isolated review probes. Existing implementation, production policies and historical evidence are unchanged.

## 1. Executive assessment

The repository has substantial, separated implementations: a structural frontend, closed quantitative-index resolution, interval usage algebra, ownership/resource-flow/error models, a small source-to-C path, a conservative C custody analyzer, a strict external-resource contract loader, specialized source/observation adapters, bounded concurrency models and a fail-closed verifier registry. The problem is not lack of an implementation. The main specification problem is that these components must not be described as one uniformly implemented, end-to-end verified source language.

The r0.3 language and contract specifications make those boundaries explicit. They preserve the separation of usage, pure dimensioned indices, credit, live duties, cost, access authority, effects and per-exit state. They specify residual custody on failed cleanup, distinguish exact iteration counts from upper bounds, and prevent structural or observation acceptance from becoming a general safety claim.

Static review also identified concrete inconsistencies and missing checks. R01/R02 concern the explicit concurrency model, R04/R05 ordinary source checking, R06 the loop-summary contract, and R07 a signed-literal boundary. R03 identifies trusted model-input assumptions that cannot support a stronger source-authority claim without additional evidence. R08–R10 concern contract interpretation, documentation and snapshot/provenance boundaries.

**These are source-inspected findings, not freshly executed Rust counterexamples.** Rust and Lean are unavailable in the review environment. The included Rust probes reproduce the relevant public API calls but have not been built or run. Fresh execution consists of the Python evidence-validator suite and the documentation/package checks recorded in `validation.json` and [../../VALIDATION.md](../../VALIDATION.md).

## 2. Snapshot and supplied Codex continuation

There is no `.git` directory in the archive. Its identity is its content hash, not an inferred branch name or commit. The supplied Codex continuation mentions a merge at `6275e2c`, ongoing W2/W3/W4 edits and later native reopen-failure evidence. Those messages describe another working-tree state; they cannot establish what was merged into this archive.

The archive contains W2 and W3 design documents but not a W3 `compiler/src/verify_dispatch.rs` implementation. It contains the basic W4 inference but not the described richer inference repairs. The later `docs/reports/cycle19-reopen-failures` package, its associated policy and the referenced local pasted-text attachment are absent. No later native trace, sanitizer outcome, proof theorem or source-unit promotion is credited to this review.

The Libevent submodule directory is empty. This makes the source archive incomplete for gates needing those pinned external sources. It does not invalidate the files that are present, nor establish that external-dependent tests would fail for semantic reasons.

### Workstream checkpoint

| Workstream | What the uploaded source supports | What remains distinct |
|---|---|---|
| W0 | Schema/CLI/core/frontend/contract workspace separation is present. | The full target crate/provider split is not complete. |
| W1 | Strict resource-v1 loader, source-text replay and selected declarative resource modules are present. | Rich lifecycle policies are not reducible to the resource vocabulary by dropping clauses. |
| W2 | `docs/design/W2_PROFILE_MANIFEST_SCHEMA.md` is a design artifact. | Declarative generalized profile implementation and source/revision substitution evidence are not supplied. |
| W3 | `docs/design/W3_OBLIGATIONS_EVIDENCE_V2.md` is a design artifact; v1 firewall is implemented. | Core obligation generation, source dispatch and accepted evidence-v2 claims are not supplied. |
| W4 | Conservative basic formal/return custody inference and validation are implemented. | General Out/Stored/Removed/Reallocated inference and the later reassignment repair are not supplied. |
| W5 | Existing in-process library modules and registry remain. | Full independent provider/host extraction is not established. |
| W6 | CT02, CT12, CT16, CT19 and CT37 are mapped in a 50-case manifest. | 45 cases remain unmapped; each mapped case retains separate stage statuses. |
| W7 | Some diagnostics and unknown reasons exist. | Generalized diagnostics remain incremental; old Q7xx allocations need coherent ownership. |

These are snapshot observations, not completion percentages. Raw Cycle 19 units remain unpromoted and Cycle 19 remains in progress.

## 3. Findings, ordered by semantic impact

### R01 — Access is not checked against destruction state

**Priority:** high for the event-model lifetime claim.  
**Evidence:** `compiler/src/concurrency.rs:226–240,342–429,452–464`.

The checker stores a `destroyed` set. `Destroy` records an identity there and rejects repeated/wrong-custodian destruction. `Access` checks task existence, footprint shape, a held guard or a static permission, and conflicts against prior accesses. It does not consult the destroyed set. Destruction also does not revoke permissions or require other readers to be quiescent.

A minimal model supplies task 0 with full permission and custody of resource 7, then emits `Destroy(0,7)` followed by `Access(0,7,Write)`. Inspection of the control flow indicates that this trace returns an acceptance certificate. Destruction is not itself recorded as a conflicting access.

**Required rule:** an access or transfer must refer to a live generation. Destruction must establish the relevant authority and quiescence, transition lifetime state, and invalidate later access/transfer for that generation. Revocation cannot be inferred from aggregate shares alone.

**Acceptance:** reject access after destruction, transfer after destruction and destruction with outstanding incompatible authority; allow a new distinct generation after the previous generation is released. Include reads, writes, atomics, guarded accesses and spawned tasks. This is not a demonstrated exploit against arbitrary C source because this event checker is not automatically lowering arbitrary source into these traces.

### R02 — A joined child can still emit events

**Priority:** high for task-lifetime/completion semantics.  
**Evidence:** `compiler/src/concurrency.rs:265–290,342–388,452–464`.

`Join` records completion and merges clocks, but it leaves the child's clock in the map. The common `known_task` predicate tests only whether that map contains the task. Therefore `Spawn(0,1); Join(0,1); Access(1,...)` with suitable static permission has no terminal-task check. The final outstanding-task check sees the child as joined.

**Required rule:** task state must distinguish active, completed and joined/terminal as appropriate. Joining consumes the relevant handle and forbids subsequent child actions. Retaining vector-clock history must not retain permission to execute.

**Acceptance:** reject post-join child accesses, locks, transfers and destruction; preserve valid pre-join child actions and parent actions ordered after join. Test nested task structures and repeated join handling separately.

### R03 — Guard and synchronization fields are trusted model assertions

**Priority:** high qualification before source-level access certification; not necessarily a violation of the existing abstract-input convention.  
**Evidence:** `compiler/src/concurrency.rs:351–366,390–417` and tests in `compiler/tests/unit/concurrency.rs`.

An access naming a mutex held by that task bypasses the static permission check. There is no independently declared mutex-to-resource/footprint invariant relation in `ConcurrentProgram`. A lock can therefore authorize an arbitrary supplied footprint under this model convention. Existing same-mutex tests intentionally rely on guard-based authorization without a permission vector.

Similarly, `TransferCustody { synchronized: true, ... }` directly merges clocks. The Boolean is an assertion in the supplied trace, not a reconstructed C synchronization proof.

**Required rule:** for a full source-authority claim, guards must be bound to explicit invariant footprints and custody/access transitions, and synchronization must be derived from admitted source/model events or named trusted assumptions. A held unrelated lock cannot create authority. The existing bounded model may retain an explicit trust premise, but its certificate must not be advertised as having established that premise.

**Acceptance:** deny unrelated-lock access in a checked-invariant profile; validate invariant opening/closing and permitted reborrows; reject forged handoff edges and mismatched source witnesses. Do not “repair” the model by deleting mutex authorization without replacing its intended invariant semantics.

### R04 — The source driver does not enforce ordinary return-type compatibility

**Priority:** high for source-language type-correctness claims.  
**Evidence:** `crates/qcore-frontend/src/frontend.rs:38–49,904–974,1438–1448,1488–1633`; `compiler/src/driver.rs:584–603,887–932`.

The structural/typed source records do not contain a complete elaborated function result type. The driver parses a return as a primitive signed integer and lowers it without comparing that value with the declared result annotation. The minimal probe is `fn main() -> bool { return 2; }`; source inspection indicates acceptance despite the mismatch.

**Required rule:** retain and elaborate declared result types, resolve type names and validate every returned value/payload and exit against them. Either reject unsupported result semantics or label the result structural/limited-profile, not fully typed.

**Acceptance:** positive i32/unit cases as defined by the selected profile; negative bool/integer mismatch, unknown result type, invalid owned-result shape, inconsistent function exits and call/result signature mismatch. This change should be coordinated with the tagged-error ABI rather than inferred from C's permissive representation.

### R05 — Axis declarations can bypass dimension checking when unused

**Priority:** medium/high for declaration and index-environment correctness.  
**Evidence:** `crates/qcore-frontend/src/frontend.rs:1519–1548`; `crates/qcore-frontend/src/check.rs`.

`elaborate_source` constructs/checks the axis environment only if some declaration has a quantitative-index type. A module containing `quant axis bad : credit<Missing>;` but no such type can therefore avoid the checker that would reject the unknown dimension.

**Required rule:** declaration well-formedness must not depend on later type usage. Build and validate dimension/axis declarations unconditionally, then resolve each type against that validated environment.

**Acceptance:** reject unknown dimensions, duplicate axes and invalid declarations both with and without a quantitative type. Preserve valid nominal `Count<T>` behavior according to the existing dimension rules; do not conflate a nominal count with a requirement for a runtime allocation type.

### R06 — Maximum iteration evidence becomes exact final custody

**Priority:** high for loop-summary soundness under the advertised bound semantics.  
**Evidence:** `compiler/src/resource_flow.rs:232–314`; `compiler/src/research_extensions.rs:18–70`.

`Flow` uses exact net change: even `is_covered_by` requires net equality. `Flow::bounded_loop(n)` computes exactly n sequential repetitions. Its comment says “at most,” and `summarize_general_loop` accepts terminating/widened maximum-count evidence and forwards the maximum to that exact repetition routine.

For a body that acquires one resource, a loop with maximum count two has possible net changes zero, one and two. The constructed summary has exact net two and does not cover the zero-iteration flow. Recomputing the same summary in `check_loop_summary` cannot repair that semantic mismatch.

**Required rule:** separate `Exact(n)` and `AtMost(n)` evidence. Exact evidence may use n-fold composition. An upper bound with nonzero net must preserve exit alternatives/identity information or be rejected/Unknown. Zero-net peak/total overapproximations remain a distinct valid case.

**Acceptance:** maximum counts 0/1/2 with early exits, positive and negative net, and zero-net bodies; verify both numeric coverage and identity/custody invariants. Add negative mutation tests changing Exact to AtMost and deleting an early exit. The source driver currently rejects source loops, so this is a model/certificate contract issue rather than a proven source-driver exploit.

### R07 — Signed minimum literal is rejected by magnitude parsing

**Priority:** medium correctness/completeness.  
**Evidence:** `compiler/src/driver.rs:887–932`; analogous parser pattern in `crates/qcore-contract/src/contract_observation.rs:127–155`.

The negative-return branch parses the magnitude into i32 before negating it. `2147483648` is not positive i32, so `return -2147483648;` is rejected. This contradicts an unrestricted “signed 32-bit integer literal” description, although it is fail-closed rather than unsafe acceptance.

The observation evaluator has the corresponding textual-minimum issue for i128. A fact can contain the minimum value; its literal spelling is not accepted through the positive-magnitude path.

**Required rule:** parse the magnitude in a suitable widened/unsigned representation and range-check the final signed result, or explicitly version/document a narrower literal profile. Do not introduce unchecked negation.

**Acceptance:** zero, negative zero, both extrema, one outside each bound, and arithmetic involving supplied minimum facts. Literal completeness does not repair R04's result-type issue.

### R08 — “Contract checked” is ambiguous without consumer coverage

**Priority:** high reporting/trust-boundary qualification.  
**Evidence:** `compiler/src/driver.rs:98–118,210–299`; `crates/qcore-frontend/src/frontend.rs:485–598`; `crates/qcore-contract/src/contract_loader.rs:1–61,133–185`; `crates/qcore-contract/src/contract_observation.rs:1–93`.

The ordinary driver explicitly does not discharge arbitrary contract predicates. The strict resource loader builds trusted assumptions. The observation interpreter evaluates supplied facts. Specialized adapters derive facts under restricted premises. These are useful but distinct meanings of acceptance.

The presence of a `requires` clause does not prove its truth at every caller. A nonzero lifecycle fact supplied to the evaluator is not independently a proof of its English meaning. A syntactically legal resource declaration can be false of the external function; whole-file validation cannot discover that false assumption by its name.

**Required rule:** select a consumer and report clause coverage, assumptions and evidence category. Preserve fail-closed behavior in the strict loader and observer. Keep structural tooling, but prevent its results from being relabeled full-contract discharge.

**Acceptance:** corpus modules retain correct consumer classification; unknown clauses are rejected by strict consumers, not skipped; false/missing observations and stale source assumptions are rejected; valid precondition exclusions stay distinct from checked observations.

### R09 — Entry-point, validation and diagnostic documentation drift

**Priority:** medium documentation/operability.  
**Evidence:** original `docs/QCORE_MODULE_MODES.md:46–50`, `contract/qcore/README.md:16–20`, `docs/VALIDATION.md`, revision-0.2 diagnostic table; current `crates/qcorec/Cargo.toml` and source-scope driver.

Several commands refer to a `qcorec` binary in `qcore-compiler` after the CLI has moved to the `qcorec` package. The old validation document describes an earlier prototype path not present in the archive. Revision 0.2 allocates Q700–Q712 to Libevent while current source-scope diagnostics use Q700/Q701.

**Revision:** correct package commands, archive the old validation narrative, publish fresh validation limitations, and document a single diagnostic ownership direction. Historical diagnostic records are preserved; code-level migration/golden tests remain future work.

### R10 — Source/proof/observation composition is narrower than cycle labels

**Priority:** high scope/provenance qualification.  
**Evidence:** source and design inventory; `compiler/src/verifier_plugin.rs:737–836`; CT manifest; absence of later Codex artifacts.

An explicit C11 execution candidate, a native trace, a Lean theorem about a model and a source-bound C projection have different subjects. Their conjunction is not automatically a full Rust/Lean/C refinement theorem or a whole-program safety certificate. The current plugin-v1 firewall correctly refuses Safe and downgrades unsupported Unsafe claims.

**Required rule:** publish an artifact/property/profile/scope/bounds/assumptions/coverage/evidence tuple. Do not promote raw units, broaden OS failure guarantees, claim arbitrary schedules, or import unmerged later work based only on prose.

**Acceptance:** evidence mutations, scope substitution, missing obligations, unsupported counterexamples and empty evidence fail closed. Preserve v1 refusal during any future W3 transition. Count CT stages separately and keep external source/build identities explicit.

## 4. Specification changes delivered

The revision creates a primary language specification, companion contract specification, current-consumer EBNF, and a collected extended design BNF. It also updates navigation and the current validation record while preserving the previous design and evidence documents as historical material.

The most consequential semantic requirements are generation-indexed lifetime checks, terminal task state, checked mutex invariants, exact-versus-bounded loop custody, unconditional type/declaration validation, typed per-exit residuals and consumer-specific contract/evidence claims. These requirements are not declarations that the compiler has already been repaired.

`source_inventory.json` records file hashes, line counts and selected review anchors. `acceptance.json` records open and observed requirements without adding false CT mappings. The isolated `probe/` package exercises R01–R07 and illustrates R08 using the current public Rust APIs; it is outside the production workspace and does not alter its standing gate.

## 5. Validation and reproduction

Freshly executed on the archived source and again on the revised documentation tree:

```sh
python3 -B -m unittest discover -s scripts/tests -v
```

The original run passed 39 tests. The revised-tree run and package/documentation checks are recorded in `validation.json`; logs are included. These Python tests concern evidence/trace validator behavior, not the complete Rust type checker.

On a complete development checkout with the appropriate Rust toolchain and dependencies, the unexecuted review reproductions are invoked with:

```sh
cargo run --manifest-path docs/reviews/r0_3/probe/Cargo.toml
cargo run --manifest-path docs/reviews/r0_3/probe/Cargo.toml -- --require-conformance
```

The first command reports observations. The second requires the r0.3 behavior and is expected to exit unsuccessfully on the reviewed snapshot. R03's row specifically tests a stronger checked-invariant profile, not an assertion that the old abstract guard convention was never intentional. These commands were not run here, and the probe's own build remains unverified.

Rust tests, formatting, Clippy, examples, Lean builds/audits, pinned external audits and new sanitizer runs remain unexecuted in this review. A complete checkout should run the existing standing gate in addition to these probes; no historical pass is substituted for a fresh result.

## 6. Implementation order required by the review

Repair and test R01/R02/R04/R05/R06 before widening the associated safety or typing claims. Define the checked-invariant/trust boundary for R03 rather than accidentally assuming it. Repair the bounded literal issue separately. Carry these requirements into W2–W4 design work, preserve the strict contract vocabulary and v1 firewall, and expand CT coverage only with actual stage evidence.

The source state returned with this revision has the same production semantics and historical evidence as the uploaded archive. What has changed is the specification's precision, its explicit open requirements and the ability to audit which claim belongs to which implemented boundary.
