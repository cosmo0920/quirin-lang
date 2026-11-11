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

import QCore.OpCallbacks

/-!
# A store of a formal that is not a function pointer opens the member

Card **LOG-PRINT-BODY-CONTRACTS** (gateway rule 12n, a fail-closed repair).
Rule 12n's member closure forwards a formal's values only when the formal is
a function pointer: its values are then exactly the function-pointer actuals
its callers pass, which the facts record. A formal of another type (a
`void *`, spelled `void (*)` by Clang when the declarator was `void (*f)`)
can carry any value its callers hand it, and its calls record no actual, so
a store of it into a member opens the member.

The model: a formal's type class, the flow a store of it contributes, and a
unit's reading. What it shows:

* only a function-pointer formal forwards; any other formal's store is open
  (`a_non_function_formal_store_is_open`);
* a unit holding an open store reads the member open, wherever it stands in
  the run, so the run's closure is open (`an_open_unit_keeps_the_member_open`)
  and the invocation is refused (`a_non_function_formal_store_never_decides`);
* reading a non-function formal as a function pointer (the defect) could
  close a member over no candidate at all, which decides as the identity: the
  refusal is the direction that never reads a callback as never invoked
  (`the_defect_would_close_the_member_empty`).

What this is not: a model of Clang's type spellings. Which spelling is a
function pointer (`R (*)(P...)`, a parameter list after `(*)`) is the
reading's own, a rule-28 extraction premise; `tests/log_print_member_contracts.rs`
is its control.
-/

namespace QCore.MemberFormalType

open QCore.OpCallbacks (Closure UnitReading runClosure memberDecision)
open QCore.OpCallbacks (SuppliedClass)
open QCore.CallbackContracts (Decision)

/-- A formal's type, as the reading classifies it. -/
inductive FormalType where
  /-- `R (*)(P...)`: a pointer to a function. -/
  | functionPointer
  /-- Anything else: `void *` (however spelled), an array pointer, a scalar. -/
  | other
  deriving DecidableEq, Repr

/-- What a store of the formal contributes to the member: its callers'
function-pointer actuals, or an open set. -/
def formalStore (t : FormalType) (actuals : List String) : Closure :=
  match t with
  | .functionPointer => .closed actuals
  | .other => .opened

/-- **Only a function-pointer formal forwards; any other formal's store is
open.** -/
theorem a_non_function_formal_store_is_open (actuals : List String) :
    formalStore .other actuals = .opened := rfl

/-- **A unit reading the member open, anywhere in the run, keeps the run's
closure open.** -/
theorem an_open_unit_keeps_the_member_open {us : List UnitReading}
    (h : some .opened ∈ us) : runClosure us = .opened := by
  induction us with
  | nil => cases h
  | cons u rest ih =>
    rcases List.mem_cons.mp h with he | hr
    · subst he; rfl
    · rcases u with _ | (ns | _)
      · rfl
      · simp [runClosure, ih hr]
      · rfl

/-- **A store of a formal that is not a function pointer never decides**:
whatever the other units read, the invocation is refused. -/
theorem a_non_function_formal_store_never_decides (actuals : List String)
    {us : List UnitReading} (h : some (formalStore .other actuals) ∈ us)
    (classOf : String → SuppliedClass) :
    memberDecision (runClosure us) classOf = .refused := by
  rw [an_open_unit_keeps_the_member_open h]
  rfl

/-- **The defect closed the member empty.** Read as a function pointer, a
`void *` formal whose calls record no function-pointer actual contributes
the empty closed set; a run of such units closes the member empty, and the
invocation decides as the identity: a callback read as never invoked. -/
theorem the_defect_would_close_the_member_empty (classOf : String → SuppliedClass) :
    memberDecision (runClosure [some (formalStore .functionPointer [])]) classOf =
      memberDecision (.closed []) classOf := rfl

end QCore.MemberFormalType
