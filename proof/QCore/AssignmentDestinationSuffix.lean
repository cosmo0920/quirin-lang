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
# Naming an assignment's destination (card `W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE`)

`crates/qcore-c-gateway/src/legacy_c.rs`'s `direct_assignment_destination`
named a left-hand side not headed by `*` by the **first suffix that spells a
place** (`canonical_place`: a name followed by `.f`, `->f` and `[digits]`
selections). For a declaration that is right — `struct T * p` declares and
initializes `p` — but for a store into a sub-object it names the wrong object:
`recs [ i ] . name` is named `name`. The analyzer then binds the right-hand
side's identity to the local `name`, and `x_free(name)` "releases" it, so
`recs[i].name = x_dup(s); x_free(name);` certified `Safe` while the C leaks one
object and frees another (hard invariant 1).

The repair names a destination only when the suffix **is the whole left-hand
side**, or when the tokens before it are a *declarator prefix* — a nonempty run
of identifiers, `*` and `,` — so that the suffix is the declared object. Every
other left-hand side that had a place-spelling suffix is a store into a
sub-object, and the classifier's lexical indirect-access gate refuses it (the
typed scalar-access routes are consulted after that refusal exactly as for any
other `a.b = …`).

This module models the two readers over an abstract token alphabet and proves:

* **Refinement** — the repaired reader names nothing the old one did not
  (`the_repair_names_nothing_new`), so it can only take a name away.
* **No moved lowering for an exact place or a declaration** — whenever the whole
  left-hand side is a place, or the prefix is a declarator prefix, both readers
  give the same answer (`an_exact_place_keeps_its_name`,
  `a_declaration_keeps_its_name`).
* **What a name now means** — a named destination is the whole left-hand side
  or a declarator's object: no selection, subscript, call or parenthesis
  precedes it (`a_named_destination_follows_no_selection`).
* **The verdict direction** — with the statements that lost their name refused,
  and a refused statement never certifying (the fact
  `QCore.RefusedWrites.a_refusal_never_certifies` records for `transfer`), a
  function the repaired classifier certifies was lowered exactly as before and
  was certified before: the repair never promotes (`the_repair_never_promotes`).
* **Non-vacuity** — the card's fixture: the old reader names
  `recs [ i ] . name` as `name`, the repaired one names nothing, and in a
  two-location store model the old binding claims the acquired object is
  released while it is in fact still held by `recs[i].name`
  (`the_old_name_releases_the_wrong_object`).

## Premises, not theorems

The token alphabet abstracts the importer's lexer, which emits one token per
punctuation character (`->` is `-` `>`). `canonical_place`'s parenthesis
stripping is not modelled: `stripParentheses` removes only a group that wraps
the **whole** slice, so it can only turn a suffix that would not otherwise spell
a place into one that does at the *same* start index, and the refinement and
exact-place facts are stated for the reader it feeds. That the Rust readers
transcribe these definitions is a premise, as in `QCore.RefusedWrites`.
-/

namespace QCore.AssignmentDestinationSuffix

/-- The importer's token classes that the two readers distinguish. -/
inductive Tok where
  | ident (n : Nat)
  | num (n : Nat)
  | star
  | comma
  | dot
  | minus
  | gt
  | lbrack
  | rbrack
  | lparen
  | rparen
  | other
deriving DecidableEq, Repr

/-- The selections after the root of a place: `. f`, `- > f`, `[ digits ]`. -/
def isSelections : List Tok → Bool
  | [] => true
  | Tok.dot :: Tok.ident _ :: rest => isSelections rest
  | Tok.minus :: Tok.gt :: Tok.ident _ :: rest => isSelections rest
  | Tok.lbrack :: Tok.num _ :: Tok.rbrack :: rest => isSelections rest
  | _ => false

/-- `canonical_place`'s shape test: a name and then selections only. -/
def isPlace : List Tok → Bool
  | Tok.ident _ :: rest => isSelections rest
  | _ => false

