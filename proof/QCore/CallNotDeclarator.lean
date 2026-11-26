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
CALL-WITH-DEREF-ACTUAL-READ-AS-DECLARATION (gateway rule 89). The classifier's
declaration-likeness test is a disjunction of four token tests: a keyword
specifier first, an `IDENT IDENT` prefix, an `IDENT *` prefix, and an
`IDENT ( *` prefix (a function-pointer declarator `T (*NAME)`). The shadow
guard refuses a declaration-like statement that names a formal outside an
initializer. The rule drops only the fourth disjunct, and only when the
statement's Clang node is a `CallExpr`: a call expression declares nothing.
Faithful Clang extraction of the node kind and the Rust/model correspondence
remain premises (trust boundary 28).
-/
namespace QCore.CallNotDeclarator

/-- The statement's Clang node, as far as the rule asks. -/
inductive Reading where
  | unmatched
  | declaration
  | call
  | other
  deriving DecidableEq, Repr

def Reading.isCall : Reading → Bool
  | .call => true
  | _ => false

/-- The `IDENT ( *` disjunct with the rule. -/
def declaratorShaped (lexical : Bool) (r : Reading) : Bool :=
  lexical && !r.isCall

/-- The declaration-likeness test before the rule. -/
def oldDeclarationLike (specifier pair star parenStar : Bool) : Bool :=
  specifier || pair || star || parenStar

/-- The declaration-likeness test with the rule. -/
def declarationLike (specifier pair star parenStar : Bool) (r : Reading) : Bool :=
  specifier || pair || star || declaratorShaped parenStar r

/-- The shadow guard: a declaration-like statement naming a formal outside an
    initializer is refused. -/
def shadowRefused (declLike namesFormal : Bool) : Bool :=
  declLike && namesFormal

def bools : List Bool := [false, true]

def readings : List Reading := [.unmatched, .declaration, .call, .other]

def evaluatedVectors : List Bool :=
  bools.flatMap fun s => bools.flatMap fun p => bools.flatMap fun st =>
  bools.flatMap fun ps => readings.map fun r => declarationLike s p st ps r

def vectors : List Bool := [false, false, false, false, true, true, false, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true, true]

theorem vectors_match : evaluatedVectors = vectors := by decide

/-- The rule only ever narrows the test. -/
theorem never_adds_a_declaration (s p st ps : Bool) (r : Reading)
    (h : declarationLike s p st ps r = true) : oldDeclarationLike s p st ps = true := by
  cases s <;> cases p <;> cases st <;> cases ps <;> cases r <;>
    simp_all [declarationLike, oldDeclarationLike, declaratorShaped, Reading.isCall]

/-- A statement the guard refused before and admits now is a call. -/
theorem an_admission_is_a_call (s p st ps namesFormal : Bool) (r : Reading)
    (old : shadowRefused (oldDeclarationLike s p st ps) namesFormal = true)
    (new : shadowRefused (declarationLike s p st ps r) namesFormal = false) :
    r = .call := by
  cases s <;> cases p <;> cases st <;> cases ps <;> cases namesFormal <;> cases r <;>
    simp_all [shadowRefused, declarationLike, oldDeclarationLike, declaratorShaped,
      Reading.isCall]

/-- ... and only through the `IDENT ( *` disjunct: the other three are
    untouched. -/
theorem an_admission_is_only_the_paren_star_disjunct (s p st ps namesFormal : Bool)
    (r : Reading)
    (old : shadowRefused (oldDeclarationLike s p st ps) namesFormal = true)
    (new : shadowRefused (declarationLike s p st ps r) namesFormal = false) :
    s = false ∧ p = false ∧ st = false ∧ ps = true := by
  cases s <;> cases p <;> cases st <;> cases ps <;> cases namesFormal <;> cases r <;>
    simp_all [shadowRefused, declarationLike, oldDeclarationLike, declaratorShaped,
      Reading.isCall]

/-- Every statement Clang reads as a declaration keeps the old answer, so every
    real declarator (`T (*out);`, `T (*out)[4];`, `void (*out)(void);`) stays
    refused exactly when it was. -/
theorem a_declaration_keeps_the_old_test (s p st ps : Bool) :
    declarationLike s p st ps .declaration = oldDeclarationLike s p st ps := by
  cases s <;> cases p <;> cases st <;> cases ps <;> rfl

theorem a_declaration_keeps_the_refusal (s p st ps namesFormal : Bool) :
    shadowRefused (declarationLike s p st ps .declaration) namesFormal =
      shadowRefused (oldDeclarationLike s p st ps) namesFormal := by
  rw [a_declaration_keeps_the_old_test]

/-- No unique node, or any node but a call, keeps the old answer (fail-closed). -/
theorem a_non_call_keeps_the_old_test (s p st ps : Bool) (r : Reading) (h : r ≠ .call) :
    declarationLike s p st ps r = oldDeclarationLike s p st ps := by
  cases r <;> first | exact absurd rfl h |
    (cases s <;> cases p <;> cases st <;> cases ps <;> rfl)

theorem no_ast_answer_keeps_the_old_test (s p st ps : Bool) :
    declarationLike s p st ps .unmatched = oldDeclarationLike s p st ps :=
  a_non_call_keeps_the_old_test s p st ps .unmatched (by decide)

/-- A keyword-specified declarator (`int (*out);`, `void (*out)(void);`) is
    declaration-like whatever Clang answers. -/
theorem a_keyword_declarator_stays (p st ps : Bool) (r : Reading) :
    declarationLike true p st ps r = true := by
  simp [declarationLike]

end QCore.CallNotDeclarator
