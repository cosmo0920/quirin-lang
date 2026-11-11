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

import QCore.ResidualAccessReadings

/-!
# String-literal reads and decayed array members (card W48-BU-SINGLE-LEAF-BODIES, gateway rule 35)

The abstract rule implemented by the rule package
`crates/gateway-rules/rule-35-literal-decay` (card RULE35-PILOT):
`literal_read.rs` (arm 1, hosted in the gateway's lexical access predicate
`legacy_token_shapes::contains_unmodeled_access`) and `array_member_decay.rs`
(arm 2, hosted in the W44 typed call-actual reader
`typed_access_positions::StatementIndex::argument`).

**Arm 1.** A `*` token whose operand begins with a literal that is only read
reads one character of a string literal: an array object of static storage
duration (C11 6.4.5p6), which names no object of the program. The read touches
no binding and adds no obligation. A store or increment keeps the refusal, and
every other token of the predicate keeps its own answer, so the exempted
positions are a subset of the old refusals.

**Arm 2.** An array member decayed to a pointer at a call actual is the address
of its first element (C11 6.3.2.1p3), so it is read exactly as rule 14c reads
`&PATH[0]`: admitted only for a member place of a one-dimensional array of
builtin scalars that is neither `volatile` nor `_Atomic`.

## What this is not

A model of the decisions, of the predicate's monotonicity and of the reads'
transfer. Faithful lexing of literals, Clang's cast kinds and member types, and
the Lean-to-Rust correspondence are premises (trust boundary 28), as is rule
14c's premise that a callee writes within the array the address points into.
-/

namespace QCore.LiteralDecayReadings

/-! ## Arm 1: the decision -/

inductive LiteralDecision where
  | literal
  | store
  | notLiteral
  deriving DecidableEq, Repr

/-- `literal_read::decide`: whether the next token is a literal, and
whether the literal run is stored into or incremented. -/
def decideLiteral (nextIsLiteral : Bool) (store : Bool) : LiteralDecision :=
  if !nextIsLiteral then .notLiteral
  else if store then .store
  else .literal

/-- The decision vectors, spelled as `literal_read::DECISION_VECTORS`. -/
def literalVectors : List (Bool × Bool × LiteralDecision) :=
  [ (true, false, .literal)
  , (true, true, .store)
  , (false, false, .notLiteral)
  , (false, true, .notLiteral) ]

theorem literal_vectors_hold :
    literalVectors.all (fun (n, s, d) => decideLiteral n s == d) = true := by
  decide

theorem a_star_is_exempt_exactly_on_a_read_literal (n s : Bool) :
    decideLiteral n s = .literal ↔ n = true ∧ s = false := by
  cases n <;> cases s <;> simp [decideLiteral]

theorem a_store_into_a_literal_is_never_exempt (n : Bool) :
    decideLiteral n true ≠ .literal := by
  cases n <;> simp [decideLiteral]

/-! ## Arm 1: the predicate -/

/-- One token as the predicate sees it: a star with its decision inputs, or any
other token with the predicate's own old answer. -/
inductive Tok where
  | star (oldRefuses : Bool) (nextIsLiteral : Bool) (store : Bool)
  | other (refuses : Bool)
  deriving DecidableEq, Repr

def oldRefuses : Tok → Bool
  | .star r _ _ => r
  | .other r => r

/-- The amended per-token answer: an exempt star refuses nothing; every other
token answers as before. -/
def newRefuses : Tok → Bool
  | .star r n s => if decideLiteral n s == .literal then false else r
  | .other r => r

/-- `contains_unmodeled_access`: some token refuses. -/
def gate (answer : Tok → Bool) (ts : List Tok) : Bool := ts.any answer

theorem a_token_refuses_only_where_it_refused_before (t : Tok)
    (h : newRefuses t = true) : oldRefuses t = true := by
  cases t with
  | star r n s =>
    simp only [newRefuses] at h
    split at h
    · exact absurd h (by simp)
    · exact h
  | other r => exact h

/-- The amended gate refuses only a run the old gate refused: the rule only
admits. -/
theorem the_gate_only_admits (ts : List Tok)
    (h : gate newRefuses ts = true) : gate oldRefuses ts = true := by
  simp only [gate, List.any_eq_true] at h ⊢
  obtain ⟨t, hm, ht⟩ := h
  exact ⟨t, hm, a_token_refuses_only_where_it_refused_before t ht⟩

/-- A token that is not an exempt star answers exactly as before. -/
theorem every_other_token_is_unchanged (t : Tok)
    (h : ∀ r n s, t = .star r n s → decideLiteral n s ≠ .literal) :
    newRefuses t = oldRefuses t := by
  cases t with
  | star r n s =>
    have hne := h r n s rfl
    simp [newRefuses, oldRefuses, hne]
  | other r => rfl

