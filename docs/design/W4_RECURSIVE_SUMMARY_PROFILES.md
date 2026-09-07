# W4 recursive summary profiles — neutral-frame recursion

Design document for card **C03** (workstream W4, lane Custody, kind
`bounded_design`). Written against implementation checkpoint `a4492da`.

**This document specifies; it implements nothing.** No production analysis,
checker, contract, profile or schema file changes with it. No verdict moves, no
raw Cycle 19 unit is promoted, no pinned artifact is touched, and no CT case is
mapped. Everything it claims about current behaviour is measured by
`crates/qcore-c-gateway/tests/c03_neutral_frame_recursion_probe.rs`, a
design probe over the **unchanged** gateway; everything it claims about the
proposed route is a specification with an acceptance obligation attached, not a
result. The archived record is `docs/reports/remaining-c03/README.md`.

## 1. Where this sits

W4 publishes a custody summary for an internal C function only after a fresh
run of the intraprocedural analyzer re-derives it from the body
(`crates/qcore-c-gateway/src/legacy_summary.rs`, conditions 1–6 of its header).
Two recursive routes exist on top of that discipline:

| Route | Restricts | Admits | Justification |
|---|---|---|---|
| Contextual recursion (`docs/reports/w4-contextual-recursion/README.md`) | one immutable explicit **integer** control, partitioned Zero/NonZero | arbitrary custody effects | ranks 0 and 1; a NonZero context may only call an already-validated literal-zero context |
| Unsigned countdown (`docs/reports/w4-unsigned-countdown/README.md`) | one immutable explicit **unsigned** control, actual exactly `0` or `control - 1` | arbitrary custody effects | strong induction over concrete natural counter values |

Both constrain the **control shape** and leave the **effect content** free,
paying for that freedom with a well-founded rank. The obvious next increments
along that axis (`n / 2`, `n - 2`, several controls) are more of the same
domain, not a new one.

C03 therefore selects a family on the orthogonal axis: **constrain the effect
content to the bottom element and leave the control shape free.** That family
is *neutral-frame recursion* (NFR).

## 2. Corpus motivation

A read-only census of the C sources in the Cycle 19 audit scope
(`build/cycle19/fluent-bit-clang-source`, 309 `.c` files, 8,239 extracted
function definitions) found **45 self-recursive functions** and **27 file-local
recursion cycles** of size ≥ 2. Bucketed by hand-checked classification:

| Bucket | Count | Meaning |
|---|---|---|
| A — custody-neutral | 25 | no acquisition or release on any path |
| B — scalar rank | 2 | the changed recursive actual is an integer expression on a scalar formal |
| C — heap-link chain | 2 | the changed actual is a pointer-field dereference **and** the body releases |
| D — other / unclear | 16 | the changed actual is a local previously assigned from a pointer field, a call result, or unchanged |

Independently of the bucket, the *argument shape* cross-tab is: 18 of 45 recurse
on a pointer-field dereference (16 of them in bucket A), 3 of 45 on a genuine
scalar rank, the remaining 24 unclear.

Two readings matter for family selection:

- **The two implemented routes reach at most 3 of 45 functions in this corpus.**
  Scalar-rank recursion is rare in this code.
- **The largest single family is neutral (25 of 45)**, and the most replicated
  *shape* across the two libraries the configured audits actually target
  (CMetrics, CTraces) is the variant/kvlist/array tree walk — six file-local
  cycles, of which four are neutral encoders/cloners and two are releasing
  destroyers.

Census limitations, all material: no preprocessing (`#if`-disabled and
macro-generated code is counted or missed), lexical call detection (a release
reached through a function pointer is invisible — one such case,
`cJSON_Delete`, was found and reclassified by hand), a name-based
resource-token list, and a per-file call graph, so 27 cycles is a **lower
bound** and every count above is an estimate, not an inventory.

## 3. Families considered

