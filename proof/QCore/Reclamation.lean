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
# Hazard-pointer and epoch-based reclamation (card CG29-EXECUTION-COVERAGE-AND-RECLAMATION)

The Rust side is `crates/qcore-core/src/reclamation.rs`. Gap entry W10: do
not claim lock-free reclamation safety from allocation generations alone;
connect the hazard-pointer and epoch protocols to contracts and a checker.

Each protocol is a checker over a trace of protocol steps. An object is
`fresh`, then `linked` into the shared structure, `unlinked`, `retired` and
finally `reclaimed`. The checker enforces only the protocol's own rules.
It never tests "this object is not reclaimed" at an access. That fact is
proved here from the rules, so the rules are what is relied on.

**Hazard pointers** (`hpStep`).
* A thread publishes a hazard in one of its slots (`protect`).
* It validates the hazard only while the object is still linked
  (`validate`: the re-read of the source still sees it).
* It accesses an object only through a validated hazard.
* An object is retired only once unlinked, and reclaimed only when retired
  and no slot of any thread holds it.

**Epoch-based reclamation** (`ebrStep`).
* A thread enters a critical section at the current global epoch and may
  access a linked object, or one unlinked during its critical section.
* The epoch advances only when every active thread is in the current epoch.
* A retired object is reclaimed only two epochs after its retirement.

What is proved: in every trace either checker accepts, no access is made to
a reclaimed object (`hp_accepted_safe`, `ebr_accepted_safe`). The invariants
behind this are `hp_step_preserves` and `ebr_step_preserves`.

The trace is the protocol's steps in one interleaving. That this
interleaving exists for the C program, i.e. that the reclaimer's read of the
hazards (or announcements) cannot miss a validated reader, is the
store-buffering handshake. `QCore.ExecutionCoverage` decides that handshake
over every execution (`sbFences` race-free, `sbRelaxed` racy). A helper
implementing a protocol step is a `Declared` boundary on the Rust side.
-/

namespace QCore.Reclamation

inductive Status where
  | fresh | linked | unlinked | retired | reclaimed
deriving DecidableEq, Repr

inductive Slot where
  | empty
  | pending (o : Nat)
  | valid (o : Nat)
deriving DecidableEq, Repr

inductive Ev where
  | link (o : Nat)
  | unlink (o : Nat)
  | protect (t s o : Nat)
  | validate (t s o : Nat)
  | clear (t s : Nat)
  /-- An access by thread `t` (through hazard slot `s`, for hazard pointers). -/
  | access (t s o : Nat)
  | retire (t o : Nat)
  | reclaim (t o : Nat)
  | enter (t : Nat)
  | exit (t : Nat)
  | advance (t : Nat)
deriving DecidableEq, Repr

inductive Violation where
  | notFresh | notLinked | outOfRange | validationFailed | unprotectedAccess | notUnlinked
  | notRetired | hazardHeld | foreignEvent | notActive | alreadyActive | epochNotQuiescent
  | gracePeriod
deriving DecidableEq, Repr

def upd {α : Type} (f : Nat → α) (a : Nat) (v : α) : Nat → α := fun x => if x = a then v else f x

def upd2 {α : Type} (f : Nat → Nat → α) (a b : Nat) (v : α) : Nat → Nat → α :=
  fun x y => if x = a ∧ y = b then v else f x y

/-! ## Hazard pointers -/

structure HpConfig where
  threads : Nat
  slots : Nat
deriving DecidableEq, Repr

structure HpState where
  status : Nat → Status
  slot : Nat → Nat → Slot

def HpState.init : HpState := ⟨fun _ => .fresh, fun _ _ => .empty⟩

def Slot.holds : Slot → Nat → Bool
  | .pending o, o' => o == o'
  | .valid o, o' => o == o'
  | .empty, _ => false

/-- The reclaimer's scan: no slot of any thread holds `o`. -/
def hazardFree (c : HpConfig) (s : HpState) (o : Nat) : Bool :=
  (List.range c.threads).all fun t => (List.range c.slots).all fun i => !(s.slot t i).holds o