/-- A run that differs only by exempt stars is admitted by the amended gate
whenever every other token was admitted. -/
theorem a_run_refused_only_at_literal_stars_is_admitted (ts : List Tok)
    (h : ∀ t ∈ ts, oldRefuses t = true →
      ∃ r n s, t = .star r n s ∧ decideLiteral n s = .literal) :
    gate newRefuses ts = false := by
  simp only [gate, List.any_eq_false]
  intro t hm
  cases t with
  | star r n s =>
    by_cases hd : decideLiteral n s = .literal
    · simp [newRefuses, hd]
    · cases hr : r
      · simp [newRefuses, hd]
      · obtain ⟨r', n', s', he, hl⟩ := h _ hm (by simp [oldRefuses, hr])
        cases he
        exact absurd hl hd
  | other r =>
    cases hr : r
    · simp [newRefuses]
    · obtain ⟨_, _, _, he, _⟩ := h _ hm (by simp [oldRefuses, hr])
      cases he

/-! ## Arm 1: the read's transfer -/

/-- The projection's state as far as a read can reach it: bindings of names to
identities and the obligations owed. -/
structure State where
  bindings : List (String × Nat)
  obligations : List Nat
  deriving DecidableEq, Repr

/-- A read of one character of a static literal object: the value read is data
(not folded, so both branch edges stay), and nothing the projection names is
touched. -/
def readLiteral (s : State) : State × Option Nat := (s, none)

theorem a_literal_read_changes_no_binding_and_owes_nothing (s : State) :
    (readLiteral s).1 = s ∧ (readLiteral s).2 = none := ⟨rfl, rfl⟩

/-! ## Arm 2: the decision -/

inductive Element where
  | scalar
  | pointer
  | record
  | array
  | other
  deriving DecidableEq, Repr

inductive DecayDecision where
  | admit
  | notMember
  | element
  | qualified
  deriving DecidableEq, Repr

/-- `array_member_decay::decide`. -/
def decideDecay (member : Bool) (e : Element) (qualified : Bool) : DecayDecision :=
  if !member then .notMember
  else if qualified then .qualified
  else if e != .scalar then .element
  else .admit

/-- The decision vectors, spelled as `array_member_decay::DECISION_VECTORS`. -/
def decayVectors : List (Bool × Element × Bool × DecayDecision) :=
  [ (true, .scalar, false, .admit)
  , (true, .pointer, false, .element)
  , (true, .record, false, .element)
  , (true, .array, false, .element)
  , (true, .other, false, .element)
  , (true, .scalar, true, .qualified)
  , (false, .scalar, false, .notMember)
  , (false, .pointer, true, .notMember) ]

theorem decay_vectors_hold :
    decayVectors.all (fun (m, e, q, d) => decideDecay m e q == d) = true := by
  decide

theorem a_decay_is_admitted_exactly_for_a_scalar_member (m : Bool) (e : Element) (q : Bool) :
    decideDecay m e q = .admit ↔ m = true ∧ q = false ∧ e = .scalar := by
  cases m <;> cases q <;> cases e <;> simp [decideDecay]

theorem a_pointer_or_record_element_is_never_admitted (m q : Bool) (e : Element)
    (h : e = .pointer ∨ e = .record ∨ e = .array) : decideDecay m e q ≠ .admit := by
  rcases h with h | h | h <;> subst h <;> cases m <;> cases q <;> simp [decideDecay]

/-! ## Arm 2: the decayed address is rule 14c's element address -/

/-- A pointer value as an address: a place and an element index (the decayed
member is the place's array at index 0). -/
structure Address where
  place : String
  index : Nat
  deriving DecidableEq, Repr

/-- `&PATH[I]`. -/
def elementAddress (place : String) (i : Nat) : Address := ⟨place, i⟩

/-- C11 6.3.2.1p3: the decayed array `PATH` is the address of its initial
element. -/
def decayed (place : String) : Address := elementAddress place 0

theorem the_decay_is_the_first_element_address (place : String) :
    decayed place = elementAddress place 0 := by
  simp [decayed]

/-- An admitted decay is exactly an admitted rule-14c element address with a
scalar element type. -/
theorem an_admitted_decay_is_a_14c_element_address (e : Element)
    (h : decideDecay true e false = .admit) :
    QCore.ResidualAccessReadings.addressShapeAdmitted .element (e == .scalar) = true := by
  cases e <;> simp [decideDecay] at h ⊢ <;> rfl

theorem a_14c_refused_element_is_never_a_decay (e : Element)
    (h : decideDecay true e false = .admit) : e = .scalar := by
  cases e <;> simp [decideDecay] at h ⊢

end QCore.LiteralDecayReadings
