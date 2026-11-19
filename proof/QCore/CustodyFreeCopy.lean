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
# Copy sources that carry no resource identity (card AG2-UNRESOLVED-ALIAS, gateway rule 62)

The abstract rule implemented by `crates/qcore-c-gateway/src/custody_free_copy.rs`
(`decide`, `read`, `CustodyFreeCopies::admits`) and its arm in
`legacy_transfer::transfer_base`.

Route G lowers `D = S;` to one alias node. When `S` has no binding, the old
transfer reports `S` as an unresolved resource identity and gives `D` an
unknown, possibly null binding. Rule 62 leaves `D` unbound, with no report,
for three sources:

* a string or character literal and an enumeration constant, which no
  instruction ever binds (they are never a destination), so their value is
  custody-free by construction;
* an unbound pointer formal whose copies are only read through: every
  reference to a copy is a rebinding, a copy into another copy, a null test, a
  dereference base or a call actual, and the formal itself is never written or
  address-taken.

## What is proved

* The decision table and its vectors (`decision_vectors_hold`), and that a
  copy handed back, written, address-taken or used otherwise withholds the
  reading for its formal (`a_custody_use_of_a_copy_withholds`), as does a
  write to the formal (`a_write_to_the_formal_withholds`).
* The transfer: the admitted copy changes no obligation, leaves the
  destination unbound and every other binding unchanged, and **gives the
  destination exactly the source's binding** (`the_copy_denotes_what_the_source_denotes`),
  so every later decision taken on the destination's binding is the one the
  source would get (`every_binding_decision_agrees`): a release refuses, a
  return escapes nothing.
* A literal or constant is never bound in any state a run reaches from an
  entry where it is unbound, provided no instruction writes it
  (`a_never_written_name_stays_unbound`), so the guard "source unbound" always
  holds for one and the reading is total for it.
* Withheld, the old answer stands, unresolved-identity report included
  (`withheld_keeps_the_report`): W4 summary derivation still sees the formal
  named wherever a copy has a custody use; a bound source never reaches the
  rule (`a_bound_source_keeps_the_old_answer`).

## What this is not

A model of one alias transfer and of the decision, over an abstract state of
name bindings and obligation states. Clang's extraction of declaration kinds,
storage, types and parent nodes, the tokenizer's literal placeholder, route G's
lowering of the copy as one alias node, and the never-bound closure (rules 5b
and 10l) that the formal and every copy must satisfy are premises (trust
boundary 28), not proved here.
-/

namespace QCore.CustodyFreeCopy

/-! ## The decision -/

inductive Subject where
  | formal
  | copy
  deriving DecidableEq, Repr

inductive Use where
  | copyToLocal
  | initLocal
  | nullTest
  | dereference
  | callArgument
  | rebind
  | «return»
  | write
  | address
  | other
  deriving DecidableEq, Repr

inductive Decision where
  | admit
  | withhold
  deriving DecidableEq, Repr

/-- `custody_free_copy::decide`. -/
def decide : Subject → Use → Decision
  | .formal, .rebind => .withhold
  | .formal, .write => .withhold
  | .formal, .address => .withhold
  | .formal, .other => .withhold
  | .formal, _ => .admit
  | .copy, .return => .withhold
  | .copy, .write => .withhold
  | .copy, .address => .withhold
  | .copy, .other => .withhold
  | .copy, _ => .admit

/-- The decision vectors, spelled as `custody_free_copy::DECISION_VECTORS`. -/
def decisionVectors : List (Subject × Use × Decision) :=
  [ (.formal, .copyToLocal, .admit)
  , (.formal, .initLocal, .admit)
  , (.formal, .nullTest, .admit)
  , (.formal, .dereference, .admit)
  , (.formal, .callArgument, .admit)
  , (.formal, .rebind, .withhold)
  , (.formal, .return, .admit)
  , (.formal, .write, .withhold)
  , (.formal, .address, .withhold)
  , (.formal, .other, .withhold)
  , (.copy, .copyToLocal, .admit)
  , (.copy, .initLocal, .admit)
  , (.copy, .nullTest, .admit)
  , (.copy, .dereference, .admit)
  , (.copy, .callArgument, .admit)
  , (.copy, .rebind, .admit)
  , (.copy, .return, .withhold)
  , (.copy, .write, .withhold)
  , (.copy, .address, .withhold)
  , (.copy, .other, .withhold) ]

theorem decision_vectors_hold :
    decisionVectors.all (fun (s, u, d) => decide s u == d) = true := by
  decide

/-- The reading for one formal and its copies: every reference admitted, and
the never-bound premise answered for the formal and every copy. -/
def admits (neverBound : Bool) (refs : List (Subject × Use)) : Bool :=
  neverBound && refs.all (fun (s, u) => decide s u == .admit)

