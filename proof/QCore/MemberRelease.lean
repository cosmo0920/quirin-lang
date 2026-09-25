import QCore.CallArgumentAccess

/-!
# Member releases, member null tests and the unit-callee guard
(card W45-CALLEE-SUMMARY-CASCADE)

The abstract rules implemented by `legacy_member_release.rs` and its hooks in
`legacy_call_argument_access.rs` and `legacy_c.rs` (`crates/qcore-c-gateway`).

1. **A pointer member handed to a nullable release.** A typed pointer read
   (`B->F`, `B[I]`, `*B`) with at least one dereferenced base is admitted as a
   whole actual at a `Released { nullable: true }` position, and only there.
   The admitted call is lowered with that position read as `Borrowed`, after
   the unchanged obligation block of `QCore.CallArgumentAccess` (the typed read
   of every base, then `ArgumentReads`).
2. **A pointer member null test** lowers to the typed read of its base: a read
   with no destination and no frame, refining nothing on either edge.
3. **The guard.** Before every call to a function the unit defines,
   `ArgumentReads` over the call's plain-name actuals refuses an actual whose
   target owns sub-custody. The node is appended and every edge into the call
   is redirected to it.

## Premises, stated and not proved here

* The value a pointer read yields denotes a tracked identity only if its base
  owns sub-custody or its own place is bound (`TrackedOnlyThrough`). This is
  W44-INDIRECT-CALL-ARGUMENT-ACCESS's premise 4, a construction fact of the
  projection (field links, contained children, collections and bound places
  are its only records of a pointer stored in memory).
* A site a caller tracks is reachable through an actual only through that
  actual's sub-custody (`ReachableOnlyThroughSubCustody`), by the same
  construction.
* Faithful C token/type extraction and the Rust correspondence are review
  premises (trust boundary 28). Untracked liveness is outside the property.
-/

namespace QCore.MemberRelease

open QCore.CallArgumentAccess (Facts ownsSubCustody refusedReads Instr)

/-! ## Rule 1: admission -/

/-- A contract's mode at one position, as far as rule 1 distinguishes it. -/
inductive Mode where
  | borrowed
  | releasedNullable
  | releasedNonnull
  | consumed
  | owned
  | out
  | stored
  | removed
  deriving DecidableEq, Repr

/-- `admits_release`. -/
def admitsRelease (mode : Option Mode) (pointerValued : Bool) (bases : List Nat) : Bool :=
  pointerValued && !bases.isEmpty && mode == some .releasedNullable

theorem only_a_nullable_release_admits (mode : Option Mode) (pv : Bool) (bases : List Nat)
    (h : admitsRelease mode pv bases = true) : mode = some .releasedNullable := by
  simp only [admitsRelease, Bool.and_eq_true, beq_iff_eq] at h
  exact h.2

theorem a_non_null_release_never_admits (pv : Bool) (bases : List Nat) :
    admitsRelease (some .releasedNonnull) pv bases = false := by
  simp [admitsRelease]

theorem a_consuming_position_never_admits (pv : Bool) (bases : List Nat) :
    admitsRelease (some .consumed) pv bases = false := by
  simp [admitsRelease]

theorem a_scalar_value_never_admits (mode : Option Mode) (bases : List Nat) :
    admitsRelease mode false bases = false := by
  simp [admitsRelease]

/-- `S.F` over a local record dereferences no base and is never released. -/
theorem a_baseless_read_never_admits (mode : Option Mode) (pv : Bool) :
    admitsRelease mode pv [] = false := by
  simp [admitsRelease]

/-! ## Rule 1: the lowering changes no tracked effect -/

/-- The custody effect of a call on the set of released tracked sites: every
released position whose actual denotes a tracked site releases it. -/
def releasesAt (mode : Mode) : Bool :=
  mode == .releasedNullable || mode == .releasedNonnull

def effect (modes : List Mode) (actuals : List (Option Nat)) (released : Nat → Bool) :
    Nat → Bool :=
  fun site => released site ||
    ((modes.zip actuals).any fun pair => releasesAt pair.1 && pair.2 == some site)

/-- `rewrite_released_positions`: the admitted position is read as `Borrowed`. -/
def rewrite (modes : List Mode) (index : Nat) : List Mode := modes.set index .borrowed

theorem zip_set_none_any (modes : List Mode) (actuals : List (Option Nat)) (index site : Nat)
    (h : actuals[index]? = some none) :
    ((modes.set index .borrowed).zip actuals).any
        (fun pair => releasesAt pair.1 && pair.2 == some site) =
      (modes.zip actuals).any (fun pair => releasesAt pair.1 && pair.2 == some site) := by
  induction modes generalizing actuals index with
  | nil => simp
  | cons mode rest ih =>
    cases actuals with
    | nil => simp at h
    | cons actual more =>
      cases index with
      | zero =>
        simp at h
        subst h
        simp [releasesAt]
      | succ index =>
        simp only [List.set_cons_succ, List.zip_cons_cons, List.any_cons]
        rw [ih more index (by simpa using h)]

