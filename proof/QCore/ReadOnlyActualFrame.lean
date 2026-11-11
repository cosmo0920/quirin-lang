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

import QCore.TypedRootDeclarations

/-!
The W48 read-only call-actual exception to the pointer-return root premise.
The Rust body check and exact Clang/lexical event extraction remain the
standing transcription premises. This model records why omitting only a
certified read-only actual from `may_bind` preserves every other binding
cause, and why a missing certificate refuses.
-/

namespace QCore.ReadOnlyActualFrame

/-- The three positive checks the Rust route needs at the call site. -/
structure Certificate where
  bodyChecked : Bool
  directCallee : Bool
  exactActual : Bool
  deriving DecidableEq, Repr

def Certificate.accepts (c : Certificate) : Bool :=
  c.bodyChecked && c.directCallee && c.exactActual

/-- Events that may make a pointer-return root unsafe to hand out. -/
inductive Event where
  | written (root : Nat)
  | addressTaken (root : Nat)
  | otherCallActual (root : Nat)
  | listCountActual (root : Nat) (certificate : Certificate)
  deriving DecidableEq, Repr

def Event.mayBind (root : Nat) : Event → Bool
  | .written n => n == root
  | .addressTaken n => n == root
  | .otherCallActual n => n == root
  | .listCountActual n c => n == root && !c.accepts

def cleanRoot (root : Nat) (events : List Event) : Bool :=
  events.all fun event => !event.mayBind root

/-- Rule 5b still needs the independent never-bound closure. -/
def admitsPointerReturn (neverBound : Bool) (root : Nat)
    (events : List Event) : Bool :=
  neverBound && cleanRoot root events

/-- A checked, direct, exact read-only actual cannot mark its root. -/
theorem checked_list_count_does_not_bind (root : Nat)
    (c : Certificate) (h : c.accepts = true) :
    (Event.listCountActual root c).mayBind root = false := by
  simp [Event.mayBind, h]

/-- Any missing part of the certificate keeps the ordinary call refusal. -/
theorem uncertified_list_count_binds (root : Nat)
    (c : Certificate) (h : c.accepts = false) :
    (Event.listCountActual root c).mayBind root = true := by
  simp [Event.mayBind, h]

/-- Adding one proved read-only event does not change the root decision. -/
theorem checked_actual_preserves_decision (root : Nat) (events : List Event)
    (c : Certificate) (h : c.accepts = true) :
    cleanRoot root (.listCountActual root c :: events) = cleanRoot root events := by
  simp [cleanRoot, checked_list_count_does_not_bind root c h]

theorem checked_actual_preserves_return (neverBound : Bool) (root : Nat)
    (events : List Event) (c : Certificate) (h : c.accepts = true) :
    admitsPointerReturn neverBound root (.listCountActual root c :: events) =
      admitsPointerReturn neverBound root events := by
  simp [admitsPointerReturn, checked_actual_preserves_decision root events c h]

theorem a_bound_root_is_never_admitted (root : Nat) (events : List Event) :
    admitsPointerReturn false root events = false := by
  simp [admitsPointerReturn]

/-- Any other call actual on the same root still refuses it. -/
theorem other_call_still_refuses (root : Nat) (events : List Event) :
    cleanRoot root (.otherCallActual root :: events) = false := by
  simp [cleanRoot, Event.mayBind]

theorem another_actual_blocks_the_return (root : Nat) (events : List Event) :
    admitsPointerReturn true root (.otherCallActual root :: events) = false := by
  simp [admitsPointerReturn, other_call_still_refuses]

/-- A direct write still refuses it. -/
theorem write_still_refuses (root : Nat) (events : List Event) :
    cleanRoot root (.written root :: events) = false := by
  simp [cleanRoot, Event.mayBind]

end QCore.ReadOnlyActualFrame
