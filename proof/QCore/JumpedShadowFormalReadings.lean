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

import QCore.NameKeyedHolderShadowing

/-!
# Every formal-keyed reading is one declaration (card JUMPED-SHADOW-FORMAL-READINGS, rule 86)

Rule 86 (card NAME-KEYED-HOLDER-SHADOWING) made rule 12t's formal-keyed
readings ask Clang's declaration reading. The other consumers of a formal's
spelling still read the token census `CallerFormals`: container publication
(a container rooted at a stable formal, or at a derived local `L = P->F`, is
memory the caller supplied), rule 38's descriptor-cell `formal` closure, slot
moves, member custody attribution and the counted-user pass. W4 entry custody
reads a formal's spelling as the formal's identity throughout the body. A
local shadowing the formal whose declaration control jumps over put the
classifier's guard on no path, and these readings admitted it (probes CJ1,
CJ2, CJ4, CJ5, DCJ1-DCJ3, WDJ1).

The extension, modelled here:

* `CallerFormals` itself is read by declaration: a stable formal is one only
  where `formal86` holds, a derived local only where its own name is one
  declaration (`local86`) and its root is a `formal86` formal. Every consumer
  reads the narrowed set (`reaches86`), which is a subset of the census's
  (`reaches_narrows`).
* The guard is at the function's entry: a formal that is not one declaration
  refuses the whole function (`entryGuard`), whatever control does.

