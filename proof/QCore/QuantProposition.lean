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

import QCore.HypothesisGraph

/-! Typed first-order mathematical terms and decomposed propositions.
Functions, relations, opaque carriers and valuations are interpreted inputs.
Integers use Lean `Int`, with no overflow or C conversion semantics. `WellFormed`
is the separate syntactic sort discipline; the interpretation must independently
respect it. Neither this model nor its conditional theorems establish that an
interpretation describes C storage, executions, source extraction or evidence. -/
namespace QCore.QuantProposition

inductive ParameterSort where
  | value (sort : String)
  | predicate (arguments : List String)
  | function (arguments : List String) (result : String)
  deriving DecidableEq, Repr

inductive Term where
  | var (name : String)
  | integer (value : Int)
  | apply (name : String) (arguments : List Term)
  | add (left right : Term)
  | sub (left right : Term)
  | mul (left right : Term)
  | neg (term : Term)
  deriving Repr

inductive Comparison where
  | less | lessEqual | greater | greaterEqual
  deriving Repr

inductive Expr where
  | predicate (name : String) (arguments : List Term)
  | equal (left right : Term)
  | compare (operator : Comparison) (left right : Term)
  | not (body : Expr)
  | and (left right : Expr)
  | or (left right : Expr)
  | implies (antecedent consequent : Expr)
  | forall_ (boundName sort : String) (body : Expr)
  | exists_ (boundName sort : String) (body : Expr)
  deriving Repr

abbrev Signature := String → Option ParameterSort
abbrev Scope := String → Option String

def bind (valuation : String → α) (boundName : String) (value : α) : String → α :=
  fun name => if name = boundName then value else valuation name

def bindSort (scope : Scope) (boundName sort : String) : Scope :=
  fun name => if name = boundName then some sort else scope name

/-- Pure typing: a declared function is not evidence of a source field value. -/
inductive TermTyped (signature : Signature) (scope : Scope) : Term → String → Prop where
  | var {name sort} (typed : scope name = some sort) :
      TermTyped signature scope (.var name) sort
  | integer (value : Int) : TermTyped signature scope (.integer value) "integer"
  | apply {name arguments sorts result}
      (declared : signature name = some (.function sorts result))
      (arity : arguments.length = sorts.length)
      (typed : ∀ term sort, (term, sort) ∈ arguments.zip sorts →
        TermTyped signature scope term sort) :
      TermTyped signature scope (.apply name arguments) result
  | add {left right} (l : TermTyped signature scope left "integer")
      (r : TermTyped signature scope right "integer") :
      TermTyped signature scope (.add left right) "integer"
  | sub {left right} (l : TermTyped signature scope left "integer")
      (r : TermTyped signature scope right "integer") :
      TermTyped signature scope (.sub left right) "integer"
  | mul {left right} (l : TermTyped signature scope left "integer")
      (r : TermTyped signature scope right "integer") :
      TermTyped signature scope (.mul left right) "integer"
  | neg {term} (typed : TermTyped signature scope term "integer") :
      TermTyped signature scope (.neg term) "integer"

/-- Sort checks concern syntax, not the truth of a relation or a source fact. -/
inductive WellFormed (signature : Signature) : Scope → Expr → Prop where
  | predicate {scope name arguments sorts}
      (declared : signature name = some (.predicate sorts))
      (arity : arguments.length = sorts.length)
      (typed : ∀ term sort, (term, sort) ∈ arguments.zip sorts →
        TermTyped signature scope term sort) :
      WellFormed signature scope (.predicate name arguments)
  | equal {scope left right sort}
      (leftTyped : TermTyped signature scope left sort)
      (rightTyped : TermTyped signature scope right sort) :
      WellFormed signature scope (.equal left right)
  | compare {scope operator left right}
      (leftTyped : TermTyped signature scope left "integer")
      (rightTyped : TermTyped signature scope right "integer") :
      WellFormed signature scope (.compare operator left right)
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
  | forall_ {scope boundName sort body}
      (typed : WellFormed signature (bindSort scope boundName sort) body) :
      WellFormed signature scope (.forall_ boundName sort body)
  | exists_ {scope boundName sort body}
      (typed : WellFormed signature (bindSort scope boundName sort) body) :
      WellFormed signature scope (.exists_ boundName sort body)

inductive SemanticValue (Opaque : Type u) where
  | integer (value : Int)
  | opaque (value : Opaque)

structure Interpretation (Opaque : Type u) where
  sortHolds : String → SemanticValue Opaque → Prop
  relation : String → List (SemanticValue Opaque) → Prop
  function : String → List (SemanticValue Opaque) → Option (SemanticValue Opaque)

def integerBinary (operation : Int → Int → Int) :
    SemanticValue Opaque → SemanticValue Opaque → Option (SemanticValue Opaque)
  | .integer left, .integer right => some (.integer (operation left right))
  | _, _ => none

