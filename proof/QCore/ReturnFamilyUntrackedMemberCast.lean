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
# An untracked member or subscript root through a cast group is a base (card RETURN-FAMILY-UNTRACKED-MEMBER-THROUGH-CAST, rule 74t)

Family S of `legacy_return_family` admits any use of an untracked root, but a
root that `->` or `[` accesses is a **base** of the read node (and a `->` root
carries its C02-P-ALIAS reading), so the flow check stands behind the
syntactic premise: a possibly-null binding gives `unknown_alias`. At the base
the root was a member or subscript base only when `->` or `[` came right after
the name (`oldMember`). So `( ( T * ) p ) - > n` made no base, and
`long w(void) { const void *p = 0; return ((const struct s *)p)->n; }` was
`Safe` (`old_misses_the_base`).

Rule 74t keeps that test and adds one (`accessed`). It reads back from the
root through `(` and through every group `cast_semantics::type_group_open`
reads as a type, counting the opening parentheses (`opened`). It then reads
forward through at most that many `)` (`closeRun`), and requires at least
one. The root is accessed when `->` or `[` follows. The root is a base when
either test says so (`member`), and a `->` root when either says `->`
(`arrow`).

* The old tests imply the new ones: every base and `->` root the base read is
  kept (`old_member_kept`, `old_arrow_kept`, `old_bases_kept`), and family S
  refuses exactly where it refused, since the change touches no refusal.
* A base or a `->` root only adds an obligation, so a result the tip admits
  the base admitted too (`the_fix_only_refuses_more`). No reading admits
  more than at the base.
* `type_group_open` asks no resolution, so a group rule 74h would call
  undecided is read as a cast. The operand reading of such a group stops the
  walk, and every root that reading accesses the cast reading accesses too
  (`every_group_reading_keeps_the_base`, via `accessed_mono`).
* A word of a cast's type name is followed inside its group by another word
  or a `*`, or by the `)` before an operand, never by `)` then `->` or `[`. So
  the new test never makes it a root (`a_type_word_is_no_root`).
* Closing parentheses past the ones the walk opened are not read, so a group
  with an operand before the root (`( n + p ) - > m`) is no access of the root
  (`an_operand_before_the_root_is_no_access`).

## What this is not

A model of the token scan or of the flow check. Each item's class is the Rust
reader's output (trust boundary 28), bound by `memberVectors`. Bases' and `->`
roots' obligations are abstracted as predicates that the flow check decides.
-/

namespace QCore.ReturnFamilyUntrackedMemberCast

/-- One token step back from the untracked root, nearest first. -/
inductive Back where
  /-- A `(`. -/
  | opening
  /-- A `)` closing a group `type_group_open` reads as a type. -/
  | cast
  /-- Anything else: the walk stops. -/
  | other
  deriving DecidableEq, Repr

/-- One token step forward from the untracked root, nearest first. -/
inductive Fwd where
  /-- A `)`. -/
  | closing
  /-- `- >`. -/
  | arrow
  /-- `[`. -/
  | bracket
  /-- Anything else. -/
  | other
  deriving DecidableEq, Repr

/-- The opening parentheses the walk back crosses, under a reading of the
type groups: `true` reads each as a cast (transparent, `type_group_open`'s
reading), `false` as an operand (the walk stops there). -/
def openedUnder (castReading : Bool) : List Back → Nat
  | .opening :: rest => openedUnder castReading rest + 1
  | .cast :: rest => if castReading then openedUnder castReading rest else 0
  | _ => 0

/-- The reader's count: every type group is transparent. -/
def opened (b : List Back) : Nat := openedUnder true b

/-- At most `n` closing parentheses read forward: how many, and what is left. -/
def closeRun : Nat → List Fwd → Nat × List Fwd
  | n + 1, .closing :: rest => ((closeRun n rest).1 + 1, (closeRun n rest).2)
  | _, l => (0, l)