/-- When the admitted actual denotes no tracked site, reading its position as
`Borrowed` is exactly the call's own effect on every tracked site. -/
theorem the_rewrite_changes_no_tracked_effect (modes : List Mode)
    (actuals : List (Option Nat)) (index : Nat) (released : Nat → Bool)
    (h : actuals[index]? = some none) :
    effect (rewrite modes index) actuals released = effect modes actuals released := by
  funext site
  simp only [effect, rewrite]
  rw [zip_set_none_any modes actuals index site h]

/-- The projection's construction premise: a pointer read through `base`
spelling `place` denotes a tracked site only through sub-custody or a bound
place. -/
def TrackedOnlyThrough (f : Facts) (base place : Nat) (denotes : Option Nat) : Prop :=
  denotes.isSome = true → ownsSubCustody f base = true ∨ f.placeBound place = true

/-- The second obligation passing means the released value is untracked. -/
theorem an_admitted_release_names_no_tracked_identity (f : Facts) (base place : Nat)
    (denotes : Option Nat) (premise : TrackedOnlyThrough f base place denotes)
    (passes : refusedReads f [] [base] [place] = []) : denotes = none := by
  cases hd : denotes with
  | none => rfl
  | some site =>
    exfalso
    have := premise (by simp [hd])
    simp only [refusedReads, List.filter_nil, List.nil_append, List.append_eq_nil_iff,
      List.filter_eq_nil_iff, List.mem_singleton, forall_eq] at passes
    cases this with
    | inl h => simp [h] at passes
    | inr h => simp [h] at passes

/-- Rule 1 end to end: under the premise, a call admitted with its obligations
passing releases exactly the tracked sites the rewritten call releases. -/
theorem an_admitted_member_release_releases_no_tracked_site (f : Facts) (base place : Nat)
    (modes : List Mode) (actuals : List (Option Nat)) (index : Nat) (released : Nat → Bool)
    (denotes : Option Nat) (hactual : actuals[index]? = some denotes)
    (premise : TrackedOnlyThrough f base place denotes)
    (passes : refusedReads f [] [base] [place] = []) :
    effect (rewrite modes index) actuals released = effect modes actuals released := by
  have hn := an_admitted_release_names_no_tracked_identity f base place denotes premise passes
  subst hn
  exact the_rewrite_changes_no_tracked_effect modes actuals index released hactual

/-- A base that owns sub-custody refuses, so the function cannot be `Safe`. -/
theorem a_linked_owner_refuses_the_release (f : Facts) (base place : Nat)
    (h : ownsSubCustody f base = true) : refusedReads f [] [base] [place] ≠ [] := by
  simp [refusedReads, h]

/-- A bound place refuses, so the function cannot be `Safe`. -/
theorem a_bound_place_refuses_the_release (f : Facts) (base place : Nat)
    (h : f.placeBound place = true) : refusedReads f [] [base] [place] ≠ [] := by
  simp [refusedReads, h]

/-! ## Rule 2: a member null test is a read -/

/-- The lowering of a controlling expression: the member null test's base read,
or the refusal when the reader declines. -/
def lowerNullTest : Option Nat → Instr
  | some base => .typedRead [base]
  | none => .refusal

/-- Both successors of the test receive the state the read leaves, which is the
state before it: the typed read has no destination and no frame. -/
theorem a_member_null_test_changes_no_binding (f : Facts) (base : Nat) :
    (QCore.IndirectScalarAccess.transfer f.state ⟨[base], none, none⟩).1.binding =
      f.state.binding := by
  funext name
  exact QCore.IndirectScalarAccess.without_a_frame_only_the_destination_changes f.state
    ⟨[base], none, none⟩ name rfl (by simp)

theorem a_declined_null_test_keeps_the_refusal : lowerNullTest none = .refusal := rfl

theorem an_admitted_null_test_obliges_its_base (base : Nat) :
    lowerNullTest (some base) = .typedRead [base] := rfl

/-! ## Rule 3: the guard -/

/-- The caller-side construction premise: a site the caller tracks is reachable
through an actual only through that actual's sub-custody. -/
def ReachableOnlyThroughSubCustody (f : Facts) (reach : Nat → List Nat) : Prop :=
  ∀ actual, reach actual ≠ [] → ownsSubCustody f actual = true

/-- A passing guard leaves no caller-tracked site reachable through any actual,
so a summary that releases objects reachable through its formals releases none
the caller tracks. -/
theorem a_passing_guard_hides_no_caller_release (f : Facts) (reach : Nat → List Nat)
    (actuals : List Nat) (premise : ReachableOnlyThroughSubCustody f reach)
    (passes : refusedReads f [] actuals [] = []) : ∀ actual ∈ actuals, reach actual = [] := by
  intro actual hmem
  cases hr : reach actual with
  | nil => rfl
  | cons first rest =>
    exfalso
    have hs := premise actual (by simp [hr])
    simp only [refusedReads, List.filter_nil, List.nil_append, List.append_nil,
      List.filter_eq_nil_iff] at passes
    have := passes actual hmem
    simp [hs] at this

