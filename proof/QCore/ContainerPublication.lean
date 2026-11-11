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
# Intrusive-container publication (card W45-FB-RETRY-LIFETIME)

The abstract rule implemented by `legacy_c::container_publication`
(`crates/qcore-c-gateway/src/legacy_container_publication.rs`), gateway
constraint 10d, contract specification §C04.8.

`mk_list_add(&node->_head, &queue->pending)` links the object `node` points
to into a list the caller reaches. Before the card the declaration read both
positions as borrowed: the object stayed the body's, so a body that allocated
it, linked it and returned was accused of a leak, and a helper that linked a
caller's object told its callers nothing had happened.

The model follows one tracked identity through a straight-line sequence of
operations:

* `Status` is the identity's custody: live (the body holds it), published
  (a container reached from the caller holds it), released, or transferred
  by a `return`.
* `Container` is where a publication links it: a container reached from a
  formal the body never rebinds (memory the caller supplied), or anything
  else.
* `step` is the rule. A refusal and a violation are absorbing (`run`).
* `verdict` reads an exit: a live identity at an exit is a leak.

## What this is not

A model of the decision, as the gateway's other Lean modules are. That the
lexical reader recognises the entry and the container of the C it reads, the
join of the published set by intersection, and the correspondence with the
compiled Rust are review premises (trust boundary 28), pinned by
`crates/qcore-c-gateway/tests/retry_lifetime.rs` and the module's unit tests.
It is not a proof that a Fluent Bit list is used correctly.
-/

namespace QCore.ContainerPublication

/-- The custody of one tracked identity. -/
inductive Status where
  | live
  | published
  | released
  | transferred
  deriving DecidableEq, Repr

/-- Where a publication links the identity. -/
inductive Container where
  | callerFormal
  | other
  deriving DecidableEq, Repr

/-- One operation on the identity. -/
inductive Op where
  | publish (container : Container)
  | unpublish
  | release
  | ret
  deriving DecidableEq, Repr

/-- The state after an operation: a status, a refusal (`Unknown`), or a
violation candidate (provisional `Unsafe`). -/
inductive Outcome where
  | ok (status : Status)
  | refused
  | violation
  deriving DecidableEq, Repr

/-- The rule, one operation at a time. -/
def step : Status → Op → Outcome
  | .live, .publish .callerFormal => .ok .published
  | .live, .publish .other => .refused
  | _, .publish _ => .refused
  | .published, .unpublish => .ok .live
  | _, .unpublish => .refused
  | .live, .release => .ok .released
  | .published, .release => .violation
  | .released, .release => .violation
  | .transferred, .release => .violation
  | .live, .ret => .ok .transferred
  | .published, .ret => .refused
  | .released, .ret => .violation
  | .transferred, .ret => .violation

/-- A sequence of operations; a refusal and a violation absorb the rest. -/
def run : Outcome → List Op → Outcome
  | outcome, [] => outcome
  | .ok status, op :: ops => run (step status op) ops
  | .refused, _ :: _ => .refused
  | .violation, _ :: _ => .violation

/-- A function's verdict for the identity. -/
inductive Verdict where
  | safe
  | provisionalUnsafe
  | unknown
  deriving DecidableEq, Repr

/-- An exit: a live identity is a leak. -/
def verdict : Outcome → Verdict
  | .ok .live => .provisionalUnsafe
  | .ok _ => .safe
  | .refused => .unknown
  | .violation => .provisionalUnsafe

/-- The pre-card borrowed reading: publication and unlink change nothing. -/
def stepBorrowed : Status → Op → Outcome
  | status, .publish _ => .ok status
  | status, .unpublish => .ok status
  | status, op => step status op

def runBorrowed : Outcome → List Op → Outcome
  | outcome, [] => outcome
  | .ok status, op :: ops => runBorrowed (stepBorrowed status op) ops
  | .refused, _ :: _ => .refused
  | .violation, _ :: _ => .violation

