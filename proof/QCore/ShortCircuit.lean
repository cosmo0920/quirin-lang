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

namespace QCore.ShortCircuit

/-! C short-circuit conditions evaluate the left operand first; the right
operand's effects occur only if its Boolean value is needed. -/
inductive Operator where
  | and | or
  deriving DecidableEq
inductive Evaluation where
  | left (value : Bool)
  | right (value : Bool)
  deriving DecidableEq

def evaluatesRight (op : Operator) (left : Bool) : Bool :=
  match op with
  | .and => left
  | .or => !left

def trace (op : Operator) (left right : Bool) : List Evaluation :=
  if evaluatesRight op left then [.left left, .right right] else [.left left]

def result (op : Operator) (left right : Bool) : Bool :=
  if evaluatesRight op left then right else left

theorem and_false_skips_right (right : Bool) :
    trace .and false right = [.left false] := rfl

theorem or_true_skips_right (right : Bool) :
    trace .or true right = [.left true] := rfl

theorem evaluated_right_follows_left {op : Operator} {left right : Bool}
    (h : evaluatesRight op left = true) :
    trace op left right = [.left left, .right right] := by simp [trace,h]

theorem and_result_is_conjunction (left right : Bool) :
    result .and left right = (left && right) := by cases left <;> rfl

theorem or_result_is_disjunction (left right : Bool) :
    result .or left right = (left || right) := by cases left <;> rfl

theorem skipped_right_cannot_change_result {op : Operator} {left : Bool}
    (h : evaluatesRight op left = false) (first second : Bool) :
    result op left first = result op left second := by simp [result,h]

end QCore.ShortCircuit
