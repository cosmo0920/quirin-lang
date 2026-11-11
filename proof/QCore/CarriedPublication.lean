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
# A publication carries its owner's field-linked children (card MONKEY-VHOST-PROPERTY-CUSTODY)

The abstract rule implemented by `crates/qcore-c-gateway/src/carried_publication.rs`
and its hooks in `legacy_container_publication.rs`, gateway rule 12d.

Rule 10d (`QCore.ContainerPublication`) discharges a body-acquired object
linked into a caller's container. An object that owned something through a
field link (`ha->name = mk_string_dup(v)`) was published with its children
withdrawn and the call refused, because an unlink could not hand them back.
The Monkey `mk_vhost_set_property` `Name` branch was `Unknown` for that reason
alone.

The rule has two steps:

* `publishStep` decides each child the publication reaches through a field
  link: a child that is exactly live, linked on every path, with no
  unresolved effect, not itself published and owning nothing of its own is
  carried (it escapes with its owner); every other child, an untracked one
  included, refuses, and rule 10d's withdrawal stands.
* `unlinkStep` decides each child an unlink of the published owner reaches: a
  must-linked, exactly escaped child is handed back; a child with an escaped
  bit that is partial or of mixed status refuses the unlink; any other child
  was never carried and is left as it is.

The sequence model below follows one owner and one linked child through a
straight-line body: a carried child comes back with its owner on an unlink,
and releasing the owner never discharges it (P-B08-SHALLOW), so the custody
the container held is owed again.

## What this is not

A model of the decision. That the flow's status bits, partial links,
unresolved effects and published set are what the Rust reads, the lexical
reading of entries and containers, and the correspondence with the compiled
Rust are review premises (trust boundary 28), pinned by the shared-vector
tests in `crates/qcore-c-gateway/tests/unit/carried_publication.rs` and the
fixtures in `crates/qcore-c-gateway/tests/monkey_vhost_property_custody.rs`.
It is not a proof that Monkey uses its lists correctly.
-/

namespace QCore.CarriedPublication

/-- A child's joined status bits (the flow's `LIVE`, `RELEASED`, `ESCAPED`,
`CONTAINED`). -/
structure Bits where
  live : Bool
  released : Bool
  escaped : Bool
  contained : Bool
  deriving DecidableEq, Repr

/-- Exactly live: the one status a carry accepts. -/
def Bits.exactlyLive (b : Bits) : Bool :=
  b.live && !b.released && !b.escaped && !b.contained

/-- Exactly escaped: the one status an unlink hands back. -/
def Bits.exactlyEscaped (b : Bits) : Bool :=
  !b.live && !b.released && b.escaped && !b.contained

/-- What the flow holds about one child an owner links. -/
structure ChildFacts where
  partialLink : Bool
  status : Option Bits
  unresolved : Bool
  published : Bool
  owns : Bool
  deriving DecidableEq, Repr

/-- What a publication of the owner does to one child. -/
inductive PublishStep where
  | carry
  | refuse
  deriving DecidableEq, Repr

/-- The publish step (`carried_publication::publish_step`). -/
def publishStep (c : ChildFacts) : PublishStep :=
  match c.status with
  | some b =>
    if b.exactlyLive && !c.partialLink && !c.unresolved && !c.published && !c.owns then
      .carry
    else
      .refuse
  | none => .refuse

/-- What an unlink of the published owner does to one child. -/
inductive UnlinkStep where
  | keep
  | restore
  | refuse
  deriving DecidableEq, Repr

/-- The unlink step (`carried_publication::unlink_step`). -/
def unlinkStep (partialLink : Bool) : Option Bits → UnlinkStep
  | some b =>
    if b.exactlyEscaped && !partialLink then .restore
    else if b.escaped then .refuse
    else .keep
  | none => .keep

/-! ## The decision tables -/

