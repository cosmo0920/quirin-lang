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
# Holder paths, the removal of one registration, and removal poisoning
(cards FB-EVENT-LOOP-RETENTION-AND-REGISTRATION and
RETENTION-UNMATCHED-REMOVAL-POISONS, rule 12t)

The abstract rule implemented by `crates/qcore-c-gateway/src/retained_by.rs`
(`decide`, `removal_key`, `remove`, `end_holder`, `replay`,
`PATH_DECISION_VECTORS`, `PATH_TRACE_VECTORS`) and its route-G adapter
`legacy_retained_by.rs`, contract specification §C04.14.

USER DECISIONS 2026-10-16: "Tracked holder path" — a holder reached through a
member is spelled as a path and tracked by the analysis; "R3: quant state" —
`quant live #loop: #holds(event) |-> #absent;` removes exactly the
registration of `event` from `loop`, every other registration unchanged.

USER DECISION 2026-10-17 ("Yes, small fix card now"), card
RETENTION-UNMATCHED-REMOVAL-POISONS: Monkey's `_mk_event_del` clears the
event's registered status even through a loop that does not hold it, so a
later removal through the right loop does nothing. The previous model read a
removal that matched no record as the identity (`an_unmatched_removal_keeps_every_record`)
and then let a later matching removal end the record: the sequence "unmatched
removal, matched removal, release" decided although the right loop still held
the event. **Corrected:** a removal drops only the exactly matching record that
is not poisoned, and poisons every other record whose object it may name; a
poisoned record ends only when its holder ends (`Event.destroy`, or the
release of a tracked holder).

## The model

`QCore.RetainedBy`'s table, extended by a path holder (`Holder.path`: the
object at a member path below an unrebound formal, resolved at the call), an
interior of a tracked identity (`Retained.interior`), a removal event
(`Event.remove`, carrying the object and the holder it names, each `none` when
undecided) and a destroy of a holder (`Event.destroy`). A record carries a
`poisoned` flag. A path holder outlives the frame like a tracked holder: a
record held there refuses at the exit unless it ended first.

## The theorems

* `an_unresolved_path_never_decides`.
* `a_removal_drops_exactly_its_unpoisoned_record` and
  `an_unmatched_removal_poisons_what_it_may_name` (the corrected removal step;
  they replace `a_removal_drops_exactly_its_record` and
  `an_unmatched_removal_keeps_every_record`, which admitted the defect).
* `a_matched_removal_ends_an_unpoisoned_record`.
* `a_poisoned_record_ends_only_by_its_loop_destroy`: a poisoned record is
  still there after any events none of which ends its holder.
* `the_old_sequence_no_longer_decides`: a record removed through another path,
  then through its own, is never `Safe` without its loop's destroy.
* `a_path_record_ends_only_by_its_own_removal_or_its_destroy` (corrected from
  `a_path_record_ends_only_by_its_own_removal`, which a destroy now refutes)
  and `a_path_record_without_its_removal_or_destroy_is_never_safe`.
* `a_destroy_ends_every_record_at_its_loop`.
* `a_safe_path_never_releases_a_held_identity`,
  `a_safe_path_leaves_no_outliving_holder_holding`.
* `path_decision_vectors_hold`, `path_trace_vectors_hold`: the tables shared
  with Rust.

## What this is not

A model over abstract identities. That the gateway resolves a path only below
an unrebound formal named through member accesses off the path
(`path_stable`, premise `P-HOLDER-PATH-STABLE`), and reads `&X->f` as an
interior of `X`, are review premises (trust boundary 28), pinned by the shared
vectors and `crates/qcore-c-gateway/tests/event_loop_retention.rs`. Two
distinct paths are never read as one loop (no derived equality).
-/

namespace QCore.RetentionPath

inductive Retained where
  | static
  | callerObject (name : Nat)
  | tracked (site : Nat)
  | interior (site : Nat) (field : Nat)
  | other
  deriving DecidableEq, Repr

