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

import QCore.QuantProposition
import QCore.PremiseFormula.Librdkafka

/-!
# The cJSON detach hypotheses as typed atoms (card SP6-SYMBOLIC-HYPOTHESIS-ATOMS)

Contract specification C04.18.7 (typed atoms) and C04.18.9 (the SP6 block).
The nine `detach_*` atoms of `cJSON_DetachItemViaPointer` were English
sentences (`resource_hypothesis(detach_*, "...")`); each is now a typed atom
`quant proposition #detach_*(...) = FORMULA;` whose parameters are the
registered C04.18.9 terms its part of the declaration's premise formula uses,
in their **typed rendering** (`render`):

* an element-sorted term (`#parent`, sort `obj`) is a value parameter;
* a set-valued term `#t(x₁…xₙ)` of element sort `S` is a predicate parameter
  `#t : predicate<X₁…Xₙ, S>`, `#t(x₁…xₙ, y)` meaning `y in #t(x₁…xₙ)`;
* a `prop` term is a predicate over its arguments (`#nodup : predicate<obj>`);
* an `eff` term is a predicate over its arguments and one run
  (`#keeps_links : predicate<obj, run>`);
* `F |-> E` is `forall (#f: fn) { (F(#f) => forall (#r: run) { (#runs(#f,#r)
  => E(#r)) }) }`, where `#runs` (the SP6 block, with the value sort `run`)
  is the maps-to operator's own semantics (`Model.runs`).

`render M` interprets each parameter as the same component of the C04.18.9
model `M` its term denotes, so a typed atom means what the premise formula's
part means and nothing more.

* Each atom's formula below is the Rust reader's rendering of the atom as
  `cjson.qv` states it (`crates/qcore-contract/tests/cjson_typed_atoms.rs`,
  `the_nine_atoms_are_the_lean_definitions`).
* **Faithfulness**: in the rendering of the model a
  `QCore.CjsonRepresentationPremises` environment induces (`detachModel`),
  each atom holds exactly when its `CjsonRepresentationPremises` definition
  does (`chain_atom_iff` … `predecessor_atom_iff`, collected in
  `typed_atoms_are_the_definitions`).
* The declaration's premise formula still holds exactly when the nine atoms
  hold (`detach_formula_is_the_nine_typed_atoms`, from SP2's
  `detach_formula_is_the_nine_atoms`).
* A string is not a hypothesis atom: `accept` refuses `.str` for a declared
  atom and admits an opaque statement only for a generated legacy atom
  (`string_is_not_a_hypothesis`, `hypothesisVectors`, bound to the Rust
  `HYPOTHESIS_DECISION_VECTORS`).

Nothing here establishes that an environment describes cJSON: the atoms stay
undischarged evidence-only metadata.
-/

namespace QCore.CjsonTypedAtoms

open QCore.QuantProposition
open QCore.PremiseFormula (Model detachModel detachFormula eval)

/-! ## The typed rendering of a C04.18.9 model -/

/-- The values a typed atom quantifies over: the model's objects, its
callbacks (the same carrier, C04.18.9 `fn`) and the runs of a callback. -/
inductive Carrier (M : Model) where
  | obj (u : M.U)
  | fn (u : M.U)
  | run (r : M.Run)

/-- The value sorts: `obj`, `fn` (C04.18.9) and `run` (the SP6 block). -/
def sortHolds (M : Model) (s : String) (v : SemanticValue (Carrier M)) : Prop :=
  (s = "obj" ∧ ∃ u, v = .opaque (.obj u)) ∨
  (s = "fn" ∧ ∃ u, v = .opaque (.fn u)) ∨
  (s = "run" ∧ ∃ r, v = .opaque (.run r))

/-- Each predicate parameter is its C04.18.9 term's model component. -/
def relation (M : Model) (name : String) : List (SemanticValue (Carrier M)) → Prop
  | [.opaque (.obj x)] =>
      (name = "live" ∧ M.live x) ∨ (name = "nodup" ∧ M.nodup x)
  | [.opaque (.obj o), .opaque (.obj x)] =>
      (name = "owned" ∧ M.owned o x) ∨ (name = "children" ∧ M.children o x) ∨
      (name = "child" ∧ M.child o x) ∨ (name = "first" ∧ M.first o x) ∨
      (name = "last" ∧ M.last o x) ∨ (name = "next" ∧ M.next o x) ∨
      (name = "prev" ∧ M.prev o x) ∨ (name = "next_reach" ∧ M.nextReach o x)
  | [.opaque (.obj o), .opaque (.obj b), .opaque (.obj a)] =>
      name = "pred" ∧ M.pred o b a
  | [.opaque (.fn f)] => name = "concurrent" ∧ M.concurrent f
  | [.opaque (.fn f), .opaque (.run r)] => name = "runs" ∧ M.runs f r
  | [.opaque (.obj o), .opaque (.run r)] => name = "keeps_links" ∧ M.keepsLinks o r
  | _ => False

