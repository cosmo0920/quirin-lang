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
# A declaration of several init-declarators is its declarations one per statement (card LATER-DECLARATOR-INITIALISER-UNREAD)

Gateway rule 74k. The statement reader read `T *a = X, *b = Y;` as one
assignment to `a` whose value `X , * b = Y` dereferences `b`, and refused it.
The card reads each init-declarator as its own initialisation
(`crates/qcore-c-gateway/src/later_declarator.rs`, hook
`legacy_later_declarator.rs`).

## What is proved

* **C's meaning.** A declaration executes its declarators in order, each
  after the previous one's full declarator (a sequence point, C11 6.7.6p3),
  under the shared specifiers. That is exactly the sequence of the same
  declarations written one per statement (`the_declaration_is_its_split`), and
  the split keeps every declarator, its name and its pointer level, under the
  same specifiers (`the_split_keeps_every_declarator`,
  `each_piece_is_one_declarator`).
* **The reading.** The lowered block runs as the sequence of the pieces'
  lowerings (`the_block_reads_as_the_sequence`); with the C meaning, the block
  reads the declaration exactly as the one-per-statement declarations read
  (`reading_the_declaration_is_reading_its_split`). An uninitialised piece
  lowered as no event changes nothing (`no_event_changes_nothing`).
* **The splitter.** Splitting the top-level commas of declarators that hold
  no top-level comma gives the declarators back
  (`splitting_the_joined_declarators_gives_them_back`).
* **Fail-closed.** The per-declarator decision is a finite table, bound to the
  Rust by shared vectors (`vectors_match`); the hook lowers nothing unless
  every declarator is admitted (`one_unread_declarator_keeps_the_old_reading`)
  and one is a pointer
  (`a_declaration_without_a_pointer_keeps_the_old_reading`);
  a non-automatic object, an initialiser spelling a cast that is not the null
  constant, a non-pointer initialiser that is not a numeric literal, and an
  uninitialised declarator that may evaluate something are each refused.

## What this is not

A proof that the Rust lexical splitter, the Clang `DeclStmt` match and the
statement lowering compute what they are said to (trust boundary 28), pinned
by `crates/qcore-c-gateway/tests/later_declarator_initialiser.rs`. The C
meaning of a declaration as an ordered fold is the premise read from C11
6.7.6p3 and 6.2.1p7, not derived from a C semantics here.
-/

namespace QCore.LaterDeclarator

/-! ## The decision table -/

/-- What the AST and the piece's own spelling say about one declarator. -/
structure Facts where
  automaticObject : Bool
  initialised : Bool
  evaluatesNothing : Bool
  pointer : Bool
  nullConstant : Bool
  spellsACast : Bool
  numericLiteral : Bool
  deriving DecidableEq, Repr

/-- How one declarator is lowered. -/
inductive Lowering where
  | noEvent
  | alone
  deriving DecidableEq, Repr

/-- The per-declarator decision; `none` keeps the old reading. -/
def decide (f : Facts) : Option Lowering :=
  if !f.automaticObject then none
  else if !f.initialised then (if f.evaluatesNothing then some .noEvent else none)
  else if f.pointer then (if f.spellsACast && !f.nullConstant then none else some .alone)
  else if f.numericLiteral then some .alone
  else none

def code : Option Lowering → Nat
  | none => 0
  | some .noEvent => 1
  | some .alone => 2

def bools : List Bool := [false, true]

def everyFacts : List Facts :=
  bools.flatMap fun a => bools.flatMap fun b => bools.flatMap fun c =>
  bools.flatMap fun d => bools.flatMap fun e => bools.flatMap fun f =>
  bools.map fun g => ⟨a, b, c, d, e, f, g⟩

def evaluatedVectors : List Nat := everyFacts.map fun f => code (decide f)

def vectors : List Nat := [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 2, 0, 2, 0, 2, 0, 2, 2, 2, 0, 0, 2, 2, 2, 2, 0, 2, 0, 2, 0, 2, 0, 2, 2, 2, 0, 0, 2, 2, 2, 2]

theorem vectors_match : evaluatedVectors = vectors := by decide

theorem a_non_automatic_object_is_refused (f : Facts) (h : f.automaticObject = false) :
    decide f = none := by
  simp [decide, h]

theorem an_uninitialised_declarator_must_evaluate_nothing (f : Facts)
    (hi : f.initialised = false) (he : f.evaluatesNothing = false) :
    decide f = none := by
  simp [decide, hi, he]

