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
# Container root reach and the foreign unlink (card W45-CONTAINER-ROOT-REACH)

The abstract rule implemented by `legacy_c::container_publication` rules 5
and 6 (`crates/qcore-c-gateway/src/legacy_container_publication.rs`),
gateway constraint 10d's second paragraph.

* `Root` is where a container is reached from. `lifetime` is the stated
  premise about its storage: an unrebound formal (or a local initialised from
  one) reaches memory its caller supplied (the caller's frame or longer), and
  a static object lives for the program (P-CRR-STATIC). Every other root has
  no premise, and `admitted` refuses it.
* `Holder` follows one element object: linked in a container, held by the
  body after an unlink, released, or refused. `step` is the rule: an unlink of
  an element of an admitted container hands its custody to the body once
  (P-CRR-MEMBER: the container owned it; P-CRR-LINKED: it is still linked).
* `withheld` is the summary gate: a body with a foreign unlink tells its
  callers nothing. `the_borrowed_summary_would_hide_a_double_release` is why.

## What this is not

A model of the decision, as the gateway's other Lean modules are. That the
lexical reader recognises roots, element bindings and every write of an
element name, and the correspondence with the compiled Rust, are review
premises (trust boundary 28), pinned by
`crates/qcore-c-gateway/tests/container_root_reach.rs`. The premises
P-CRR-STATIC, P-CRR-MEMBER and P-CRR-LINKED are stated, not proved.
-/

namespace QCore.ContainerRootReach

/-- Where a container is reached from. -/
inductive Root where
  | stableFormal
  | staticObject
  | reboundFormal
  | localObject
  | calleeResult
  | element
  | bodyOwner
  | unnamed
  deriving DecidableEq, Repr

/-- How long a container's storage lives, from shortest to longest. -/
inductive Lifetime where
  | call
  | caller
  | program
  deriving DecidableEq, Repr

def Lifetime.rank : Lifetime → Nat
  | .call => 0
  | .caller => 1
  | .program => 2

/-- The stated premise per root; `none` where nothing is premised. -/
def lifetime : Root → Option Lifetime
  | .stableFormal => some .caller
  | .staticObject => some .program
  | .localObject => some .call
  | _ => none

/-- The container outlives the function's own call. -/
def OutlivesCall (root : Root) : Prop :=
  ∃ l, lifetime root = some l ∧ Lifetime.call.rank < l.rank

/-- The roots rules 5 and 6 admit. -/
def admitted : Root → Bool
  | .stableFormal => true
  | .staticObject => true
  | _ => false

-- Each admitted root outlives the function, under its premise.

theorem an_admitted_root_outlives_the_call (root : Root) (h : admitted root = true) :
    OutlivesCall root := by
  cases root <;> simp [admitted] at h
  · exact ⟨.caller, rfl, by decide⟩
  · exact ⟨.program, rfl, by decide⟩

theorem a_static_root_lives_for_the_program : lifetime .staticObject = some .program := rfl

-- Refusal is preserved for every unlisted shape.

theorem only_the_listed_roots_are_admitted (root : Root) :
    admitted root = true ↔ root = .stableFormal ∨ root = .staticObject := by
  cases root <;> simp [admitted]

theorem a_local_container_does_not_outlive_the_call : ¬ OutlivesCall .localObject := by
  rintro ⟨l, h, hl⟩
  simp [lifetime] at h
  subst h
  simp [Lifetime.rank] at hl

theorem a_root_without_a_premise_is_refused (root : Root) (h : lifetime root = none) :
    admitted root = false := by
  cases root <;> simp [lifetime] at h <;> rfl

/-- One element object's custody. -/
inductive Holder where
  | linked
  | body
  | released
  deriving DecidableEq, Repr

/-- One operation on the element. -/
inductive Op where
  | unlink (root : Root)
  | publish (root : Root)
  | release
  deriving DecidableEq, Repr

/-- A holder, a refusal (`Unknown`) or a violation candidate. -/
inductive Outcome where
  | ok (holder : Holder)
  | refused
  | violation
  deriving DecidableEq, Repr

/-- Rules 5 and 6 for an element reached through a container rooted at
`root` (the unlink's root). A release of a linked element is refused: the
body does not hold it (rule 5l's untracked element). -/
def step : Holder → Op → Outcome
  | .linked, .unlink root => if admitted root then .ok .body else .refused
  | .linked, .publish _ => .refused
  | .linked, .release => .refused
  | .body, .unlink _ => .refused
  | .body, .publish root => if admitted root then .ok .linked else .refused
  | .body, .release => .ok .released
  | .released, .release => .violation
  | .released, _ => .violation

def run : Outcome → List Op → Outcome
  | outcome, [] => outcome
  | .ok holder, op :: ops => run (step holder op) ops
  | .refused, _ :: _ => .refused
  | .violation, _ :: _ => .violation

inductive Verdict where
  | safe
  | provisionalUnsafe
  | unknown
  deriving DecidableEq, Repr

/-- An exit: an element the body still holds is a leak; one linked in a
container outliving the call, or released, owes nothing. -/
def verdict : Outcome → Verdict
  | .ok .body => .provisionalUnsafe
  | .ok _ => .safe
  | .refused => .unknown
  | .violation => .provisionalUnsafe

theorem run_refused (ops : List Op) : run .refused ops = .refused := by
  cases ops <;> rfl

theorem run_violation (ops : List Op) : run .violation ops = .violation := by
  cases ops <;> rfl

-- A foreign unlink transfers custody exactly once.

theorem a_foreign_unlink_hands_the_element_to_the_body :
    step .linked (.unlink .stableFormal) = .ok .body := rfl

theorem a_second_unlink_is_refused (root : Root) : step .body (.unlink root) = .refused := rfl

theorem custody_moves_once (root₁ root₂ : Root) (ops : List Op) :
    run (.ok .linked) (.unlink root₁ :: .unlink root₂ :: ops) ≠ .ok .body := by
  cases h : admitted root₁ <;> simp [run, step, h, run_refused]

-- The accepting control and the two refusing ones.

theorem a_foreign_unlink_then_release_is_safe :
    verdict (run (.ok .linked) [.unlink .stableFormal, .release]) = .safe := by decide

theorem a_foreign_unlink_then_republication_is_safe :
    verdict (run (.ok .linked) [.unlink .stableFormal, .publish .stableFormal]) = .safe := by
  decide

theorem a_foreign_unlink_then_leak_is_a_leak :
    verdict (run (.ok .linked) [.unlink .stableFormal]) = .provisionalUnsafe := by decide

theorem a_foreign_unlink_then_double_release_is_a_violation (ops : List Op) :
    verdict (run (.ok .linked) (.unlink .stableFormal :: .release :: .release :: ops))
      = .provisionalUnsafe := by
  simp [run, step, admitted, run_violation, verdict]

theorem an_unlink_from_an_unadmitted_root_is_refused (root : Root) (ops : List Op)
    (h : admitted root = false) :
    verdict (run (.ok .linked) (.unlink root :: ops)) = .unknown := by
  simp [run, step, h, run_refused, verdict]

theorem a_republication_into_a_local_container_is_refused :
    verdict (run (.ok .linked) [.unlink .stableFormal, .publish .localObject]) = .unknown := by
  decide

/-- Non-vacuity of the leak: the element's custody is the body's only after
the unlink. -/
theorem a_linked_element_owes_nothing : verdict (run (.ok .linked) []) = .safe := by decide

-- The summary gate.

/-- A body's call-level summary: withheld when it unlinks a foreign element. -/
def withheld (ops : List Op) : Bool :=
  ops.any fun op => match op with
    | .unlink _ => true
    | _ => false

/-- A caller that published an object, called a body that unlinked and
released it, and then unlinked and released it again: the one object's
real history is a double release. -/
def realHistory : List Op :=
  [.unlink .stableFormal, .release, .unlink .stableFormal, .release]

/-- The caller's view with a `Borrowed` summary for that body: the callee's
two steps vanish, and the caller's own unlink and release look clean. -/
def borrowedView : List Op := [.unlink .stableFormal, .release]

theorem the_borrowed_summary_would_hide_a_double_release :
    verdict (run (.ok .linked) realHistory) ≠ .safe
      ∧ verdict (run (.ok .linked) borrowedView) = .safe := by
  decide

theorem the_draining_body_is_withheld :
    withheld [.unlink .stableFormal, .release] = true := rfl

end QCore.ContainerRootReach
