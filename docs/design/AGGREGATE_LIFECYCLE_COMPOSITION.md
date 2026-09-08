# Aggregate lifecycle and Cycle 18/19 composition

Card **O05**. Workstreams W3, W4, W5. 2026-09-08.

A compositional contract for **one aggregate parent** and **one wrapper
lifecycle** across **admitted callbacks**, with the counterexample suite that
decides it. The contract is executable twice: as
`crates/qcore-core/src/aggregate_composition.rs` (52 tests) and as
`proof/QCore/AggregateComposition.lean` (39 audited theorems).

This document is the specification. It states what the relation decides, every
premise it assumes and does not derive, every class of behaviour that refuses a
whole-composition claim outright, and the follow-on units. It promotes nothing:
no raw Cycle 19 unit changes class, no CT case is mapped, no contract corpus
count moves, and no whole-Monkey or whole-composition claim about any upstream
codebase is made anywhere in it.

---

## 1. Why a composition contract is the missing piece

Cycle 19's descriptor-failure record closes with the exact statement this card
answers:

> Remaining work includes … aggregate source custody, remaining Fluent Bit
> wrapper lifecycle and full Monkey/Cycle 18 event composition. **The descriptor
> transition model must retain these unresolved obligations when composed with
> those callers.**
> — `docs/reports/cycle19-file-descriptor-failures/README.md`

Four separate lines of evidence exist and none of them composes:

| Evidence | What it establishes | What it cannot say |
|---|---|---|
| Cycle 19 native observations (`cycle19-*`, ~20 reports) | per-syscall post-states on fixed injected sequences: 40 allocation generations released, five mapping epochs, one descriptor residual reclaimed separately by the harness | nothing about a production caller, an arbitrary kernel failure, or what the aggregate above the chunk owes |
| O04 (`remaining-o04`) | per-exit residuals on three modeled failure arms of one caller, all naming one acquisition site | the constructor's own rollback and success exits stay Unknown; its declared contract is a **named premise**, not a derived fact |
| Cycle 18 reactor model (`qcore-core::event_reactor`) | callback assignment inventory, generation-safe lifecycle, `L06NotQuiescent` | binds no C source; P05 moved one capability through the process boundary and recorded that it **has no production caller** |
| T02/T03 mutex invariants and `scoped_tasks` | guard footprints, completion capsules, creator-only join | are not connected to any aggregate's teardown |

Each is sound in its own scope and each *stops* exactly where the next begins.
The composition question — "may these be published as one claim?" — has, until
this card, had no relation at all, so the honest answer to it has been "there is
nothing to answer with". The relation below is that answer, and its default is
refusal.

---

## 2. The subject: one aggregate parent, one wrapper

### 2.1 The aggregate parent

The shape the relation is built for is the ChunkIO container that Cycle 19
measured, with O00's identity labels
(`docs/reports/remaining-o00/README.md` §3):

```
  stream / context aggregate            (root)
    └── ID-P  chunk object              (heap allocation)
          ├── ID-N  chunk name          (heap allocation)
          └── ID-B  backend child       (heap allocation)
                ├── ID-Q  backend path  (heap allocation)
                ├── ID-D  descriptor    (OS endpoint)
                └── ID-M  mapping       (OS endpoint)
```

Two asymmetries O00 recorded make this the right subject and not a toy:

* **ID-D and ID-M are acquired conditionally.** O00 §2.1 E6a: a *successful*
  construction may hand back a live chunk with no descriptor and no mapping.
  A composition contract that assumes an endpoint exists because its parent does
  is already wrong.
* **A retained endpoint is a real, measured outcome.** Cycle 19's descriptor
  failures leave "one descriptor [surviving] parent release … reclaimed
  separately by the harness". That is neither a discharge nor a leak; it is a
  residual with a custodian outside the aggregate, and the contract must be able
  to *say* that without either promoting or accusing.

### 2.2 The wrapper lifecycle

The wrapper is the Fluent Bit input-chunk object, again with O00's labels:

```
  ID-IC  wrapper object                 (heap allocation, its own tree)
    └── ID-RM  routes mask              (heap allocation)
  holds: ID-P (the aggregate's chunk)
  publishes: ID-PB — the aggregate store, the msgpack callback, the instance
             list, and one of four hash tables
```

The wrapper is where O00's blockers B08, B09 and B10 live: a pointer-field
custody transfer, a callback that retains the wrapper, and a hash-table
publication. In the relation these are not modeled away — they are exactly what
`Publish` / `Reclaim` and the `WrapperPublicationSurvivesTeardown` refusal are
for.

