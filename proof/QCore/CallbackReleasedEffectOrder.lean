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
# A candidate's member effect composed after the release

Card **CALLBACK-MEMBER-RELEASED-EFFECT-ORDER** (gateway rule 74p, contract
specification C04.19.6). The transfer (`legacy_transfer.rs`) applies a
call's custody contract, the release of a parameter included, before the
callback route (`legacy_callback_route::apply`) composes a candidate's
member-path states at a plain member, synchronous or table invocation
through that parameter (and a member reading deferred to the object's
constructed state, which reads `constructed` and then the same
composition). In C the callback runs first. r60 refuses the slot form
(`callback-beside-release`); this module proves the late order sound for
every form, so the plain member form needs no such refusal.

Modelled as the Rust code decides it:

* `St` keeps each site's liveness and the reading the composition takes
  from the released object (its field links; `constructed` and the
  registrations alike), which the release never writes (premise
  P-B08-SHALLOW: `resource_action` writes statuses, `contained` and the
  retentions the site holds, never `field_links`).
* `release s D G` is `resource_action` on site `s`: admitted only while `s`
  is live and no guard site in `G` is live (a live holder retaining `s`,
  `refuse_release_of_held`); it releases `s` and what it contains (`D`,
  `cascade_resource_action`). `none` is any outcome that is not admitted
  (a refusal or a diagnostic).
* `compose C` is `legacy_member_path_route::apply` on the linked objects
  `C` a candidate states `#released` or `#moved`: admitted only while every
  one is live (`if single && live`), each then released; otherwise
  `member-path-undecided`.
* `late` is the analysis order (release, then composition); `cOrder` the
  order the C code runs in (composition, then release).

