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
# `&(p)->f` is no address of `p` (card K1-CALL-ACTUAL-ACCESS, rule 75)

Rule 10o's reader of parenthesised writes seeds a name `p` as written or
addressed whole when a unary `&` stands right before a group `( p )`. Macro
expansion spells a member address with such a group:
`mtx_lock(&(rktp)->rktp_lock)`. Rule 75 drops that seed, for the never-bound
closure only, when the group is followed by `->`
(`parenthesised_member_operand::member_operand`, bound to `DECISION_VECTORS`).

C's grammar decides what the `&` takes. A postfix operator binds tighter
than a unary one (C11 6.5.2, 6.5.3), so `& ( p ) -> f` is `& ( ( p ) -> f )`;
a parenthesised expression is the expression it encloses (6.5.1p5). The model
names what follows the group (`Next`), the operand C gives the `&`
(`ampOperand`: the whole object only when no postfix operator follows), the
plain reader's seed for `& p NEXT` (it exempts every postfix continuation)
and rule 10o's parenthesised seed before and after rule 75. It proves:

* **no whole-object address is missed** (`a_whole_address_is_seeded`): when
  the `&` takes `p` itself, the closure still seeds `p`, for one group and for
  every list of groups (`every_whole_address_is_seeded`);
* **only a sub-object address is dropped** (`only_a_member_operand_is_dropped`);
* **the reading never exceeds the plain one** (`at_least_the_plain_reader`) and
  agrees with it on the arrow (`agrees_with_the_plain_reader_on_the_arrow`);
* **the other consumers are unchanged** (`the_old_reading_is_kept_without_the_arm`);
* the decision vectors.

## What this is not

A model of tokenisation, of C's whole grammar, or of the closure's other
seeds and rounds: that the importer splits `->` into `-` `>`, and that the
increment and compound seeds read only the tokens after the group (which
cannot be `-` `>` and match those), are the gateway's premises (trust
boundary 28).
-/

namespace QCore.ParenthesisedMemberOperand

/-- What follows the group `( p )` (or the name `p`): an arrow `->`, a `.`,
a `[`, a `(`, or anything else. -/
inductive Next where
  | arrow
  | dot
  | bracket
  | call
  | other
  deriving DecidableEq, Repr

/-- The operand C gives a unary `&` before `( p ) NEXT`: `p` itself only when
no postfix operator follows; otherwise a postfix expression built on `p` (a
member, an element, a call result), never `p`. -/
inductive Operand where
  | whole
  | sub
  deriving DecidableEq, Repr

def ampOperand : Next → Operand
  | .other => .whole
  | _ => .sub

/-- The plain reader's seed for `& p NEXT` (`seed_addresses_and_writes`: a
name followed by `->`, `.`, `[` or `(` is not addressed whole). -/
def plainSeed : Next → Bool
  | .other => true
  | _ => false

/-- Rule 10o's seed for `& ( p ) NEXT` (a unary `&`): always. -/
def oldSeed (_ : Next) : Bool := true

/-- Rule 75 (`member_operand`): the arrow's seed is dropped. -/
def memberOperand : Next → Bool
  | .arrow => true
  | _ => false

/-- The never-bound closure's seed with rule 75's arm. -/
def newSeed (n : Next) : Bool := oldSeed n && !memberOperand n

/-- The reading rule 10o's other consumers keep (`arm = false`) and the
closure's (`arm = true`). -/
def reading (arm : Bool) (n : Next) : Bool := oldSeed n && !(arm && memberOperand n)

/-- The Rust's `DECISION_VECTORS` (`member_operand`), as what follows the group. -/
def decisionVectors : List (Next × Bool) :=
  [ (.arrow, true),     -- `& ( p ) - > f`
    (.arrow, true),     -- `& ( ( p ) ) - > f`
    (.arrow, true),     -- `& ( p ) - > a . b`
    (.dot, false),      -- `& ( p ) . f`
    (.bracket, false),  -- `& ( p ) [ 0 ]`
    (.other, false),    -- `& ( p ) ;`
    (.other, false),    -- `& ( p ) - - ;` (the decrement seed still fires)
    (.other, false),    -- `& ( p ) - 1`
    (.other, false) ]   -- `& ( p ) )`

theorem decision_vectors_hold :
    decisionVectors.all (fun (n, r) => memberOperand n == r) = true := by
  decide

/-- **No whole-object address is missed.** -/
theorem a_whole_address_is_seeded (n : Next) (h : ampOperand n = .whole) :
    newSeed n = true := by
  cases n <;> simp_all [ampOperand, newSeed, oldSeed, memberOperand]

/-- **Only a sub-object address is dropped**: a seed rule 10o had and rule 75
drops is a group whose `&` takes a postfix expression, never `p`. -/
theorem only_a_member_operand_is_dropped (n : Next)
    (old : oldSeed n = true) (new : newSeed n = false) : ampOperand n = .sub := by
  cases n <;> simp_all [ampOperand, newSeed, oldSeed, memberOperand]

/-- **Never more permissive than the plain spelling**: where the plain reader
seeds `& p NEXT`, the closure seeds `& ( p ) NEXT`. -/
theorem at_least_the_plain_reader (n : Next) (h : plainSeed n = true) :
    newSeed n = true := by
  cases n <;> simp_all [plainSeed, newSeed, oldSeed, memberOperand]

/-- On the arrow, the parenthesised and plain spellings are read alike. -/
theorem agrees_with_the_plain_reader_on_the_arrow :
    newSeed .arrow = plainSeed .arrow := by
  decide

/-- Rule 10o's other consumers (the aggregate store's premises, `may_bind`)
keep the old reading byte for byte. -/
theorem the_old_reading_is_kept_without_the_arm (n : Next) :
    reading false n = oldSeed n := by
  simp [reading]

theorem the_closure_reads_with_the_arm (n : Next) : reading true n = newSeed n := by
  simp [reading, newSeed]

/-- The names the closure seeds over a body's groups (name, what follows). -/
def seeded (seed : Next → Bool) (groups : List (String × Next)) (p : String) : Bool :=
  groups.any (fun (q, n) => q == p && seed n)

/-- **Lifted to a body**: a name some group addresses whole is seeded by the
closure, whatever other groups (member operands) the body holds. -/
theorem every_whole_address_is_seeded (groups : List (String × Next)) (p : String)
    (h : ∃ n, (p, n) ∈ groups ∧ ampOperand n = .whole) :
    seeded newSeed groups p = true := by
  obtain ⟨n, hm, hw⟩ := h
  unfold seeded
  rw [List.any_eq_true]
  exact ⟨(p, n), hm, by simp [a_whole_address_is_seeded n hw]⟩

/-- The closure's seeds are a subset of rule 10o's: rule 75 only removes. -/
theorem the_closure_only_drops_seeds (groups : List (String × Next)) (p : String)
    (h : seeded newSeed groups p = true) : seeded oldSeed groups p = true := by
  unfold seeded at *
  rw [List.any_eq_true] at *
  obtain ⟨x, hm, hx⟩ := h
  refine ⟨x, hm, ?_⟩
  simp [newSeed] at hx
  simp [hx.1, oldSeed]

end QCore.ParenthesisedMemberOperand
