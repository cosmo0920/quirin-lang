-- SPDX-License-Identifier: Apache-2.0
--
-- Licensed under the Apache License, Version 2.0 (the "License");
-- you may not use this file except in compliance with the License.
-- You may obtain a copy of the License at
--
--     http://www.apache.org/licenses/LICENSE-2.0
--
-- Unless required by applicable law or agreed to in writing, software
-- distributed under the License is distributed on an "AS IS" BASIS,
-- WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
-- See the License for the specific language governing permissions and
-- limitations under the License.

import QCore.CustodySummary

/-!
Neutral-frame recursion (NFR): the W4 recursive-summary family of card C03,
implemented in `crates/qcore-c-gateway/src/legacy_neutral.rs` and specified in
`docs/design/W4_RECURSIVE_SUMMARY_PROFILES.md`.

Three things are modelled here, matching the three things the route rests on.

1. **The lattice and its descent.** Per member the interprocedural assumption is
   a two-point lattice `uncontracted ⊏ neutral`; component states are pointwise
   maps over a finite index range. The route starts at the top element, demotes
   every member whose body fails validation, and repeats. Given only that
   validation is monotone in the assumption map — a member that validates with
   fewer contracts in force also validates with more — the descent terminates
   and lands on the **greatest post-fixed point** below all-neutral. No join and
   no widening appears, because none is used.

2. **Why the optimistic start is admissible at the bottom element and nowhere
   above it.** The neutral claim is a conjunction of negative facts about every
   prefix of an execution; an effectful claim is a statement about exits only.
   The two behave differently exactly under divergence, and the theorems below
   exhibit the gap: an exit claim is satisfied vacuously by a non-returning run
   that performs the opposite effect, while the neutral claim reads the same
   whether the run returns or not. That is why any family assuming an effect
   must supply a well-founded rank, as the contextual and unsigned-countdown
   routes do and as `ranked_calls_are_well_founded` records.

3. **What publication asserts**, by reduction to the unchanged
   `QCore.CustodySummary.validate` relation: analysis safety, a reachable exit
   in the abstract semantics, and per-exit agreement, with the neutral summary's
   four pointwise consequences (no bound formal, no out slot, no containment, no
   returned resource).

Premises, stated and not proved here, exactly as the two existing recursive
routes record them: Clang import, the complete call inventory and CFG
construction are trusted implementation premises of the gateway, and
monotonicity of validation is a property of the abstract interpreter rather than
something this model derives. Nothing here establishes termination of any C
function, and `a_divergent_run_satisfies_every_exit_claim` is precisely the
statement that no exit-state contract can.
-/

namespace QCore.NeutralFrameRecursion
open QCore.CustodySummary

/-! ## The assumption lattice -/

/-- A component assumption map: `true` is the optimistic element `neutral`,
    `false` the conservative `uncontracted`. Members are indexed by `Nat`; a
    component of size `n` uses the indices below `n` and nothing else. -/
abbrev Assign := Nat → Bool

/-- The pointwise order. `Le a b` reads "`a` assumes no more than `b`". -/
def Le (a b : Assign) : Prop := ∀ i, a i = true → b i = true

/-- Everything outside the component is `uncontracted`. -/
def Bounded (n : Nat) (a : Assign) : Prop := ∀ i, n ≤ i → a i = false

/-- The top element of a size-`n` component: every member optimistic. -/
def top (n : Nat) : Assign := fun i => decide (i < n)

/-- One descent round: a member stays optimistic exactly while its body still
    validates under the current map. -/
def step (valid : Assign → Nat → Bool) (a : Assign) : Assign := fun i => a i && valid a i

/-- A post-fixed point: every member the map still assumes is one whose body
    validates under that same map. This is what publication requires. -/
def Post (valid : Assign → Nat → Bool) (a : Assign) : Prop :=
  ∀ i, a i = true → valid a i = true

/-- Validation is monotone in the assumption map: making more contracts
    available never turns a validating body into a failing one. This is a
    property of the abstract interpreter, supplied as a premise. -/
def MonotoneValid (valid : Assign → Nat → Bool) : Prop :=
  ∀ a b, Le a b → ∀ i, valid a i = true → valid b i = true

/-- Explicit iteration, so the descent's reduction is definitional. -/
def iter {α : Type} (f : α → α) : Nat → α → α
  | 0, a => a
  | k + 1, a => iter f k (f a)