Theorems: `the_release_writes_no_link`; `late_admits_only_disjoint` (an
admitted late order composed no object the release released);
`late_is_the_c_order` (where the late order admits, the C order admits
with the same state: no `Unknown` becomes `Safe` by the order);
`the_late_order_never_admits_what_the_c_order_refuses`;
`a_link_the_release_releases_refuses` (the fixture's contained payload and
the released object itself); `a_kept_path_changes_nothing`;
`the_late_order_is_stricter` (a vector where only the C order admits);
`order_vectors_hold` (the fixture's rows).
-/

namespace QCore.CallbackReleasedEffectOrder

/-- The abstract state the two steps touch. -/
structure St where
  /-- Each site's liveness. -/
  live : Nat → Bool
  /-- The objects linked at the members of each site (the composition's
  reading of the released object). -/
  links : Nat → List Nat

/-- Release every site of `S`. -/
def kill (S : List Nat) (st : St) : St :=
  { st with live := fun x => !(decide (x ∈ S)) && st.live x }

/-- `resource_action` on site `s` containing `D`, guarded by `G`. -/
def release (s : Nat) (D G : List Nat) (st : St) : Option St :=
  if st.live s && !(G.any st.live) then some (kill (s :: D) st) else none

/-- The composition of a candidate's states taking the linked objects `C`. -/
def compose (C : List Nat) (st : St) : Option St :=
  if C.all st.live then some (kill C st) else none

/-- The analysis order: the release, then the composition. -/
def late (s : Nat) (D G C : List Nat) (st : St) : Option St :=
  (release s D G st).bind (compose C)

/-- The C order: the callback, then the release. -/
def cOrder (s : Nat) (D G C : List Nat) (st : St) : Option St :=
  (compose C st).bind (release s D G)

theorem kill_live (S : List Nat) (st : St) (x : Nat) :
    (kill S st).live x = (!(decide (x ∈ S)) && st.live x) := rfl

theorem kill_links (S : List Nat) (st : St) : (kill S st).links = st.links := rfl

theorem kill_comm (S T : List Nat) (st : St) : kill S (kill T st) = kill T (kill S st) := by
  cases st with
  | mk live links =>
    simp only [kill, St.mk.injEq, and_true]
    funext x
    cases decide (x ∈ S) <;> cases decide (x ∈ T) <;> rfl

/-- Releasing only lowers liveness. -/
theorem kill_lowers {S : List Nat} {st : St} {x : Nat} (h : (kill S st).live x = true) :
    st.live x = true := by
  rw [kill_live] at h
  cases hx : st.live x <;> simp_all

/-- The release writes no link: the composition reads the released object's
links (and constructed state) as they were before the release. -/
theorem the_release_writes_no_link {s : Nat} {D G : List Nat} {st t : St}
    (h : release s D G st = some t) : t.links = st.links := by
  unfold release at h
  split at h
  · cases h; rfl
  · cases h

/-- An admitted late order composed no object the release released. -/
theorem late_admits_only_disjoint {s : Nat} {D G C : List Nat} {st t : St}
    (h : late s D G C st = some t) : ∀ c ∈ C, c ∉ s :: D ∧ st.live c = true := by
  unfold late release at h
  split at h
  · simp only [Option.bind_some, compose] at h
    split at h
    · rename_i hall
      intro c hc
      have hlive := List.all_eq_true.mp hall c hc
      rw [kill_live] at hlive
      constructor
      · intro hmem
        simp [hmem] at hlive
      · exact kill_lowers hlive
    · cases h
  · cases h

/-- Where the late order admits, the C order admits with the same state. -/
theorem late_is_the_c_order {s : Nat} {D G C : List Nat} {st t : St}
    (h : late s D G C st = some t) : cOrder s D G C st = some t := by
  have hdis := late_admits_only_disjoint h
  unfold late release at h
  split at h
  · rename_i hguard
    simp only [Option.bind_some, compose] at h
    split at h
    · cases h
      have hall : C.all st.live = true :=
        List.all_eq_true.mpr fun c hc => (hdis c hc).2
      have hs : s ∉ C := fun hsC => (hdis s hsC).1 (List.mem_cons_self ..)
      have hguard' : ((kill C st).live s && !(G.any (kill C st).live)) = true := by
        simp only [Bool.and_eq_true, Bool.not_eq_true'] at hguard ⊢
        refine ⟨?_, ?_⟩
        · rw [kill_live]; simp [hs, hguard.1]
        · cases hany : G.any (kill C st).live
          · rfl
          · exfalso
            obtain ⟨g, hg, hgl⟩ := List.any_eq_true.mp hany
            have : G.any st.live = true := List.any_eq_true.mpr ⟨g, hg, kill_lowers hgl⟩
            rw [hguard.2] at this
            cases this
      unfold cOrder compose release
      simp only [hall, ite_true, Option.bind_some, hguard', kill_comm]
    · cases h
  · cases h

/-- The contrapositive: the late order never admits what the C order
refuses. -/
theorem the_late_order_never_admits_what_the_c_order_refuses {s : Nat} {D G C : List Nat}
    {st : St} (h : cOrder s D G C st = none) : late s D G C st = none := by
  cases hl : late s D G C st with
  | none => rfl
  | some t => rw [late_is_the_c_order hl] at h; cases h

/-- A linked object the release itself releases (the released object, or
one it contains) refuses in the late order. -/
theorem a_link_the_release_releases_refuses {s c : Nat} {D G C : List Nat} {st : St}
    (hc : c ∈ C) (hr : c ∈ s :: D) : late s D G C st = none := by
  cases hl : late s D G C st with
  | none => rfl
  | some t => exact absurd hr (late_admits_only_disjoint hl c hc).1

/-- A path every candidate leaves `#kept` composes nothing. -/
theorem a_kept_path_changes_nothing (st : St) : compose [] st = some st := by
  cases st; rfl

/-- The vector state: sites 0 (the op), 1 and 2 (items), all live. -/
def st0 : St := ⟨fun _ => true, fun _ => []⟩

/-- The late order is stricter, not equal: the op containing item 1 that a
candidate also takes at a link (the fixture's contained payload) is refused
late; the C order's model admits it (in C it is a double release, which the
late order's refusal keeps from `Safe`). -/
theorem the_late_order_is_stricter :
    late 0 [1] [] [1] st0 = none ∧ (cOrder 0 [1] [] [1] st0).isSome = true := by
  constructor
  · exact a_link_the_release_releases_refuses (List.mem_singleton_self 1) (by simp)
  · decide

/-- The fixture's rows: a disjoint payload decides alike in both orders; a
payload the op contains refuses late; the released object linked at its
own member refuses late; a held guard refuses both. -/
theorem order_vectors_hold :
    (late 0 [] [] [1] st0).isSome = true ∧
    (cOrder 0 [] [] [1] st0).isSome = true ∧
    (late 0 [1] [] [1] st0).isNone = true ∧
    (late 0 [] [] [0] st0).isNone = true ∧
    (late 0 [] [2] [1] st0).isNone = true ∧
    (cOrder 0 [] [2] [1] st0).isNone = true := by
  decide

end QCore.CallbackReleasedEffectOrder