A consumer is modelled as a list of events, each admitted or not; a keyed
event is admitted only where the reading says the name reaches the caller.
`no_verdict_becomes_safe`: a function `Safe` under the extended rule (the
entry guard passes and every event is admitted under the narrowed reading)
was `Safe` under the census reading. Witnesses: CJ1 (a container rooted at a
jumped shadow of a formal), CJ5 (one rooted at a jumped shadow of a derived
local) and WDJ1 (a release through a jumped shadow read as the formal's) are
`Safe` before and refused now; the controls are `Safe` under both.

Not proved (trust boundary 28): Clang's declaration and reference ids and the
token census, as for rule 86. Without Clang's reading of a function, the
entry guard is the lexical over-approximation (`lexically_unguarded`), whose
completeness is a stated premise; that a refusal it adds never admits is the
same `entryGuard` argument. Pinned by
`crates/qcore-c-gateway/tests/jumped_shadow_formal_readings.rs`.
-/

namespace QCore.JumpedShadowFormalReadings

open QCore.NameKeyedHolderShadowing (Formal formal86 formal12t a_formal_was_one_before)
open QCore.InPlaceCertifiedUse (oneDeclaration)

/-- A local's spelling as route G knows it: the token census's verdict that
it is a derived local, its declaration ids, its own `VarDecl` id and the ids
its references designate. -/
structure Local where
  name : Nat
  census : Bool
  decls : List Nat
  id : Nat
  refs : List Nat
  deriving DecidableEq, Repr

/-- The local is one declaration: its `VarDecl`, every reference its. -/
def local86 (l : Local) : Bool :=
  l.census && l.decls == [l.id] && oneDeclaration l.decls l.refs

/-- A name a consumer may read as memory the caller supplied. -/
inductive Name where
  | formal (f : Formal)
  | derived (l : Local) (root : Formal)
  | other
  deriving Repr

/-- The census reading (`CallerFormals::reaches_caller` at base). -/
def reaches12t : Name → Bool
  | .formal f => formal12t f
  | .derived l root => l.census && formal12t root
  | .other => false

/-- The extended rule 86 reading. -/
def reaches86 : Name → Bool
  | .formal f => formal86 f
  | .derived l root => local86 l && formal86 root
  | .other => false

theorem local86_census (l : Local) (h : local86 l = true) : l.census = true := by
  simp only [local86, Bool.and_eq_true] at h
  exact h.1.1

theorem reaches_narrows (n : Name) (h : reaches86 n = true) : reaches12t n = true := by
  cases n with
  | formal f => exact a_formal_was_one_before f h
  | derived l root =>
      simp only [reaches86, reaches12t, Bool.and_eq_true] at h ⊢
      exact ⟨local86_census l h.1, a_formal_was_one_before root h.2⟩
  | other => simp [reaches86] at h

/-- One consumer event: a reading keyed on a name (a publication into its
container, a descriptor cell's formal closure, a caller's slot, a counted-user
attribution) or anything decided without one. -/
inductive Event where
  | keyed (n : Name)
  | plain (ok : Bool)
  deriving Repr

def admits (reaches : Name → Bool) : Event → Bool
  | .keyed n => reaches n
  | .plain ok => ok

def safe (reaches : Name → Bool) (es : List Event) : Bool :=
  es.all (admits reaches)

/-- A formal's declarations, read whether or not the census calls it stable. -/
def oneDecl (f : Formal) : Bool :=
  f.decls == [f.parm] && oneDeclaration f.decls f.refs

/-- The guard at the entry: every formal is one declaration. -/
def entryGuard (formals : List Formal) : Bool :=
  formals.all oneDecl

/-- The function's verdict at base: the census reading, the entry guard absent
(a jumped declaration's refusal lies on no path). -/
def verdict12t (es : List Event) : Bool := safe reaches12t es

/-- The extended rule's verdict. -/
def verdict86 (formals : List Formal) (es : List Event) : Bool :=
  entryGuard formals && safe reaches86 es

theorem admits_narrows (e : Event) (h : admits reaches86 e = true) :
    admits reaches12t e = true := by
  cases e with
  | keyed n => exact reaches_narrows n h
  | plain ok => exact h

theorem safe_narrows (es : List Event) (h : safe reaches86 es = true) :
    safe reaches12t es = true := by
  simp only [safe, List.all_eq_true] at h ⊢
  exact fun e he => admits_narrows e (h e he)

/-- **No verdict becomes `Safe`.** -/
theorem no_verdict_becomes_safe (formals : List Formal) (es : List Event)
    (h : verdict86 formals es = true) : verdict12t es = true := by
  simp only [verdict86, Bool.and_eq_true] at h
  exact safe_narrows es h.2

/-- A formal with a second declaration refuses the whole function, whatever
its events (a jumped declaration, an unrecognized spelling). -/
theorem a_shadowed_formal_refuses_at_entry (formals : List Formal) (f : Formal)
    (hf : f ∈ formals) (d : Nat) (hd : d ∈ f.decls) (hne : d ≠ f.parm)
    (es : List Event) : verdict86 formals es = false := by
  have hone : oneDecl f = false := by
    cases h : oneDecl f with
    | false => rfl
    | true =>
        simp only [oneDecl, Bool.and_eq_true, beq_iff_eq] at h
        rw [h.1] at hd
        exact absurd (List.mem_singleton.mp hd) hne
  have : entryGuard formals = false := by
    cases h : entryGuard formals with
    | false => rfl
    | true =>
        simp only [entryGuard, List.all_eq_true] at h
        rw [h f hf] at hone
        cases hone
  simp [verdict86, this]

/-- Each consumer reads the narrowed set: a keyed event on a name that fails
rule 86 is refused. -/
theorem a_keyed_event_needs_one_declaration (n : Name) (h : reaches86 n = false)
    (formals : List Formal) (es : List Event) (he : Event.keyed n ∈ es) :
    verdict86 formals es = false := by
  have : safe reaches86 es = false := by
    cases hs : safe reaches86 es with
    | false => rfl
    | true =>
        simp only [safe, List.all_eq_true] at hs
        have := hs _ he
        simp only [admits] at this
        rw [h] at this
        cases this
  simp [verdict86, this]

/-! ### Witness programs

The formal `queue` (name 1, `ParmVarDecl` 0) with a jumped local of the same
name (declaration 5): the body names both. `holder` is name 2 with parameter
3, unshadowed. The derived local `h` (name 4, `VarDecl` 6) is shadowed by a
jumped local (declaration 7). -/

def queueShadowed : Formal := ⟨1, true, [0, 5], 0, [5]⟩
def queueOwn : Formal := ⟨1, true, [0], 0, [0]⟩
def holder : Formal := ⟨2, true, [3], 3, [3]⟩
def hShadowed : Local := ⟨4, true, [6, 7], 6, [6, 7]⟩
def hOwn : Local := ⟨4, true, [6], 6, [6]⟩

/-- Probe CJ1: a publication into `&queue->pending` where `queue` is a jumped
shadow. -/
theorem probe_cj1_was_safe_and_is_refused :
    verdict12t [.keyed (.formal queueShadowed)] = true ∧
      verdict86 [queueShadowed] [.keyed (.formal queueShadowed)] = false := by
  decide

/-- Probe CJ5: a publication into `h`, the jumped shadow of the derived
local `h = holder->list`. No formal is shadowed: the narrowed reading
refuses. -/
theorem probe_cj5_was_safe_and_is_refused :
    verdict12t [.keyed (.derived hShadowed holder)] = true ∧
      verdict86 [holder] [.keyed (.derived hShadowed holder)] = false := by
  decide

/-- Probe WDJ1: `r_free(p)` through a jumped shadow of `p`, read by W4 as the
formal's release, keyed on no `CallerFormals` reading: the entry guard
refuses. -/
theorem probe_wdj1_was_safe_and_is_refused :
    verdict12t [.plain true] = true ∧ verdict86 [queueShadowed] [.plain true] = false := by
  decide

/-- The controls: unshadowed formals and derived locals keep their readings. -/
theorem the_controls_are_safe_under_both :
    verdict12t [.keyed (.formal queueOwn), .keyed (.derived hOwn holder)] = true ∧
      verdict86 [queueOwn, holder]
        [.keyed (.formal queueOwn), .keyed (.derived hOwn holder)] = true := by
  decide

end QCore.JumpedShadowFormalReadings