/-- How many members of a size-`n` component the map still assumes. -/
def liveCount (n : Nat) (a : Assign) : Nat := (List.range n).countP a

/-! ### Counting helpers -/

private theorem countP_le {α : Type} (l : List α) {p q : α → Bool}
    (mono : ∀ x, p x = true → q x = true) : l.countP p ≤ l.countP q := by
  induction l with
  | nil => simp
  | cons x xs ih =>
      rw [List.countP_cons, List.countP_cons]
      cases hp : p x with
      | false => cases hq : q x <;> simp <;> omega
      | true =>
          have hq := mono x hp
          simp [hq]
          omega

private theorem countP_lt_of_witness {α : Type} (l : List α) {p q : α → Bool}
    (mono : ∀ x, p x = true → q x = true) {x : α}
    (mem : x ∈ l) (hq : q x = true) (hp : p x = false) :
    l.countP p < l.countP q := by
  induction l with
  | nil => cases mem
  | cons y ys ih =>
      rw [List.countP_cons, List.countP_cons]
      by_cases hy : y = x
      · subst hy
        have base := countP_le ys mono
        simp [hp, hq]
        omega
      · have mem' : x ∈ ys := by
          rcases List.mem_cons.mp mem with h | h
          · exact absurd h.symm hy
          · exact h
        have lower := ih mem'
        cases hpy : p y with
        | false => cases hqy : q y <;> simp <;> omega
        | true =>
            have hqy := mono y hpy
            simp [hqy]
            omega

/-! ## Descent -/

/-- A round only ever demotes: the descent is monotone downward. -/
theorem step_descends (valid : Assign → Nat → Bool) (a : Assign) :
    Le (step valid a) a := by
  intro i h
  exact (Bool.and_eq_true _ _ |>.mp h).1

/-- A round never optimistically extends a component. -/
theorem step_preserves_bounds {n : Nat} {valid : Assign → Nat → Bool} {a : Assign}
    (bounded : Bounded n a) : Bounded n (step valid a) := by
  intro i hi
  simp [step, bounded i hi]

theorem top_is_bounded (n : Nat) : Bounded n (top n) := by
  intro i hi
  simp [top]
  omega

/-- A map is a post-fixed point exactly when a round leaves it alone. -/
theorem post_fixed_iff_stable {valid : Assign → Nat → Bool} {a : Assign} :
    Post valid a ↔ step valid a = a := by
  constructor
  · intro post
    funext i
    cases h : a i with
    | false => simp [step, h]
    | true => simp [step, h, post i h]
  · intro stable i hi
    have := congrFun stable i
    rw [step] at this
    simp [hi] at this
    exact this

/-- A round preserves the order, given monotone validation. -/
theorem step_is_monotone {valid : Assign → Nat → Bool} (mono : MonotoneValid valid)
    {a b : Assign} (order : Le a b) : Le (step valid a) (step valid b) := by
  intro i h
  obtain ⟨live, checked⟩ := Bool.and_eq_true _ _ |>.mp h
  simp [step, order i live, mono a b order i checked]

/-- Every member the map assumes lies inside the component. -/
private theorem live_is_in_range {n : Nat} {a : Assign} (bounded : Bounded n a) {i : Nat}
    (live : a i = true) : i < n := by
  cases Nat.lt_or_ge i n with
  | inl lower => exact lower
  | inr upper =>
      rw [bounded i upper] at live
      exact absurd live (by simp)

private theorem step_strictly_shrinks {n : Nat} {valid : Assign → Nat → Bool} {a : Assign}
    (bounded : Bounded n a) {i : Nat} (live : a i = true) (refused : valid a i = false) :
    liveCount n (step valid a) < liveCount n a := by
  refine countP_lt_of_witness (List.range n) ?_
    (List.mem_range.mpr (live_is_in_range bounded live)) live ?_
  · intro x hx
    exact (Bool.and_eq_true _ _ |>.mp hx).1
  · simp [step, refused]

/-- Once a round changes nothing, no later round does either. -/
private theorem iter_of_stable {valid : Assign → Nat → Bool} {a : Assign}
    (stable : step valid a = a) : ∀ k, iter (step valid) k a = a := by
  intro k
  induction k with
  | zero => rfl
  | succ k ih =>
      show iter (step valid) k (step valid a) = a
      rw [stable]
      exact ih