/-- The typed rendering of `M`: no function parameter occurs. -/
def render (M : Model) : Interpretation (Carrier M) where
  sortHolds := sortHolds M
  relation := relation M
  function := fun _ _ => none

/-- The one value parameter, `#parent`. -/
def valuation (M : Model) : String → SemanticValue (Carrier M) :=
  fun name => if name = "parent" then .opaque (.obj M.parent) else .integer 0

/-! ## The nine atoms (the Rust reader's `lean` rendering of `cjson.qv`) -/

def chainAtom : Expr := (.and (.and (.and (.and (.and (.and (.predicate "live" [(.var "parent")]) (.forall_ "c" "obj" (.implies (.predicate "owned" [(.var "parent"), (.var "c")]) (.predicate "children" [(.var "parent"), (.var "c")])))) (.forall_ "c" "obj" (.implies (.predicate "children" [(.var "parent"), (.var "c")]) (.and (.predicate "owned" [(.var "parent"), (.var "c")]) (.predicate "live" [(.var "c")]))))) (.forall_ "c" "obj" (.implies (.predicate "child" [(.var "parent"), (.var "c")]) (.predicate "first" [(.var "parent"), (.var "c")])))) (.forall_ "c" "obj" (.implies (.predicate "first" [(.var "parent"), (.var "c")]) (.predicate "child" [(.var "parent"), (.var "c")])))) (.forall_ "b" "obj" (.implies (.predicate "children" [(.var "parent"), (.var "b")]) (.forall_ "a" "obj" (.implies (.predicate "pred" [(.var "parent"), (.var "b"), (.var "a")]) (.predicate "next" [(.var "a"), (.var "b")])))))) (.forall_ "c" "obj" (.implies (.predicate "children" [(.var "parent"), (.var "c")]) (.not (.predicate "next_reach" [(.var "c"), (.var "c")])))))

def distinctAtom : Expr := (.predicate "nodup" [(.var "parent")])

def emptyAtom : Expr := (.and (.implies (.forall_ "c" "obj" (.not (.predicate "child" [(.var "parent"), (.var "c")]))) (.forall_ "c" "obj" (.not (.predicate "children" [(.var "parent"), (.var "c")])))) (.implies (.forall_ "c" "obj" (.not (.predicate "children" [(.var "parent"), (.var "c")]))) (.forall_ "c" "obj" (.not (.predicate "child" [(.var "parent"), (.var "c")])))))

def firstPrevAtom : Expr := (.forall_ "f" "obj" (.implies (.predicate "first" [(.var "parent"), (.var "f")]) (.forall_ "l" "obj" (.implies (.predicate "last" [(.var "parent"), (.var "l")]) (.predicate "prev" [(.var "f"), (.var "l")])))))

def lastNextAtom : Expr := (.forall_ "l" "obj" (.implies (.predicate "last" [(.var "parent"), (.var "l")]) (.forall_ "y" "obj" (.not (.predicate "next" [(.var "l"), (.var "y")])))))

def linkTargetsAtom : Expr := (.forall_ "c" "obj" (.implies (.predicate "children" [(.var "parent"), (.var "c")]) (.forall_ "y" "obj" (.implies (.or (.predicate "next" [(.var "c"), (.var "y")]) (.predicate "prev" [(.var "c"), (.var "y")])) (.and (.predicate "children" [(.var "parent"), (.var "y")]) (.predicate "live" [(.var "y")]))))))

def noWriterAtom : Expr := (.forall_ "f" "fn" (.implies (.predicate "concurrent" [(.var "f")]) (.forall_ "r" "run" (.implies (.predicate "runs" [(.var "f"), (.var "r")]) (.predicate "keeps_links" [(.var "parent"), (.var "r")])))))

def parentOutsideAtom : Expr := (.not (.predicate "children" [(.var "parent"), (.var "parent")]))

def predecessorAtom : Expr := (.forall_ "b" "obj" (.implies (.predicate "children" [(.var "parent"), (.var "b")]) (.forall_ "a" "obj" (.implies (.predicate "pred" [(.var "parent"), (.var "b"), (.var "a")]) (.predicate "prev" [(.var "b"), (.var "a")])))))

