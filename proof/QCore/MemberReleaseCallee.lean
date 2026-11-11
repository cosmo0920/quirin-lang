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
# A released member is not released again through a callee's summary

Card **W45-SR5-SOUNDNESS** (`MEMBER-RELEASE-BEFORE-SUMMARY-RELEASE`,
`crates/qcore-c-gateway/src/legacy_member_release.rs`, section 4). Rule 5k
admits `R0(B->F); … R(B);` when `R` releases its only parameter. Before this
card `R` could be a callee this unit defines whose body releases `B->F` again
(`q_nullable_free(n->buf); node_destroy(n);`): a double release published
`Safe` at function and unit level.

`QCore.MemberRelease` proves an admitted release is the last use of its place
*within the body*; this module carries the interprocedural case. Members are
`Nat`s. A callee this unit defines has a **member effect**: either
characterised (every use of its formal is a member access, a null test, or a
whole single actual of a callee whose effect is known), with the members it
touches and the members it releases, or unknown. A callee with no body here is
governed by its declaration (`none`): the stated premise P-B08-SHALLOW, that a
declared release does not cascade.

## What is proved

* **An admitted place is not released again by any callee of the body**: when
  every `R(B)` of the body admits `F`, no characterised effect among them
  releases `F`.
* **A callee that releases the member withholds the place and is reported**;
  **an uncharacterised callee withholds it**; a callee that touches only other
  members admits it (the neighbour stays `Safe`).
* **The chain composes**: a wrapper's effect carries its callee's releases, so
  `q_nullable_free(n->buf); node_drop(n);` with `node_drop(n) {
  node_destroy(n); }` is reported too; a wrapper over an unknown callee is
  unknown.
* **A reported place is never admitted**, and witnesses pin the probe's rows.

## What this is not

That the Rust reader computes these effects (`characterise`, `member_effect`)
is trust boundary 28, as P1–P4 are for `QCore.MemberRelease`. P-B08-SHALLOW is
a hypothesis, not a theorem: a declared release that does cascade to members is
outside what the gateway can see.
-/

namespace QCore.MemberReleaseCallee

/-- What a callee this unit defines does to the members of its only formal. -/
inductive Effect where
  | characterised (touched released : List Nat)
  | unknown
  deriving DecidableEq, Repr

/-- Every released member is touched (the Rust `released ⊆ touched`). -/
def WellFormed : Effect → Prop
  | .characterised touched released => ∀ f ∈ released, f ∈ touched
  | .unknown => True

/-- Whether one `R(B)` admits the member `F`: `none` is a callee with no body
here (its declaration governs, P-B08-SHALLOW). -/
def admitsAfter (effect : Option Effect) (f : Nat) : Bool :=
  match effect with
  | none => true
  | some (.characterised touched _) => !touched.contains f
  | some .unknown => false

/-- Whether one `R(B)` releases the member `F` again. -/
def releasesAgain (effect : Option Effect) (f : Nat) : Bool :=
  match effect with
  | some (.characterised _ released) => released.contains f
  | _ => false

/-- The place `B->F` is admissible only when every `R(B)` of the body admits
`F` (with P1–P4 of `QCore.MemberRelease`). -/
def admissible (calls : List (Option Effect)) (f : Nat) : Bool := calls.all (admitsAfter · f)

/-- The place is reported as a double release when some `R(B)` releases it again. -/
def reported (calls : List (Option Effect)) (f : Nat) : Bool := calls.any (releasesAgain · f)

