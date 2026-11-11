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

import QCore.TypedAccessPathPositions

/-!
Typed-position root premises read from the Clang AST
(W47-TYPED-ROOT-DECLARATION-READER, gateway rule 10l). The AST extraction and
the Rust-to-Lean correspondence remain review premises (standing constraint
28). This module models the reader's answers and proves:

* **soundness of the closure**: a name outside a seeded, settled set — the
  set the Rust loop returns — has no binding in the model (`Bound`, the least
  set closed under the extracted binding events), and in particular no event
  of the body binds it;
* **over-approximation**: the served never-bound answer joins the lexical
  closure, so it implies the lexical closure's answer, and where the lexical
  declaration reader read the function the served answer *is* the lexical
  one;
* **rule 5b's premise is kept**: a served pointer root is never named by the
  lexical `may_bind`, nor by the AST's, so an actual at a `Borrowed` position
  is never a pointer root;
* **P3's return theorem is preserved**: a pointer return with a root either
  closure binds refuses, and an admitted return still hands nothing back;
* **declared locals**: exactly one automatic, non-formal, typed declaration;
  a lexical `yes` stands;
* **fail-closed**: a function the AST reader cannot answer serves no root.
-/

namespace QCore.TypedRootDeclarations

open QCore.TypedAccessPathPositions

/-! ## The binding closure -/

/-- One binding-relevant event of a body, as the AST reader extracts it.
Names are numbered. -/
inductive BindingEvent where
  /-- The object root of a `&` operand. -/
  | addressOf (n : Nat)
  /-- The object root of `++`/`--`. -/
  | incDec (n : Nat)
  /-- The object root of a compound assignment. -/
  | compound (n : Nat)
  /-- The object root of a sub-object store `s.f = …`. -/
  | subObjectStore (n : Nat)
  /-- A formal the function's own contract or a custody use protects: it
  holds a tracked identity at entry. -/
  | protectedFormal (n : Nat)
  /-- The roots of an actual at a position its contract does not declare
  `Borrowed` (every position of an uncontracted or indirect callee, and every
  operand of an atomic or `va_arg` builtin). -/
  | custodyActual (roots : List Nat)
  /-- The names of an actual at a declared `Borrowed` position. -/
  | borrowedActual (names : List Nat)
  /-- `dest = value` or an initialiser: `isOpaque` when the value holds a call,
  an address, a statement expression or a name this function does not
  declare; `names` are the declared names it reads. -/
  | assign (dest : Nat) (isOpaque : Bool) (names : List Nat)
  deriving DecidableEq, Repr

/-- The names an event binds outright. -/
def seeds : BindingEvent → List Nat
  | .addressOf n => [n]
  | .incDec n => [n]
  | .compound n => [n]
  | .subObjectStore n => [n]
  | .protectedFormal n => [n]
  | .custodyActual roots => roots
  | _ => []

/-- The model: the least set of names the body may bind to a tracked identity. -/
inductive Bound (body : List BindingEvent) : Nat → Prop where
  | seed {e : BindingEvent} {n : Nat} : e ∈ body → n ∈ seeds e → Bound body n
  | opaqueAssign {d : Nat} {names : List Nat} :
      BindingEvent.assign d true names ∈ body → Bound body d
  | copied {d m : Nat} {o : Bool} {names : List Nat} :
      BindingEvent.assign d o names ∈ body → m ∈ names → Bound body m → Bound body d

/-- Every seed is in `s`. -/
def Seeded (body : List BindingEvent) (s : List Nat) : Prop :=
  ∀ e ∈ body, ∀ n ∈ seeds e, n ∈ s

/-- No assignment would add its destination to `s`: the loop's exit test. -/
def Settled (body : List BindingEvent) (s : List Nat) : Prop :=
  ∀ d o names, BindingEvent.assign d o names ∈ body → (o = true ∨ ∃ m ∈ names, m ∈ s) → d ∈ s

/-- A seeded, settled set contains every binding of the model. -/
theorem bound_within_a_settled_set {body : List BindingEvent} {s : List Nat}
    (hs : Seeded body s) (ht : Settled body s) {n : Nat} (h : Bound body n) : n ∈ s := by
  induction h with
  | seed he hn => exact hs _ he _ hn
  | opaqueAssign he => exact ht _ _ _ he (Or.inl rfl)
  | copied he hm _ ih => exact ht _ _ _ he (Or.inr ⟨_, hm, ih⟩)