/-- A copy is admitted exactly on a read-through use or its own rebinding. -/
theorem a_copy_is_admitted_exactly_on_a_read (u : Use) :
    decide .copy u = .admit ↔
      (u = .copyToLocal ∨ u = .initLocal ∨ u = .nullTest ∨ u = .dereference ∨
        u = .callArgument ∨ u = .rebind) := by
  cases u <;> simp [decide]

/-- A copy handed back (`return`), written, address-taken or used in any
other way (a store of its value) withholds the reading for its formal. -/
theorem a_custody_use_of_a_copy_withholds (nb : Bool) (refs : List (Subject × Use))
    (u : Use) (hu : u = .return ∨ u = .write ∨ u = .address ∨ u = .other)
    (h : (Subject.copy, u) ∈ refs) : admits nb refs = false := by
  unfold admits
  cases nb
  · rfl
  · simp only [Bool.true_and]
    rw [List.all_eq_false]
    refine ⟨(Subject.copy, u), h, ?_⟩
    rcases hu with hu | hu | hu | hu <;> subst hu <;> decide

/-- A write to the formal, or its address, withholds the reading: a copy
would no longer hold the value the caller handed in. -/
theorem a_write_to_the_formal_withholds (nb : Bool) (refs : List (Subject × Use))
    (u : Use) (hu : u = .rebind ∨ u = .write ∨ u = .address ∨ u = .other)
    (h : (Subject.formal, u) ∈ refs) : admits nb refs = false := by
  unfold admits
  cases nb
  · rfl
  · simp only [Bool.true_and]
    rw [List.all_eq_false]
    refine ⟨(Subject.formal, u), h, ?_⟩
    rcases hu with hu | hu | hu | hu <;> subst hu <;> decide

/-- Without the never-bound premise nothing is admitted. -/
theorem no_premise_no_reading (refs : List (Subject × Use)) : admits false refs = false := rfl

/-! ## The transfer -/

/-- A binding of a name: the obligation sites it may denote. -/
structure Binding where
  targets : List Nat
  unknown : Bool
  mayBeNull : Bool
  deriving DecidableEq, Repr

/-- Bindings by name and obligation states by site; `true` is live. -/
structure State where
  bindings : Nat → Option Binding
  obligations : Nat → Option Bool

/-- Drop one name's binding. -/
def unbind (st : State) (d : Nat) : State :=
  { st with bindings := fun p => if p = d then none else st.bindings p }

/-- The old alias transfer: a bound source's binding is copied; an unbound one
gives an unknown, possibly null binding and an unresolved-identity report
(`true`). -/
def oldAlias (st : State) (d src : Nat) : State × Bool :=
  match st.bindings src with
  | some b => ({ st with bindings := fun p => if p = d then some b else st.bindings p }, false)
  | none =>
      ({ st with bindings := fun p =>
          if p = d then some { targets := [], unknown := true, mayBeNull := true }
          else st.bindings p }, true)

/-- Rule 62's alias transfer: where a reading admits the copy and the source
is unbound, the destination is left unbound with no report; everywhere else
the old transfer. -/
def alias (admitted : Bool) (st : State) (d src : Nat) : State × Bool :=
  if admitted && (st.bindings src).isNone then (unbind st d, false) else oldAlias st d src

/-- No obligation is created or discharged by the copy. -/
theorem the_copy_changes_no_obligation (admitted : Bool) (st : State) (d src : Nat) :
    (alias admitted st d src).1.obligations = st.obligations := by
  unfold alias oldAlias unbind
  split
  · rfl
  · cases st.bindings src <;> rfl

/-- The admitted copy gives the destination exactly the source's binding:
none. The destination denotes what the source denotes. -/
theorem the_copy_denotes_what_the_source_denotes (st : State) (d src : Nat)
    (h : st.bindings src = none) :
    (alias true st d src).1.bindings d = st.bindings src := by
  simp [alias, unbind, h]

/-- So every decision the analysis takes on a name's binding answers for the
copy exactly as for the source. -/
theorem every_binding_decision_agrees {α : Type} (decision : Option Binding → α)
    (st : State) (d src : Nat) (h : st.bindings src = none) :
    decision ((alias true st d src).1.bindings d) = decision (st.bindings src) := by
  rw [the_copy_denotes_what_the_source_denotes st d src h]

/-- No other name's binding changes. -/
theorem no_other_binding_changes (st : State) (d src p : Nat) (h : st.bindings src = none)
    (hp : p ≠ d) : (alias true st d src).1.bindings p = st.bindings p := by
  simp [alias, unbind, h, hp]

/-- The admitted copy reports nothing. -/
theorem the_admitted_copy_reports_nothing (st : State) (d src : Nat)
    (h : st.bindings src = none) : (alias true st d src).2 = false := by
  simp [alias, h]

