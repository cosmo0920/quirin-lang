# Cycle 17: Research Extensions and Production Closure

**Status: Implementation complete; native Windows acceptance run pending**

The executable and proof workstreams described below are implemented. The
aggregate certificate deliberately still requires separately recorded Linux
and Windows sanitizer runs. This checkout has produced the Linux Rust/Lean
results, but no native Windows result was available during this development
run, so the program-level acceptance gate is not yet recorded as complete.

Cycle 17 promotes every item in `QCore.Metatheory.remainingResearch` from an
explicit post-roadmap boundary into one numbered milestone. It is intentionally
one roadmap task with ordered internal workstreams. Planning it does not expand
the guarantees of the completed Cycle 16 implementation.

## Scope

Cycle 17 must close all six promoted items:

1. principal inference for the implemented usage, effect, resource-vector,
   ownership, and path-constraint fragment;
2. sound summaries for general nonzero-net resource loops;
3. sound analysis of unrestricted conforming legacy C, with precision claims
   stated separately from soundness;
4. a useful categorical unification connected mechanically to the executable
   ledgers rather than supplied only as an analogy;
5. full C11 weak-memory semantics for concurrent source and target behavior;
6. a native concurrent backend with verified custody, cleanup, and observation
   behavior.

Features previously deferred by the concurrency amendment—dynamic task
collections, condition variables, ownership-carrying queues, dynamic mutex
invariants, atomic publication, and reclamation—must be classified explicitly
under the weak-memory/backend workstreams. They cannot enter the accepted
profile without their own executable and proof gates.

## Ordered workstreams

### 17A: Inference and loops

Define the decidable constraint fragment and its normal form before claiming
principality. Implement inference certificates checked independently of the
solver. Extend CFG loop summaries with explicit invariants, termination or
widening rules, and peak/cumulative resource obligations. Prove soundness,
principal-result uniqueness up to normalization, and loop-summary
conservation. Test satisfiable, unsatisfiable, ambiguous, widening, and
non-terminating cases.

### 17B: Native concurrency and the C11 memory model

Give source events and emitted C11 atomics one common execution relation,
including modification order, reads-from, synchronizes-with, and permitted
weak outcomes. Implement a native threaded runtime and emitter without using
the current event-list order as evidence of happens-before. Require litmus
tests, schedule-aware differential tests, ThreadSanitizer where supported, and
native Linux and Windows execution. Prove access authorization, custody and
cleanup preservation, race freedom for accepted non-atomic accesses, and
source/target refinement under the modeled C11 executions.

### 17C: Unrestricted legacy-C analysis

Define "unrestricted" as all conforming C11 translation units after
preprocessing, while retaining a conservative `Unknown` verdict wherever
precision is unavailable. Model pointer provenance, unions, casts, function
pointers, callbacks, dynamic indices, interprocedural ownership transfer,
undefined behavior, atomics, and external summaries. Require differential
parser tests and seeded Safe/Unsafe/Unknown corpora. Prove that `Safe` implies
the stated resource and concurrency properties; completeness is not required.

### 17D: Categorical account and final composition

Define the categorical or triangulated structures, their objects and
morphisms, and a mechanized interpretation into the executable QCore-C
semantics. The construction must explain which ledgers compose uniformly and
which remain deliberately separate. Complete the milestone with one combined
certificate and audited Lean theorem connecting inferred source, looped
execution, native concurrent C11, and imported legacy-C obligations.

## Acceptance gates

Cycle 17 is complete only when all of the following hold:

- each of the six constructors currently listed by
  `QCore.Metatheory.remainingResearch` is discharged by an audited theorem and
  removed from that list;
- the Rust workspace has executable positive, negative, boundary, property,
  differential, sanitizer, and weak-memory litmus coverage for the promoted
  behavior;
- the native concurrent backend runs on Linux and Windows, with platform gaps
  reported as failures rather than inferred from another host;
- unrestricted legacy-C `Safe` results are backed by checked summaries, while
  unsupported or imprecise cases remain `Unknown`;
- source, checked IR, and emitted C11 observations validate for every accepted
  Cycle 17 feature; and
- `cargo test --workspace`, strict formatting and Clippy, a placeholder-free
  `lake build`, and the Cycle 17 theorem-dependency audit all pass.

## Evaluation

This milestone is technically coherent but substantially larger and riskier
than Cycles 13–16. Workstream 17A should precede 17C because legacy-C summary
precision depends on the inference and loop domains. Workstream 17B should be
stable before the final legacy-C concurrency claims, and 17D depends on all
executable semantics being fixed.

The highest-risk claims are principality, unrestricted-C soundness, full C11
weak-memory refinement, and categorical correspondence. Each needs an explicit
formal scope before implementation begins. The native backend is comparatively
direct engineering, but its translation theorem depends on the weak-memory
model. The single Cycle 17 label is therefore suitable as a program-level
roadmap task; its workstreams should be delivered and reviewed independently,
and the cycle must remain incomplete until the aggregate gate passes.

Cycle 18 is the next roadmap milestone. Its event-driven verification model
depends on Cycle 17's native concurrent backend and C11 weak-memory relation.

## Implemented artifacts

- `compiler/src/inference.rs` implements the normalized conjunctive
  usage/effect/resource/ownership/path fragment. Solver output is accepted only
  after the separate certificate checker reconstructs its extrema and rejects
  noncanonical, merely satisfying, ambiguous, or inconsistent output.
- `Flow` now carries an entry requirement. `bounded_loop` summarizes positive
  and negative net changes with checked arithmetic; Cycle 17 certificates
  distinguish termination bounds, widening caps, and unbounded cases for which
  no finite claim is made.
- `analyze_unrestricted_c11` invokes the production C11 preprocessor and maps
  every unsupported projection to `Unknown`. A `Safe` result carries a second
  certificate that rechecks every function and rejects any retained unknown
  instruction. `compiler/examples/cycle17_legacy_audit.rs` exposes the gate for
  external C source trees.
- `compiler/src/weak_memory.rs` checks explicit reads-from, per-object
  modification order, synchronizes-with, program-order/happens-before,
  sequentially-consistent order, coherence, non-atomic races, and source/target
  observations. Litmus coverage includes permitted relaxed store buffering and
  release/acquire message passing.
- `compiler/src/native_concurrency.rs` runs chosen schedules on native worker
  threads and checks atomic observations, synchronized custody transfer,
  exactly-once destruction, and joins. Dynamic task collections, condition
  variables, ownership queues, dynamic mutex invariants, atomic publication,
  and reclamation have explicit `Accepted`/`Unknown` profile entries.
- `compiler/src/research_extensions.rs` interprets loop and native cleanup
  ledgers as endpoint-checked morphisms, keeps effects/path facts/C11 relations
  deliberately separate, and creates the aggregate Cycle 17 certificate only
  after all component and platform checks pass.
- `proof/QCore/ResearchExtensions.lean` proves principality uniqueness,
  nonzero-loop conservation, conservative legacy-C acceptance, weak-memory
  race freedom and observation equality, native custody/cleanup and platform
  evidence, ledger composition, six per-workstream discharge theorems, and the
  combined certificate theorem. `proof/AuditCycle17.lean` audits their
  dependencies.

## Verification commands

```sh
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
if rg -n '\b(sorry|admit)\b|^[[:space:]]*axiom\b' proof --glob '*.lean'; then exit 1; fi
cd proof
lake build
lake env lean AuditCycle17.lean
```

The adjacent CFL checkout was also passed through the unrestricted importer;
the exact source revision, command, and conservative result are recorded in
`docs/cycles/CYCLE_17_CFL_VALIDATION.md`.