theorem a_cast_that_is_not_null_is_refused (f : Facts) (hi : f.initialised = true)
    (hp : f.pointer = true) (hc : f.spellsACast = true) (hn : f.nullConstant = false) :
    decide f = none := by
  unfold decide
  cases f.automaticObject <;> simp [hi, hp, hc, hn]

theorem a_scalar_needs_a_numeric_literal (f : Facts) (hi : f.initialised = true)
    (hp : f.pointer = false) (hl : f.numericLiteral = false) :
    decide f = none := by
  unfold decide
  cases f.automaticObject <;> simp [hi, hp, hl]

theorem no_event_only_without_an_initialiser (f : Facts) (h : decide f = some .noEvent) :
    f.initialised = false ∧ f.evaluatesNothing = true := by
  revert h
  unfold decide
  cases f.automaticObject <;> cases f.initialised <;> cases f.evaluatesNothing <;>
    cases f.pointer <;> cases f.nullConstant <;> cases f.spellsACast <;>
    cases f.numericLiteral <;> simp

/-- Every declarator lowered, or none. -/
def decideEach (fs : List Facts) : Option (List Lowering) :=
  fs.mapM decide

/-- The whole declaration: only one with a pointer declarator is split (a
    declaration of scalars alone keeps the scalar routes it had), and then
    every declarator lowered, or none. -/
def decideAll (fs : List Facts) : Option (List Lowering) :=
  if fs.any (·.pointer) then decideEach fs else none

theorem a_declaration_without_a_pointer_keeps_the_old_reading (fs : List Facts)
    (h : ∀ f ∈ fs, f.pointer = false) : decideAll fs = none := by
  have : fs.any (·.pointer) = false := by
    simp only [List.any_eq_false]
    intro f hf
    simp [h f hf]
  simp [decideAll, this]

theorem one_unread_declarator_keeps_the_old_reading (fs : List Facts) (f : Facts)
    (hf : f ∈ fs) (h : decide f = none) : decideAll fs = none := by
  suffices hs : decideEach fs = none by
    unfold decideAll
    split <;> simp [hs]
  unfold decideEach
  induction fs with
  | nil => cases hf
  | cons x xs ih =>
    cases hf with
    | head => simp [List.mapM_cons, h]
    | tail _ hm =>
      simp only [List.mapM_cons]
      cases hx : decide x with
      | none => simp
      | some l => simp [ih hm]

theorem every_admitted_declarator_is_decided (fs : List Facts) (ls : List Lowering)
    (h : decideAll fs = some ls) : ls.length = fs.length := by
  have he : decideEach fs = some ls := by
    unfold decideAll at h
    split at h
    · exact h
    · cases h
  clear h
  unfold decideEach at he
  induction fs generalizing ls with
  | nil => simp at he; simp [← he]
  | cons x xs ih =>
    simp only [List.mapM_cons] at he
    cases hx : decide x with
    | none => simp [hx] at he
    | some l =>
      cases hr : xs.mapM decide with
      | none => simp [hx, hr] at he
      | some rest =>
        simp [hx, hr] at he
        subst he
        simp [ih rest hr]

/-! ## C's meaning of a declaration, and its split -/

/-- A declarator: its name, its pointer level, and its initialiser. -/
structure Declarator (ε : Type) where
  name : String
  stars : Nat
  init : Option ε

/-- A declaration: shared specifiers and its declarators, in order. -/
structure Declaration (σ ε : Type) where
  specifiers : σ
  declarators : List (Declarator ε)

/-- The same declarations written one per statement. -/
def split {σ ε : Type} (d : Declaration σ ε) : List (Declaration σ ε) :=
  d.declarators.map fun x => ⟨d.specifiers, [x]⟩

/-- C's meaning of a declaration over a state space `S`, given the meaning
    `step` of one declarator under the specifiers: the declarators in order,
    each full declarator a sequence point (C11 6.7.6p3). -/
def cexec {σ ε S : Type} (step : σ → Declarator ε → S → S)
    (d : Declaration σ ε) (s : S) : S :=
  d.declarators.foldl (fun s x => step d.specifiers x s) s

theorem the_declaration_is_its_split {σ ε S : Type} (step : σ → Declarator ε → S → S)
    (d : Declaration σ ε) (s : S) :
    cexec step d s = (split d).foldl (fun s d' => cexec step d' s) s := by
  unfold cexec split
  generalize d.declarators = xs
  induction xs generalizing s with
  | nil => rfl
  | cons x xs ih => simp [List.foldl, ih]

theorem the_split_keeps_every_declarator {σ ε : Type} (d : Declaration σ ε) :
    (split d).flatMap Declaration.declarators = d.declarators := by
  unfold split
  generalize d.declarators = xs
  induction xs with
  | nil => rfl
  | cons x xs ih => simp [ih]

