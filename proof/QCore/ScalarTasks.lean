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
import QCore.ScalarTypedErrors
import QCore.ScalarPositions
import QCore.Transferability

/-! Fixed-width scalars across a task boundary (card `W48-QR-SCALAR-TYPES-R4`).

Model of the task route's fixed-width positions in
`qcore_core::task_execution`: `copy` captures and worker parameters and
results of every type of the table, the value statement a worker performs on
what it received, the typed `ArithmeticError` it raises, and the join that
hands the worker's outcome to its creator.

What is established:

* **Captures.** A `copy` capture is admitted exactly at the worker parameter's
  own type — there is no implicit conversion at the boundary — and hands the
  worker the very number the creator held (`capture_admitted_iff`,
  `capture_preserves_value`). The new transfer kind is duplicated and crosses
  by `copy` only (`fixedScalar_crosses_only_by_copy`).
* **The worker's statement.** A captured operand is known only at run time in
  the worker, so a statement over it is decided by the shared decision of
  `QCore.ScalarPositions` as a raising one: admitted under the row's
  permission and refused without it (`captured_operand_raises_under_permission`,
  `captured_operand_refused_without_permission`); the raised variant is one the
  error edge carries (`raised_variant_is_raisable`,
  `conversion_raise_is_raisable`).
* **Join.** The ledger's completion store hands a completed worker's outcome
  to the joining creator unchanged and consumes it (`join_returns_the_completion`,
  `join_consumes_the_completion`, `join_leaves_other_tasks`).
* **End to end.** **A worker's typed error reaches the creator unchanged
  through join** — `?` raises that very variant and `match` runs the `Err` arm
  with it (`worker_error_reaches_creator_unchanged`,
  `worker_conversion_error_reaches_creator_unchanged`) — and a worker's value
  reaches it as a well-typed, exact value (`worker_value_reaches_creator`).
* `taskAgreement_holds`: the task vectors, replayed through the Rust reference
  execution by `compiler/tests/scalar_tasks.rs`.

What is not: the Rust checker's own abstract interpretation, the emitted C's
refinement of the reference run, or the C11 thread and memory model. The
binding to Rust is the vector table and the native boundary tables, not an
extraction theorem. -/

namespace QCore.ScalarTasks

open QCore.ScalarTypes
open QCore.ScalarTypedErrors
open QCore.ScalarPositions

/-! ### Captures -/

/-- A `copy` capture of the creator's value `v` into a worker parameter of type
`param`: admitted only at the parameter's own type, and then the value itself. -/
def capture (param : Ty) (v : TVal) : Option TVal :=
  if v.ty = param then some v else none

theorem capture_admitted_iff {param : Ty} {v w : TVal} :
    capture param v = some w ↔ v.ty = param ∧ w = v := by
  unfold capture
  split <;> simp_all [eq_comm]

/-- A capture never converts: a value of another type is refused. -/
theorem capture_refuses_another_type {param : Ty} {v : TVal} (h : v.ty ≠ param) :
    capture param v = none := by
  simp [capture, h]

/-- The worker receives the creator's number, at the parameter's type. -/
theorem capture_preserves_value {param : Ty} {v w : TVal} (h : capture param v = some w) :
    w.n = v.n ∧ w.ty = param := by
  obtain ⟨hty, rfl⟩ := capture_admitted_iff.mp h
  exact ⟨rfl, hty⟩

/-- The fixed-width kind is on `i32`'s footing: duplicated, and admitted by a
`copy` capture and by nothing else. -/
theorem fixedScalar_crosses_only_by_copy :
    Transferability.transferable .fixedScalar = .duplicated ∧
      Transferability.admits .fixedScalar .copy = some .duplicate ∧
      ∀ m ∈ Transferability.allModes, m ≠ .copy →
        Transferability.admits .fixedScalar m = none := by
  refine ⟨rfl, rfl, ?_⟩
  decide

