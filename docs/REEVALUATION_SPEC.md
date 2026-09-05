# QCore-C reevaluation specification

Date: 2026-09-05. Status: proposed acceptance specification with an executed diagnostic audit
and a [six-finding repair record](#six-finding-repair-on-2026-09-05).
The subsequent [seven-finding follow-up and repair](REEVALUATION_FOLLOWUP.md#repair-and-containment-on-2026-09-05)
records the current ownership, mutex-history, and provider containment changes.

The initial audit established certificate soundness as the immediate priority.
Six independent audit probes disagreed with the intended semantics: three invalid weak-memory candidates are
accepted, one valid C11 candidate is rejected, and two insufficient plugin
requests receive core `Safe` certificates. This is evidence against the checked
boundaries, not a claim that an emitted application has exhibited a runtime bug.

This document specifies reevaluation of the entire research stack: the inherited
sequential language, proposed revision 0.2, and Cycles 17–20. It does not declare
those features implemented or repair the compiler. Requirements below are
proposed project policy; paper results are attributed separately.

The inspected checkout had HEAD `e4ed3df5b92176435159d6833c56cfe1d44d7a4f`
and existing edits in `legacy_c.rs`. Other files changed concurrently during the
audit. Results describe the working tree when each command ran, not a clean
checkout of HEAD. Audited-file hashes and raw logs accompany this document in
[`reevaluation/`](reevaluation/). Existing source edits were not modified by this audit.

## 1. Claim and verdict contract

Every published result MUST identify `(artifact, property, semantic profile,
scope, bounds, assumptions, evidence version)`. A bare `Safe` label is insufficient.
These claims MUST remain separate:

| Claim | Required evidence | Does not establish |
|---|---|---|
| Syntax accepted | Actual parser run, source and spans | Type correctness |
| Source accepted | All required checker passes on the same typed IR | Arbitrary legacy-C safety |
| Candidate consistent | Valid relations for one named memory model | Exploration of all executions |
| Bounded program safe | Complete exploration within explicit bounds | Unbounded safety or termination |
| Property proved | Sound proof/checker against an operational semantics | Correct extraction or backend without a bridge theorem |
| Observed agreement | Recorded executions and an observation relation | Universal compiler refinement |
| Library projection | Exact analyzed functions and missing obligations | Safety of omitted functions or whole translation units |
| Plugin certificate checked | Capability-specific semantic evidence replay | Truth of caller-supplied Boolean claims |

Use `Safe` only for the named property after all obligations are discharged.
Use `Unsafe` for a validated reachable violation under the same assumptions.
Use `Unknown` for unsupported operations, overapproximate alarms without a
feasible witness, incomplete coverage, solver failure, timeout, invalid evidence,
or unresolved assumptions. Report invalid input and infrastructure failure as
separate reason codes even if the public API maps them to `Unknown`.

A type-system rejection is not automatically an unsafe runtime program. An
inconsistent candidate graph is not an unsafe program: it is an execution that
the memory model forbids. Missing evidence is not a counterexample. Conditional
proofs MUST retain their assumptions in every composed result.

## 2. Recent research and its consequences

Search scope: primary papers, author/institution pages, and artifacts published
in 2024–2026 available by this audit date; older foundations retained where
necessary. This is a targeted literature review, not an exhaustive systematic
review. Abstract-only reading is identified; none of the external tools or
reported benchmark results below was reproduced in this audit. Publication
dates come from the paper/page, not search-engine crawl timestamps.

| Source | Verified contribution and reading depth | Proposed QCore-C reevaluation action |
|---|---|---|
| Beyer and Strejček, **Evaluating Software Verifiers for C, Java, and SV-LIB**, TACAS 2026. [Paper](https://link.springer.com/chapter/10.1007/978-3-032-22749-2_23) | Read report sections on evaluation and witnesses. Separately evaluates verifiers and witness validators, including handcrafted witnesses; identifies remaining incorrect results. | Evaluate the firewall itself with independent valid and invalid certificates. Report results per property, retaining unknowns and validation failures. |
| Svoboda et al., **A Pointer-Ownership Model for C Inspired by Rust**, 2026. [Institutional abstract and DOI](https://www.sei.cmu.edu/library/a-pointer-ownership-model-for-c-inspired-by-rust/) | Abstract only. Temporal ownership analysis uses constraints and lifetime reasoning; reports both unsafe-example detection and acceptance of safe examples. | Add held-out Juliet temporal-safety cases and safe counterparts. Evaluate false rejection as well as false safety; inferred contracts remain untrusted until checked. |
| Villani et al., **Tree Borrows**, PLDI 2025. [Author project page](https://plf.inf.ethz.ch/research/pldi25-tree-borrows.html) | Read project summary. An aliasing model addresses patterns rejected by Stacked Borrows and has mechanized optimization arguments. | Test reborrows, delayed mutation, alias invalidation, and safe-but-rejected patterns. Treat it as design evidence, not the semantics of C or QCore-C. |
| Grannan et al., **Place Capability Graphs**, 2025. [Preprint](https://arxiv.org/abs/2503.21691) | Abstract only. Models ownership/borrowing using compiler-derived information, including composite types and loops. | Measure the source-to-capability bridge, field-sensitive identities, loop joins, and interprocedural borrowing; count unmapped constructs. |
| Lian and Wang, **Automatic Linear Resource Bound Analysis for Rust via Prophecy Potentials**, 2025. [Preprint](https://arxiv.org/abs/2502.19810) | Abstract only. RaRust combines amortized resource reasoning with shared/mutable borrows and a resource-aware borrow calculus. | Test potential transfer through aliases and reborrows; distinguish physical memory, usage grades, and spendable credit. |
| Kahn et al., **Efficient Cost Bounds with Linear Maps**, 2025. [Preprint](https://arxiv.org/abs/2509.22982) | Abstract only. Represents cost-free types using linear maps and checks cost constraints through matrix inequalities. | Separate inference success from certificate checking and principality. Independently replay inferred inequalities and evaluate solver scaling. |
| Golovin et al., **RELINCHE: Automatically Checking Linearizability under Relaxed Memory Consistency**, POPL 2025. [Paper](https://plv.mpi-sws.org/genmc/popl2025-relinche.pdf) | Read introduction/method overview. Checks outcomes and synchronization requirements for bounded concurrent-library clients; SC reasoning alone misses weak-memory violations. | Test queue/refcount and callback compositions against outcome sets and required happens-before relations, not one successful schedule. |
| Marmanis et al., **Model Checking C/C++ with Mixed-Size Accesses**, POPL 2025. [Paper](https://plv.mpi-sws.org/genmc/popl2025-mixer.pdf) | Read abstract/introduction. Addresses overlapping accesses of different sizes in weak-memory model checking. | Test overlapping byte footprints, unions, bit-fields, and target layouts. Unsupported mixed-size behavior must remain unknown. |
| Gäher et al., **RefinedRust: A Type System for High-Assurance Verification of Rust Programs**, PLDI 2024. [Paper](https://plv.mpi-sws.org/refinedrust/paper-refinedrust.pdf), [artifact](https://zenodo.org/records/10912439) | Read abstract and published contribution summary. Foundational refinement typing addresses mutable borrows, invariants, and layouts. | Require explicit semantic statements connecting executable predicates to ownership/lifetime properties; Boolean acceptance lemmas alone are insufficient. |
| Beyer et al., **SV-LIB 1.0: A Standard Exchange Format for Software-Verification Tasks**, 2025. [Preprint](https://arxiv.org/abs/2511.21509) | Abstract only. Defines a common representation for programs, specifications, witnesses, and validation tasks. | Version the complete verification task and its semantics, not just the provider response envelope. |

Two older foundations remain relevant. [Alive2 (PLDI 2021)](https://web.ist.utl.pt/nuno.lopes/pubs.php?id=alive2-pldi21)
is explicitly bounded translation validation: its relevance here is refinement
and an explicit supported domain, not an automatic proof of QCore's C emitter.
[GenMC](https://plv.mpi-sws.org/genmc/) supports named weak-memory models and
bounded exploration; select the same model before comparing verdicts. Neither
tool is a drop-in oracle for all QCore constructs.

For the existing C11 claim, the normative baseline used for the four memory
probes is the public [WG14 N1570 draft](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf),
§5.1.2.4 and §7.17. Do not silently substitute C++20 release-sequence rules,
RC11, or hardware models for that baseline.

## 3. Reproduced discrepancies

Run from the repository root:

```sh
cargo run -p qcore-compiler --example reevaluation_probe
```

The [probe source](../compiler/examples/reevaluation_probe.rs) prints JSON and
exits **1** when intended expectations fail. A nonzero exit is intentional
evidence of an unresolved finding, not a build failure. The archived
[output](reevaluation/probes.json) records six failed expectations and two
passing controls. These API probes are separate from end-to-end source tests.

Severity: P1 blocks the affected safety claim; P2 blocks the affected precision
or reproducibility claim. No application exploit or production impact is asserted.

| ID | Severity | Counterexample and observed result | Defect location / required correction |
|---|---|---|---|
| WM01 | P1 | Plain store and load both use `AcquireRelease`; accepted. C11 disallows this order on these operations. | [`weak_memory.rs`](../compiler/src/weak_memory.rs), `invalid_order`: use an explicit legality table for each operation kind. |
| WM02 | P1 | Same-thread writes `x=1; x=2` have supplied modification order `init < x=2 < x=1`; accepted. This reverses happens-before. | `validate_modification_order` / `validate_coherence`: acyclic and total does not imply consistency with happens-before. |
| WM03 | P1 | Both threads perform an SC store then an SC load of the other location; both loads read zero; accepted. No SC order permits this outcome. | `validate_sc`: checks membership and happens-before but omits the required relationship between SC loads and writes. |
| WM04 | P2 | Release store, same-thread relaxed store, then another thread's acquire reads the latter. The valid head-to-acquire synchronization edge is rejected. | `expected_synchronizes_with`: only follows direct release/acquire reads-from; implement the selected C11 release-sequence relation or explicitly report unsupported. |
| VP01 | P1 | One `ApiRecognized=true` fact for a nonexistent source file and unaudited revision obtains a `ResourceContracts` certificate. | [`verifier_plugin.rs`](../compiler/src/verifier_plugin.rs), `BuiltInProvider::verify`, `check_evidence`, and first-party manifests: equality with request facts is not a capability-specific resource proof. |
| VP02 | P1 | A custom provider supplies `Safe` with empty claims and empty coverage for an empty-fact request; the firewall creates a certificate. The built-in provider's empty-input check correctly returns `Unknown`. | `check_evidence`: empty `.all(...)` succeeds. Require a core-established obligation set and explicit proof of any legitimate empty scope. Provider-local guards cannot enforce the firewall boundary. |

The first four expected results follow the cited C11 clauses. VP01–VP02 follow
Cycle 20's stated independent-checking and fail-closed requirements. If request
facts are intentionally trusted assertions, the API and documentation must
declare that narrower trust boundary; such certificates must not be advertised
as independent source verification. The probes do not establish that the CLI
currently routes application verification through this API.

Regression requirements: separate WM01 store/load cases and all order values;
reverse each coherence edge in turn; test all SC store-buffering orders and
retain the relaxed positive control; test release-sequence continuation and
termination; vary VP01's predicate, source, profile, and capability independently;
test VP02 through both custom in-process and actual process providers. A fix
must pass the intended expectations without special-casing probe identifiers.

## 4. Additional findings from code inspection

These are demonstrated structural gaps or follow-up hypotheses, not additional
executed counterexamples. Keep them out of the six-probe defect count.

**FM01 — proof scope, P1 for broad claims.**
[`ResearchExtensions.lean`](../proof/QCore/ResearchExtensions.lean) represents
the C11 relations and race property as Boolean fields. `accepted_c11_is_race_free`
concludes that `nonAtomicRacesOrdered = true`; it does not prove the Rust graph
algorithm implements the C11 rules. Similarly,
[`VerifierPlugin.lean`](../proof/QCore/VerifierPlugin.lean) checks evidence flags.
Theorems about these models are valid but narrower than implementation soundness.
`remainingResearch := []` and a theorem proving that list empty do not discharge
the named research problems. Replace broad completion claims with an obligation
table and require graph semantics plus a checked implementation bridge.

**VP03 — package identity and reproducibility, P1 for untrusted providers.**
Manifest validation checks digest shape, and evidence checks echoed digest
equality. `ProcessProvider::new`, `from_package`, and `verify` do not compute
the package/executable digest. `deterministic=true` is also echoed metadata,
not an observed repeatability check. Recompute content identities, bind all
semantic inputs, and test replacement/symlink/TOCTOU cases. Repeatability tests
are evidence only for the executed runs, not proof of universal determinism.

**VP04 — process deadlines, P2; reproduction pending.**
The timeout loop kills the direct child and then joins I/O threads. Descendants
can inherit pipes and delay EOF, allowing joins or `recv()` to outlive the
deadline. Test a bounded helper that exits while a descendant retains stdout,
and a child that never reads stdin. Bound process-tree termination and I/O
completion by the same deadline. Memory/network/filesystem isolation is already
recorded as unfinished in Cycle 20; environment clearing does not satisfy it.

**VP05 — source scope integration, P1 if called verification.**
[`driver.rs`](../compiler/src/driver.rs), `compile_files_with_registry`, resolves
source declarations but does not invoke `registry.verify` on derived facts.
The returned scope resolution is discarded. Track this as configuration
validation until actual obligations are generated, providers invoked, and
results attached to the compilation artifact. Test using a counting provider
whose verifier returns `Unknown`, not just a provider whose manifest resolves.

**FM02 — composition correspondence, P2.**
Lean `compose [unknown, rejected]` returns unknown, while Rust scans for any
unsafe member first and returns unsafe. This is an implementation/model
disagreement, not a false-safe example. Choose precedence, then test every
verdict permutation and prove order independence for the intended result.
Composition must also require the expected provider set; a supplied list of
certificates alone cannot identify an omitted required provider.

**TV01 — observation agreement boundary.**
[`translation_validation.rs`](../compiler/src/translation_validation.rs)
reuses `lower_cleanup_cfg` to check the boundary and accepts a caller-supplied
stdout string. This supports structural replay and observation agreement;
it cannot by itself attest which executable produced that output or establish
all-execution refinement. Preserve those useful checks and add independent
semantic checks and run identity rather than broadening their interpretation.

**EV01 — evaluation provenance.**
[`VALIDATION.md`](VALIDATION.md) carefully limits its claims to a Python design
experiment, but references `prototype/run_validation.py` in an extracted bundle;
that prototype is not present in this checkout. This audit did not reproduce
its 48 tests or 9,154 numeric interleavings. Keep their bundle and digest in the
evidence manifest. They are not compiler conformance or threaded executions.
The checked-in CI workflow runs Rust checks, but has no Lean job despite the
stated continuous Lean acceptance requirement.

## 5. Complete reevaluation coverage matrix

All rows are required workstreams. Existing unit tests may supply cases after
review; this table does not assert they are absent. Every row needs positive,
negative, unsupported, malformed-evidence, and boundary cases where applicable.

| Area / existing implementation | Required obligations and adversarial cases | Oracle / exit condition |
|---|---|---|
| Lexer, parser, frontend | Parse actual examples; precedence, shadowing, duplicate names, UTF-8 spans, multi-file resolution, unsupported syntax | Expected AST/typed IR and source-located diagnostics; grammar-reference coverage is tracked separately |
| Indices and quantitative types (`index`, `check`, `usage`) | Natural-number boundaries, dimension mismatch, impossible intervals, branch demand, substitution, recursion, arithmetic overflow, semiring laws | Independently checked equations/inequalities; generated algebraic cases and rejection mutations |
| Inference (`inference`) | Sound inferred product, incomparable solutions, cycles, unsatisfiable constraints, solver unknown, principality relative to a declared order | Replay constraints; prove minimality separately; inference timeout never certifies a default |
| Ownership | Move/alias identity, split/recombine, reborrow expiry, region escape, fresh generations, alias-preserving calls | Conservation and lifetime invariants on concrete states plus source tests |
| Resource flow | Net, peak and cumulative cost, negative-net paths, nonzero-net loops, recursion, allocation failure, integer limits | Independent allocation traces and prefix maxima; induction for bounds beyond enumeration |
| Typed errors / error regions | Branch-sensitive payload custody, quant opening, linear discharge, nested handlers, payload lifetimes | Every normal/error exit has exactly its declared owners, demand and cleanup obligations |
| Advanced control | Cancellation at each safe point, failing destructors, LIFO order, double-error priority, exactly-once closure captures | Enumerate bounded failure points; compare error identity and resource post-state |
| Whole-program checks | Calls transfer the same identities; SCC summaries, recursive contracts, indirect/foreign calls, cached assumptions | No missing call effect; cache invalidation by semantic input; source-to-IR obligations are traceable |
| CFG and C11 lowering | Every cleanup edge, tag, ABI layout, value conversion, evaluation order, observable I/O, undefined-behavior boundary | Independent semantic checks and native differential execution; compare outcome sets for nondeterminism |
| Concurrency authority | Share conservation, overlapping footprints, child creation/join failure, completion capsules, task escape, guard transfer | Pointwise ownership/permission invariants; no permission recovery before successful join |
| Weak memory | WM01–WM04, read/read and read/write coherence, SC/mo/rf constraints, initialization, mutex ownership/order, spawn/join graph validity | Matched-model litmus oracle; consistent and inconsistent graphs; candidate validity and exploration completeness reported separately |
| Atomic extensions | RMW atomicity, compare-exchange failure orders, fences, release sequences, publication and reclamation | Explicit unsupported status until semantics and evidence exist; race freedom does not prove reclamation safety |
| Legacy C gateway | Macros/includes, compile database flags, target layouts, loops, goto/switch, arrays, fields, nullness, realloc failure, aliases, unknown callbacks | Compare extracted obligations to Clang-derived inventory; unexplained omissions prevent whole-unit safety |
| Libevent / native adapters | Priority order, callback reentrancy, delete/free during callbacks, generation reuse, shutdown with pending callbacks, cross-context conflicts | Pinned source/build + complete bounded schedule exploration + actual observation hooks |
| ChunkIO / Fluent Bit libraries | Nested teardown, delete-only post-states, ownership transfer on failures, refcount final release, queue publication, socket custody | Per-provider positive/negative traces and pinned Cycle 19 parity; raw projection does not mint safety |
| Plugin protocol/firewall | Missing obligations, empty witnesses, wrong predicate/capability, forged claims, wrong package/source/profile, malformed framing, replay | Independently derived obligation set and semantic checker; all malformed/incomplete proposals fail closed |
| Plugin composition/isolation | Missing required provider, overlapping ownership, contradictory assumptions, verdict permutations, crash/timeout/pipe descendants, resource limits | Composition theorem and bounded process harness; absence or failure cannot trigger optimistic fallback |
| Lean and implementation bridge | Semantic theorem statements, executable definitions, extraction/serialization relation, axioms, every cycle audit | Build and axiom audit plus explicit bridge obligations; compilation of a theorem is not semantic adequacy |

### 5.1 Revision 0.2 conformance corpus

The original CT01–CT50 cases in spec §55 remain mandatory and unchanged.
[`ct_manifest.json`](reevaluation/ct_manifest.json) preserves each case and its
expected outcome. Every entry starts `unmapped`: this audit has not established
which existing source fixtures meet it. That status does not mean no related
unit test exists.

For each CT entry, add a real source fixture, source span, semantic profile,
expected diagnostic or post-state, checker stage, backend applicability, and
proof obligation. Track stages separately: parser, type checker, reference
execution, CFG, native backend, and proof. Hand-constructed Rust events cannot
alone mark a source conformance case complete. Preserve CT01–CT50 as the
language corpus; add WM/VP regressions and library cases in separate namespaces.

### 5.2 Resource accounting oracle

For a concrete trace, compute `live(t)` from distinct live allocation identities
and modeled overhead, `peak = max_t live(t)`, and cumulative cost from all
charged acquisitions/work events. Returned-but-unjoined allocations stay live.
Aliased views do not create new allocations. Permission shares authorize
access; they do not multiply allocation size or represent read-count credit.

For sequential composition, verify the entry requirement and the maximum of
the first segment's peak and its net change plus the second peak. For
concurrency, bound incremental peaks jointly with the initial live set and
runtime overhead; do not double count shared initial allocations. Check
adversarial schedules where local peaks coincide. Work, span, wall time,
allocator-resident memory and logical live bytes require distinct cost models.

Small-domain enumeration is a bug-finding oracle. General claims about FULL,
arbitrary indices, or unbounded recursion require the corresponding algebraic
or inductive proof. Include 0, 1, FULL−1, FULL, FULL+1 and representation limits.

## 6. Evidence and implementation contract

The core MUST derive an obligation inventory from the normalized program.
Plugins may propose witnesses, not the list of obligations whose absence would
make their own proof succeed. For each capability, define a versioned relation
`check(program, profile, assumptions, witness) -> result` and prove that an
accepted witness entails that capability's property.

A request/result manifest MUST bind: raw and normalized source digests; all
headers, generated files and compile flags; working directory; compiler/checker
versions; target triple, ABI/layout, library revisions and backend; FULL and
cost model; memory model and bounds; foreign summaries and observation hooks;
provider package contents; required capabilities; and evidence schema version.
Cache keys MUST include every field affecting semantics. Missing input identity
is unknown, not an empty-string default.

Certificates MUST contain the obligation IDs, semantic witnesses, assumptions
actually used, checker identity, dependency certificates and scope. Coverage
must refer to the core inventory, not merely the submitted fact count. An
empty scope is safe only with an independently checked no-obligations argument.
Native observations additionally bind emitted source, compiler invocation,
executable digest, exit status, seed/input, profile and instrumentation version.

Composition MUST validate the required set of capabilities/providers and shared
resource identities. It must preserve all component assumptions and evidence,
not just return a leading certificate as if it certified the whole composition.
An assumption conflict is unknown; a negative verdict needs its own validated
witness before taking precedence. A partial-order observation relation must
preserve dependencies and errors while allowing independent event permutations.

## 7. Experimental protocol and scoring

1. Freeze a clean commit and submodules, archive local patch if any, pin tools,
   target, dependencies and environment. Give every case a stable ID and digest.
2. Establish expected labels independently from the checker under test. Use
   reviewed semantics, reduced proofs or validated witnesses. Tool voting is
   supporting evidence; disagreements stay unresolved until investigated.
3. Split cases into historical regressions, development mutations, and held-out
   external cases. Group near-duplicates/templates together to prevent leakage.
   Include both safe and unsafe variants and unsupported-feature sentinels.
4. Run every required task, retaining failures. Record preparation, parsing,
   analysis, witness validation and execution costs separately. Set default
   per-case budgets before measurement: 60 seconds and 2 GiB for core cases;
   300 seconds and 4 GiB for library cases. Larger budgets form a named separate
   lane. These are proposed starting budgets, not performance promises.
5. Run deterministic cases twice to detect disagreement; investigate differences.
   For timing, use at least five measured repetitions with declared warm/cold
   policy and report median and spread. Pin CPU/runtime/container settings.
6. For concurrent cases, record explored bounds, graph count, reductions and
   completeness status. Increasing a bound or finding no native failure does
   not prove unbounded safety. Test schedule and event-ID permutation invariance.
7. Minimize disagreements and classify root cause: oracle, parser/projection,
   semantic checker, witness checker, backend, unsupported feature, environment,
   or specification ambiguity. Preserve the original case and minimized form.

Publish raw counts per property, profile and lane: expected-safe, expected-unsafe,
unknown-oracle; reported-safe, reported-unsafe, unknown, timeout, crash and invalid
input; and separately validated/refuted/unresolved witnesses.

Report false-safe count/rate over adjudicated unsafe tasks, false-unsafe
count/rate over adjudicated safe tasks, decisive coverage over all in-scope
tasks, and a separate supported-feature fraction over the complete submitted
corpus. Never exclude unknowns from the coverage denominator. Candidate-graph
acceptance/rejection errors have their own table and are not program-safety
accuracy. Frontend rejection is not scored as a runtime counterexample.

For C libraries publish denominators for built translation units, discovered
functions, projected functions, complete property obligations and certified
units. Projection percentage is not memory-safety percentage. Count validated
warnings per distinct root cause; report duplicate instances separately.

An aggregate performance score cannot compensate for false safety. The release
gate is zero known false-safe regressions in each claimed domain, plus required
coverage and evidence completion. Zero observed errors on a finite corpus is
not a theorem of soundness. Precision and timing targets are compared against
the frozen baseline rather than invented retrospectively.

## 8. Formal acceptance obligations

For each advertised guarantee record a theorem statement, source file, semantic
domain, executable checker, bridge relation, trusted dependencies, axiom output,
and exclusions. Required obligations include permission conservation; unique
destruction custody; lifetime validity; resource prefix bounds; error/cleanup
preservation; concurrent authorization and race freedom; mutex invariant
transfer; selected-memory-model consistency; lowering refinement; and plugin
soundness/composition.

Replace evidence flags with concrete graphs/states or explicitly parameterize
the theorem by proven predicates and then discharge those predicates for the
implementation. Prove preservation/progress for the declared operational
semantics, qualifying progress by blocking, fairness and environmental
assumptions. Do not equate termination of a finite event list with termination
of arbitrary source programs.

CI MUST run the pinned Lean build and every required `AuditCycle*.lean`, reject
`sorryAx` and unauthorized project axioms, and archive the theorem dependency
surface. Standard foundational dependencies such as `propext` and `Quot.sound`
must be declared, not mistaken for unfinished project proofs. Auditing axioms
does not replace review of what the theorems actually state.

## 9. Ordered implementation and acceptance gates

| Gate | Concrete deliverable | Completion criterion |
|---|---|---|
| G0: reproducible baseline | Snapshot, logs, CT mapping, claim inventory | Another run identifies exactly the same inputs; unrelated working-tree failures classified |
| G1: certificate containment | WM01–WM03 and VP01–VP02 fixes; explicit profile restrictions | Invalid candidates and insufficient evidence cannot obtain the affected certificates; positive controls survive |
| G2: semantic adequacy | Memory-model relation, WM04 fix/profile rule, core-derived plugin obligations | Matched-oracle litmus suite and evidence-mutation corpus pass; unsupported constructs fail closed |
| G3: source integration | CT01–CT50 stage mapping, provider invocation, source/CFG/native evidence links | Every claimed source feature is exercised end to end; no manifest-only verification claims |
| G4: library/provider parity | Pinned Cycle 19 reruns and provider extraction/composition table | Same source, build, verdicts, coverage and assumptions; differences explained case by case |
| G5: proof and platform closure | Implementation bridges, Lean CI, native platform/sanitizer/hook records | All claims have matching proofs or explicitly bounded evidence; open platform gates stay open |
| G6: reevaluation publication | Raw results, scripts, held-out cases and revised claim inventory | Independent reproduction; no known false safety in claimed scope; unresolved cases visible |

Implement G1 before expanding provider coverage. Do not remove Cycle 19's
existing analysis routes until G4 parity is recorded. Historical cycle records
should retain their dates and add superseding evidence links, rather than have
their old observations silently rewritten.

## 10. What ran in this audit

| Command / check | Observed result |
|---|---|
| `cargo test --workspace` | Library stage: 198 passed, 2 failed; command stopped before the integration stages |
| Explicit Cycle 20 integration suite | 13 passed |
| Explicit pinned Libevent integration suite | 1 passed |
| Reevaluation example | Built and ran; 6 failed semantic expectations, 2 passing controls; exit 1 |
| `lake build` | Succeeded, 27 jobs |
| `lake env lean AuditCycle17.lean` and `AuditCycle20.lean` | Both succeeded; axiom outputs archived |
| Clippy for the new example with `-D warnings` | Blocked by `large_enum_variant` in the already modified `legacy_c.rs` library dependency |
| New example formatting | Formatted with Rust 2024 rustfmt; check recorded with artifacts |

Observed library failures: `for_do_while_break_and_continue_are_imported_as_cfg_edges`
expected Safe but received Unknown; `main_source_scope_excludes_macro_expanded_header_bodies`
failed its `header_helper` inventory assertion. They are baseline observations
from a mutable checkout; this audit does not attribute them to HEAD or endorse
either expected label without further examination.

Rust/cargo versions were 1.98.0. No external model checker, Juliet/SV-COMP corpus,
full Fluent Bit audit, new sanitizer run, native Windows/macOS validation, or
Python design bundle was executed. This document specifies those future
reevaluation gates and records a focused audit; it does not claim that whole
reevaluation has passed.

### Precommit recheck on 2026-09-05

After the independent Cycle 19 changes were committed as `490f515`, the
reevaluation changes were checked again: `cargo test --workspace` passed
201 library tests and 14 integration tests; workspace formatting and Clippy
with `--all-targets -- -D warnings` passed. The reevaluation example still
reported the same six failed expectations and two passing controls, exiting 1
as designed. The earlier failures above remain historical observations, not
current baseline failures. The new logs and baseline identity are recorded in
[`precommit_metadata.json`](reevaluation/precommit_metadata.json).


### Six-finding repair on 2026-09-05

The original probe source is unchanged. All six findings and both controls now
meet their expected verdicts (zero failed expectations, exit 0). The original
logs above remain historical evidence. Current results are in
[`repair_probes.json`](reevaluation/repair_probes.json); code hashes, baseline
commit, tool versions, and commands are in
[`repair_metadata.json`](reevaluation/repair_metadata.json).

| Finding | Repair and regression coverage | Status |
|---|---|---|
| WM01 | Explicit load/store legality tables; test each operation separately with every exposed order, including valid SC operations. | Fixed in the represented atomic subset |
| WM02 | Enforce write/write, write/read, read/write, and read/read coherence against happens-before and modification order; reverse each relation with matching positive cases, including equal-valued distinct writes. | Fixed for these coherence checks |
| WM03 | Constrain SC order by modification order and constrain each SC load's reads-from source by the last preceding SC write. Check all 24 SC store-buffering orders against all four outcomes using a serial-schedule oracle, plus mixed SC/non-SC source cases. The relaxed store-buffering control remains accepted. | Fixed for represented loads/stores |
| WM04 | Derive synchronization from contiguous C11 release sequences of same-thread stores. Test continuation, foreign-store termination, absence/presence of required edges, and overlapping release heads. | Fixed for represented stores |
| VP01 | Built-ins return `Unknown`; the core rejects assertion-only evidence even when every claim exactly matches the request. Mutate predicates, source path, revision, profile, and capability across first-party providers; exercise an executable provider too. First-party checker metadata now identifies the unsupported assertion schema. | Certificate issuance contained; semantic verification remains unsupported |
| VP02 | The core explicitly rejects empty facts, claims, or coverage without an independently established no-obligations proof. Test matching empty proposals through both in-process and actual executable providers, checking that rejection occurs at the evidence firewall. | Empty-scope certificate issuance contained |

The memory rules use [N1570](https://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf)
§5.1.2.4 and §7.17.3, §7.17.7.1–2. The API checks one supplied candidate;
these changes do not establish complete execution exploration or an
implementation soundness theorem. RMWs, compare-exchange, fences, consume, and
mixed-size accesses are not represented by this event API. Mutex and thread
lifecycle adequacy remain in the broader coverage matrix.

Protocol v1 has no core-derived obligation inventory, semantic witnesses, or
independently bound source identity. Consequently **no v1 plugin `Safe` proposal
can obtain a core certificate**, including proposals containing predicates
named `ResourceBalanced` or `ConcurrencySafe`. Checking those names or requiring
more caller-supplied Boolean facts would not repair the trust boundary.
Built-ins also return `Unknown` for false assertions, since these do not supply
reachable counterexamples. Third-party `Unsafe` witness validation remains a
separate open obligation. Manifest capabilities and wildcard targets describe
configuration routing only; they do not establish verified capability/profile
support. Source scopes still validate configuration only.

Previous tests that obtained certificates from Boolean facts now assert
`Unknown`; those old results were not legitimate semantic positive controls.
The original composition assumption-conflict test is retained with internal
certificate fixtures, separately from the public firewall. Existing Cycle 19
analysis routes and their tests remain in place. Capability-specific witnesses,
core-derived inventories (including legitimate empty scopes), source integration,
package identity, process-tree isolation, and Lean implementation bridges remain
open. This is containment of the two reported false-safe cases, not completion
of G2–G6 or all Cycle 20 capabilities.

Validation of the repaired working tree:

- `cargo test --workspace`: 208 library tests and 17 integration tests passed,
  including the pinned Libevent audit. [Log](reevaluation/repair_workspace_tests.log).
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
  [Log](reevaluation/repair_clippy.log).
- `cargo fmt --all -- --check`: passed. [Log](reevaluation/repair_fmt.log).
- `cargo run -p qcore-compiler --example reevaluation_probe`: all eight
  expectations passed. This command is now also a CI gate.

No Lean files changed and no new Lean build or external weak-memory model
checker run is claimed for this repair. The bounded SC schedule oracle and
hand-constructed graph regressions are the executed memory evidence.