/-- A child is carried exactly when it is exactly live and unencumbered. -/
theorem carry_iff (c : ChildFacts) :
    publishStep c = .carry ↔
      (∃ b, c.status = some b ∧ b.exactlyLive = true) ∧ c.partialLink = false ∧
        c.unresolved = false ∧ c.published = false ∧ c.owns = false := by
  rcases c with ⟨p, s, u, q, o⟩
  cases s with
  | none => simp [publishStep]
  | some b =>
    cases p <;> cases u <;> cases q <;> cases o <;> cases h : b.exactlyLive <;>
      simp [publishStep, h]

/-- An untracked child is never carried: no rule says whose it is. -/
theorem an_untracked_child_refuses (p u q o : Bool) :
    publishStep ⟨p, none, u, q, o⟩ = .refuse := rfl

/-- Each encumbrance alone keeps the withdrawal. -/
theorem a_partial_link_refuses (b : Bits) (u q o : Bool) :
    publishStep ⟨true, some b, u, q, o⟩ = .refuse := by
  simp [publishStep]

theorem an_unresolved_child_refuses (b : Bits) (p q o : Bool) :
    publishStep ⟨p, some b, true, q, o⟩ = .refuse := by
  simp [publishStep]

theorem a_published_child_refuses (b : Bits) (p u o : Bool) :
    publishStep ⟨p, some b, u, true, o⟩ = .refuse := by
  simp [publishStep]

theorem a_child_that_owns_refuses (b : Bits) (p u q : Bool) :
    publishStep ⟨p, some b, u, q, true⟩ = .refuse := by
  simp [publishStep]

/-- An unlink hands back exactly a must-linked, exactly escaped child. -/
theorem restore_iff (p : Bool) (s : Option Bits) :
    unlinkStep p s = .restore ↔ p = false ∧ ∃ b, s = some b ∧ b.exactlyEscaped = true := by
  cases s with
  | none => simp [unlinkStep]
  | some b =>
    cases p <;> cases h : b.exactlyEscaped <;> cases e : b.escaped <;>
      simp [unlinkStep, h, e]

/-- A child that may have been carried on some paths only refuses the unlink. -/
theorem a_partially_carried_child_refuses (b : Bits) (h : b.escaped = true) :
    unlinkStep true (some b) = .refuse := by
  simp [unlinkStep, h]

/-- A child that never escaped is left as it is. -/
theorem a_never_escaped_child_is_kept (p : Bool) (b : Bits) (h : b.escaped = false) :
    unlinkStep p (some b) = .keep := by
  have : b.exactlyEscaped = false := by simp [Bits.exactlyEscaped, h]
  simp [unlinkStep, this, h]

/-- `(partial, tracked, live, released, escaped, contained, unresolved,
published, owns)` and the step, shared with `PUBLISH_VECTORS`. -/
abbrev PublishVector :=
  Bool × Bool × Bool × Bool × Bool × Bool × Bool × Bool × Bool × PublishStep

def factsOf : PublishVector → ChildFacts
  | (p, t, l, r, e, c, u, q, o, _) =>
    ⟨p, if t then some ⟨l, r, e, c⟩ else none, u, q, o⟩

def publishVectors : List PublishVector := [
  (false, false, false, false, false, false, false, false, false, .refuse),
  (true, false, false, false, false, false, true, true, true, .refuse),
  (false, true, true, false, false, false, false, false, false, .carry),
  (true, true, true, false, false, false, false, false, false, .refuse),
  (false, true, true, false, false, false, true, false, false, .refuse),
  (false, true, true, false, false, false, false, true, false, .refuse),
  (false, true, true, false, false, false, false, false, true, .refuse),
  (false, true, true, true, false, false, false, false, false, .refuse),
  (false, true, true, false, true, false, false, false, false, .refuse),
  (false, true, true, false, false, true, false, false, false, .refuse),
  (false, true, false, true, false, false, false, false, false, .refuse),
  (false, true, false, false, true, false, false, false, false, .refuse),
  (false, true, false, false, false, true, false, false, false, .refuse),
  (false, true, false, false, false, false, false, false, false, .refuse)
]

theorem publish_vectors_hold :
    publishVectors.all (fun v => publishStep (factsOf v) == v.2.2.2.2.2.2.2.2.2) = true := by
  decide