/-! ### The worker's value statement -/

/-- A captured operand is known only at run time in the worker: under the
row's `throws<ArithmeticError>` a statement over it is admitted as one that can
raise. -/
theorem captured_operand_raises_under_permission (t : Option Width) (op : Op) (ty : Ty)
    (y : Known) (hty : ty ≠ .bool) :
    decideBinary t true op ty .dynamic y = some (.dynamic, true) := by
  cases y <;> simp [decideBinary, hty]

/-- Without the permission it is refused, whatever the other operand. -/
theorem captured_operand_refused_without_permission (t : Option Width) (op : Op) (ty : Ty)
    (y : Known) : decideBinary t false op ty .dynamic y = none :=
  decideBinary_without_permission

/-- The variants a raising arithmetic statement's error edge carries: a zero
divisor or an overflow for `/` and `%`, an overflow otherwise. This is the
Rust `scalar_values::raisable` for arithmetic. -/
def raisable : Op → List ArithError
  | .div | .rem => [.divisionByZero, .overflow]
  | _ => [.overflow]

/-- Every variant a worker's arithmetic raises is one its error edge carries. -/
theorem raised_variant_is_raisable {t : Option Width} {op : Op} {ty : Ty} {a b : Int}
    {e : ArithError} (h : checkedE t op ty a b = .error e) : e ∈ raisable op := by
  rw [checkedE_raises_classified h]
  cases op <;> simp only [classify, raisable] <;> (try split) <;> simp

/-- A conversion's raise is `outOfRange`, the one variant its edge carries. -/
theorem conversion_raise_is_raisable {t : Option Width} {src dst : Ty} {v : Int}
    {e : ArithError} (h : convertE t src dst v = .error e) : e ∈ [ArithError.outOfRange] := by
  rw [convertE_raises_out_of_range h]
  simp

/-! ### The completion store and join -/

/-- A worker's completion as its creator collects it: a value of the worker's
declared result type, or the typed error it raised. -/
inductive Outcome where
  | value (v : TVal)
  | raised (e : ArithError)
  deriving DecidableEq, Repr

/-- The completion capsules the ledger holds, by task. -/
abbrev Capsules := Nat → Option Outcome

/-- A worker completes: its outcome is stored under its task. -/
def complete (c : Capsules) (task : Nat) (o : Outcome) : Capsules :=
  fun k => if k = task then some o else c k

/-- The creator joins a task: it receives the stored outcome, and the capsule
is consumed. A task with no completion cannot be joined. -/
def join (c : Capsules) (task : Nat) : Option (Outcome × Capsules) :=
  (c task).map fun o => (o, fun k => if k = task then none else c k)

theorem join_returns_the_completion (c : Capsules) (task : Nat) (o : Outcome) :
    (join (complete c task o) task).map Prod.fst = some o := by
  simp [join, complete]

/-- A second join of the same task finds nothing: the outcome is collected
exactly once. -/
theorem join_consumes_the_completion {c r : Capsules} {task : Nat} {o p : Outcome}
    (h : join (complete c task o) task = some (p, r)) : r task = none := by
  simp only [join, complete, ite_true, Option.map_some] at h
  cases h
  simp

/-- Joining one task leaves every other task's capsule as it was. -/
theorem join_leaves_other_tasks {c r : Capsules} {task other : Nat} {p : Outcome}
    (h : join c task = some (p, r)) (hne : other ≠ task) : r other = c other := by
  unfold join at h
  cases hc : c task with
  | none => simp [hc] at h
  | some q =>
    simp only [hc, Option.map_some, Option.some.injEq, Prod.mk.injEq] at h
    rw [← h.2]
    simp [hne]

/-- A worker performing `a OP b` at `ty` on its captured operands. -/
def workerBinary (t : Option Width) (op : Op) (ty : Ty) (a b : Int) : Outcome :=
  match checkedE t op ty a b with
  | .ok v => .value ⟨ty, v⟩
  | .error e => .raised e

