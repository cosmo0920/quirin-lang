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

import QCore.NullDerefGrade
import QCore.IndexQuant
import QCore.BorrowLiveness

/-!
# A dereference read through the `quant` family: existence, extent, liveness (card NULL-DEREF-QTT-GROUNDING)

The user's direction (2026-10-11): `quant null-deref` is read through the
original `quant` family, and the relations between the parts are to be derived
from the proven models, verbatim: "This can be derived from the proven lean.
And it can be natural." Normative text: contract specification C09.3
("Decomposition"), language specification §L07.

A dereference `*B` or `B->f` is safe only when three parts hold. Each part is
a projection of the same dereference use, owned by an existing system:

* **(a) existence**, `quant null-deref #B`: `B` names an identity, so it is
  not NULL (§L07 grade, §L09 identity; `QCore.NullDeref`,
  `QCore.NullDerefGrade`);
* **(b) extent**, `quant checks B, Idx #i: #i == 0` in element units of `*B`:
  `B`'s extent covers the dereferenced object (§L06, C09.1–C09.2;
  `QCore.IndexExtent`, `QCore.IndexQuant`);
* **(c) liveness**, custody's `live` obligation: the identity has not been
  released (§L08 and the custody property; `QCore.BorrowLiveness`).

Nothing below is a new rule or axiom. The one modelling choice is the concrete
pointer value `Ptr`: NULL, or a position in an identity's object with the
number of elements that remain from it and that identity's custody status.
"NULL has no extent" and "NULL names no identity" are what `Ptr.null` *is*,
not premises. A bounds fact's count is *sound* for a pointer when it does not
overstate the elements that remain (`ExtentSound`), which is what a C09.1
extent claims.

The entailments, each a theorem:

1. `checks_zero_entails_null_deref`: a caller's discharged `quant checks B,
   Idx #i: #i == 0` (the existing call-side decision, `QCore.IndexQuant`, on
   the existing element access, `QCore.IndexExtent`) puts a non-NULL value at
   `B`, which `relevant` admits and `QCore.NullDeref` may read non-null.
2. `null_deref_does_not_entail_checks_zero`, `an_unknown_extent_is_never_in_bounds`:
   the converse fails for a one-past-the-end pointer and for a non-NULL pointer
   with no known extent, so `null-deref` is a separate, weaker projection and is
   not defined as `checks #i == 0`.
3. `relevant_obligates_existence`, `relevant_obligates_a_stated_part`,
   `an_unstated_part_is_never_obligated`, `zero_lower_obligates_no_part`: a
   part's caller demand is §L07 demand substitution, the grade times the part's
   per-use demand (`one` when stated, `zero` otherwise), so a lower bound of at
   least one obligates (a) and each part the callee's contract states, and a
   lower bound of 0 obligates none.
4. `existence_and_extent_do_not_discharge_liveness`,
   `a_released_identity_is_refused_although_it_exists_and_has_extent`: a
   released identity has (a) and (b) and fails (c), so no theorem can let (a)
   or (b) discharge liveness.

**Two levels** (USER DECISION 2026-10-11, "Two levels"): contract authors may
write the shorthand `quant deref #B;` or its decomposition `quant null-deref
#B; quant checks B, Idx #i: #i == 0; quant live #B;`, or a mix, and both mean
the same. The last section models the loader's elaboration and merge
(`crates/qcore-contract/src/deref_parts.rs`):
`elaboration_preserves_meaning`, `merging_an_agreeing_overlap_is_idempotent`,
`merge_merge`, `shorthand_and_decomposition_are_one_canonical_set`,
`one_canonical_set_one_meaning`. `quant null-deref #B;` keeps its meaning: the
existence part alone.

What this is not: a change to any analysis. No consumer uses entailment 1 to
discharge a NULL obligation (a later behaviour card,
`NULL-DEREF-FROM-INDEX-ZERO-BOUNDS`). That the loader's canonical parts are
this model's `elaborate` and `merge` is a review premise (trust boundary 28),
pinned by `crates/qcore-contract/tests/deref_parts.rs`.
-/

namespace QCore.DerefDecomposition

open QCore.IndexExtent (Count IndexFact Decision elementAccess element_access_sound Describes)
open QCore.IndexQuant (Pred Rel Term Admitted admitted reading requirement bothInBounds
  both_in_bounds_sound)
