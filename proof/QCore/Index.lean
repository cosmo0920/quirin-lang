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

namespace QCore

inductive Dimension where
  | named : String -> Dimension
  | count : String -> Dimension
  deriving DecidableEq, Repr

/-- Index terms are indexed by their nominal dimension. Ill-dimensioned
    addition, comparison operands, min/max, and monus are therefore not
    representable in the formal core. Scaling takes a natural constant. -/
inductive IndexTerm : Dimension -> Type where
  | literal {dimension} : Nat -> IndexTerm dimension
  | var {dimension} : Nat -> IndexTerm dimension
  | add {dimension} : IndexTerm dimension -> IndexTerm dimension -> IndexTerm dimension
  | scale {dimension} : Nat -> IndexTerm dimension -> IndexTerm dimension
  | min {dimension} : IndexTerm dimension -> IndexTerm dimension -> IndexTerm dimension
  | max {dimension} : IndexTerm dimension -> IndexTerm dimension -> IndexTerm dimension
  | monus {dimension} : IndexTerm dimension -> IndexTerm dimension -> IndexTerm dimension

namespace IndexTerm

def eval (valuation : (dimension : Dimension) -> Nat -> Nat) :
    IndexTerm dimension -> Nat
  | literal value => value
  | var name => valuation dimension name
  | add left right => eval valuation left + eval valuation right
  | scale factor term => factor * eval valuation term
  | min left right => Nat.min (eval valuation left) (eval valuation right)
  | max left right => Nat.max (eval valuation left) (eval valuation right)
  | monus left right => eval valuation left - eval valuation right

theorem eval_add (valuation : (dimension : Dimension) -> Nat -> Nat)
    (left right : IndexTerm dimension) :
    eval valuation (.add left right) = eval valuation left + eval valuation right := by
  rfl

theorem eval_scale (valuation : (dimension : Dimension) -> Nat -> Nat)
    (factor : Nat) (term : IndexTerm dimension) :
    eval valuation (.scale factor term) = factor * eval valuation term := by
  rfl

end IndexTerm
end QCore
