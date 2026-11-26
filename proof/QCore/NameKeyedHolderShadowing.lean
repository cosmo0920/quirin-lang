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

import QCore.RetentionPath
import QCore.InPlaceCertifiedUse

/-!
# A formal-keyed reading is one declaration (card NAME-KEYED-HOLDER-SHADOWING, rule 86)

Rule 12t reads, from a formal's spelling, a retained actual `NAME` (or an
interior `&NAME->f`) as a caller's object and a path holder `NAME->f` as
storage the caller supplied, and keys a removal on either. The spelling is a
formal when the token census says it is an unrebound formal
(`CallerFormals::contains`). A block-scope local that shadows the formal and
is named by the same spelling was read as the formal. Where the shadow's
declaration is jumped over (before a switch's first label, past a `goto`
target), the classifier's guard refuses it on no path, and the route reached
`Safe` (probes J1-J9).

Rule 86 also asks Clang's declaration reading: the name has one declaration
in the function, its `ParmVarDecl`, and every `DeclRefExpr` of the name
designates it (`InPlaceCertifiedUse.oneDeclaration`, with the one declaration
the parameter). A formal that fails it is read as a plain name: what rule 12t
read, or an object this body cannot decide (`Retained.other`). A path rooted
there is no path holder (`Holder.foreign`).

* `formal86` implies `formal12t` (`a_formal_was_one_before`), and a name a
  second declaration or a reference elsewhere can name is no formal
  (`a_shadowed_name_is_not_a_formal`, `a_foreign_reference_is_not_a_formal`).
* Over any spelled path, a `Safe` replay under rule 86 replayed `Safe` under
  rule 12t (`no_verdict_becomes_safe`). The proof is a simulation over
  `QCore.RetentionPath`: under rule 86 a retention decides as before or is
  undecided, and a removal's key is the old key with sides possibly
  undecided, which drops no more and poisons no less. Every rule 12t record
  is then covered by a rule 86 record of the same object and holder,
  poisoned at least as much (`Covers`).
