# Revision 0.3 review: R08 / R10 contract and evidence consumers

2026-09-07. Bounded review of current implementation at integrated base
`c7039ad`, against `unverified_specs/REVIEW.md` R08/R10. The uploaded review's
archive and verification limitations are retained: its source-inspection findings
are not execution evidence for this checkout. This report records local checks
and one concrete replay hardening; it does not close general source refinement,
whole-program composition or the complete revision 0.3 review.

## Concrete gap and repair

The public `ContractObservationReplay` accepted caller-built core tasks whose
obligations changed their observed scope into a function scope, named an
unobserved run or contract module, or advertised unexamined path/schedule/operation
bounds. The provider cannot perform this mutation through the checked request
protocol: the core task envelope is independently compared. The defect was that
the replay API accepted those stronger/misbound caller-built claims when supplied
a consistent envelope.

The archived baseline regression demonstrates seven mutations accepted both by
direct replay and by the host with a caller-built task. Replay now requires:

- The existing exact v2.0 observed-run bounds at task and obligation level,
  plus the existing explicit trusted-observation premise and equal assumptions.
- The deriver's deferred `ObservedRuns` scope marker (empty run-id set).
  Fabricated function scopes or explicit run selections are rejected. Actual
  run identities and complete native bindings remain in the certificate's
  replayed witness; the empty marker is not a claim that no runs were checked.
- Exact required-hook function and contract-module identity for each observed
  contract obligation. Digest-pinned source checks and hook coverage still run.

The normal derived inventory and accepted witness semantics are unchanged.
No new scope or witness kind is supported. The reduced Lean model computes the
scope/bounds/module condition from structured values and proves that acceptance
preserves it, and that a substituted claim rejects. This is a reduced structural
model, not a proof of Rust parsing, digest authentication, native execution or C
source refinement.

## Consumer coverage and limits

| Consumer | What acceptance establishes | Existing fail-closed evidence / remaining premise |
| --- | --- | --- |
| Frontend parser | Structural syntax and clause representation | New `contract_consumers` test demonstrates parsing can succeed while resource loading or observation evaluation rejects. Parsing alone is not a contract proof. |
| Resource contract loader | Complete translation of selected `resource_contract_v1` clauses into declared custody modes | `unsupported_or_incomplete_contracts_fail_atomically` and whole-corpus wrong-consumer rejection cover unsupported clauses, missing modes, mixed declarations and empty inputs. `certificates_replay_exact_assumptions_and_reject_table_mutation` rechecks exact retained source assumptions. These files remain trusted declarations of external behavior. |
| Observation evaluator | Every supported `requires` / `ensures normal` predicate over supplied scalar values | New consumer test covers false/missing/unsupported postconditions and missing post-state behind false preconditions. Native delete/cleanup tests mutate individual predicates and recorded facts. `OutsidePreconditions` is distinct and earns no coverage; `check_contract_facts` rejects it as a transition. Values are not authenticated by the evaluator. |
| Typed C import and specialized source interpreters | Bounded syntax projections and specifically interpreted facts | H4/H5 profile tests retain exact declared assumptions and reject malformed selections; source summary constructors remain controlled. Opaque calls confer no effect proof. Direct revision arguments are caller assertions; pinned audit checks identity separately. General source/model refinement is open. |
| Protocol v1 host | No positive safety acceptance | Existing in-process and process regressions reject all Safe proposals, including empty scopes and matching Boolean facts; unverified Unsafe is Unknown. Reproducibility metadata alone does not discharge semantics. |
| Evidence v2 host / replay | Core-derived obligations discharged by capability-specific replay under retained task bounds and assumptions | Existing mutation corpus covers source/task/capability/coverage substitution, missing witness/checker/obligations, incomplete derivation, false/missing observations, stale contracts, profile mismatch, duplicate runs and explicit distrust. New regression closes scope/module/bounds relabelling. Unsupported counterexamples remain Unknown. |
| Composition with inventory | Conjunction over the exact required obligation-ID set | Existing tests reject missing/extra coverage and empty scope/results; conflicting identities/assumptions/bounds reject. Component certificates retain their own tasks and witnesses. This is not a source/native/model refinement theorem or a whole-program claim. |

Native run bindings are retained declared evidence, with format/profile checks;
the `observations_trusted` assumption is material. A recorded executable digest
is not independently authenticated by replay or equated with QCore input text.
The host's public checker injection point is trusted application integration;
provider process responses cannot install a replay implementation. Those are
explicit API trust boundaries, not cryptographic guarantees.

The compatibility composition result retains a leading component certificate;
callers needing every component use the existing certificate collection or
per-capability dispatch outcomes. Do not read that leading certificate as an
aggregate source-proof object. No production/raw-unit promotion, coverage
increase, new native observation or CT mapping follows from this increment.

## Validation

`observation-baseline.log` records the failing negative regression on unchanged
base replay, including every accepted mutation. `observation-fixed.log` records
all eight v2 integration tests passing after repair; `consumer.log` covers the
new consumer distinction regression. The complete standing gate and all Lean
audits are archived with return codes and counts in `validation.json`.

The full pinned C audit was not rerun: no C gateway, profile, source-import,
audit-driver or pinned-source logic changed. The prior H4 full185 evidence remains
its own evidence package; it is not relabelled as an execution of this change.
