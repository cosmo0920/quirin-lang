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

import QCore.ScalarTypes

/-! Fixed-width tasks on the composed epoch route (card
`W48-QR-SCALAR-TYPES-R6`, `W48-QR-TYPES-EPOCH-TASKS`).

`source.epoch.tasks.v1` composes the unedited flat-epoch checker with the
unedited scoped-task route. Before this card the composition refused a task
half naming any fixed-width type or value statement, because its accounting had
been measured over `i32` captures only. This module states why admitting them
changes no accounting, and names the two error paths the value statement adds.

* **The derived shape reads no scalar type.** The composition derives, per
  creation, the capture modes, whether the result is an owned Buffer, its
  disposal and the creator's error edges; every byte figure is declared. A
  capture's derived clause is its mode alone (`clause_ignores_the_type`), so
  the derived shape of a spawn is the same for any typing of its copies and any
  payload type of its Buffers (`the_derived_shape_is_type_blind`), and the
  plan–ledger agreement is decided exactly as for the `i32` program it erases
  to (`agreement_is_type_blind`).
* **A worker's `ArithmeticError` is a join-failure edge.** A worker that raises
  is joined by its creator through `?` or `match`, which is the edge CT23's
  recovery plan already carries (`a_raising_worker_is_a_join_failure_edge`).
* **A creator's raise abandons no credit.** A raising statement of the creator
  is an error exit of the creator's scope. The scope rule admits an exit only
  when no child of the scope is outstanding, and the children's outstanding
  credit is the sum of the grants of the outstanding children, so at every
  admitted creator exit it is zero
  (`an_admitted_creator_exit_abandons_no_child_credit`) — the fact the
  creator's allocation-failure exit already relied on.

What is not proved here: that the Rust reader derives these fields from the
checked IR faithfully (tested: `compiler/tests/scalar_epoch_tasks.rs`), that the
scope rule is `scoped_tasks`' (by inspection, as in `QCore.TaskExecution`), or
anything about execution, which is the task route's emitted C, compared with
its reference natively. -/
namespace QCore.ScalarEpochTasks

open QCore.ScalarTypes

/-- A spawn argument as the checked task IR has it: a copy of a value of some
§G02t type, a move of an owned Buffer of some payload type, or a read loan. -/
inductive Arg where
  | copy (ty : Ty)
  | move (payload : Ty)
  | loan (payload : Ty)
  deriving DecidableEq, Repr

/-- The plan's capture clause. -/
inductive Clause where
  | copy | move | read
  deriving DecidableEq, Repr

/-- The derived clause of one argument: its mode, nothing else. -/
def clause : Arg → Clause
  | .copy _ => .copy
  | .move _ => .move
  | .loan _ => .read

/-- The `i32` program a typed argument erases to. -/
def erase : Arg → Arg
  | .copy _ => .copy .i32
  | .move _ => .move .i32
  | .loan _ => .loan .i32

theorem clause_ignores_the_type (a : Arg) : clause (erase a) = clause a := by
  cases a <;> rfl

/-- The result the derivation reads: whether the worker returns an owned
Buffer, of whatever payload type. -/
inductive Result where
  | scalar (ty : Ty)
  | unit
  | buffer (payload : Ty)
  deriving DecidableEq, Repr

def retains : Result → Bool
  | .buffer _ => true
  | _ => false

def eraseResult : Result → Result
  | .scalar _ => .scalar .i32
  | .unit => .unit
  | .buffer _ => .buffer .i32

/-- One creation as the ledger derives it. -/
structure Spawn where
  worker : Nat
  args : List Arg
  result : Result
  joinFailure : Bool
  deriving DecidableEq, Repr

/-- The derived shape of one creation: what the composition compares. -/
structure Shape where
  worker : Nat
  clauses : List Clause
  retains : Bool
  joinFailure : Bool
  deriving DecidableEq, Repr

def shape (s : Spawn) : Shape :=
  ⟨s.worker, s.args.map clause, retains s.result, s.joinFailure⟩

def eraseSpawn (s : Spawn) : Spawn :=
  { s with args := s.args.map erase, result := eraseResult s.result }

theorem retains_ignores_the_type (r : Result) : retains (eraseResult r) = retains r := by
  cases r <;> rfl

/-- **The derived shape is type-blind**: a creation and its `i32` erasure
derive the same shape. -/
theorem the_derived_shape_is_type_blind (s : Spawn) : shape (eraseSpawn s) = shape s := by
  unfold shape eraseSpawn
  simp only [List.map_map]
  congr 1
  · exact List.map_congr_left (fun a _ => clause_ignores_the_type a)
  · exact retains_ignores_the_type s.result

/-- The plan–ledger agreement, as the composition decides it: the declared rows
equal the derived shapes. -/
def agrees (declared : List Shape) (ledger : List Spawn) : Bool :=
  declared == ledger.map shape

/-- **Agreement is type-blind**: a plan agrees with a typed region exactly when
it agrees with that region's `i32` erasure. -/
theorem agreement_is_type_blind (declared : List Shape) (ledger : List Spawn) :
    agrees declared (ledger.map eraseSpawn) = agrees declared ledger := by
  unfold agrees
  have same : ledger.map (shape ∘ eraseSpawn) = ledger.map shape :=
    List.map_congr_left (fun s _ => the_derived_shape_is_type_blind s)
  rw [List.map_map, same]

/-! ## The worker's raise -/

/-- How the creator consumes one joined outcome: an infallible unwrap or a
fallible elimination (`?` or `match`). -/
inductive Consumption where
  | unwrap | fallible
  deriving DecidableEq, Repr

/-- The join-failure edge the ledger derives: a fallible elimination. -/
def joinFailure : Consumption → Bool
  | .fallible => true
  | .unwrap => false

/-- A worker that can raise `ArithmeticError` has a nonempty error row, so its
outcome cannot be unwrapped infallibly: the creator consumes it fallibly, and
the derived join-failure edge is present. -/
def consumption (canRaise : Bool) : Consumption :=
  if canRaise then .fallible else .unwrap

theorem a_raising_worker_is_a_join_failure_edge :
    joinFailure (consumption true) = true := rfl

/-! ## The creator's raise -/

/-- The scope rule's exit admission: an exit of the creator's scope is admitted
only when no child is outstanding. -/
def exitAdmitted (outstanding : List Nat) : Bool := outstanding.isEmpty

/-- The children's outstanding credit: the sum of the grants of the
outstanding children. -/
def outstandingCredit (grant : Nat → Nat) (outstanding : List Nat) : Nat :=
  (outstanding.map grant).sum

/-- **An admitted creator exit abandons no child credit**, whatever the grants. -/
theorem an_admitted_creator_exit_abandons_no_child_credit (grant : Nat → Nat)
    (outstanding : List Nat) (admitted : exitAdmitted outstanding = true) :
    outstandingCredit grant outstanding = 0 := by
  unfold exitAdmitted at admitted
  cases outstanding with
  | nil => rfl
  | cons head tail => simp at admitted

/-- And an exit with a child outstanding is refused, so a raise there never
reaches the epoch at all. -/
theorem an_exit_with_a_child_outstanding_is_refused (child : Nat) (rest : List Nat) :
    exitAdmitted (child :: rest) = false := rfl

end QCore.ScalarEpochTasks
