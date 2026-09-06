import QCore.CustodySummary

/-!
Contextual custody inference with exhaustive integer contexts and strictly
lower-rank call dependencies. This extends the existing exit-validation
relation; it does not assume a recursive function's own summary. Typed scalar
recognition, context-specific CFG extraction and complete call inventories are
premises supplied by the C gateway, not proved C parser facts. Well-foundedness
below concerns admitted contextual calls, not termination of arbitrary C loops.
-/
namespace QCore.ContextualCustody
open QCore.CustodySummary

inductive ScalarContext where
  | zero
  | nonzero
  deriving DecidableEq, Repr

structure Key where
  function : Nat
  context : ScalarContext
  deriving DecidableEq, Repr

structure Evidence where
  rank : Nat
  summary : Summary
  analysisSafe : Bool
  exits : List ExitObservation
  dependencies : List Key

abbrev Table := Key → Option Evidence

/-- Every installed context is validated, and every admitted call uses a
previously established, strictly lower-ranked context. -/
def Checked (table : Table) : Prop :=
  ∀ key evidence, table key = some evidence →
    validate evidence.summary evidence.analysisSafe evidence.exits = true ∧
    ∀ dependency ∈ evidence.dependencies,
      ∃ earlier, table dependency = some earlier ∧ earlier.rank < evidence.rank

def Calls (table : Table) (caller callee : Key) : Prop :=
  ∃ evidence, table caller = some evidence ∧ callee ∈ evidence.dependencies

def rankOf (table : Table) (key : Key) : Nat :=
  match table key with
  | some evidence => evidence.rank
  | none => 0

theorem checked_call_uses_validated_lower_context
    {table : Table} (checked : Checked table) {caller callee : Key}
    (call : Calls table caller callee) :
    ∃ evidence, table callee = some evidence ∧
      validate evidence.summary evidence.analysisSafe evidence.exits = true ∧
      evidence.rank < rankOf table caller := by
  obtain ⟨current, lookup, member⟩ := call
  obtain ⟨earlier, found, lower⟩ := (checked caller current lookup).2 callee member
  refine ⟨earlier, found, (checked callee earlier found).1, ?_⟩
  simpa [rankOf, lookup] using lower

theorem checked_call_decreases_rank
    {table : Table} (checked : Checked table) {caller callee : Key}
    (call : Calls table caller callee) :
    rankOf table callee < rankOf table caller := by
  obtain ⟨evidence, found, _, lower⟩ := checked_call_uses_validated_lower_context checked call
  simpa [rankOf, found] using lower

theorem checked_calls_are_well_founded
    {table : Table} (checked : Checked table) :
    WellFounded (fun callee caller => Calls table caller callee) := by
  apply Subrelation.wf (r := (measure (rankOf table)).rel)
  · intro callee caller call
    exact checked_call_decreases_rank checked call
  · exact (measure (rankOf table)).wf

theorem rank_zero_context_has_no_recursive_dependencies
    {table : Table} (checked : Checked table) {key : Key} {evidence : Evidence}
    (found : table key = some evidence) (base : evidence.rank = 0) :
    evidence.dependencies = [] := by
  cases dependencies : evidence.dependencies with
  | nil => rfl
  | cons dependency rest =>
    have member : dependency ∈ evidence.dependencies := by simp [dependencies]
    obtain ⟨_, _, lower⟩ := (checked key evidence found).2 dependency member
    rw [base] at lower
    exact False.elim (Nat.not_lt_zero _ lower)

theorem checked_context_never_assumes_itself
    {table : Table} (checked : Checked table) (key : Key) :
    ¬ Calls table key key := by
  intro call
  exact Nat.lt_irrefl _ (checked_call_decreases_rank checked call)

theorem checked_contexts_cannot_assume_each_other
    {table : Table} (checked : Checked table) {first second : Key}
    (forward : Calls table first second) : ¬ Calls table second first := by
  intro backward
  exact Nat.lt_asymm (checked_call_decreases_rank checked forward)
    (checked_call_decreases_rank checked backward)

/-- Publication requires both exhaustive contexts, with the same custody
contract. A successful base context alone is insufficient. -/
def Published (table : Table) (function : Nat) (summary : Summary) : Prop :=
  Checked table ∧
  ∃ zero nonzero,
    table ⟨function, .zero⟩ = some zero ∧
    table ⟨function, .nonzero⟩ = some nonzero ∧
    zero.summary = summary ∧ nonzero.summary = summary

def classify (value : Int) : ScalarContext := if value = 0 then .zero else .nonzero

theorem integer_contexts_are_exhaustive (value : Int) :
    (value = 0 ∧ classify value = .zero) ∨
    (value ≠ 0 ∧ classify value = .nonzero) := by
  by_cases zero : value = 0
  · exact Or.inl ⟨zero, by simp [classify, zero]⟩
  · exact Or.inr ⟨zero, by simp [classify, zero]⟩

theorem publication_covers_every_integer_context
    {table : Table} {function : Nat} {summary : Summary}
    (published : Published table function summary) (value : Int) :
    ∃ evidence, table ⟨function, classify value⟩ = some evidence ∧
      evidence.summary = summary ∧
      validate summary evidence.analysisSafe evidence.exits = true := by
  obtain ⟨checked, zero, nonzero, foundZero, foundNonzero, sameZero, sameNonzero⟩ := published
  cases context : classify value with
  | zero =>
    refine ⟨zero, foundZero, sameZero, ?_⟩
    rw [← sameZero]
    exact (checked _ _ foundZero).1
  | nonzero =>
    refine ⟨nonzero, foundNonzero, sameNonzero, ?_⟩
    rw [← sameNonzero]
    exact (checked _ _ foundNonzero).1

theorem publication_agrees_with_every_context_exit
    {table : Table} {function : Nat} {summary : Summary}
    (published : Published table function summary) (value : Int) :
    ∃ evidence, table ⟨function, classify value⟩ = some evidence ∧
      evidence.exits.isEmpty = false ∧
      ∀ observation ∈ evidence.exits, exitMatches summary observation = true := by
  obtain ⟨evidence, found, _, valid⟩ := publication_covers_every_integer_context published value
  exact ⟨evidence, found,
    validated_summary_has_a_reachable_exit _ _ _ valid,
    validated_summary_matches_every_exit _ _ _ valid⟩

theorem missing_nonzero_context_prevents_publication
    {table : Table} {function : Nat} (summary : Summary)
    (missing : table ⟨function, .nonzero⟩ = none) :
    ¬ Published table function summary := by
  rintro ⟨_, _, _, _, found, _, _⟩
  rw [missing] at found
  contradiction

end QCore.ContextualCustody