/-- An atom holds of a model: its typed rendering holds at `#parent`. -/
def Holds (M : Model) (atom : Expr) : Prop := holds (render M) (valuation M) atom

/-! ## What each atom states of any model -/

section Readings
variable (M : Model)

@[simp] theorem render_relation : (render M).relation = relation M := rfl

@[simp] theorem sortMember_obj (v : SemanticValue (Carrier M)) :
    sortMember (render M) "obj" v ↔ ∃ u, v = .opaque (.obj u) := by
  simp [sortMember, render, sortHolds]

@[simp] theorem sortMember_fn (v : SemanticValue (Carrier M)) :
    sortMember (render M) "fn" v ↔ ∃ u, v = .opaque (.fn u) := by
  simp [sortMember, render, sortHolds]

@[simp] theorem sortMember_run (v : SemanticValue (Carrier M)) :
    sortMember (render M) "run" v ↔ ∃ r, v = .opaque (.run r) := by
  simp [sortMember, render, sortHolds]

theorem chain_atom_reads :
    Holds M chainAtom ↔
      (((((M.live M.parent ∧ ∀ c, M.owned M.parent c → M.children M.parent c) ∧
        ∀ c, M.children M.parent c → M.owned M.parent c ∧ M.live c) ∧
        ∀ c, M.child M.parent c → M.first M.parent c) ∧
        ∀ c, M.first M.parent c → M.child M.parent c) ∧
        ∀ b, M.children M.parent b → ∀ a, M.pred M.parent b a → M.next a b) ∧
        ∀ c, M.children M.parent c → ¬ M.nextReach c c := by
  simp [Holds, chainAtom, holds, evaluate, relation, valuation, QuantProposition.bind]

theorem distinct_atom_reads : Holds M distinctAtom ↔ M.nodup M.parent := by
  simp [Holds, distinctAtom, holds, evaluate, relation, valuation]

theorem empty_atom_reads :
    Holds M emptyAtom ↔
      ((∀ c, ¬ M.child M.parent c) → ∀ c, ¬ M.children M.parent c) ∧
        ((∀ c, ¬ M.children M.parent c) → ∀ c, ¬ M.child M.parent c) := by
  simp [Holds, emptyAtom, holds, evaluate, relation, valuation, QuantProposition.bind]

theorem first_prev_atom_reads :
    Holds M firstPrevAtom ↔
      ∀ f, M.first M.parent f → ∀ l, M.last M.parent l → M.prev f l := by
  simp [Holds, firstPrevAtom, holds, evaluate, relation, valuation, QuantProposition.bind]

theorem last_next_atom_reads :
    Holds M lastNextAtom ↔ ∀ l, M.last M.parent l → ∀ y, ¬ M.next l y := by
  simp [Holds, lastNextAtom, holds, evaluate, relation, valuation, QuantProposition.bind]

theorem link_targets_atom_reads :
    Holds M linkTargetsAtom ↔
      ∀ c, M.children M.parent c → ∀ y, M.next c y ∨ M.prev c y →
        M.children M.parent y ∧ M.live y := by
  simp [Holds, linkTargetsAtom, holds, evaluate, relation, valuation, QuantProposition.bind]

/-- `#concurrent |-> #keeps_links(#parent)`, rendered: every run of every
concurrent callback keeps the parent's links (the maps-to semantics of
`QCore.PremiseFormula.eval`). -/
theorem no_writer_atom_reads :
    Holds M noWriterAtom ↔
      ∀ f, M.concurrent f → ∀ r, M.runs f r → M.keepsLinks M.parent r := by
  simp [Holds, noWriterAtom, holds, evaluate, relation, valuation, QuantProposition.bind]

theorem parent_outside_atom_reads :
    Holds M parentOutsideAtom ↔ ¬ M.children M.parent M.parent := by
  simp [Holds, parentOutsideAtom, holds, evaluate, relation, valuation]

theorem predecessor_atom_reads :
    Holds M predecessorAtom ↔
      ∀ b, M.children M.parent b → ∀ a, M.pred M.parent b a → M.prev b a := by
  simp [Holds, predecessorAtom, holds, evaluate, relation, valuation, QuantProposition.bind]

end Readings

/-! ## Faithfulness: each atom is its `CjsonRepresentationPremises` definition -/

section Faithful
open QCore.CjsonRepresentationPremises
variable (e : Environment)