inductive Holder where
  | frame (name : Nat)
  | tracked (site : Nat)
  | path (root : Nat) (field : Nat)
  | global
  | foreign
  deriving DecidableEq, Repr

inductive HeldObject where
  | site (site : Nat)
  | siteInterior (site : Nat) (field : Nat)
  | name (name : Nat)
  deriving DecidableEq, Repr

/-- The identity a record keeps alive. -/
def HeldObject.identity : HeldObject → Option Nat
  | .site s => some s
  | .siteInterior s _ => some s
  | .name _ => none

inductive HeldBy where
  | frame (name : Nat)
  | site (site : Nat)
  | path (root : Nat) (field : Nat)
  deriving DecidableEq, Repr

inductive Decision where
  | outlives
  | record (object : HeldObject) (by_ : HeldBy)
  | undecided
  deriving DecidableEq, Repr

def heldBy : Holder → Option HeldBy
  | .frame l => some (.frame l)
  | .tracked h => some (.site h)
  | .path r f => some (.path r f)
  | .global => none
  | .foreign => none

/-- Rule 12t's table with path holders and interiors (`retained_by::decide`). -/
def decide : Retained → Holder → Decision
  | .static, _ => .outlives
  | .other, _ => .undecided
  | _, .foreign => .undecided
  | _, .global => .undecided
  | .callerObject _, .frame _ => .outlives
  | .callerObject n, .tracked h => .record (.name n) (.site h)
  | .callerObject n, .path r f => .record (.name n) (.path r f)
  | .tracked s, .frame l => .record (.site s) (.frame l)
  | .tracked s, .tracked h => .record (.site s) (.site h)
  | .tracked s, .path r f => .record (.site s) (.path r f)
  | .interior s f, .frame l => .record (.siteInterior s f) (.frame l)
  | .interior s f, .tracked h => .record (.siteInterior s f) (.site h)
  | .interior s f, .path r g => .record (.siteInterior s f) (.path r g)