theorem each_piece_is_one_declarator {σ ε : Type} (d : Declaration σ ε) (p : Declaration σ ε)
    (h : p ∈ split d) :
    p.specifiers = d.specifiers ∧ ∃ x ∈ d.declarators, p.declarators = [x] := by
  unfold split at h
  simp only [List.mem_map] at h
  obtain ⟨x, hx, rfl⟩ := h
  exact ⟨rfl, x, hx, rfl⟩

/-! ## The reading -/

/-- The analyser's run of an instruction list. -/
def run {ι S : Type} (exec : ι → S → S) (is : List ι) (s : S) : S :=
  is.foldl (fun s i => exec i s) s

/-- The block the hook lowers: each piece's lowering, in order. -/
def lowerSplit {σ ε ι : Type} (lowerOne : Declaration σ ε → List ι)
    (d : Declaration σ ε) : List ι :=
  (split d).flatMap lowerOne

theorem the_block_reads_as_the_sequence {σ ε ι S : Type} (exec : ι → S → S)
    (lowerOne : Declaration σ ε → List ι) (d : Declaration σ ε) (s : S) :
    run exec (lowerSplit lowerOne d) s
      = (split d).foldl (fun s p => run exec (lowerOne p) s) s := by
  unfold lowerSplit run
  generalize split d = ps
  induction ps generalizing s with
  | nil => rfl
  | cons p ps ih => simp [List.flatMap_cons, List.foldl_append, ih]

/-- If the analyser's reading of each declaration written alone is faithful to
    C's step for it, the block is faithful to the declaration: the multi-
    declarator declaration reads exactly as its declarations one per
    statement read. -/
theorem reading_the_declaration_is_reading_its_split {σ ε ι S : Type}
    (step : σ → Declarator ε → S → S) (exec : ι → S → S)
    (lowerOne : Declaration σ ε → List ι)
    (faithful : ∀ spec x s, run exec (lowerOne ⟨spec, [x]⟩) s = step spec x s)
    (d : Declaration σ ε) (s : S) :
    run exec (lowerSplit lowerOne d) s = cexec step d s := by
  rw [the_block_reads_as_the_sequence, the_declaration_is_its_split]
  unfold split
  generalize d.declarators = xs
  induction xs generalizing s with
  | nil => rfl
  | cons x xs ih =>
    simp only [List.map_cons, List.foldl_cons]
    rw [faithful]
    have := ih (step d.specifiers x s)
    simpa [cexec] using this

theorem no_event_changes_nothing {ι S : Type} (exec : ι → S → S) (s : S) :
    run exec ([] : List ι) s = s := rfl

/-! ## The splitter -/

/-- The current group and the groups after it. -/
def splitComma : List String → List String × List (List String)
  | [] => ([], [])
  | t :: ts =>
    let r := splitComma ts
    if t = "," then ([], r.1 :: r.2) else (t :: r.1, r.2)

def groups (ts : List String) : List (List String) :=
  (splitComma ts).1 :: (splitComma ts).2

/-- Declarators joined by top-level commas. -/
def join : List (List String) → List String
  | [] => []
  | [g] => g
  | g :: g' :: gs => g ++ "," :: join (g' :: gs)

theorem split_after_a_group (g rest : List String) (h : "," ∉ g) :
    splitComma (g ++ rest) = (g ++ (splitComma rest).1, (splitComma rest).2) := by
  induction g with
  | nil => rfl
  | cons t g ih =>
    have ht : t ≠ "," := fun e => h (e ▸ List.mem_cons_self)
    have hg : "," ∉ g := fun m => h (List.mem_cons_of_mem _ m)
    simp [splitComma, ht, ih hg]

theorem splitting_the_joined_declarators_gives_them_back (g : List String)
    (gs : List (List String)) (h : ∀ x ∈ g :: gs, "," ∉ x) :
    groups (join (g :: gs)) = g :: gs := by
  induction gs generalizing g with
  | nil =>
    have := split_after_a_group g [] (h g List.mem_cons_self)
    simp only [List.append_nil, splitComma] at this
    simp [groups, join, this]
  | cons g' gs ih =>
    have hg := h g List.mem_cons_self
    have hrest : ∀ x ∈ g' :: gs, "," ∉ x := fun x m => h x (List.mem_cons_of_mem _ m)
    have inner := ih g' hrest
    unfold groups at inner
    have := split_after_a_group g ("," :: join (g' :: gs)) hg
    simp only [groups, join, this, splitComma]
    simp at inner
    simp [inner]

end QCore.LaterDeclarator