def hpStep (c : HpConfig) (s : HpState) : Ev → Except Violation HpState
  | .link o =>
    if s.status o = .fresh then .ok { s with status := upd s.status o .linked } else .error .notFresh
  | .unlink o =>
    if s.status o = .linked then .ok { s with status := upd s.status o .unlinked }
    else .error .notLinked
  | .protect t i o =>
    if t < c.threads ∧ i < c.slots then .ok { s with slot := upd2 s.slot t i (.pending o) }
    else .error .outOfRange
  | .validate t i o =>
    if s.slot t i = .pending o ∧ s.status o = .linked then
      .ok { s with slot := upd2 s.slot t i (.valid o) }
    else .error .validationFailed
  | .clear t i =>
    if t < c.threads ∧ i < c.slots then .ok { s with slot := upd2 s.slot t i .empty }
    else .error .outOfRange
  | .access t i o => if s.slot t i = .valid o then .ok s else .error .unprotectedAccess
  | .retire _ o =>
    if s.status o = .unlinked then .ok { s with status := upd s.status o .retired }
    else .error .notUnlinked
  | .reclaim _ o =>
    if s.status o = .retired then
      if hazardFree c s o then .ok { s with status := upd s.status o .reclaimed }
      else .error .hazardHeld
    else .error .notRetired
  | _ => .error .foreignEvent

/-- Every validated hazard names an object that is linked, unlinked or
retired, never fresh or reclaimed; slots outside the configuration stay
empty. -/
def HpInv (c : HpConfig) (s : HpState) : Prop :=
  (∀ t i o, s.slot t i = .valid o →
    s.status o = .linked ∨ s.status o = .unlinked ∨ s.status o = .retired) ∧
  (∀ t i, ¬ (t < c.threads ∧ i < c.slots) → s.slot t i = .empty)

theorem hp_init_inv (c : HpConfig) : HpInv c HpState.init :=
  ⟨fun _ _ _ h => by simp [HpState.init] at h, fun _ _ _ => rfl⟩

theorem hazardFree_spec {c : HpConfig} {s : HpState} {o : Nat} (h : hazardFree c s o = true)
    {t i : Nat} (ht : t < c.threads) (hi : i < c.slots) : (s.slot t i).holds o = false := by
  simp only [hazardFree, List.all_eq_true, List.mem_range] at h
  simpa using h t ht i hi