/-- The first index at or after `k` whose suffix spells a place. -/
def firstFrom (l : List Tok) : Nat → Nat → Option Nat
  | _, 0 => none
  | k, fuel + 1 => if isPlace (l.drop k) then some k else firstFrom l (k + 1) fuel

/-- `first_suffix_place`: the first index whose suffix spells a place. -/
def firstSuffix (l : List Tok) : Option Nat := firstFrom l 0 l.length

/-- The old reader: the first place-spelling suffix, wherever it starts. -/
def oldDestination (l : List Tok) : Option (List Tok) :=
  (firstSuffix l).map l.drop

/-- `is_declarator_prefix`: a nonempty run of identifiers, `*` and `,`. -/
def declaratorToken : Tok → Bool
  | Tok.ident _ => true
  | Tok.star => true
  | Tok.comma => true
  | _ => false

def isDeclaratorPrefix (p : List Tok) : Bool := !p.isEmpty && p.all declaratorToken

/-- The repaired reader: the suffix is named only when it is the whole
    left-hand side or follows a declarator prefix. -/
def newDestination (l : List Tok) : Option (List Tok) :=
  match firstSuffix l with
  | none => none
  | some k => if k = 0 || isDeclaratorPrefix (l.take k) then some (l.drop k) else none

/-- A left-hand side the old reader named and the repaired one does not: a
    store into a sub-object, which the lexical gate now refuses. -/
def namesASubObject (l : List Tok) : Bool :=
  (oldDestination l).isSome && (newDestination l).isNone

/-! ## 1. Refinement and preservation -/

theorem the_repair_names_nothing_new (l : List Tok) (p : List Tok) :
    newDestination l = some p → oldDestination l = some p := by
  unfold newDestination oldDestination
  cases h : firstSuffix l with
  | none => simp
  | some k =>
      simp only [Option.map_some]
      split
      · intro hp; exact hp
      · intro hp; cases hp

theorem firstFrom_self (l : List Tok) (k fuel : Nat) (h : isPlace (l.drop k) = true) :
    firstFrom l k (fuel + 1) = some k := by
  simp [firstFrom, h]

/-- The whole left-hand side spells a place: both readers name it whole. -/
theorem an_exact_place_keeps_its_name (l : List Tok) (h : isPlace l = true) :
    newDestination l = some l ∧ oldDestination l = some l := by
  have hlen : l.length ≠ 0 := by
    intro h0
    rw [List.length_eq_zero_iff] at h0
    subst h0
    simp [isPlace] at h
  obtain ⟨fuel, hf⟩ : ∃ fuel, l.length = fuel + 1 := Nat.exists_eq_succ_of_ne_zero hlen
  have hfirst : firstSuffix l = some 0 := by
    unfold firstSuffix
    rw [hf]
    exact firstFrom_self l 0 fuel (by simpa using h)
  simp [newDestination, oldDestination, hfirst]

/-- Whenever the first place-spelling suffix follows a declarator prefix (or
    nothing), the repaired reader answers exactly as the old one. -/
theorem a_declaration_keeps_its_name (l : List Tok) (k : Nat)
    (hk : firstSuffix l = some k) (hp : k = 0 ∨ isDeclaratorPrefix (l.take k) = true) :
    newDestination l = oldDestination l := by
  unfold newDestination oldDestination
  rw [hk]
  rcases hp with h0 | hd
  · simp [h0]
  · simp [hd]

/-- The two readers disagree exactly on the sub-object stores. -/
theorem the_readers_differ_only_on_sub_objects (l : List Tok) :
    newDestination l ≠ oldDestination l → namesASubObject l = true := by
  intro hne
  unfold namesASubObject
  unfold newDestination oldDestination at *
  cases hk : firstSuffix l with
  | none => simp [hk] at hne
  | some k =>
      simp only [hk, Option.map_some] at hne ⊢
      split at hne
      · exact absurd rfl hne
      · rename_i hcond
        simp [hcond]

