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

/-! Finite scheduler cleanup model. No extraction, callback-body closure,
TLS API success, concurrency or deployment theorem is asserted. -/
namespace QCore.SchedulerLifetime
inductive Effect where
  | read (resource : Nat) | write (resource : Nat) | release (resource : Nat)
  deriving DecidableEq, Repr
inductive Step where
  | freeTls (resource : Nat) | clearTls
  | unlink (parent : Nat) | resetEvent (event : Nat)
  | callback (parent : Nat) (effects : List Effect)
  | deleteCoroutine (resource : Nat) | freeParent (resource : Nat)
  deriving DecidableEq, Repr
structure Thread where
  parent : Nat
  coroutine : Nat
  suspended : Bool
  borrowed : List Nat
  callback : Option (List Effect)
  deriving DecidableEq, Repr
structure State where
  worker : Nat
  currentWorker : Nat
  currentThread : Option Nat
  tlsParams : Option Nat
  selected : List Nat
  other : List Nat
  threads : List Thread
  live : List Nat
  callbackOwned : List Nat
  eventConnection : List Nat
  deriving DecidableEq, Repr

def effectResource : Effect → Nat
  | .read p | .write p | .release p => p

def callbackStep (protectedIds : List Nat) (live owned : List Nat) (effect : Effect) :
    Option (List Nat × List Nat) :=
  let p := effectResource effect
  if p ∈ live then
    match effect with
    | .read _ => some (live, owned)
    | .write _ => if p ∈ owned ∧ p ∉ protectedIds then some (live, owned) else none
    | .release _ => if p ∈ owned ∧ p ∉ protectedIds then
        some (live.filter (· != p), owned.filter (· != p)) else none
  else none

def callbackReplay (protectedIds : List Nat) : List Nat → List Nat → List Effect →
    Option (List Nat × List Nat)
  | live, owned, [] => some (live, owned)
  | live, owned, e :: es => do
    let (nextLive, nextOwned) ← callbackStep protectedIds live owned e
    callbackReplay protectedIds nextLive nextOwned es

def ownedIds (s : State) : List Nat := s.threads.flatMap (fun t => [t.parent, t.coroutine])
def borrowedIds (s : State) : List Nat := s.threads.flatMap Thread.borrowed

def admission (s : State) : Prop :=
  s.worker = s.currentWorker ∧
  s.threads.length ≤ 32 ∧ s.live.length ≤ 512 ∧
  s.selected.Nodup ∧ s.other.Nodup ∧
  (∀ p ∈ s.selected, p ∉ s.other) ∧
  (∀ p, (p ∈ s.selected ∨ p ∈ s.other) ↔ ∃ t ∈ s.threads, t.parent = p) ∧
  (ownedIds s).Nodup ∧
  (∀ t ∈ s.threads, t.suspended = true ∧ t.borrowed.length = 3 ∧
    t.parent ∈ s.live ∧ t.coroutine ∈ s.live ∧
    (∀ p ∈ t.borrowed, p ∈ s.live ∧ p ∉ ownedIds s) ∧
    (∀ es, t.callback = some es → es.length ≤ 64)) ∧
  (∀ p ∈ s.callbackOwned, p ∈ s.live ∧ p ∉ ownedIds s ∧
    p ∉ borrowedIds s ∧ s.tlsParams ≠ some p ∧ s.currentThread ≠ some p) ∧
  (∀ p, s.tlsParams = some p → p ∈ s.live ∧ p ∉ ownedIds s ∧ p ∉ borrowedIds s) ∧
  (∀ p, s.currentThread = some p → p ∈ s.live ∧ p ∉ s.selected ∧
    ∃ t ∈ s.threads, t.parent = p) ∧
  (∀ p ∈ s.eventConnection, p ∈ s.live ∧ ∃ t ∈ s.threads, t.borrowed[2]! = p)

/-- Ordered effects of one C destroy call, with both explicit callback branches. -/
def destroyTrace (params : Option Nat) (t : Thread) (event : Nat) : List Step :=
  (match params with | none => [] | some p => [.freeTls p, .clearTls]) ++
  [.unlink t.parent, .resetEvent event] ++
  (match t.callback with | none => [] | some es => [.callback t.parent es]) ++
  [.deleteCoroutine t.coroutine, .freeParent t.parent]

/-- Resource retirement is deletion, never a proof that a C free occurred. -/
def retire (ids live : List Nat) := live.filter (fun p => !(p ∈ ids))

