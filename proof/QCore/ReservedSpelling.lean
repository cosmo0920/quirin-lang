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
# A held call is only ever tool-minted

Card **W45-SR5-SOUNDNESS** (`HELD-PREFIX-SOURCE-FORGES-PENDING`,
`crates/qcore-c-gateway/src/legacy_reserved_spelling.rs`). A rewrite marks a
call site by prefixing its callee's name with a marker, and every later reader
classifies a call by that prefix alone; the pending-review and pin-mismatch
markers classify as `Pending`. STABILITY-REEVAL-5 measured a source that
spelled the marker itself publishing `Pending`.

A name is a list of characters (`Nat` codes). The escape runs first: every
source name that starts with a reserved prefix (a marker, or the escape prefix
itself) gets the escape prefix prepended. Then a rewrite mints `m ++ w` for
every name `w` in the environment's withheld set `W`.

## What is proved

* **No escaped name carries a marker**, when the escape prefix and every
  marker diverge (neither is a prefix of the other; the Rust constants do).
* **A marked name after minting was minted**: it is `m ++ w` with `w ∈ W`, so
  a call classified as held names a member of the tool's own withheld set.
* Hence **`Pending` arises only from a tool-minted reason**: the verdict of a
  call is `Pending` only for a callee the environment withheld.
* A source-spelled marker is never held, the escape is the identity on every
  name that spells no reserved prefix, and a witness pins the measured shape.

## What this is not

That the Rust lexer yields the identifiers this model calls names, and that
the rewrites run in the order the model composes (escape, then mint), are
premises (trust boundary 28). Sequential rewrites (review, then pin) compose
under the premise that no withheld name is itself marked.
-/

namespace QCore.ReservedSpelling

/-- A name, as its character codes. -/
abbrev Name := List Nat

/-- `startsWith n p`: `p` is a prefix of `n`. -/
def startsWith : Name → Name → Bool
  | _, [] => true
  | [], _ :: _ => false
  | a :: as, b :: bs => a == b && startsWith as bs

/-- `diverge a b`: at some position both have, `a` and `b` differ, so neither
is a prefix of the other. -/
def diverge : Name → Name → Bool
  | a :: as, b :: bs => a != b || diverge as bs
  | _, _ => false

/-- A name starting with a prefix that diverges from `m` does not start with `m`. -/
theorem diverge_append_not_startsWith (e m x : Name) (h : diverge e m = true) :
    startsWith (e ++ x) m = false := by
  induction e generalizing m with
  | nil => simp [diverge] at h
  | cons a as ih =>
    cases m with
    | nil => simp [diverge] at h
    | cons b bs =>
      simp only [diverge, Bool.or_eq_true, bne_iff_ne, ne_eq] at h
      simp only [List.cons_append, startsWith, Bool.and_eq_false_iff, beq_eq_false_iff_ne, ne_eq]
      rcases h with h | h
      · exact Or.inl h
      · exact Or.inr (ih bs h)

/-- The reserved prefixes: the rewrite markers and the escape prefix. -/
structure Prefixes where
  markers : List Name
  escape : Name

def reserved (P : Prefixes) (n : Name) : Bool :=
  P.markers.any (fun m => startsWith n m) || startsWith n P.escape

/-- The escape of one name. -/
def escapeName (P : Prefixes) (n : Name) : Name :=
  if reserved P n then P.escape ++ n else n

/-- The escape of a stream. -/
def escapeStream (P : Prefixes) (s : List Name) : List Name := s.map (escapeName P)

/-- The premise the Rust constants satisfy: the escape prefix diverges from
every marker. -/
def WellSeparated (P : Prefixes) : Prop := ∀ m ∈ P.markers, diverge P.escape m = true

/-- **No escaped name carries a marker.** -/
theorem an_escaped_name_carries_no_marker (P : Prefixes) (sep : WellSeparated P) (n : Name)
    (m : Name) (hm : m ∈ P.markers) : startsWith (escapeName P n) m = false := by
  unfold escapeName
  split
  · exact diverge_append_not_startsWith _ _ _ (sep m hm)
  · rename_i h
    simp only [reserved, Bool.or_eq_true, List.any_eq_true, not_or, not_exists, not_and,
      Bool.not_eq_true] at h
    exact h.1 m hm