/-! ## 2. What a name now means -/

theorem firstFrom_ge (l : List Tok) :
    ∀ (fuel k j : Nat), firstFrom l k fuel = some j → k ≤ j ∧ isPlace (l.drop j) = true
  | 0, _, _, h => by simp [firstFrom] at h
  | fuel + 1, k, j, h => by
      unfold firstFrom at h
      split at h
      · rename_i hp
        cases h
        exact ⟨Nat.le_refl _, hp⟩
      · have ⟨hle, hp⟩ := firstFrom_ge l fuel (k + 1) j h
        exact ⟨Nat.le_of_succ_le hle, hp⟩

/-- A selection-bearing token: one a prefix before a sub-object's member name
    necessarily carries. -/
def selectionToken : Tok → Bool
  | Tok.dot => true
  | Tok.gt => true
  | Tok.rbrack => true
  | Tok.rparen => true
  | _ => false

/-- A named destination is a place, and the tokens before it — if any — are a
    declarator prefix: no `.`, `->`, `]` or `)` precedes it. -/
theorem a_named_destination_follows_no_selection (l p : List Tok)
    (h : newDestination l = some p) :
    isPlace p = true ∧ ∃ pre, l = pre ++ p ∧ (pre = [] ∨ isDeclaratorPrefix pre = true) ∧
      ∀ t ∈ pre, selectionToken t = false := by
  unfold newDestination at h
  cases hk : firstSuffix l with
  | none => simp [hk] at h
  | some k =>
      rw [hk] at h
      simp only at h
      split at h
      · rename_i hcond
        cases h
        have hplace := (firstFrom_ge l l.length 0 k hk).2
        refine ⟨hplace, l.take k, (List.take_append_drop k l).symm, ?_, ?_⟩
        · rcases Bool.or_eq_true_iff.mp hcond with h0 | hd
          · left
            have : k = 0 := by simpa using h0
            subst this
            simp
          · right; exact hd
        · intro t ht
          rcases Bool.or_eq_true_iff.mp hcond with h0 | hd
          · have : k = 0 := by simpa using h0
            subst this
            simp at ht
          · have hall : (l.take k).all declaratorToken = true := by
              unfold isDeclaratorPrefix at hd
              simp only [Bool.and_eq_true] at hd
              exact hd.2
            have ht' := List.all_eq_true.mp hall t ht
            cases t <;> simp_all [declaratorToken, selectionToken]
      · cases h

/-! ## 3. The verdict direction

A function body is abstracted to its left-hand sides; `lower` maps each to its
instruction given the destination the reader names. A statement the repaired
reader stopped naming is refused. The status function is abstract, and carries
only the fact `QCore.RefusedWrites.a_refusal_never_certifies` records for
`transfer`: a body containing a refusal is not `safe`. -/

inductive Instr where
  | refused
  | lowered (destination : Option (List Tok))
deriving DecidableEq, Repr

/-- The old classifier: the old reader's name, whatever it is. -/
def oldLower (l : List Tok) : Instr := Instr.lowered (oldDestination l)

/-- The repaired classifier: a sub-object store is refused, everything else is
    lowered with the (unchanged) name. -/
def newLower (l : List Tok) : Instr :=
  if namesASubObject l then Instr.refused else Instr.lowered (newDestination l)

theorem newLower_eq_old_or_refused (l : List Tok) :
    newLower l = oldLower l ∨ newLower l = Instr.refused := by
  unfold newLower oldLower
  split
  · right; rfl
  · rename_i hs
    left
    have : newDestination l = oldDestination l := by
      by_cases hne : newDestination l = oldDestination l
      · exact hne
      · exact absurd (the_readers_differ_only_on_sub_objects l hne) hs
    rw [this]

/-- A status that never certifies a body carrying a refusal. -/
structure Status where
  safe : List Instr → Bool
  refusal_blocks : ∀ body, Instr.refused ∈ body → safe body = false

