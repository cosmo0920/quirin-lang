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

import QCore.StdBuffer

/-! `source.loop.v1` — the checked bounded counted loop (card `LOOP-V1`).

This module proves the three claims the Rust construct rests on.

**Unrolling is sound, and it is what the checker does.** A loop's bound is a
number the checker holds, so the flat event list the *inherited* cycle-3
ownership checker is handed is the body's events repeated `count` times. The
theorem `running_the_unrolling_is_iterating_the_body` says that list has exactly
the effect of running the body `count` times — so the flat list the checker
decides and the program a reader wrote are the same program, and nothing is
widened in between. That is the whole justification for choosing exact
unrolling over a fixed point: on a route whose evidence is a byte comparison
between a reference execution and a native run, an over-approximation of the
loop's effect is an observation the reference cannot predict.

**A body's net custody effect must be the identity, and one pass decides it.**
`an_identity_body_iterates` lifts one pass's identity to every bound, and
`a_body_that_only_acquires_grows_the_ledger` is its non-vacuous converse: the
acquiring body is exactly the shape a leak is made of, and it moves the ledger
at a bound of one.

**A zero bound checks nothing, which is why it is refused.** `unroll` of a
zero-bound loop is the empty list, so every statement in its body would be
admitted unexamined. Refusing it is the fail-closed reading of that fact, and
`a_zero_bound_unrolls_to_nothing` is the fact.

What is **not** proved here: that the Rust lowering extracts these items
faithfully from source tokens (a tested Rust boundary, the `ConcurrencyPrelude`
/ `PipeDesugaring` / `BranchJoin` precedent); that a C compiler's `for` runs the
body the same number of times the model does (that is what the sanitized native
byte comparison cross-checks); and anything about a bound the checker cannot
compute, which is absent from this model by construction — `count` is a `Nat`.
-/

namespace QCore.SourceLoop

open QCore.StdBuffer

/-- A statement of a route that admits this construct: one of that route's own
events, or a loop whose body is a list of the same.

**The body's items are the route's items**, which is the modelled form of the
Rust statement that a loop body's statements are the admitting route's
statements, read by that route's own reader. -/
inductive Item where
  | event (e : Event)
  | loop (index : String) (count : Nat) (body : List Item)
  deriving Repr

/-- The largest bound the construct admits, and the largest unrolled event total
one entry may present. Both are in the model because both are refusals the
construct owns, not implementation limits hidden behind it. -/
def maxIterations : Nat := 4096

def maxUnrolledEvents : Nat := 8192

/-- `n` copies of `xs`, concatenated. -/
def repeatAppend (count : Nat) (xs : List Event) : List Event :=
  match count with
  | 0 => []
  | n + 1 => xs ++ repeatAppend n xs

mutual
  /-- The flat event list one item presents to the inherited checker. -/
  def unroll : Item → List Event
    | .event e => [e]
    | .loop _ count body => repeatAppend count (unrollList body)

  /-- The flat event list a sequence of items presents. -/
  def unrollList : List Item → List Event
    | [] => []
    | item :: rest => unroll item ++ unrollList rest
end

/-! ## Running a body, iteratively

Defined **independently of `unroll`**, so the agreement theorem below is a
theorem rather than a restatement. -/

/-- Apply `f` `count` times, left to right. -/
def iterate (f : Ledger → Ledger) : Nat → Ledger → Ledger
  | 0, ledger => ledger
  | n + 1, ledger => iterate f n (f ledger)

/-! ## Unrolling is sound -/

theorem run_append (ledger : Ledger) (xs ys : List Event) :
    run ledger (xs ++ ys) = run (run ledger xs) ys := by
  induction xs generalizing ledger with
  | nil => rfl
  | cons x rest ih => simpa [run] using ih (step ledger x)

/-- **The headline.** Running the unrolled list is running the body `count`
times: the flat list the checker decides and the loop a reader wrote have the
same effect, exactly, with nothing approximated between them. -/
theorem running_the_unrolling_is_iterating_the_body
    (ledger : Ledger) (count : Nat) (body : List Event) :
    run ledger (repeatAppend count body) = iterate (fun l => run l body) count ledger := by
  induction count generalizing ledger with
  | zero => rfl
  | succ n ih => simpa [repeatAppend, iterate, run_append] using ih (run ledger body)

