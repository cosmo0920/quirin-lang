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

/-!
MEMBER-ACCESS-PROJECTION (gateway rule 38h). The lexical indirect-access gate
admits a declaration with no initializer only for a keyword specifier and
literal array bounds; the same declaration spelled with a typedef specifier or
a constant-expression bound reads as an access. The rule turns that refusal
into the keyword branch's admission only when Clang's AST shows exactly one
declaration statement of automatic, uninitialized, constant-typed objects.
Faithful Clang extraction and the Rust/model correspondence remain premises
(trust boundary 28).
-/
namespace QCore.TypedDeclarationReading

/-- What the AST says about the statement, premise by premise. -/
structure Facts where
  declarationStatement : Bool
  onlyObjects : Bool
  noneInitialized : Bool
  automatic : Bool
  constantTypes : Bool
  deriving DecidableEq, Repr

def Facts.all (f : Facts) : Bool :=
  f.declarationStatement && f.onlyObjects && f.noneInitialized && f.automatic && f.constantTypes

/-- The gate's refusal with the rule: the old refusal, the gate's own
    declaration-prefix test, and the AST's answer (none when no unique node
    matched). -/
def refuses (old declarationLike : Bool) (facts : Option Facts) : Bool :=
  old && !(declarationLike && (match facts with
    | some f => f.all
    | none => false))

def bools : List Bool := [false, true]

def everyFacts : List Facts :=
  bools.flatMap fun a => bools.flatMap fun b => bools.flatMap fun c =>
  bools.flatMap fun d => bools.map fun e => ⟨a, b, c, d, e⟩

def evaluatedVectors : List Bool :=
  bools.flatMap fun old => bools.flatMap fun dl =>
    refuses old dl none :: everyFacts.map fun f => refuses old dl (some f)

def vectors : List Bool := [false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, false]

theorem vectors_match : evaluatedVectors = vectors := by decide

/-- Every statement the old gate admitted keeps its admission. -/
theorem an_old_admission_stays (dl : Bool) (facts : Option Facts) :
    refuses false dl facts = false := by
  simp [refuses]

/-- With no unique AST node the refusal is the old one, byte for byte. -/
theorem no_ast_answer_keeps_the_refusal (old dl : Bool) :
    refuses old dl none = old := by
  cases old <;> cases dl <;> rfl

/-- A statement the gate's declaration-prefix test rejects keeps its answer. -/
theorem a_non_declaration_keeps_its_answer (old : Bool) (facts : Option Facts) :
    refuses old false facts = old := by
  cases old <;> simp [refuses]

/-- A refusal becomes an admission only on every fact. -/
theorem a_new_admission_needs_every_fact (dl : Bool) (facts : Option Facts)
    (h : refuses true dl facts = false) :
    dl = true ∧ ∃ f, facts = some f ∧ f.declarationStatement = true ∧
      f.onlyObjects = true ∧ f.noneInitialized = true ∧ f.automatic = true ∧
      f.constantTypes = true := by
  cases dl with
  | false => simp [refuses] at h
  | true =>
    cases facts with
    | none => simp [refuses] at h
    | some f =>
      simp only [refuses, Facts.all, Bool.true_and, Bool.not_eq_eq_eq_not, Bool.not_false,
        Bool.and_eq_true] at h
      exact ⟨rfl, f, rfl, h.1.1.1.1, h.1.1.1.2, h.1.1.2, h.1.2, h.2⟩

theorem an_initialized_declaration_keeps_the_refusal (old dl : Bool) (f : Facts)
    (h : f.noneInitialized = false) : refuses old dl (some f) = old := by
  cases old <;> simp [refuses, Facts.all, h]

theorem a_variably_modified_declaration_keeps_the_refusal (old dl : Bool) (f : Facts)
    (h : f.constantTypes = false) : refuses old dl (some f) = old := by
  cases old <;> simp [refuses, Facts.all, h]

theorem a_non_automatic_declaration_keeps_the_refusal (old dl : Bool) (f : Facts)
    (h : f.automatic = false) : refuses old dl (some f) = old := by
  cases old <;> simp [refuses, Facts.all, h]

theorem a_non_object_declaration_keeps_the_refusal (old dl : Bool) (f : Facts)
    (h : f.onlyObjects = false) : refuses old dl (some f) = old := by
  cases old <;> simp [refuses, Facts.all, h]

/-- The admitted statement is lowered as the keyword branch lowers it: `NoOp`,
    which changes no binding, no other state and no diagnostic. -/
structure State (ι ρ ε : Type) where
  bindings : String → Option ι
  remainder : ρ
  diagnostics : List ε

def noOp {ι ρ ε : Type} (s : State ι ρ ε) : State ι ρ ε := s

theorem the_lowering_changes_no_binding {ι ρ ε : Type} (s : State ι ρ ε) (name : String) :
    (noOp s).bindings name = s.bindings name := rfl

theorem the_lowering_keeps_every_diagnostic {ι ρ ε : Type} (s : State ι ρ ε) :
    (noOp s).diagnostics = s.diagnostics := rfl

theorem the_lowering_keeps_all_other_state {ι ρ ε : Type} (s : State ι ρ ε) :
    (noOp s).remainder = s.remainder := rfl

end QCore.TypedDeclarationReading