/-- **The descent terminates on a post-fixed point.** Each round that changes
    anything demotes at least one member, so `liveCount` rounds suffice — for
    the top element of a size-`n` component, at most `n`. -/
theorem descent_reaches_a_post_fixed_point {n : Nat} {valid : Assign → Nat → Bool} :
    ∀ (k : Nat) (a : Assign), Bounded n a → liveCount n a ≤ k →
      Post valid (iter (step valid) k a) := by
  intro k
  induction k with
  | zero =>
      intro a bounded budget i live
      exfalso
      have live : a i = true := live
      have positive :=
        countP_lt_of_witness (List.range n) (p := fun _ => false) (q := a)
          (by intro x hx; exact absurd hx (by simp))
          (List.mem_range.mpr (live_is_in_range bounded live)) live (by simp)
      have empty : List.countP (fun _ => false) (List.range n) = 0 := by simp
      have exhausted : List.countP a (List.range n) ≤ 0 := budget
      omega
  | succ k ih =>
      intro a bounded budget
      show Post valid (iter (step valid) k (step valid a))
      by_cases post : Post valid a
      · have stable : step valid a = a := post_fixed_iff_stable.mp post
        rw [stable, iter_of_stable stable k]
        exact post
      · have witness : ∃ i, a i = true ∧ valid a i = false := by
          cases Classical.em (∃ i, a i = true ∧ valid a i = false) with
          | inl found => exact found
          | inr missing =>
              refine absurd (fun i live => ?_) post
              cases hv : valid a i with
              | true => rfl
              | false => exact absurd ⟨i, live, hv⟩ missing
        obtain ⟨i, live, refused⟩ := witness
        have shrink := step_strictly_shrinks (n := n) bounded live refused
        exact ih (step valid a) (step_preserves_bounds bounded) (by omega)

theorem descent_stays_bounded {n : Nat} {valid : Assign → Nat → Bool} :
    ∀ (k : Nat) (a : Assign), Bounded n a → Bounded n (iter (step valid) k a) := by
  intro k
  induction k with
  | zero => intro a bounded; exact bounded
  | succ k ih => intro a bounded; exact ih (step valid a) (step_preserves_bounds bounded)

/-- **The descent lands on the greatest post-fixed point.** Any post-fixed point
    below the starting map is below every round of the descent, hence below its
    limit: the route publishes as much as it soundly can, and never more. -/
theorem descent_is_the_greatest_post_fixed_point {valid : Assign → Nat → Bool}
    (mono : MonotoneValid valid) {t : Assign} (post : Post valid t) :
    ∀ (k : Nat) (a : Assign), Le t a → Le t (iter (step valid) k a) := by
  intro k
  induction k with
  | zero => intro a order; exact order
  | succ k ih =>
      intro a order
      refine ih (step valid a) ?_
      intro i live
      have checked : valid a i = true := mono t a order i (post i live)
      simp [step, order i live, checked]

/-! ## What a fixed point publishes -/

/-- Publication follows the fixed point: a member the route publishes is one
    whose body validated under the very map published with it (V5–V8). -/
theorem every_published_member_validated_under_the_published_map
    {valid : Assign → Nat → Bool} {a : Assign} (post : Post valid a) {i : Nat}
    (published : a i = true) : valid a i = true :=
  post i published

/-- A member whose body the fixed point refuses is never published. -/
theorem a_refused_member_is_never_published
    {valid : Assign → Nat → Bool} {a : Assign} (post : Post valid a) {i : Nat}
    (refused : valid a i = false) : a i = false := by
  cases h : a i with
  | false => rfl
  | true => exact absurd (post i h) (by simp [refused])

/-- A component no map validates publishes nothing at all. -/
theorem an_always_refusing_component_publishes_nothing
    {valid : Assign → Nat → Bool} {a : Assign} (post : Post valid a)
    (refuses : ∀ b i, valid b i = false) (i : Nat) : a i = false :=
  a_refused_member_is_never_published post (refuses a i)

/-- Non-vacuity in the other direction: the route is not a refusal machine. A
    component every member of which validates keeps the top element, so every
    member is published. -/
