# O05 — Aggregate lifecycle and Cycle 18/19 composition — 2026-09-08

**Card:** O05, priority P3, workstreams W3/W4/W5, lane Integrator, kind
`bounded_design`, scope M. Base: `c60fd91` ("Consolidate the second delegated
T04/P05/P06/P07/O04/F01/CT increment into the status ledger").

**Deliverable:** a compositional contract and counterexample suite for one
aggregate parent and one wrapper lifecycle across admitted callbacks.

Delivered under the stricter closing standard — design **plus** a bounded Rust
model **plus** Lean-proven theorems:

| Artefact | Content |
|---|---|
| [`docs/design/AGGREGATE_LIFECYCLE_COMPOSITION.md`](../../design/AGGREGATE_LIFECYCLE_COMPOSITION.md) | the contract: subject, relation, premises, unsupported classes, counterexample inventory, follow-ons |
| `crates/qcore-core/src/aggregate_composition.rs` | the relation, its 28 typed refusals and its certificate |
| `crates/qcore-core/tests/unit/aggregate_composition.rs` | the reference program and **52** rows |
| `proof/QCore/AggregateComposition.lean` | the model of the decisions, **39** audited theorems |
| `proof/AuditO05.lean` | the axiom surface |

**Nothing is promoted.** Raw Cycle 19 units stay 2 Safe / 42 provisional Unsafe /
137 Unknown plus four data-only units. CT coverage stays 15/50 mapped. The
contract corpus stays 59 modules. `qcore-schemas` stays 5.0.0. No diagnostic code
was added, removed or changed; no gate was weakened; no pinned expectation moved.
No whole-Monkey and no whole-composition claim about any upstream codebase is
made.

---

## 1. Integrator decision recorded first: O03 is absent from this branch

The card brief names O03's target-qualified `close(2)` post-state relation as a
landed dependency, at `crates/qcore-core/src/os_endpoint.rs` with
`proof/QCore/OsEndpointPostState.lean` and
`docs/reports/remaining-o03/README.md`. **None of the three exists on this
branch.** Measured:

```
$ ls crates/qcore-core/src/os_endpoint.rs
ls: cannot access '...': No such file or directory
$ ls proof/QCore/OsEndpointPostState.lean
ls: cannot access '...': No such file or directory
$ ls docs/reports/remaining-o03
ls: cannot access '...': No such file or directory
```

T05/T06/T08 and P08 are likewise absent; O04, P05 and E03 are present, and the
base commit is the second delegated wave's consolidation.

**Decision.** Do not fabricate an O03 dependency and do not stall. The relation
defines the seam O03 plugs into and names the gap:

* `CustodyEvidence::ObservedEndpointPostState` is the evidence kind a
  target-qualified post-state relation would supply;
* `DeclaredPremise::EndpointPostStateSupplied` names who has to supply it; and
* `UnsupportedClass::OsCustody` refuses the whole composition when nobody does.

Reconciling the two vocabularies once O03 lands is recorded as **O05-O2**. No
claim in this card depends on O03 having landed.

---

## 2. What the relation decides

`check_aggregate_composition(program) -> AggregateCompositionResult`, verdict
`Composed` or `Unknown`. **There is no violation verdict.** Every input is a
record another pass supplied, so a contradictory record is a reason to say
nothing, never a reason to accuse — the same fail-closed direction O04 pins for
one exit (`an_unmodeled_path_alleges_no_residual`) and E03 pins for certificate
merging (`composition_is_never_a_violation`).

The subject is the ChunkIO-shaped aggregate O00 mapped — root, chunk object,
chunk name, backend child, descriptor endpoint, mapping endpoint — plus the
Fluent-Bit-shaped wrapper with its own two allocations, its publications and one
hold on the aggregate's chunk. The design document carries the full
specification; this section records only what each acceptance item became.

### Acceptance 1 — child allocations and OS endpoints stay distinct across parent release

| Rule | Refusal | Lean |
|---|---|---|
| no two members share `(kind, generation)` | `DuplicateMemberIdentity` | `duplicate_identities_are_unknown`, `a_composed_program_separates_every_identity` |
| every member acquired once, released once, never released before acquired, and the root released | `MemberNeverAcquired`, `MemberAcquiredTwice`, `MemberNotReleased`, `MemberReleasedTwice`, `ReleaseBeforeAcquire`, `ParentNotReleased` | six theorems plus `a_composed_program_releases_every_member` / `a_composed_program_acquires_every_member` |
| two members sharing a raw OS number have disjoint live intervals; a retained or unresolved member never stops being live | `AddressReusedWhileLive` | `a_reused_live_address_is_unknown` |
| only an OS endpoint may be retained, and only to a declared external custodian | `RetainedAllocation`, `RetainedEndpointWithoutCustodian`, `RetainedEndpointCustodianNotExternal` | `a_retained_allocation_is_unknown`, `a_retained_endpoint_without_a_custodian_is_unknown` |
| a status fact is never custody evidence | `StatusFactIsNotCustodyEvidence` | `no_status_fact_discharges_custody`, `a_composed_program_offers_no_status_fact` |
| an unresolved disposition refuses under its own reason and alleges nothing else | `UnresolvedDisposition(reason)` | `an_unresolved_disposition_is_unknown`, `an_unresolved_member_never_composes` |

The status-fact rule is the direct encoding of Cycle 19's own conclusion — "Down-ring
membership and a zero up counter therefore cannot discharge descriptor custody"
— together with O04's `a_success_looking_integer_does_not_discharge_the_identity`.
The address rule admits CT50's positive direction (reuse after release) and
refuses reuse across a **retained** epoch, which is the state the Cycle 19
suppressed-close fixtures actually leave behind.

### Acceptance 2 — callback quiescence and task/guard prerequisites before teardown

Decided at the teardown point, the index of the first parent release:

| Rule | Refusal | Lean |
|---|---|---|
| a callback whose declared authority touches a member payload is quiescent | `CallbackNotQuiescent(state)` | `a_live_callback_over_a_member_payload_is_unknown`, `a_composed_program_quiesces_every_member_callback` |
| every task a callback started is declared and joined | `CallbackSpawnedTaskUnaccounted` | `an_unaccounted_spawned_task_is_unknown` |
| every task capturing a member is joined — `Completed` is not enough | `TaskNotJoined(state)` | `an_unjoined_task_capturing_a_member_is_unknown`, `a_composed_program_joins_every_capturing_task` |
| every guard over a footprint touching a member is released | `GuardOutstanding` | `an_outstanding_guard_over_a_member_is_unknown` |

Refusing `Completed` is deliberate and rests on existing measured evidence: V02
pinned that a completed-but-unjoined child keeps its captures in
`Custodian::Capsule`, and CT14/CT37 execute that on real Linux threads.

Two admitting rows keep the rule from being blunt and are tested: a callback
whose declared authority touches no member payload may be `Running`, and a guard
over a disjoint footprint may be `Held`. Both rest on the declared-completeness
premises in §3.

### Acceptance 3 — durability stated separately; unsupported classes prevent the claim

`DurabilityPremise` is a **separate certificate field** from `DeclaredPremise`,
and offering one as release evidence refuses
(`DurabilityAssumedAsDischarged`; Lean `no_durability_premise_discharges_custody`
and `a_composed_program_discharges_no_durability_premise`). All five premises are
exercised. A composed claim therefore never means "the data reached stable
storage"; Cycle 19 measured three ways custody and durability come apart
(cleanup continues after a failed sync, an unmapped commit clears the transaction
without syncing, and a prior successful sync survives a later failure).

Six unsupported classes — `Capacity`, `Metadata`, `Schedule`, `Callback`,
`Backend`, `OsCustody` — each refuse the whole composition. Lean:
`any_unsupported_class_blocks_the_composition` is quantified over the class,
`an_unadmitted_callback_blocks_the_composition` derives the callback class from
an unadmitted record, and `a_composed_program_has_no_unsupported_class` is the
soundness direction. A test pins that an otherwise perfect program carrying one
unsupported class refuses with **exactly** that one refusal.

### The wrapper lifecycle

`WrapperOverlapsAggregate`, `WrapperReleasesForeignMember`,
`OwnedHoldAlsoReleasedByParent`, `BorrowedHoldReleasedByWrapper`,
`BorrowedHoldOutlivesParentRelease`, `WrapperPublicationUnreclaimed`,
`WrapperPublicationSurvivesTeardown`. The owned/parent pair is O00's blocker
**B02** — the constructor both links the chunk into the stream and returns it as
an owned result — and the relation refuses that shape by name rather than
choosing a winner. The publication rules are O00's **B09**/**B10**.

---

## 3. Premises, named and not derived

Nine, carried on the certificate:

`ConstructorContract`, `DestructorContract`, `BackendProfile`,
`CallbackAuthorityDeclared`, `GuardFootprintDeclared`, `TaskLedgerSupplied`,
`ExitResidualsSupplied`, `EndpointPostStateSupplied`, `ObservationsTrusted`.

Two more are inherited and not proved: that the reader extracts a disposition
faithfully, and C effective types (C02 §2 through O04 §3). The design document
§4 gives each premise its provenance. **The relation decides nothing about any of
them**; a composed claim is exactly the conjunction of §2's checks under them.

**Suppressed syscalls are not promoted.** Cycle 19's fixtures suppress selected
`close`/`open`/`munmap`/`fsync` calls with `EIO` and do not execute the real
syscall; their own record says the post-state of an arbitrary kernel error is not
inferred. `UnresolvedReason::SuppressedSyscall` exists so a disposition resting
on a suppressed call is refused *with that reason*, and
`UnsupportedClass::OsCustody` refuses the composition when no post-state relation
decides the endpoint at all.

---

## 4. Counterexample inventory

52 test functions over one reference program: root, child allocation, child
name, backend, descriptor endpoint retained by a named external custodian,
mapping endpoint, wrapper object, wrapper table, one publication, one quiescent
callback with one joined spawned task, one joined task, one released guard.
Every negative row asserts the verdict is `Unknown`, the exact typed refusal is
present, and **no certificate is published**; the reference program composing is
what makes each refusal attributable to its own single mutation.

| Group | Rows |
|---|---:|
| admitting rows (reference, owned hold, unrelated callback authority, unrelated task, address reuse after release, disjoint guard footprint, certificate premise fields) | 7 |
| identity separation | 8 |
| dispositions and evidence | 7 |
| ledger completeness (parent never released, empty aggregate) | 2 |
| quiescence | 7 |
| unsupported classes | 3 |
| the wrapper | 9 |
| structural (malformed-only reporting, wrapper step without a wrapper, containment cycle, reclaim before publish) | 4 |
| relation-wide (sorted/deduplicated, independent defects, never a violation, determinism under reordering, typed rendering) | 5 |
| **total** | **52** |

Several rows loop over a whole vocabulary — five unresolved reasons, four status
facts, five durability premises, six unsupported classes — so the 52 functions
decide considerably more than 52 cases.

Every one of the 28 `CompositionRefusal` constructors is reached by at least one
row. Four of `MalformedKind`'s seventeen structural kinds are reached
(`UnknownMember`, `WrapperStepWithoutWrapper`, `ParentCycle`/`RootHasAParent`,
`PublicationReclaimedBeforePublish`); the other thirteen are exercised only
through the shared early-return path, which is what
`a_malformed_program_reports_only_structural_refusals` pins.

---

## 5. Lean inventory

`proof/QCore/AggregateComposition.lean`, printed by `proof/AuditO05.lean`.
**39 audited theorems.** Axiom surface measured: `propext`, `Classical.choice`,
`Quot.sound` and nothing else. No new axiom, no `sorry`, no placeholder.
`scripts/check_lean_audits.py` moves from **23 audits to 24**, and `AuditAll`'s
whole-project closure check passes with the module imported into `QCore.lean`.

| Family | Theorems |
|---|---:|
| structural discipline (empty aggregate, malformed-only reporting) | 3 |
| unsupported classes | 3 |
| identity separation and address reuse | 3 |
| the release ledger, evidence and dispositions | 10 |
| quiescence | 4 |
| the wrapper | 4 |
| standing properties (never a violation, composed iff refusal-free, and six soundness directions) | 9 |
| non-vacuity witnesses | 3 |

The three non-vacuity witnesses are the guard against a model that proves every
refusal by accepting nothing: `the_reference_aggregate_composes` decides a
complete aggregate `Composed`, and two single mutations of it —
`a_live_callback_makes_the_reference_unknown` and
`a_reused_live_descriptor_number_makes_the_reference_unknown` — decide `Unknown`.
All three are closed by `decide`.

Two abstractions from the Rust relation are recorded in the module header rather
than glossed:

* the **structural refusals are an input field** in Lean rather than a computed
  function, and every semantic theorem is stated under the hypothesis that the
  list is empty — which is exactly the early return the Rust relation takes; and
* **certificate premise sets are not modeled**, because the relation decides
  nothing about them.

Correspondence between the Rust and Lean relations is by construction and
review, not by extraction and not by a digest-checked transcription in F01's
style. That is recorded as **O05-O7**.

---

## 6. Applying the contract to the real subject today

With the evidence that exists on this branch, the §2 subject composes to
**`Unknown`**, for named reasons. That is the correct answer and the point of the
exercise:

| Member | Blocker | Refusal |
|---|---|---|
| chunk object | O00 B02 — the constructor publishes into `st->chunks` and returns an owned result | `UnresolvedDisposition(UnmodeledAggregateStore)` |
| backend child, backend path | O00 B04 — `ch->backend` published before the child's failure arms | `UnresolvedDisposition(UncontractedCallee)` |
| descriptor, mapping | no target-qualified OS post-state relation on this branch (§1) | `UnsupportedClass::OsCustody` |
| wrapper publications | O00 B09/B10 — the msgpack callback and one of four hash tables | `WrapperPublicationSurvivesTeardown` |
| callbacks | the reactor route binds no C source (P05 §8) | `UnsupportedClass::Callback` |

So the Cycle 18 / Cycle 19 composition **does not close**, and each blocker now
attaches to a specific member and a specific typed class. This is a
specification result, not a verification result, and it is not an upstream defect
claim about Fluent Bit, ChunkIO, Monkey or Libevent.

---

## 7. Boundaries, recorded open

* **O05-O1 — no production caller.** The relation is a dependency-free
  `qcore-core` model with its own tests. It is not wired into `qcore-audit`, the
  Cycle 19 compatibility adapter or any provider route, and the card scopes it
  that way on purpose.
* **O05-O2 — no OS endpoint post-state relation on this branch** (§1). The
  evidence kind and the premise are the seam; O03 supplies the relation.
* **O05-O3 — no producer from source.** Nothing derives an
  `AggregateCompositionProgram` from C. O04's `LegacyExitResidual` is the natural
  source for `ExitResidualDischarged` dispositions; the mapping is not written,
  and O00's B02/B03/B04/B08/B09/B10 sit on that path.
* **O05-O4 — one teardown point.** Callback, task and guard states are declared
  "as at the first parent release". A teardown interleaved with registration, or
  a re-registration afterwards, is not expressible.
* **O05-O5 — one wrapper, one hold.** Two wrappers over one aggregate, a wrapper
  over two members, nested wrappers, and a wrapper that deliberately outlives the
  aggregate are all outside the shape.
* **O05-O6 — flow-insensitive containment.** Containment is a static `parent`
  link; a member that moves between containers mid-schedule (a chunk moving
  between up and down rings) is not checkable. Sound here only because ring
  membership is a `StatusFact` that discharges nothing.
* **O05-O7 — no Rust/Lean refinement** (§5).
* **O05-O8 — no schema record.** Nothing crosses a process boundary; carrying an
  aggregate composition in `qcore-schemas` would be a major bump from 5.0.0 and
  is deliberately not attempted.
* Out of scope by the card and untouched: universal failure recovery, a
  whole-Monkey theorem, full native refinement, and models of capacity, metadata
  or schedules.

---

## 8. Files changed

| Path | Change |
|---|---|
| `crates/qcore-core/src/aggregate_composition.rs` | new — the relation |
| `crates/qcore-core/tests/unit/aggregate_composition.rs` | new — 52 rows |
| `crates/qcore-core/src/lib.rs` | one added `pub mod` line |
| `proof/QCore/AggregateComposition.lean` | new — 39 audited theorems |
| `proof/AuditO05.lean` | new — the axiom surface |
| `proof/QCore.lean` | one added import |
| `docs/design/AGGREGATE_LIFECYCLE_COMPOSITION.md` | new — the contract |
| `docs/reports/remaining-o05/README.md` | this record |

No existing production body, test, contract, profile, schema or pinned artefact
was edited. `AGENTS.md`, `CLAUDE.md`, `docs/reevaluation/ct_manifest.json`,
`docs/reviews/r0_3/acceptance.json` and
`docs/reviews/r0_3/acceptance_matrix.json` are owned by the integrator and are
untouched; proposed text is in §10.

---

## 9. Gate results

Linux, this worktree, `git submodule update --init --recursive` run once
(`third_party/libevent` pin test passes). Lean run with
`PATH="$HOME/.elan/bin:$PATH"`.

| Gate | Command | Result | Exit |
|---|---|---|---|
| Workspace tests | `cargo test --workspace` | **1,193 passed / 0 failed / 1 ignored** (baseline 1,141 + 52) | 0 |
| Format | `cargo fmt --all -- --check` | clean | 0 |
| Clippy | `cargo clippy --all-targets -- -D warnings` | clean | 0 |
| Probe | `cargo run -p qcore-compiler --example reevaluation_probe` | ran | 0 |
| Follow-up | `cargo run -p qcore-compiler --example reevaluation_followup` | ran | 0 |
| Python | `python3 -B -m unittest discover -s scripts/tests` | **43 ran, OK** | 0 |
| Lean build | `cd proof && lake build` | 68 jobs, completed, no warning | 0 |
| Lean audits | `python3 -B scripts/check_lean_audits.py` | **24 audits passed** (baseline 23) | 0 |
| Provider tests | `cargo test --locked --offline --manifest-path providers/Cargo.toml --workspace` | **56 passed / 0 failed** | 0 |
| Provider fmt | `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | clean | 0 |
| Provider clippy | `cargo clippy --locked --offline --manifest-path providers/Cargo.toml --all-targets -- -D warnings` | clean | 0 |

**The 185-unit Cycle 19 corpus was not re-measured, and is unchanged by
construction.** This card adds one new `qcore-core` module that nothing links: no
analysis, gateway, audit, contract, profile or provider code path reaches it, and
no existing file's behaviour changed. The four archived artefacts are left
untouched and **CORPUS-REPIN** remains the standing follow-up it already was,
still naming C02, G02 and O04. The canonical CMetrics audit was likewise not
re-run: this worktree has no prepared Clang CMake build tree and the Fluent Bit
checkout is off the profile pin that preparation requires — the same
pre-existing limitation O04 §5.1 and P05 §6 recorded.

Baseline measured in this worktree before any edit, for the record:

```
$ cargo test --workspace   # at c60fd91, no files added
1141
```

---

## 10. Integrator notes (proposed, not applied)

* **AGENTS.md / CLAUDE.md status ledger.** The workspace test count moves 1,141
  → 1,193 and the Lean audit count 23 → 24. No verdict, CT (15/50),
  contract-corpus (59) or schema (5.0.0) figure changes. Suggested paragraph:

  > Aggregate lifecycle composition (O05): a bounded composition relation in
  > dependency-free `qcore-core` decides whether one aggregate parent, one
  > wrapper lifecycle and their admitted callbacks, tasks and guards may be
  > published as a single whole-composition claim. Its verdict is `Composed` or
  > `Unknown` and it has no violation verdict: every input is a record another
  > pass supplied, so a contradictory record refuses with a typed reason and
  > alleges nothing. Logical `(kind, generation)` identities must be distinct and
  > two members sharing a raw OS number must have disjoint live intervals, where
  > a retained or unresolved member never stops being live; callback quiescence,
  > joined tasks and released guards are prerequisites at the teardown point,
  > with a completed-but-unjoined task refused; durability premises sit in their
  > own certificate field and are refused as release evidence, as are ring
  > membership, a zero up counter, a success-looking integer and a nulled field;
  > and any declared capacity, metadata, schedule, callback, backend or
  > OS-custody class refuses the whole claim. 52 Rust rows over one reference
  > program and 39 audited Lean theorems with no new axiom. Applying the contract
  > to the ChunkIO/Fluent-Bit subject today yields `Unknown` with each blocker
  > attached to a named member and class; the Cycle 18 / Cycle 19 composition
  > does not close. Not wired into any production route, no producer from source,
  > and O03's OS post-state relation is absent from this branch. Evidence:
  > `docs/reports/remaining-o05/README.md` and
  > `docs/design/AGGREGATE_LIFECYCLE_COMPOSITION.md`. Nothing is promoted.

* **`docs/reviews/r0_3/acceptance_matrix.json`.** O05 closes as *designed and
  modeled*, carrying three evidence classes: the design document, the 52-row
  Rust counterexample suite, and the 39 `QCore.AggregateComposition` theorems.
  Eight boundaries stay open as O05-O1 … O05-O8.
* **`docs/reevaluation/ct_manifest.json`.** No CT case is mapped or unmapped by
  this card; coverage stays 15/50.
* **O03 dependency.** The plan's dependency list for O05 should record that O03
  had not landed on the base commit and that O05 was executed against the seam
  rather than the relation (§1).
