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

import QCore.ElementDestroyer

/-! Card RULE34-PILOT: rule 34 as a switchable package at the transfer.

The gateway's transfer (`legacy_transfer.rs`, its `ContractCall` arm) asks the
transfer arm `destroyer-order` (`rule-34-element-destroyer`) for the phase of
each declared container effect of one admitted contract call: an element
destroyer's unlink (`resource_unpublished(p)` of a parameter the declaration
releases) before the call's own reading, every other effect after it. The arm
refuses a call with no such unlink, and the host's order (every effect after
the call) stands. With rule 34 off the arm is not asked, and the host's guard
refuses a call that still has a destroyer's unlink: the host's own order would
release before unlinking, which rule 34 forbids
(`QCore.ElementDestroyer.the_release_never_precedes_the_unlink` is the model
of that order's refusal; the Rust host's release-first reading is not
modelled, so off never falls back to it).

These theorems state that the arm's phases are the Rust table
(`phaseVectors` is `DECISION_VECTORS`), that the switch on is exactly rule 34's
caller reading (`QCore.ElementDestroyer.lower`), that off refuses the
destroyer and changes nothing else, and that off never widens. The Rust/model
correspondence and the host's reading of the declaration remain premises
(trust boundary 28), pinned by the package's and the gateway's tests. -/
namespace QCore.ElementDestroyerSwitch

open QCore.ContainerRootReach
open QCore.ElementDestroyer

/-- One declared container effect of an admitted contract call. -/
structure DeclaredEffect where
  unlinks : Bool
  releases : Bool
  deriving DecidableEq, Repr

/-- When an effect applies, relative to the call's own reading. -/
inductive Phase where
  | beforeCall
  | afterCall
  deriving DecidableEq, Repr

/-- An element destroyer's unlink. -/
def isDestroyer (effect : DeclaredEffect) : Bool := effect.unlinks && effect.releases

/-- The arm's phase of one effect. -/
def phase (effect : DeclaredEffect) : Phase :=
  if isDestroyer effect then .beforeCall else .afterCall

/-- The arm's answer: the phases when some effect is a destroyer's unlink,
`none` (a refusal) otherwise. -/
def order (effects : List DeclaredEffect) : Option (List Phase) :=
  if effects.any isDestroyer then some (effects.map phase) else none

/-- The Rust table `DECISION_VECTORS`: `(unlinks, releases, phase)`. -/
def phaseVectors : List (Bool × Bool × Phase) :=
  [ (false, false, .afterCall)
  , (false, true, .afterCall)
  , (true, false, .afterCall)
  , (true, true, .beforeCall) ]

theorem the_phase_vectors_are_the_arm (unlinks releases : Bool) :
    (unlinks, releases, phase ⟨unlinks, releases⟩) ∈ phaseVectors := by
  cases unlinks <;> cases releases <;> decide

theorem every_phase_vector_is_the_arm_phase :
    phaseVectors.all (fun row => phase ⟨row.1, row.2.1⟩ == row.2.2) = true := by
  decide

theorem an_admission_has_one_phase_per_effect (effects : List DeclaredEffect)
    (phases : List Phase) (h : order effects = some phases) :
    phases.length = effects.length := by
  unfold order at h
  split at h
  · cases h
    simp
  · cases h

theorem only_a_destroyer_moves_before_the_call (effect : DeclaredEffect) :
    phase effect = .beforeCall ↔ effect.unlinks = true ∧ effect.releases = true := by
  cases effect with
  | mk unlinks releases => cases unlinks <;> cases releases <;> decide

/-- What the transfer does with one call's effects. -/
inductive HostOrder where
  | ordered (phases : List Phase)
  | refused
  deriving DecidableEq, Repr

/-- The host with rule 34 on or off: the arm is asked only while on; its
admission is the order; otherwise a call that still has a destroyer's unlink
is refused (the guard), and every other call keeps the host's order. -/
def hostOrder (on : Bool) (effects : List DeclaredEffect) : HostOrder :=
  match (if on then order effects else none) with
  | some phases => .ordered phases
  | none =>
    if effects.any isDestroyer then .refused
    else .ordered (effects.map fun _ => .afterCall)

theorem the_switch_on_orders_every_destroyer_first (effects : List DeclaredEffect)
    (h : effects.any isDestroyer = true) :
    hostOrder true effects = .ordered (effects.map phase) := by
  simp [hostOrder, order, h]

theorem the_switch_off_refuses_a_destroyer (effects : List DeclaredEffect)
    (h : effects.any isDestroyer = true) :
    hostOrder false effects = .refused := by
  simp [hostOrder, h]

/-- Rule 34 changes nothing for a call without a destroyer's unlink: on or
off, the host's order stands. -/
theorem a_call_without_a_destroyer_ignores_the_switch (effects : List DeclaredEffect)
    (h : effects.any isDestroyer = false) (on : Bool) :
    hostOrder on effects = .ordered (effects.map fun _ => .afterCall) := by
  cases on <;> simp [hostOrder, order, h]

theorem with_no_destroyer_the_arm_order_is_the_host_order (effects : List DeclaredEffect)
    (h : effects.any isDestroyer = false) :
    effects.map phase = effects.map fun _ => .afterCall := by
  apply List.map_congr_left
  intro effect mem
  have : isDestroyer effect = false := by
    cases hd : isDestroyer effect
    · rfl
    · have : effects.any isDestroyer = true := List.any_eq_true.mpr ⟨effect, mem, hd⟩
      simp_all
  simp [phase, this]

/-- The caller's reading of one element-destroyer call on a linked element
reached through `root`, followed by `ops`, with rule 34 on or off. -/
def destroyUnder (on : Bool) (root : Root) (ops : List Op) : Outcome :=
  match hostOrder on [⟨true, true⟩] with
  | .ordered _ => run (.ok .linked) (lower root ++ ops)
  | .refused => .refused

theorem the_switch_on_is_rule_34 (root : Root) (ops : List Op) :
    destroyUnder true root ops = run (.ok .linked) (lower root ++ ops) := by
  simp [destroyUnder, hostOrder, order, isDestroyer]

theorem the_switch_on_destroys_an_admitted_element_safely (root : Root)
    (h : admitted root = true) : verdict (destroyUnder true root []) = .safe := by
  rw [the_switch_on_is_rule_34, List.append_nil]
  exact destroying_an_admitted_element_is_safe root h

theorem the_switch_off_refuses_the_destroyer (root : Root) (ops : List Op) :
    destroyUnder false root ops = .refused := by
  simp [destroyUnder, hostOrder, isDestroyer]

theorem the_switch_off_reads_unknown (root : Root) (ops : List Op) :
    verdict (destroyUnder false root ops) = .unknown := by
  rw [the_switch_off_refuses_the_destroyer]
  rfl

/-- Off never widens: whatever rule 34 off reads `Safe`, it reads `Safe` on. -/
theorem the_switch_off_never_widens (root : Root) (ops : List Op)
    (h : verdict (destroyUnder false root ops) = .safe) :
    verdict (destroyUnder true root ops) = .safe := by
  rw [the_switch_off_reads_unknown] at h
  cases h

/-- And off never admits what on refuses. -/
theorem the_switch_off_never_admits_a_refused_destroy (root : Root) (ops : List Op)
    (_refused : verdict (destroyUnder true root ops) ≠ .safe) :
    verdict (destroyUnder false root ops) ≠ .safe := by
  rw [the_switch_off_reads_unknown]
  intro absurd
  cases absurd

end QCore.ElementDestroyerSwitch