/-- `(partial, tracked, live, released, escaped)` and the step, shared with
`UNLINK_VECTORS`. -/
abbrev UnlinkVector := Bool × Bool × Bool × Bool × Bool × UnlinkStep

def statusOf : UnlinkVector → Option Bits
  | (_, t, l, r, e, _) => if t then some ⟨l, r, e, false⟩ else none

def unlinkVectors : List UnlinkVector := [
  (false, false, false, false, false, .keep),
  (true, false, false, false, false, .keep),
  (false, true, false, false, true, .restore),
  (true, true, false, false, true, .refuse),
  (false, true, true, false, true, .refuse),
  (false, true, false, true, true, .refuse),
  (false, true, true, false, false, .keep),
  (true, true, true, false, false, .keep),
  (false, true, false, true, false, .keep)
]

theorem unlink_vectors_hold :
    unlinkVectors.all (fun v => unlinkStep v.1 (statusOf v) == v.2.2.2.2.2) = true := by
  decide

/-! ## One owner and one linked child through a body -/

/-- The custody of one tracked identity. -/
inductive Custody where
  | live
  | escaped
  | released
  deriving DecidableEq, Repr

/-- Where a publication links the owner. -/
inductive Container where
  | callerFormal
  | other
  deriving DecidableEq, Repr

/-- The owner, its field-linked child, and whether the owner is published. -/
structure World where
  owner : Custody
  child : Custody
  published : Bool
  deriving DecidableEq, Repr

/-- One operation. `publish c clean`: `clean` is the child's carry premise
(`publishStep … = .carry`). -/
inductive Op where
  | publish (container : Container) (clean : Bool)
  | unpublish
  | releaseOwner
  | releaseChild
  deriving DecidableEq, Repr

inductive Outcome where
  | ok (w : World)
  | refused
  | violation
  deriving DecidableEq, Repr

/-- The rule, one operation at a time. A release does not cascade
(P-B08-SHALLOW). -/
def step (w : World) : Op → Outcome
  | .publish .callerFormal true =>
    if w.owner = .live ∧ w.child = .live ∧ w.published = false then
      .ok { owner := .escaped, child := .escaped, published := true }
    else .refused
  | .publish _ _ => .refused
  | .unpublish =>
    if w.published = true ∧ w.owner = .escaped ∧ w.child = .escaped then
      .ok { owner := .live, child := .live, published := false }
    else .refused
  | .releaseOwner =>
    if w.owner = .live then .ok { w with owner := .released } else .violation
  | .releaseChild =>
    if w.child = .live then .ok { w with child := .released } else .violation

/-- Rule 10d before this card: a publication of an owner that links a child
is withdrawn. -/
def stepWithdrawn (w : World) : Op → Outcome
  | .publish _ _ => .refused
  | op => step w op

def run (step : World → Op → Outcome) : Outcome → List Op → Outcome
  | outcome, [] => outcome
  | .ok w, op :: ops => run step (step w op) ops
  | .refused, _ :: _ => .refused
  | .violation, _ :: _ => .violation

inductive Verdict where
  | safe
  | provisionalUnsafe
  | unknown
  deriving DecidableEq, Repr

/-- An exit: an identity still live is a leak. -/
def verdict : Outcome → Verdict
  | .ok w => if w.owner = .live ∨ w.child = .live then .provisionalUnsafe else .safe
  | .refused => .unknown
  | .violation => .provisionalUnsafe

/-- `ha = alloc; ha->name = dup(v);`: both live, nothing published. -/
def linked : World := { owner := .live, child := .live, published := false }

theorem run_refused (s : World → Op → Outcome) (ops : List Op) :
    run s .refused ops = .refused := by
  cases ops <;> rfl

theorem run_violation (s : World → Op → Outcome) (ops : List Op) :
    run s .violation ops = .violation := by
  cases ops <;> rfl