/-- A worker performing `a as dst` on its captured operand. -/
def workerConvert (t : Option Width) (src dst : Ty) (a : Int) : Outcome :=
  match convertE t src dst a with
  | .ok v => .value ⟨dst, v⟩
  | .error e => .raised e

/-- The collected outcome as the creator's `Result`. -/
def toResult : Outcome → Except ArithError Int
  | .value v => .ok v.n
  | .raised e => .error e

/-- **A worker's typed error reaches its creator unchanged through join**: the
joined outcome is the very variant the worker raised; `?` raises it out of the
creator; `match` runs the `Err` arm with it. -/
theorem worker_error_reaches_creator_unchanged {t : Option Width} {op : Op} {ty : Ty}
    {a b : Int} {e : ArithError} (h : checkedE t op ty a b = .error e) (c : Capsules)
    (task : Nat) :
    (join (complete c task (workerBinary t op ty a b)) task).map
        (fun p => toResult p.1) = some (.error e) ∧
      (∀ {α : Type} (rest : Int → Except ArithError α),
        propagate (toResult (.raised e)) rest = .error e) ∧
      (∀ {β : Type} (ok : Int → β) (err : ArithError → β),
        handle (toResult (.raised e)) ok err = err e) := by
  refine ⟨?_, fun rest => propagate_raise e rest, fun ok err => handle_raise e ok err⟩
  simp [join, complete, workerBinary, h, toResult]

/-- The same for a worker's conversion: `outOfRange` reaches the creator. -/
theorem worker_conversion_error_reaches_creator_unchanged {t : Option Width} {src dst : Ty}
    {a : Int} {e : ArithError} (h : convertE t src dst a = .error e) (c : Capsules)
    (task : Nat) :
    (join (complete c task (workerConvert t src dst a)) task).map
        (fun p => toResult p.1) = some (.error .outOfRange) := by
  rw [convertE_raises_out_of_range h] at h
  simp [join, complete, workerConvert, h, toResult]

/-- A worker's value reaches its creator at the worker's result type, a value
of that type, and the mathematically exact result. -/
theorem worker_value_reaches_creator {t : Option Width} {op : Op} {ty : Ty} {a b v : Int}
    (h : checkedE t op ty a b = .ok v) (c : Capsules) (task : Nat) :
    (join (complete c task (workerBinary t op ty a b)) task).map Prod.fst =
        some (.value ⟨ty, v⟩) ∧
      fits t ty v = true ∧ v = exact op a b := by
  refine ⟨?_, checkedE_in_range h, checkedE_exact h⟩
  simp [join, complete, workerBinary, h]

/-! ### Vectors -/

/-- The task vectors: `(operation, type, a, b, the collected outcome)` at the
64-bit target, `none` for a value and `some e` for a raise. Replayed through
the Rust reference execution by `lean_task_vectors_match_the_reference_run`. -/
def taskAgreement : List (Op × Ty × Int × Int × Except ArithError Int) := [
  (.add, .u8, 200, 55, .ok 255), (.add, .u8, 200, 56, .error .overflow),
  (.sub, .u16, 0, 1, .error .overflow), (.mul, .i64, 3037000500, 3037000500, .error .overflow),
  (.div, .i32, -2147483648, -1, .error .overflow), (.rem, .u32, 7, 0, .error .divisionByZero),
  (.div, .usize, 10, 3, .ok 3), (.add, .isize, 9223372036854775807, 1, .error .overflow),
  (.rem, .i8, -128, 0, .error .divisionByZero), (.sub, .i16, -32768, 1, .error .overflow)]

theorem taskAgreement_holds :
    taskAgreement.all (fun (op, ty, a, b, r) =>
      sameOutcome (toResult (workerBinary (some .w64) op ty a b)) r) = true := by
  decide

end QCore.ScalarTasks