/-- The guard refuses exactly when an actual owns sub-custody. -/
theorem the_guard_refuses_an_owner_of_sub_custody (f : Facts) (actuals : List Nat)
    (actual : Nat) (hmem : actual ∈ actuals) (h : ownsSubCustody f actual = true) :
    refusedReads f [] actuals [] ≠ [] := by
  intro hnil
  simp only [refusedReads, List.filter_nil, List.nil_append, List.append_nil,
    List.filter_eq_nil_iff] at hnil
  have := hnil actual hmem
  simp [h] at this

/-! ## Rule 3: inserting the guard node -/

/-- A CFG as its successor lists, indexed by node id. -/
abbrev Graph := List (List Nat)

def redirect (target fresh : Nat) (successors : List Nat) : List Nat :=
  successors.map fun next => if next = target then fresh else next

/-- `guard_unit_callees` for one call node `target`: every edge into it is
redirected to the appended guard, whose only successor is the call. -/
def insertGuard (g : Graph) (target : Nat) : Graph :=
  g.map (redirect target g.length) ++ [[target]]

theorem the_guard_is_appended (g : Graph) (target : Nat) :
    (insertGuard g target).length = g.length + 1 := by
  simp [insertGuard]

theorem the_guard_leads_to_the_call (g : Graph) (target : Nat) :
    (insertGuard g target)[g.length]? = some [target] := by
  simp [insertGuard]

/-- No existing node's identity moves: node `u` keeps its successors, with the
call replaced by the guard. -/
theorem every_existing_node_keeps_its_id (g : Graph) (target u : Nat) (hu : u < g.length) :
    (insertGuard g target)[u]? = (g[u]?).map (redirect target g.length) := by
  simp [insertGuard, List.getElem?_append_left, hu]

/-- After insertion no existing node reaches the call except through the
guard. -/
theorem every_edge_into_the_call_passes_the_guard (g : Graph) (target u next : Nat)
    (ht : target < g.length) (hu : u < g.length) (successors : List Nat)
    (hs : (insertGuard g target)[u]? = some successors) (hn : next ∈ successors) :
    next ≠ target := by
  rw [every_existing_node_keeps_its_id g target u hu] at hs
  cases hg : g[u]? with
  | none => simp [hg] at hs
  | some old =>
    simp only [hg, Option.map_some, Option.some.injEq] at hs
    subst hs
    simp only [redirect, List.mem_map] at hn
    obtain ⟨previous, _, hprev⟩ := hn
    by_cases hp : previous = target
    · simp [hp] at hprev
      omega
    · simp [hp] at hprev
      rw [← hprev]
      exact hp

/-! ## Rule 3: which callees are guarded

`member_releasing_functions` selects, lexically, every body with a call whose
whole actual is a pointer read (`candidate`), closed under "calls a selected
body". A summary can hide a rule-1 release only if its body, or a body its
summary chain reaches, used rule 1, which needs such a call. -/

/-- `f` reaches a candidate body through calls (a summary chain). -/
inductive ReachesCandidate (calls : Nat → List Nat) (candidate : Nat → Bool) : Nat → Prop where
  | self (f : Nat) : candidate f = true → ReachesCandidate calls candidate f
  | step (f g : Nat) : g ∈ calls f → ReachesCandidate calls candidate g →
      ReachesCandidate calls candidate f

/-- The selection's two closure facts, as the Rust fixed point establishes them. -/
structure Selection (calls : Nat → List Nat) (candidate selected : Nat → Bool) : Prop where
  candidates : ∀ f, candidate f = true → selected f = true
  closed : ∀ f g, g ∈ calls f → selected g = true → selected f = true

/-- Every body whose summary chain reaches a rule-1 candidate is selected, so
every call to it is guarded (unless a declaration governs it). -/
theorem every_summary_chain_to_a_member_release_is_guarded (calls : Nat → List Nat)
    (candidate selected : Nat → Bool) (h : Selection calls candidate selected) (f : Nat)
    (hr : ReachesCandidate calls candidate f) : selected f = true := by
  induction hr with
  | self f hc => exact h.candidates f hc
  | step f g hg _ ih => exact h.closed f g hg ih

/-! ## Witnesses -/

/-- `rd_kafka_error_destroy(acl_res->error)` at `Released { nullable }`:
admitted; at `Released { nonnull }` or from `S.F`: refused. -/
theorem the_witness_admissions :
    admitsRelease (some .releasedNullable) true [5] = true ∧
      admitsRelease (some .releasedNonnull) true [5] = false ∧
      admitsRelease (some .releasedNullable) true [] = false := by
  decide

/-- A two-node graph `0 → 1` guarded at `1`: `0 → 2 → 1`. -/
theorem the_witness_insertion : insertGuard [[1], []] 1 = [[2], [], [1]] := by decide

end QCore.MemberRelease
