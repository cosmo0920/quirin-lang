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

import QCore.RetainedBy
import QCore.InPlaceCertifiedUse

/-!
# A frame holder is one declaration (card FRAME-HOLDER-SHADOWED-NAME, rule 74o)

Rule 12t reads a holder actual `&NAME` as a frame holder of this function when
the token census admits every mention of `NAME` (holder actuals and the
declarator) and the function declares exactly one automatic `VarDecl` of the
name (`one_automatic_object`). The callback registry's frame holder
(`frame_registry_holder`) reads a registry actual the same way. Both counted
declarations only, so a block-scope holder shadowing a file-scope object the
body also names as `&NAME` was read as one frame object, and the file-scope
object's `&NAME` became the frame holder (probes F1-F4 `Safe`).

Rule 74o asks `QCore.InPlaceCertifiedUse.oneDeclaration` too: one declaration
and every `DeclRefExpr` of the name designating it. This module extends that
one-declaration result from in-place objects to frame holders:

* `frame74o` implies `frame12t`: the new reading is a frame holder only where
  the old one was (`a_frame_holder_was_one_before`), and a name a reference
  outside its one declaration can name is never one
  (`a_shadowed_name_is_not_a_frame_holder`).
* Over any spelled path, a `Safe` replay under rule 74o is a `Safe` replay
  under rule 12t (`no_verdict_becomes_safe`): a retention whose holder is no
  longer a frame is undecided (or a static object, decided as before), and a
  reset of such a holder drops nothing, which only keeps records longer.
* The registry's frame key is the old key or refused
  (`a_registry_key_is_the_old_one_or_refused`).
* Witnesses: probes F1, F3 and F4 were `Safe` and are refused; the controls
  read as before.

Not proved (trust boundary 28): which `VarDecl` and `DeclRefExpr` ids Clang
gives a name (`decls`, `refs`), and what the token census decides (`census`).
Both are pinned by `crates/qcore-c-gateway/tests/frame_holder_shadowed_name.rs`.
-/

namespace QCore.FrameHolderOneDeclaration

open QCore.RetainedBy
open QCore.InPlaceCertifiedUse (oneDeclaration one_declaration_iff)

/-- What route G knows of one `&NAME` holder actual: the name, the token
census's verdict over the body, and Clang's declaration and reference ids of
the name in the function. -/
structure Spelling where
  name : Nat
  census : Bool
  decls : List Nat
  refs : List Nat
  deriving DecidableEq, Repr

/-- Rule 12t's frame holder: the census and `one_automatic_object`. -/
def frame12t (s : Spelling) : Bool := s.census && s.decls.length == 1

/-- Rule 74o's frame holder: the census, `one_automatic_object` and
`every_reference_is`. -/
def frame74o (s : Spelling) : Bool := s.census && oneDeclaration s.decls s.refs

theorem a_frame_holder_was_one_before (s : Spelling) (h : frame74o s = true) :
    frame12t s = true := by
  simp only [frame74o, Bool.and_eq_true] at h
  obtain ⟨hc, ho⟩ := h
  obtain ⟨d, hd, _⟩ := (one_declaration_iff s.decls s.refs).mp ho
  simp [frame12t, hc, hd]

/-- A name a reference outside its one declaration names (a file-scope object
the local shadows) is never a frame holder. -/
theorem a_shadowed_name_is_not_a_frame_holder (n : Nat) (c : Bool) (d r : Nat)
    (refs : List Nat) (hr : r ∈ refs) (hne : r ≠ d) :
    frame74o ⟨n, c, [d], refs⟩ = false := by
  cases ho : oneDeclaration [d] refs with
  | false => simp [frame74o, ho]
  | true =>
      obtain ⟨d', hd, hall⟩ := (one_declaration_iff [d] refs).mp ho
      cases hd
      exact absurd (hall r hr) hne

