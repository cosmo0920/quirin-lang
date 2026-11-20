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

import QCore.PointerStore
import QCore.RetentionPath

/-!
# A formal's pointer stored into a member is a retention (card CG10-MEMBER-STORE-RETAINED-BY, gateway rule 70)

USER DECISION 2026-11-20 on round 3's decision class 1, question 2: "Read it
as retention, option 1 with creating the relevant proof(s)."

Rule 10v refuses `s->p = q` for a formal `q` (`QCore.PointerStore`'s
`formalValue`): nothing names the owner of `*s` or bounds `*q`'s lifetime by
it (Q36's example C). Rule 70 supplies that premise as the contract
specification already spells it for a declared callee (§C04.14.3, a holder
path `resource_retained_by(q, s.m)`): the body takes no custody of `q`, and
its summary states the retention, which each caller decides with rule 12t's
table (`QCore.RetentionPath.decide`).

The module has four parts.

1. **The flow decision** (`decideRetained`, bound to the Rust
   `DECISION_VECTORS` of `member_store_retention.rs`): admitted only into an
   untracked owner, for a formal that carries no identity.
2. **The summary as an obligation** (`Summary`, `callEvents`): a call of a
   body with a retention summary releases nothing (no custody moves) and, for
   each retention, records exactly what `QCore.RetentionPath.decide` records;
   an overwritten path that is not a path holder of the caller refuses the
   call; a function pointer (a static value) records nothing.
3. **Soundness** (`Act`, `exec`, `check`): over a concrete memory whose
   holders keep pointers and whose objects are released, every trace the
   checker admits (release refused while a holder retains the object, the
   checker's own release and use-after-release refusals) executes without a
   use-after-release through a holder and without a double release, also
   through the holder; admission composes over concatenation, so caller
   discharge composes call by call.
4. **The counterexample**: a caller that releases the retained pointer while
   the holder keeps it is refused, and the refused trace faults when the
   holder reads.

## What this is not

A model of the Clang reading or of the CFG: that the store is
`F->m[.n…] = q` with `F`, `q` formals never rebound and the member in `*F`
is the gateway's (`retained_formal`), and faithful extraction is trust
boundary 28. The exit rule (a record whose holder outlives the frame refuses)
is `QCore.RetentionPath.replay`'s; a retention is not forwarded into the
caller's own summary.
-/

namespace QCore.MemberStoreRetention

open QCore.PointerStore (OwnerClass Decision decideStore Value)
open QCore.RetentionPath (Retained Holder HeldBy)

/-! ## 1. The flow decision -/

/-- `member_store_retention::decide`: whether the retained formal carries an
identity on a reaching path, and the destination's owner. -/
def decideRetained (carries : Bool) (o : OwnerClass) : Decision :=
  if carries then .namedValue
  else
    match o with
    | .untracked => .admit
    | .tracked => .ownedField
    | .unresolved => .unresolvedOwner

/-- The Rust `DECISION_VECTORS`. -/
def decisionVectors : List (Bool × OwnerClass × Decision) := [
  (false, .untracked, .admit),
  (false, .tracked, .ownedField),
  (false, .unresolved, .unresolvedOwner),
  (true, .untracked, .namedValue),
  (true, .tracked, .namedValue),
  (true, .unresolved, .namedValue)
]

theorem decision_vectors_hold :
    decisionVectors.all (fun (c, o, d) => decideRetained c o == d) = true := by
  decide

/-- Admitted exactly into an untracked owner, for a value that carries no
identity: a tracked owner may own its member (only O04-B08 rewrites one). -/
theorem admitted_iff_untracked_and_carrying_nothing (c : Bool) (o : OwnerClass) :
    decideRetained c o = .admit ↔ c = false ∧ o = .untracked := by
  cases c <;> cases o <;> simp [decideRetained]

/-- Rule 10v alone refuses every formal value, whatever the owner: rule 70
reads only what 10v refused for want of the owner premise. -/
theorem rule_10v_refuses_every_formal_value (c : Bool) (o : OwnerClass) :
    decideStore .formalPointer c o = .formalValue := rfl

/-! ## 2. The summary as an obligation -/

/-- A derived retention summary: each retention `(retained formal, holder
root formal, field)` and each overwritten path `(root formal, field)`. -/
structure Summary where
  retentions : List (Nat × Nat × Nat)
  overwrites : List (Nat × Nat)
  deriving DecidableEq, Repr

/-- The caller's resolution of a holder: a path holder of its own formal. -/
def isPath : Holder → Bool
  | .path _ _ => true
  | _ => false

/-- One call of a body with summary `s`: the caller's class of each retained
actual and its resolution of each holder path. `none` is the call's refusal
(an overwritten path the caller cannot name); otherwise the retentions,
decided by rule 12t's table. -/
def callEvents (s : Summary) (actual : Nat → Retained) (holder : Nat → Nat → Holder) :
    Option (List QCore.RetentionPath.Event) :=
  if s.overwrites.all (fun p => isPath (holder p.1 p.2)) then
    some (s.retentions.map fun r => .retain (QCore.RetentionPath.decide (actual r.1) (holder r.2.1 r.2.2)))
  else none

/-- **The body takes no custody**: a call of a retention summary is only
retentions; it releases, removes or destroys nothing. -/
theorem a_call_takes_no_custody (s : Summary) (a : Nat → Retained) (h : Nat → Nat → Holder)
    (es : List QCore.RetentionPath.Event) (he : callEvents s a h = some es) :
    ∀ e ∈ es, ∃ d, e = .retain d := by
  unfold callEvents at he
  split at he
  · cases he
    intro e he
    simp only [List.mem_map] at he
    obtain ⟨r, _, rfl⟩ := he
    exact ⟨_, rfl⟩
  · cases he

/-- **An overwritten path the caller cannot name refuses the call**, whatever
was stored there (P-CG11-CALLER-SLOT carried up one frame). -/
theorem an_unnamed_overwrite_refuses (s : Summary) (a : Nat → Retained)
    (h : Nat → Nat → Holder) (r f : Nat) (hm : (r, f) ∈ s.overwrites)
    (hp : isPath (h r f) = false) : callEvents s a h = none := by
  unfold callEvents
  have : s.overwrites.all (fun p => isPath (h p.1 p.2)) = false := by
    rw [List.all_eq_false]
    exact ⟨(r, f), hm, by simp [hp]⟩
  simp [this]

/-- **The retention is the caller's obligation**: a caller's object retained
in a path holder of the caller is recorded (it then refuses a release while
held, and the exit while held, `QCore.RetentionPath.replay`). -/
theorem a_callers_object_is_recorded (n r f : Nat) :
    QCore.RetentionPath.decide (.callerObject n) (.path r f) =
      .record (.name n) (.path r f) ∧
    QCore.RetentionPath.step [] (.retain (.record (.name n) (.path r f))) =
      some [⟨.name n, .path r f, false⟩] :=
  ⟨rfl, rfl⟩

/-- **The summary states the retention**: a call of the summary of
`F->m = q` (one retention of formal `q` at `F.m`, its path overwritten), whose
caller hands its own object at `q` and its own formal's path at `F.m`, is
exactly that retention's record. -/
theorem a_summary_call_states_its_retention (q r f n r' f' : Nat) (a : Nat → Retained)
    (h : Nat → Nat → Holder) (ha : a q = .callerObject n) (hh : h r f = .path r' f') :
    callEvents ⟨[(q, r, f)], [(r, f)]⟩ a h =
      some [.retain (.record (.name n) (.path r' f'))] := by
  simp [callEvents, isPath, hh, ha, QCore.RetentionPath.decide]

/-- A tracked object retained in a path holder is recorded, and its release
is refused while the record stands. -/
theorem a_tracked_object_is_held (s r f : Nat) :
    QCore.RetentionPath.step [] (.retain (QCore.RetentionPath.decide (.tracked s) (.path r f)))
      = some [⟨.site s, .path r f, false⟩] ∧
    QCore.RetentionPath.step [⟨.site s, .path r f, false⟩] (.release s) = none := by
  refine ⟨rfl, ?_⟩
  simp [QCore.RetentionPath.step, QCore.RetentionPath.holds,
    QCore.RetentionPath.HeldObject.identity]

/-- An undecided actual (anything rule 12t cannot class) refuses the call. -/
theorem an_undecided_actual_refuses (r f : Nat) :
    QCore.RetentionPath.step [] (.retain (QCore.RetentionPath.decide .other (.path r f))) = none :=
  rfl

/-- **Function pointers carry no lifetime** (the subclass): a function
designator or function-pointer value is static, so its store records
nothing and changes no record, whatever the holder. -/
theorem a_function_pointer_adds_no_obligation (h : Holder) (rs : QCore.RetentionPath.Records) :
    QCore.RetentionPath.step rs (.retain (QCore.RetentionPath.decide .static h)) = some rs := by
  cases h <;> rfl

/-! ## 3. Soundness over a concrete memory -/

/-- What a frame does: a holder now keeps object `o` (`none`: a static,
function or null value), a release of an object, a reset of a holder, the
holder's read of what it keeps, and the holder's own release of what it
keeps. -/
inductive Act where
  | store (o : Option Nat) (b : HeldBy)
  | release (s : Nat)
  | reset (b : HeldBy)
  | use (b : HeldBy)
  | releaseHeld (b : HeldBy)
  deriving DecidableEq, Repr

/-- A concrete memory: the released objects and what each holder keeps. -/
structure Mem where
  freed : List Nat
  content : HeldBy → Option Nat

def Mem.put (m : Mem) (b : HeldBy) (o : Option Nat) : Mem :=
  { m with content := fun b' => if b' = b then o else m.content b' }

/-- One step of the memory; `none` is a fault: a read through a holder of a
released object, or a double release (also by the holder). -/
def exec (m : Mem) : Act → Option Mem
  | .store o b => some (m.put b o)
  | .release s => if s ∈ m.freed then none else some { m with freed := s :: m.freed }
  | .reset b => some (m.put b none)
  | .use b =>
      match m.content b with
      | some s => if s ∈ m.freed then none else some m
      | none => some m
  | .releaseHeld b =>
      match m.content b with
      | some s => if s ∈ m.freed then none else some { (m.put b none) with freed := s :: m.freed }
      | none => some m

def execAll (m : Mem) : List Act → Option Mem
  | [] => some m
  | a :: as => (exec m a).bind fun m' => execAll m' as

/-- The checker's state: the retention records and the released objects. -/
structure Chk where
  held : List (Nat × HeldBy)
  released : List Nat

def Chk.drop (c : Chk) (b : HeldBy) : List (Nat × HeldBy) :=
  c.held.filter fun r => r.2 != b

/-- The analysis' step; `none` is a refusal. A release of a held object is
rule 12t's `retention-released-while-held`; a release of a released object
and a store of one are the base analysis' own refusals; the holder's release
of what it keeps is refused while another holder keeps it too. -/
def check (c : Chk) : Act → Option Chk
  | .store (some s) b =>
      if s ∈ c.released then none else some { c with held := (s, b) :: c.drop b }
  | .store none b => some { c with held := c.drop b }
  | .release s =>
      if s ∈ c.released then none
      else if c.held.any (fun r => r.1 == s) then none
      else some { c with released := s :: c.released }
  | .reset b => some { c with held := c.drop b }
  | .use _ => some c
  | .releaseHeld b =>
      match c.held.find? (fun r => r.2 == b) with
      | some (s, _) =>
          if s ∈ c.released then none
          else if (c.drop b).any (fun r => r.1 == s) then none
          else some { held := c.drop b, released := s :: c.released }
      | none => some c

def checkAll (c : Chk) : List Act → Option Chk
  | [] => some c
  | a :: as => (check c a).bind fun c' => checkAll c' as

/-- The invariant tying the checker to the memory: the same released objects;
everything a holder keeps is recorded for that holder and is not released;
every record is what its holder keeps. -/
def Inv (c : Chk) (m : Mem) : Prop :=
  m.freed = c.released ∧
  (∀ b s, m.content b = some s → (s, b) ∈ c.held ∧ s ∉ c.released) ∧
  (∀ s b, (s, b) ∈ c.held → m.content b = some s)

theorem mem_drop {c : Chk} {b b' : HeldBy} {s : Nat} :
    (s, b') ∈ c.drop b ↔ (s, b') ∈ c.held ∧ b' ≠ b := by
  simp [Chk.drop, List.mem_filter, bne_iff_ne]

/-- Dropping a holder keeps the invariant's coverage of every other holder. -/
theorem cover_put_none {c : Chk} {m : Mem} {b : HeldBy}
    (hcov : ∀ b s, m.content b = some s → (s, b) ∈ c.held ∧ s ∉ c.released) :
    ∀ b' s, (m.put b none).content b' = some s → (s, b') ∈ c.drop b ∧ s ∉ c.released := by
  intro b' s h'
  simp only [Mem.put] at h'
  split at h'
  · cases h'
  · rename_i hb
    obtain ⟨h1, h2⟩ := hcov b' s h'
    exact ⟨mem_drop.mpr ⟨h1, hb⟩, h2⟩

theorem records_put_none {c : Chk} {m : Mem} {b : HeldBy}
    (hrec : ∀ s b, (s, b) ∈ c.held → m.content b = some s) :
    ∀ s b', (s, b') ∈ c.drop b → (m.put b none).content b' = some s := by
  intro s b' h'
  obtain ⟨h1, h2⟩ := mem_drop.mp h'
  simp [Mem.put, h2, hrec s b' h1]

/-- **One admitted step preserves the invariant and does not fault.** -/
theorem step_sound (c c' : Chk) (m : Mem) (a : Act) (hi : Inv c m)
    (hc : check c a = some c') : ∃ m', exec m a = some m' ∧ Inv c' m' := by
  obtain ⟨hf, hcov, hrec⟩ := hi
  cases a with
  | store o b =>
      cases o with
      | some s =>
          simp only [check] at hc
          split at hc
          · cases hc
          · rename_i hs
            cases hc
            refine ⟨m.put b (some s), rfl, hf, ?_, ?_⟩
            · intro b' s' h'
              simp only [Mem.put] at h'
              split at h'
              · rename_i hb
                cases h'
                subst hb
                exact ⟨List.mem_cons_self .., hs⟩
              · rename_i hb
                obtain ⟨h1, h2⟩ := hcov b' s' h'
                exact ⟨List.mem_cons_of_mem _ (mem_drop.mpr ⟨h1, hb⟩), h2⟩
            · intro s' b' h'
              rcases List.mem_cons.mp h' with h' | h'
              · cases h'; simp [Mem.put]
              · obtain ⟨h1, h2⟩ := mem_drop.mp h'
                simp [Mem.put, h2, hrec s' b' h1]
      | none =>
          simp only [check] at hc
          cases hc
          exact ⟨m.put b none, rfl, hf, cover_put_none hcov, records_put_none hrec⟩
  | release s =>
      simp only [check] at hc
      split at hc
      · cases hc
      · rename_i hs
        split at hc
        · cases hc
        · rename_i hh
          cases hc
          have hnot : s ∉ m.freed := by rw [hf]; exact hs
          refine ⟨{ m with freed := s :: m.freed }, by simp [exec, hnot], by simp [hf], ?_, hrec⟩
          intro b' s' h'
          obtain ⟨h1, h2⟩ := hcov b' s' h'
          refine ⟨h1, ?_⟩
          intro hm
          rcases List.mem_cons.mp hm with hm | hm
          · subst hm
            apply hh
            rw [List.any_eq_true]
            exact ⟨(s', b'), h1, by simp⟩
          · exact h2 hm
  | reset b =>
      simp only [check] at hc
      cases hc
      exact ⟨m.put b none, rfl, hf, cover_put_none hcov, records_put_none hrec⟩
  | use b =>
      simp only [check] at hc
      cases hc
      refine ⟨m, ?_, hf, hcov, hrec⟩
      simp only [exec]
      cases hb : m.content b with
      | none => rfl
      | some s =>
          have : s ∉ m.freed := by rw [hf]; exact (hcov b s hb).2
          simp [this]
  | releaseHeld b =>
      simp only [check] at hc
      cases hfind : c.held.find? (fun r => r.2 == b) with
      | none =>
          rw [hfind] at hc
          cases hc
          refine ⟨m, ?_, hf, hcov, hrec⟩
          simp only [exec]
          cases hb : m.content b with
          | none => rfl
          | some s =>
              exfalso
              have hmem := (hcov b s hb).1
              have := List.find?_eq_none.mp hfind (s, b) hmem
              simp at this
      | some r =>
          obtain ⟨s, b0⟩ := r
          rw [hfind] at hc
          simp only at hc
          have hmem := List.mem_of_find?_eq_some hfind
          have hb0 : b0 = b := by
            have := List.find?_some hfind
            simpa using this
          subst hb0
          have hcont : m.content b0 = some s := hrec s b0 hmem
          split at hc
          · cases hc
          · rename_i hs
            split at hc
            · cases hc
            · rename_i hother
              cases hc
              have hnot : s ∉ m.freed := by rw [hf]; exact hs
              refine ⟨{ (m.put b0 none) with freed := s :: m.freed },
                by simp [exec, hcont, hnot], by simp [hf], ?_, ?_⟩
              · intro b' s' h'
                have h'' : (m.put b0 none).content b' = some s' := h'
                obtain ⟨hd, h2⟩ := cover_put_none hcov b' s' h''
                refine ⟨hd, ?_⟩
                intro hm
                rcases List.mem_cons.mp hm with hm | hm
                · subst hm
                  apply hother
                  rw [List.any_eq_true]
                  exact ⟨(s', b'), hd, by simp⟩
                · exact h2 hm
              · exact records_put_none hrec

/-- The empty frame: nothing held, nothing released. -/
def Chk.init : Chk := ⟨[], []⟩
def Mem.init : Mem := ⟨[], fun _ => none⟩

theorem init_inv : Inv Chk.init Mem.init := by
  refine ⟨rfl, ?_, ?_⟩
  · intro b s h; cases h
  · intro s b h; cases h

/-- **Soundness**: every trace the checker admits from a related state
executes without a fault, and the invariant holds at its end. -/
theorem checked_traces_do_not_fault (as : List Act) :
    ∀ (c : Chk) (m : Mem), Inv c m → ∀ c', checkAll c as = some c' →
      ∃ m', execAll m as = some m' ∧ Inv c' m' := by
  induction as with
  | nil =>
      intro c m hi c' hc
      cases hc
      exact ⟨m, rfl, hi⟩
  | cons a as ih =>
      intro c m hi c' hc
      simp only [checkAll] at hc
      cases h1 : check c a with
      | none => rw [h1] at hc; cases hc
      | some c1 =>
          rw [h1] at hc
          simp only [Option.bind_some] at hc
          obtain ⟨m1, he, hi1⟩ := step_sound c c1 m a hi h1
          obtain ⟨m', he', hi'⟩ := ih c1 m1 hi1 c' hc
          exact ⟨m', by simp [execAll, he, he'], hi'⟩

/-- **A caller that keeps the pointer alive while the holder retains it has
no use-after-free and no double release through the holder**: from the empty
frame, an admitted trace never faults. -/
theorem an_admitted_caller_never_faults (as : List Act)
    (h : checkAll Chk.init as ≠ none) : execAll Mem.init as ≠ none := by
  cases hc : checkAll Chk.init as with
  | none => exact absurd hc h
  | some c' =>
      obtain ⟨m', he, _⟩ := checked_traces_do_not_fault as Chk.init Mem.init init_inv c' hc
      rw [he]
      simp

theorem checkAll_append (c : Chk) (p q : List Act) :
    checkAll c (p ++ q) = (checkAll c p).bind fun c' => checkAll c' q := by
  induction p generalizing c with
  | nil => simp [checkAll]
  | cons a as ih =>
      simp only [List.cons_append, checkAll]
      cases check c a with
      | none => rfl
      | some c1 => simp only [Option.bind_some]; exact ih c1

/-- **Caller discharge composes**: two admitted segments (two calls, each
discharged where it stands) are admitted together, and the whole is sound by
`checked_traces_do_not_fault`. -/
theorem caller_discharge_composes (c c1 c2 : Chk) (p q : List Act)
    (h1 : checkAll c p = some c1) (h2 : checkAll c1 q = some c2) :
    checkAll c (p ++ q) = some c2 := by
  rw [checkAll_append, h1]
  exact h2

/-- The same composition for calls of retention summaries, on rule 12t's
records (`QCore.RetentionPath.run`). -/
theorem summary_calls_compose (rs r1 r2 : QCore.RetentionPath.Records)
    (e1 e2 : List QCore.RetentionPath.Event)
    (h1 : QCore.RetentionPath.run rs e1 = some r1) (h2 : QCore.RetentionPath.run r1 e2 = some r2) :
    QCore.RetentionPath.run rs (e1 ++ e2) = some r2 := by
  rw [QCore.RetentionPath.run_append, h1]
  exact h2

/-- A store of no object (a function pointer, a static or null value) never
adds an obligation: every release refused after it was refused before. -/
theorem a_static_store_adds_no_obligation (c c' : Chk) (b : HeldBy) (s : Nat)
    (hc : check c (.store none b) = some c') (hr : check c' (.release s) = none) :
    check c (.release s) = none := by
  simp only [check] at hc
  cases hc
  simp only [check] at hr ⊢
  by_cases h : s ∈ c.released
  · simp [h]
  · simp only [h, ite_false] at hr ⊢
    by_cases hany : (c.held.filter fun r => r.2 != b).any (fun r => r.1 == s) = true
    · have : c.held.any (fun r => r.1 == s) = true := by
        rw [List.any_eq_true] at hany ⊢
        obtain ⟨r, hr', he⟩ := hany
        exact ⟨r, (List.mem_filter.mp hr').1, he⟩
      simp only [this, ite_true]
    · exfalso
      have hany' : (c.held.filter fun r => r.2 != b).any (fun r => r.1 == s) = false := by
        cases hx : (c.held.filter fun r => r.2 != b).any (fun r => r.1 == s)
        · rfl
        · exact absurd hx hany
      simp [Chk.drop, hany'] at hr

/-! ## 4. The counterexample -/

def holderB : HeldBy := .path 0 0

/-- **A caller that releases the retained pointer while the holder keeps it
is refused.** -/
theorem releasing_the_retained_pointer_is_refused :
    checkAll Chk.init [.store (some 1) holderB, .release 1] = none := by
  simp [checkAll, check, Chk.init, Chk.drop, holderB]

/-- The refused trace is a real hazard: when the holder reads, it faults. -/
theorem the_refused_trace_faults_when_the_holder_reads :
    execAll Mem.init [.store (some 1) holderB, .release 1, .use holderB] = none := by
  simp [execAll, exec, Mem.init, Mem.put, holderB]

/-- A caller that resets the holder before the release discharges it. -/
theorem a_reset_before_the_release_discharges_it :
    checkAll Chk.init [.store (some 1) holderB, .use holderB, .reset holderB, .release 1]
      ≠ none := by
  simp [checkAll, check, Chk.init, Chk.drop, holderB]

/-- **A caller that cannot discharge stays refused**: a retention still held
by a path holder at its exit outlives the frame (`QCore.RetentionPath.replay`). -/
theorem a_caller_that_cannot_discharge_is_refused (n r f : Nat) :
    QCore.RetentionPath.replay
      [.retain (QCore.RetentionPath.decide (.callerObject n) (.path r f))] = false := rfl

end QCore.MemberStoreRetention