/-- The access a token list starts with: `some true` for `->`, `some false`
for `[`. -/
def access : List Fwd → Option Bool
  | .arrow :: _ => some true
  | .bracket :: _ => some false
  | _ => none

/-- Rule 74t's test under a reading of the type groups. -/
def accessedUnder (castReading : Bool) (b : List Back) (f : List Fwd) : Option Bool :=
  if (closeRun (openedUnder castReading b) f).1 = 0 then none
  else access (closeRun (openedUnder castReading b) f).2

/-- Rule 74t's test (`accessed_through_groups`). -/
def accessed (b : List Back) (f : List Fwd) : Option Bool := accessedUnder true b f

/-- The base's member and subscript test: `->` or `[` right after the name. -/
def oldMember (f : List Fwd) : Bool := (access f).isSome

/-- The base's `->` root. -/
def oldArrow (f : List Fwd) : Bool := access f == some true

/-- Rule 74t's member or subscript base: either test. -/
def member (b : List Back) (f : List Fwd) : Bool := oldMember f || (accessed b f).isSome

/-- Rule 74t's `->` root: either test. -/
def arrow (b : List Back) (f : List Fwd) : Bool := oldArrow f || accessed b f == some true

theorem old_member_kept (b : List Back) (f : List Fwd) (h : oldMember f = true) :
    member b f = true := by
  simp [member, h]

theorem old_arrow_kept (b : List Back) (f : List Fwd) (h : oldArrow f = true) :
    arrow b f = true := by
  simp [arrow, h]

/-! ## Every reading of a type group keeps the base -/

theorem closeRun_le : ∀ (n : Nat) (f : List Fwd), (closeRun n f).1 ≤ n
  | 0, f => by cases f <;> simp [closeRun]
  | n + 1, [] => by simp [closeRun]
  | n + 1, .closing :: rest => by
      have := closeRun_le n rest
      simp [closeRun]
      omega
  | n + 1, .arrow :: _ => by simp [closeRun]
  | n + 1, .bracket :: _ => by simp [closeRun]
  | n + 1, .other :: _ => by simp [closeRun]

/-- When what is left starts with an access, more allowance reads the same
closing parentheses. -/
theorem closeRun_stable :
    ∀ (n m : Nat) (f : List Fwd), n ≤ m → (access (closeRun n f).2).isSome = true →
      closeRun m f = closeRun n f
  | 0, 0, _, _, _ => rfl
  | 0, m + 1, [], _, _ => by simp [closeRun]
  | 0, m + 1, .closing :: _, _, h => by simp [closeRun, access] at h
  | 0, m + 1, .arrow :: _, _, _ => by simp [closeRun]
  | 0, m + 1, .bracket :: _, _, _ => by simp [closeRun]
  | 0, m + 1, .other :: _, _, _ => by simp [closeRun]
  | n + 1, 0, _, hle, _ => by omega
  | n + 1, m + 1, [], _, _ => by simp [closeRun]
  | n + 1, m + 1, .closing :: rest, hle, h => by
      have ih := closeRun_stable n m rest (by omega) (by simpa [closeRun] using h)
      simp [closeRun, ih]
  | n + 1, m + 1, .arrow :: _, _, _ => by simp [closeRun]
  | n + 1, m + 1, .bracket :: _, _, _ => by simp [closeRun]
  | n + 1, m + 1, .other :: _, _, _ => by simp [closeRun]

theorem opened_mono : ∀ b : List Back, openedUnder false b ≤ openedUnder true b
  | [] => by simp [openedUnder]
  | .opening :: rest => by
      have := opened_mono rest
      simp [openedUnder]
      omega
  | .cast :: _ => by simp [openedUnder]
  | .other :: _ => by simp [openedUnder]