/-- The executable checks of the two properties. -/
def seededB (body : List BindingEvent) (s : List Nat) : Bool :=
  body.all fun e => (seeds e).all fun n => decide (n ∈ s)

def settledB (body : List BindingEvent) (s : List Nat) : Bool :=
  body.all fun e =>
    match e with
    | .assign d o names => !(o || names.any fun m => decide (m ∈ s)) || decide (d ∈ s)
    | _ => true

theorem seeded_of_check {body : List BindingEvent} {s : List Nat} (h : seededB body s = true) :
    Seeded body s := by
  intro e he n hn
  simp only [seededB, List.all_eq_true, decide_eq_true_eq] at h
  exact h e he n hn

theorem settled_of_check {body : List BindingEvent} {s : List Nat} (h : settledB body s = true) :
    Settled body s := by
  intro d o names he hc
  simp only [settledB, List.all_eq_true] at h
  have := h _ he
  simp only [Bool.or_eq_true, Bool.not_eq_true', decide_eq_true_eq] at this
  rcases this with hn | hd
  · rcases hc with ho | ⟨m, hm, hms⟩
    · simp [ho] at hn
    · simp only [Bool.or_eq_false_iff, List.any_eq_false, decide_eq_true_eq] at hn
      exact absurd hms (hn.2 m hm)
  · exact hd

/-- **Soundness.** A name the computed (seeded and settled) set leaves out is
never bound in the model. -/
theorem an_unlisted_name_is_never_bound {body : List BindingEvent} {s : List Nat} {n : Nat}
    (hs : seededB body s = true) (ht : settledB body s = true) (hn : n ∉ s) :
    ¬ Bound body n :=
  fun h => hn (bound_within_a_settled_set (seeded_of_check hs) (settled_of_check ht) h)

/-- ... and so no event of the body binds it outright, and it is never the
destination of an opaque assignment or of a copy of a bound name. -/
theorem a_never_bound_root_has_no_binding {body : List BindingEvent} {n : Nat}
    (h : ¬ Bound body n) :
    (∀ e ∈ body, n ∉ seeds e) ∧ (∀ names, BindingEvent.assign n true names ∉ body) ∧
      (∀ o names m, BindingEvent.assign n o names ∈ body → m ∈ names → ¬ Bound body m) :=
  ⟨fun _ he hn => h (.seed he hn), fun _ he => h (.opaqueAssign he),
    fun _ _ _ he hm hb => h (.copied he hm hb)⟩

/-! ## The served answer -/

/-- A reader's never-bound answer for one name. `ast` is `none` when the AST
reader cannot answer the function; `unique` when the name is exactly one
formal or automatic local of it. -/
structure Answers where
  lexicalReadable : Bool
  /-- The lexical answer (`never_bound`), meaningful when readable. -/
  lexicalNeverBound : Bool
  /-- The lexical closure's own answer, computed whether or not readable. -/
  lexicalClosureNeverBound : Bool
  ast : Option Bool
  unique : Bool
  lexicalMayBind : Bool
  astMayBind : Bool
  deriving DecidableEq, Repr

/-- The AST's answer, joined with the lexical closure. -/
def astServed (a : Answers) : Bool :=
  match a.ast with
  | some never => never && a.unique && a.lexicalClosureNeverBound
  | none => false

/-- Rule 10l: the lexical answer where the lexical reader read the function,
otherwise the joined AST answer. -/
def servedNeverBound (a : Answers) : Bool :=
  if a.lexicalReadable then a.lexicalNeverBound else astServed a

/-- Rule 5b's premise for a pointer handed out of the body. -/
def servedPointerRoot (a : Answers) : Bool :=
  if a.lexicalReadable then a.lexicalNeverBound && !a.lexicalMayBind
  else astServed a && !a.lexicalMayBind && !a.astMayBind

theorem the_lexical_answer_stands (a : Answers) (h : a.lexicalReadable = true) :
    servedNeverBound a = a.lexicalNeverBound ∧
      servedPointerRoot a = (a.lexicalNeverBound && !a.lexicalMayBind) := by
  simp [servedNeverBound, servedPointerRoot, h]

/-- Over-approximation: an AST-served never-bound answer is never less
conservative than the lexical closure. -/
theorem the_served_answer_implies_the_lexical_closure (a : Answers)
    (hr : a.lexicalReadable = false) (h : servedNeverBound a = true) :
    a.lexicalClosureNeverBound = true := by
  unfold servedNeverBound astServed at h
  cases ha : a.ast <;> simp_all

theorem the_served_answer_implies_the_ast (a : Answers)
    (hr : a.lexicalReadable = false) (h : servedNeverBound a = true) :
    a.ast = some true ∧ a.unique = true := by
  unfold servedNeverBound astServed at h
  cases ha : a.ast <;> simp_all

/-- Fail-closed: a function the AST reader cannot answer serves no root. -/
theorem an_unanswered_function_serves_no_root (a : Answers)
    (hr : a.lexicalReadable = false) (h : a.ast = none) :
    servedNeverBound a = false ∧ servedPointerRoot a = false := by
  simp [servedNeverBound, servedPointerRoot, astServed, hr, h]

theorem a_name_declared_more_than_once_serves_no_root (a : Answers)
    (hr : a.lexicalReadable = false) (h : a.unique = false) :
    servedNeverBound a = false ∧ servedPointerRoot a = false := by
  simp [servedNeverBound, servedPointerRoot, astServed, hr, h]
  cases a.ast <;> simp

/-- Rule 5b's `may_bind` half is never dropped: a served pointer root is
named by neither `may_bind`. -/
theorem a_pointer_root_is_named_by_no_may_bind (a : Answers)
    (h : servedPointerRoot a = true) :
    a.lexicalMayBind = false ∧ (a.lexicalReadable = false → a.astMayBind = false) := by
  unfold servedPointerRoot at h
  cases hr : a.lexicalReadable <;> simp_all

theorem a_pointer_root_is_never_bound (a : Answers) (h : servedPointerRoot a = true) :
    servedNeverBound a = true := by
  unfold servedPointerRoot servedNeverBound at *
  cases hr : a.lexicalReadable <;> simp_all

/-- The names an event makes `may_bind`: every name written, address-taken
or named in any actual. -/
def mayBindNames : BindingEvent → List Nat
  | .assign d _ _ => [d]
  | .borrowedActual names => names
  | e => seeds e

/-- An actual at a `Borrowed` position binds nothing, but is `may_bind`. -/
theorem a_borrowed_actual_is_may_bind_not_bound (n : Nat) :
    n ∈ mayBindNames (.borrowedActual [n]) ∧ seeds (.borrowedActual [n]) = [] := by
  simp [mayBindNames, seeds]

theorem every_seed_is_may_bind (e : BindingEvent) (n : Nat) (h : n ∈ seeds e) :
    n ∈ mayBindNames e := by
  cases e <;> simp_all [seeds, mayBindNames]

/-! ## P3's return theorem is preserved -/

/-- A P3 candidate whose untracked-root field is the served pointer premise
over every root. -/
def withServedRoots (c : Candidate) (roots : List Answers) : Candidate :=
  { c with rootsUntracked := roots.all servedPointerRoot }

/-- A pointer return with a root the lexical closure binds refuses. -/
theorem a_return_through_a_lexically_bound_root_refuses (c : Candidate)
    (roots : List Answers) (hp : c.position = .returnExpression) (hv : c.pointerValue = true)
    (r : Answers) (hr : r ∈ roots) (hr' : r.lexicalReadable = false)
    (hb : r.lexicalClosureNeverBound = false) :
    admits (withServedRoots c roots) = false := by
  apply a_returned_pointer_from_a_tracked_root_refuses
  · simpa [withServedRoots] using hp
  · simpa [withServedRoots] using hv
  · simp only [withServedRoots, List.all_eq_false]
    refine ⟨r, hr, ?_⟩
    cases h : r.ast <;> simp [servedPointerRoot, astServed, hr', hb, h]

/-- A pointer return with a root the AST closure binds refuses. -/
theorem a_return_through_an_ast_bound_root_refuses (c : Candidate)
    (roots : List Answers) (hp : c.position = .returnExpression) (hv : c.pointerValue = true)
    (r : Answers) (hr : r ∈ roots) (hr' : r.lexicalReadable = false)
    (hb : r.ast = some false) :
    admits (withServedRoots c roots) = false := by
  apply a_returned_pointer_from_a_tracked_root_refuses
  · simpa [withServedRoots] using hp
  · simpa [withServedRoots] using hv
  · simp only [withServedRoots, List.all_eq_false]
    refine ⟨r, hr, ?_⟩
    simp [servedPointerRoot, astServed, hr', hb]

/-- Whatever roots are served, an admitted typed return hands nothing back. -/
theorem a_served_return_hands_nothing_back (s : IndirectChainAccess.State)
    (p : TypedAccessPathStatements.Statement)
    (r : IndirectChainAccess.State × List QCore.IndirectScalarAccess.Diagnostic × Exit)
    (h : lowerReturn s p = some r) : r.2.2 = .handsNothingBack :=
  a_typed_return_hands_nothing_back s p r h

/-! ## Declared locals -/

structure Declaration where
  name : Nat
  formal : Bool
  automatic : Bool
  /-- A proven scalar or a pointer value. -/
  typed : Bool
  deriving DecidableEq, Repr

def astDeclaredLocal (decls : List Declaration) (n : Nat) : Bool :=
  match decls.filter (fun d => d.name == n) with
  | [d] => d.automatic && !d.formal && d.typed
  | _ => false

/-- Rule 10l: a lexical `yes` stands; otherwise the AST's answer. -/
def servedDeclaredLocal (lexical : Bool) (decls : List Declaration) (n : Nat) : Bool :=
  lexical || astDeclaredLocal decls n

theorem a_lexical_declared_local_stands (decls : List Declaration) (n : Nat) :
    servedDeclaredLocal true decls n = true := by
  simp [servedDeclaredLocal]

theorem an_ast_declared_local_is_one_automatic_non_formal_declaration
    (decls : List Declaration) (n : Nat) (h : astDeclaredLocal decls n = true) :
    ∃ d, decls.filter (fun d => d.name == n) = [d] ∧ d.automatic = true ∧
      d.formal = false ∧ d.typed = true := by
  unfold astDeclaredLocal at h
  split at h
  · rename_i d hd
    simp only [Bool.and_eq_true, Bool.not_eq_true'] at h
    exact ⟨d, hd, h.1.1, h.1.2, h.2⟩
  · simp at h

theorem a_shadowed_name_is_not_an_ast_declared_local (d e : Declaration) (n : Nat)
    (hd : d.name = n) (he : e.name = n) : astDeclaredLocal [d, e] n = false := by
  simp [astDeclaredLocal, hd, he]

/-! ## Non-vacuity -/

/-- `b` (name 0) read only; `a` (name 1) written from a call, then copied
into `c` (name 2); `d` (name 3) address-taken; `e` (name 4) at a `Borrowed`
position. -/
def exampleBody : List BindingEvent :=
  [.assign 1 true [], .assign 2 false [1], .addressOf 3, .borrowedActual [4]]

def exampleSet : List Nat := [3, 1, 2]

theorem the_example_set_is_seeded_and_settled :
    seededB exampleBody exampleSet = true ∧ settledB exampleBody exampleSet = true := by
  decide

theorem an_untouched_root_is_never_bound : ¬ Bound exampleBody 0 :=
  an_unlisted_name_is_never_bound the_example_set_is_seeded_and_settled.1
    the_example_set_is_seeded_and_settled.2 (by decide)

theorem a_borrowed_root_is_never_bound : ¬ Bound exampleBody 4 :=
  an_unlisted_name_is_never_bound the_example_set_is_seeded_and_settled.1
    the_example_set_is_seeded_and_settled.2 (by decide)

theorem a_copy_of_a_call_result_is_bound : Bound exampleBody 2 :=
  .copied (m := 1) (o := false) (names := [1]) (by decide) (by decide)
    (.opaqueAssign (d := 1) (names := []) (by decide))

theorem an_address_taken_root_is_bound : Bound exampleBody 3 :=
  .seed (e := .addressOf 3) (by decide) (by decide)

/-- The served answer for an untouched formal in an unreadable function. -/
def untouched : Answers :=
  { lexicalReadable := false, lexicalNeverBound := false, lexicalClosureNeverBound := true,
    ast := some true, unique := true, lexicalMayBind := false, astMayBind := false }

theorem an_untouched_root_is_served : servedPointerRoot untouched = true := by decide

theorem the_same_root_at_a_borrowed_actual_is_not_a_pointer_root :
    servedPointerRoot { untouched with astMayBind := true } = false := by decide

theorem the_same_root_bound_lexically_is_not_served :
    servedNeverBound { untouched with lexicalClosureNeverBound := false } = false := by decide

end QCore.TypedRootDeclarations