/-- A partial interpretation refuses absent functions and noninteger arithmetic. -/
def evaluate (meaning : Interpretation Opaque) (valuation : String → SemanticValue Opaque) :
    Term → Option (SemanticValue Opaque)
  | .var name => some (valuation name)
  | .integer value => some (.integer value)
  | .apply name arguments => do
      let values ← arguments.mapM (evaluate meaning valuation)
      meaning.function name values
  | .add left right => do
      integerBinary (· + ·) (← evaluate meaning valuation left) (← evaluate meaning valuation right)
  | .sub left right => do
      integerBinary (· - ·) (← evaluate meaning valuation left) (← evaluate meaning valuation right)
  | .mul left right => do
      integerBinary (· * ·) (← evaluate meaning valuation left) (← evaluate meaning valuation right)
  | .neg term => do
      let .integer value ← evaluate meaning valuation term | none
      some (.integer (-value))

def ordered (operator : Comparison) : SemanticValue Opaque → SemanticValue Opaque → Prop
  | .integer left, .integer right => match operator with
      | .less => left < right
      | .lessEqual => left ≤ right
      | .greater => left > right
      | .greaterEqual => left ≥ right
  | _, _ => False

/-- Integer quantifiers range over mathematical integers, independent of opaque sorts. -/
def sortMember (meaning : Interpretation Opaque) (sort : String) (value : SemanticValue Opaque) : Prop :=
  if sort = "integer" then ∃ integer, value = .integer integer else meaning.sortHolds sort value

def holds (meaning : Interpretation Opaque) (valuation : String → SemanticValue Opaque) : Expr → Prop
  | .predicate name arguments =>
      ∃ values, arguments.mapM (evaluate meaning valuation) = some values ∧ meaning.relation name values
  | .equal left right =>
      ∃ value, evaluate meaning valuation left = some value ∧ evaluate meaning valuation right = some value
  | .compare operator left right =>
      ∃ l r, evaluate meaning valuation left = some l ∧ evaluate meaning valuation right = some r ∧ ordered operator l r
  | .not body => ¬ holds meaning valuation body
  | .and left right => holds meaning valuation left ∧ holds meaning valuation right
  | .or left right => holds meaning valuation left ∨ holds meaning valuation right
  | .implies left right => holds meaning valuation left → holds meaning valuation right
  | .forall_ boundName sort body =>
      ∀ value, sortMember meaning sort value → holds meaning (bind valuation boundName value) body
  | .exists_ boundName sort body =>
      ∃ value, sortMember meaning sort value ∧ holds meaning (bind valuation boundName value) body

/-- Decomposition keeps the same boundName, sort and valuation in both pieces. -/
theorem forall_conjunction_decomposition
    (meaning : Interpretation Value) (valuation : String → SemanticValue Value)
    (boundName sort : String) (left right : Expr) :
    holds meaning valuation (.forall_ boundName sort (.and left right)) ↔
      holds meaning valuation (.and (.forall_ boundName sort left) (.forall_ boundName sort right)) := by
  constructor
  · intro all
    exact ⟨fun value typed => (all value typed).1,
      fun value typed => (all value typed).2⟩
  · rintro ⟨leftAll, rightAll⟩ value typed
    exact ⟨leftAll value typed, rightAll value typed⟩

/-- A guarded alternative survives decomposition; alternatives are not joined. -/
theorem guarded_alternative_decomposition
    (meaning : Interpretation Value) (valuation : String → SemanticValue Value)
    (boundName sort : String) (observed nested scalar declaration effects : Expr) :
    holds meaning valuation (.forall_ boundName sort
      (.implies observed (.and (.or nested scalar) (.and declaration effects)))) ↔
    holds meaning valuation (.and
      (.forall_ boundName sort (.implies observed (.or nested scalar)))
      (.and (.forall_ boundName sort (.implies observed declaration))
        (.forall_ boundName sort (.implies observed effects)))) := by
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
    (meaning : Interpretation Value) (valuation : String → SemanticValue Value)
    (nested scalar declaration effects : Expr)
    (scalarHolds : holds meaning valuation scalar)
    (declarationHolds : holds meaning valuation declaration)
    (effectsHold : holds meaning valuation effects) :
    holds meaning valuation (.and (.or nested scalar) (.and declaration effects)) := by
  exact ⟨Or.inr scalarHolds, declarationHolds, effectsHold⟩

/-- An implication is not a conjunction and does not assert its antecedent. -/
theorem implication_does_not_establish_antecedent
    (meaning : Interpretation Value) (valuation : String → SemanticValue Value) (left right : Expr)
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

/-- Addition is mathematical integer addition, with no machine truncation. -/
theorem integer_addition_semantics
    (meaning : Interpretation Value) (valuation : String → SemanticValue Value)
    (left right : Term) (a b : Int)
    (leftValue : evaluate meaning valuation left = some (.integer a))
    (rightValue : evaluate meaning valuation right = some (.integer b)) :
    evaluate meaning valuation (.add left right) = some (.integer (a + b)) := by
  simp [evaluate, leftValue, rightValue, integerBinary]

theorem noninteger_arithmetic_refuses
    (meaning : Interpretation Value) (valuation : String → SemanticValue Value)
    (left right : Term) (value : Value)
    (leftValue : evaluate meaning valuation left = some (.opaque value)) :
    evaluate meaning valuation (.add left right) = none := by
  simp [evaluate, leftValue]
  cases evaluate meaning valuation right <;> simp [integerBinary]

theorem arithmetic_result_has_integer_sort
    (signature : Signature) (scope : Scope) (left right : Term) (sort : String)
    (typed : TermTyped signature scope (.add left right) sort) : sort = "integer" := by
  cases typed
  rfl

end QCore.QuantProposition