theorem run_refused (ops : List Op) : run .refused ops = .refused := by
  cases ops <;> rfl

theorem run_violation (ops : List Op) : run .violation ops = .violation := by
  cases ops <;> rfl

-- A publication into a caller's container discharges the identity.

theorem a_publication_into_a_callers_container_discharges :
    verdict (run (.ok .live) [.publish .callerFormal]) = .safe := by
  decide

/-- Non-vacuity: the borrowed reading accused the same body. -/
theorem the_borrowed_reading_accused_the_publication :
    verdict (runBorrowed (.ok .live) [.publish .callerFormal]) = .provisionalUnsafe := by
  decide

-- Releasing a published identity is never `Safe`, whatever follows.

theorem a_release_while_published_is_a_violation (ops : List Op) :
    verdict (run (.ok .live) (.publish .callerFormal :: .release :: ops)) = .provisionalUnsafe := by
  simp [run, step, run_violation, verdict]

theorem a_release_while_published_is_never_safe (ops : List Op) :
    verdict (run (.ok .live) (.publish .callerFormal :: .release :: ops)) ≠ .safe := by
  rw [a_release_while_published_is_a_violation]
  decide

-- An unlink before the release restores the body's custody.

theorem an_unlink_restores_custody :
    run (.ok .live) [.publish .callerFormal, .unpublish] = .ok .live := by
  decide

theorem publish_unlink_release_is_safe :
    verdict (run (.ok .live) [.publish .callerFormal, .unpublish, .release]) = .safe := by
  decide

/-- The unlink hands back an obligation: publish, unlink and stop is a leak. -/
theorem an_unlink_without_a_release_leaks :
    verdict (run (.ok .live) [.publish .callerFormal, .unpublish]) = .provisionalUnsafe := by
  decide

-- Refusals.

theorem a_publication_into_another_container_is_refused (ops : List Op) :
    verdict (run (.ok .live) (.publish .other :: ops)) = .unknown := by
  simp [run, step, run_refused, verdict]

theorem a_return_of_a_published_identity_is_refused (ops : List Op) :
    verdict (run (.ok .live) (.publish .callerFormal :: .ret :: ops)) = .unknown := by
  simp [run, step, run_refused, verdict]

theorem an_unlink_of_an_unpublished_identity_is_refused (status : Status)
    (h : status ≠ .published) : step status .unpublish = .refused := by
  cases status <;> simp_all [step]

theorem only_a_live_identity_is_published (status : Status) (container : Container)
    (h : step status (.publish container) = .ok .published) :
    status = .live ∧ container = .callerFormal := by
  cases status <;> cases container <;> simp_all [step]

/-- A second publication of the same identity is refused. -/
theorem a_published_identity_is_not_published_again (container : Container) :
    step .published (.publish container) = .refused := by
  cases container <;> rfl

-- The rule never turns a refusal or a violation into `Safe`.

theorem no_step_certifies_after_a_refusal (ops : List Op) :
    verdict (run .refused ops) ≠ .safe := by
  rw [run_refused]
  decide

theorem no_step_certifies_after_a_violation (ops : List Op) :
    verdict (run .violation ops) ≠ .safe := by
  rw [run_violation]
  decide

-- The published set joins by intersection: a site is published after a join
-- only when every arriving path published it.

/-- The join of two paths' published sets, as membership predicates. -/
def joinPublished (one other : Nat → Bool) (site : Nat) : Bool :=
  one site && other site

theorem a_joined_publication_was_published_on_both (one other : Nat → Bool) (site : Nat)
    (h : joinPublished one other site = true) : one site = true ∧ other site = true := by
  simp [joinPublished] at h
  exact h

/-- An unlink after a join of a publishing and a non-publishing path is not
admitted: the joined set does not hold the site. -/
theorem a_one_sided_publication_is_not_unlinked (one other : Nat → Bool) (site : Nat)
    (h : other site = false) : joinPublished one other site = false := by
  simp [joinPublished, h]

end QCore.ContainerPublication
