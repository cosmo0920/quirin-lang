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

import QCore.AmpersandRunParity

/-!
# A `)` that may close a cast makes the next single `&` unary (card CAST-BEFORE-AMPERSAND-READ-AS-BINARY, rule 74f)

Rule 74e (`QCore.AmpersandRunParity`) reads a run of `&` tokens by its
parity, and leaves the reading of a single `&` (unary address or binary AND)
to the token before it. This module states the cast-close reading the shared
detector `ampersand_runs::may_close_a_cast` adds for a single `&` after `)`,
bound to its `CAST_VECTORS` by class:

* the base refused hand-on read every `)` as an operand end, so it missed the
  address under a cast (`old_reading_misses_a_cast`); the tip reads unary
  wherever the base did and adds only the cast closes
  (`the_fix_only_adds_addresses`);
* an undecided group, a single name not known as an object, is a cast
  (`an_undecided_group_is_a_cast`); an argument list and a value group are
  binary (`a_call_is_binary`, `a_value_group_is_binary`);
* reading unary hands on every prefix of the chain, a superset of the binary
  reading's one place (`the_address_hands_on_a_superset`), so reading more
  `&` as unary only hands on more places, the direction in which a refusal
  withdraws more and accuses less.

## What this is not

A model of C's type-name grammar or of the token classification. The class of
each group (`Group`) is the Rust detector's output, a premise
(trust boundary 28), as are the importer's character tokens and the
completeness of the declared-object set the refused hand-on reads.
-/

namespace QCore.CastBeforeAmpersand

/-- The class `may_close_a_cast` gives the group a `)` closes. -/
inductive Group where
  /-- An argument list: its `(` follows a callee name, `)` or `]`. -/
  | arguments
  /-- Holds a token no scalar cast's type name holds at its top level. -/
  | expression
  /-- A single name the reader knows as a declared object. -/
  | object
  /-- A single name not known as an object: a variable or a typedef name. -/
  | undecided
  /-- Only names, `*` and nested declarator groups: a type name. -/
  | typeName
  /-- An unmatched `)`. -/
  | unmatched
  deriving DecidableEq, Repr

/-- The token before a single `&`, as the readers classify it. -/
inductive Before where
  /-- Nothing, an operator or an opening bracket: `&` is unary everywhere. -/
  | notOperandEnd
  /-- A name, a number or `]`: an operand end, `&` is binary. -/
  | operandEnd
  /-- A `)` closing a group of the given class. -/
  | close (g : Group)
  deriving DecidableEq, Repr

/-- `may_close_a_cast`: every class but an argument list, an expression or a
known object may close a cast. -/
def Group.mayBeCast : Group → Bool
  | .arguments => false
  | .expression => false
  | .object => false
  | .undecided => true
  | .typeName => true
  | .unmatched => true

/-- The base refused hand-on (`ends_operand`): unary only when the token
before is not an operand end; a `)` always ended an operand. -/
def oldUnary : Before → Bool
  | .notOperandEnd => true
  | .operandEnd => false
  | .close _ => false

/-- Rule 74f: unary also after a `)` that may close a cast. -/
def unary : Before → Bool
  | .notOperandEnd => true
  | .operandEnd => false
  | .close g => g.mayBeCast

/-- `(T *) &x->f`: the base read a binary AND, the tip reads the address. -/
theorem old_reading_misses_a_cast :
    oldUnary (.close .typeName) = false ∧ unary (.close .typeName) = true := by
  decide

/-- The tip reads unary wherever the base did. -/
theorem the_fix_only_adds_addresses (b : Before) (h : oldUnary b = true) :
    unary b = true := by
  cases b <;> simp_all [oldUnary, unary]

/-- What the tip adds is exactly a `)` that may close a cast. -/
theorem the_fix_adds_only_cast_closes (b : Before) :
    (oldUnary b = false ∧ unary b = true) ↔ ∃ g, b = .close g ∧ g.mayBeCast = true := by
  cases b <;> simp [oldUnary, unary]

/-- The fail-closed resolution: `(a) & b` with `a` possibly a type is a cast. -/
theorem an_undecided_group_is_a_cast : unary (.close .undecided) = true := rfl

/-- `f(x) & mask` is a binary AND. -/
theorem a_call_is_binary : unary (.close .arguments) = false := rfl

/-- `(a + 1) & m` and `(a) & b` with `a` a declared object are binary. -/
theorem a_value_group_is_binary :
    unary (.close .expression) = false ∧ unary (.close .object) = false := by
  decide

/-- The places a chain `root -> f1 -> … ` hands on: under a unary `&` every
prefix (`chain`), under a binary one the full place only. -/
def handed (address : Bool) (chain : List String) : List String :=
  if address then chain else chain.getLast?.toList

theorem the_address_hands_on_a_superset (chain : List String) (p : String)
    (h : p ∈ handed false chain) : p ∈ handed true chain := by
  simp only [handed] at h ⊢
  cases hl : chain.getLast? with
  | none => simp [hl] at h
  | some q =>
    simp [hl] at h
    subst h
    exact List.mem_of_getLast? hl

/-- The ledger shape: `(T *) &x->f` hands on `x` too. -/
theorem a_cast_hands_on_the_root :
    "x" ∈ handed (unary (.close .typeName)) ["x", "x->f"] ∧
      "x" ∉ handed (oldUnary (.close .typeName)) ["x", "x->f"] := by
  decide

/-- The classes of `ampersand_runs::CAST_VECTORS`, in order, with the
expected answer of `may_close_a_cast`. -/
def castVectors : List (Group × Bool) :=
  [ (.typeName, true),     -- `( T * ) & x`
    (.typeName, true),     -- `( struct s * ) & v`
    (.typeName, true),     -- `( void ) & x`
    (.undecided, true),    -- `( my_t ) & x`, objects {x}
    (.undecided, true),    -- `( a ) & b`
    (.object, false),      -- `( a ) & b`, objects {a}
    (.arguments, false),   -- `f ( x ) & mask`
    (.arguments, false),   -- `( * fp ) ( x ) & m`
    (.expression, false),  -- `( a + 1 ) & m`
    (.expression, false),  -- `( p [ 0 ] ) & m`
    (.expression, false),  -- `( s . n ) & m`
    (.typeName, true),     -- `( void ( * ) ( int , . . . ) ) & f`
    (.typeName, true),     -- `( char ( * ) [ 4 ] ) & b`
    (.typeName, true),     -- `return ( T * ) & x`
    (.arguments, false),   -- `sizeof ( T ) & x`
    (.unmatched, true) ]   -- `T * ) & x`

theorem cast_vectors_hold :
    castVectors.all (fun (g, cast) => g.mayBeCast == cast) = true := by
  decide

end QCore.CastBeforeAmpersand
