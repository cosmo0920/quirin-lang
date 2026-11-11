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

import QCore.ContainerRootReach
import QCore.SchedulerLifetime

/-!
# The element destroyer (card W48-SCHED-CLEANUP-PRODUCTION, gateway rule 34)

The abstract rule implemented by the transfer arm `destroyer-order`
(`rule-34-element-destroyer`, asked by `transfer_rules::container_order`
since card RULE34-PILOT; `QCore.ElementDestroyerSwitch` models its phases
and switch) and the `ContractCall` arm of `legacy_transfer.rs`, with the loader placement
rule of `qcore_contract::contract_counted_users::validate`.

* A declaration may state `resource_unpublished(p)` beside
  `resource_released(p, …)`: the call unlinks the object `p` designates and
  then ends it. `lower` is how a caller reads such a call: rule 10f's unlink
  first, then the release (`ContainerRootReach.step`). Every other entry keeps
  the unlink's refusal; nothing new is admitted about roots.
* `placement` is the loader rule: a publication's entry stays
  `resource_borrowed`, and an unlink may sit beside a borrow or a release,
  never beside another custody mode.
* The bridge to the scheduler model (`QCore.SchedulerLifetime`): the
  element-visible steps of one modelled `mk_http_thread_destroy`
  (`destroyTrace`), read through `elementOp`, are exactly `lower`, for every
  params and callback branch; and a successful modelled cleanup of a list
  leaves each element unlinked and retired. That model is finite and its
  prerequisites (parent embedding, callback footprint, TLS, coroutine,
  membership, aliases) are Open premises named on the production declaration;
  this module proves nothing about the C bodies.

## What this is not

A model of the decision, as the gateway's other Lean modules are. The lexical
reader, the Rust correspondence (trust boundary 28) and the premises
P-CRR-MEMBER / P-CRR-LINKED are review premises, pinned by
`crates/qcore-c-gateway/tests/element_destroyer.rs`.
-/

namespace QCore.ElementDestroyer

open QCore.ContainerRootReach

/-- How a caller reads one element-destroyer call whose entry is reached
through a container rooted at `root`: the unlink, then the release. -/
def lower (root : Root) : List Op := [.unlink root, .release]

/-- The custody modes a container effect may sit beside. -/
inductive Mode where
  | borrowed
  | owned
  | consumed
  | released
  | out
  | stored
  | removed
  deriving DecidableEq, Repr

inductive Effect where
  | published
  | unpublished
  deriving DecidableEq, Repr

/-- The loader's placement rule. -/
def placement : Effect → Mode → Bool
  | .published, .borrowed => true
  | .unpublished, .borrowed => true
  | .unpublished, .released => true
  | _, _ => false

theorem a_publication_keeps_its_borrowed_entry (mode : Mode) :
    placement .published mode = true ↔ mode = .borrowed := by
  cases mode <;> simp [placement]

theorem an_unlink_sits_beside_a_borrow_or_a_release_only (mode : Mode) :
    placement .unpublished mode = true ↔ mode = .borrowed ∨ mode = .released := by
  cases mode <;> simp [placement]

-- The caller's reading.

theorem destroying_an_admitted_element_is_safe (root : Root) (h : admitted root = true) :
    verdict (run (.ok .linked) (lower root)) = .safe := by
  cases root <;> simp [admitted] at h <;> decide

theorem destroying_an_unadmitted_element_is_refused (root : Root) (ops : List Op)
    (h : admitted root = false) :
    verdict (run (.ok .linked) (lower root ++ ops)) = .unknown := by
  simp [lower, run, step, h, run_refused, verdict]

/-- The order matters: a release before the unlink would release an element
the body does not hold, which rule 5l refuses. -/
theorem the_release_never_precedes_the_unlink (root : Root) :
    run (.ok .linked) [.release, .unlink root] = .refused := rfl