/-- **An admitted place is not released again through any callee's summary.** -/
theorem an_admitted_member_is_not_released_again (calls : List (Option Effect)) (f : Nat)
    (wf : ∀ e ∈ calls, ∀ x, e = some x → WellFormed x) (h : admissible calls f = true) :
    ∀ e ∈ calls, releasesAgain e f = false := by
  intro e he
  have ha := List.all_eq_true.mp h e he
  cases e with
  | none => rfl
  | some x =>
    cases x with
    | unknown => rfl
    | characterised touched released =>
      have hw := wf _ he _ rfl
      simp only [admitsAfter, Bool.not_eq_true'] at ha
      simp only [releasesAgain]
      cases hc : released.contains f with
      | false => rfl
      | true =>
        have hr : f ∈ released := by simpa using hc
        have ht : touched.contains f = true := by simpa using hw f hr
        rw [ht] at ha
        exact absurd ha (by decide)

/-- **A reported place is never admitted.** -/
theorem a_reported_place_is_never_admitted (calls : List (Option Effect)) (f : Nat)
    (wf : ∀ e ∈ calls, ∀ x, e = some x → WellFormed x) (h : reported calls f = true) :
    admissible calls f = false := by
  cases hadm : admissible calls f with
  | false => rfl
  | true =>
    obtain ⟨e, he, hr⟩ := List.any_eq_true.mp h
    rw [an_admitted_member_is_not_released_again calls f wf hadm e he] at hr
    exact absurd hr (by decide)

/-- An uncharacterised callee withholds the place. -/
theorem an_uncharacterised_callee_withholds (calls : List (Option Effect)) (f : Nat)
    (h : some .unknown ∈ calls) : admissible calls f = false := by
  cases hadm : admissible calls f with
  | false => rfl
  | true => exact absurd (List.all_eq_true.mp hadm _ h) (by simp [admitsAfter])

/-- A callee that touches only other members admits the place. -/
theorem a_callee_of_other_members_admits (touched released : List Nat) (f : Nat)
    (h : f ∉ touched) : admitsAfter (some (.characterised touched released)) f = true := by
  simp [admitsAfter, h]

/-- A callee with no body here admits the place (its declaration governs). -/
theorem a_declared_callee_admits (f : Nat) : admitsAfter none f = true := rfl

/-- The effect of a wrapper whose formal reaches one inner callee. -/
def compose (outer inner : Effect) : Effect :=
  match outer, inner with
  | .characterised t r, .characterised t' r' => .characterised (t ++ t') (r ++ r')
  | _, _ => .unknown

/-- **A wrapper carries its callee's releases.** -/
theorem a_wrapper_carries_its_callees_releases (t r t' r' : List Nat) (f : Nat)
    (h : f ∈ r') : releasesAgain (some (compose (.characterised t r) (.characterised t' r'))) f
      = true := by
  simp [compose, releasesAgain, h]

/-- A wrapper over an unknown callee is unknown. -/
theorem a_wrapper_over_an_unknown_callee_is_unknown (outer : Effect) :
    compose outer .unknown = .unknown := by
  cases outer <;> rfl

/-- Composition keeps `released ⊆ touched`. -/
theorem compose_well_formed (outer inner : Effect) (ho : WellFormed outer)
    (hi : WellFormed inner) : WellFormed (compose outer inner) := by
  cases outer with
  | unknown => trivial
  | characterised t r =>
    cases inner with
    | unknown => trivial
    | characterised t' r' =>
      intro f hf
      simp only [List.mem_append] at hf ⊢
      rcases hf with hf | hf
      · exact Or.inl (ho f hf)
      · exact Or.inr (hi f hf)

/-! ## Witnesses: the probe's rows

Member `0` is `buf`, `1` is `next`. `node_destroy` touches and releases `buf`;
`node_destroy_next` touches and releases `next`; `free` has no body here. -/

def nodeDestroy : Effect := .characterised [0] [0]
def nodeDestroyNext : Effect := .characterised [1] [1]

/-- `q_nullable_free(n->buf); node_destroy(n);`: reported, not admitted. -/
theorem the_measured_hole_is_reported : reported [some nodeDestroy] 0 = true := by decide

theorem the_measured_hole_is_not_admitted : admissible [some nodeDestroy] 0 = false := by decide

/-- Through the wrapper `node_drop(n) { node_destroy(n); }`. -/
theorem the_wrapper_row_is_reported :
    reported [some (compose (.characterised [] []) nodeDestroy)] 0 = true := by decide

/-- `q_nullable_free(n->buf); node_destroy_next(n);`: still admitted. -/
theorem the_other_member_row_stays_admitted : admissible [some nodeDestroyNext] 0 = true := by
  decide

/-- `if (n->buf) q_nullable_free(n->buf); q_nullable_free(n);`: still admitted. -/
theorem the_declared_destructor_stays_admitted : admissible [none] 0 = true := by decide

end QCore.MemberReleaseCallee