theorem an_always_validating_component_publishes_every_member
    {n : Nat} {valid : Assign → Nat → Bool} (admits : ∀ b i, valid b i = true) (k : Nat) :
    ∀ i, i < n → iter (step valid) k (top n) i = true := by
  have fixed : ∀ k, iter (step valid) k (top n) = top n := by
    intro k
    induction k with
    | zero => rfl
    | succ k ih =>
        have stable : step valid (top n) = top n := by
          funext i
          simp [step, admits]
        show iter (step valid) k (step valid (top n)) = top n
        rw [stable]
        exact ih
  intro i hi
  rw [fixed k]
  simp [top, hi]

/-! ### The cascade lemma -/

/-- Reachability along the component's intra-component call edges. -/
inductive Reaches (calls : Nat → Nat → Bool) : Nat → Nat → Prop where
  | direct {i j : Nat} : calls i j = true → Reaches calls i j
  | trans {i j k : Nat} : Reaches calls i j → Reaches calls j k → Reaches calls i k

/-- The validation premise the gateway supplies: a body validates only when
    every member it reachably calls is still assumed, because a call into a
    demoted member is an unmodelled call. -/
def CallsAreModelled (valid : Assign → Nat → Bool) (calls : Nat → Nat → Bool) : Prop :=
  ∀ a i, valid a i = true → ∀ j, calls i j = true → a j = true

/-- **Cascade.** Demoting a member demotes everything that reaches it. -/
theorem cascade_demotion_propagates_along_calls
    {valid : Assign → Nat → Bool} {calls : Nat → Nat → Bool}
    (modelled : CallsAreModelled valid calls) {a : Assign} (post : Post valid a)
    {i j : Nat} (reach : Reaches calls i j) (published : a i = true) : a j = true := by
  induction reach with
  | direct edge => exact modelled a _ (post _ published) _ edge
  | trans _ _ ihL ihR => exact ihR (ihL published)

/-- **All-or-nothing publication over a strongly connected component.** If one
    member is demoted, every member is: the fixed point of a strongly connected
    component is either the whole component or empty. This is why the route's
    default publication granularity is the component. -/
theorem a_strongly_connected_component_publishes_all_or_nothing
    {n : Nat} {valid : Assign → Nat → Bool} {calls : Nat → Nat → Bool}
    (modelled : CallsAreModelled valid calls) {a : Assign} (post : Post valid a)
    (connected : ∀ i j, i < n → j < n → Reaches calls i j)
    {j : Nat} (hj : j < n) (demoted : a j = false) :
    ∀ i, i < n → a i = false := by
  intro i hi
  cases h : a i with
  | false => rfl
  | true =>
      exact absurd
        (cascade_demotion_propagates_along_calls modelled post (connected i j hi hj) h)
        (by simp [demoted])

/-! ## Why the optimistic start is admissible only at the bottom element -/

/-- One custody event a frame may perform on an identity its caller holds. -/
inductive Event where
  | quiet
  | release
  | acquire
  deriving DecidableEq, Repr

/-- One concrete execution of a call: the events it performed, and whether it
    returned at all. -/
structure Run where
  events : List Event
  returns : Bool
  deriving DecidableEq, Repr

/-- **(N-prefix).** The neutral claim: at every point of the execution — that
    is, over the whole event list and hence over each of its prefixes — no
    custody event touched an identity the caller holds. -/
def NeutralClaim (run : Run) : Bool := run.events.all (· == Event.quiet)

/-- **(exit claim).** What a `released` contract asserts: *if* the call returns,
    the release happened. It says nothing at all about a call that does not. -/
def ReleaseExitClaim (run : Run) : Bool :=
  !run.returns || run.events.contains Event.release

/-- The result conjunct of one exit's agreement, extracted without unfolding the
    rest of the relation. -/
theorem result_of_exit_match {summary : Summary} {observation : ExitObservation}
    (matched : exitMatches summary observation = true) :
    resultMatches summary observation = true := by
  unfold exitMatches at matched
  exact (Bool.and_eq_true _ _ |>.mp matched).2

private theorem all_of_prefix {p : Event → Bool} {pre run : List Event}
    (prefix_of : pre <+: run) (whole : run.all p = true) : pre.all p = true := by
  obtain ⟨rest, rfl⟩ := prefix_of
  rw [List.all_append] at whole
  exact (Bool.and_eq_true _ _ |>.mp whole).1

/-- The neutral claim is prefix-closed, which is what lets its soundness rest on
    an induction over execution prefixes rather than on a termination rank. -/