* Witnesses: J2 (a shadow retained by a frame holder) and J6 (a removal
  through the shadow dropping the formal's record) are `Safe` under rule
  12t's reading and refused under rule 86's. The controls agree.

Not proved (trust boundary 28): Clang's declaration and reference ids
(`decls`, `parm`, `refs`) and the token census (`census`). They are pinned by
`crates/qcore-c-gateway/tests/name_keyed_holder_shadowing.rs` and the unit
tests of `legacy_formal_declaration.rs`.
-/

namespace QCore.NameKeyedHolderShadowing

open QCore.RetentionPath
open QCore.InPlaceCertifiedUse (oneDeclaration one_declaration_iff)

/-- What route G knows of a formal's spelling: its name, the token census's
verdict, every declaration id of the name in the function, the
`ParmVarDecl`'s id and the ids every `DeclRefExpr` of the name refers to. -/
structure Formal where
  name : Nat
  census : Bool
  decls : List Nat
  parm : Nat
  refs : List Nat
  deriving DecidableEq, Repr

/-- Rule 12t's formal: the token census. -/
def formal12t (f : Formal) : Bool := f.census

/-- Rule 86's formal: the census, the parameter the one declaration, and
every reference designating it. -/
def formal86 (f : Formal) : Bool :=
  f.census && f.decls == [f.parm] && oneDeclaration f.decls f.refs

theorem a_formal_was_one_before (f : Formal) (h : formal86 f = true) :
    formal12t f = true := by
  simp only [formal86, Bool.and_eq_true] at h
  exact h.1.1

/-- A second declaration of the name (a shadowing local) refuses. -/
theorem a_shadowed_name_is_not_a_formal (f : Formal) (d : Nat) (hd : d ∈ f.decls)
    (hne : d ≠ f.parm) : formal86 f = false := by
  cases h : formal86 f with
  | false => rfl
  | true =>
      simp only [formal86, Bool.and_eq_true, beq_iff_eq] at h
      rw [h.1.2] at hd
      exact absurd (List.mem_singleton.mp hd) hne

/-- A reference to any other declaration refuses. -/
theorem a_foreign_reference_is_not_a_formal (f : Formal) (r : Nat) (hr : r ∈ f.refs)
    (hne : r ≠ f.parm) : formal86 f = false := by
  cases h : formal86 f with
  | false => rfl
  | true =>
      simp only [formal86, Bool.and_eq_true, beq_iff_eq] at h
      obtain ⟨⟨_, hdecl⟩, hone⟩ := h
      obtain ⟨d, hd, hall⟩ := (one_declaration_iff f.decls f.refs).mp hone
      rw [hdecl] at hd
      cases hd
      exact absurd (hall r hr) hne

/-- Where the parameter is the one declaration and every reference is its,
the readings agree. -/
theorem the_readings_agree_on_one_declaration (n p : Nat) (c : Bool) (refs : List Nat)
    (hall : ∀ r ∈ refs, r = p) :
    formal86 ⟨n, c, [p], p, refs⟩ = formal12t ⟨n, c, [p], p, refs⟩ := by
  have : oneDeclaration [p] refs = true := (one_declaration_iff [p] refs).mpr ⟨p, rfl, hall⟩
  simp [formal86, formal12t, this]

/-- A retained actual as spelled: a formal (with what rule 12t read for it), or
anything rule 12t decides without a formal. -/
inductive RSpell where
  | formal (f : Formal) (read : Retained)
  | plain (r : Retained)
  deriving Repr

/-- A holder as spelled: a path below a formal, or anything else. -/
inductive HSpell where
  | path (f : Formal) (field : Nat)
  | plain (h : Holder)
  deriving Repr

def retainedOf (formal : Formal → Bool) : RSpell → Retained
  | .formal f r => if formal f then r else .other
  | .plain r => r

def holderOf (formal : Formal → Bool) : HSpell → Holder
  | .path f g => if formal f then .path f.name g else .foreign
  | .plain h => h

/-- One event of a path, as spelled. -/
inductive SEvent where
  | retain (p : RSpell) (h : HSpell)
  | remove (p : RSpell) (h : HSpell)
  | other (e : Event)
  deriving Repr

def lower (formal : Formal → Bool) : SEvent → List Event
  | .retain p h => [.retain (decide (retainedOf formal p) (holderOf formal h))]
  | .remove p h => [.remove (removalKey (retainedOf formal p) (holderOf formal h))]
  | .other e => [e]

def lowerAll (formal : Formal → Bool) : List SEvent → List Event
  | [] => []
  | e :: es => lower formal e ++ lowerAll formal es

/-! ### Rule 86's readings are rule 12t's or undecided -/

theorem retained_weakens (p : RSpell) :
    retainedOf formal86 p = retainedOf formal12t p ∨ retainedOf formal86 p = .other := by
  cases p with
  | plain r => left; rfl
  | formal f r =>
      simp only [retainedOf]
      cases h : formal86 f with
      | true => left; simp [a_formal_was_one_before f h]
      | false => right; rfl

theorem holder_weakens (h : HSpell) :
    holderOf formal86 h = holderOf formal12t h ∨ holderOf formal86 h = .foreign := by
  cases h with
  | plain x => left; rfl
  | path f g =>
      simp only [holderOf]
      cases hf : formal86 f with
      | true => left; simp [a_formal_was_one_before f hf]
      | false => right; rfl

theorem decision_weakens (p : RSpell) (h : HSpell) :
    decide (retainedOf formal86 p) (holderOf formal86 h)
        = decide (retainedOf formal12t p) (holderOf formal12t h) ∨
      decide (retainedOf formal86 p) (holderOf formal86 h) = .undecided := by
  rcases retained_weakens p with hp | hp
  · rcases holder_weakens h with hh | hh
    · left; rw [hp, hh]
    · rw [hp, hh]
      cases retainedOf formal12t p with
      | static => left; simp [RetentionPath.decide]
      | callerObject n => right; rfl
      | tracked s => right; rfl
      | interior s f => right; rfl
      | other => right; rfl
  · right; rw [hp]; cases holderOf formal86 h <;> rfl

/-- A removal key whose sides are the old ones or undecided. -/
def Weaker (k' k : Option HeldObject × Option HeldBy) : Prop :=
  (k'.1 = k.1 ∨ k'.1 = none) ∧ (k'.2 = k.2 ∨ k'.2 = none)

theorem key_weakens (p : RSpell) (h : HSpell) :
    Weaker (removalKey (retainedOf formal86 p) (holderOf formal86 h))
      (removalKey (retainedOf formal12t p) (holderOf formal12t h)) := by
  constructor
  · rcases retained_weakens p with hp | hp
    · left; simp [removalKey, hp]
    · right; simp [removalKey, hp, heldObject]
  · rcases holder_weakens h with hh | hh
    · left; simp [removalKey, hh]
    · right; simp [removalKey, hh, heldBy]

/-! ### Simulation: rule 86 keeps at least rule 12t's records -/

/-- Every record of `a` has one in `b` of the same object and holder,
poisoned at least as much. -/
def Covers (a b : Records) : Prop :=
  ∀ r ∈ a, ∃ r' ∈ b, r'.object = r.object ∧ r'.holder = r.holder ∧
    (r.poisoned = true → r'.poisoned = true)

theorem covers_refl (a : Records) : Covers a a :=
  fun r hr => ⟨r, hr, rfl, rfl, id⟩

theorem covers_nil (b : Records) : Covers [] b := by
  intro r hr; cases hr

theorem covers_cons {a b : Records} (h : Covers a b) (r : Rec) : Covers (r :: a) (r :: b) := by
  intro x hx
  cases hx with
  | head => exact ⟨r, List.mem_cons_self .., rfl, rfl, id⟩
  | tail _ hx =>
      obtain ⟨x', hx', ho, hh, hp⟩ := h x hx
      exact ⟨x', List.mem_cons_of_mem _ hx', ho, hh, hp⟩

theorem covers_filter_holder {a b : Records} (h : Covers a b) (c : HeldBy) :
    Covers (a.filter fun r => r.holder != c) (b.filter fun r => r.holder != c) := by
  intro r hr
  simp only [List.mem_filter] at hr
  obtain ⟨r', hr', ho, hh, hp⟩ := h r hr.1
  refine ⟨r', ?_, ho, hh, hp⟩
  simp only [List.mem_filter]
  exact ⟨hr', hh ▸ hr.2⟩

theorem holds_covers {a b : Records} (h : Covers a b) (s : Nat) (hb : holds b s = false) :
    holds a s = false := by
  simp only [holds, List.any_eq_false] at hb ⊢
  intro r hr
  obtain ⟨r', hr', ho, _, _⟩ := h r hr
  have := hb r' hr'
  rw [ho] at this
  exact this

theorem exact_of_weaker {k k' : Option HeldObject × Option HeldBy} (hw : Weaker k' k)
    (r r' : Rec) (ho : r'.object = r.object) (hh : r'.holder = r.holder)
    (hp : r.poisoned = true → r'.poisoned = true) (he : exact k' r' = true) :
    exact k r = true := by
  simp only [exact, Bool.and_eq_true, beq_iff_eq, Bool.not_eq_true'] at he ⊢
  obtain ⟨⟨h1, h2⟩, h3⟩ := he
  rcases hw with ⟨w1 | w1, w2 | w2⟩
  · refine ⟨⟨?_, ?_⟩, ?_⟩
    · rw [← w1, h1, ho]
    · rw [← w2, h2, hh]
    · cases hq : r.poisoned
      · rfl
      · rw [hp hq] at h3; cases h3
  all_goals simp_all

theorem poisoned_of_weaker {o o' : Option HeldObject} (hw : o' = o ∨ o' = none)
    (r r' : Rec) (ho : r'.object = r.object) (hp : r.poisoned = true → r'.poisoned = true) :
    (poisonIf o r).poisoned = true → (poisonIf o' r').poisoned = true := by
  intro h
  unfold poisonIf at h ⊢
  split at h
  · rename_i hm
    have hm' : mayName o' r'.object = true := by
      rcases hw with rfl | rfl
      · rw [ho]; exact hm
      · rfl
    simp [hm']
  · split
    · rfl
    · exact hp h

theorem remove_covers {a b : Records} (h : Covers a b)
    {k k' : Option HeldObject × Option HeldBy} (hw : Weaker k' k) :
    Covers ((a.filter fun r => !exact k r).map (poisonIf k.1))
      ((b.filter fun r => !exact k' r).map (poisonIf k'.1)) := by
  intro r hr
  simp only [List.mem_map, List.mem_filter] at hr
  obtain ⟨r0, ⟨hr0, hex⟩, rfl⟩ := hr
  obtain ⟨r0', hr0', ho, hh, hp⟩ := h r0 hr0
  have hkeep : (!exact k' r0') = true := by
    cases he : exact k' r0' with
    | false => rfl
    | true =>
        have := exact_of_weaker hw r0 r0' ho hh hp he
        simp [this] at hex
  refine ⟨poisonIf k'.1 r0', ?_, ?_, ?_, ?_⟩
  · simp only [List.mem_map, List.mem_filter]
    exact ⟨r0', ⟨hr0', hkeep⟩, rfl⟩
  · rw [(poisonIf_object _ _).1, (poisonIf_object _ _).1, ho]
  · rw [(poisonIf_object _ _).2, (poisonIf_object _ _).2, hh]
  · exact poisoned_of_weaker hw.1 r0 r0' ho hp

/-- The same event applied to covered records succeeds whenever it succeeds
on the cover, and keeps the cover. -/
theorem step_covers {a b : Records} (h : Covers a b) (e : Event) (b' : Records)
    (hb : step b e = some b') : ∃ a', step a e = some a' ∧ Covers a' b' := by
  cases e with
  | retain d =>
      cases d with
      | outlives =>
          simp only [step, Option.some.injEq] at hb
          exact ⟨a, rfl, hb ▸ h⟩
      | undecided => simp [step] at hb
      | record o x =>
          simp only [step, Option.some.injEq] at hb
          exact ⟨_, rfl, hb ▸ covers_cons h _⟩
  | release s =>
      simp only [step] at hb ⊢
      cases hh : holds b s with
      | true => simp [hh] at hb
      | false =>
          simp only [hh, Bool.false_eq_true, ↓reduceIte, Option.some.injEq] at hb
          simp only [holds_covers h s hh, Bool.false_eq_true, ↓reduceIte]
          exact ⟨_, rfl, hb ▸ covers_filter_holder h _⟩
  | releaseHolder s =>
      simp only [step, Option.some.injEq] at hb
      exact ⟨_, rfl, hb ▸ covers_filter_holder h _⟩
  | remove k =>
      simp only [step, Option.some.injEq] at hb
      exact ⟨_, rfl, hb ▸ remove_covers h ⟨Or.inl rfl, Or.inl rfl⟩⟩
  | destroy c =>
      simp only [step, Option.some.injEq] at hb
      exact ⟨_, rfl, hb ▸ covers_filter_holder h _⟩

theorem run_single (rs : Records) (e : Event) : run rs [e] = step rs e := by
  cases h : step rs e <;> simp [run, h]

/-- One spelled event: what rule 86 applies, rule 12t applies with no more
refusals, and the cover is kept. -/
theorem lower_covers {a b : Records} (h : Covers a b) (e : SEvent) (b' : Records)
    (hb : run b (lower formal86 e) = some b') :
    ∃ a', run a (lower formal12t e) = some a' ∧ Covers a' b' := by
  cases e with
  | retain p x =>
      simp only [lower, run_single] at hb ⊢
      rcases decision_weakens p x with hd | hd
      · rw [hd] at hb
        exact step_covers h _ b' hb
      · rw [hd] at hb
        simp [step] at hb
  | remove p x =>
      simp only [lower, run_single, step, Option.some.injEq] at hb ⊢
      exact ⟨_, rfl, hb ▸ remove_covers h (key_weakens p x)⟩
  | other e =>
      simp only [lower, run_single] at hb ⊢
      exact step_covers h e b' hb

theorem run_lowerAll_covers (es : List SEvent) {a b : Records} (h : Covers a b)
    (b' : Records) (hb : run b (lowerAll formal86 es) = some b') :
    ∃ a', run a (lowerAll formal12t es) = some a' ∧ Covers a' b' := by
  induction es generalizing a b with
  | nil =>
      simp only [lowerAll, run, Option.some.injEq] at hb ⊢
      exact ⟨a, rfl, hb ▸ h⟩
  | cons e es ih =>
      simp only [lowerAll, run_append] at hb ⊢
      cases hm : run b (lower formal86 e) with
      | none => simp [hm] at hb
      | some m =>
          rw [hm, Option.bind_some] at hb
          obtain ⟨am, ham, hsub⟩ := lower_covers h e m hm
          rw [ham, Option.bind_some]
          exact ih hsub hb

/-- **No verdict becomes `Safe`.** A spelled path that replays `Safe` under
rule 86's formal reading replayed `Safe` under rule 12t's. -/
theorem no_verdict_becomes_safe (es : List SEvent)
    (hsafe : replay (lowerAll formal86 es) = true) :
    replay (lowerAll formal12t es) = true := by
  unfold replay at hsafe ⊢
  cases hn : run [] (lowerAll formal86 es) with
  | none => simp [hn] at hsafe
  | some b' =>
      rw [hn] at hsafe
      obtain ⟨a', ha, hcov⟩ := run_lowerAll_covers es (covers_refl []) b' hn
      rw [ha]
      simp only [Bool.not_eq_true', List.any_eq_false] at hsafe ⊢
      intro r hr
      obtain ⟨r', hr', _, hh, _⟩ := hcov r hr
      have := hsafe r' hr'
      unfold outlivesFrame at this ⊢
      rw [← hh]
      exact this

/-! ### Witness programs

The formal `o` is name 3 with `ParmVarDecl` 0. A shadowing local of the same
name is declaration 1. In the jumped-over probes the body names both, so the
references are `[0, 1]`. The path root `coll` (name 0, parameter 2) is
unshadowed. -/

def shadowed : Formal := ⟨3, true, [0, 1], 0, [0, 1]⟩
def own : Formal := ⟨3, true, [0], 0, [0]⟩
def coll : Formal := ⟨0, true, [2], 2, [2]⟩

/-- Probe J2: the shadow, read as the caller's object, retained by a frame
holder. `Safe` under rule 12t's reading, refused under rule 86's. -/
theorem probe_j2_was_safe_and_is_refused :
    replay (lowerAll formal12t
        [.retain (.formal shadowed (.callerObject 3)) (.plain (.frame 9))]) = true ∧
      replay (lowerAll formal86
        [.retain (.formal shadowed (.callerObject 3)) (.plain (.frame 9))]) = false := by
  decide

/-- Probe J6: the formal retained by the path `coll->evl`, then a removal
through the shadow. Rule 12t's reading dropped the formal's record: `Safe`.
Rule 86 refuses the retention where it stands. -/
theorem probe_j6_was_safe_and_is_refused :
    replay (lowerAll formal12t
        [.retain (.formal shadowed (.callerObject 3)) (.path coll 0),
         .remove (.formal shadowed (.callerObject 3)) (.path coll 0)]) = true ∧
      replay (lowerAll formal86
        [.retain (.formal shadowed (.callerObject 3)) (.path coll 0),
         .remove (.formal shadowed (.callerObject 3)) (.path coll 0)]) = false := by
  decide

/-- A removal through a shadow, after a retention rule 86 still decides (the
formal named elsewhere by a spelling that is one declaration), is unmatched:
it drops nothing and poisons the record, which then refuses at the exit. -/
theorem a_shadowed_removal_drops_nothing :
    replay (lowerAll formal86
        [.retain (.formal own (.callerObject 3)) (.path coll 0),
         .remove (.formal shadowed (.callerObject 3)) (.path coll 0)]) = false := by
  decide

/-- The controls: an unshadowed formal reads as before, in a frame holder and
in a path holder with its removal. -/
theorem the_control_is_safe_under_both :
    replay (lowerAll formal12t
        [.retain (.formal own (.callerObject 3)) (.plain (.frame 9))]) = true ∧
      replay (lowerAll formal86
        [.retain (.formal own (.callerObject 3)) (.plain (.frame 9))]) = true ∧
      replay (lowerAll formal86
        [.retain (.formal own (.callerObject 3)) (.path coll 0),
         .remove (.formal own (.callerObject 3)) (.path coll 0)]) = true := by
  decide

end QCore.NameKeyedHolderShadowing