/-- Withheld, the old answer stands. -/
theorem the_withheld_answer_is_unchanged (st : State) (d src : Nat) :
    alias false st d src = oldAlias st d src := by
  simp [alias]

/-- Withheld for an unbound source, the unresolved-identity report stands:
W4 summary derivation still sees the source named. -/
theorem withheld_keeps_the_report (nb : Bool) (refs : List (Subject × Use)) (st : State)
    (d src : Nat) (hw : admits nb refs = false) (h : st.bindings src = none) :
    (alias (admits nb refs) st d src).2 = true := by
  rw [hw]
  simp [alias, oldAlias, h]

/-- A bound source is never read by the rule. -/
theorem a_bound_source_keeps_the_old_answer (admitted : Bool) (st : State) (d src : Nat)
    (b : Binding) (h : st.bindings src = some b) :
    alias admitted st d src = oldAlias st d src := by
  simp [alias, h]

/-! ## Custody uses of the copy -/

inductive Outcome where
  | refused
  | discharged (site : Nat)
  deriving DecidableEq, Repr

/-- A release (or a transfer) of a name: refused without a binding naming
exactly one live site. -/
def release (st : State) (x : Nat) : Outcome :=
  match st.bindings x with
  | some { targets := [s], unknown := false, .. } =>
      if st.obligations s = some true then .discharged s else .refused
  | _ => .refused

/-- A release of the copy refuses, as a release of the unbound source does. -/
theorem a_release_of_the_copy_refuses (st : State) (d src : Nat)
    (h : st.bindings src = none) : release (alias true st d src).1 d = .refused := by
  simp [release, alias, unbind, h]

/-- `return` of a name escapes the obligation it names, if any. -/
def escapeIfOwned (st : State) (x : Nat) : List Nat :=
  match st.bindings x with
  | some b => b.targets
  | none => []

/-- Handing the copy out escapes nothing: no fresh owner leaves the function. -/
theorem handing_the_copy_out_escapes_nothing (st : State) (d src : Nat)
    (h : st.bindings src = none) : escapeIfOwned (alias true st d src).1 d = [] := by
  simp [escapeIfOwned, alias, unbind, h]

/-! ## Literals and enumeration constants are never bound -/

/-- The instructions that write a binding, by their destination. -/
inductive Instr where
  | copy (d src : Nat)
  | acquire (d site : Nat)
  | clear (d : Nat)
  deriving DecidableEq, Repr

def Instr.dest : Instr → Nat
  | .copy d _ => d
  | .acquire d _ => d
  | .clear d => d

/-- One instruction under rule 62 (`admitted` answers per copy). -/
def step (admitted : Nat → Nat → Bool) (st : State) : Instr → State
  | .copy d src => (alias (admitted d src) st d src).1
  | .acquire d site =>
      { bindings := fun p =>
          if p = d then some { targets := [site], unknown := false, mayBeNull := false }
          else st.bindings p
        obligations := fun s => if s = site then some true else st.obligations s }
  | .clear d =>
      { st with bindings := fun p =>
          if p = d then some { targets := [], unknown := false, mayBeNull := true }
          else st.bindings p }

/-- A run of instructions. -/
def run (admitted : Nat → Nat → Bool) (st : State) : List Instr → State
  | [] => st
  | i :: rest => run admitted (step admitted st i) rest

theorem step_keeps_other_bindings (admitted : Nat → Nat → Bool) (st : State) (i : Instr)
    (c : Nat) (hc : i.dest ≠ c) : (step admitted st i).bindings c = st.bindings c := by
  cases i with
  | copy d src =>
      simp only [Instr.dest] at hc
      simp only [step, alias, unbind, oldAlias]
      split
      · simp [Ne.symm hc]
      · cases st.bindings src <;> simp [Ne.symm hc]
  | acquire d site =>
      simp only [Instr.dest] at hc
      simp [step, Ne.symm hc]
  | clear d =>
      simp only [Instr.dest] at hc
      simp [step, Ne.symm hc]

/-- A name no instruction writes (a literal placeholder or an enumeration
constant, which route G only ever reads) stays unbound along every run from
an entry where it is unbound; so rule 62's guard holds whenever one is
copied. -/
theorem a_never_written_name_stays_unbound (admitted : Nat → Nat → Bool) (prog : List Instr)
    (c : Nat) (hw : ∀ i ∈ prog, i.dest ≠ c) (st : State) (h : st.bindings c = none) :
    (run admitted st prog).bindings c = none := by
  induction prog generalizing st with
  | nil => simpa [run] using h
  | cons i rest ih =>
      simp only [run]
      apply ih
      · intro j hj
        exact hw j (List.mem_cons_of_mem i hj)
      · rw [step_keeps_other_bindings admitted st i c (hw i (List.mem_cons_self))]
        exact h

end QCore.CustodyFreeCopy