theorem neutral_claim_is_prefix_closed {run : Run} (neutral : NeutralClaim run = true)
    {pre : List Event} (prefix_of : pre <+: run.events) :
    NeutralClaim { events := pre, returns := run.returns } = true :=
  all_of_prefix prefix_of neutral

/-- The neutral claim reads the same whether the call returns or not: divergence
    cannot manufacture it, because the fact asserted about a non-returning call
    is the same fact asserted about a returning one. -/
theorem neutral_claim_is_independent_of_returning (events : List Event) (b c : Bool) :
    NeutralClaim { events := events, returns := b }
      = NeutralClaim { events := events, returns := c } := rfl

/-- An exit claim, by contrast, is *entirely* about returning. -/
theorem an_exit_claim_depends_on_returning :
    ReleaseExitClaim { events := [Event.quiet], returns := false } = true ∧
      ReleaseExitClaim { events := [Event.quiet], returns := true } = false := by
  constructor <;> rfl

/-- Every exit claim holds vacuously of a run that never returns. -/
theorem a_divergent_run_satisfies_every_exit_claim {run : Run} (diverges : run.returns = false) :
    ReleaseExitClaim run = true := by
  simp [ReleaseExitClaim, diverges]

/-- **The counterexample, in the model.** Exit-state validation alone cannot
    refuse a circular effectful candidate: here is a run that satisfies the
    `released` exit claim while performing no release at all, and indeed while
    performing the opposite effect. This is why no route may ever seed a
    candidate above the bottom element without a rank. -/
theorem an_exit_claim_cannot_refuse_a_divergent_effectful_candidate :
    ∃ run : Run,
      ReleaseExitClaim run = true ∧
        Event.release ∉ run.events ∧
        NeutralClaim run = false := by
  refine ⟨{ events := [Event.acquire], returns := false }, by rfl, by simp, by rfl⟩

/-- The neutral claim does refuse that run, which is the whole asymmetry: the
    bottom element is checkable on every prefix, an effectful element is not. -/
theorem the_neutral_claim_refuses_an_effectful_run {run : Run}
    (effect : Event.release ∈ run.events) : NeutralClaim run = false := by
  cases whole : NeutralClaim run with
  | false => rfl
  | true =>
      have quiet := List.all_eq_true.mp whole Event.release effect
      simp at quiet

/-! ### The rank an effectful family would have to supply -/

/-- A rank assignment strictly decreasing along every admitted call — the shape
    both implemented effectful recursive routes carry, and the shape the neutral
    route deliberately does not need. -/
def RankedCalls (calls : Nat → Nat → Bool) (rank : Nat → Nat) : Prop :=
  ∀ i j, calls i j = true → rank j < rank i

/-- Ranked calls are well founded: this, and not validation by re-analysis, is
    what refuses a circular effectful candidate. -/
theorem ranked_calls_are_well_founded {calls : Nat → Nat → Bool} {rank : Nat → Nat}
    (ranked : RankedCalls calls rank) :
    WellFounded (fun j i => calls i j = true) := by
  apply Subrelation.wf (r := (measure rank).rel)
  · intro j i edge
    exact ranked i j edge
  · exact (measure rank).wf

/-- A rank-zero member admits no call at all, so an effectful family's base
    context is validated with every component contract absent. -/
theorem a_ranked_base_member_has_no_admitted_call {calls : Nat → Nat → Bool} {rank : Nat → Nat}
    (ranked : RankedCalls calls rank) {i : Nat} (base : rank i = 0) (j : Nat) :
    calls i j = false := by
  cases edge : calls i j with
  | false => rfl
  | true =>
      have := ranked i j edge
      rw [base] at this
      omega

/-! ## What the published neutral summary asserts -/

/-- `N(m)`: every formal `borrowed`, result `neutral`. Not new vocabulary — it
    is the contract ordinary W4 derivation already publishes for an effect-free
    non-recursive body. -/
def neutralSummary (arity : Nat) : Summary :=
  { parameters := List.replicate arity ParameterMode.borrowed, result := ResultMode.neutral }

/-- The neutral summary binds nothing at entry: the assumption installs no
    resource and asserts no non-nullness, so it cannot delete a failure path. -/
theorem the_neutral_summary_binds_no_formal (arity : Nat) :
    (neutralSummary arity).parameters.map entryDisposition = List.replicate arity Option.none := by
  simp [neutralSummary, List.map_replicate, entryDisposition]

