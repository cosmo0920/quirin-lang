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

import QCore.ScalarBufferValues

/-! The fixed-width tier on the composed string/buffer route (card
`W48-QR-SCALAR-TYPES-R6`, `W48-QR-TYPES-COMPOSED-STRING`).

`source.string.v1` holding a buffer embeds `source.buffer.v1`'s statements and
decides one question of its own over them: the **borrow window** — whether a
store or a release, at a top-level statement or at any iteration of a loop
body, touches a byte a live `borrow_utf8` view reads. Card R5 refused the
fixed-width tier on this route by name because the emitter had no typed
environment; card R6 admits it, with the statements decided by the buffer
route's own runner. Two facts make that admission sound, and they are what
this module states.

* **The value decision is the buffer route's.** A conversion, a value
  statement and a typed loop of a composed entry are read by
  `source.buffer.v1`'s reader and decided by its runner, so their meaning is
  `QCore.ScalarBufferValues`': admitted exactly when the checked operation
  succeeds (`the_composed_statement_is_the_buffer_statement`), never raising
  at run time, and inside a loop admitted exactly when every iteration's
  operation succeeds (`a_composed_loop_statement_is_decided_per_iteration`).
* **The window is blind to the typed tier.** A typed statement stores no byte
  and releases nothing, so it never conflicts with a live loan
  (`a_typed_statement_never_conflicts`); a body's window decision is its
  typed-statement-free skeleton's (`the_window_ignores_typed_statements`), at
  every iteration of an unrolled loop
  (`an_unrolled_window_ignores_typed_statements`). So the window question a
  typed program asks is exactly one an `i32` program of the same stores
  already asked, and a store inside a live loan is refused as before
  (`a_store_inside_a_live_loan_still_conflicts`).

What is not proved here: that the Rust reader and runner are these
definitions (tested: `compiler/tests/scalar_composed_string.rs`), or anything
about native execution, which the same file compares with the reference. -/
namespace QCore.ScalarComposedString

open QCore.ScalarTypes

/-! ## The value decision -/

/-- The composed route's decision of one value statement: the buffer route's. -/
def composedBinary (t : Option Width) (op : Op) (ty : Ty) (a b : Int) : Option Int :=
  QCore.ScalarBufferValues.bufferBinary t op ty a b

theorem the_composed_statement_is_the_buffer_statement {t : Option Width} {op : Op} {ty : Ty}
    {a b v : Int} : composedBinary t op ty a b = some v ↔ checked t op ty a b = some v :=
  QCore.ScalarBufferValues.bufferBinary_iff_checked

theorem a_composed_loop_statement_is_decided_per_iteration {t : Option Width} {op : Op}
    {ty : Ty} {c : Nat} {b : Int} :
    QCore.ScalarBufferValues.unrolledAdmits t op ty c b = true ↔
      ∀ i : Nat, i < c → ∃ v, checked t op ty i b = some v :=
  QCore.ScalarBufferValues.unrolled_admits_iff

/-! ## The window -/

/-- A live `borrow_utf8` loan: the half-open byte range its view reads. -/
structure Loan where
  low : Nat
  high : Nat
  deriving DecidableEq, Repr

/-- A composed statement as the window reads it: a byte store, a release, or
a statement of the fixed-width tier (a conversion, a value statement, a
rendering of a typed name), which touches no byte. -/
inductive Stmt where
  | store (byte : Nat)
  | release
  | typed
  deriving DecidableEq, Repr

/-- Whether one statement conflicts with the live loans. -/
def conflicts (live : List Loan) : Stmt → Bool
  | .store byte => live.any fun loan => decide (loan.low ≤ byte) && decide (byte < loan.high)
  | .release => !live.isEmpty
  | .typed => false

/-- A body's window decision: some statement conflicts. -/
def bodyConflicts (live : List Loan) (body : List Stmt) : Bool :=
  body.any (conflicts live)

/-- The body with its typed statements erased: the `i32` skeleton. -/
def skeleton (body : List Stmt) : List Stmt :=
  body.filter fun statement => statement != .typed

theorem a_typed_statement_never_conflicts (live : List Loan) : conflicts live .typed = false :=
  rfl

theorem the_window_ignores_typed_statements (live : List Loan) (body : List Stmt) :
    bodyConflicts live body = bodyConflicts live (skeleton body) := by
  induction body with
  | nil => rfl
  | cons statement rest ih =>
    cases statement <;> simp_all [bodyConflicts, skeleton, conflicts]

/-- A loop's body unrolled over `count` iterations. -/
def unroll (body : List Stmt) : Nat → List Stmt
  | 0 => []
  | n + 1 => body ++ unroll body n

theorem skeleton_append (a b : List Stmt) : skeleton (a ++ b) = skeleton a ++ skeleton b := by
  simp [skeleton, List.filter_append]

theorem skeleton_unroll (body : List Stmt) :
    ∀ n, skeleton (unroll body n) = unroll (skeleton body) n := by
  intro n
  induction n with
  | zero => rfl
  | succ n ih => simp only [unroll, skeleton_append, ih]

theorem an_unrolled_window_ignores_typed_statements (live : List Loan) (body : List Stmt)
    (n : Nat) :
    bodyConflicts live (unroll body n) = bodyConflicts live (unroll (skeleton body) n) := by
  rw [the_window_ignores_typed_statements, skeleton_unroll]

/-- Non-vacuity: a store inside a live loan still conflicts, whatever typed
statements surround it. -/
theorem a_store_inside_a_live_loan_still_conflicts :
    bodyConflicts [⟨0, 2⟩] [.typed, .store 1, .typed] = true := by decide

theorem a_store_outside_every_live_loan_is_admitted :
    bodyConflicts [⟨0, 2⟩] [.typed, .store 3, .typed] = false := by decide

end QCore.ScalarComposedString