open QCore.NullDerefGrade (requiresIdentity admits relevant_admits_only_an_existing_identity)

/-! ## The concrete pointer value at a dereference -/

/-- NULL, or a position in an identity's object: the elements that remain from
it (`0` at one past the end) and that identity's custody status. -/
inductive Ptr where
  | null
  | into (remaining : Nat) (status : QCore.BorrowLiveness.Status)

def Ptr.isNull : Ptr → Bool
  | .null => true
  | .into _ _ => false

/-- The elements a dereference through the pointer may reach: none through
NULL. -/
def Ptr.extent : Ptr → Nat
  | .null => 0
  | .into n _ => n

/-- Custody entitles a read of the identity the pointer names (`live` or
`contained`, `QCore.BorrowLiveness.Status.entitlesARead`); NULL names none. -/
def Ptr.live : Ptr → Bool
  | .null => false
  | .into _ st => st.entitlesARead

/-- (a) existence: the pointer names an identity. -/
abbrev existence (p : Ptr) : Prop := p.isNull = false

/-- (b) extent: the dereferenced object (element 0) is within the extent. -/
abbrev extentCovers (p : Ptr) : Prop := 1 ≤ p.extent

/-- (c) liveness: the identity has not been released. -/
abbrev liveness (p : Ptr) : Prop := p.live = true

/-- A safe dereference needs all three parts. -/
abbrev safeDeref (p : Ptr) : Prop := existence p ∧ extentCovers p ∧ liveness p

/-- A bounds fact's count is sound for the pointer when it does not overstate
the elements that remain from it (what a C09.1 extent claims). -/
abbrev ExtentSound (env : Nat → Nat) (n : Count) (p : Ptr) : Prop := n.eval env ≤ p.extent

/-- NULL has no extent: every count sound for it is zero. -/
theorem null_has_no_extent (env : Nat → Nat) (n : Count) (h : ExtentSound env n .null) :
    n.eval env = 0 := by
  simpa [ExtentSound, Ptr.extent] using h

/-! ## The clause `quant checks B, Idx #i: #i == 0` and its caller decision -/

/-- `#i == 0`. -/
def zeroCheck : Pred := .cmp .eq (.mk 0 none)

/-- Its admitted set is the bounded `[0, 0]`: the caller's obligation is the
element at index 0. -/
theorem zeroCheck_admits_index_zero (ρ : Nat → Int) :
    admitted (reading ρ zeroCheck) = .bounded 0 0 := rfl

/-- The call-side decision of `QCore.IndexQuant` for an actual at offset `o`
with extent `n`: the element accesses at the hull's two ends, joined; `none`
when the set gives no obligation. -/
def callerDecision (n : Option Count) (o : Int) (a : Admitted) : Option Decision :=
  match requirement a with
  | none => none
  | some (lo, hi) =>
      some (bothInBounds (elementAccess n (.const (o + lo))) (elementAccess n (.const (o + hi))))

/-! ## Entailment 1: `checks(B, #i == 0) ⊢ null-deref(B)` -/

/-- A discharged index-0 check puts element 0 within the count. -/
theorem zero_check_covers_element_zero (env : Nat → Nat) (ρ : Nat → Int) (n : Count)
    (h : callerDecision (some n) 0 (admitted (reading ρ zeroCheck)) = some .inBounds) :
    0 < (n.eval env : Int) := by
  rw [zeroCheck_admits_index_zero] at h
  simp only [callerDecision, requirement, Option.some.injEq] at h
  have hb := (both_in_bounds_sound _ _ h).1
  have := element_access_sound env n _ 0 hb (by simp [Describes])
  exact this.2