| Family | Corpus reach here | New schema vocabulary | New declared premise | Decidable under validation-by-reanalysis |
|---|---|---|---|---|
| F1 general arithmetic rank (`n/2`, `n-2`, several controls) | ≤ 3 | none | none | yes — but it is an increment of an existing domain, not a new one |
| F2 heap-link chain teardown | 2 directly (18 by shape) | **required** (§8) | acyclicity | **no** — see §8 |
| **F3 neutral-frame recursion (NFR)** | **25 by classification, 9 reachable today** (§6.4) | none | none | yes — §4, §5 |
| F4 out-parameter accumulator recursion | not separately counted | none | none | needs F1's rank *and* all-exit out-slot validation per context; strictly larger than C03's scope |

**F3 is selected.** It is the only candidate that is both the largest measured
family and expressible in the existing contract vocabulary with no new premise.

## 4. The domain

### 4.1 What a neutral contract asserts

For a defined function `m` with formals `x1..xk`, the *neutral contract*
`N(m)` is the existing `ResourceFunctionContract` with every parameter
`Borrowed` and result `None`. It is not new vocabulary: it is exactly what
ordinary W4 derivation already publishes for a non-recursive function with no
custody effects. NFR is the closure of that ordinary outcome under recursion.

`N(m)` is read as two statements, and the distinction between them is the whole
argument of this document:

- **(N-exit)** *if* the call returns, every formal is left in the disposition
  the caller had, no containment edge into a caller-held aggregate was added,
  no formal was written through, and the returned value carries no custody the
  caller may release.
- **(N-prefix)** at **every** point of the execution of the call — returning or
  not — the frame and its callees perform no custody event on any resource
  identity the caller holds.

(N-prefix) is prefix-closed: it is a property of every finite prefix of the
execution, so a divergent call satisfies it exactly as a returning one does.
No contract expressible above the neutral element has that shape.

### 4.2 Lattice, order, convergence

The **intraprocedural** abstract domain, its joins and its fixed point are
unchanged; NFR adds only an *interprocedural assumption lattice* over one
strongly connected component `C` of the call graph:

- Per member: `A(m) ∈ {Uncontracted ⊏ Neutral}`, where `Uncontracted` is the
  conservative element (a call to `m` is an unmodelled call, hence `Unknown`)
  and `Neutral` is the optimistic one (the call site applies `N(m)`).
- Component states are maps `A : C → {Uncontracted, Neutral}`, ordered
  pointwise. Height `|C| + 1`; the lattice is finite.
- **Convergence policy.** Start at the top element `A0(m) = Neutral` for every
  member, validate every member's body against `N(m)` under `A`, and set to
  `Uncontracted` every member that fails. Repeat until stable. Each round
  demotes at least one member, so the iteration terminates in at most `|C|`
  rounds and computes the greatest post-fixed point below all-Neutral.