### 2.3 The admitted callbacks

Callbacks enter through the Cycle 18 vocabulary: an inventory of records each
carrying a **declared complete payload authority** and an **admitted** flag.
`callback_quiescent_at_teardown` is already a reserved evidence-v2 property name
on the P05 route, and `EventDiagnostic::L06NotQuiescent` is already the reactor
model's own refusal. This contract consumes the same notion at the aggregate
level; it does not re-derive it.

**A callback the reactor family does not admit is an unsupported class, not an
absent one.** `admitted: false` contributes `UnsupportedClass::Callback` and
refuses the whole composition. This is the direct analogue of P05's rule that an
empty accepted-build list is `UnsupportedBackend`, never acceptance.

---

## 3. The relation

`check_aggregate_composition(program) -> AggregateCompositionResult`.

**Verdict.** `Composed` or `Unknown`. There is deliberately no violation
verdict, and this is not a convenience: every input is a record another pass
supplied, so a contradictory record is a reason to say nothing, never a reason
to accuse. Lean: `composition_is_never_a_violation`,
`composed_iff_no_refusal`. This is the aggregate analogue of
`QCore.ExitResiduals.an_unmodeled_path_alleges_no_residual` and of E03's
`composition_is_never_a_violation`.

### 3.1 Inputs

| Field | Meaning |
|---|---|
| `members` | every tracked identity: the root, heap allocations and OS endpoints, each with a `kind`, a `generation`, an `Owner` (aggregate or wrapper), a containment `parent`, an optional raw `address` and its payloads |
| `schedule` | the ordered lifecycle: `Acquire`, `Publish`, `Reclaim`, `DropBorrow`, `Release` |
| `callbacks` | the declared assignment inventory: state, complete payload authority, admitted flag, spawned tasks |
| `tasks`, `guards`, `footprints` | the scoped-task and mutex-guard ledgers |
| `wrapper` | the held member, the hold mode (`Owned` / `Borrowed`) and the publications |
| `external_custodians` | who outside the aggregate may hold a retained endpoint |
| `unsupported` | classes the supplied records do not cover |
| `declared_premises`, `durability_premises` | carried onto the certificate; see §4 and §5 |

The **teardown point** is the index of the first `Release` performed by the
parent. Quiescence is decided against it.

### 3.2 Acceptance item 1 — child allocations and OS endpoints stay distinct across parent release

Three separate rules, because "distinct" means three different things here.

**1a. Logical identity.** No two members share `(kind, generation)`. A
descriptor epoch and its successor are distinct identities even at the same
number. Refusal: `DuplicateMemberIdentity`. Lean:
`duplicate_identities_are_unknown`, `a_composed_program_separates_every_identity`.

**1b. The release ledger.** Every member is acquired exactly once and released
exactly once, and no release precedes its acquisition. Refusals:
`MemberNeverAcquired`, `MemberAcquiredTwice`, `MemberNotReleased`,
`MemberReleasedTwice`, `ReleaseBeforeAcquire`, `ParentNotReleased`. Lean:
`an_unreleased_member_is_unknown`, `a_member_released_twice_is_unknown`,
`a_never_acquired_member_is_unknown`,
`a_release_before_its_acquisition_is_unknown`,
`a_composition_that_never_releases_the_parent_is_unknown`, and the two soundness
directions `a_composed_program_releases_every_member` /
`a_composed_program_acquires_every_member`.

**1c. Raw address separation.** Two members that share a raw OS-visible number —
a descriptor value, a mapping address — must have **disjoint live intervals**. A
member whose disposition is `Retained` or `Unresolved` never stops being live,
so nothing may take its number. Refusal: `AddressReusedWhileLive`. Lean:
`a_reused_live_address_is_unknown`.