/-- The object a removal names (`removal_key`'s object side). -/
def heldObject : Retained → Option HeldObject
  | .callerObject n => some (.name n)
  | .tracked s => some (.site s)
  | .interior s f => some (.siteInterior s f)
  | .static => none
  | .other => none

/-- What a removal names (`retained_by::removal_key`): each side `none` when
undecided. -/
def removalKey (r : Retained) (h : Holder) : Option HeldObject × Option HeldBy :=
  (heldObject r, heldBy h)

/-- A recorded retention, poisoned or not. -/
structure Rec where
  object : HeldObject
  holder : HeldBy
  poisoned : Bool
  deriving DecidableEq, Repr

inductive Event where
  | retain (d : Decision)
  | release (site : Nat)
  | releaseHolder (site : Nat)
  | remove (key : Option HeldObject × Option HeldBy)
  | destroy (holder : HeldBy)
  deriving DecidableEq, Repr

abbrev Records := List Rec

def holds (rs : Records) (s : Nat) : Bool := rs.any fun r => r.object.identity == some s

/-- Whether a removal naming `o` may clear the status of `h`'s object
(`retained_by::may_name`): only distinct tracked identities are distinct. -/
def mayName : Option HeldObject → HeldObject → Bool
  | none, _ => true
  | some o, h =>
      match o.identity, h.identity with
      | some a, some b => a == b
      | _, _ => true

/-- The record a removal drops: exactly its own, and only unpoisoned. -/
def exact (k : Option HeldObject × Option HeldBy) (r : Rec) : Bool :=
  k.1 == some r.object && k.2 == some r.holder && !r.poisoned

def poisonIf (o : Option HeldObject) (r : Rec) : Rec :=
  if mayName o r.object then { r with poisoned := true } else r

def step (rs : Records) : Event → Option Records
  | .retain .outlives => some rs
  | .retain .undecided => none
  | .retain (.record o b) => some (⟨o, b, false⟩ :: rs)
  | .release s => if holds rs s then none else some (rs.filter fun r => r.holder != .site s)
  | .releaseHolder h => some (rs.filter fun r => r.holder != .site h)
  | .remove k => some ((rs.filter fun r => !exact k r).map (poisonIf k.1))
  | .destroy b => some (rs.filter fun r => r.holder != b)

def run (rs : Records) : List Event → Option Records
  | [] => some rs
  | e :: es => (step rs e).bind fun rs' => run rs' es

/-- A record whose holder outlives the frame. -/
def outlivesFrame (r : Rec) : Bool :=
  match r.holder with
  | .frame _ => false
  | .site _ => true
  | .path _ _ => true

def replay (es : List Event) : Bool :=
  match run [] es with
  | none => false
  | some rs => !(rs.any outlivesFrame)

/-- Whether an event ends every record of holder `b`: its destroy, or the
release of a tracked holder. -/
def ends : Event → HeldBy → Bool
  | .destroy b', b => b' == b
  | .releaseHolder s, b => b == .site s
  | .release s, b => b == .site s
  | _, _ => false

theorem run_append (rs : Records) (pre post : List Event) :
    run rs (pre ++ post) = (run rs pre).bind fun r => run r post := by
  induction pre generalizing rs with
  | nil => simp [run]
  | cons e es ih =>
      simp only [List.cons_append, run]
      cases step rs e with
      | none => rfl
      | some r => simp only [Option.bind_some]; exact ih r

/-- **An unresolved path never decides**: the body reads it as a holder it
cannot name, and the retention refuses wherever it stands. -/
theorem an_unresolved_path_never_decides (r : Retained) (hr : r ≠ .static)
    (pre post : List Event) :
    replay (pre ++ .retain (decide r .foreign) :: post) = false := by
  have hd : decide r .foreign = .undecided := by cases r <;> simp_all [decide]
  rw [hd]
  unfold replay
  rw [run_append]
  cases run [] pre with
  | none => rfl
  | some rs => simp [run, step]

/-- **A removal drops exactly its unpoisoned record, and poisons what it may
name** (the corrected step; it replaces `a_removal_drops_exactly_its_record`). -/
theorem a_removal_drops_exactly_its_unpoisoned_record (rs : Records)
    (k : Option HeldObject × Option HeldBy) :
    step rs (.remove k) = some ((rs.filter fun r => !exact k r).map (poisonIf k.1)) := rfl

/-- **An unmatched removal poisons what it may name** (it replaces
`an_unmatched_removal_keeps_every_record`, which read it as the identity). -/
theorem an_unmatched_removal_poisons_what_it_may_name (rs : Records)
    (k : Option HeldObject × Option HeldBy) (hnone : ∀ r ∈ rs, exact k r = false) :
    step rs (.remove k) = some (rs.map (poisonIf k.1)) := by
  simp only [step]
  congr 2
  rw [List.filter_eq_self]
  intro r hr
  simp [hnone r hr]

theorem poisonIf_object (o : Option HeldObject) (r : Rec) :
    (poisonIf o r).object = r.object ∧ (poisonIf o r).holder = r.holder := by
  unfold poisonIf; split <;> simp

theorem poisonIf_of_poisoned (o : Option HeldObject) (r : Rec) (h : r.poisoned = true) :
    poisonIf o r = r := by
  unfold poisonIf; split
  · cases r; simp_all
  · rfl

/-- **A matched removal ends an unpoisoned record**: after it no record of
that object and holder remains unpoisoned. -/
theorem a_matched_removal_ends_an_unpoisoned_record (rs rs' : Records) (o : HeldObject)
    (b : HeldBy) (hstep : step rs (.remove (some o, some b)) = some rs') :
    ∀ r ∈ rs', r.object = o → r.holder = b → r.poisoned = true := by
  intro r hr ho hb
  simp only [step, Option.some.injEq] at hstep
  subst hstep
  simp only [List.mem_map, List.mem_filter] at hr
  obtain ⟨a, ⟨_, hex⟩, hpa⟩ := hr
  have ⟨hobj, hhold⟩ := poisonIf_object (some o) a
  rw [hpa] at hobj hhold
  unfold poisonIf at hpa
  split at hpa
  · rw [← hpa]
  · rw [← hpa] at ho hb ⊢
    cases hp : a.poisoned with
    | true => rfl
    | false => simp [exact, ho, hb, hp] at hex

/-- One step keeps a poisoned record unless the step ends its holder. -/
theorem step_keeps_a_poisoned_record (rs rs' : Records) (e : Event) (r : Rec)
    (hmem : r ∈ rs) (hp : r.poisoned = true) (hstep : step rs e = some rs')
    (hend : ends e r.holder = false) : r ∈ rs' := by
  cases e with
  | retain d =>
      cases d <;> simp [step] at hstep
      · subst hstep; exact hmem
      · subst hstep; exact List.mem_cons_of_mem _ hmem
  | release s =>
      simp only [step] at hstep
      split at hstep
      · simp at hstep
      · simp at hstep; subst hstep
        simp only [ends, beq_eq_false_iff_ne, ne_eq] at hend
        simp [hmem, hend]
  | releaseHolder h =>
      simp [step] at hstep; subst hstep
      simp only [ends, beq_eq_false_iff_ne, ne_eq] at hend
      simp [hmem, hend]
  | remove k =>
      simp [step] at hstep; subst hstep
      refine List.mem_map.mpr ⟨r, List.mem_filter.mpr ⟨hmem, ?_⟩, poisonIf_of_poisoned _ _ hp⟩
      simp [exact, hp]
  | destroy b =>
      simp [step] at hstep; subst hstep
      simp only [ends, beq_eq_false_iff_ne, ne_eq] at hend
      simp only [List.mem_filter, bne_iff_ne, ne_eq]
      exact ⟨hmem, fun h => hend h.symm⟩

/-- **A poisoned record ends only by its loop's destroy**: through any events
none of which ends its holder it is still there (if the path does not refuse
first). -/
theorem a_poisoned_record_ends_only_by_its_loop_destroy (es : List Event) (r : Rec)
    (rs : Records) (hmem : r ∈ rs) (hp : r.poisoned = true)
    (hnone : ∀ e ∈ es, ends e r.holder = false) :
    ∀ rs', run rs es = some rs' → r ∈ rs' := by
  induction es generalizing rs with
  | nil => intro rs' h; simp [run] at h; subst h; exact hmem
  | cons e es ih =>
      intro rs' h
      simp only [run] at h
      cases hs : step rs e with
      | none => simp [hs] at h
      | some rs₁ =>
          rw [hs] at h
          simp only [Option.bind_some] at h
          have hk := step_keeps_a_poisoned_record rs rs₁ e r hmem hp hs
            (hnone e (List.mem_cons_self ..))
          exact ih rs₁ hk (fun e' he' => hnone e' (List.mem_cons_of_mem _ he')) rs' h

/-- A poisoned record at a holder that outlives the frame, with no later
destroy of that holder, is never `Safe`. -/
theorem a_poisoned_record_without_its_destroy_is_never_safe (pre post : List Event)
    (rs : Records) (hpre : run [] pre = some rs) (r : Rec) (hmem : r ∈ rs)
    (hp : r.poisoned = true) (hout : outlivesFrame r = true)
    (hnone : ∀ e ∈ post, ends e r.holder = false) :
    replay (pre ++ post) = false := by
  unfold replay
  rw [run_append, hpre]
  simp only [Option.bind_some]
  cases h : run rs post with
  | none => rfl
  | some rs' =>
      have := a_poisoned_record_ends_only_by_its_loop_destroy post r rs hmem hp hnone rs' h
      simp only [Bool.not_eq_false', List.any_eq_true]
      exact ⟨r, this, hout⟩

/-- **The old sequence no longer decides**: a caller's object registered at a
path, removed through another path, then through its own, is never `Safe`
without a destroy of its loop. -/
theorem the_old_sequence_no_longer_decides (n p f q g : Nat) (hne : (q, g) ≠ (p, f))
    (post : List Event) (hnone : ∀ e ∈ post, ends e (.path p f) = false) :
    replay ([.retain (decide (.callerObject n) (.path p f)),
             .remove (removalKey (.callerObject n) (.path q g)),
             .remove (removalKey (.callerObject n) (.path p f))] ++ post) = false := by
  have hne' : HeldBy.path q g ≠ HeldBy.path p f := by
    intro h; injection h with h1 h2; exact hne (by rw [h1, h2])
  have hpre : run [] [.retain (decide (.callerObject n) (.path p f)),
             .remove (removalKey (.callerObject n) (.path q g)),
             .remove (removalKey (.callerObject n) (.path p f))] =
      some [⟨.name n, .path p f, true⟩] := by
    simp [run, step, decide, removalKey, heldObject, heldBy, exact, poisonIf, mayName,
      HeldObject.identity, hne']
  exact a_poisoned_record_without_its_destroy_is_never_safe _ post _ hpre
    ⟨.name n, .path p f, true⟩ (List.mem_singleton_self _) rfl rfl hnone

/-- One step keeps a record of `(o, b)` (perhaps now poisoned) unless it is
that record's own removal or ends its holder. -/
theorem step_keeps_a_record (rs rs' : Records) (e : Event) (r : Rec) (hmem : r ∈ rs)
    (hstep : step rs e = some rs') (hend : ends e r.holder = false)
    (hnot : e ≠ .remove (some r.object, some r.holder)) :
    ∃ r' ∈ rs', r'.object = r.object ∧ r'.holder = r.holder := by
  cases e with
  | retain d =>
      cases d <;> simp [step] at hstep
      · subst hstep; exact ⟨r, hmem, rfl, rfl⟩
      · subst hstep; exact ⟨r, List.mem_cons_of_mem _ hmem, rfl, rfl⟩
  | release s =>
      simp only [step] at hstep
      split at hstep
      · simp at hstep
      · simp at hstep; subst hstep
        simp only [ends, beq_eq_false_iff_ne, ne_eq] at hend
        exact ⟨r, by simp [hmem, hend], rfl, rfl⟩
  | releaseHolder h =>
      simp [step] at hstep; subst hstep
      simp only [ends, beq_eq_false_iff_ne, ne_eq] at hend
      exact ⟨r, by simp [hmem, hend], rfl, rfl⟩
  | remove k =>
      simp [step] at hstep; subst hstep
      have hex : exact k r = false := by
        cases hk : exact k r with
        | false => rfl
        | true =>
            exfalso; apply hnot
            obtain ⟨k1, k2⟩ := k
            simp [exact] at hk
            obtain ⟨⟨h1, h2⟩, _⟩ := hk
            rw [h1, h2]
      refine ⟨poisonIf k.1 r, List.mem_map.mpr ⟨r, List.mem_filter.mpr ⟨hmem, ?_⟩, rfl⟩,
        poisonIf_object _ _⟩
      simp [hex]
  | destroy b =>
      simp [step] at hstep; subst hstep
      simp only [ends, beq_eq_false_iff_ne, ne_eq] at hend
      exact ⟨r, by simp only [List.mem_filter, bne_iff_ne, ne_eq]; exact ⟨hmem, fun h => hend h.symm⟩,
        rfl, rfl⟩

/-- **A path record ends only by its own removal or its loop's destroy**
(corrected from `a_path_record_ends_only_by_its_own_removal`): when no later
event removes exactly that record or destroys its holder, a record of that
object and path is still there at the exit. -/
theorem a_path_record_ends_only_by_its_own_removal_or_its_destroy (es : List Event)
    (o : HeldObject) (p f : Nat) (rs : Records)
    (hmem : ∃ r ∈ rs, r.object = o ∧ r.holder = .path p f)
    (hnone : ∀ e ∈ es, e ≠ .remove (some o, some (.path p f)) ∧ ends e (.path p f) = false) :
    ∀ rs', run rs es = some rs' → ∃ r ∈ rs', r.object = o ∧ r.holder = .path p f := by
  induction es generalizing rs with
  | nil => intro rs' h; simp [run] at h; subst h; exact hmem
  | cons e es ih =>
      intro rs' h
      simp only [run] at h
      cases hs : step rs e with
      | none => simp [hs] at h
      | some rs₁ =>
          rw [hs] at h
          simp only [Option.bind_some] at h
          obtain ⟨r, hr, ho, hb⟩ := hmem
          have ⟨hne, hend⟩ := hnone e (List.mem_cons_self ..)
          obtain ⟨r', hr', ho', hb'⟩ := step_keeps_a_record rs rs₁ e r hr hs
            (by rw [hb]; exact hend) (by rw [ho, hb]; exact hne)
          exact ih rs₁ ⟨r', hr', ho'.trans ho, hb'.trans hb⟩
            (fun e' he' => hnone e' (List.mem_cons_of_mem _ he')) rs' h

theorem a_path_record_without_its_removal_or_destroy_is_never_safe (n p f : Nat)
    (es : List Event)
    (hnone : ∀ e ∈ es, e ≠ .remove (some (.name n), some (.path p f)) ∧
      ends e (.path p f) = false) :
    replay (.retain (decide (.callerObject n) (.path p f)) :: es) = false := by
  unfold replay
  simp only [run, decide, step, Option.bind_some]
  cases h : run [⟨.name n, .path p f, false⟩] es with
  | none => rfl
  | some rs' =>
      obtain ⟨r, hr, _, hb⟩ := a_path_record_ends_only_by_its_own_removal_or_its_destroy es
        (.name n) p f [⟨.name n, .path p f, false⟩] ⟨_, List.mem_singleton_self _, rfl, rfl⟩
        hnone rs' h
      simp only [Bool.not_eq_false', List.any_eq_true]
      exact ⟨r, hr, by simp [outlivesFrame, hb]⟩

/-- **A destroy ends every record at its loop**, poisoned or not. -/
theorem a_destroy_ends_every_record_at_its_loop (rs : Records) (b : HeldBy) :
    ∃ rs', step rs (.destroy b) = some rs' ∧ ∀ r ∈ rs', r.holder ≠ b := by
  refine ⟨_, rfl, fun r hr => ?_⟩
  simp only [List.mem_filter, bne_iff_ne, ne_eq] at hr
  exact hr.2

theorem release_refused_iff (rs : Records) (s : Nat) :
    step rs (.release s) = none ↔ holds rs s = true := by
  simp only [step]
  split <;> simp_all

/-- **On a `Safe` path no release reaches a held identity** (its own record
or an interior's, poisoned or not): the holder outlived the object only where
the record ended first. -/
theorem a_safe_path_never_releases_a_held_identity (pre post : List Event) (s : Nat)
    (hsafe : replay (pre ++ .release s :: post) = true) (rs : Records)
    (hpre : run [] pre = some rs) : holds rs s = false := by
  unfold replay at hsafe
  rw [run_append, hpre] at hsafe
  simp only [Option.bind_some, run] at hsafe
  cases hh : holds rs s with
  | false => rfl
  | true =>
      have : step rs (.release s) = none := (release_refused_iff rs s).mpr hh
      rw [this] at hsafe
      simp at hsafe

/-- **At the exit of a `Safe` path no tracked or path holder still holds.** -/
theorem a_safe_path_leaves_no_outliving_holder_holding (es : List Event)
    (hsafe : replay es = true) :
    ∃ rs, run [] es = some rs ∧ ∀ r ∈ rs, outlivesFrame r = false := by
  unfold replay at hsafe
  cases h : run [] es with
  | none => simp [h] at hsafe
  | some rs =>
      refine ⟨rs, rfl, fun r hr => ?_⟩
      rw [h] at hsafe
      simp only [Bool.not_eq_true', List.any_eq_false] at hsafe
      cases ht : outlivesFrame r with
      | false => rfl
      | true => exact absurd ht (hsafe r hr)

/-! ## Shared vectors -/

inductive RClass where
  | static | caller | tracked | interior | other
  deriving DecidableEq, Repr

inductive HClass where
  | path | frame | tracked | global | foreign
  deriving DecidableEq, Repr

inductive Label where
  | outlives | record | undecided
  deriving DecidableEq, Repr

def RClass.sample : RClass → Retained
  | .static => .static
  | .caller => .callerObject 0
  | .tracked => .tracked 1
  | .interior => .interior 1 0
  | .other => .other

def HClass.sample : HClass → Holder
  | .path => .path 0 0
  | .frame => .frame 0
  | .tracked => .tracked 2
  | .global => .global
  | .foreign => .foreign

def label : Decision → Label
  | .outlives => .outlives
  | .record .. => .record
  | .undecided => .undecided

/-- `PATH_DECISION_VECTORS` of `retained_by.rs`, row by row. -/
def pathDecisionVectors : List (RClass × HClass × Label) := [
  (.static, .path, .outlives),
  (.caller, .path, .record),
  (.tracked, .path, .record),
  (.interior, .path, .record),
  (.other, .path, .undecided),
  (.interior, .frame, .record),
  (.interior, .tracked, .record),
  (.interior, .global, .undecided),
  (.interior, .foreign, .undecided)
]

theorem path_decision_vectors_hold :
    pathDecisionVectors.all (fun (r, h, l) => label (decide r.sample h.sample) == l) = true := by
  decide

def coll : Holder := .path 0 0
def config : Holder := .path 1 0
def caller : Retained := .callerObject 7
def interiorP : Retained := .interior 1 0

/-- `PATH_TRACE_VECTORS` of `retained_by.rs`, each path as
`retained_by::path_trace` builds it. -/
def pathTraceVectors : List (List Event × Bool) := [
  ([.retain (decide caller coll)], false),
  ([.retain (decide caller coll), .remove (removalKey caller coll)], true),
  ([.retain (decide caller coll), .remove (removalKey caller config)], false),
  ([.retain (decide caller coll), .remove (removalKey .other coll)], false),
  ([.remove (removalKey caller coll)], true),
  ([.retain (decide interiorP (.tracked 2)), .release 1, .releaseHolder 2], false),
  ([.retain (decide interiorP (.tracked 2)), .releaseHolder 2, .release 1], true),
  ([.retain (decide caller coll), .retain (decide (.callerObject 8) coll),
    .remove (removalKey caller coll)], false),
  -- RETENTION-UNMATCHED-REMOVAL-POISONS
  ([.retain (decide caller coll), .remove (removalKey caller config),
    .remove (removalKey caller coll)], false),
  ([.retain (decide interiorP (.tracked 2)), .remove (removalKey interiorP (.tracked 3)),
    .remove (removalKey interiorP (.tracked 2)), .release 1, .releaseHolder 2,
    .releaseHolder 3], false),
  ([.retain (decide interiorP (.tracked 2)), .remove (removalKey interiorP (.tracked 2)),
    .release 1, .releaseHolder 2], true),
  ([.retain (decide interiorP (.tracked 2)), .destroy (.site 2), .release 1], true),
  ([.retain (decide interiorP (.tracked 2)), .remove (removalKey interiorP (.tracked 3)),
    .destroy (.site 2), .release 1, .releaseHolder 3], true),
  ([.retain (decide caller coll), .remove (removalKey caller config),
    .destroy (.path 0 0)], true),
  ([.retain (decide caller coll), .remove (removalKey caller .foreign),
    .remove (removalKey caller coll)], false),
  ([.retain (decide caller coll), .remove (removalKey .other coll),
    .remove (removalKey caller coll)], false),
  ([.retain (decide interiorP (.tracked 2)), .remove (removalKey (.interior 4 0) (.tracked 3)),
    .remove (removalKey interiorP (.tracked 2)), .release 1], true)
]

theorem path_trace_vectors_hold :
    pathTraceVectors.all (fun (es, safe) => replay es == safe) = true := by
  decide

end QCore.RetentionPath
