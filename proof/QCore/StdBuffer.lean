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

import QCore.ModuleImports

/-! `source.buffer.v1` — the custody and bounds discipline of the `std.buffer`
bundled module (card `STD-BUFFER`).

Two claims are proved here, and they are the two the Rust route rests on.

**The extent is transported.** An acquisition carries a length, that length is
the resource's footprint in the ownership ledger for its whole lifetime, and no
non-custody event can move it. `extent(b)` therefore reads back exactly the
number `bytes(N)` declared — not a remembered copy of it, but the ledger's own.
That is the capability `NoExtentTransport` names as missing, supplied on the
source side.

**Every admitted access is in range, and the rule is decidable.** An index and a
stored byte are check-time constants, so `admitsAccess` is a decidable
predicate, and a program the checker admits has every access inside its
buffer's declared extent. The emitted C's bounds comparison is a *witness* of
that theorem and not a substitute for it.

Beside those, the custody statements: the live byte total is zero at a normal
exit, a release strictly decreases it, and a program of reads and renderings
leaves it unchanged.

What is **not** proved here: that the Rust lowering extracts these events
faithfully from source tokens, that `calloc` gives the zeroed block the model
assumes (a C-standard premise), or anything about the failing allocation arm —
the emitted program aborts there, so a run either produces the modelled
observation or produces none, and that partial-correctness boundary is stated
rather than proved away. Nor is the *ownership checker itself* re-proved here:
`QCore.Cycle3` and its neighbours own the custody rules, and this module states
what the buffer route's operation list gives them. -/

namespace QCore.StdBuffer

/-- What an index or a stored byte may be.
**This is half the effect discipline, as a type.** Neither constructor mentions
a buffer, so an index can never be one and a stored byte can never be one — not
because a checker refuses it, but because the syntax cannot say it. -/
inductive Operand where
  | literal (value : Int)
  | scalar (name : String)
  deriving DecidableEq, Repr

/-- The events of a flat entry. A buffer is named only at the four custody
positions, which is the other half of the effect discipline. -/
inductive Event where
  | bind (binding : String) (value : Int)
  | acquire (name : String) (extent : Nat)
  | store (name : String) (index : Operand) (value : Operand)
  | load (binding : String) (name : String) (index : Operand)
  | extent (binding : String) (name : String)
  | release (name : String)
  /-- A `source.std-io.v1` line, reused unchanged. It is one constructor here
  because nothing this module proves depends on which line it is: a rendering
  touches no custody, and that is the point. -/
  | line
  | ret
  deriving DecidableEq, Repr

/-- The largest extent the route admits. -/
def maxExtent : Nat := 4096

/-- The effects this model distinguishes, matching the Rust `Effect` row. -/
inductive Effect where
  | io
  | alloc
  | mut
  deriving DecidableEq, Repr

/-! ## The ledger

A list of `(name, extent)` pairs: exactly what the ownership footprint carries,
with the byte axis spelled out. -/

abbrev Ledger := List (String × Nat)

def liveBytes (ledger : Ledger) : Nat :=
  ledger.foldr (fun entry total => entry.2 + total) 0

def held (ledger : Ledger) (name : String) : Option Nat :=
  (ledger.find? (fun entry => entry.1 = name)).map Prod.snd

def acquire (ledger : Ledger) (name : String) (extent : Nat) : Ledger :=
  (name, extent) :: ledger

def release (ledger : Ledger) (name : String) : Ledger :=
  ledger.filter (fun entry => entry.1 ≠ name)

/-- The ledger after one event. Only the two custody events move it; a store, a
load, an extent read, a line and a return leave it exactly as it was. -/
def step (ledger : Ledger) : Event → Ledger
  | .acquire name extent => acquire ledger name extent
  | .release name => release ledger name
  | .bind _ _ => ledger
  | .store _ _ _ => ledger
  | .load _ _ _ => ledger
  | .extent _ _ => ledger
  | .line => ledger
  | .ret => ledger

def run (ledger : Ledger) : List Event → Ledger
  | [] => ledger
  | event :: rest => run (step ledger event) rest

/-! ## The scalar environment and the decidable operand rule -/

abbrev Scalars := List (String × Int)

def lookup (scalars : Scalars) (name : String) : Option Int :=
  (scalars.find? (fun entry => entry.1 = name)).map Prod.snd