- **No join and no widening are used or needed**: the iteration is a monotone
  descent over meets in a finite lattice of height `|C| + 1`. (Widening would
  become relevant only for a domain with an unbounded assumption space — for
  example F2's chain regions.)
- **Cascade lemma.** Demoting `m` makes every reachable call to `m` an
  unmodelled call, so every member whose validated body contains a reachable
  call to `m` is demoted in the next round; by strong connectivity the descent
  usually collapses to all-or-nothing. A member survives only if all its call
  edges into demoted members are graph-unreachable after guard selection.
  **Publication is therefore specified at component granularity**: either every
  member of `C` publishes `N(m)`, or none does. The finer greatest-fixed-point
  descent is recorded as optional unit U2, to be justified by measurement, not
  assumed.

### 4.3 Why the optimistic start is admissible **only** at the neutral element

The standing instruction in `CLAUDE.md` — *do not accept an assume-guarantee
fixed point for a recursive component* — is preserved by this design, and the
probe measures precisely why it must be.

**Measured counterexample (probe:
`an_assumed_effectful_contract_validates_vacuously_under_divergence`).** With
`resource_released(p, nonnull)` in force for

```c
void purge(void *p, int n) { if (n > 0) { purge(p, n - 1); return; } purge(p, 1); }
```

the body validates against that contract, a `Released` summary is published,
and the caller's leak disappears — the analysis returns `Safe`. Every concrete
execution of `purge` diverges, so "if it returns, `p` was released" is
vacuously true and the exit comparison has nothing to object to. The same
happens for an owned *result*: `void *mint(int n) { void *r = mint(n); return r; }`
publishes `Owned{nonnull}` minted from nothing. A candidate with a concretely
reachable base exit that does not perform the effect *is* refused, so this is a
divergence artefact rather than a hole in the exit comparison.

The conclusion is sharper than "assume-guarantee is unsound": **validation by
re-analysis cannot by itself refuse a circular effectful candidate.** In
production that refusal comes from the *derivation strategy* — ordinary
inference never seeds a component member's own contract, and the two recursive
routes seed only at strictly smaller ranks. Any future route that assumes an
effect must supply a rank. NFR supplies none, so NFR must assume nothing above
the neutral element.

**Why the neutral element is different.** Two properties, neither of which any
higher element has:

1. `N(m)` states **no precondition**. Every formal is `Borrowed`, so the
   assumption installs no entry resource and asserts no non-nullness; it
   therefore cannot delete a failure path, which is the specific hazard
   `CLAUDE.md` names. Measured: a caller passing a null constant to a neutral
   recursive callee stays `Safe`, and the callee's own body validated with no
   formal bound.
2. `N(m)` is a conjunction of **negative, prefix-closed** facts (N-prefix).
   Soundness is then an induction on the length of the concrete execution
   prefix rather than on a termination rank: a violating custody event at step
   `k` is executed by *some* frame's straight-line instruction, and every
   frame's body was validated with no formal bound; the assumption is used only
   to conclude that an inner call that *already completed* within the first
   `k−1` steps changed nothing, which is a strictly shorter sub-execution.
   Divergence cannot manufacture a fact, because the fact asserted about a
   non-returning call is the same as the fact asserted about a returning one.

**Premise NFR-P1.** That induction needs one premise: in the abstract
semantics, a custody event applied to a value whose identity the analysis has
not resolved raises `UnknownAlias` or `Unsupported` rather than being ignored.
This is not new trust — it is the pre-existing fail-closed premise the whole
gateway rests on. The probe exercises its three relevant paths: releasing or
consuming a borrowed formal, storing a formal into a global, and any indirect
access all refuse the neutral candidate.

### 4.4 Alias bounds

The profile is chosen so that its alias obligation is **empty**, and this is the
reason to prefer it over F2:

- No formal is bound to a resource identity, so there is no entry site to alias.
- No `Stored` or `Removed` mode is claimed, so no containment edge is
  sanctioned; the sanctioned-containment check (condition 4) requires the empty
  set.
- No `Out` mode is claimed, so no slot may be written through.
- The result is `None`, so the value handed back is **untracked**. A neutral
  member may legitimately return one of its own formals; the caller-side
  consequence is fail-closed, not silent: a caller that releases the returned
  value reports `UnknownAlias` and the analysis is `Unknown`, while a caller
  that ignores the handback stays `Safe`. Both directions are measured.
- Globals and indirect accesses are refused by the existing gates, not by a new
  rule.

Consequently NFR requires **no alias analysis, no heap-link discovery and no
region abstraction**. Every one of those is what F2 would require.

### 4.5 Divergence versus checked exits

| Concrete outcome of a call to `m` | What `N(m)` claims | Sound because |
|---|---|---|
| returns normally | (N-exit) and (N-prefix) | body validated at every reachable exit under the empty entry state |
| returns on an early / error path | identical claim — NFR does not distinguish exits, and claims nothing about which exit was taken | all exits are validated alike; no exit-selective mode is published |
| diverges (never returns) | (N-prefix) only; (N-exit) is vacuous **and asserts nothing positive** | the assertion is a conjunction of negative facts; no release, no acquisition and no transfer is ever claimed to have happened |

Explicitly: **a neutral summary is not a termination claim, not a liveness
claim, and not a statement that resources held across the call are eventually
released.** The gateway's resource projection does not establish termination
for C code, and NFR does not change that. A resource a *caller* holds across a
divergent neutral call is not leaked in this projection's sense — nothing
released it, and no exit was reached — and the design deliberately declines to
say more.

## 5. Validation contract

An implementation of NFR must satisfy all of the following. V1–V4 gate the
route, V5–V9 are per member, V10 governs publication and rechecking.

| # | Obligation |
|---|---|
| V1 | The component contains **no declared contract** for any member. A declared contract keeps the component on the existing route; declared precedence is never touched. |
| V2 | Ordinary inference and both existing recursive routes run **first**. NFR is consulted only for a component that published nothing, and may never replace or weaken a contract another route published. |
| V3 | The assumption installed for the component is exactly `N(m)` for every member — every formal `Borrowed`, result `None`. No other candidate is ever seeded for a member of its own component. |
| V4 | The component's assumption map is iterated as in §4.2 and publication is all-or-nothing over the component (U1) unless U2 lands with its cascade evidence. |
| V5 | Each member's body is re-analyzed with `EntryCustody` **empty** and must report `Safe` with **zero** diagnostics — an unresolved alias, an unsupported construct, an uncontracted callee, a leak or a summary conflict each refuse the component. |
| V6 | `observations_match_contract` is applied unchanged with `N(m)`: every reachable exit leaves every formal untracked, the sanctioned containment set is empty, no out slot is claimed or written, and no exit returns a tracked resource. |
| V7 | Ordinary derivation must not read a *different* contract off the same body. A body that ordinary derivation reads as `Owned`, `Released`, `Consumed`, `Stored` or with an owned result contradicts the candidate and refuses it. (Measured: the global-store and acquiring bodies both surface as `SummaryConflict` under the emulation.) |
| V8 | At least one exit must be reachable **in the abstract semantics**, as condition 2 already requires. This is explicitly *not* a claim that an exit is concretely reachable, and no consumer may read it as one. |
| V9 | The published record is a `ResourceSummaryRecord` with `ContractTrust::InferredValidated` and contract `N(m)` — no new schema field, no new trust label, no wire change. |
| V10 | `check_legacy_safety_certificate` must **re-run the whole component computation** from the certificate's own preprocessed source and declared contracts — component detection, assumption seeding, per-member validation and the descent — and demand exact equality of the published vector. A certificate whose recorded summaries do not reproduce is refused, exactly as today. |

Two obligations are deliberately *not* on this list, and their absence is the
design's substance: there is no rank to extract, and there is no alias or shape
premise to discharge.

## 6. Prototype

### 6.1 Method and its limit

The route is not implemented. It is **emulated** by placing the candidate
contract in the *declared* environment: a declared contract is what the
recursive call site sees, and the defining body is then checked against that
same contract by the ordinary analyzer — which is precisely the assume-then-
validate step V5/V6 specify. Every run below is over unchanged production code;
declared contracts are trusted assumptions, so nothing here is a defect report.

The emulation is **more permissive than the route at exactly one point**: a
declaration survives its own member's refusal. Probe
`declared_emulation_does_not_reproduce_component_withdrawal` pins that: in a
mutual pair whose second member releases a formal, the first member stays
`Safe` under the emulation, whereas the route would demote the whole component
(§4.2). This is why V4 fixes publication at component granularity, and it is
the one acceptance obligation U1 cannot inherit from the probe.

### 6.2 Measured outcomes

All eight probe tests pass. `entry` is the verdict for the calling function
`subject`; `member` is the recursive callee's own projection.

| # | Fixture | Assumption | member | entry | Reading |
|---|---|---|---|---|---|
| 1 | neutral self-recursion | none | `Unknown` (`call to uncontracted function`) | `Unknown` | today's gap: the component **and** its callers are lost |
| 2 | neutral self-recursion, decreasing scalar | `N` | `Safe`, publishes `[Borrowed, Borrowed] → None` | `Safe` | feasibility |
| 3 | neutral self-recursion, **no** decreasing argument | `N` | `Safe`, same contract | `Safe` | the control shape is genuinely unconstrained |
| 4 | neutral **mutual** pair | `N` for both | both `Safe`, both publish | `Safe` | mutual components need no extra machinery |
| 5 | neutral body that acquires and releases a **local** | `N` | `Safe` | `Safe` | local custody is not an effect on the caller |
| 6 | body releases a formal | `N` | `Unknown` (`UnknownAlias`) | — | discrimination |
| 7 | body consumes a formal | `N` | `Unknown` (`UnknownAlias`) | — | discrimination |
| 8 | body leaks a local acquisition | `N` | `Unsafe` (`Leak`) | — | discrimination |
| 9 | body stores a formal in a global | `N` | `Unknown` + `SummaryConflict` (`Owned` vs `Borrowed`) | — | V7 |
| 10 | body acquires and returns | `N` | `Unknown` + `SummaryConflict` (`Owned{nullable}` result vs `None`) | — | V7 |
| 11 | caller passes a null constant | `N` | — | `Safe` | the assumption states no precondition |
| 12 | member returns a formal; caller releases the handback | `N` | `Safe` | `Unknown` (`UnknownAlias`) | the untracked result is fail-closed |
| 13 | same; caller ignores the handback | `N` | `Safe` | `Safe` | and is not gratuitously lost |
| 14 | member calls an uncontracted external | `N` | `Unknown` (`call to uncontracted function`) | — | fail-closed inheritance |
| 15 | divergent body, **`Released`** assumed | effectful | `Safe`, publishes `Released` | `Safe` | §4.3 counterexample |
| 16 | divergent body, **`Owned` result** assumed | effectful | — | `Safe` | §4.3 counterexample |
| 17 | real base exit, `Released` assumed | effectful | `Unknown` (`UnknownAlias`) | — | the exit comparison is not the weak point |
| 18 | recursion on `n->next` | `N` | `Unknown` (`indirect memory access is outside the resource projection`) | — | §6.4 |

### 6.3 Verdict on acceptance item 3

The domain **handles the selected family**: rows 2–5 are exactly the shapes the
census's bucket A contains (self, unranked, mutual, local-custody), and rows
6–10 and 14 show that the same assumption certifies no effect it should not.
Row 1 measures what is currently lost. The inconclusive direction stays
`Unknown` everywhere.

### 6.4 What caps the payoff today

Row 18 is the operative limit. Recursion whose changed actual is a pointer-field
read is refused by the existing indirect-access gate **before any recursion rule
is consulted** — that is C02-P (borrowed pointer-field reads), explicitly out of
scope for C02. Combining that with the census cross-tab: of the 25 neutral
recursive functions, **16 recurse through a pointer field** and would stay
`Unknown` under NFR alone; the immediately reachable set is the remaining
**≈ 9** (1 scalar-shaped, 8 unclear-shaped, each needing case-by-case
confirmation). NFR's full measured value therefore depends on C02-P landing
first, and the follow-up ordering in §9 says so.

## 7. Non-goals

NFR does not: certify any custody effect; establish termination or bounded
stack depth; unblock pointer-field recursion; describe cost or liveness; give a
caller any right over the value a neutral member returns; or promote any raw
unit. A component that fails any obligation in §5 stays `Unknown`, and so do
its callers.

## 8. Rejection boundary: heap-link chain teardown (F2)

F2 — `void purge(struct node *n) { if (!n) return; purge(n->next); free(n); }`
and the two corpus destroy-cycles it stands for — is **rejected for C03**, and
the rejection is architectural rather than a matter of effort.

1. **The effect is not expressible.** `ResourceParameterMode` names *formals*:
   `Borrowed`, `Owned`, `Released`, `Consumed`, `Stored{owner}`, `Out`,
   `Removed{owner}`. The true effect of `purge` is "releases every node
   reachable from `n` through `next`" — an unbounded set of resource
   identities, only one of which is a formal. `resource_released(n)` is the
   closest expressible candidate and it is **not a sound abstraction**: at a
   call site it leaves every other tracked identity untouched, so a caller that
   holds an interior node keeps a live site for memory the callee freed.
2. **`ResourceReturnMode` has the same gap** (`None`, `Owned`, `Reallocated`):
   there is no way to say a returned pointer *aliases* an existing tracked
   site, which is what a chain walker's handback is.
3. **The hazard is real, not hypothetical.** `chain-teardown-hazard.c` beside
   the C03 report executes both failure modes under ASan/UBSan:
   `interior` reports `heap-use-after-free` on the interior node after
   `purge(head)`, and `cyclic` reports `stack-overflow` inside `purge` —
   on a cyclic input the teardown never reaches a release at all, so every
   exit-state contract for it is vacuous. That is the same vacuity as §4.3,
   reached from the data side, and it is why an acyclicity premise would be
   load-bearing rather than decorative.
4. **Today the gap is masked, which is the dangerous part.** The gateway
   currently refuses `purge` and its callers at the indirect-access gate
   (row 18), so no false `Safe` is reachable through this route now. The
   incompleteness becomes exploitable exactly when interior aliases become
   representable — that is, when C02-P or any heap-link work lands. Hence the
   ordering rule below.

**Rejection boundary.** No inference route may publish a custody contract for a
function that releases through a pointer-field chain until (a) a
reachable-region release mode exists in `qcore-schemas` **and** (b) the
caller-side domain can represent membership in that region **and** (c) the
acyclicity premise is declared and recorded as an assumption. Any two of the
three are not enough: a region mode without caller-side membership is
unenforceable, and caller-side membership without acyclicity is unsound on
cyclic input.

## 9. Follow-up implementation units

C03's output is this specification plus the units below. None of them is
started; the sizes are estimates.

| Unit | Size | Depends on | Content | Acceptance |
|---|---|---|---|---|
| **U1** NFR derivation route | S | — | component-level `N(m)` seeding, per-member validation V5–V9, all-or-nothing publication in `legacy_summary.rs` | probe rows 2–5 move to inference without declarations; rows 6–10, 14, 18 unchanged; **185-unit and CMetrics artifacts re-measured and reported**, with any newly surfaced provisional `Unsafe` unit recorded, never suppressed |
| **U2** greatest-fixed-point descent | S | U1 | demote only failing members and re-validate, instead of all-or-nothing | a fixture where a member survives its component's partial demotion, plus the cascade lemma's unreachable-edge case; skip if U1 measures no case |
| **U3** certificate rederivation | S | U1 | V10: recheck repeats component detection, seeding and descent; tamper tests over the component boundary and the published vector | a substituted member, a widened component and a forged neutral record each refuse |
| **U4** Lean theorems | M | U1 | prefix-closure of (N-prefix); admissibility of the optimistic start at ⊥ only; non-vacuity; refusal of every element above ⊥ without a rank | new `QCore` namespace, no new axiom, `check_lean_audits.py` green |
| **U5** C02-P borrowed pointer-field reads | M | C02 | admit *reading* a pointer field of a record the reader can type, without granting custody | independently valuable; **raises NFR's reach from ≈ 9 to ≈ 25 in this corpus** |
| **U6** reachable-region release mode | L | U5 | the three-part boundary of §8: `ReleasedReachable{parameter, link_field}` (a `qcore-schemas` major version under the documented policy), caller-side region membership, declared acyclicity premise | only route to F2; **must not be started before U5**, and not at all without all three parts |
| **U7** self-dependent record hygiene | S | — | §10 observation: an `InferredValidated` record derived *under the declaration of the same function* is not independent corroboration of it; label it, or omit it | a certificate consumer cannot read a declaration back as an inference |

Recommended order: U1 → U3 → U4 (the self-contained NFR increment), then U5,
then re-measure NFR's reach, and only then reconsider U6.

## 10. Premises and open items

- **NFR-P1** (§4.3) — the analyzer's fail-closed treatment of unresolved
  identities is complete for custody events. Pre-existing gateway premise; NFR
  adds no trust but leans on it harder, because it is the *only* thing standing
  between a neutral candidate and an unnoticed effect.
- **Source extraction premises** carry over unchanged: Clang import, the
  complete call inventory and CFG construction are trusted implementation
  premises, as both existing recursive routes record.
- **Observation, recorded not repaired.** With a declared contract in force for
  a function, W4 can publish an `InferredValidated` record for that same
  function that was derived *using* the declaration — measured at probe rows 15
  and 16, where a divergent body's declared `Released` reappears as a validated
  summary. Declared contracts govern call sites anyway, so no additional
  authority is created and this is not a soundness defect; the risk is
  presentational, in a consumer reading such a record as independent
  corroboration. Unit U7.
- **Not established here**: termination or stack bounds for any C function;
  leak-freedom across a divergent call; any statement about F2 beyond §8; and
  any change to CT coverage (11/50 mapped), the contract corpus (59 modules) or
  the raw Cycle 19 totals (2 Safe / 42 provisional Unsafe / 137 Unknown plus
  four data-only units), all of which this document leaves exactly where it
  found them.