/-- The operand reading of the type groups accesses no root the cast reading
misses: the walk crosses fewer opening parentheses, and an access that
follows fewer closing ones follows the same ones under a larger allowance. -/
theorem accessed_mono (b : List Back) (f : List Fwd) (x : Bool)
    (h : accessedUnder false b f = some x) : accessed b f = some x := by
  unfold accessed accessedUnder at *
  by_cases hz : (closeRun (openedUnder false b) f).1 = 0
  · simp [hz] at h
  · simp only [hz, ite_false] at h
    have hs := closeRun_stable (openedUnder false b) (openedUnder true b) f (opened_mono b)
      (by simp [h])
    rw [hs]
    simp [hz, h]

theorem every_group_reading_keeps_the_base (b : List Back) (f : List Fwd)
    (h : (accessedUnder false b f).isSome = true) : member b f = true := by
  obtain ⟨x, hx⟩ := Option.isSome_iff_exists.mp h
  simp [member, accessed_mono b f x hx]

/-! ## What the new test never reads -/

/-- A word of a cast's type name: followed by another word or a `*` inside
its group, or by the group's `)` and then an operand. -/
theorem a_type_word_is_no_root (b : List Back) (rest : List Fwd) :
    accessed b (.other :: rest) = none ∧ accessed b (.closing :: .other :: rest) = none := by
  constructor
  · unfold accessed accessedUnder
    cases opened_eq : openedUnder true b <;> simp [closeRun]
  · unfold accessed accessedUnder
    cases opened_eq : openedUnder true b with
    | zero => simp [closeRun]
    | succ n => cases n <;> simp [closeRun, access]

/-- `( n + p ) - > m`: the walk back stops at the operand, opens nothing, and
reads no `)`. -/
theorem an_operand_before_the_root_is_no_access (rest : List Back) (f : List Fwd) :
    accessed (.other :: rest) f = none := by
  simp [accessed, accessedUnder, openedUnder, closeRun]

/-! ## Bases only add obligations -/

/-- One untracked root: its name, its items back and its items forward. -/
abbrev Root := String × List Back × List Fwd

/-- The member or subscript bases under a test. -/
def basesOf (test : List Back → List Fwd → Bool) (roots : List Root) : List String :=
  (roots.filter (fun root => test root.2.1 root.2.2)).map Prod.fst

theorem bases_kept (old new : List Back → List Fwd → Bool)
    (h : ∀ b f, old b f = true → new b f = true) (roots : List Root) (name : String)
    (hn : name ∈ basesOf old roots) : name ∈ basesOf new roots := by
  simp only [basesOf, List.mem_map, List.mem_filter] at hn ⊢
  obtain ⟨root, ⟨hm, ht⟩, he⟩ := hn
  exact ⟨root, ⟨hm, h _ _ ht⟩, he⟩

theorem old_bases_kept (roots : List Root) (name : String)
    (h : name ∈ basesOf (fun _ f => oldMember f) roots) : name ∈ basesOf member roots :=
  bases_kept _ _ (fun b f hf => old_member_kept b f hf) roots name h