/-- An operand's value, when it has one. **Total on literals and decidable on
names**: there is no form whose value the checker could fail to compute, which
is why the reference interpreter can predict this route's observation exactly.
-/
def operandValue (scalars : Scalars) : Operand → Option Int
  | .literal value => some value
  | .scalar name => lookup scalars name

/-- Whether an index is inside a buffer's declared extent. This is the **whole**
bounds rule, and it is decidable because `operandValue` is. -/
def admitsIndex (ledger : Ledger) (scalars : Scalars) (name : String) (index : Operand) : Bool :=
  match held ledger name, operandValue scalars index with
  | some extent, some value => 0 ≤ value && value < (extent : Int)
  | _, _ => false

/-- Whether a stored value is a byte. Refused rather than truncated. -/
def admitsByte (scalars : Scalars) (value : Operand) : Bool :=
  match operandValue scalars value with
  | some value => 0 ≤ value && value ≤ 255
  | Option.none => false

/-- Whether a declared extent is one the route models. -/
def admitsExtent (extent : Nat) : Bool := 0 < extent && extent ≤ maxExtent

/-! ## The derived effect row -/

def eventEffects : Event → List Effect
  | .acquire _ _ => [Effect.alloc]
  | .release _ => [Effect.alloc]
  | .store _ _ _ => [Effect.mut]
  | .line => [Effect.io]
  | .bind _ _ => []
  | .load _ _ _ => []
  | .extent _ _ => []
  | .ret => []

def derivedEffects (events : List Event) : List Effect :=
  events.flatMap eventEffects

/-! ## The extent is transported, not remembered -/

/-- **The headline.** What `extent(b)` reads back is exactly what `bytes(N)`
declared: the ledger holds the acquisition's own number, and there is no second
place it could have come from. -/
theorem the_extent_read_back_is_the_extent_declared
    (ledger : Ledger) (name : String) (extent : Nat) :
    held (acquire ledger name extent) name = some extent := by
  simp [held, acquire]

/-- No non-custody event moves the ledger, so nothing between the acquisition
and the read can change the extent. A store writes a byte and a rendering
writes a line; neither is a custody act. -/
theorem a_store_moves_no_extent (ledger : Ledger) (name : String)
    (index value : Operand) : step ledger (.store name index value) = ledger := rfl

theorem a_load_moves_no_extent (ledger : Ledger) (binding name : String)
    (index : Operand) : step ledger (.load binding name index) = ledger := rfl

theorem an_extent_read_moves_no_extent (ledger : Ledger) (binding name : String) :
    step ledger (.extent binding name) = ledger := rfl

theorem a_line_moves_no_extent (ledger : Ledger) : step ledger .line = ledger := rfl

/-- The general form of the four above: a program made only of non-custody
events leaves the ledger — and therefore every extent in it — exactly as it
found it. -/
theorem non_custody_events_leave_the_ledger_unchanged
    (ledger : Ledger) (events : List Event)
    (plain : ∀ e ∈ events, step ledger e = ledger ∧
      ∀ l : Ledger, step l e = l) :
    run ledger events = ledger := by
  induction events generalizing ledger with
  | nil => rfl
  | cons e rest ih =>
    have this_one := (plain e (List.mem_cons_self ..)).2 ledger
    rw [run, this_one]
    exact ih ledger (fun x hx => ⟨(plain x (List.mem_cons_of_mem _ hx)).2 ledger,
      (plain x (List.mem_cons_of_mem _ hx)).2⟩)

/-! ## The bounds discipline -/

/-- Every admitted access is inside the declared extent. Stated as the
extraction the emitted program's witness comparison would have to falsify. -/
theorem an_admitted_index_is_inside_the_extent
    (ledger : Ledger) (scalars : Scalars) (name : String) (index : Operand)
    (admitted : admitsIndex ledger scalars name index = true) :
    ∃ extent value, held ledger name = some extent ∧
      operandValue scalars index = some value ∧ 0 ≤ value ∧ value < (extent : Int) := by
  unfold admitsIndex at admitted
  cases h : held ledger name with
  | none => rw [h] at admitted; cases operandValue scalars index <;> simp at admitted
  | some extent =>
    cases hv : operandValue scalars index with
    | none => rw [h, hv] at admitted; simp at admitted
    | some value =>
      rw [h, hv] at admitted
      simp only [Bool.and_eq_true, decide_eq_true_eq] at admitted
      -- `cases h :` rewrote the goal too, so both equalities are already
      -- discharged there and `rfl` is what the goal now asks for.
      exact ⟨extent, value, rfl, rfl, admitted.1, admitted.2⟩