/-- **No escaped stream carries a marker.** -/
theorem an_escaped_stream_carries_no_marker (P : Prefixes) (sep : WellSeparated P)
    (s : List Name) (m : Name) (hm : m ∈ P.markers) :
    ∀ n ∈ escapeStream P s, startsWith n m = false := by
  intro n hn
  obtain ⟨x, _, rfl⟩ := List.mem_map.mp hn
  exact an_escaped_name_carries_no_marker P sep x m hm

/-- A source name spelled with a marker is never read back as marked. -/
theorem a_source_spelled_marker_is_not_held (P : Prefixes) (sep : WellSeparated P) (n m : Name)
    (hm : m ∈ P.markers) (_spelled : startsWith n m = true) :
    startsWith (escapeName P n) m = false :=
  an_escaped_name_carries_no_marker P sep n m hm

/-- The escape is the identity on every name that spells no reserved prefix. -/
theorem the_escape_keeps_every_other_name (P : Prefixes) (n : Name) (h : reserved P n = false) :
    escapeName P n = n := by
  simp [escapeName, h]

/-- A rewrite with marker `m` mints `m ++ w` for every name in the withheld set. -/
def mint (m : Name) (W : List Name) (n : Name) : Name := if n ∈ W then m ++ n else n

/-- `m ++ w` starts with `m`. -/
theorem startsWith_append (m w : Name) : startsWith (m ++ w) m = true := by
  induction m with
  | nil => cases w <;> rfl
  | cons a as ih => simp [startsWith, ih]

/-- **A marked name after minting was minted from the withheld set.** -/
theorem a_marked_name_was_minted (P : Prefixes) (sep : WellSeparated P) (m : Name)
    (hm : m ∈ P.markers) (W : List Name) (x : Name)
    (marked : startsWith (mint m W (escapeName P x)) m = true) :
    ∃ w ∈ W, mint m W (escapeName P x) = m ++ w := by
  unfold mint at marked ⊢
  split
  · rename_i hw
    exact ⟨_, hw, rfl⟩
  · rename_i hw
    simp only [hw, ↓reduceIte] at marked
    rw [an_escaped_name_carries_no_marker P sep x m hm] at marked
    exact absurd marked (by decide)

/-- The verdict a call contributes: a held (marked) callee is `Pending`, an
unmarked uncontracted one `Unknown`. -/
inductive Verdict where
  | pending
  | unknown
  deriving DecidableEq, Repr

def callVerdict (m : Name) (callee : Name) : Verdict :=
  if startsWith callee m then .pending else .unknown

/-- **`Pending` only from a tool-minted reason**: after the escape and the
mint, a call is `Pending` only for a callee the environment withheld. -/
theorem pending_only_from_a_minted_hold (P : Prefixes) (sep : WellSeparated P) (m : Name)
    (hm : m ∈ P.markers) (W : List Name) (x : Name)
    (pending : callVerdict m (mint m W (escapeName P x)) = .pending) :
    ∃ w ∈ W, mint m W (escapeName P x) = m ++ w := by
  apply a_marked_name_was_minted P sep m hm W x
  unfold callVerdict at pending
  split at pending
  · assumption
  · exact absurd pending (by decide)

/-- A genuine hold stays `Pending`: the escape does not touch a withheld name
that spells no reserved prefix, and minting marks it. -/
theorem a_genuine_hold_is_pending (P : Prefixes) (m : Name) (W : List Name) (w : Name)
    (hw : w ∈ W) (plain : reserved P w = false) :
    callVerdict m (mint m W (escapeName P w)) = .pending := by
  rw [the_escape_keeps_every_other_name P w plain]
  simp [callVerdict, mint, hw, startsWith_append]

/-! ## Witness: the measured shape

`qcore_pin_mismatch__zz` in the source, with nothing withheld: `Unknown`
after the escape (`Pending` before it). Codes stand for characters: marker
`[1, 2]`, escape `[1, 9]`. -/

def witness : Prefixes := { markers := [[1, 2]], escape := [1, 9] }

theorem witness_is_well_separated : WellSeparated witness := by
  intro m hm
  simp [witness] at hm
  subst hm
  decide

theorem the_measured_forgery_was_pending : callVerdict [1, 2] [1, 2, 7] = .pending := by decide

theorem the_measured_forgery_is_unknown :
    callVerdict [1, 2] (mint [1, 2] [] (escapeName witness [1, 2, 7])) = .unknown := by decide

end QCore.ReservedSpelling