theorem chain_atom_iff : Holds (detachModel e) chainAtom ↔ chain e := by
  rw [chain_atom_reads]
  simp only [detachModel, chain, true_and]
  constructor
  · rintro ⟨⟨⟨⟨⟨⟨hlive, hown1⟩, hown2⟩, hc1⟩, hc2⟩, hadj⟩, hacyc⟩
    refine ⟨hlive, fun x => ⟨hown1 x, fun hx => (hown2 x hx).1⟩, fun x hx => (hown2 x hx).2,
      QCore.PremiseFormula.option_eq_of_values hc1 hc2, ?_, hacyc⟩
    intro before a b suffix hs
    exact hadj b (by rw [hs]; simp) a ⟨before, suffix, hs⟩
  · rintro ⟨hlive, hown, hlive', hhead, hadj, hacyc⟩
    refine ⟨⟨⟨⟨⟨⟨hlive, fun u hu => (hown u).1 hu⟩, fun u hu => ⟨(hown u).2 hu, hlive' u hu⟩⟩,
      fun u hu => hhead ▸ hu⟩, fun u hu => hhead ▸ hu⟩, ?_⟩, hacyc⟩
    rintro b _ a ⟨x, y, hs⟩
    exact hadj x a b y hs

theorem distinct_atom_iff : Holds (detachModel e) distinctAtom ↔ distinct e := by
  rw [distinct_atom_reads]
  simp [detachModel, distinct]

theorem empty_atom_iff : Holds (detachModel e) emptyAtom ↔ empty e := by
  rw [empty_atom_reads]
  simp only [detachModel, empty, true_and]
  constructor
  · rintro ⟨he1, he2⟩
    constructor
    · intro hc
      cases hcs : e.children with
      | nil => rfl
      | cons x xs =>
          exact absurd (hcs ▸ List.mem_cons_self ..) (he1 (fun u h => by simp [hc] at h) x)
    · intro hcs
      cases hc : e.child with
      | none => rfl
      | some x => exact absurd hc (he2 (fun u h => by simp [hcs] at h) x)
  · rintro ⟨he1, he2⟩
    constructor
    · intro hc u hu
      have : e.child = none := by
        cases h : e.child with
        | none => rfl
        | some v => exact absurd h (hc v)
      rw [he1 this] at hu
      exact absurd hu (by simp)
    · intro hcs u hu
      have : e.children = [] := by
        cases h : e.children with
        | nil => rfl
        | cons x xs => exact absurd (h ▸ List.mem_cons_self ..) (hcs x)
      rw [he2 this] at hu
      exact absurd hu (by simp)

theorem first_prev_atom_iff : Holds (detachModel e) firstPrevAtom ↔ firstPrev e := by
  rw [first_prev_atom_reads]
  simp only [detachModel, firstPrev, true_and]
  exact ⟨fun h f l hf hl => h f hf l hl, fun h f hf l hl => h f l hf hl⟩

theorem last_next_atom_iff : Holds (detachModel e) lastNextAtom ↔ lastNext e := by
  rw [last_next_atom_reads]
  simp only [detachModel, lastNext, true_and]
  constructor
  · intro h l hl
    cases hn : e.next l with
    | none => rfl
    | some y => exact absurd hn (h l hl y)
  · intro h l hl y hy
    rw [h l hl] at hy
    exact absurd hy (by simp)

theorem link_targets_atom_iff : Holds (detachModel e) linkTargetsAtom ↔ linkTargets e := by
  rw [link_targets_atom_reads]
  simp [detachModel, linkTargets]

theorem no_writer_atom_iff : Holds (detachModel e) noWriterAtom ↔ noWriter e := by
  rw [no_writer_atom_reads]
  simp [detachModel, noWriter]

theorem parent_outside_atom_iff :
    Holds (detachModel e) parentOutsideAtom ↔ parentOutside e := by
  rw [parent_outside_atom_reads]
  simp [detachModel, parentOutside]

theorem predecessor_atom_iff : Holds (detachModel e) predecessorAtom ↔ predecessor e := by
  rw [predecessor_atom_reads]
  simp only [detachModel, predecessor, true_and]
  constructor
  · intro h before a b suffix hs
    exact h b (by rw [hs]; simp) a ⟨before, suffix, hs⟩
  · rintro h b _ a ⟨x, y, hs⟩
    exact h x a b y hs