This is the rule CT50 measures natively (two owned lifetimes, `MAP_FIXED_NOREPLACE`,
the same numeric address, distinct generations 1 and 2) and the rule the Cycle 19
reopen evidence states in the other direction ("numeric descriptor and mapping-address
reuse is allowed **after the old epoch is released**"). The relation admits the
first and refuses the second, and both directions are tested.

**Dispositions.** A release step records one of three outcomes:

| Disposition | Meaning | Rule |
|---|---|---|
| `Released` | released here, exactly once | admitted |
| `Retained { custodian }` | still live afterwards, in the hands of a custodian **outside** the aggregate | admitted only for an OS **endpoint**, and only with a custodian the program declares external |
| `Unresolved(reason)` | neither established | always refuses, under its own reason |

`RetainedAllocation` refuses a retained *heap* allocation: a heap allocation that
survives parent release is a leak, not a residual. Cycle 19's own accounting
holds the same line — all 40 allocation generations released, one *descriptor*
residual. `RetainedEndpointWithoutCustodian` refuses an unattributed residual;
`RetainedEndpointCustodianNotExternal` refuses handing one to something inside
the aggregate. Lean: `a_retained_allocation_is_unknown`,
`a_retained_endpoint_without_a_custodian_is_unknown`.

`UnresolvedReason` names why, and the name is load-bearing:
`SuppressedSyscall`, `UncontractedCallee`, `UnmodeledBranch`, `UnknownBackend`,
`UnmodeledAggregateStore`. Lean: `an_unresolved_disposition_is_unknown` and the
promotion-direction `an_unresolved_member_never_composes`. A test asserts that an
unresolved member produces *only* its own reason — never `MemberNotReleased`,
never `MemberReleasedTwice`.

**Evidence, and what is not evidence.** A release step names where its
disposition came from. Three kinds are admitted — a per-exit residual ledger
(O04), an observed OS endpoint post-state, a declared destructor contract — and
two are refused outright:

* **`StatusFact`** — `RingMembership`, `ZeroUpCounter`, `SuccessInteger`,
  `NullFieldAfterRelease`. Cycle 19 measured every one of the first two directly:
  "Down-ring membership and a zero up counter therefore cannot discharge
  descriptor custody." O04 measured the third
  (`a_success_looking_integer_does_not_discharge_the_identity`). Refusal:
  `StatusFactIsNotCustodyEvidence`. Lean: `no_status_fact_discharges_custody`,
  `a_composed_program_offers_no_status_fact`.
* **`DurabilityDischarged`** — see §5.

### 3.3 Acceptance item 2 — callback quiescence and task/guard prerequisites before teardown

At the teardown point:

* every callback whose **declared** authority intersects any member payload is
  `Quiescent` — not `Registered`, not `Running`
  (`CallbackNotQuiescent`; Lean `a_live_callback_over_a_member_payload_is_unknown`,
  `a_composed_program_quiesces_every_member_callback`);
* every task the callback inventory says a callback **started** is declared and
  `Joined` (`CallbackSpawnedTaskUnaccounted`; Lean
  `an_unaccounted_spawned_task_is_unknown`). A quiescent callback that left a
  task running is still a refusal — deregistration is not quiescence of what it
  spawned;
* every task whose captures intersect the members is `Joined`, not `Outstanding`
  and not `Completed` (`TaskNotJoined`; Lean
  `an_unjoined_task_capturing_a_member_is_unknown`,
  `a_composed_program_joins_every_capturing_task`). `Completed` is refused on
  purpose: V02 pinned that a completed-but-unjoined child keeps its captures in
  `Custodian::Capsule`, and CT14/CT37 execute exactly that on real Linux threads;
* every guard over a footprint touching a member is `Released`
  (`GuardOutstanding`; Lean `an_outstanding_guard_over_a_member_is_unknown`).

Two positive rows keep the rule from being a blunt instrument and are tested: a
callback whose declared authority touches no member payload may be `Running`,
and a guard over a disjoint footprint may be `Held`. Both rest on the declared
completeness premises in §4.

### 3.4 The wrapper lifecycle

| Rule | Refusal |
|---|---|
| the two containment trees never cross: a wrapper allocation is not a child of the aggregate and vice versa, and the held member is aggregate-owned | `WrapperOverlapsAggregate` |
| the wrapper releases only what it owns, or the held member when the hold is owning | `WrapperReleasesForeignMember` |
| an **owned** hold is released by the wrapper and *not* by the parent | `OwnedHoldAlsoReleasedByParent` |
| a **borrowed** hold is not released by the wrapper … | `BorrowedHoldReleasedByWrapper` |
| … and its borrow is dropped **strictly before** the parent releases the held member | `BorrowedHoldOutlivesParentRelease` |
| every publication is withdrawn, and withdrawn strictly before the teardown point | `WrapperPublicationUnreclaimed`, `WrapperPublicationSurvivesTeardown` |

The owned/parent pair is exactly O00's blocker **B02**: `cio_chunk_open` links
the chunk into `st->chunks` *and* returns it as an owned result, so the caller's
`ic->chunk = chunk` and the stream's teardown both claim it. The relation refuses
that shape by name rather than choosing a winner. The publication rule is O00's
**B09**/**B10**: a msgpack callback and a hash-table entry that still point at
the wrapper when the parent starts releasing.

### 3.5 Acceptance item 3 — unsupported classes prevent whole-composition claims

Six classes; any one present refuses the whole claim before anything else is
believed.

| Class | What it covers | Why it is here |
|---|---|---|
| `Capacity` | resource-limit arms, partial writes, quota | O00 §2.1 E6a — the resource-limit arm that yields no descriptor and no mapping |
| `Metadata` | checksums, permissions, ownership, trim | Cycle 19's profile fixes "checksum and trim disabled"; a metadata variant is outside every observation |
| `Schedule` | dispatch order, priorities, ordering assumptions | `EventOperation::AssumeOrdering` / `InvalidOrderingEvidence`; P05 records that ordering assumptions have no wire record and stay Unknown |
| `Callback` | a callback outside the admitted family | P05's `UnsupportedBackend` discipline, at the callback level |
| `Backend` | a backend outside the accepted profile list | `cycle19-unsupported-close` — the symbolic partial-release classification with unresolved backend custody |
| `OsCustody` | OS custody the supplied post-state relation does not decide | the suppressed-syscall discipline; see §5.2 |

Lean: `any_unsupported_class_blocks_the_composition` is quantified over the
class, `an_unadmitted_callback_blocks_the_composition` derives the callback
class, and `a_composed_program_has_no_unsupported_class` is the soundness
direction. A test pins that an otherwise perfect program with one unsupported
class refuses with **exactly** that one refusal.

---

## 4. Premises: named, not derived

Nine declared premises are carried on the certificate and are **assumed**. None
is discharged anywhere in this card.

| Premise | What it assumes | Where it is already a premise |
|---|---|---|
| `ConstructorContract` | the constructor hands the caller the custody its declared contract says | O04 §3 — the constructor's own body does **not** discharge it (E5/E6 stay Unknown) |
| `DestructorContract` | the destructor releases what its contract declares | O00 R04; O04 measures that weakening it to `Borrowed` turns every cleanup arm into a residual |
| `BackendProfile` | the selected backend profile is the one that was built | `cycle19-unsupported-constructors`; the reactor's accepted-profile list |
| `CallbackAuthorityDeclared` | each callback's declared payload authority is **complete** | Cycle 18 `CallbackSummary::payload_authority`; P05 records the declared program and its callback authority as explicit premises |
| `GuardFootprintDeclared` | each guard's declared footprint is complete | T02's `DeclaredFootprintShape` |
| `TaskLedgerSupplied` | the scoped-task ledger reports every task and its terminal state | `qcore-core::scoped_tasks`; V02's terminal-state matrix |
| `ExitResidualsSupplied` | per-exit residuals came from a validated projection | O04's `LegacyExitResidual`; itself resting on C02's declaration-completeness and C-effective-type premises |
| `EndpointPostStateSupplied` | endpoint post-states came from a target-qualified relation | the Cycle 19 native observations, at their recorded target and profile |
| `ObservationsTrusted` | recorded observations are trusted for the runs they record | the standing `observations_trusted` premise with `observed_runs_only` bounds |

Two further premises are inherited and are **not** proved: that the reader
extracts a disposition faithfully, and C effective types (C02 §2, carried
through O04 §3).

**The relation decides nothing about these.** They are fields on the certificate
so that a reader can see them; a composed claim is exactly the conjunction of
§3's checks *under* them.

---

## 5. Synchronization and durability, stated separately

### 5.1 Carried, never discharged

`DurabilityPremise` is a **separate field** from `DeclaredPremise` on the
certificate: `SyncSucceeded`, `DataReachedStableStorage`,
`MetadataReachedStableStorage`, `MappingFlushed`, `RenameIsAtomic`.

Offering one as release evidence is a refusal —
`DurabilityAssumedAsDischarged` — for all five, tested in Rust and proved in
Lean (`no_durability_premise_discharges_custody`, and the soundness direction
`a_composed_program_discharges_no_durability_premise`). Custody and durability
are different questions and this relation answers only the first.

That separation is not decorative. Cycle 19 measured three ways in which the two
come apart on real Linux:

* "Down still unmaps/closes after failed sync, and unmapped commit clears the
  transaction without syncing." A completed teardown is not a durable one.
* "prior successful sync survives failure, while combined sync/unmap failure
  stays unsynced." A successful sync earlier in the sequence says nothing about
  the state after a later failure.
* "POSIX resize return values and errno remain separate."

A composed claim therefore never means "the data is on disk". It means the
custody ledger closes, under the durability premises the certificate names.

### 5.2 Suppressed syscalls are never promoted to API guarantees

Cycle 19's native fixtures **suppress** selected `close`/`open`/`munmap`/`fsync`
calls with `EIO`: they do not execute the real syscall. Their own record says
so: "A suppressed close has a known retained endpoint; these fixtures do not
infer the post-state of an arbitrary kernel close error."

The relation keeps that line in two places:

* `UnresolvedReason::SuppressedSyscall` exists so a disposition that rests on a
  suppressed call is *refused with that reason*, not silently accepted; and
* `UnsupportedClass::OsCustody` refuses the whole composition when the supplied
  post-state relation does not decide the endpoint at all.

A future target-qualified OS post-state relation (the O03 direction) would
supply `CustodyEvidence::ObservedEndpointPostState` under its own target
qualification. **That relation is absent from this branch** (§8, O05-O2): the
evidence kind is defined and consumed here, and the premise
`EndpointPostStateSupplied` names exactly who has to supply it.

---

## 6. The counterexample suite

`crates/qcore-core/tests/unit/aggregate_composition.rs`, 52 rows over a reference
program shaped after §2: root, child allocation, child name, backend, descriptor
endpoint (retained by a named external custodian), mapping endpoint, wrapper
object, wrapper table, one publication, one quiescent callback with one joined
spawned task, one joined task, one released guard.

Seven admitting rows, so no refusal is vacuous:

| Row | What it admits |
|---|---|
| the reference program | the whole shape, including a **retained** descriptor with a named custodian |
| an owned hold the wrapper alone releases | the `Owned` branch |
| a callback with no authority over a member payload | it may be `Running` |
| a task capturing nothing in the aggregate | it may be `Outstanding` |
| a raw descriptor number reused after its epoch is released | CT50's positive direction |
| a held guard over a disjoint footprint | it may be `Held` |
| the certificate's premise fields | durability premises land in their own field, all eight declared premises in theirs |

Forty-five counterexample, structural and relation-wide rows. Each negative row
asserts three things: the verdict is `Unknown`, the exact typed refusal is
present, and **no certificate is published**. Because the reference program
composes, every refusal is attributable to its own single mutation.

| Group | Rows | Refusals exercised |
|---|---:|---|
| identity separation | 8 | duplicate identity, never/twice acquired, not/twice released, release before acquire, and both directions of address reuse (retained epoch, and a second epoch acquired before the first is released) |
| dispositions and evidence | 7 | all five unresolved reasons, retained without a custodian, undeclared custodian, retained heap allocation, all four status facts, all five durability premises |
| ledger completeness | 2 | the parent never released, the empty aggregate |
| quiescence | 7 | registered, running, undeclared spawned task, unjoined spawned task, outstanding task, completed-but-unjoined task, held guard |
| unsupported classes | 3 | all six classes, an unadmitted callback, and the "exactly one refusal" row |
| the wrapper | 9 | both overlap shapes, owned/parent double custody, borrowed release by the wrapper, borrow never dropped, borrow dropped late, foreign release, unreclaimed publication, publication surviving teardown |
| structural | 4 | malformed-only reporting, a wrapper step with no wrapper, a containment cycle, a reclaim before its publish |
| relation-wide | 5 | refusals sorted and deduplicated, independent defects all reported, never publishes a violation, determinism under member reordering, typed reason rendering |

Several rows are loops over a whole vocabulary — five unresolved reasons, four
status facts, five durability premises, six unsupported classes — so the 52
functions decide considerably more than 52 cases.

---

## 7. What composes, and what does not, today

Applying the relation to the §2 subject with the evidence that actually exists
on this branch, the answer is **Unknown**, for named reasons — which is the
correct answer and the point of the exercise:

| Member | Best available disposition | Refusal it currently produces |
|---|---|---|
| ID-P chunk object | O04 discharges it on three caller arms; the constructor's E5/E6 stay Unknown | `UnresolvedDisposition(UnmodeledAggregateStore)` on the success exit (O00 B02) |
| ID-N chunk name | released inside the destructor, under `DestructorContract` | admitted, on the premise |
| ID-B / ID-Q backend child and path | O00 B04: `ch->backend` is published before the child's failure arms | `UnresolvedDisposition(UncontractedCallee)` |
| ID-D descriptor | Cycle 19 suppressed-close fixtures leave a residual | `UnsupportedClass::OsCustody` — no target-qualified post-state relation exists here |
| ID-M mapping | same | same |
| ID-IC / ID-RM wrapper | O00 B08/B09/B10 | `WrapperPublicationSurvivesTeardown` for the msgpack and hash-table publications |
| callbacks | the msgpack callback retains the wrapper; the reactor route binds no C source (P05) | `UnsupportedClass::Callback` |

So the composition of Cycle 18 and Cycle 19 **does not close**, and this document
says exactly which member and which class each blocker attaches to. That is a
specification result, not a verification result. Nothing here promotes any raw
Cycle 19 unit, maps any CT case, or claims anything about upstream Fluent Bit,
ChunkIO, Monkey or Libevent.

---

## 8. Boundaries and follow-on units

Recorded open, not repaired:

* **O05-O1 — no production caller.** The relation is not wired into
  `qcore-audit`, the Cycle 19 compatibility adapter, or any provider route. It
  is a dependency-free `qcore-core` model with its own tests, exactly as the
  card scopes it. Wiring it needs a producer that can build a `members` /
  `schedule` pair from a real projection, which is O05-O3 below.
* **O05-O2 — no OS endpoint post-state relation on this branch.** The card's
  brief named O03's target-qualified `close(2)` post-state relation as a
  dependency; it is **absent from this branch** (no `crates/qcore-core/src/os_endpoint.rs`,
  no `proof/QCore/OsEndpointPostState.lean`, no `docs/reports/remaining-o03/`).
  `CustodyEvidence::ObservedEndpointPostState` and the premise
  `EndpointPostStateSupplied` are the seam it plugs into; until it lands, an
  endpoint whose post-state nobody decides is `UnsupportedClass::OsCustody`.
  Reconciling the two vocabularies is a named follow-on.
* **O05-O3 — no producer from source.** Nothing derives an
  `AggregateCompositionProgram` from C. O04's `LegacyExitResidual` is the
  natural source for `ExitResidualDischarged` dispositions, but the mapping is
  not written, and O00's blockers B02/B03/B04/B08/B09/B10 all sit on that path.
* **O05-O4 — one teardown point.** Quiescence is decided against the *first*
  parent release. A teardown interleaved with callback registration, or a
  re-registration after the teardown point, is not expressible. The states are
  declared "as at the teardown point"; a per-step callback state machine
  (which the Cycle 18 model already has, at its own level) is a separate unit.
* **O05-O5 — one wrapper, one hold.** Exactly one wrapper, holding exactly one
  aggregate member. Two wrappers over one aggregate, a wrapper over two members,
  and nested wrappers are all outside the shape. So is a wrapper that outlives
  the aggregate deliberately.
* **O05-O6 — flow-insensitive containment.** Containment is a static `parent`
  link. A member that moves between containers during the schedule (a chunk
  moving between the up and down rings, which Cycle 19 measures) is not
  expressible; the relation sees only the final link. This is sound here because
  ring membership is a `StatusFact` and never discharges anything, but it does
  mean the relation cannot yet *check* a re-parenting.
* **O05-O7 — no Rust/Lean refinement.** The Lean model reproduces the decisions
  by construction and review, not by extraction or by a digest-checked
  transcription in F01's style. Two abstractions are recorded in the module
  header: the structural refusals are an input field rather than a computed
  function (every semantic theorem is stated with that list empty, matching the
  Rust early return), and certificate premise sets are not modeled because the
  relation decides nothing about them. A digest-checked correspondence in F01's
  style is a separate unit.
* **O05-O8 — no schema record.** Nothing crosses a process boundary. Carrying an
  aggregate composition in `qcore-schemas` would be a major version bump
  (currently 5.0.0) and is deliberately not attempted.
* **Out of scope by the card, and untouched:** universal failure recovery, a
  whole-Monkey theorem, full native refinement, capacity/metadata/schedule
  models, and any promotion of a raw Cycle 19 unit.

---

## 9. Reading order

| Artefact | What it holds |
|---|---|
| `crates/qcore-core/src/aggregate_composition.rs` | the relation, its typed refusals and its certificate |
| `crates/qcore-core/tests/unit/aggregate_composition.rs` | the reference program and the 52-row counterexample suite |
| `proof/QCore/AggregateComposition.lean` | the model of the decisions, 39 audited theorems |
| `proof/AuditO05.lean` | the axiom surface |
| `docs/reports/remaining-o05/README.md` | the evidence record, gate results and integrator notes |