/-- Family S's result over one expression: `none` refuses; otherwise the
flow check admits only when every base's obligation `ok` and every `->`
root's C02-P-ALIAS obligation `alias` discharge. The bases another test makes
(rule 74r's starred bases, `others`) are unchanged by this card. -/
def admits (ok alias : String → Bool) : Option (List String × List String) → Bool
  | none => false
  | some (bases, arrows) => bases.all ok && arrows.all alias

/-- The change only adds bases and `->` roots and touches no refusal, so a
result the tip admits the base admitted too. -/
theorem the_fix_only_refuses_more (ok alias : String → Bool) (refused : Bool)
    (others : List String) (roots : List Root)
    (h : admits ok alias
      (if refused then none
       else some (others ++ basesOf member roots, basesOf arrow roots)) = true) :
    admits ok alias
      (if refused then none
       else some (others ++ basesOf (fun _ f => oldMember f) roots,
         basesOf (fun _ f => oldArrow f) roots)) = true := by
  cases refused with
  | true => simp [admits] at h
  | false =>
    simp only [admits, Bool.and_eq_true, List.all_eq_true, Bool.false_eq_true, ite_false,
      List.mem_append] at h ⊢
    obtain ⟨hb, ha⟩ := h
    refine ⟨fun name hn => ?_, fun name hn => ?_⟩
    · rcases hn with hn | hn
      · exact hb name (Or.inl hn)
      · exact hb name (Or.inr (old_bases_kept roots name hn))
    · exact ha name (bases_kept _ _ (fun b f hf => old_arrow_kept b f hf) roots name hn)

/-! ## The readings -/

/-- `( ( T * ) p ) - > n`: the base missed the access; the tip makes `p` a base
and a `->` root. -/
theorem old_misses_the_base :
    oldMember [.closing, .arrow] = false ∧ member [.cast, .opening] [.closing, .arrow] = true ∧
      arrow [.cast, .opening] [.closing, .arrow] = true := by
  decide

/-- `( ( long * ) p ) + n`: a converted value used as an operand is no access. -/
theorem a_converted_operand_is_no_base :
    member [.cast, .opening] [.closing, .other] = false := by
  decide

/-! ## The token reader's vectors -/

/-- `(spelling, the items back from the untracked `p` nearest first, the items
forward, a base at the base, a base, a `->` root)`. The Rust unit test reads
the spellings and the last two fields; the fourth field is pinned by
`old_vectors_hold`. -/
def memberVectors : List (String × List Back × List Fwd × Bool × Bool × Bool) := [
  ("( ( T * ) p ) - > n", [.cast, .opening], [.closing, .arrow], false, true, true),
  ("( ( long * ) p ) [ 0 ]", [.cast, .opening], [.closing, .bracket], false, true, false),
  ("( ( long * ) ( p ) ) - > n", [.opening, .cast, .opening], [.closing, .closing, .arrow],
    false, true, true),
  ("( p ) - > n", [.opening], [.closing, .arrow], false, true, true),
  ("( p ) [ 0 ]", [.opening], [.closing, .bracket], false, true, false),
  ("( ( ( long * ) p ) ) - > n", [.cast, .opening, .opening], [.closing, .closing, .arrow],
    false, true, true),
  ("( ( long * ) ( char * ) p ) - > n", [.cast, .cast, .opening], [.closing, .arrow],
    false, true, true),
  ("( ( T * ) p ) - > a [ 1 ]", [.cast, .opening], [.closing, .arrow], false, true, true),
  ("( int ) ( ( long * ) p ) [ 1 ] + n", [.cast, .opening, .cast], [.closing, .bracket],
    false, true, false),
  ("n + ( ( unsigned long * ) p ) [ 1 ]", [.cast, .opening, .other], [.closing, .bracket],
    false, true, false),
  ("p - > n", [], [.arrow], true, true, true),
  ("p [ 0 ]", [], [.bracket], true, true, false),
  ("( long ) p", [.cast], [], false, false, false),
  ("( ( long * ) p ) + n", [.cast, .opening], [.closing, .other], false, false, false),
  ("( n + p ) - > m", [.other], [.closing, .arrow], false, false, false),
  ("( ( long * ) p + n ) - > m", [.cast, .opening], [.other], false, false, false),
  ("( n + ( p ) ) - > m", [.opening, .other], [.closing, .closing, .arrow],
    false, false, false)
]

def memberVectorsAgree : Bool :=
  memberVectors.all (fun (_, b, f, _, base, arrowRoot) =>
    member b f == base && arrow b f == arrowRoot)

def oldVectorsAgree : Bool :=
  memberVectors.all (fun (_, _, f, old, _, _) => oldMember f == old)

theorem member_vectors_hold : memberVectorsAgree = true := by decide

theorem old_vectors_hold : oldVectorsAgree = true := by decide

end QCore.ReturnFamilyUntrackedMemberCast