theorem hp_step_preserves {c : HpConfig} {s s' : HpState} {e : Ev} (hinv : HpInv c s)
    (h : hpStep c s e = .ok s') : HpInv c s' := by
  obtain ⟨hv, hr⟩ := hinv
  cases e with
  | link o =>
    simp only [hpStep] at h
    split at h
    · rename_i hf
      cases h
      refine ⟨fun t i o' hs => ?_, hr⟩
      have := hv t i o' hs
      simp only [upd]
      split
      · subst_vars; rw [hf] at this; simp at this
      · exact this
    · cases h
  | unlink o =>
    simp only [hpStep] at h
    split at h
    · cases h
      refine ⟨fun t i o' hs => ?_, hr⟩
      have := hv t i o' hs
      simp only [upd]
      split
      · simp
      · exact this
    · cases h
  | protect t i o =>
    simp only [hpStep] at h
    split at h
    · rename_i hin
      cases h
      refine ⟨fun t' i' o' hs => ?_, fun t' i' hn => ?_⟩
      · simp only [upd2] at hs
        split at hs
        · cases hs
        · exact hv t' i' o' hs
      · simp only [upd2]
        split
        · rename_i heq; obtain ⟨rfl, rfl⟩ := heq; exact absurd hin hn
        · exact hr t' i' hn
    · cases h
  | validate t i o =>
    simp only [hpStep] at h
    split at h
    · rename_i hc
      cases h
      refine ⟨fun t' i' o' hs => ?_, fun t' i' hn => ?_⟩
      · simp only [upd2] at hs
        split at hs
        · cases hs; exact .inl hc.2
        · exact hv t' i' o' hs
      · simp only [upd2]
        split
        · rename_i heq; obtain ⟨rfl, rfl⟩ := heq
          have := hr t' i' hn; rw [this] at hc; cases hc.1
        · exact hr t' i' hn
    · cases h
  | clear t i =>
    simp only [hpStep] at h
    split at h
    · cases h
      refine ⟨fun t' i' o' hs => ?_, fun t' i' hn => ?_⟩
      · simp only [upd2] at hs
        split at hs
        · cases hs
        · exact hv t' i' o' hs
      · simp only [upd2]
        split
        · rfl
        · exact hr t' i' hn
    · cases h
  | access t i o =>
    simp only [hpStep] at h
    split at h
    · cases h; exact ⟨hv, hr⟩
    · cases h
  | retire t o =>
    simp only [hpStep] at h
    split at h
    · cases h
      refine ⟨fun t' i o' hs => ?_, hr⟩
      have := hv t' i o' hs
      simp only [upd]
      split
      · simp
      · exact this
    · cases h
  | reclaim t o =>
    simp only [hpStep] at h
    split at h
    · split at h
      · rename_i _ hfree
        cases h
        refine ⟨fun t' i o' hs => ?_, hr⟩
        simp only [upd]
        split
        · rename_i heq
          subst heq
          by_cases hin : t' < c.threads ∧ i < c.slots
          · have := hazardFree_spec hfree hin.1 hin.2
            rw [hs] at this; simp [Slot.holds] at this
          · rw [hr t' i hin] at hs; cases hs
        · exact hv t' i o' hs
      · cases h
    · cases h
  | enter t => simp [hpStep] at h
  | exit t => simp [hpStep] at h
  | advance t => simp [hpStep] at h

/-- **Hazard-pointer safety, one step.** An accepted access is to an object
that is not reclaimed (nor fresh). -/
theorem hp_access_safe {c : HpConfig} {s s' : HpState} {t i o : Nat} (hinv : HpInv c s)
    (h : hpStep c s (.access t i o) = .ok s') : s.status o ≠ .reclaimed := by
  simp only [hpStep] at h
  split at h
  · rename_i hs
    rcases hinv.1 t i o hs with h1 | h1 | h1 <;> rw [h1] <;> simp
  · cases h

def hpRun (c : HpConfig) : HpState → List Ev → Except Violation HpState
  | s, [] => .ok s
  | s, e :: es =>
    match hpStep c s e with
    | .ok s' => hpRun c s' es
    | .error v => .error v

/-- Every access of the trace, from state `s`, is to an object not yet
reclaimed when it is made. -/
def HpSafe (c : HpConfig) : HpState → List Ev → Prop
  | _, [] => True
  | s, e :: es =>
    (∀ t i o, e = .access t i o → s.status o ≠ .reclaimed) ∧
      ∀ s', hpStep c s e = .ok s' → HpSafe c s' es

/-- **Hazard-pointer safety.** In every trace the checker accepts, no access
is made to a reclaimed object. -/
theorem hp_accepted_safe (c : HpConfig) :
    ∀ (es : List Ev) (s : HpState), HpInv c s → (∃ s', hpRun c s es = .ok s') → HpSafe c s es := by
  intro es
  induction es with
  | nil => intros; trivial
  | cons e es ih =>
    intro s hinv ⟨sf, hrun⟩
    simp only [hpRun] at hrun
    split at hrun
    · rename_i s' hstep
      refine ⟨fun t i o he => ?_, fun s'' hs'' => ?_⟩
      · subst he; exact hp_access_safe hinv hstep
      · rw [hstep] at hs''; cases hs''
        exact ih s' (hp_step_preserves hinv hstep) ⟨sf, hrun⟩
    · cases hrun

theorem hp_trace_safe (c : HpConfig) (es : List Ev) (h : ∃ s', hpRun c HpState.init es = .ok s') :
    HpSafe c HpState.init es :=
  hp_accepted_safe c es _ (hp_init_inv c) h

/-! ## Epoch-based reclamation -/

structure EbrConfig where
  threads : Nat
deriving DecidableEq, Repr

structure EbrState where
  status : Nat → Status
  epoch : Nat
  /-- The epoch an active thread entered at; `none` when quiescent. -/
  announced : Nat → Option Nat
  /-- `o` was unlinked while `t` was active, so `t` may still hold it. -/
  reach : Nat → Nat → Bool
  retiredAt : Nat → Option Nat

def EbrState.init : EbrState :=
  ⟨fun _ => .fresh, 0, fun _ => none, fun _ _ => false, fun _ => none⟩

/-- Every active thread has announced the current epoch. -/
def quiescent (c : EbrConfig) (s : EbrState) : Bool :=
  (List.range c.threads).all fun t => s.announced t == none || s.announced t == some s.epoch

def ebrStep (c : EbrConfig) (s : EbrState) : Ev → Except Violation EbrState
  | .link o =>
    if s.status o = .fresh then .ok { s with status := upd s.status o .linked } else .error .notFresh
  | .enter t =>
    if t < c.threads then
      if s.announced t = none then
        .ok { s with announced := upd s.announced t (some s.epoch),
                     reach := upd s.reach t (fun _ => false) }
      else .error .alreadyActive
    else .error .outOfRange
  | .exit t =>
    if (s.announced t).isSome then
      .ok { s with announced := upd s.announced t none, reach := upd s.reach t (fun _ => false) }
    else .error .notActive
  | .unlink o =>
    if s.status o = .linked then
      .ok { s with status := upd s.status o .unlinked,
                   reach := fun t o' => (o' == o && (s.announced t).isSome) || s.reach t o' }
    else .error .notLinked
  | .access t _ o =>
    if (s.announced t).isSome ∧ (s.status o = .linked ∨ s.reach t o = true) then .ok s
    else .error .unprotectedAccess
  | .retire _ o =>
    if s.status o = .unlinked then
      .ok { s with status := upd s.status o .retired, retiredAt := upd s.retiredAt o (some s.epoch) }
    else .error .notUnlinked
  | .advance _ =>
    if quiescent c s then .ok { s with epoch := s.epoch + 1 } else .error .epochNotQuiescent
  | .reclaim _ o =>
    if s.status o = .retired then
      match s.retiredAt o with
      | some r => if r + 2 ≤ s.epoch then .ok { s with status := upd s.status o .reclaimed }
                  else .error .gracePeriod
      | none => .error .notRetired
    else .error .notRetired
  | _ => .error .foreignEvent

/-- Active threads are in range and at most one epoch behind; a thread that
may hold an unlinked object entered no later than its retirement. -/
def EbrInv (c : EbrConfig) (s : EbrState) : Prop :=
  (∀ t et, s.announced t = some et → t < c.threads ∧ et ≤ s.epoch ∧ s.epoch ≤ et + 1) ∧
  (∀ t o, s.reach t o = true → ∃ et, s.announced t = some et ∧
    (s.status o = .unlinked ∨ (s.status o = .retired ∧ ∀ r, s.retiredAt o = some r → et ≤ r)))

theorem ebr_init_inv (c : EbrConfig) : EbrInv c EbrState.init :=
  ⟨fun _ _ h => by simp [EbrState.init] at h, fun _ _ h => by simp [EbrState.init] at h⟩

theorem quiescent_spec {c : EbrConfig} {s : EbrState} (h : quiescent c s = true) {t et : Nat}
    (ht : t < c.threads) (he : s.announced t = some et) : et = s.epoch := by
  simp only [quiescent, List.all_eq_true, List.mem_range] at h
  have := h t ht
  simp [he] at this
  exact this

theorem ebr_step_preserves {c : EbrConfig} {s s' : EbrState} {e : Ev} (hinv : EbrInv c s)
    (h : ebrStep c s e = .ok s') : EbrInv c s' := by
  obtain ⟨ha, hb⟩ := hinv
  cases e with
  | link o =>
    simp only [ebrStep] at h
    split at h
    · rename_i hf
      cases h
      refine ⟨ha, fun t o' hr => ?_⟩
      obtain ⟨et, het, hst⟩ := hb t o' hr
      refine ⟨et, het, ?_⟩
      simp only [upd]
      split
      · subst_vars; rw [hf] at hst; simp at hst
      · exact hst
    · cases h
  | enter t =>
    simp only [ebrStep] at h
    split at h
    · rename_i ht
      split at h
      · cases h
        refine ⟨fun t' et he => ?_, fun t' o hr => ?_⟩
        · simp only [upd] at he
          split at he
          · subst_vars; cases he; exact ⟨ht, Nat.le_refl _, Nat.le_succ _⟩
          · exact ha t' et he
        · simp only [upd] at hr ⊢
          split at hr
          · cases hr
          · rename_i hne
            obtain ⟨et, het, hst⟩ := hb t' o hr
            exact ⟨et, by simp [hne, het], hst⟩
      · cases h
    · cases h
  | exit t =>
    simp only [ebrStep] at h
    split at h
    · cases h
      refine ⟨fun t' et he => ?_, fun t' o hr => ?_⟩
      · simp only [upd] at he
        split at he
        · cases he
        · exact ha t' et he
      · simp only [upd] at hr ⊢
        split at hr
        · cases hr
        · rename_i hne
          obtain ⟨et, het, hst⟩ := hb t' o hr
          exact ⟨et, by simp [hne, het], hst⟩
    · cases h
  | unlink o =>
    simp only [ebrStep] at h
    split at h
    · rename_i hl
      cases h
      refine ⟨ha, fun t o' hr => ?_⟩
      simp only [Bool.or_eq_true, Bool.and_eq_true, beq_iff_eq] at hr
      rcases hr with ⟨rfl, hact⟩ | hr
      · obtain ⟨et, het⟩ := Option.isSome_iff_exists.mp hact
        exact ⟨et, het, .inl (by simp [upd])⟩
      · obtain ⟨et, het, hst⟩ := hb t o' hr
        refine ⟨et, het, ?_⟩
        simp only [upd]
        split
        · subst_vars; rw [hl] at hst; simp at hst
        · exact hst
    · cases h
  | access t i o =>
    simp only [ebrStep] at h
    split at h
    · cases h; exact ⟨ha, hb⟩
    · cases h
  | retire t o =>
    simp only [ebrStep] at h
    split at h
    · rename_i hu
      cases h
      refine ⟨ha, fun t' o' hr => ?_⟩
      obtain ⟨et, het, hst⟩ := hb t' o' hr
      refine ⟨et, het, ?_⟩
      simp only [upd]
      split
      · rename_i heq
        subst heq
        refine .inr ⟨rfl, fun r hr' => ?_⟩
        simp at hr'
        subst hr'
        exact (ha t' et het).2.1
      · exact hst
    · cases h
  | advance t =>
    simp only [ebrStep] at h
    split at h
    · rename_i hq
      cases h
      refine ⟨fun t' et he => ?_, hb⟩
      have hrange := (ha t' et he).1
      have := quiescent_spec hq hrange he
      subst this
      exact ⟨hrange, Nat.le_succ _, Nat.le_refl _⟩
    · cases h
  | reclaim t o =>
    simp only [ebrStep] at h
    split at h
    · rename_i hret
      split at h
      · rename_i r hr
        split at h
        · rename_i hgrace
          cases h
          refine ⟨ha, fun t' o' hreach => ?_⟩
          obtain ⟨et, het, hst⟩ := hb t' o' hreach
          simp only [upd]
          split
          · rename_i heq
            subst heq
            exfalso
            rcases hst with hst | ⟨-, hle⟩
            · rw [hret] at hst; cases hst
            · have h1 := hle r hr
              have h2 := (ha t' et het).2.2
              omega
          · exact ⟨et, het, hst⟩
        · cases h
      · cases h
    · cases h
  | protect t i o => simp [ebrStep] at h
  | validate t i o => simp [ebrStep] at h
  | clear t i => simp [ebrStep] at h

/-- **Epoch safety, one step.** An accepted access is to an object that is
not reclaimed. -/
theorem ebr_access_safe {c : EbrConfig} {s s' : EbrState} {t i o : Nat} (hinv : EbrInv c s)
    (h : ebrStep c s (.access t i o) = .ok s') : s.status o ≠ .reclaimed := by
  simp only [ebrStep] at h
  split at h
  · rename_i hc
    rcases hc.2 with hl | hr
    · rw [hl]; simp
    · obtain ⟨et, -, hst⟩ := hinv.2 t o hr
      rcases hst with hst | ⟨hst, -⟩ <;> rw [hst] <;> simp
  · cases h

def ebrRun (c : EbrConfig) : EbrState → List Ev → Except Violation EbrState
  | s, [] => .ok s
  | s, e :: es =>
    match ebrStep c s e with
    | .ok s' => ebrRun c s' es
    | .error v => .error v

def EbrSafe (c : EbrConfig) : EbrState → List Ev → Prop
  | _, [] => True
  | s, e :: es =>
    (∀ t i o, e = .access t i o → s.status o ≠ .reclaimed) ∧
      ∀ s', ebrStep c s e = .ok s' → EbrSafe c s' es

/-- **Epoch safety.** In every trace the checker accepts, no access is made
to a reclaimed object. -/
theorem ebr_accepted_safe (c : EbrConfig) :
    ∀ (es : List Ev) (s : EbrState), EbrInv c s → (∃ s', ebrRun c s es = .ok s') →
      EbrSafe c s es := by
  intro es
  induction es with
  | nil => intros; trivial
  | cons e es ih =>
    intro s hinv ⟨sf, hrun⟩
    simp only [ebrRun] at hrun
    split at hrun
    · rename_i s' hstep
      refine ⟨fun t i o he => ?_, fun s'' hs'' => ?_⟩
      · subst he; exact ebr_access_safe hinv hstep
      · rw [hstep] at hs''; cases hs''
        exact ih s' (ebr_step_preserves hinv hstep) ⟨sf, hrun⟩
    · cases hrun

theorem ebr_trace_safe (c : EbrConfig) (es : List Ev) (h : ∃ s', ebrRun c EbrState.init es = .ok s') :
    EbrSafe c EbrState.init es :=
  ebr_accepted_safe c es _ (ebr_init_inv c) h

/-! ## Shared trace vectors

Each `def` is parsed by the Rust binding test
(`crates/qcore-core/tests/unit/reclamation.rs`), run through
`check_hazard_pointer_trace` or `check_epoch_trace`, and its outcome (the
index and violation of the first refused step, or acceptance) compared with
`hpVectors` / `ebrVectors`. Objects are 1 and 2, threads 0 and 1. -/

/-- Reader 0 protects and validates object 1 before the writer unlinks and
retires it; the reclaim waits for the hazard to clear. -/
def hpProtectedRead : List Ev :=
  [.link 1, .protect 0 0 1, .validate 0 0 1, .unlink 1, .retire 1 1, .access 0 0 1,
    .clear 0 0, .reclaim 1 1]

/-- Use after retire: an access with no validated hazard. -/
def hpUseAfterRetire : List Ev :=
  [.link 1, .unlink 1, .retire 1 1, .access 0 0 1]

/-- A hazard published after the unlink cannot be validated. -/
def hpLateValidation : List Ev :=
  [.link 1, .unlink 1, .protect 0 0 1, .validate 0 0 1, .access 0 0 1]

/-- A reclaim while a hazard still holds the object. -/
def hpReclaimWhileHazard : List Ev :=
  [.link 1, .protect 0 0 1, .validate 0 0 1, .unlink 1, .retire 1 1, .reclaim 1 1]

/-- A reclaim of an object never retired. -/
def hpReclaimUnretired : List Ev :=
  [.link 1, .unlink 1, .reclaim 1 1]

/-- Re-protecting the slot for another object drops the first hazard. -/
def hpReprotect : List Ev :=
  [.link 1, .link 2, .protect 0 0 1, .validate 0 0 1, .protect 0 0 2, .access 0 0 1]

/-- A thread in the critical section keeps object 1 alive across one advance;
the reclaim comes two epochs after the retire. -/
def ebrGracePeriod : List Ev :=
  [.link 1, .enter 0, .access 0 0 1, .unlink 1, .retire 1 1, .access 0 0 1, .exit 0,
    .advance 1, .advance 1, .reclaim 1 1]

/-- The epoch cannot advance past a thread still in the previous epoch. -/
def ebrStalledAdvance : List Ev :=
  [.link 1, .enter 0, .unlink 1, .retire 1 1, .advance 1, .advance 1]

/-- A reclaim one epoch after the retire is refused. -/
def ebrEarlyReclaim : List Ev :=
  [.link 1, .unlink 1, .retire 1 1, .advance 1, .reclaim 1 1]

/-- A thread entering after the unlink cannot access the object. -/
def ebrLateEntry : List Ev :=
  [.link 1, .unlink 1, .enter 0, .access 0 0 1]

/-- An access outside a critical section. -/
def ebrOutsideSection : List Ev :=
  [.link 1, .access 0 0 1]

/-- The index and violation of the first refused step, if any. -/
def outcome {σ : Type} (step : σ → Ev → Except Violation σ) : σ → Nat → List Ev →
    Option (Nat × Violation)
  | _, _, [] => none
  | s, n, e :: es =>
    match step s e with
    | .ok s' => outcome step s' (n + 1) es
    | .error v => some (n, v)

def hpTwo : HpConfig := { threads := 2, slots := 1 }
def ebrTwo : EbrConfig := { threads := 2 }

def hpVectors : List (String × List Ev × Option (Nat × Violation)) := [
  ("hpProtectedRead", hpProtectedRead, none),
  ("hpUseAfterRetire", hpUseAfterRetire, some (3, .unprotectedAccess)),
  ("hpLateValidation", hpLateValidation, some (3, .validationFailed)),
  ("hpReclaimWhileHazard", hpReclaimWhileHazard, some (5, .hazardHeld)),
  ("hpReclaimUnretired", hpReclaimUnretired, some (2, .notRetired)),
  ("hpReprotect", hpReprotect, some (5, .unprotectedAccess))
]

def ebrVectors : List (String × List Ev × Option (Nat × Violation)) := [
  ("ebrGracePeriod", ebrGracePeriod, none),
  ("ebrStalledAdvance", ebrStalledAdvance, some (5, .epochNotQuiescent)),
  ("ebrEarlyReclaim", ebrEarlyReclaim, some (4, .gracePeriod)),
  ("ebrLateEntry", ebrLateEntry, some (3, .unprotectedAccess)),
  ("ebrOutsideSection", ebrOutsideSection, some (1, .unprotectedAccess))
]

theorem hp_vectors_hold :
    hpVectors.all (fun v => outcome (hpStep hpTwo) HpState.init 0 v.2.1 == v.2.2) = true := by
  decide

theorem ebr_vectors_hold :
    ebrVectors.all (fun v => outcome (ebrStep ebrTwo) EbrState.init 0 v.2.1 == v.2.2) = true := by
  decide

theorem hp_use_after_retire_refused :
    outcome (hpStep hpTwo) HpState.init 0 hpUseAfterRetire = some (3, .unprotectedAccess) := by
  decide

end QCore.Reclamation
