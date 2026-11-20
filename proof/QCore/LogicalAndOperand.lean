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

/-!
# A logical AND before a parenthesised name is not its address (card CG10-READER-GAPS, rule 71)

The analyzer's token stream splits every punctuator into single characters,
so the source's `&&` is the two tokens `&` `&`. Rule 10o's reader of
parenthesised writes read the `&` right before a group `( p )` as the unary
address of `p`; rule 71 reads it as the second half of a logical AND when
exactly two `&` stand there after an operand end
(`logical_and_operand::logical_and_before`, bound to `DECISION_VECTORS`).

The model keeps the source's tokens (`STok`) and the character tokens the
reader sees (`CTok`), both as the prefix before the group, most recent token
first. It proves:

* **no real address is missed** (`a_unary_address_is_read`): when the
  source's last token before the group is a unary `&` (it follows no operand
  end) the reader answers address, except for the one spelling C forbids,
  `X & &(p)` (a bitwise AND with a pointer operand, C11 6.5.10p2), which is a
  hypothesis of the theorem, not an axiom;
* **what is read as a logical AND is one** (`a_logical_and_is_one`): the
  source spelled `X && (p)`, or the forbidden `X & & (p)`;
* the decision vectors.

## What this is not

A model of tokenisation or of C's grammar: that the importer splits
punctuators into characters and that a `&` after an operand end is binary
are the gateway's premises (trust boundary 28).
-/

namespace QCore.LogicalAndOperand

/-- A source token before the group: `&`, `&&`, an operand end (a name, a
number, `)` or `]`), or anything else. -/
inductive STok where
  | amp
  | andand
  | opEnd
  | other
  deriving DecidableEq, Repr

/-- A character token the reader sees. -/
inductive CTok where
  | amp
  | opEnd
  | other
  deriving DecidableEq, Repr

/-- One source token's characters, most recent first. -/
def piece : STok → List CTok
  | .amp => [.amp]
  | .andand => [.amp, .amp]
  | .opEnd => [.opEnd]
  | .other => [.other]

/-- The reader's view of a source prefix (both most recent first). -/
def split : List STok → List CTok
  | [] => []
  | t :: ts => piece t ++ split ts

/-- Rule 71: the last `&` is the second character of a logical AND. -/
def logicalAnd : List CTok → Bool
  | .amp :: .amp :: .opEnd :: _ => true
  | _ => false

/-- The reader's answer: the group is addressed. -/
def address : List CTok → Bool
  | .amp :: rest => !logicalAnd (.amp :: rest)
  | _ => false

/-- The Rust's `DECISION_VECTORS`, most recent token first. -/
def decisionVectors : List (List CTok × Bool) :=
  [ ([.amp, .amp, .opEnd], false),          -- `K & & ( p )`: a logical AND
    ([.amp, .amp, .opEnd, .other], false),  -- `x ) & & ( p )`
    ([.amp, .other], true),                 -- `f ( & ( p ) )`
    ([.amp, .amp, .amp, .opEnd], true),     -- `a & & & ( p )`
    ([.amp, .amp, .other], true),           -- `( & & ( p )`
    ([.amp], true),                         -- `& ( p )`
    ([.amp, .other], true) ]                -- `= & ( p )`

theorem decision_vectors_hold :
    decisionVectors.all (fun (p, r) => address p == r) = true := by
  decide

/-- **No real address is missed.** A unary `&` (one following no operand
end) right before the group is read as an address, unless the source is the
forbidden `X & &(p)`. -/
theorem a_unary_address_is_read (pre : List STok)
    (unary : ∀ rest, pre ≠ .opEnd :: rest)
    (wellTyped : ∀ rest, pre ≠ .amp :: .opEnd :: rest) :
    address (split (.amp :: pre)) = true := by
  match pre, unary, wellTyped with
  | [], _, _ => rfl
  | .amp :: [], _, _ => rfl
  | .amp :: .amp :: _, _, _ => rfl
  | .amp :: .andand :: _, _, _ => rfl
  | .amp :: .opEnd :: rest, _, h => exact absurd rfl (h rest)
  | .amp :: .other :: _, _, _ => rfl
  | .andand :: _, _, _ => rfl
  | .opEnd :: rest, h, _ => exact absurd rfl (h rest)
  | .other :: _, _, _ => rfl

/-- **What is read as a logical AND is one**: the source spelled `X && (p)`,
or the forbidden `X & &(p)`. -/
theorem a_logical_and_is_one (ss : List STok) (h : logicalAnd (split ss) = true) :
    (∃ r, ss = .andand :: .opEnd :: r) ∨ (∃ r, ss = .amp :: .amp :: .opEnd :: r) := by
  match ss, h with
  | .andand :: .opEnd :: r, _ => exact .inl ⟨r, rfl⟩
  | .amp :: .amp :: .opEnd :: r, _ => exact .inr ⟨r, rfl⟩
  | [], h => simp [split, logicalAnd] at h
  | [.amp], h => simp [split, piece, logicalAnd] at h
  | [.andand], h => simp [split, piece, logicalAnd] at h
  | .opEnd :: _, h => simp [split, piece, logicalAnd] at h
  | .other :: _, h => simp [split, piece, logicalAnd] at h
  | .andand :: .amp :: _, h => simp [split, piece, logicalAnd] at h
  | .andand :: .andand :: _, h => simp [split, piece, logicalAnd] at h
  | .andand :: .other :: _, h => simp [split, piece, logicalAnd] at h
  | [.amp, .amp], h => simp [split, piece, logicalAnd] at h
  | .amp :: .andand :: _, h => simp [split, piece, logicalAnd] at h
  | .amp :: .opEnd :: _, h => simp [split, piece, logicalAnd] at h
  | .amp :: .other :: _, h => simp [split, piece, logicalAnd] at h
  | .amp :: .amp :: .amp :: _, h => simp [split, piece, logicalAnd] at h
  | .amp :: .amp :: .andand :: _, h => simp [split, piece, logicalAnd] at h
  | .amp :: .amp :: .other :: _, h => simp [split, piece, logicalAnd] at h

/-- A logical AND neither writes nor addresses its operand, so the reader
that drops its seed drops no event: every address the reader still answers
is a group after a unary `&`, and the only groups it stops answering are
logical ANDs (or the forbidden spelling). -/
theorem only_a_logical_and_is_dropped (cs : List CTok)
    (h : cs.head? = some .amp) (hn : address cs = false) : logicalAnd cs = true := by
  match cs, h, hn with
  | .amp :: rest, _, hn => simpa [address] using hn

end QCore.LogicalAndOperand
