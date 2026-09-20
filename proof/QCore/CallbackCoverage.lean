import Std

namespace QCore.CallbackCoverage

/-! Total, fail-closed accounting for the current Monkey callback route.

This is the semantic boundary implemented by
`qcore_verification::callback_coverage`. The Rust status vocabulary currently
contains only `Unknown`: there is no source-to-event-program projection and no
composed-certificate input. Any future composed arm needs a new model and a
separate Rust implementation; it is not represented here speculatively. -/

inductive CurrentStatus where
  | unknown
  deriving DecidableEq, Repr

structure CurrentEvidence where
  sourceKnown : Bool
  everySelectedPathChecked : Bool
  deriving DecidableEq, Repr

def currentAccount (_evidence : CurrentEvidence) : CurrentStatus :=
  .unknown

/-- Complete native path replay is still `Unknown` on the current route. -/
theorem every_current_account_is_unknown (evidence : CurrentEvidence) :
    currentAccount evidence = .unknown := rfl

def accounted (selected reported : List String) : Bool :=
  decide selected.Nodup && decide reported.Nodup &&
    selected.all reported.contains && reported.all selected.contains

theorem accounted_has_exact_members
    {selected reported : List String} (accepted : accounted selected reported = true) :
    ∀ callback, callback ∈ selected ↔ callback ∈ reported := by
  have facts :
      ((decide selected.Nodup = true ∧ decide reported.Nodup = true) ∧
      selected.all reported.contains = true) ∧ reported.all selected.contains = true := by
    simpa [accounted, Bool.and_eq_true] using accepted
  intro callback
  constructor
  · intro present
    have contained := List.all_eq_true.mp facts.1.2 callback present
    simpa using contained
  · intro present
    have contained := List.all_eq_true.mp facts.2 callback present
    simpa using contained

/-- Omitting a selected callback cannot satisfy total accounting. -/
theorem omitted_selected_callback_is_not_accounted
    (selected reported : List String) (callback : String)
    (present : callback ∈ selected) (missing : callback ∉ reported) :
    accounted selected reported = false := by
  apply Bool.eq_false_iff.mpr
  intro accepted
  exact missing ((accounted_has_exact_members accepted callback).mp present)

/-- Duplicate rows are rejected rather than used to inflate accounting. -/
theorem duplicate_callback_is_not_accounted
    (selected reported : List String) (callback : String) :
    accounted selected (callback :: callback :: reported) = false := by
  simp [accounted]

/-- A row outside the selected denominator is rejected. -/
theorem unselected_callback_is_not_accounted
    (selected reported : List String) (callback : String)
    (foreign : callback ∉ selected) :
    accounted selected (callback :: reported) = false := by
  apply Bool.eq_false_iff.mpr
  intro accepted
  exact foreign ((accounted_has_exact_members accepted callback).mpr (by simp))

end QCore.CallbackCoverage
