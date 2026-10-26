import QCore.HypothesisGraph

/-! First-order meaning of decomposed `quant proposition` declarations.
Identifiers, sorts and relations are symbolic names. A relation interpretation
and a value valuation are independent inputs: neither declaration nor theorem
application proves that these inputs describe C storage, executions or evidence.
The single semantic carrier below abstracts the Rust named sorts; `WellFormed`
checks the separate syntactic sort discipline. No C extraction is claimed. -/
namespace QCore.QuantProposition

inductive ParameterSort where
  | value (sort : String)
  | predicate (arguments : List String)
  deriving DecidableEq, Repr

inductive Expr where
  | predicate (name : String) (arguments : List String)
  | equal (left right : String)
  | not (body : Expr)
  | and (left right : Expr)
  | or (left right : Expr)
  | implies (antecedent consequent : Expr)
  | forall_ (variable sort : String) (body : Expr)
  | exists_ (variable sort : String) (body : Expr)
  deriving Repr

abbrev Signature := String → Option ParameterSort
abbrev Scope := String → Option String

def bind (valuation : String → α) (variable : String) (value : α) : String → α :=
  fun name => if name = variable then value else valuation name

def bindSort (scope : Scope) (variable sort : String) : Scope :=
  fun name => if name = variable then some sort else scope name

def argumentsWellTyped (scope : Scope) : List String → List String → Prop
  | [], [] => True
  | argument :: arguments, sort :: sorts =>
      scope argument = some sort ∧ argumentsWellTyped scope arguments sorts
  | _, _ => False

/-- Sort checks concern syntax, not the truth of a relation or a source fact. -/
inductive WellFormed (signature : Signature) : Scope → Expr → Prop where
  | predicate {scope name arguments sorts}
      (declared : signature name = some (.predicate sorts))
      (typed : argumentsWellTyped scope arguments sorts) :
      WellFormed signature scope (.predicate name arguments)
  | equal {scope left right sort}
      (leftTyped : scope left = some sort) (rightTyped : scope right = some sort) :
      WellFormed signature scope (.equal left right)
  | not {scope body} (typed : WellFormed signature scope body) :
      WellFormed signature scope (.not body)
  | and {scope left right}
      (leftTyped : WellFormed signature scope left)
      (rightTyped : WellFormed signature scope right) :
      WellFormed signature scope (.and left right)
  | or {scope left right}
      (leftTyped : WellFormed signature scope left)
      (rightTyped : WellFormed signature scope right) :
      WellFormed signature scope (.or left right)
  | implies {scope left right}
      (leftTyped : WellFormed signature scope left)
      (rightTyped : WellFormed signature scope right) :
      WellFormed signature scope (.implies left right)
  | forall_ {scope variable sort body}
      (typed : WellFormed signature (bindSort scope variable sort) body) :
      WellFormed signature scope (.forall_ variable sort body)
  | exists_ {scope variable sort body}
      (typed : WellFormed signature (bindSort scope variable sort) body) :
      WellFormed signature scope (.exists_ variable sort body)

structure Interpretation (Value : Type u) where
  sortHolds : String → Value → Prop
  relation : String → List Value → Prop

/-- Quantifiers range over the explicitly interpreted named sort. -/
def holds (meaning : Interpretation Value) (valuation : String → Value) : Expr → Prop
  | .predicate name arguments => meaning.relation name (arguments.map valuation)
  | .equal left right => valuation left = valuation right
  | .not body => ¬ holds meaning valuation body
  | .and left right => holds meaning valuation left ∧ holds meaning valuation right
  | .or left right => holds meaning valuation left ∨ holds meaning valuation right
  | .implies left right => holds meaning valuation left → holds meaning valuation right
  | .forall_ variable sort body =>
      ∀ value, meaning.sortHolds sort value → holds meaning (bind valuation variable value) body
  | .exists_ variable sort body =>
      ∃ value, meaning.sortHolds sort value ∧ holds meaning (bind valuation variable value) body

/-- Decomposition keeps the same variable, sort and valuation in both pieces. -/
theorem forall_conjunction_decomposition
    (meaning : Interpretation Value) (valuation : String → Value)
    (variable sort : String) (left right : Expr) :
    holds meaning valuation (.forall_ variable sort (.and left right)) ↔
      holds meaning valuation (.and (.forall_ variable sort left) (.forall_ variable sort right)) := by
  constructor
  · intro all
    exact ⟨fun value typed => (all value typed).1,
      fun value typed => (all value typed).2⟩
  · rintro ⟨leftAll, rightAll⟩ value typed
    exact ⟨leftAll value typed, rightAll value typed⟩

/-- A guarded alternative survives decomposition; alternatives are not joined. -/
theorem guarded_alternative_decomposition
    (meaning : Interpretation Value) (valuation : String → Value)
    (variable sort : String) (observed nested scalar declaration effects : Expr) :
    holds meaning valuation (.forall_ variable sort
      (.implies observed (.and (.or nested scalar) (.and declaration effects)))) ↔
    holds meaning valuation (.and
      (.forall_ variable sort (.implies observed (.or nested scalar)))
      (.and (.forall_ variable sort (.implies observed declaration))
        (.forall_ variable sort (.implies observed effects)))) := by
  constructor
  · intro all
    exact ⟨fun value typed seen => (all value typed seen).1,
      fun value typed seen => (all value typed seen).2.1,
      fun value typed seen => (all value typed seen).2.2⟩
  · rintro ⟨route, declarationAll, effectsAll⟩ value typed seen
    exact ⟨route value typed seen, declarationAll value typed seen,
      effectsAll value typed seen⟩

/-- The scalar route suffices even when no nested route has been established. -/
theorem scalar_alternative_is_sufficient
    (meaning : Interpretation Value) (valuation : String → Value)
    (nested scalar declaration effects : Expr)
    (scalarHolds : holds meaning valuation scalar)
    (declarationHolds : holds meaning valuation declaration)
    (effectsHold : holds meaning valuation effects) :
    holds meaning valuation (.and (.or nested scalar) (.and declaration effects)) := by
  exact ⟨Or.inr scalarHolds, declarationHolds, effectsHold⟩

/-- An implication is not a conjunction and does not assert its antecedent. -/
theorem implication_does_not_establish_antecedent
    (meaning : Interpretation Value) (valuation : String → Value) (left right : Expr)
    (absent : ¬ holds meaning valuation left) :
    holds meaning valuation (.implies left right) ∧
      ¬ holds meaning valuation (.and left right) := by
  exact ⟨fun antecedent => False.elim (absent antecedent), fun conjunction => absent conjunction.1⟩

/-- Evidence compatibility does not decide the truth of an unobserved relation. -/
def evidenceCompatible : HypothesisGraph.Disposition → Prop → Prop
  | .open, _ => True
  | .supported, proposition => proposition
  | .refuted, proposition => ¬ proposition

theorem missing_evidence_allows_both_truth_values :
    evidenceCompatible .open True ∧ evidenceCompatible .open False := by
  exact ⟨True.intro, True.intro⟩

theorem missing_evidence_is_not_refutation (proposition : Prop) :
    evidenceCompatible .open proposition ∧
      HypothesisGraph.Disposition.open ≠ HypothesisGraph.Disposition.refuted := by
  exact ⟨True.intro, by intro impossible; cases impossible⟩

end QCore.QuantProposition