/-- A loop's unrolling is its body's unrolling, `count` times over — the
definitional half of the claim above, stated so the two can be cited
separately. -/
theorem a_loop_unrolls_to_its_body_repeated (index : String) (count : Nat)
    (body : List Item) :
    unroll (.loop index count body) = repeatAppend count (unrollList body) := rfl

/-- **Nesting costs nothing new.** An inner loop is unrolled by the same
function, so a nest is checked by the same rule at every depth rather than by a
second one. -/
theorem a_nested_loop_unrolls_by_the_same_rule
    (outer inner : String) (m n : Nat) (body : List Item) :
    unroll (.loop outer m [.loop inner n body]) =
      repeatAppend m (repeatAppend n (unrollList body) ++ []) := by
  simp [unroll, unrollList]

/-! ## The zero bound -/

/-- A zero-bound loop unrolls to nothing, so **no statement of its body is ever
examined**. That is why the construct refuses a bound of zero rather than
admitting a loop with an unchecked body: admitting it would be the one way a
statement could enter a program without passing the checker. -/
theorem a_zero_bound_unrolls_to_nothing (index : String) (body : List Item) :
    unroll (.loop index 0 body) = [] := rfl

/-- And so it moves nothing, whatever the body says. -/
theorem a_zero_bound_runs_no_event (ledger : Ledger) (body : List Event) :
    run ledger (repeatAppend 0 body) = ledger := rfl

/-- The positive control: a bound of one runs the body exactly once, so the
refusal above is about zero and not about small numbers. -/
theorem a_bound_of_one_runs_the_body_once (ledger : Ledger) (body : List Event) :
    run ledger (repeatAppend 1 body) = run ledger body := by
  simp [repeatAppend]

/-! ## Per-iteration custody identity -/

/-- The custody facts one iteration must not move: the whole ledger, which
carries every owner and every extent.

The Rust gate compares owner names, resource types and footprints rather than
resource *identities*, and this ledger is that comparison: an entry is a name
and an extent, and two acquisitions of one extent under one name are the same
entry. That is deliberate — a body that releases a buffer and acquires an
equivalent one *is* the identity on custody. -/
def isIdentity (body : List Event) : Prop := ∀ ledger : Ledger, run ledger body = ledger

/-- **One pass decides every bound.** If a body leaves the ledger where it found
it, so does any number of passes through it — which is what makes the gate a
statement about the body rather than about the count. -/
theorem an_identity_body_iterates (body : List Event) (identity : isIdentity body)
    (count : Nat) (ledger : Ledger) :
    run ledger (repeatAppend count body) = ledger := by
  induction count generalizing ledger with
  | zero => rfl
  | succ n ih => simpa [repeatAppend, run_append, identity ledger] using ih ledger

/-- A body of non-custody events is the identity — the positive control, and the
reason a loop of stores and lines is admitted. -/
theorem a_body_of_stores_and_lines_is_the_identity
    (body : List Event)
    (plain : ∀ e ∈ body, ∀ l : Ledger, step l e = l) :
    isIdentity body := by
  intro ledger
  induction body generalizing ledger with
  | nil => rfl
  | cons e rest ih =>
    rw [run, plain e (List.mem_cons_self ..) ledger]
    exact ih (fun x hx => plain x (List.mem_cons_of_mem _ hx)) ledger

/-- An acquire-and-release body is the identity, which is the shape the
discipline exists to admit rather than a case it happens not to refuse. -/
theorem an_acquire_then_release_body_is_the_identity
    (ledger : Ledger) (name : String) (extent : Nat)
    (fresh : ∀ entry ∈ ledger, entry.1 ≠ name) :
    run ledger [.acquire name extent, .release name] = ledger := by
  simp [run, step, acquire, release]
  intro a b mem
  exact fresh (a, b) mem

/-- **Non-vacuous, in the accusing direction.** A body that acquires and does
not release moves the ledger at a bound of one, so the gate has something to
refuse — and at every larger bound it keeps moving it. -/
theorem a_body_that_only_acquires_grows_the_ledger
    (ledger : Ledger) (name : String) (extent : Nat) (positive : 0 < extent) :
    liveBytes (run ledger [Event.acquire name extent]) > liveBytes ledger := by
  simp [run, step, acquire, liveBytes]
  omega