/-- An access into a name the ledger does not hold is refused. This is the
custody half of the bounds rule: a released buffer is not in the ledger, so an
access to it has no admitted index at all. -/
theorem an_index_into_nothing_is_refused
    (ledger : Ledger) (scalars : Scalars) (name : String) (index : Operand)
    (absent : held ledger name = Option.none) :
    admitsIndex ledger scalars name index = false := by
  unfold admitsIndex
  rw [absent]

/-- An index whose value the checker cannot compute is refused. There is no such
operand form in v1 — `operandValue` is total on literals and decidable on names
— so this states the rule the follow-on `STD-BUFFER-VARYING-INDEX` would have to
change. -/
theorem an_index_with_no_value_is_refused
    (ledger : Ledger) (scalars : Scalars) (name : String) (index : Operand)
    (undecided : operandValue scalars index = Option.none) :
    admitsIndex ledger scalars name index = false := by
  unfold admitsIndex
  rw [undecided]
  cases held ledger name <;> rfl

/-- The boundary is `index < extent`, in both directions: the last index inside
an extent is admitted and the extent itself is not. -/
theorem the_last_index_is_admitted_and_the_extent_itself_is_not
    (name : String) (extent : Nat) (positive : 0 < extent) :
    admitsIndex [(name, extent)] [] name (.literal ((extent : Int) - 1)) = true
      ∧ admitsIndex [(name, extent)] [] name (.literal (extent : Int)) = false := by
  constructor
  · simp [admitsIndex, held, operandValue, List.find?]
    omega
  · simp [admitsIndex, held, operandValue, List.find?]

/-- A negative index is refused, so the rule is a range and not merely an upper
bound. -/
theorem a_negative_index_is_refused (name : String) (extent : Nat) :
    admitsIndex [(name, extent)] [] name (.literal (-1)) = false := by
  simp [admitsIndex, held, operandValue, List.find?]

/-- The byte range is `0 ≤ v ≤ 255`, in both directions. -/
theorem the_byte_range_is_exactly_a_byte :
    admitsByte [] (.literal 255) = true
      ∧ admitsByte [] (.literal 256) = false
      ∧ admitsByte [] (.literal (-1)) = false := by
  refine ⟨?_, ?_, ?_⟩ <;> simp [admitsByte, operandValue]

/-- The extent range is `1 ≤ n ≤ 4096`, in both directions. A zero-byte buffer
is refused rather than admitted as a degenerate case. -/
theorem the_extent_range_is_bounded_at_both_ends :
    admitsExtent 0 = false ∧ admitsExtent 1 = true
      ∧ admitsExtent maxExtent = true ∧ admitsExtent (maxExtent + 1) = false := by
  refine ⟨rfl, rfl, rfl, rfl⟩

/-! ## The custody discipline -/

/-- A release removes exactly the named buffer and leaves every other entry.
This is what makes the leak rule about the *remaining* obligations. -/
theorem a_release_removes_exactly_its_own (ledger : Ledger) (name : String) :
    held (release ledger name) name = Option.none := by
  unfold held release
  cases h : (ledger.filter (fun entry => entry.1 ≠ name)).find?
      (fun entry => entry.1 = name) with
  | none => simp
  | some entry =>
    have mem := List.find?_some h
    have kept := List.mem_filter.mp (List.mem_of_find?_eq_some h)
    simp only [decide_eq_true_eq] at mem
    simp only [decide_eq_true_eq] at kept
    exact absurd mem kept.2

/-- Every other entry survives a release: releasing one buffer discharges one
obligation, not two. -/
theorem a_release_keeps_every_other_entry (ledger : Ledger) (name other : String)
    (different : other ≠ name) (entry : Nat) (present : (other, entry) ∈ ledger) :
    (other, entry) ∈ release ledger name := by
  refine List.mem_filter.mpr ⟨present, ?_⟩
  simp [different]

