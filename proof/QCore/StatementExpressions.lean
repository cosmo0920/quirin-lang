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

import QCore.ElaborationCore

/-! Clang's selected `_Generic` association and GNU statement expressions
(card CG07-GENERIC-AND-STATEMENT-EXPRESSIONS, gateway rule 11g): a model of
`crates/qcore-c-gateway/src/generic_association.rs`,
`statement_expression.rs`, the elaborator's statement-expression arm in
`elaboration.rs`, and the route-G lowering of `legacy_statement_expression.rs`.

Bound mechanically: `statement_expression::tests::cg07_lean_vectors_match`
reads `admissionVectors` (= `ADMISSION_VECTORS`) and `positionTable`
(= `POSITIONS`); `elaboration::tests::cg07_lifetime_vectors_match_commutes`
reads `lifetimeVectors` against `commutes`; `tests/cg07_statement_expressions.rs`
runs the route-G controls. The Clang-JSON reading and node matching stay a
review premise (trust boundary 28).

Stated here:
* **Selection.** Only the selected association's events occur; the
  controlling expression and every unselected association contribute none.
* **Value.** A statement expression's value is its final expression's, in
  the store its items left.
* **Lifetimes.** Every inner object's lifetime ends at the block, after
  every event of the block.
* **Lowering.** In the admitted positions, the route's order (the items,
  then the position with the final expression in place of the block) is C's.
* **Flat names.** The route has no block scopes; with inner names fresh
  (the hygiene premise) the flat lowering agrees with the scoped one on
  every other name, and without it it does not.
* **Escape.** An admitted used value never points into the block's storage.
* **Forms and positions.** Unlisted positions are refused; so are the
  premises' failures. -/
namespace QCore.StatementExpressions

open QCore.ElaborationCore

/-! ## Generic selection -/

/-- One association: the events its expression performs and its value. -/
structure Assoc where
  events : List CEv
  value : Store → Int

/-- The events C performs for `_Generic (control, assocs…)` when Clang
selected association `sel`: that association's, and never the controlling
expression's (`_control` is unused on purpose). -/
def selectEvents (_control : List CEv) (assocs : List Assoc) (sel : Nat) :
    Option (List CEv) :=
  (assocs[sel]?).map (·.events)

theorem select_events (control : List CEv) (assocs : List Assoc) (sel : Nat) (a : Assoc)
    (h : assocs[sel]? = some a) : selectEvents control assocs sel = some a.events := by
  simp [selectEvents, h]

