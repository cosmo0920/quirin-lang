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

import QCore.RetentionPath

/-!
# A null pointer constant retains nothing (card RETENTION-NULL-CONSTANT-ACTUAL)

Gateway rule 12t, amended in place by card OPENSSL-UNSTATED-CALLBACK-SLOTS,
which carries the follow-up RETENTION-NULL-CONSTANT-ACTUAL; contract
specification §C04.14. The abstract rule beside `QCore.RetentionPath` (the
retention table with path holders), implemented by
`crates/qcore-c-gateway/src/retained_by.rs` (`Retained::Nothing`,
`null_pointer_constant`, `NULL_DECISION_VECTORS`) and its route-G adapter
`legacy_retained_by.rs` (`null_retention`).

A retained actual is read as a null pointer constant (after the cast reading:
`(void *) 0`, a pointer cast of one, `0`, `NULL`) or as before. A null
pointer designates no object, so nothing is retained and nothing outlives
anything: the call decides with no record, whatever the holder
(`a_null_constant_retains_nothing`, `a_null_constant_never_records`). Every
other actual is decided exactly by `QCore.RetentionPath.decide`
(`every_other_actual_is_decided_as_before`), so a local, a formal or a call
result that may be null keeps its refusal where it had one
(`a_non_static_actual_in_an_unknown_holder_still_refuses`). On every path a
null retention is the identity on the records
(`a_null_retention_is_the_identity_on_records`) and leaves the replay's
outcome unchanged wherever it stands
(`a_null_retention_never_changes_a_replay`): it can only remove the
refusal the old reading put there, never a record or a release check.

## What this is not

That the gateway's structural reading of a spelling is a null pointer
(premise: a C11 6.3.2.3p3 null pointer constant, or a pointer cast of one,
compares unequal to a pointer to any object) is a review premise pinned by
`retained_by.rs`'s `NULL_SPELLING_VECTORS` and
`crates/qcore-c-gateway/tests/retention_null_constant.rs`; nothing is
evaluated, so `(char *) (1 - 1)` is not read as one.
-/

namespace QCore.RetentionNullConstant

open QCore.RetentionPath

/-- A retained actual: a null pointer constant, or what it denotes as
before. -/
inductive Actual where
  | nullConstant
  | denotes (r : Retained)
  deriving DecidableEq, Repr

/-- Rule 12t's table with the null reading (`retained_by::decide` with
`Retained::Nothing`). -/
def decideActual : Actual → Holder → Decision
  | .nullConstant, _ => .outlives
  | .denotes r, h => QCore.RetentionPath.decide r h

theorem a_null_constant_retains_nothing (h : Holder) :
    decideActual .nullConstant h = .outlives := rfl

theorem a_null_constant_never_records (h : Holder) (o : HeldObject) (b : HeldBy) :
    decideActual .nullConstant h ≠ .record o b := by
  simp [decideActual]

theorem a_null_constant_never_refuses (h : Holder) :
    decideActual .nullConstant h ≠ .undecided := by
  simp [decideActual]

theorem every_other_actual_is_decided_as_before (r : Retained) (h : Holder) :
    decideActual (.denotes r) h = QCore.RetentionPath.decide r h := rfl

/-- The refusals the reading keeps: a non-static actual in a holder the body
cannot name, or in the `global` holder. -/
theorem a_non_static_actual_in_an_unknown_holder_still_refuses (r : Retained)
    (hr : r ≠ .static) :
    decideActual (.denotes r) .foreign = .undecided ∧
      decideActual (.denotes r) .global = .undecided := by
  cases r <;> simp_all [decideActual, QCore.RetentionPath.decide]

theorem an_unreadable_actual_still_refuses (h : Holder) :
    decideActual (.denotes .other) h = .undecided := by
  cases h <;> rfl

theorem a_null_retention_is_the_identity_on_records (rs : Records) (h : Holder) :
    step rs (.retain (decideActual .nullConstant h)) = some rs := rfl

/-- Wherever a null retention stands on a path, the replay is the replay of
the path without it. -/
theorem a_null_retention_never_changes_a_replay (pre post : List Event) (h : Holder) :
    replay (pre ++ .retain (decideActual .nullConstant h) :: post) = replay (pre ++ post) := by
  unfold replay
  rw [run_append, run_append]
  cases run [] pre with
  | none => rfl
  | some rs => simp [run, step, decideActual]

/-! ## The vectors shared with `retained_by.rs` -/

/-- `NULL_DECISION_VECTORS`: the null class in every holder class. -/
def nullDecisionVectors : List (HClass × Label) := [
  (.frame, .outlives),
  (.tracked, .outlives),
  (.global, .outlives),
  (.foreign, .outlives),
  (.path, .outlives)
]

theorem null_decision_vectors_hold :
    nullDecisionVectors.all
      (fun (h, l) => label (decideActual .nullConstant h.sample) == l) = true := by
  decide

/-- `PATH_DECISION_VECTORS` still hold through the null reading's table. -/
theorem the_path_vectors_hold_through_the_reading :
    pathDecisionVectors.all
      (fun (r, h, l) => label (decideActual (.denotes r.sample) h.sample) == l) = true := by
  decide

end QCore.RetentionNullConstant