/-- Where every reference designates the one declaration, the readings agree. -/
theorem the_readings_agree_on_one_declaration (n : Nat) (c : Bool) (d : Nat)
    (refs : List Nat) (hall : ∀ r ∈ refs, r = d) :
    frame74o ⟨n, c, [d], refs⟩ = frame12t ⟨n, c, [d], refs⟩ := by
  have : oneDeclaration [d] refs = true :=
    (one_declaration_iff [d] refs).mpr ⟨d, rfl, hall⟩
  simp [frame74o, frame12t, this]

/-- The holder a `&NAME` actual denotes under a frame reading: the frame
holder, or one the body cannot decide (`Holder.foreign`). -/
def holderOf (frame : Spelling → Bool) (s : Spelling) : Holder :=
  if frame s then .frame s.name else .foreign

/-- A retention whose holder is a shadowed name is undecided, unless the
retained object is static (which outlives every holder, as before). -/
theorem a_shadowed_holder_is_undecided (n : Nat) (c : Bool) (d r : Nat)
    (refs : List Nat) (hr : r ∈ refs) (hne : r ≠ d) (p : Retained)
    (hp : p ≠ .static) :
    decide p (holderOf frame74o ⟨n, c, [d], refs⟩) = .undecided := by
  simp only [holderOf, a_shadowed_name_is_not_a_frame_holder n c d r refs hr hne]
  exact foreign_holder_is_undecided p hp

/-- One event of a path, as spelled: a retention or reset at a `&NAME` actual
read by the frame reading, a retention in a holder decided otherwise, or any
other event, read as before. -/
inductive SEvent where
  | retainAt (p : Retained) (s : Spelling)
  | retainIn (p : Retained) (h : Holder)
  | resetAt (s : Spelling)
  | other (e : Event)
  deriving Repr

/-- The events route G applies under a frame reading. A reset whose holder is
no frame holder is dropped (`legacy_retained_by.rs` keeps only the resets it
reads). -/
def lower (frame : Spelling → Bool) : SEvent → List Event
  | .retainAt p s => [.retain (decide p (holderOf frame s))]
  | .retainIn p h => [.retain (decide p h)]
  | .resetAt s => if frame s then [.resetFrame s.name] else []
  | .other e => [e]

def lowerAll (frame : Spelling → Bool) : List SEvent → List Event
  | [] => []
  | e :: es => lower frame e ++ lowerAll frame es

/-! ### Simulation: fewer records under rule 12t -/

theorem run_single (rs : Records) (e : Event) : run rs [e] = step rs e := by
  cases h : step rs e <;> simp [run, h]

theorem drop_mono {a b : Records} (h : a ⊆ b) (x : HeldBy) : drop a x ⊆ drop b x := by
  intro r hr
  simp only [drop, List.mem_filter] at hr ⊢
  exact ⟨h hr.1, hr.2⟩

theorem drop_subset (a : Records) (x : HeldBy) : drop a x ⊆ a := by
  intro r hr
  simp only [drop, List.mem_filter] at hr
  exact hr.1

theorem holds_mono {a b : Records} (h : a ⊆ b) (s : Nat) (hb : holds b s = false) :
    holds a s = false := by
  simp only [holds, List.any_eq_false] at hb ⊢
  exact fun r hr => hb r (h hr)