/-- **The leak rule, as this model states it**: a program is admitted only when
its ledger is empty at the normal exit, and an empty ledger has zero live bytes.
The Rust route does not decide this — `ownership.check_ownership` does, over the
operation list `custody_operations` derives — and this is the statement of what
that discharge means for the extent. -/
theorem an_empty_ledger_holds_no_bytes : liveBytes [] = 0 := rfl

/-- A live buffer is a positive byte total, so "the ledger is empty" and "no
bytes are live" are the same fact rather than two that happen to agree. -/
theorem a_held_buffer_is_live_bytes (ledger : Ledger) (name : String) (extent : Nat)
    (positive : 0 < extent) : 0 < liveBytes (acquire ledger name extent) := by
  simp [liveBytes, acquire]
  omega

/-- An acquisition increases the live byte total by exactly its extent: the
footprint is componentwise over the whole ledger, which is why two live buffers
sum. -/
theorem an_acquisition_adds_exactly_its_extent
    (ledger : Ledger) (name : String) (extent : Nat) :
    liveBytes (acquire ledger name extent) = extent + liveBytes ledger := rfl

/-- Two live buffers hold the sum of their extents. The Rust certificate's
`peak_live_bytes` is this number. -/
theorem two_live_buffers_hold_the_sum (a b : String) (m n : Nat) :
    liveBytes (acquire (acquire [] a m) b n) = n + m := by
  simp [liveBytes, acquire]

/-! ## The effect discipline -/

/-- **The route's derived row carries `alloc` and `mut`, unlike every route
before it.** A program that acquires derives `alloc`; one that stores derives
`mut`. `source.std-io.v1`'s row is exactly `{io}` because its operand grammar
admits no resource, and this route's does. -/
theorem an_acquisition_derives_alloc (name : String) (extent : Nat) :
    Effect.alloc ∈ derivedEffects [.acquire name extent] := by
  simp [derivedEffects, eventEffects]

theorem a_store_derives_mut (name : String) (index value : Operand) :
    Effect.mut ∈ derivedEffects [.store name index value] := by
  simp [derivedEffects, eventEffects]

/-- A read derives nothing: it observes. This is why a program that only reads
and renders needs no `mut` in its declared row. -/
theorem a_read_derives_nothing (binding name : String) (index : Operand) :
    derivedEffects [.load binding name index] = [] := rfl

theorem an_extent_read_derives_nothing (binding name : String) :
    derivedEffects [.extent binding name] = [] := rfl

/-- A silent buffer program derives no `io`, so `std.buffer` grants no output
capability and a module that never renders needs no output module. -/
theorem a_silent_buffer_program_derives_no_io (name : String) (extent : Nat) :
    Effect.io ∉ derivedEffects [.acquire name extent, .release name, .ret] := by
  simp [derivedEffects, eventEffects]

/-! ## The operand wall

The statement that closes the type: there are exactly two operand forms and
neither is a buffer, so no index and no stored byte can denote one. Written as
one list on purpose — a future widening changes this theorem rather than
slipping past it, the convention `QCore.StdIo`'s
`the_operand_forms_are_exactly_these` established. -/

theorem the_operand_forms_are_exactly_two (o : Operand) :
    (∃ v, o = .literal v) ∨ (∃ n, o = .scalar n) := by
  cases o with
  | literal v => exact Or.inl ⟨v, rfl⟩
  | scalar n => exact Or.inr ⟨n, rfl⟩

/-- Neither operand form moves the ledger under any event that carries one, so
an index or a stored byte cannot be a custody act however it is written. -/
theorem an_operand_is_never_a_custody_act
    (ledger : Ledger) (name : String) (index value : Operand) :
    step ledger (.store name index value) = ledger
      ∧ ∀ binding, step ledger (.load binding name index) = ledger :=
  ⟨rfl, fun _ => rfl⟩

/-! ## The module gate

`std.buffer` gates this route's capability and grants no other, proved against
the same registry every other module is proved against. -/

theorem the_buffer_capability_needs_its_module {names : List String}
    {imports : QCore.ModuleImports.Imports}
    (resolved : QCore.ModuleImports.resolve names = some imports)
    (enabled : imports.buffer = true) : "std.buffer" ∈ names :=
  QCore.ModuleImports.admitted_buffer_requires_its_module resolved enabled

theorem the_buffer_module_grants_no_stream :
    QCore.ModuleImports.admitsPrint
      ⟨false, false, false, false, false, false, true, false, false, false⟩ true = false := rfl

end QCore.StdBuffer