/-- Full ordered cleanup replay; inputs are abstract allocation tokens. -/
def cleanup (protectedIds : List Nat) : List Nat → State → Option (State × List Step)
  | [], s => some (s, [])
  | parent :: rest, s => do
    let thread ← s.threads.find? (fun t => t.parent == parent)
    let event := thread.borrowed[2]!
    let live := match s.tlsParams with
      | none => s.live
      | some p => s.live.filter (· != p)
    let (live, callbackOwned) ← match thread.callback with
      | none => some (live, s.callbackOwned)
      | some effects => callbackReplay protectedIds live s.callbackOwned effects
    let next := { s with
      tlsParams := none
      selected := s.selected.filter (· != parent)
      threads := s.threads.filter (fun t => t.parent != parent)
      live := retire [thread.parent, thread.coroutine] live
      callbackOwned := callbackOwned
      eventConnection := if event ∈ s.eventConnection then s.eventConnection else event :: s.eventConnection }
    let (after, tail) ← cleanup protectedIds rest next
    pure (after, destroyTrace s.tlsParams thread event ++ tail)

/-- This mathematical decision is not extraction of the Rust implementation. -/
noncomputable def checkModel (before : State) (trace : List Step) : Option State := by
  classical
  exact if admission before ∧ trace.length ≤ 2560 then
    match cleanup (ownedIds before ++ borrowedIds before ++ before.tlsParams.toList)
        before.selected before with
    | none => none
    | some (after, expected) => if trace = expected then some after else none
  else none

theorem wrong_worker_refuses (before : State) (trace : List Step)
    (wrong : before.worker ≠ before.currentWorker) : checkModel before trace = none := by
  simp only [checkModel]
  split
  next h => exact False.elim (wrong h.1.1)
  next => rfl

theorem empty_cleanup_has_no_effects (s : State) (protectedIds : List Nat) :
    cleanup protectedIds [] s = some (s, []) := by rfl


theorem release_protected_refuses (protectedIds live owned : List Nat) (p : Nat)
    (inProtected : p ∈ protectedIds) :
    callbackStep protectedIds live owned (.release p) = none := by
  simp [callbackStep, effectResource, inProtected]

theorem write_unowned_refuses (protectedIds live owned : List Nat) (p : Nat)
    (unowned : p ∉ owned) : callbackStep protectedIds live owned (.write p) = none := by
  simp [callbackStep, effectResource, unowned]

theorem absent_lifetime_refuses (protectedIds live owned : List Nat) (effect : Effect)
    (dead : effectResource effect ∉ live) :
    callbackStep protectedIds live owned effect = none := by
  simp [callbackStep, dead]

theorem released_resource_is_absent (live : List Nat) (p : Nat) :
    p ∉ live.filter (· != p) := by simp

theorem retirement_preserves_borrow (ids live : List Nat) (p : Nat)
    (liveBorrow : p ∈ live) (separate : p ∉ ids) : p ∈ retire ids live := by
  simp [retire, liveBorrow, separate]

theorem retired_owner_cannot_remain (ids live : List Nat) (p : Nat)
    (owned : p ∈ ids) : p ∉ retire ids live := by simp [retire, owned]

theorem invalid_membership_forbids_admission (s : State) (p : Nat)
    (selected : p ∈ s.selected) (other : p ∈ s.other) : ¬ admission s := by
  intro h
  exact h.2.2.2.2.2.1 p selected other

theorem retiring_current_tls_forbids_admission (s : State) (p : Nat)
    (tls : s.currentThread = some p) (selected : p ∈ s.selected) : ¬ admission s := by
  intro h
  exact (h.2.2.2.2.2.2.2.2.2.2.2.1 p tls).2.1 selected

/-- TLS carries a parent role, never an arbitrary live allocation token. This
also holds for empty cleanup; emptiness does not repair a malformed role. -/
theorem non_parent_current_tls_forbids_admission (s : State) (p : Nat)
    (tls : s.currentThread = some p)
    (nonparent : ∀ t ∈ s.threads, t.parent ≠ p) : ¬ admission s := by
  intro h
  obtain ⟨t, member, equal⟩ := (h.2.2.2.2.2.2.2.2.2.2.2.1 p tls).2.2
  exact nonparent t member equal


theorem tls_absent_destroy_has_no_params_free_and_clear :
    destroyTrace none { parent := 1, coroutine := 2, suspended := true, borrowed := [3,4,5], callback := none } 5 =
    [.unlink 1, .resetEvent 5, .deleteCoroutine 2, .freeParent 1] := by rfl

theorem callback_is_explicit_before_retirement (t : Thread) (es : List Effect)
    (callback : t.callback = some es) (event : Nat) :
    destroyTrace none t event = [.unlink t.parent, .resetEvent event,
      .callback t.parent es, .deleteCoroutine t.coroutine, .freeParent t.parent] := by
  simp [destroyTrace, callback]
end QCore.SchedulerLifetime
