import Std

namespace QCore.ChunkLifecycleCoverage

/-! Total, fail-closed lifecycle accounting for a source-selected chunk
function inventory.

This is the semantic boundary implemented by
`qcore_verification::chunk_lifecycle_coverage`. `QCore.ChunkLifecycle` already
states what an *accepted teardown* discharges; what is stated here is the layer
above it: when a **function's** lifecycle result may be `Safe`, and that every
other shape keeps an `Unknown` naming its outstanding legs.

Three facts are the point of the module.

* `Safe` needs all nine legs **and** a terminal composition. A prefix
  composition — the shape of a close whose container stays live by premise —
  cannot reach it, because an assumed premise is not a discharge.
* A delete-only operation is never `Safe` and keeps a nonempty retained
  post-state.
* Accounting is exact: an omitted, duplicated or unselected row rejects the
  operation instead of shrinking the denominator. -/

inductive Leg where
  | contextTeardown
  | streamTeardown
  | chunkTeardown
  | backendTeardown
  | primaryListUnlink
  | residencyListUnlink
  | contextCounters
  | errorExits
  | wrapperObligations
  deriving DecidableEq, BEq, Repr

/-- The nine obligations an accepted lifecycle result must discharge. -/
def requiredLegs : List Leg :=
  [.contextTeardown, .streamTeardown, .chunkTeardown, .backendTeardown,
   .primaryListUnlink, .residencyListUnlink, .contextCounters,
   .errorExits, .wrapperObligations]

theorem requiredLegs_length : requiredLegs.length = 9 := rfl

inductive Role where
  | destructionDependent
  | deleteOnly
  | nonDestructive
  | unclassified
  deriving DecidableEq, BEq, Repr

/-- How far a composed model program reaches. -/
inductive CompositionKind where
  | terminal
  | «prefix»
  deriving DecidableEq, BEq, Repr

/-- A composition carries its kind together with the legs the re-run program
actually reached. There is no way to carry legs without a composition. -/
structure Composition where
  kind : CompositionKind
  legs : List Leg
  deriving DecidableEq, BEq, Repr

structure Evidence where
  role : Role
  composition : Option Composition
  /-- Identities a partial operation leaves live. -/
  retained : List String
  errorExitsModeled : Bool
  wrapperDischarged : Bool
  deriving DecidableEq, BEq, Repr

def Evidence.modelLegs (evidence : Evidence) : List Leg :=
  match evidence.composition with
  | none => []
  | some composition => composition.legs

def Evidence.discharged (evidence : Evidence) : List Leg :=
  evidence.modelLegs ++
    (if evidence.errorExitsModeled then [Leg.errorExits] else []) ++
    (if evidence.wrapperDischarged then [Leg.wrapperObligations] else [])

def Evidence.outstanding (evidence : Evidence) : List Leg :=
  requiredLegs.filter (fun leg => !evidence.discharged.contains leg)

/-- A row is well formed when its role and its evidence agree: only a
destruction-dependent row carries a composition, and a delete-only row retains
at least one live identity. The Rust adapter rejects every other shape as an
operation error rather than publishing it as a residual. -/
def Evidence.WellFormed (evidence : Evidence) : Prop :=
  (evidence.role ≠ .destructionDependent → evidence.composition = none) ∧
  (evidence.role = .deleteOnly → evidence.retained ≠ []) ∧
  (evidence.role ≠ .deleteOnly → evidence.retained = [])

inductive Status where
  | safe
  | unknown (outstanding : List Leg)
  deriving DecidableEq, BEq, Repr

def isSafe (evidence : Evidence) : Bool :=
  decide (evidence.role = Role.destructionDependent) &&
    (match evidence.composition with
     | some composition => decide (composition.kind = CompositionKind.terminal)
     | none => false) &&
    evidence.outstanding.isEmpty

def account (evidence : Evidence) : Status :=
  if isSafe evidence then .safe else .unknown evidence.outstanding

/-- An accepted lifecycle result is destruction-dependent, carries a terminal
composition, and has no outstanding leg. -/
theorem safe_is_terminal_and_complete {evidence : Evidence}
    (accepted : account evidence = .safe) :
    evidence.role = .destructionDependent ∧
      (∃ composition, evidence.composition = some composition ∧
        composition.kind = .terminal) ∧
      evidence.outstanding = [] := by
  have safe : isSafe evidence = true := by
    cases decision : isSafe evidence with
    | true => rfl
    | false => simp [account, decision] at accepted
  have facts :
      (decide (evidence.role = Role.destructionDependent) = true ∧
        (match evidence.composition with
         | some composition => decide (composition.kind = CompositionKind.terminal)
         | none => false) = true) ∧
      evidence.outstanding.isEmpty = true := by
    simpa [isSafe, Bool.and_eq_true] using safe
  refine ⟨of_decide_eq_true facts.1.1, ?_, List.isEmpty_iff.mp facts.2⟩
  cases composition : evidence.composition with
  | none =>
    rw [composition] at facts
    exact absurd facts.1.2 (by simp)
  | some value =>
    refine ⟨value, rfl, ?_⟩
    rw [composition] at facts
    exact of_decide_eq_true facts.1.2

/-- Every required leg an accepted result does not discharge is impossible:
an accepted result discharges all nine. -/
theorem safe_discharges_every_required_leg {evidence : Evidence}
    (accepted : account evidence = .safe) {leg : Leg} (required : leg ∈ requiredLegs) :
    evidence.discharged.contains leg = true := by
  have empty := (safe_is_terminal_and_complete accepted).2.2
  cases contained : evidence.discharged.contains leg with
  | true => rfl
  | false =>
    have member : leg ∈ evidence.outstanding :=
      List.mem_filter.mpr ⟨required, by simp [contained]⟩
    rw [empty] at member
    cases member