/-- The `Name` branch: the owner and its carried child go into the caller's
container, and the exit owes nothing. -/
theorem the_name_branch_is_safe :
    verdict (run step (.ok linked) [.publish .callerFormal true]) = .safe := by
  decide

/-- Non-vacuity: rule 10d's withdrawal left the same body `Unknown`. -/
theorem the_withdrawal_refused_the_name_branch :
    verdict (run stepWithdrawn (.ok linked) [.publish .callerFormal true]) = .unknown := by
  decide

/-- A child that fails the carry premise keeps the withdrawal, whatever
follows. -/
theorem an_encumbered_child_keeps_the_refusal (c : Container) (ops : List Op) :
    verdict (run step (.ok linked) (.publish c false :: ops)) = .unknown := by
  cases c <;> simp [run, step, run_refused, verdict]

/-- A container the function does not show outlives it still refuses. -/
theorem another_container_refuses (clean : Bool) (ops : List Op) :
    verdict (run step (.ok linked) (.publish .other clean :: ops)) = .unknown := by
  simp [run, step, run_refused, verdict]

/-- An unlink hands back the owner and the child it carried. -/
theorem an_unlink_hands_back_both :
    run step (.ok linked) [.publish .callerFormal true, .unpublish] = .ok linked := by
  decide

/-- After the unlink, releasing the owner alone leaves the child owed
(P-B08-SHALLOW): never `Safe`. -/
theorem releasing_the_owner_after_the_unlink_leaks_the_child :
    verdict (run step (.ok linked)
      [.publish .callerFormal true, .unpublish, .releaseOwner]) = .provisionalUnsafe := by
  decide

/-- Handing both back and releasing both is `Safe`. -/
theorem a_full_teardown_after_the_unlink_is_safe :
    verdict (run step (.ok linked)
      [.publish .callerFormal true, .unpublish, .releaseChild, .releaseOwner]) = .safe := by
  decide

/-- Releasing a carried child while its owner is published is a violation
candidate, whatever follows. -/
theorem releasing_a_carried_child_is_a_violation (ops : List Op) :
    verdict (run step (.ok linked)
      (.publish .callerFormal true :: .releaseChild :: ops)) = .provisionalUnsafe := by
  simp [run, step, linked, run_violation, verdict]

/-- So is releasing the published owner. -/
theorem releasing_the_published_owner_is_a_violation (ops : List Op) :
    verdict (run step (.ok linked)
      (.publish .callerFormal true :: .releaseOwner :: ops)) = .provisionalUnsafe := by
  simp [run, step, linked, run_violation, verdict]

/-- An unlink of an owner the body did not publish refuses. -/
theorem an_unlink_without_a_publication_refuses (ops : List Op) :
    verdict (run step (.ok linked) (.unpublish :: ops)) = .unknown := by
  simp [run, step, linked, run_refused, verdict]

/-- A second publication refuses. -/
theorem a_second_publication_refuses (c : Container) (clean : Bool) (ops : List Op) :
    verdict (run step (.ok linked)
      (.publish .callerFormal true :: .publish c clean :: ops)) = .unknown := by
  cases c <;> cases clean <;> simp [run, step, linked, run_refused, verdict]

/-- Nothing certifies after a refusal or a violation. -/
theorem no_step_certifies_after_a_refusal (s : World → Op → Outcome) (ops : List Op) :
    verdict (run s .refused ops) ≠ .safe := by
  rw [run_refused]
  decide

theorem no_step_certifies_after_a_violation (s : World → Op → Outcome) (ops : List Op) :
    verdict (run s .violation ops) ≠ .safe := by
  rw [run_violation]
  decide

/-- The carry only replaces a refusal: wherever the pre-card rule reached a
verdict without refusing, this rule reaches the same one. -/
theorem the_carry_only_replaces_a_refusal (w : World) (op : Op)
    (h : stepWithdrawn w op ≠ .refused) : step w op = stepWithdrawn w op := by
  cases op with
  | publish c clean => simp [stepWithdrawn] at h
  | unpublish => rfl
  | releaseOwner => rfl
  | releaseChild => rfl

end QCore.CarriedPublication