/-- **Entailment 1.** A caller whose `quant checks B, Idx #i: #i == 0` is
discharged against a count sound for its actual passes a non-NULL value: one
that `relevant` admits (`QCore.NullDerefGrade`) and that `QCore.NullDeref` may
soundly read non-null. NULL has no extent, so a bounds proof of index 0
discharges the NULL obligation. -/
theorem checks_zero_entails_null_deref (env : Nat → Nat) (ρ : Nat → Int) (n : Count) (p : Ptr)
    (hs : ExtentSound env n p)
    (h : callerDecision (some n) 0 (admitted (reading ρ zeroCheck)) = some .inBounds) :
    existence p ∧ admits Usage.relevant (!p.isNull) = true ∧
      ∀ ps, QCore.NullDeref.sound (.nonNull ps) p.isNull := by
  have hpos := zero_check_covers_element_zero env ρ n h
  have hne : p.isNull = false := by
    cases p with
    | null =>
        have := null_has_no_extent env n hs
        omega
    | into _ _ => rfl
  refine ⟨hne, ?_, fun _ => hne⟩
  rw [relevant_admits_only_an_existing_identity, hne]
  rfl

/-- Entailment 1 also gives (b): the extent covers the dereferenced object. -/
theorem checks_zero_entails_extent (env : Nat → Nat) (ρ : Nat → Int) (n : Count) (p : Ptr)
    (hs : ExtentSound env n p)
    (h : callerDecision (some n) 0 (admitted (reading ρ zeroCheck)) = some .inBounds) :
    extentCovers p := by
  have hpos := zero_check_covers_element_zero env ρ n h
  simp only [ExtentSound] at hs
  simp only [extentCovers]
  omega

/-! ## Entailment 2: the converse fails -/

/-- One past the end of a live object. -/
def onePast : Ptr := .into 0 QCore.BorrowLiveness.Status.liveOnly

/-- **The converse fails.** A one-past-the-end pointer exists (it is not NULL,
so `null-deref` holds of it), yet no count sound for it discharges the index-0
check: null-deref is the weaker projection, and must not be defined as
`checks #i == 0`. -/
theorem null_deref_does_not_entail_checks_zero :
    existence onePast ∧ ¬ extentCovers onePast ∧
      ∀ (env : Nat → Nat) (ρ : Nat → Int) (n : Count), ExtentSound env n onePast →
        callerDecision (some n) 0 (admitted (reading ρ zeroCheck)) ≠ some .inBounds := by
  refine ⟨rfl, by simp [extentCovers, onePast, Ptr.extent], ?_⟩
  intro env ρ n hs h
  have hpos := zero_check_covers_element_zero env ρ n h
  simp only [ExtentSound, onePast, Ptr.extent] at hs
  omega

/-- **The converse fails for an unknown extent too.** A non-NULL pointer with
no known extent never discharges the index-0 check (`missingExtent`), whatever
it points to. -/
theorem an_unknown_extent_is_never_in_bounds (ρ : Nat → Int) :
    existence (.into 1 QCore.BorrowLiveness.Status.liveOnly) ∧
      callerDecision none 0 (admitted (reading ρ zeroCheck)) =
        some (.undecided .missingExtent) := by
  refine ⟨rfl, ?_⟩
  rw [zeroCheck_admits_index_zero]
  rfl

/-! ## Entailment 3: the grade decides which parts are caller obligations -/

inductive Part where
  | existence | extent | liveness
  deriving DecidableEq, Repr

/-- A part's demand per use of `B`: `one` when the callee's contract states it
(the clause `quant null-deref #B` states existence; `quant checks B, Idx #i:
#i == 0` states extent; a `Borrowed` custody position states liveness),
`zero` otherwise. -/
def partDemand (stated : Part → Bool) (part : Part) : Usage :=
  if stated part then Usage.one else Usage.zero

/-- §L07 demand substitution: a part's caller demand is the use's grade times
the part's per-use demand (`QCore.UsageContext.application`'s scaling). -/
def callerDemand (g : Usage) (stated : Part → Bool) (part : Part) : Usage :=
  Usage.mul g (partDemand stated part)

/-- The caller is obligated to a part when its caller demand requires the
identity. -/
def obligated (g : Usage) (stated : Part → Bool) (part : Part) : Bool :=
  requiresIdentity (callerDemand g stated part)

/-- The scaling is the substitution law of `QCore.UsageContext`. -/
theorem callerDemand_is_substitution (g : Usage) (stated : Part → Bool) (part : Part)
    (x : Nat) :
    QCore.UsageContext.application (fun _ => Usage.zero) (fun _ => partDemand stated part) g x =
      Usage.add Usage.zero (callerDemand g stated part) :=
  QCore.UsageContext.application_substitution _ _ g x

