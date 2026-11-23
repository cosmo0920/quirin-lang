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

import QCore.CastBeforeAmpersand

/-!
# A `)` that may close a cast lets the next single `*` be a dereference (card CAST-BEFORE-STAR-READ-AS-BINARY, rule 74g)

Rule 74f (`QCore.CastBeforeAmpersand`) reads a single `&` after a `)` that may
close a cast as unary. This module states the same detector's reading of a
single `*` before a lone name, for the two readers that use it. Its classes
are bound to `ampersand_runs::STAR_VECTORS`.

* **The refused hand-on.** The base read every `)` as an operand end, so it
  handed on `p` for `(char *) *p` and missed the slot `*p`
  (`old_reading_misses_the_slot`). The tip hands on both after a `)` that may
  close a cast. Every place the base handed on, the tip still hands on
  (`the_fix_only_adds_places`). The union contains the places of both
  readings (`the_union_covers_both_readings`). That is why an undecided group
  hands on both (`an_undecided_group_hands_on_both`), while an argument list
  and a known object are binary (`a_call_is_binary`, `an_object_is_binary`).
* **Premise 4 of `aggregate_store`.** The base read `(T) *b` as no
  dereference. The tip reads a dereference after every `)` that may close a
  cast, so it refuses wherever the base refused
  (`premise_four_only_refuses_more`) and also refuses a cast
  (`premise_four_refuses_a_cast`).

## What this is not

A model of C's type-name grammar or of the token classification. The class of
each group (`Group`) is the Rust detector's output, a premise (trust boundary
28), as are the importer's character tokens and the completeness of the
declared-object set the refused hand-on reads.
-/

namespace QCore.CastBeforeStar

open QCore.CastBeforeAmpersand

/-- The places the base refused hand-on handed on for `* p` (a lone name)
after `b`: the slot `*p` for a unary `*` (`oldUnary`), else `p`. -/
def oldHanded (b : Before) (p : String) : List String :=
  if oldUnary b then ["*" ++ p] else [p]

/-- Rule 74g: after a `)` that may close a cast, both readings' places. -/
def handed : Before → String → List String
  | .notOperandEnd, p => ["*" ++ p]
  | .operandEnd, p => [p]
  | .close g, p => if g.mayBeCast then ["*" ++ p, p] else [p]

/-- `(char *) *p`: the base missed the slot, the tip hands it on. -/
theorem old_reading_misses_the_slot :
    "*p" ∉ oldHanded (.close .typeName) "p" ∧ "*p" ∈ handed (.close .typeName) "p" := by
  decide

/-- Every place the base handed on, the tip still hands on. -/
theorem the_fix_only_adds_places (b : Before) (p q : String)
    (h : q ∈ oldHanded b p) : q ∈ handed b p := by
  cases b with
  | notOperandEnd => simpa [oldHanded, oldUnary, handed] using h
  | operandEnd => simpa [oldHanded, oldUnary, handed] using h
  | close g =>
    simp only [oldHanded, oldUnary] at h
    cases hg : g.mayBeCast <;> simp_all [handed]

/-- The unary reading's place (`*p`) and the binary reading's (`p`) are both
handed on after a `)` that may close a cast. -/
theorem the_union_covers_both_readings (g : Group) (p : String)
    (h : g.mayBeCast = true) :
    ("*" ++ p) ∈ handed (.close g) p ∧ p ∈ handed (.close g) p := by
  simp [handed, h]

/-- `(a) * b` with `a` possibly a type: both `*b` and `b`. -/
theorem an_undecided_group_hands_on_both (p : String) :
    handed (.close .undecided) p = ["*" ++ p, p] := rfl

/-- `f(x) * n`: a multiplication, `n` alone. -/
theorem a_call_is_binary (p : String) : handed (.close .arguments) p = [p] := rfl

/-- `(a) * b` with `a` a declared object, and `(a + 1) * n`: binary. -/
theorem an_object_is_binary (p : String) :
    handed (.close .object) p = [p] ∧ handed (.close .expression) p = [p] :=
  ⟨rfl, rfl⟩

/-- Premise 4 at the base: a dereference only when the token before the `*`
ends no operand. -/
def oldDereferenced (b : Before) : Bool := oldUnary b

/-- Rule 74g: also after a `)` that may close a cast. -/
def dereferenced (b : Before) : Bool := unary b

/-- Premise 4 refuses wherever the base refused. -/
theorem premise_four_only_refuses_more (b : Before) (h : oldDereferenced b = true) :
    dereferenced b = true :=
  the_fix_only_adds_addresses b h

/-- `(void) *b`: the base admitted the base, the tip refuses it. -/
theorem premise_four_refuses_a_cast :
    oldDereferenced (.close .typeName) = false ∧ dereferenced (.close .typeName) = true := by
  decide

/-- The classes of `ampersand_runs::STAR_VECTORS`, in order, with the
expected answer of `may_close_a_cast`. -/
def starVectors : List (Group × Bool) :=
  [ (.typeName, true),     -- `( char * ) * p`
    (.typeName, true),     -- `( unsigned char ) * p`, objects {p}
    (.undecided, true),    -- `( a ) * b`
    (.object, false),      -- `( a ) * b`, objects {a}
    (.arguments, false),   -- `f ( x ) * n`
    (.typeName, true),     -- `( void ( * ) ( int , . . . ) ) * fp`
    (.typeName, true),     -- `( char ( * ) [ 4 ] ) * b`
    (.arguments, false),   -- `sizeof ( T * ) * n`
    (.arguments, false),   -- `__typeof__ ( x ) * b`
    (.expression, false) ] -- `( a + 1 ) * n`

theorem star_vectors_hold :
    starVectors.all (fun (g, cast) => g.mayBeCast == cast) = true := by
  decide

end QCore.CastBeforeStar