/-- A missing leg keeps the function `Unknown`, and the published row names it. -/
theorem a_missing_leg_stays_unknown {evidence : Evidence} {leg : Leg}
    (required : leg ∈ requiredLegs) (missing : evidence.discharged.contains leg = false) :
    account evidence = .unknown evidence.outstanding ∧ leg ∈ evidence.outstanding := by
  have member : leg ∈ evidence.outstanding :=
    List.mem_filter.mpr ⟨required, by simp [missing]⟩
  refine ⟨?_, member⟩
  have nonempty : evidence.outstanding.isEmpty = false := by
    cases values : evidence.outstanding.isEmpty with
    | false => rfl
    | true =>
      have empty := List.isEmpty_iff.mp values
      rw [empty] at member
      cases member
  have refused : isSafe evidence = false := by
    simp [isSafe, nonempty]
  simp [account, refused]

/-- A row whose role is not destruction-dependent is never `Safe`. -/
theorem only_destruction_dependent_rows_can_be_safe {evidence : Evidence}
    (other : evidence.role ≠ .destructionDependent) : account evidence ≠ .safe := by
  intro accepted
  exact other (safe_is_terminal_and_complete accepted).1

theorem delete_only_is_never_safe {evidence : Evidence}
    (role : evidence.role = .deleteOnly) : account evidence ≠ .safe := by
  refine only_destruction_dependent_rows_can_be_safe ?_
  rw [role]
  intro contradiction
  cases contradiction

theorem unclassified_is_never_safe {evidence : Evidence}
    (role : evidence.role = .unclassified) : account evidence ≠ .safe := by
  refine only_destruction_dependent_rows_can_be_safe ?_
  rw [role]
  intro contradiction
  cases contradiction

theorem non_destructive_is_never_safe {evidence : Evidence}
    (role : evidence.role = .nonDestructive) : account evidence ≠ .safe := by
  refine only_destruction_dependent_rows_can_be_safe ?_
  rw [role]
  intro contradiction
  cases contradiction

/-- A composition that only reaches a prefix leaves its container live by
premise; it cannot promote the function. -/
theorem a_prefix_composition_is_never_safe {evidence : Evidence} {composition : Composition}
    (bound : evidence.composition = some composition)
    (partial_reach : composition.kind = .prefix) : account evidence ≠ .safe := by
  intro accepted
  obtain ⟨value, equality, terminal⟩ := (safe_is_terminal_and_complete accepted).2.1
  rw [bound] at equality
  have same : composition = value := Option.some.inj equality
  subst same
  rw [partial_reach] at terminal
  exact absurd terminal (by simp)

/-- With no composition at all the seven model legs are outstanding, so the row
stays `Unknown` however much source evidence accompanies it. -/
theorem no_composition_is_never_safe {evidence : Evidence}
    (absent : evidence.composition = none) : account evidence ≠ .safe := by
  intro accepted
  obtain ⟨_, equality, _⟩ := (safe_is_terminal_and_complete accepted).2.1
  rw [absent] at equality
  exact absurd equality (by simp)

/-- A well-formed delete-only row keeps a nonempty retained post-state. -/
theorem delete_only_retains_its_partial_post_state {evidence : Evidence}
    (wellFormed : evidence.WellFormed) (role : evidence.role = .deleteOnly) :
    evidence.retained ≠ [] := wellFormed.2.1 role

/-- A well-formed row that is not destruction-dependent carries no composition,
so no leg can arrive without the role that admits it. -/
theorem only_destruction_dependent_rows_carry_a_composition {evidence : Evidence}
    (wellFormed : evidence.WellFormed) (other : evidence.role ≠ .destructionDependent) :
    evidence.modelLegs = [] := by
  unfold Evidence.modelLegs
  rw [wellFormed.1 other]

/-! ### Exact accounting -/

def accounted (selected reported : List String) : Bool :=
  decide selected.Nodup && decide reported.Nodup &&
    selected.all reported.contains && reported.all selected.contains

theorem accounted_has_exact_members
    {selected reported : List String} (accepted : accounted selected reported = true) :
    ∀ function, function ∈ selected ↔ function ∈ reported := by
  have facts :
      ((decide selected.Nodup = true ∧ decide reported.Nodup = true) ∧
      selected.all reported.contains = true) ∧ reported.all selected.contains = true := by
    simpa [accounted, Bool.and_eq_true] using accepted
  intro function
  constructor
  · intro present
    simpa using List.all_eq_true.mp facts.1.2 function present
  · intro present
    simpa using List.all_eq_true.mp facts.2 function present

theorem omitted_selected_function_is_not_accounted
    (selected reported : List String) (function : String)
    (present : function ∈ selected) (missing : function ∉ reported) :
    accounted selected reported = false := by
  apply Bool.eq_false_iff.mpr
  intro accepted
  exact missing ((accounted_has_exact_members accepted function).mp present)

theorem duplicate_function_is_not_accounted
    (selected reported : List String) (function : String) :
    accounted selected (function :: function :: reported) = false := by
  simp [accounted]

theorem unselected_function_is_not_accounted
    (selected reported : List String) (function : String)
    (foreign : function ∉ selected) :
    accounted selected (function :: reported) = false := by
  apply Bool.eq_false_iff.mpr
  intro accepted
  exact foreign ((accounted_has_exact_members accepted function).mpr (by simp))

end QCore.ChunkLifecycleCoverage