/-- **Faithfulness of the nine typed atoms**, collected: in the model an
entry environment induces, each atom holds exactly when the
`CjsonRepresentationPremises` proposition its binding manifest names does. -/
theorem typed_atoms_are_the_definitions :
    (Holds (detachModel e) chainAtom ↔ chain e) ∧
    (Holds (detachModel e) distinctAtom ↔ distinct e) ∧
    (Holds (detachModel e) emptyAtom ↔ empty e) ∧
    (Holds (detachModel e) firstPrevAtom ↔ firstPrev e) ∧
    (Holds (detachModel e) lastNextAtom ↔ lastNext e) ∧
    (Holds (detachModel e) linkTargetsAtom ↔ linkTargets e) ∧
    (Holds (detachModel e) noWriterAtom ↔ noWriter e) ∧
    (Holds (detachModel e) parentOutsideAtom ↔ parentOutside e) ∧
    (Holds (detachModel e) predecessorAtom ↔ predecessor e) :=
  ⟨chain_atom_iff e, distinct_atom_iff e, empty_atom_iff e, first_prev_atom_iff e,
    last_next_atom_iff e, link_targets_atom_iff e, no_writer_atom_iff e,
    parent_outside_atom_iff e, predecessor_atom_iff e⟩

/-- The declaration's premise formula (`detach_list_representation`) holds
exactly when its nine typed atoms do: the decomposition the typed atoms
state is the premise, conjunct for conjunct, neither narrower nor wider. -/
theorem detach_formula_is_the_nine_typed_atoms :
    (eval (detachModel e) [] detachFormula).holds ↔
      Holds (detachModel e) chainAtom ∧ Holds (detachModel e) distinctAtom ∧
        Holds (detachModel e) emptyAtom ∧ Holds (detachModel e) firstPrevAtom ∧
        Holds (detachModel e) lastNextAtom ∧ Holds (detachModel e) linkTargetsAtom ∧
        Holds (detachModel e) noWriterAtom ∧ Holds (detachModel e) parentOutsideAtom ∧
        Holds (detachModel e) predecessorAtom := by
  rw [QCore.PremiseFormula.detach_formula_is_the_nine_atoms, chain_atom_iff, distinct_atom_iff,
    empty_atom_iff, first_prev_atom_iff, last_next_atom_iff, link_targets_atom_iff,
    no_writer_atom_iff, parent_outside_atom_iff, predecessor_atom_iff]

/-- The atoms assume something: an environment whose parent is a child of
itself falsifies `parentOutsideAtom` (so the conjunction is not vacuous). -/
theorem parent_outside_atom_can_fail :
    ¬ Holds (detachModel ⟨0, 0, [0], fun _ => True, fun _ => True, some 0,
      fun _ => none, fun _ => none, []⟩) parentOutsideAtom := by
  rw [parent_outside_atom_iff]
  simp [parentOutside]

end Faithful

/-! ## A string is not a hypothesis atom (the `QF002` ban) -/

/-- An atom's statement in a hypothesis graph: an opaque string, or a typed
definition (C04.18.7). -/
inductive AtomStatement where
  | str (s : String)
  | typed (definition : Expr)

/-- Graph admission of one atom's statement (`qcore_contract::hypothesis::validate`).
`legacy` is whether the atom is a generated `legacy_atom_*` (the erasure of a
named premise): it alone keeps an opaque statement, and it is never typed. A
declared atom is admitted only typed. -/
def accept (legacy : Bool) : AtomStatement → Bool
  | .str _ => legacy
  | .typed _ => !legacy

/-- Card SP6: no string is a declared hypothesis atom, whatever it says. -/
theorem string_is_not_a_hypothesis (s : String) : accept false (.str s) = false := rfl

/-- A declared atom is admitted exactly when it is typed. -/
theorem declared_accepted_iff_typed (a : AtomStatement) :
    accept false a = true ↔ ∃ d, a = .typed d := by
  cases a <;> simp [accept]

/-- The atom decision vectors shared with the Rust table
`qcore_contract::hypothesis::ATOM_DECISION_VECTORS`: whether the atom is a
generated legacy atom, its statement, and whether the graph admits it. -/
def hypothesisVectors : List (Bool × AtomStatement × Bool) := [
  (false, .str "For this removed-result declaration, chain child identities are pairwise distinct", false),
  (false, .typed (.predicate "nodup" [(.var "parent")]), true),
  (false, .str "#nodup(#parent)", false),
  (false, .str "A", false),
  (true, .str "#concurrent |-> #inert", true),
  (true, .str "p = 0", true),
  (true, .typed (.predicate "nodup" [(.var "parent")]), false)
]

theorem hypothesis_vectors_agree :
    hypothesisVectors.all (fun (legacy, statement, expected) => accept legacy statement == expected) = true := by
  rfl

end QCore.CjsonTypedAtoms