private theorem obligated_iff (g : Usage) (stated : Part → Bool) (part : Part) :
    obligated g stated part = (requiresIdentity g && stated part) := by
  unfold obligated callerDemand partDemand requiresIdentity
  cases stated part
  · simp [Usage.mul, Usage.zero]
  · simp [Usage.mul, Usage.one]

/-- **(a) is a caller obligation under `relevant`**: the null-deref clause
states existence, and a grade with lower bound at least one obligates it. -/
theorem relevant_obligates_existence (g : Usage) (stated : Part → Bool)
    (hg : requiresIdentity g = true) (hc : stated .existence = true) :
    obligated g stated .existence = true := by
  rw [obligated_iff, hg, hc]; rfl

/-- **(b) and (c) are caller obligations wherever the contract states them.** -/
theorem relevant_obligates_a_stated_part (g : Usage) (stated : Part → Bool) (part : Part)
    (hg : requiresIdentity g = true) (hc : stated part = true) :
    obligated g stated part = true := by
  rw [obligated_iff, hg, hc]; rfl

/-- A part the contract does not state is never obligated, whatever the grade. -/
theorem an_unstated_part_is_never_obligated (g : Usage) (stated : Part → Bool) (part : Part)
    (hc : stated part = false) : obligated g stated part = false := by
  rw [obligated_iff, hc]; simp

/-- **With lower bound 0 (`many`) no part is a caller obligation.** -/
theorem zero_lower_obligates_no_part (g : Usage) (stated : Part → Bool) (hg : g.lower = 0) :
    ∀ part, obligated g stated part = false := by
  intro part
  rw [obligated_iff]
  simp [requiresIdentity, hg]