/-- The same event, applied to fewer records, refuses no more often and leaves
fewer records. -/
theorem step_mono {a b : Records} (h : a ⊆ b) (e : Event) (b' : Records)
    (hb : step b e = some b') : ∃ a', step a e = some a' ∧ a' ⊆ b' := by
  cases e with
  | retain d =>
      cases d with
      | outlives =>
          simp only [step, Option.some.injEq] at hb
          exact ⟨a, rfl, hb ▸ h⟩
      | undecided => simp [step] at hb
      | record o x =>
          simp only [step, Option.some.injEq] at hb
          exact ⟨(o, x) :: a, rfl, hb ▸ List.cons_subset_cons _ h⟩
  | release s =>
      simp only [step] at hb ⊢
      cases hh : holds b s with
      | true => simp [hh] at hb
      | false =>
          simp only [hh, Bool.false_eq_true, ↓reduceIte, Option.some.injEq] at hb
          have ha := holds_mono h s hh
          simp only [ha, Bool.false_eq_true, ↓reduceIte]
          exact ⟨_, rfl, hb ▸ drop_mono h _⟩
  | resetFrame l =>
      simp only [step, Option.some.injEq] at hb
      exact ⟨_, rfl, hb ▸ drop_mono h _⟩
  | resetSite x =>
      simp only [step, Option.some.injEq] at hb
      exact ⟨_, rfl, hb ▸ drop_mono h _⟩

/-- One spelled event: what rule 74o applies, rule 12t applies to fewer
records with no more refusals. -/
theorem lower_mono {a b : Records} (h : a ⊆ b) (e : SEvent) (b' : Records)
    (hb : run b (lower frame74o e) = some b') :
    ∃ a', run a (lower frame12t e) = some a' ∧ a' ⊆ b' := by
  cases e with
  | retainAt p s =>
      simp only [lower, run_single] at hb ⊢
      cases hf : frame74o s with
      | true =>
          have h12 := a_frame_holder_was_one_before s hf
          simp only [holderOf, hf, h12, ↓reduceIte] at hb ⊢
          exact step_mono h _ b' hb
      | false =>
          simp only [holderOf, hf, Bool.false_eq_true, ↓reduceIte] at hb
          cases p with
          | static =>
              simp only [static_outlives_every_holder, step, Option.some.injEq] at hb ⊢
              exact ⟨a, rfl, hb ▸ h⟩
          | callerObject n => simp [RetainedBy.decide, step] at hb
          | tracked x => simp [RetainedBy.decide, step] at hb
          | other => simp [RetainedBy.decide, step] at hb
  | retainIn p x =>
      simp only [lower, run_single] at hb ⊢
      exact step_mono h _ b' hb
  | resetAt s =>
      simp only [lower] at hb ⊢
      cases hf : frame74o s with
      | true =>
          have h12 := a_frame_holder_was_one_before s hf
          simp only [hf, h12, ↓reduceIte, run_single] at hb ⊢
          exact step_mono h _ b' hb
      | false =>
          simp only [hf, Bool.false_eq_true, ↓reduceIte, run, Option.some.injEq] at hb
          cases h12 : frame12t s with
          | true =>
              simp only [↓reduceIte, run_single, step]
              exact ⟨_, rfl, fun r hr => hb ▸ h (drop_subset a _ hr)⟩
          | false =>
              simp only [Bool.false_eq_true, ↓reduceIte, run]
              exact ⟨a, rfl, hb ▸ h⟩
  | other e =>
      simp only [lower, run_single] at hb ⊢
      exact step_mono h _ b' hb

theorem run_lowerAll_mono (es : List SEvent) {a b : Records} (h : a ⊆ b) (b' : Records)
    (hb : run b (lowerAll frame74o es) = some b') :
    ∃ a', run a (lowerAll frame12t es) = some a' ∧ a' ⊆ b' := by
  induction es generalizing a b with
  | nil =>
      simp only [lowerAll, run, Option.some.injEq] at hb ⊢
      exact ⟨a, rfl, hb ▸ h⟩
  | cons e es ih =>
      simp only [lowerAll, run_append] at hb ⊢
      cases hm : run b (lower frame74o e) with
      | none => simp [hm] at hb
      | some m =>
          rw [hm, Option.bind_some] at hb
          obtain ⟨am, ham, hsub⟩ := lower_mono h e m hm
          rw [ham, Option.bind_some]
          exact ih hsub hb

/-- **No verdict becomes `Safe`.** A spelled path that replays `Safe` under
rule 74o's frame reading replayed `Safe` under rule 12t's. -/
theorem no_verdict_becomes_safe (es : List SEvent)
    (hsafe : replay (lowerAll frame74o es) = true) :
    replay (lowerAll frame12t es) = true := by
  unfold replay at hsafe ⊢
  cases hn : run [] (lowerAll frame74o es) with
  | none => simp [hn] at hsafe
  | some b' =>
      rw [hn] at hsafe
      obtain ⟨a', ha, hsub⟩ := run_lowerAll_mono es (List.Subset.refl []) b' hn
      rw [ha]
      simp only [Bool.not_eq_true', List.any_eq_false] at hsafe ⊢
      exact fun r hr => hsafe r (hsub hr)

/-! ### The callback registry's frame key -/

/-- The registry key a `&NAME` registry actual denotes: its frame key, or
`none` (`callback-registry-undecided`). -/
def registryKey (frame : Spelling → Bool) (s : Spelling) : Option Nat :=
  if frame s then some s.name else none

theorem a_registry_key_is_the_old_one_or_refused (s : Spelling) :
    registryKey frame74o s = registryKey frame12t s ∨ registryKey frame74o s = none := by
  cases hf : frame74o s with
  | true =>
      left
      simp [registryKey, hf, a_frame_holder_was_one_before s hf]
  | false => right; simp [registryKey, hf]

theorem a_shadowed_registry_is_refused (n : Nat) (c : Bool) (d r : Nat) (refs : List Nat)
    (hr : r ∈ refs) (hne : r ≠ d) : registryKey frame74o ⟨n, c, [d], refs⟩ = none := by
  simp [registryKey, a_shadowed_name_is_not_a_frame_holder n c d r refs hr hne]

/-! ### Witness programs

`g` is name 0; the block-scope `struct st g` is declaration 1 and the
file-scope `g` is declaration 0, so the body's references are `[1, 0]` when it
names both and `[1]` when it names only the local. The census admits every
mention in each probe. -/

/-- `g` as the shadowing probes spell it: both objects named. -/
def shadowed : Spelling := ⟨0, true, [1], [1, 0]⟩

/-- `g` as the control spells it: the local only. -/
def own : Spelling := ⟨0, true, [1], [1]⟩

/-- Probe F1 (and F2, the same events): a caller's object `src` (name 5)
retained by the local `g`, then by the file-scope `g`. Rule 12t read the
second holder as the frame: `Safe`. Rule 74o refuses, as does the true holder,
a program-lifetime object. -/
theorem probe_f1_was_safe_and_is_refused :
    replay (lowerAll frame12t
        [.retainAt (.callerObject 5) shadowed, .retainAt (.callerObject 5) shadowed]) = true ∧
      replay (lowerAll frame74o
        [.retainAt (.callerObject 5) shadowed, .retainAt (.callerObject 5) shadowed]) = false ∧
      replay [.retain (decide (.callerObject 5) .global)] = false := by
  decide

/-- Probe F3: a tracked `p` (site 1) retained by the file-scope `g`, the local
`g` reset, then `p` released. Rule 12t dropped the file-scope record with the
local's reset: `Safe`. Rule 74o refuses the retention. -/
theorem probe_f3_was_safe_and_is_refused :
    replay (lowerAll frame12t
        [.retainAt (.tracked 1) shadowed, .resetAt shadowed, .other (.release 1)]) = true ∧
      replay (lowerAll frame74o
        [.retainAt (.tracked 1) shadowed, .resetAt shadowed, .other (.release 1)]) = false := by
  decide

/-- Probe F4: the file-scope registry `gl` was read as the local's frame key;
it is now refused. -/
theorem probe_f4_was_a_frame_key_and_is_refused :
    registryKey frame12t shadowed = some 0 ∧ registryKey frame74o shadowed = none := by
  decide

/-- The controls: a frame holder whose every reference is its own reads as
before, and a caller's object retained there is `Safe`. -/
theorem the_control_is_safe_under_both :
    replay (lowerAll frame12t [.retainAt (.callerObject 5) own]) = true ∧
      replay (lowerAll frame74o [.retainAt (.callerObject 5) own]) = true ∧
      registryKey frame74o own = some 0 := by
  decide

end QCore.FrameHolderOneDeclaration
