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

import QCore.VerdictSemigroup

/-! Card W48-RK-LOG-CALLER-FRAME (gateway rule 32): librdkafka's logger read
from its callers' frame.

Three parts, each a finite model, not a C refinement (trust boundary 28):

* The path split `qcore_contract::caller_frame` derives from the reviewed
  `logger-effects-v1` model: each event is in the frame, the frame boundary
  (a later consumer's take) or infeasible for a fresh op (a reply with a
  reply queue); each in-frame event names the premises covering its
  caller-visible effects. A path is proved when it names none, premised with
  exactly the ones it names, infeasible, or outside the frame. The model's
  ten paths derive three premises, each needed by some feasible path.
* The sequential enqueue: `rd_kafka_q_enq1` over a forward chain keeps each
  forward queue, links the fresh op into the last queue and drops the keeps
  innermost first. Every reference count is restored, no queue is destroyed
  while each forwarded queue holds a reference (its forward link's), and no
  object but the fresh op is linked or destroyed: a caller's tracked objects
  are untouched. The disabled-queue reply of a fresh op frees only the op
  and its string.
* The premises are not vacuous: an interleaved unforward-and-release makes
  the forward drop the last and destroys a queue, and a synchronous queue
  event callback can release a caller's object.
* Rule 32a, the self-reliance refusal: a caller-frame declaration serves only
  callers outside its frame. A call from a frame member's body to a member
  (itself included) is held, `Unknown` with the analyser category
  `incomplete-obligation-coverage`, so no member's row rests on the frame's
  own declarations; a caller outside the frame reads them unchanged, and the
  hold never moves a row towards `Safe`. -/
namespace QCore.RkLogCallerFrame

/-! ## The path split -/

inductive Event where
  | createOp | createText | forward | reply | transfer | take | callback
  | destroyText | destroyOp
  deriving DecidableEq, Repr

inductive Premise where
  /-- P-RK-PLUGIN-CALLBACK-CLOSURE (the log callback). -/
  | callbackClosure
  /-- P-RK-LOG-ENQUEUE-NO-EVENT-CALLBACK (a synchronous queue event callback). -/
  | noEventCallback
  /-- P-RK-LOG-ENQUEUE-FORWARD-NOT-LAST (a concurrent last release). -/
  | forwardNotLast
  deriving DecidableEq, Repr

inductive Frame where
  | inFrame | boundary | infeasible
  deriving DecidableEq, Repr

open Event Premise Frame

/-- The projection's frame standing of each event (`caller-frame.json`). -/
def frameOf : Event → Frame
  | reply => infeasible
  | take => boundary
  | _ => inFrame

/-- The premises an in-frame event's caller-visible obligations name: the log
callback, the IO-event callback raised by linking, and the forward release. -/
def premisesOf : Event → List Premise
  | callback => [callbackClosure]
  | transfer => [noEventCallback]
  | forward => [forwardNotLast]
  | _ => []

inductive Standing where
  | proved
  | premised (premises : List Premise)
  | infeasible
  | outside (pre : List Event)
  deriving DecidableEq, Repr

/-- The events before the frame boundary. -/
def inside (es : List Event) : List Event :=
  es.takeWhile (fun e => frameOf e != boundary)

/-- Each premise once, in first-use order. -/
def dedup : List Premise → List Premise
  | [] => []
  | p :: ps => if p ∈ dedup ps then dedup ps else p :: dedup ps

def needed (es : List Event) : List Premise :=
  dedup (es.flatMap premisesOf)

/-- The derived standing (the Rust `check`'s derivation). -/
def derive (es : List Event) : Standing :=
  if es.any (fun e => frameOf e == infeasible) then Standing.infeasible
  else if es.any (fun e => frameOf e == boundary) then Standing.outside (inside es)
  else match needed es with
    | [] => Standing.proved
    | ps => Standing.premised ps

theorem mem_dedup (p : Premise) (ps : List Premise) : p ∈ dedup ps ↔ p ∈ ps := by
  induction ps with
  | nil => simp [dedup]
  | cons q qs ih =>
    unfold dedup
    split
    · rename_i h
      constructor
      · intro hp; exact List.mem_cons_of_mem _ (ih.mp hp)
      · intro hp
        cases List.mem_cons.mp hp with
        | inl e => subst e; exact h
        | inr r => exact ih.mpr r
    · constructor
      · intro hp
        cases List.mem_cons.mp hp with
        | inl e => exact e ▸ List.mem_cons_self
        | inr r => exact List.mem_cons_of_mem _ (ih.mp r)
      · intro hp
        cases List.mem_cons.mp hp with
        | inl e => exact e ▸ List.mem_cons_self
        | inr r => exact List.mem_cons_of_mem _ (ih.mpr r)

/-- A path every premise of whose events is in `ps`'s derived set is covered. -/
theorem premised_covers_every_caller_event (es : List Event) (ps : List Premise)
    (h : derive es = Standing.premised ps) :
    ∀ e ∈ es, ∀ p ∈ premisesOf e, p ∈ ps := by
  intro e he p hp
  unfold derive at h
  split at h
  · cases h
  · split at h
    · cases h
    · have hn : p ∈ needed es := (mem_dedup p _).mpr (List.mem_flatMap.mpr ⟨e, he, hp⟩)
      revert h hn
      cases needed es with
      | nil => intro h; cases h
      | cons q qs => intro h hn; cases h; exact hn

/-- A proved path's events name no premise: nothing caller-visible is assumed. -/
theorem proved_path_needs_no_premise (es : List Event)
    (h : derive es = Standing.proved) : ∀ e ∈ es, premisesOf e = [] := by
  intro e he
  unfold derive at h
  split at h
  · cases h
  · split at h
    · cases h
    · cases hp : premisesOf e with
      | nil => rfl
      | cons p ps =>
        have hn : p ∈ needed es :=
          (mem_dedup p _).mpr (List.mem_flatMap.mpr ⟨e, he, by simp [hp]⟩)
        revert h hn
        cases needed es with
        | nil => intro _ hn; cases hn
        | cons q qs => intro h; cases h

/-- A path through a reply (impossible for a fresh op's empty reply queue)
is never proved or premised: it is infeasible. -/
theorem a_reply_path_is_infeasible (es : List Event) (h : reply ∈ es) :
    derive es = Standing.infeasible := by
  unfold derive
  have : es.any (fun e => frameOf e == infeasible) = true :=
    List.any_eq_true.mpr ⟨reply, h, by decide⟩
  simp [this]

/-- The model's ten paths (`logger-effects-v1`). -/
def filtered : List Event := []
def queueMissingTerminating : List Event := []
def callbackAbsent : List Event := []
def immediateCallback : List Event := [callback]
def localQueue : List Event := [createOp, createText, transfer]
def forwardedQueue : List Event := [createOp, createText, forward, transfer]
def disabledNoReply : List Event := [createOp, createText, destroyText, destroyOp]
def disabledReplyQueue : List Event := [createOp, createText, reply, transfer]
def forwardedDisabledNoReply : List Event :=
  [createOp, createText, forward, destroyText, destroyOp]
def consumerCallbackCleanup : List Event :=
  [createOp, createText, transfer, take, callback, destroyText, destroyOp]

theorem model_paths_derive :
    derive filtered = Standing.proved ∧
    derive queueMissingTerminating = Standing.proved ∧
    derive callbackAbsent = Standing.proved ∧
    derive disabledNoReply = Standing.proved ∧
    derive immediateCallback = Standing.premised [callbackClosure] ∧
    derive localQueue = Standing.premised [noEventCallback] ∧
    derive forwardedQueue = Standing.premised [forwardNotLast, noEventCallback] ∧
    derive forwardedDisabledNoReply = Standing.premised [forwardNotLast] ∧
    derive disabledReplyQueue = Standing.infeasible ∧
    derive consumerCallbackCleanup = Standing.outside localQueue := by
  decide

/-- The consumer path's in-frame prefix is exactly the local enqueue: its
callback and cleanup run after the call has returned. -/
theorem the_consumer_path_leaves_the_frame_after_the_local_enqueue :
    inside consumerCallbackCleanup = localQueue := by decide

/-- Every one of the three premises is needed by some feasible in-frame path,
so a declaration stating fewer covers fewer effects than the body has. -/
theorem each_premise_is_needed :
    (∃ ps, derive immediateCallback = Standing.premised ps ∧ callbackClosure ∈ ps) ∧
    (∃ ps, derive localQueue = Standing.premised ps ∧ noEventCallback ∈ ps) ∧
    (∃ ps, derive forwardedDisabledNoReply = Standing.premised ps ∧ forwardNotLast ∈ ps) :=
  ⟨⟨_, rfl, by decide⟩, ⟨_, rfl, by decide⟩, ⟨_, rfl, by decide⟩⟩

/-! ## The sequential enqueue -/

/-- A state: reference counts, the queue each object is linked into, and
destruction. Objects and queues are numbered. -/
structure St where
  refs : Nat → Nat
  linked : Nat → Option Nat
  destroyed : Nat → Bool

inductive Eff where
  | keep (q : Nat)
  | drop (q : Nat)
  | link (o q : Nat)
  | free (o : Nat)

def step (s : St) : Eff → St
  | .keep q => { s with refs := fun x => if x = q then s.refs x + 1 else s.refs x }
  | .drop q =>
    { s with
      refs := fun x => if x = q then s.refs x - 1 else s.refs x
      destroyed := fun x => if x = q ∧ s.refs x - 1 = 0 then true else s.destroyed x }
  | .link o q => { s with linked := fun x => if x = o then some q else s.linked x }
  | .free o => { s with destroyed := fun x => if x = o then true else s.destroyed x }

def run (s : St) : List Eff → St
  | [] => s
  | e :: es => run (step s e) es

theorem run_append (s : St) (a b : List Eff) : run s (a ++ b) = run (run s a) b := by
  induction a generalizing s with
  | nil => rfl
  | cons e es ih => exact ih (step s e)

private theorem step_drop_refs (s : St) (q : Nat) :
    (step s (.drop q)).refs = fun x => if x = q then s.refs x - 1 else s.refs x := rfl

private theorem step_drop_destroyed (s : St) (q : Nat) :
    (step s (.drop q)).destroyed =
      fun x => if x = q ∧ s.refs x - 1 = 0 then true else s.destroyed x := rfl

/-- `rd_kafka_q_enq1` of the fresh op `o` over a forward chain: keep the next
queue, enqueue there, drop it; the last queue links the op. -/
def enq (o : Nat) : List Nat → List Eff
  | [] => []
  | [q] => [.link o q]
  | _ :: r :: rest => .keep r :: enq o (r :: rest) ++ [.drop r]

/-- The sequential enqueue restores every reference count and destroys
nothing, while every forwarded queue holds a reference (its forward link's). -/
theorem enq_restores_and_destroys_nothing (o : Nat) (chain : List Nat) :
    ∀ s : St, (∀ q ∈ chain.tail, 1 ≤ s.refs q) →
      (run s (enq o chain)).refs = s.refs ∧
      (run s (enq o chain)).destroyed = s.destroyed := by
  induction chain with
  | nil => intro s _; exact ⟨rfl, rfl⟩
  | cons q rest ih =>
    intro s hs
    cases rest with
    | nil => exact ⟨rfl, rfl⟩
    | cons r rest' =>
      have hr1 : 1 ≤ s.refs r := hs r List.mem_cons_self
      have h1 : ∀ x ∈ (r :: rest').tail, 1 ≤ (step s (.keep r)).refs x := by
        intro x hx
        have := hs x (List.mem_cons_of_mem _ hx)
        simp only [step]
        split <;> omega
      have ⟨hr, hd⟩ := ih (step s (.keep r)) h1
      have e : run s (enq o (q :: r :: rest')) =
          step (run (step s (.keep r)) (enq o (r :: rest'))) (.drop r) := by
        simp only [enq]; rw [run_append]; rfl
      rw [e, step_drop_refs, step_drop_destroyed, hr, hd]
      constructor
      · funext x
        simp only [step]
        split <;> simp_all
      · funext x
        by_cases hx : x = r
        · subst hx
          have hne : s.refs x ≠ 0 := by omega
          simp [step, hne]
        · simp [step, hx]

/-- The enqueue links the fresh op and nothing else. -/
theorem enq_links_only_the_fresh_op (o : Nat) (chain : List Nat) :
    ∀ s : St, ∀ x, x ≠ o → (run s (enq o chain)).linked x = s.linked x := by
  induction chain with
  | nil => intro s x _; rfl
  | cons q rest ih =>
    intro s x hx
    cases rest with
    | nil => simp [enq, run, step, hx]
    | cons r rest' =>
      simp only [enq]
      rw [run_append]
      simp only [run, step]
      exact ih _ x hx

/-- A caller's tracked object (anything but the fresh op) keeps its reference
count, its link and its liveness across the sequential enqueue. -/
theorem a_tracked_object_is_untouched (o : Nat) (chain : List Nat) (s : St)
    (hs : ∀ q ∈ chain.tail, 1 ≤ s.refs q) (x : Nat) (hx : x ≠ o) :
    (run s (enq o chain)).refs x = s.refs x ∧
    (run s (enq o chain)).linked x = s.linked x ∧
    (run s (enq o chain)).destroyed x = s.destroyed x := by
  have ⟨hr, hd⟩ := enq_restores_and_destroys_nothing o chain s hs
  exact ⟨by rw [hr], enq_links_only_the_fresh_op o chain s x hx, by rw [hd]⟩

/-- A disabled queue's reply to a fresh op (no reply queue) destroys it: its
string `t`, then the op `o`. Nothing else changes. -/
def disabledReply (o t : Nat) : List Eff := [.free t, .free o]

theorem the_disabled_reply_frees_only_the_fresh_objects (o t : Nat) (s : St)
    (x : Nat) (hx : x ≠ o) (ht : x ≠ t) :
    (run s (disabledReply o t)).refs x = s.refs x ∧
    (run s (disabledReply o t)).linked x = s.linked x ∧
    (run s (disabledReply o t)).destroyed x = s.destroyed x := by
  simp [disabledReply, run, step, hx, ht]

/-- The caller-frame admission (rule 32): on every feasible in-frame path
that makes an op — the sequential enqueue over any forward chain whose
forwarded queues each hold a reference, and the disabled-queue reply — every
object other than the call's fresh op `o` and string `t` keeps its reference
count, link and liveness, so a caller reads the call as the identity on its
own custody. The log callback and the queue event callback are not in these
effect lists: their premises cover them. -/
theorem caller_frame_admission (o t : Nat) (chain : List Nat) (s : St)
    (hs : ∀ q ∈ chain.tail, 1 ≤ s.refs q) (x : Nat) (hx : x ≠ o) (ht : x ≠ t) :
    ((run s (enq o chain)).refs x = s.refs x ∧
      (run s (enq o chain)).linked x = s.linked x ∧
      (run s (enq o chain)).destroyed x = s.destroyed x) ∧
    ((run s (disabledReply o t)).refs x = s.refs x ∧
      (run s (disabledReply o t)).linked x = s.linked x ∧
      (run s (disabledReply o t)).destroyed x = s.destroyed x) :=
  ⟨a_tracked_object_is_untouched o chain s hs x hx,
    the_disabled_reply_frees_only_the_fresh_objects o t s x hx ht⟩

/-! ## The premises are not vacuous -/

def zero : St := { refs := fun _ => 0, linked := fun _ => none, destroyed := fun _ => false }

/-- Queue 1 held only by its forward link (one reference). -/
def oneLink : St := { zero with refs := fun x => if x = 1 then 1 else 0 }

/-- Another thread unforwards queue 1 and releases its link's reference between
the enqueue's keep and drop: the enqueue's drop is then the last and destroys
the queue inside the caller's call. P-RK-LOG-ENQUEUE-FORWARD-NOT-LAST excludes
exactly this. -/
theorem an_interleaved_release_makes_the_forward_drop_last :
    (run oneLink [.keep 1, .drop 1, .drop 1]).destroyed 1 = true := by decide

/-- Without it the same chain destroys nothing. -/
theorem without_the_interleaving_nothing_is_destroyed :
    (run oneLink (enq 9 [0, 1])).destroyed 1 = false := by decide

/-- A queue event callback runs arbitrary code synchronously at the link: one
that releases a caller's object 5 changes the caller's frame.
P-RK-LOG-ENQUEUE-NO-EVENT-CALLBACK excludes exactly this. -/
theorem an_event_callback_can_release_a_tracked_object :
    (run zero ([.link 9 0] ++ [.free 5])).destroyed 5 = true ∧
    (run zero (enq 9 [0])).destroyed 5 = false := by decide

/-! ## Rule 32a: the declaration serves only callers outside the frame

The gateway's frame registry is the reviewed projection's `declarations`
(`qcore_contract::caller_frame::FRAMES`). A row is the fold of its links; a
call link names its callee and the value the callee's declaration would
contribute. Inside a member's body, a call to a member is held. That the
gateway holds exactly these call sites is a Rust boundary, pinned by
`crates/qcore-c-gateway/tests/caller_frame_hold.rs` (trust boundary 28). -/

section SelfReliance

open QCore.VerdictSemigroup
open QCore.ReasonCategory

/-- One link of a row: a call to a declared callee contributing `applied`
when its declaration is read, or any other link. -/
inductive Item where
  | call (callee : String) (applied : V)
  | other (v : V)
  deriving DecidableEq, Repr

/-- The value of a call held inside its own caller frame. -/
def frameHeld : V := (Link.held .incompleteObligationCoverage).value

/-- Whether `caller`'s body calling `callee` stays inside one frame. -/
def withinFrame (members : List String) (caller callee : String) : Prop :=
  caller ∈ members ∧ callee ∈ members

instance (members : List String) (caller callee : String) :
    Decidable (withinFrame members caller callee) := by
  unfold withinFrame; infer_instance

/-- The value one link contributes to `caller`'s row. -/
def Item.value (members : List String) (caller : String) : Item → V
  | .call callee v => if withinFrame members caller callee then frameHeld else v
  | .other v => v

/-- `caller`'s row under the frame `members`. -/
def row (members : List String) (caller : String) (items : List Item) : V :=
  fold (items.map (Item.value members caller))

/-- The held value is `Unknown`, never `Pending`: the hold is the tool's
decision, not a named human act. -/
theorem a_frame_hold_is_unknown : frameHeld = .unknown := by
  decide

/-- A member's call to a member never reads a declaration: what the
declaration would contribute never reaches the row. -/
theorem a_member_never_reads_a_member_declaration (members : List String)
    (caller callee : String) (hc : caller ∈ members) (hd : callee ∈ members) (v : V) :
    Item.value members caller (.call callee v) = frameHeld := by
  simp [Item.value, withinFrame, hc, hd]

/-- So a member's row with a call to a member is neither `Safe` nor
`Pending`, whatever the member declarations claim. -/
theorem a_held_member_row_is_never_safe (members : List String) (caller callee : String)
    (hc : caller ∈ members) (hd : callee ∈ members) (items : List Item) (v : V)
    (member : Item.call callee v ∈ items) :
    2 ≤ (row members caller items).rank := by
  have link : Item.value members caller (.call callee v) ∈
      items.map (Item.value members caller) := List.mem_map_of_mem member
  have below := every_link_is_below_the_fold link
  rw [a_member_never_reads_a_member_declaration members caller callee hc hd,
    a_frame_hold_is_unknown] at below
  exact below

/-- A caller outside the frame reads the declaration exactly as declared. -/
theorem an_outside_caller_reads_the_declaration (members : List String)
    (caller callee : String) (outside : caller ∉ members) (v : V) :
    Item.value members caller (.call callee v) = v := by
  simp [Item.value, withinFrame, outside]

/-- The hold never certifies: a row `Safe` or `Pending` under the frame has
exactly the value it would have with no frame at all. -/
theorem the_hold_never_certifies (members : List String) (caller : String)
    (items : List Item) (low : (row members caller items).rank ≤ 1) :
    row members caller items = row [] caller items := by
  have values : ∀ i ∈ items, Item.value members caller i = Item.value [] caller i := by
    intro i member
    cases i with
    | other v => rfl
    | call callee v =>
      by_cases h : withinFrame members caller callee
      · exfalso
        have link : Item.value members caller (.call callee v) ∈
            items.map (Item.value members caller) := List.mem_map_of_mem member
        have below := every_link_is_below_the_fold link
        simp only [Item.value, h, ite_true, a_frame_hold_is_unknown] at below
        unfold row at low
        have two : V.unknown.rank = 2 := rfl
        omega
      · have h0 : ¬withinFrame [] caller callee := fun w => by
          simp [withinFrame] at w
        simp only [Item.value, h, h0, ite_false]
  unfold row
  congr 1
  exact List.map_congr_left values

/-- The logger frame: the halted regression. `rd_kafka_log0`'s row, whose
only other link is `Safe`, read `rd_kafka_log_buf`'s caller-frame declaration
(`Safe`) and was `Safe`; under the hold it is `Unknown`, while a caller
outside the frame still reads `rd_kafka_log0`'s declaration. -/
def logger : List String := ["rd_kafka_log_buf", "rd_kafka_log0"]

theorem the_logger_rows_are_decided_by_their_bodies :
    row [] "rd_kafka_log0" [.other .safe, .call "rd_kafka_log_buf" .safe] = .safe ∧
    row logger "rd_kafka_log0" [.other .safe, .call "rd_kafka_log_buf" .safe] = .unknown ∧
    row logger "rd_kafka_mock_cgrp_classic_member_active"
      [.other .safe, .call "rd_kafka_log0" .safe] = .safe := by
  decide

end SelfReliance

end QCore.RkLogCallerFrame