/-- It also names no owner, so the alias obligation is empty by construction. -/
theorem the_neutral_summary_names_no_owner (arity : Nat) :
    (neutralSummary arity).parameters.map entryOwner = List.replicate arity Option.none := by
  simp [neutralSummary, List.map_replicate, entryOwner]

theorem the_neutral_summary_governs_every_formal (arity : Nat) :
    governedModes (neutralSummary arity)
      = List.replicate arity (Option.some ParameterMode.borrowed) := by
  simp [governedModes, neutralSummary, List.map_replicate]

/-- V6, pointwise: a formal the neutral summary governs carries no disposition
    at any exit. -/
theorem a_neutral_formal_carries_no_disposition {disposition : Option Disposition}
    (matched : governedMatches (Option.some ParameterMode.borrowed) disposition = true) :
    disposition = Option.none := by
  cases disposition with
  | none => rfl
  | some d => cases d <;> simp [governedMatches, parameterMatches] at matched

/-- V6, pointwise: no out slot is claimed, so none may be written through. -/
theorem a_neutral_formal_writes_through_no_slot {slot : Option OutSlot}
    (matched : outSlotMatches ParameterMode.borrowed slot = true) : slot = Option.none := by
  cases slot with
  | none => rfl
  | some value => cases value <;> simp [outSlotMatches] at matched

/-- V6, pointwise: the sanctioned containment set is empty. -/
theorem a_neutral_formal_is_never_contained {owner : Option Nat}
    (matched : containmentMatches ParameterMode.borrowed owner = true) : owner = Option.none := by
  cases owner with
  | none => rfl
  | some value => simp [containmentMatches] at matched

/-- V6, pointwise: no exit hands back a tracked resource, so the caller receives
    no custody it could release. -/
theorem a_neutral_exit_returns_nothing {arity : Nat} {observation : ExitObservation}
    (matched : resultMatches (neutralSummary arity) observation = true) :
    observation.returned = Returned.nothing := by
  simpa [resultMatches, neutralSummary] using matched

/-- V5 through the unchanged validation relation. -/
theorem published_neutral_member_is_analyzed_safely {arity : Nat} {safe : Bool}
    {exits : List ExitObservation}
    (validated : validate (neutralSummary arity) safe exits = true) : safe = true :=
  validated_summary_is_analyzed_safely _ _ _ validated

/-- V8: an exit is reachable *in the abstract semantics*. This is explicitly not
    a claim that an exit is concretely reachable, and
    `a_divergent_run_satisfies_every_exit_claim` is why no consumer may read it
    as one. -/
theorem published_neutral_member_has_a_reachable_exit {arity : Nat} {safe : Bool}
    {exits : List ExitObservation}
    (validated : validate (neutralSummary arity) safe exits = true) : exits.isEmpty = false :=
  validated_summary_has_a_reachable_exit _ _ _ validated

/-- V6 as a whole: every reachable exit agrees with the neutral summary. -/
theorem published_neutral_member_matches_every_exit {arity : Nat} {safe : Bool}
    {exits : List ExitObservation}
    (validated : validate (neutralSummary arity) safe exits = true) :
    ∀ observation ∈ exits, exitMatches (neutralSummary arity) observation = true :=
  validated_summary_matches_every_exit _ _ _ validated

/-- Publication end to end: a member published by a post-fixed point, whose
    validation is the unchanged relation, hands its callers a summary that is
    analyzed safely, has a reachable exit, agrees with every exit, and returns
    nothing anywhere. -/
theorem a_published_neutral_member_hands_back_no_custody
    {valid : Assign → Nat → Bool} {a : Assign} (post : Post valid a) {i : Nat}
    (published : a i = true) {arity : Nat} {safe : Bool} {exits : List ExitObservation}
    (evidence : valid a i = true →
      validate (neutralSummary arity) safe exits = true) :
    safe = true ∧ exits.isEmpty = false ∧
      ∀ observation ∈ exits, observation.returned = Returned.nothing := by
  have validated := evidence (post i published)
  refine ⟨published_neutral_member_is_analyzed_safely validated,
    published_neutral_member_has_a_reachable_exit validated, ?_⟩
  intro observation member
  have matched := published_neutral_member_matches_every_exit validated observation member
  exact a_neutral_exit_returns_nothing (result_of_exit_match matched)

end QCore.NeutralFrameRecursion