/-- The repair never promotes: a body the repaired classifier certifies was
    lowered exactly as before, and so was certified before. -/
theorem the_repair_never_promotes (st : Status) (body : List (List Tok))
    (h : st.safe (body.map newLower) = true) :
    body.map newLower = body.map oldLower ∧ st.safe (body.map oldLower) = true := by
  have hsame : body.map newLower = body.map oldLower := by
    apply List.map_congr_left
    intro l hl
    rcases newLower_eq_old_or_refused l with he | hr
    · exact he
    · exfalso
      have hin : Instr.refused ∈ body.map newLower :=
        List.mem_map.mpr ⟨l, hl, hr⟩
      have := st.refusal_blocks _ hin
      rw [h] at this
      cases this
  exact ⟨hsame, hsame ▸ h⟩

/-! ## 4. Non-vacuity: the card's fixture -/

/-- `recs [ i ] . name`, with `recs = 0`, `i = 1`, `name = 2`. -/
def fixture : List Tok :=
  [Tok.ident 0, Tok.lbrack, Tok.ident 1, Tok.rbrack, Tok.dot, Tok.ident 2]

theorem the_old_reader_names_the_member_as_the_local :
    oldDestination fixture = some [Tok.ident 2] := by decide

theorem the_repaired_reader_names_nothing :
    newDestination fixture = none := by decide

theorem the_fixture_is_a_sub_object_store : namesASubObject fixture = true := by decide

theorem the_fixture_is_refused : newLower fixture = Instr.refused := by decide

/-- `struct T * p` (`struct = 3`, `T = 4`, `p = 5`) keeps its name. -/
theorem a_pointer_declaration_is_still_named :
    newDestination [Tok.ident 3, Tok.ident 4, Tok.star, Tok.ident 5] =
      some [Tok.ident 5] := by decide

/-- `p - > f` is a whole place and keeps its (selection) name. -/
theorem a_whole_member_place_is_still_named :
    newDestination [Tok.ident 5, Tok.minus, Tok.gt, Tok.ident 6] =
      some [Tok.ident 5, Tok.minus, Tok.gt, Tok.ident 6] := by decide

/-- `( * p ) - > f`: the old reader named `f`; the repaired one names nothing. -/
theorem a_parenthesised_dereference_is_a_sub_object :
    oldDestination [Tok.lparen, Tok.star, Tok.ident 5, Tok.rparen, Tok.minus, Tok.gt,
      Tok.ident 6] = some [Tok.ident 6] ∧
    newDestination [Tok.lparen, Tok.star, Tok.ident 5, Tok.rparen, Tok.minus, Tok.gt,
      Tok.ident 6] = none := by decide

/-! A two-location store model of the fixture: `recs[i].name` and the local
`name` are distinct locations. `recs[i].name = x_dup(s)` stores the acquired
object into the location its left-hand side designates; `x_free(name)` releases
whatever the local holds. -/

inductive Loc where
  | local_ (n : Nat)
  | member (base field : Nat)
deriving DecidableEq, Repr

/-- The object each location holds after the store (`0` is null). -/
def storeInto (target : Loc) (object : Nat) : Loc → Nat :=
  fun at_ => if at_ = target then object else 0

/-- The object `x_free(name)` releases: the local's value. -/
def releasedBy (memory : Loc → Nat) : Nat := memory (Loc.local_ 2)

/-- In C the acquired object `7` is stored into `recs[i].name`, so
    `x_free(name)` releases null and `7` is still held by the member; the old
    name stored it into the local and "released" it. -/
theorem the_old_name_releases_the_wrong_object :
    releasedBy (storeInto (Loc.member 0 2) 7) = 0 ∧
      storeInto (Loc.member 0 2) 7 (Loc.member 0 2) = 7 ∧
      releasedBy (storeInto (Loc.local_ 2) 7) = 7 := by decide

end QCore.AssignmentDestinationSuffix
