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

import QCore.Index

namespace QCore.QuantIndexBinding

/-! Semantic account of an elaborated, type-local `where n := e` binding.
The Rust front end resolves closed dependency graphs, checks dimensions and
u128 arithmetic before constructing this dimension-indexed core. These proofs
cover substitution and local scope, not the Rust parser or graph resolver. -/

def extend (valuation : (d : Dimension) → Nat → Nat)
    (dimension : Dimension) (name value : Nat) : (d : Dimension) → Nat → Nat :=
  fun d n => if d = dimension ∧ n = name then value else valuation d n

def substitute (name : Nat) (replacement : IndexTerm dimension) :
    IndexTerm dimension → IndexTerm dimension
  | .literal n => .literal n
  | .var n => if n = name then replacement else .var n
  | .add a b => .add (substitute name replacement a) (substitute name replacement b)
  | .scale n a => .scale n (substitute name replacement a)
  | .min a b => .min (substitute name replacement a) (substitute name replacement b)
  | .max a b => .max (substitute name replacement a) (substitute name replacement b)
  | .monus a b => .monus (substitute name replacement a) (substitute name replacement b)

theorem substitution_preserves_evaluation
    (valuation : (d : Dimension) → Nat → Nat) (name : Nat)
    (replacement term : IndexTerm dimension) :
    (substitute name replacement term).eval valuation =
      term.eval (extend valuation dimension name (replacement.eval valuation)) := by
  induction term with
  | literal n => rfl
  | var n =>
      by_cases h : n = name
      · simp [substitute, IndexTerm.eval, extend, h]
      · simp [substitute, IndexTerm.eval, extend, h]
  | add a b ha hb => simp [substitute, IndexTerm.eval, ha, hb]
  | scale n a ha => simp [substitute, IndexTerm.eval, ha]
  | min a b ha hb => simp [substitute, IndexTerm.eval, ha, hb]
  | max a b ha hb => simp [substitute, IndexTerm.eval, ha, hb]
  | monus a b ha hb => simp [substitute, IndexTerm.eval, ha, hb]

theorem binding_preserves_other_names
    (valuation : (d : Dimension) → Nat → Nat) (dimension : Dimension)
    (name other value : Nat) (different : other ≠ name) :
    extend valuation dimension name value dimension other = valuation dimension other := by
  simp [extend, different]

theorem binding_preserves_other_dimensions
    (valuation : (d : Dimension) → Nat → Nat) (dimension other : Dimension)
    (name index value : Nat) (different : other ≠ dimension) :
    extend valuation dimension name value other index = valuation other index := by
  simp [extend, different]

def bounded (value : Nat) : Option Nat :=
  if value < 2 ^ 128 then some value else none

theorem accepted_value_fits_u128 (input output : Nat)
    (accepted : bounded input = some output) : output < 2 ^ 128 := by
  unfold bounded at accepted
  split at accepted
  · cases accepted
    assumption
  · contradiction

theorem five_binding (valuation : (d : Dimension) → Nat → Nat)
    (dimension : Dimension) (name : Nat) :
    (substitute name (.literal 5) (.var name) : IndexTerm dimension).eval valuation = 5 := by
  simp [substitute, IndexTerm.eval]

end QCore.QuantIndexBinding