theorem destroying_twice_is_never_safe (root₁ root₂ : Root) (ops : List Op) :
    verdict (run (.ok .linked) (lower root₁ ++ lower root₂ ++ ops)) ≠ .safe := by
  cases h : admitted root₁ <;>
    simp [lower, run, step, h, run_refused, run_violation, verdict]

theorem a_release_after_the_destroyer_is_a_violation (root : Root) (ops : List Op)
    (h : admitted root = true) :
    verdict (run (.ok .linked) (lower root ++ .release :: ops)) = .provisionalUnsafe := by
  simp [lower, run, step, h, run_violation, verdict]

theorem an_unlink_before_the_destroyer_is_refused (root₁ root₂ : Root) (ops : List Op) :
    verdict (run (.ok .linked) (.unlink root₁ :: lower root₂ ++ ops)) ≠ .safe := by
  cases h : admitted root₁ <;> simp [lower, run, step, h, run_refused, verdict]

/-- An object the body published itself: the destroyer's unlink hands it back
and its release ends it. -/
theorem destroying_a_published_object_is_safe :
    verdict (run (.ok .body) (.publish .stableFormal :: lower .stableFormal)) = .safe := by
  decide

/-- The summary gate is unchanged: a destroying body tells its callers
nothing (rule 10f(3)). -/
theorem a_destroying_body_is_withheld (root : Root) : withheld (lower root) = true := rfl

-- The bridge to the scheduler model.

open QCore.SchedulerLifetime in
/-- The step of one modelled destroy that the caller's element sees: its
unlink and the free of the parent allocation it is embedded in
(`exact_parent_embedding`). Params, event, callback and coroutine steps touch
no element under the declaration's named premises. -/
def elementOp (parent : Nat) (root : Root) : Step → Option Op
  | .unlink p => if p = parent then some (.unlink root) else none
  | .freeParent p => if p = parent then some .release else none
  | .freeTls _ => none
  | .clearTls => none
  | .resetEvent _ => none
  | .callback _ _ => none
  | .deleteCoroutine _ => none

open QCore.SchedulerLifetime in
theorem the_modelled_destroy_is_an_unlink_then_a_release
    (params : Option Nat) (t : Thread) (event : Nat) (root : Root) :
    (destroyTrace params t event).filterMap (elementOp t.parent root) = lower root := by
  cases params <;> cases h : t.callback <;>
    simp [destroyTrace, lower, h, List.filterMap_cons, elementOp]

open QCore.SchedulerLifetime in
theorem the_modelled_destroy_of_an_admitted_element_is_safe
    (params : Option Nat) (t : Thread) (event : Nat) (root : Root)
    (h : admitted root = true) :
    verdict (run (.ok .linked)
      ((destroyTrace params t event).filterMap (elementOp t.parent root))) = .safe := by
  rw [the_modelled_destroy_is_an_unlink_then_a_release]
  exact destroying_an_admitted_element_is_safe root h

open QCore.SchedulerLifetime in
/-- A successful modelled cleanup of a one-element list unlinks that element
from the selected list and retires its parent allocation: the two clauses the
production declaration states. -/
theorem a_modelled_cleanup_unlinks_and_retires (protectedIds : List Nat) (p : Nat)
    (s after : State) (trace : List Step)
    (h : cleanup protectedIds [p] s = some (after, trace)) :
    p ∉ after.selected ∧ p ∉ after.live := by
  simp only [cleanup] at h
  cases hfind : s.threads.find? (fun t => t.parent == p) with
  | none => simp [hfind] at h
  | some thread =>
    have hparent : thread.parent = p := by
      have := List.find?_some hfind
      simpa using this
    cases hcb : thread.callback with
    | none =>
      simp [hfind, hcb] at h
      obtain ⟨rfl, -⟩ := h
      simp [retire, hparent]
    | some effects =>
      simp [hfind, hcb] at h
      obtain ⟨pair, -, hpair⟩ := Option.bind_eq_some_iff.mp h
      simp at hpair
      obtain ⟨rfl, -⟩ := hpair
      simp [retire, hparent]

end QCore.ElementDestroyer
