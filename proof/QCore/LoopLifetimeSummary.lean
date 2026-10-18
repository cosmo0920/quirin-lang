import QCore.RetentionPath
import QCore.LocalCondition
/-! S3 summary abstraction boundaries. Source/type/store extraction remains a
trust boundary. Keys include source/configuration/function scope; no candidate
site overlap supplies runtime equality. Endings across a call use only must
destroys: unordered removals cannot reconstruct incoming poisoning or order.
Rust: loop_lifetime_summary.rs and loop_lifetime_source.rs. -/
namespace QCore.LoopLifetimeSummary
inductive Root where
  | formal (scope index : Nat)
  | result (scope : Nat)
  | lifted (footprint : Nat)
  deriving DecidableEq, Repr
structure Path where
  root : Root
  fields : List Nat
  deriving DecidableEq, Repr
structure Registration where
  object : Path
  holder : Path
  poisoned : Bool
  deriving DecidableEq, Repr

def concrete (path : Path) : Bool :=
  match path.root with
  | .lifted _ => false
  | _ => true

def mayAlias (_left _right : Path) : Bool := true

def destroy (holder : Path) (rs : List Registration) : List Registration :=
  if concrete holder then rs.filter (fun r => r.holder != holder) else rs

structure ReleaseEffect where
  object : Path
  destroyedBefore : List Path
  deriving DecidableEq, Repr

/-- A may-release acts on incoming records before the callee's postconditions.
Only holders already destroyed before that release discharge it. -/
def releaseRefuses (effect : ReleaseEffect) (record : Registration) : Bool :=
  mayAlias effect.object record.object &&
    !(concrete record.holder && effect.destroyedBefore.contains record.holder)

theorem a_release_without_an_earlier_ending_refuses_every_incoming_record
    (object : Path) (record : Registration) :
    releaseRefuses ⟨object, []⟩ record = true := by
  simp [releaseRefuses, mayAlias]

theorem a_destroy_before_release_discharges_its_exact_holder
    (object : Path) (record : Registration) (knownHolder : concrete record.holder = true) :
    releaseRefuses ⟨object, [record.holder]⟩ record = false := by
  simp [releaseRefuses, knownHolder]

theorem a_later_destroy_does_not_retroactively_discharge_a_release
    (object _laterDestroy : Path) (record : Registration) :
    releaseRefuses ⟨object, []⟩ record = true := by
  simp [releaseRefuses, mayAlias]

theorem a_lifted_prefix_never_discharges_a_release
    (effect : ReleaseEffect) (record : Registration)
    (unknownHolder : concrete record.holder = false) : releaseRefuses effect record = true := by
  simp [releaseRefuses, mayAlias, unknownHolder]

def mayJoin (left right : List Registration) : List Registration := left ++ right

def mustJoin (left right : List Path) : List Path :=
  left.filter (fun p => right.contains p)

def admitted (openReasons obligations : List Nat) : Bool :=
  openReasons.isEmpty && obligations.isEmpty

def demote (old discovered : List Nat) : List Nat := old ++ discovered

def appendPath (base : Path) (fields : List Nat) : Path :=
  { root := base.root, fields := base.fields ++ fields }

def substitute (scope : Nat) (actuals : List (Option Path))
    (resultPath : Option Path) (footprint : Option Nat) (path : Path) : Option Path :=
  let liftOr (base : Option Path) :=
    match base with
    | some p => some (appendPath p path.fields)
    | none => footprint.map fun f => { root := .lifted f, fields := path.fields }
  match path.root with
  | .formal owner i => if owner == scope then
      match actuals[i]? with
      | some actual => liftOr actual
      | none => none
    else none
  | .result owner => if owner == scope then liftOr resultPath else none
  | .lifted _ => some path

theorem a_may_join_keeps_every_left_registration
    (left right : List Registration) (r : Registration) (h : r ∈ left) :
    r ∈ mayJoin left right := by simp [mayJoin, h]

theorem a_may_join_keeps_every_right_registration
    (left right : List Registration) (r : Registration) (h : r ∈ right) :
    r ∈ mayJoin left right := by simp [mayJoin, h]

theorem a_must_ending_stands_on_both_paths
    (left right : List Path) (p : Path) (h : p ∈ mustJoin left right) :
    p ∈ left ∧ p ∈ right := by
  simpa [mustJoin, List.mem_filter, List.contains_iff_mem] using h

theorem a_destroy_ends_every_registration_at_its_holder
    (rs : List Registration) (h : Path) (r : Registration)
    (atHolder : r.holder = h) (knownHolder : concrete h = true) : r ∉ destroy h rs := by
  simp [destroy, knownHolder, List.mem_filter, atHolder]