/-- The controlling expression contributes no event. -/
theorem control_no_event (c c' : List CEv) (assocs : List Assoc) (sel : Nat) :
    selectEvents c assocs sel = selectEvents c' assocs sel := rfl

/-- An unselected association contributes no event: replacing it changes
nothing. -/
theorem unselected_no_event (c : List CEv) (assocs : List Assoc) (sel j : Nat) (a : Assoc)
    (h : j ≠ sel) : selectEvents c (assocs.set j a) sel = selectEvents c assocs sel := by
  simp [selectEvents, h]

/-! ## Statement expressions -/

/-- A trace event: a concrete event, or an inner object's lifetime start or
end. -/
inductive TEv where
  | ev (e : CEv)
  | start (p : Place)
  | stop (p : Place)

/-- `({ items; final; })`: its inner objects, its items' events in order, its
final expression's events and value. -/
structure Block where
  inner : List Place
  items : List (List CEv)
  final : List CEv
  value : Store → Int

def Block.body (b : Block) : List CEv := b.items.flatten ++ b.final

/-- C's evaluation from store `s`: the inner lifetimes start, the items run in
order (each full expression a sequence point), then the final expression,
then the lifetimes end; the value is the final expression's in the store the
items left. -/
def exec (b : Block) (s : Store) : Int × List TEv :=
  (b.value (run b.body s), b.inner.map .start ++ b.body.map .ev ++ b.inner.map .stop)

/-- **Value.** The value is the final expression's, after every item. -/
theorem value_is_final (b : Block) (s : Store) :
    (exec b s).1 = b.value (run (b.items.flatten ++ b.final) s) := rfl

/-- **Lifetimes.** The trace ends with every inner lifetime's end, and every
event of the block comes before them. -/
theorem lifetimes_end_at_block (b : Block) (s : Store) :
    ∃ pre, (exec b s).2 = pre ++ b.inner.map .stop ∧ ∀ e ∈ b.body, TEv.ev e ∈ pre := by
  refine ⟨b.inner.map .start ++ b.body.map .ev, rfl, ?_⟩
  intro e he
  exact List.mem_append_right _ (List.mem_map_of_mem he)

/-- The concrete events of a trace (the route has no lifetime events). -/
def events : List TEv → List CEv
  | [] => []
  | .ev e :: rest => e :: events rest
  | .start _ :: rest => events rest
  | .stop _ :: rest => events rest

theorem events_append (a b : List TEv) : events (a ++ b) = events a ++ events b := by
  induction a with
  | nil => rfl
  | cons x a ih => cases x <;> simp [events, ih]

theorem events_starts (l : List Place) : events (l.map .start) = [] := by
  induction l with
  | nil => rfl
  | cons _ l ih => simp [events, ih]

theorem events_stops (l : List Place) : events (l.map .stop) = [] := by
  induction l with
  | nil => rfl
  | cons _ l ih => simp [events, ih]

theorem events_evs (l : List CEv) : events (l.map .ev) = l := by
  induction l with
  | nil => rfl
  | cons _ l ih => simp [events, ih]

theorem events_exec (b : Block) (s : Store) : events (exec b s).2 = b.body := by
  simp [exec, events_append, events_starts, events_stops, events_evs]

/-- C: `p = ({ … })` completes the block, then stores its value. -/
def assignC (b : Block) (p : Place) (s : Store) : Store :=
  update (run (events (exec b s).2) s) p (exec b s).1

/-- The route: the items, then `p = e` (the final expression's events, then
the store of its value). -/
def assignRoute (b : Block) (p : Place) (s : Store) : Store :=
  let u := run b.final (run b.items.flatten s)
  update u p (b.value u)

/-- **Lowering (assigned, initializer).** The route's order is C's. -/
theorem lowering_exact (b : Block) (p : Place) (s : Store) :
    assignC b p s = assignRoute b p s := by
  rw [assignC, events_exec]
  simp [assignRoute, exec, Block.body, run_append]

/-- **Lowering (discarded).** A dropped value leaves the block's own effect;
the route parses the block as a block. -/
theorem discarded_exact (b : Block) (s : Store) :
    run (events (exec b s).2) s = run b.final (run b.items.flatten s) := by
  simp [events_exec, Block.body, run_append]

/-- **Lowering (returned).** `return ({ … })` returns the value `return e`
returns after the items. -/
theorem returned_exact (b : Block) (s : Store) :
    (exec b s).1 = b.value (run b.final (run b.items.flatten s)) := by
  simp [exec, Block.body, run_append]

/-! ## Flat names -/

/-- Leaving a scope restores every inner name's outer value. -/
def restore (old : Store) (inner : List Place) (t : Store) : Store :=
  fun q => if q ∈ inner then old q else t q

def scopedRun (b : Block) (s : Store) : Store := restore s b.inner (run b.body s)

/-- The route's flat lowering: no scope, no restore. -/
def flatRun (b : Block) (s : Store) : Store := run b.body s

/-- **Flat names.** On every name the block does not declare, the flat
lowering is the scoped one: with the hygiene premise (no inner name is used
outside the block's scope), nothing after the block can tell them apart. -/
theorem flat_names_exact (b : Block) (s : Store) (q : Place) (h : q ∉ b.inner) :
    flatRun b s q = scopedRun b s q := by
  simp [flatRun, scopedRun, restore, h]

/-- Without the premise the flat lowering is wrong: an inner declaration
shadowing an outer name leaves its write visible under that name. -/
theorem shadow_breaks_flat :
    flatRun ⟨[0], [[.write 0 1]], [], fun _ => 0⟩ (fun _ => 0) 0 ≠
      scopedRun ⟨[0], [[.write 0 1]], [], fun _ => 0⟩ (fun _ => 0) 0 := by
  simp [flatRun, scopedRun, restore, Block.body, run, step, update]

/-! ## Escape -/

/-- A value: an integer, or the address of an object. -/
inductive Val where
  | int (v : Int)
  | addr (p : Place)

/-- The admission of one statement expression (`statement_expression::admit`). -/
def admit (positionOk hygienic automatic usesValue addressTaken lastIsValue : Bool) : Bool :=
  positionOk && hygienic && automatic && (!usesValue || (!addressTaken && lastIsValue))

/-- **Escape.** An admitted used value never points into the block's own
storage: a value can hold an inner object's address only if the block took
it, and the admission refuses a block that takes one. -/
theorem admitted_no_escape (inner taken : List Place) (v : Val)
    (hv : ∀ p, v = .addr p → p ∈ inner → p ∈ taken)
    (hadmit : admit true true true true (inner.any (fun p => decide (p ∈ taken))) true = true) :
    ∀ p, v = .addr p → p ∉ inner := by
  intro p hp hin
  have ht := hv p hp hin
  have hany : inner.any (fun p => decide (p ∈ taken)) = true :=
    List.any_eq_true.mpr ⟨p, hin, by simp [ht]⟩
  simp [admit, hany] at hadmit

theorem escape_refused (p h a l : Bool) : admit p h a true true l = false := by
  cases p <;> cases h <;> cases a <;> cases l <;> rfl

theorem shadow_refused (p a u t l : Bool) : admit p false a u t l = false := by
  cases p <;> rfl

theorem static_refused (p h u t l : Bool) : admit p h false u t l = false := by
  cases p <;> cases h <;> rfl

theorem no_value_refused (p h a : Bool) : admit p h a true false false = false := by
  cases p <;> cases h <;> cases a <;> rfl

/-- A dropped value needs neither premise of the value. -/
theorem discarded_needs_no_value (t l : Bool) : admit true true true false t l = true := by
  cases t <;> cases l <;> rfl

/-- `admissionVectors` = `ADMISSION_VECTORS`: `(position_ok, hygienic,
automatic, uses_value, address_taken, last_is_value, admitted)`. -/
def admissionVectors : List (Bool × Bool × Bool × Bool × Bool × Bool × Bool) := [
  (true, true, true, false, false, false, true),
  (true, true, true, false, true, false, true),
  (true, true, true, true, false, true, true),
  (true, true, true, true, true, true, false),
  (true, true, true, true, false, false, false),
  (false, true, true, false, false, true, false),
  (true, false, true, false, false, true, false),
  (true, true, false, false, false, true, false),
  (true, false, true, true, false, true, false),
  (false, true, true, true, false, true, false)
]

theorem admission_vectors_hold :
    admissionVectors.all (fun v =>
      admit v.1 v.2.1 v.2.2.1 v.2.2.2.1 v.2.2.2.2.1 v.2.2.2.2.2.1 == v.2.2.2.2.2.2) = true := by
  decide

/-! ## Positions -/

/-- `positionTable` = `POSITIONS`. -/
def positionTable : List (String × Bool) := [
  ("discarded", true),
  ("initializer", true),
  ("assigned", true),
  ("returned", true),
  ("operand", false),
  ("call actual", false),
  ("condition", false),
  ("assigned path", false),
  ("cast value", false)
]

def positionAdmits (name : String) : Bool :=
  ((positionTable.find? (·.1 == name)).map (·.2)).getD false

theorem unlisted_position_refused (name : String)
    (h : positionTable.find? (·.1 == name) = none) : positionAdmits name = false := by
  simp [positionAdmits, h]

theorem admitted_positions :
    positionAdmits "discarded" = true ∧ positionAdmits "initializer" = true ∧
      positionAdmits "assigned" = true ∧ positionAdmits "returned" = true := by decide

theorem operand_positions_refused :
    positionAdmits "operand" = false ∧ positionAdmits "call actual" = false ∧
      positionAdmits "condition" = false ∧ positionAdmits "assigned path" = false ∧
      positionAdmits "cast value" = false := by decide

/-! ## Lifetime events in the order check -/

/-- The abstract events of the order check, with lifetimes. -/
inductive LEv where
  | start (p : Nat)
  | stop (p : Nat)
  | read (p : Nat)
  | write (p : Nat)
  deriving DecidableEq, Repr

def LEv.place : LEv → Nat
  | .start p => p
  | .stop p => p
  | .read p => p
  | .write p => p

/-- `commutes` on lifetime events over distinct named objects: a lifetime
start or end touches its own object only; two reads commute. -/
def lifetimeCommutes : LEv → LEv → Bool
  | .read _, .read _ => true
  | a, b => a.place != b.place

theorem lifetime_touches_own (a b : LEv) (h : lifetimeCommutes a b = true)
    (hr : ¬ (∃ p q, a = .read p ∧ b = .read q)) : a.place ≠ b.place := by
  cases a <;> cases b <;> simp_all [lifetimeCommutes, LEv.place]

/-- `lifetimeVectors`, read by `cg07_lifetime_vectors_match_commutes`. -/
def lifetimeVectors : List (LEv × LEv × Bool) := [
  (.start 0, .read 1, true),
  (.start 0, .read 0, false),
  (.stop 0, .write 1, true),
  (.stop 0, .write 0, false),
  (.start 0, .stop 1, true),
  (.stop 0, .start 0, false),
  (.read 0, .stop 1, true),
  (.write 1, .stop 1, false)
]

theorem lifetime_vectors_hold :
    lifetimeVectors.all (fun v => lifetimeCommutes v.1 v.2.1 == v.2.2) = true := by decide

end QCore.StatementExpressions