/-- Consequently such a body is not the identity, which is the refusal stated as
a negation rather than as an absence. -/
theorem a_body_that_only_acquires_is_not_the_identity
    (name : String) (extent : Nat) (positive : 0 < extent) :
    ¬ isIdentity [Event.acquire name extent] := by
  intro identity
  have grows := a_body_that_only_acquires_grows_the_ledger [] name extent positive
  rw [identity []] at grows
  omega

/-! ## The index -/

/-- The values the index takes, in order: `0, 1, …, count-1`. -/
def indexValues (count : Nat) : List Nat := List.range count

theorem the_index_takes_every_value_below_the_bound (count : Nat) (k : Nat) :
    k ∈ indexValues count ↔ k < count := by
  simp [indexValues]

theorem the_index_never_reaches_the_bound (count : Nat) :
    count ∉ indexValues count := by
  simp [indexValues]

/-! ## The cost, bounded rather than paid -/

/-- How many events an item unrolls to. The checking route runs the body once
per iteration, so this is the work the checker does — and it is a **product**
over a nest, which is why the budget is a product rather than a depth. -/
def unrolledLength (item : Item) : Nat := (unroll item).length

theorem repeatAppend_length (count : Nat) (xs : List Event) :
    (repeatAppend count xs).length = count * xs.length := by
  induction count with
  | zero => simp [repeatAppend]
  | succ n ih => simp [repeatAppend, ih, Nat.succ_mul, Nat.add_comm]

theorem a_loops_cost_is_its_bound_times_its_body
    (index : String) (count : Nat) (body : List Item) :
    unrolledLength (.loop index count body) = count * (unrollList body).length := by
  simp [unrolledLength, unroll, repeatAppend_length]

/-! ## Neutrality

A program that writes no loop is unchanged by this construct. -/

/-- The loop-free items, written **independently** of `unroll`: this stepper has
no loop case and refuses one outright. -/
def loopFree : Item → Bool
  | .event _ => true
  | .loop _ _ _ => false

theorem a_loop_free_item_unrolls_to_itself (e : Event) : unroll (.event e) = [e] := rfl

theorem a_loop_free_sequence_unrolls_to_itself (events : List Event) :
    unrollList (events.map Item.event) = events := by
  induction events with
  | nil => rfl
  | cons e rest ih => simp [unrollList, unroll, ih]

/-- And the claim is not vacuous: a loop is the one item the two disagree on. -/
theorem a_loop_is_the_one_item_that_is_not_loop_free
    (index : String) (count : Nat) (body : List Item) :
    loopFree (.loop index count body) = false ∧ ∀ e : Event, loopFree (.event e) = true :=
  ⟨rfl, fun _ => rfl⟩

/-! ## The refused shapes, as one list

A widening must change a theorem rather than slip past one — the convention
`QCore.StdIo.the_operand_forms_are_exactly_these` established. -/

/-- What `source.loop.v1` refuses. Each row is a decision this card made, and
each is stated so a future card that admits it has to edit this list. -/
inductive Refused where
  /-- A bound the checker cannot compute. -/
  | varyingBound
  /-- A bound of zero: its body would never be checked. -/
  | zeroBound
  /-- A bound above `maxIterations`, or an entry above `maxUnrolledEvents`. -/
  | beyondTheBudget
  /-- `break` and `continue`: an early exit needs a per-exit cleanup discharge. -/
  | earlyExit
  /-- A body whose net custody effect is not the identity. -/
  | nonIdentityBody
  /-- A body binding that shadows one the entry already made. -/
  | shadowingBinding
  /-- A range whose low end is not `0`. -/
  | otherLowEnd
  deriving DecidableEq, Repr

def refusedShapes : List Refused :=
  [.varyingBound, .zeroBound, .beyondTheBudget, .earlyExit, .nonIdentityBody,
   .shadowingBinding, .otherLowEnd]

theorem refused_shapes_are_exactly_these (r : Refused) : r ∈ refusedShapes := by
  cases r <;> simp [refusedShapes]

end QCore.SourceLoop