theorem a_destroy_keeps_other_holders
    (rs : List Registration) (h : Path) (r : Registration)
    (present : r ∈ rs) (other : r.holder ≠ h) : r ∈ destroy h rs := by
  simp only [destroy]
  split <;> simp_all [List.mem_filter]

theorem one_lifted_holder_never_ends_a_whole_footprint
    (rs : List Registration) (footprint : Nat) (fields : List Nat) :
    destroy ⟨.lifted footprint, fields⟩ rs = rs := by simp [destroy, concrete]

theorem formal_and_result_roots_remain_possible_aliases
    (owner scope index : Nat) (left right : List Nat) :
    mayAlias ⟨.formal owner index, left⟩ ⟨.result scope, right⟩ = true := rfl

theorem the_fold_only_demotes (old discovered : List Nat) (n : Nat)
    (h : n ∈ old) : n ∈ demote old discovered := by simp [demote, h]

theorem an_open_summary_is_never_admitted (reason : Nat)
    (rest obligations : List Nat) : admitted (reason :: rest) obligations = false := by
  simp [admitted]

theorem an_undischarged_precondition_is_never_admitted
    (reasons : List Nat) (obligation : Nat) (rest : List Nat) :
    admitted reasons (obligation :: rest) = false := by simp [admitted]

theorem a_foreign_function_formal_never_substitutes
    (scope owner index : Nat) (actuals : List (Option Path))
    (resultPath : Option Path) (footprint : Option Nat) (fields : List Nat)
    (different : owner ≠ scope) :
    substitute scope actuals resultPath footprint ⟨.formal owner index, fields⟩ = none := by
  simp [substitute, different]

theorem a_known_actual_preserves_its_own_root
    (scope index : Nat) (actuals : List (Option Path)) (actual : Path)
    (resultPath : Option Path) (footprint : Option Nat) (fields : List Nat)
    (known : actuals[index]? = some (some actual)) :
    substitute scope actuals resultPath footprint ⟨.formal scope index, fields⟩ =
      some (appendPath actual fields) := by simp [substitute, known]

theorem an_unresolved_actual_without_a_footprint_never_substitutes
    (scope index : Nat) (actuals : List (Option Path))
    (resultPath : Option Path) (fields : List Nat)
    (unknownActual : actuals[index]? = some none) :
    substitute scope actuals resultPath none ⟨.formal scope index, fields⟩ = none := by
  simp [substitute, unknownActual]

theorem a_closed_footprint_keeps_an_unnameable_registration_root
    (scope index footprint : Nat) (actuals : List (Option Path))
    (resultPath : Option Path) (fields : List Nat)
    (unknownActual : actuals[index]? = some none) :
    substitute scope actuals resultPath (some footprint) ⟨.formal scope index, fields⟩ =
      some ⟨.lifted footprint, fields⟩ := by
  simp [substitute, unknownActual]

/-- Shared with Rust's shared_destroy_vectors; source identity/canonicalization
is an extraction premise, not established by these abstract identifiers. -/
def destroyVectors : List (Bool × Bool × Bool × Bool) := [
  (false, true, false, false), (true, true, false, false),
  (false, false, false, true), (true, false, false, true),
  (false, true, true, true), (true, true, true, true)]

def destroyVector (poisoned sameHolder lifted : Bool) : Bool :=
  let holder : Path := ⟨if lifted then .lifted 0 else .formal 0 0, [1]⟩
  let selected : Path := if sameHolder then holder else ⟨.formal 0 1, [1]⟩
  let record : Registration := ⟨⟨.formal 0 0, [0]⟩, holder, poisoned⟩
  !(destroy selected [record]).isEmpty

theorem shared_destroy_vectors_hold :
    destroyVectors.all (fun (p, s, l, expected) => destroyVector p s l == expected) = true := by
  decide

/-- The single-body retention machine's poisoning rule remains authoritative;
S3 never reinterprets an unmatched removal as an identity operation. -/
theorem the_ordered_poisoning_boundary
    (events : List QCore.RetentionPath.Event) (record : QCore.RetentionPath.Rec)
    (records : QCore.RetentionPath.Records) (present : record ∈ records)
    (poisoned : record.poisoned = true)
    (notEnded : ∀ event ∈ events, QCore.RetentionPath.ends event record.holder = false) :
    ∀ after, QCore.RetentionPath.run records events = some after → record ∈ after :=
  QCore.RetentionPath.a_poisoned_record_ends_only_by_its_loop_destroy
    events record records present poisoned notEnded

end QCore.LoopLifetimeSummary