/-- The body's grade decides it: a body that dereferences `B` first on every
terminating path (the walk's inference) obligates every stated part; a body
with one path that does not obligates none. -/
theorem body_grade_decides_the_obligations (paths : List (List QCore.NullDeref.Event))
    (g : Usage) (hg : QCore.NullDerefGrade.bodyGrade paths = some g) (stated : Part → Bool) :
    (QCore.NullDeref.inferred (paths.map QCore.NullDerefGrade.firstOf) = true →
        ∀ part, obligated g stated part = stated part) ∧
      (QCore.NullDeref.inferred (paths.map QCore.NullDerefGrade.firstOf) = false →
        ∀ part, obligated g stated part = false) := by
  have hv := QCore.NullDerefGrade.bodyGrade_valid paths g hg
  constructor
  · intro hi part
    obtain ⟨g', hg', ha⟩ := (QCore.NullDerefGrade.relevant_accepts_body_iff_inferred paths).mpr hi
    rw [hg] at hg'; cases hg'
    have hl := (QCore.NullDerefGrade.relevant_accepts_iff g hv).mp ha
    rw [obligated_iff, (QCore.NullDerefGrade.requiresIdentity_iff g).mpr hl]
    simp
  · intro hn part
    have hz : g.lower = 0 := by
      have hna : ¬ (1 ≤ g.lower) := by
        intro hl
        have := (QCore.NullDerefGrade.relevant_accepts_body_iff_inferred paths).mp
          ⟨g, hg, (QCore.NullDerefGrade.relevant_accepts_iff g hv).mpr hl⟩
        rw [hn] at this; cases this
      omega
    exact zero_lower_obligates_no_part g stated hz part

/-! ## Entailment 4: (a) and (b) never discharge (c) -/

/-- A position at the start of a one-element object whose identity was
released on every reaching path (a dangling pointer). -/
def dangling : Ptr := .into 1 QCore.BorrowLiveness.Status.releasedOnly

/-- **Independence of liveness.** The dangling pointer has (a) and (b), and a
sound count discharges its index-0 check, yet it fails (c): no theorem can let
existence or extent discharge liveness. -/
theorem existence_and_extent_do_not_discharge_liveness :
    existence dangling ∧ extentCovers dangling ∧ ¬ liveness dangling ∧
      ExtentSound (fun _ => 0) (.const 1) dangling ∧
      callerDecision (some (.const 1)) 0 (admitted (reading (fun _ => 0) zeroCheck)) =
        some .inBounds := by
  refine ⟨rfl, by decide, by decide, by decide, ?_⟩
  rw [zeroCheck_admits_index_zero]
  rfl

/-- The custody rule refuses that identity (`QCore.BorrowLiveness`): (c) is
decided by custody alone, although (a) and (b) hold. -/
theorem a_released_identity_is_refused_although_it_exists_and_has_extent :
    existence dangling ∧ extentCovers dangling ∧
      QCore.BorrowLiveness.liveness
        (QCore.BorrowLiveness.settledAt QCore.BorrowLiveness.Status.releasedOnly) =
        .definitelyNotLive :=
  ⟨rfl, by decide, QCore.BorrowLiveness.a_settled_released_identity_is_refused⟩

/-- The three parts are independent of each other's absence: each fails alone. -/
theorem each_part_fails_alone :
    (¬ existence .null) ∧
      (existence onePast ∧ liveness onePast ∧ ¬ extentCovers onePast) ∧
      (existence dangling ∧ extentCovers dangling ∧ ¬ liveness dangling) := by
  refine ⟨by decide, ⟨rfl, by decide, by decide⟩, ⟨rfl, by decide, by decide⟩⟩

/-! ## Two levels: the shorthand `quant deref #B;` and its canonical parts

USER DECISION 2026-10-11 ("Two levels"): authors may write the shorthand, the
parts, or a mix, and both mean the same. The loader
(`crates/qcore-contract/src/deref_parts.rs`) elaborates every shorthand into
canonical parts and merges agreeing overlaps; these theorems show that the
elaboration and the merge preserve meaning in the three-part model. -/

/-- A written dereference clause on the formal at position `b`. -/
inductive Clause where
  | nullDeref (b : Nat)
  | checks (b : Nat) (p : Pred)
  | live (b : Nat)
  | deref (b : Nat)
  deriving DecidableEq, Repr

/-- What a clause says of the arguments (`args b` is the value at position
`b`, `ρ` the scalar formals): existence, every admitted index within the
extent (C09.2's meaning), liveness; the shorthand, a safe dereference. -/
def meaning (ρ : Nat → Int) (args : Nat → Ptr) : Clause → Prop
  | .nullDeref b => existence (args b)
  | .checks b p => ∀ i : Int, p.holds ρ i = true → 0 ≤ i ∧ i < ((args b).extent : Int)
  | .live b => liveness (args b)
  | .deref b => safeDeref (args b)

/-- A contract holds of the arguments when every clause does. -/
def Holds (ρ : Nat → Int) (args : Nat → Ptr) (cs : List Clause) : Prop :=
  ∀ c ∈ cs, meaning ρ args c

/-- The elaboration of one clause: the shorthand into its three parts. -/
def elabClause : Clause → List Clause
  | .deref b => [.nullDeref b, .checks b zeroCheck, .live b]
  | c => [c]

def elaborate (cs : List Clause) : List Clause := cs.flatMap elabClause

/-- The extent part's meaning is exactly (b): index 0 is within the extent. -/
theorem zero_check_means_extent (ρ : Nat → Int) (p : Ptr) :
    (∀ i : Int, zeroCheck.holds ρ i = true → 0 ≤ i ∧ i < (p.extent : Int)) ↔ extentCovers p := by
  have hz : ∀ i : Int, zeroCheck.holds ρ i = true ↔ i = 0 := by
    intro i
    simp only [zeroCheck, QCore.IndexQuant.Pred.holds, QCore.IndexQuant.Rel.holds,
      QCore.IndexQuant.Term.eval]
    exact ⟨of_decide_eq_true, decide_eq_true⟩
  constructor
  · intro h
    have := h 0 ((hz 0).mpr rfl)
    simp only [extentCovers]
    omega
  · intro h i hi
    have hi0 := (hz i).mp hi
    subst hi0
    simp only [extentCovers] at h
    omega

/-- **The shorthand means its decomposition.** -/
theorem shorthand_means_its_parts (ρ : Nat → Int) (args : Nat → Ptr) (b : Nat) :
    meaning ρ args (.deref b) ↔ Holds ρ args (elabClause (.deref b)) := by
  simp only [meaning, safeDeref, Holds, elabClause, List.mem_cons, List.not_mem_nil, or_false,
    forall_eq_or_imp, forall_eq]
  rw [zero_check_means_extent]

theorem elab_preserves (ρ : Nat → Int) (args : Nat → Ptr) (c : Clause) :
    meaning ρ args c ↔ Holds ρ args (elabClause c) := by
  cases c with
  | deref b => exact shorthand_means_its_parts ρ args b
  | nullDeref b => simp [Holds, elabClause]
  | checks b p => simp [Holds, elabClause]
  | live b => simp [Holds, elabClause]

/-- **Elaboration preserves meaning**: a contract and its elaboration hold of
exactly the same arguments. -/
theorem elaboration_preserves_meaning (ρ : Nat → Int) (args : Nat → Ptr) (cs : List Clause) :
    Holds ρ args cs ↔ Holds ρ args (elaborate cs) := by
  induction cs with
  | nil => simp [Holds, elaborate]
  | cons c cs ih =>
      have hc := elab_preserves ρ args c
      simp only [Holds, elaborate, List.flatMap_cons, List.mem_append, List.mem_cons] at ih hc ⊢
      constructor
      · rintro h d (hd | hd)
        · exact hc.mp (h c (Or.inl rfl)) d hd
        · exact ih.mp (fun e he => h e (Or.inr he)) d hd
      · intro h e he
        rcases he with rfl | he
        · exact hc.mpr (fun d hd => h d (Or.inl hd))
        · exact ih.mpr (fun d hd => h d (Or.inr hd)) e he

/-- Merge one canonical part: an agreeing (equal) part already present is
merged, anything else appended. -/
def mergeInto (acc : List Clause) (c : Clause) : List Clause :=
  if c ∈ acc then acc else acc ++ [c]

def merge (cs : List Clause) : List Clause := cs.foldl mergeInto []

/-- The canonical part set of a contract. -/
def canonical (cs : List Clause) : List Clause := merge (elaborate cs)

private theorem mem_foldl_mergeInto : ∀ (cs acc : List Clause) (c : Clause),
    c ∈ cs.foldl mergeInto acc ↔ c ∈ acc ∨ c ∈ cs
  | [], acc, c => by simp
  | d :: ds, acc, c => by
      simp only [List.foldl_cons]
      rw [mem_foldl_mergeInto ds (mergeInto acc d) c]
      unfold mergeInto
      split
      · rename_i hd
        simp only [List.mem_cons]
        constructor
        · rintro (h | h)
          · exact Or.inl h
          · exact Or.inr (Or.inr h)
        · rintro (h | h | h)
          · exact Or.inl h
          · subst h; exact Or.inl hd
          · exact Or.inr h
      · simp only [List.mem_append, List.mem_cons, List.not_mem_nil, or_false]
        constructor
        · rintro ((h | h) | h)
          · exact Or.inl h
          · exact Or.inr (Or.inl h)
          · exact Or.inr (Or.inr h)
        · rintro (h | h | h)
          · exact Or.inl (Or.inl h)
          · exact Or.inl (Or.inr h)
          · exact Or.inr h

theorem mem_merge (cs : List Clause) (c : Clause) : c ∈ merge cs ↔ c ∈ cs := by
  simp [merge, mem_foldl_mergeInto]

/-- Merging preserves meaning. -/
theorem merge_preserves_meaning (ρ : Nat → Int) (args : Nat → Ptr) (cs : List Clause) :
    Holds ρ args (merge cs) ↔ Holds ρ args cs := by
  simp only [Holds, mem_merge]

private theorem foldl_mergeInto_of_mem : ∀ (ys acc : List Clause),
    (∀ y ∈ ys, y ∈ acc) → ys.foldl mergeInto acc = acc
  | [], _, _ => rfl
  | y :: ys, acc, h => by
      have hy : y ∈ acc := h y List.mem_cons_self
      simp only [List.foldl_cons, mergeInto, hy, ↓reduceIte]
      exact foldl_mergeInto_of_mem ys acc (fun z hz => h z (List.mem_cons_of_mem _ hz))

/-- **Merging an agreeing overlap is idempotent**: parts already present
(the overlap agrees) change nothing. -/
theorem merging_an_agreeing_overlap_is_idempotent (xs ys : List Clause)
    (h : ∀ y ∈ ys, y ∈ xs) : merge (xs ++ ys) = merge xs := by
  simp only [merge, List.foldl_append]
  exact foldl_mergeInto_of_mem ys _ (fun y hy => (mem_merge xs y).mpr (h y hy))

private theorem foldl_mergeInto_nodup : ∀ (cs acc : List Clause),
    acc.Nodup → (cs.foldl mergeInto acc).Nodup
  | [], _, h => h
  | d :: ds, acc, h => by
      simp only [List.foldl_cons]
      apply foldl_mergeInto_nodup ds
      unfold mergeInto
      by_cases hd : d ∈ acc
      · simp only [hd, ↓reduceIte]; exact h
      · simp only [hd, ↓reduceIte]
        rw [List.nodup_append]
        refine ⟨h, by simp, ?_⟩
        intro a ha b hb
        simp only [List.mem_singleton] at hb
        subst hb
        intro e; subst e; exact hd ha

private theorem foldl_mergeInto_fresh : ∀ (ys acc : List Clause),
    (∀ y ∈ ys, y ∉ acc) → ys.Nodup → ys.foldl mergeInto acc = acc ++ ys
  | [], acc, _, _ => by simp
  | y :: ys, acc, hfresh, hnd => by
      have hy : y ∉ acc := hfresh y List.mem_cons_self
      have hnd' := List.nodup_cons.mp hnd
      simp only [List.foldl_cons, mergeInto, hy, ↓reduceIte]
      rw [foldl_mergeInto_fresh ys (acc ++ [y]) ?_ hnd'.2]
      · simp
      · intro z hz hzacc
        simp only [List.mem_append, List.mem_singleton] at hzacc
        rcases hzacc with h | rfl
        · exact hfresh z (List.mem_cons_of_mem _ hz) h
        · exact hnd'.1 hz

/-- Merging the merge changes nothing. -/
theorem merge_merge (cs : List Clause) : merge (merge cs) = merge cs := by
  have hnd : (merge cs).Nodup := foldl_mergeInto_nodup cs [] List.nodup_nil
  have := foldl_mergeInto_fresh (merge cs) [] (by simp) hnd
  simpa [merge] using this

/-- **A shorthand and its decomposition have one canonical part set.** -/
theorem shorthand_and_decomposition_are_one_canonical_set (b : Nat) :
    canonical [.deref b] = canonical [.nullDeref b, .checks b zeroCheck, .live b] := by
  simp [canonical, elaborate, elabClause, merge, mergeInto]

/-- A mix (the shorthand with a part it already gives) is the same set. -/
theorem a_mix_is_the_shorthands_canonical_set (b : Nat) :
    canonical [.deref b, .nullDeref b] = canonical [.deref b] ∧
      canonical [.live b, .deref b] =
        [.live b, .nullDeref b, .checks b zeroCheck] ∧
      (∀ c, c ∈ canonical [.live b, .deref b] ↔ c ∈ canonical [.deref b]) := by
  refine ⟨by simp [canonical, elaborate, elabClause, merge, mergeInto],
    by simp [canonical, elaborate, elabClause, merge, mergeInto], ?_⟩
  intro c
  simp only [canonical, mem_merge, elaborate, elabClause, List.flatMap_cons, List.flatMap_nil,
    List.mem_append, List.mem_cons, List.not_mem_nil, or_false, List.append_nil]
  constructor
  · rintro (h | h | h | h) <;> simp [h]
  · rintro (h | h | h) <;> simp [h]

/-- Contracts with one canonical part set hold of the same arguments. -/
theorem one_canonical_set_one_meaning (ρ : Nat → Int) (args : Nat → Ptr) (cs ds : List Clause)
    (h : ∀ c, c ∈ canonical cs ↔ c ∈ canonical ds) : Holds ρ args cs ↔ Holds ρ args ds := by
  rw [elaboration_preserves_meaning ρ args cs, elaboration_preserves_meaning ρ args ds,
    ← merge_preserves_meaning ρ args (elaborate cs), ← merge_preserves_meaning ρ args (elaborate ds)]
  simp only [Holds]
  exact ⟨fun hc c hm => hc c ((h c).mpr hm), fun hd c hm => hd c ((h c).mp hm)⟩

end QCore.DerefDecomposition
