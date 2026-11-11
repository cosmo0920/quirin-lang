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

namespace QCore.AdvancedControl

/-! Cycle 15's reduced account of cancellation, fallible cleanup, and linear
ownership capture. -/

abbrev ResourceId := Nat
abbrev ErrorId := Nat

inductive PrimaryExit where
  | normal
  | error (error : ErrorId)
  | cancelled
  deriving DecidableEq, Repr

inductive Outcome where
  | normal
  | error (error : ErrorId)
  | cancelled
  | cleanupFailure (errors : List ErrorId)
  | doubleError (primary : PrimaryExit) (cleanup : List ErrorId)
  deriving DecidableEq, Repr

def finish (primary : PrimaryExit) (cleanup : List ErrorId) : Outcome :=
  match primary, cleanup with
  | .normal, [] => .normal
  | .normal, errors => .cleanupFailure errors
  | .error error, [] => .error error
  | primary@(.error _), errors => .doubleError primary errors
  | .cancelled, [] => .cancelled
  | .cancelled, errors => .doubleError .cancelled errors

/-- A secondary cleanup error never replaces an existing primary error. -/
theorem double_error_preserves_primary_error
    (primary cleanupError : ErrorId) (rest : List ErrorId) :
    finish (.error primary) (cleanupError :: rest) =
      .doubleError (.error primary) (cleanupError :: rest) := by
  rfl

/-- A secondary cleanup error also preserves cancellation as the primary exit. -/
theorem double_error_preserves_cancellation
    (cleanupError : ErrorId) (rest : List ErrorId) :
    finish .cancelled (cleanupError :: rest) =
      .doubleError .cancelled (cleanupError :: rest) := by
  rfl

def cleanup (resources : List ResourceId) : List ResourceId :=
  resources.reverse

theorem cleanup_is_lifo (resources : List ResourceId) :
    cleanup resources = resources.reverse := by
  rfl

def LinearlyCleans (resources cleaned : List ResourceId) : Prop :=
  cleaned.reverse = resources ∧ resources.Nodup

theorem cleanup_preserves_linearity (resources : List ResourceId)
    (unique : resources.Nodup) : LinearlyCleans resources (cleanup resources) := by
  exact ⟨by simp [cleanup], unique⟩

inductive HandlerKind where
  | oneShot
  | resumable
  | multiShot
  deriving DecidableEq, Repr

structure Handler where
  kind : HandlerKind
  ownedCaptures : List ResourceId
  invocations : Nat
  deriving DecidableEq, Repr

def Handler.Valid (handler : Handler) : Prop :=
  handler.ownedCaptures.Nodup ∧
  (handler.ownedCaptures ≠ [] →
    handler.kind = .oneShot ∧ handler.invocations = 1)

instance (handler : Handler) : Decidable handler.Valid := by
  unfold Handler.Valid
  infer_instance

def checkHandler (handler : Handler) : Option Handler :=
  if handler.Valid then some handler else none

private theorem checked_handler_valid {handler checked : Handler}
    (result : checkHandler handler = some checked) : handler.Valid := by
  unfold checkHandler at result
  split at result
  · assumption
  · contradiction

theorem checked_owned_capture_is_one_shot
    {handler checked : Handler}
    (result : checkHandler handler = some checked)
    (owned : handler.ownedCaptures ≠ []) :
    handler.kind = .oneShot :=
  (checked_handler_valid result).2 owned |>.1

theorem checked_owned_capture_is_consumed_once
    {handler checked : Handler}
    (result : checkHandler handler = some checked)
    (owned : handler.ownedCaptures ≠ []) :
    handler.invocations = 1 :=
  (checked_handler_valid result).2 owned |>.2

theorem checked_captures_are_unique
    {handler checked : Handler}
    (result : checkHandler handler = some checked) :
    handler.ownedCaptures.Nodup :=
  (checked_handler_valid result).1

end QCore.AdvancedControl
