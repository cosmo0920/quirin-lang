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

import QCore.RelationalPost

/-!
# Relational forwarding and the per-class row (card CALLEE-RETENTION-CLASS1, rule 12s)

The abstract rule implemented by
`crates/qcore-c-gateway/src/legacy_relational_forwarding.rs`
(`infer_forwarded`, `relational_row`), amending rule 11r
(`QCore.RelationalPost`).

USER DECISION 2026-10-05 ("split by shape"): a custody-taking insert is a
conditional custody transfer on success, a return-code disposition. Its
callers are forwarding wrappers, `return f(ARGS);`, which rule 11r refused.

## The model

A forwarding body's paths each end at an exit: a literal `v`, or a forwarded
call whose callee returned `x`; either way the function returns that value
(the same type, so C converts nothing). The classes are the callee's
relations, pairwise disjoint and total. The body restricted to class `r`
keeps exactly the paths whose returned value satisfies `r`: a literal outside
`r` diverges, and the forwarded call is read under the callee's class `r` —
the disposition rule 11r's decided test `f() OP K` (true exactly on `r` by
`test_shape_is_exact`) selects on its true edge, the false edge being the
other classes' restricted bodies.

## The theorems

* `every_path_is_kept_by_one_class`, `no_path_is_kept_by_two_classes`: the
  restricted bodies partition the paths.
* `a_forwarded_call_applies_the_callee_class`: on the restricted body of `r`
  the forwarded call is classified under the callee's own class `r`.
* `the_published_class_holds_at_every_exit`: if every restricted body
  validates its class's disposition, every exit returning a value in `r`
  has `r`'s disposition — the summary callers select from is true of the
  body.
* `a_caller_edge_selects_the_exit_disposition`: through rule 11r's
  selection, a caller's accepted edge applies exactly that disposition.
* `the_row_covers_every_path`: the per-class row checks every path once.
* `a_conversion_can_move_a_value_between_classes`: why the two result types
  must be spelled alike.
* `representative_sat`, `meets_self`: every relation is inhabited, so a
  class always meets itself.

## What this is not

A model of the decision over abstract dispositions `D`. That the ordinary
inference validates a restricted body exactly on the paths it keeps, that the
lowering of `return f(ARGS);` is the C it replaces on the kept path, and the
correspondence with the compiled Rust are review premises (trust boundary 28),
pinned by `legacy_relational_forwarding.rs`'s unit tests and
`crates/qcore-c-gateway/tests/callee_retention_class1.rs`.
-/

namespace QCore.RelationalForwarding

open QCore.RelationalPost

/-- Pairwise disjoint relations. -/
def Disjoint (cs : List Rel) : Prop :=
  ∀ r ∈ cs, ∀ s ∈ cs, r ≠ s → ∀ x, ¬ (r.sat x = true ∧ s.sat x = true)

/-- Relations covering every integer. -/
def Total (cs : List Rel) : Prop := ∀ x : Int, ∃ r ∈ cs, r.sat x = true

/-- One path's exit: whether it forwards, and the value the function returns
(the literal, or the callee's returned value). -/
structure PathEnd where
  forwarded : Bool
  value : Int

/-- The restricted body of class `r` keeps a path exactly when its returned
value lies in `r`. -/
def kept (r : Rel) (p : PathEnd) : Bool := r.sat p.value

theorem every_path_is_kept_by_one_class (cs : List Rel) (htotal : Total cs) (p : PathEnd) :
    ∃ r ∈ cs, kept r p = true :=
  htotal p.value

theorem no_path_is_kept_by_two_classes (cs : List Rel) (hdisjoint : Disjoint cs) (p : PathEnd)
    (r s : Rel) (hr : r ∈ cs) (hs : s ∈ cs) (hkr : kept r p = true) (hks : kept s p = true) :
    r = s := by
  by_cases h : r = s
  · exact h
  · exact absurd ⟨hkr, hks⟩ (hdisjoint r hr s hs h p.value)

/-- Every relation holds of some integer. -/
def representative (r : Rel) : Int :=
  match r.op with
  | .eq | .le | .ge => r.k
  | .ne | .gt => r.k + 1
  | .lt => r.k - 1

theorem representative_sat (r : Rel) : r.sat (representative r) = true := by
  obtain ⟨o, k⟩ := r
  unfold Rel.sat
  rw [holds_iff]
  cases o <;> simp only [representative] <;> omega

theorem meets_self (r : Rel) : meets r r = true :=
  meets_complete r r (representative r) (representative_sat r) (representative_sat r)

variable {D : Type} [DecidableEq D]

/-- **On the restricted body of `r`, the forwarded call takes the callee's
class `r`.** The decided test's true edge is `r` itself; when the callee's
classes are disjoint and list `r` once, the only class that edge meets is
`r`'s, so rule 11r's selection applies its disposition. -/
theorem a_forwarded_call_applies_the_callee_class (r : Rel) (d : D)
    (rest : List (Rel × D))
    (hrest : ∀ c ∈ rest, meets c.1 r = false) :
    select ((r, d) :: rest) r = some d := by
  have hfilter : ((r, d) :: rest).filter (fun c => meets c.1 r) = [(r, d)] := by
    simp only [List.filter_cons, meets_self, ↓reduceIte]
    have : rest.filter (fun c => meets c.1 r) = [] := by
      apply List.filter_eq_nil_iff.mpr
      intro c hc
      simp [hrest c hc]
    rw [this]
  unfold select
  rw [hfilter]
  simp

omit [DecidableEq D] in
/-- A disjoint class list lists a class's relation once, so every other class
misses it. -/
theorem disjoint_others_miss (cs : List (Rel × D)) (hd : Disjoint (cs.map Prod.fst))
    (r : Rel) (c : Rel × D) (hc : c ∈ cs) (hr : r ∈ cs.map Prod.fst) (hne : c.1 ≠ r) :
    meets c.1 r = false := by
  cases hm : meets c.1 r with
  | false => rfl
  | true =>
      obtain ⟨x, hx1, hx2⟩ := meets_sound c.1 r hm
      exact absurd ⟨hx1, hx2⟩
        (hd c.1 (List.mem_map.mpr ⟨c, hc, rfl⟩) r hr hne x)

/-- Per-class validation: on the paths the restricted body of `r` keeps, the
disposition `disp` the body establishes is `d`. -/
def Validates (paths : List PathEnd) (disp : PathEnd → D) (r : Rel) (d : D) : Prop :=
  ∀ p ∈ paths, kept r p = true → disp p = d

omit [DecidableEq D] in
/-- **The published class holds at every exit.** -/
theorem the_published_class_holds_at_every_exit (paths : List PathEnd) (disp : PathEnd → D)
    (classes : List (Rel × D)) (hv : ∀ c ∈ classes, Validates paths disp c.1 c.2)
    (p : PathEnd) (hp : p ∈ paths) (c : Rel × D) (hc : c ∈ classes)
    (hsat : c.1.sat p.value = true) : disp p = c.2 :=
  hv c hc p hp hsat

/-- **A caller's accepted edge selects the exit disposition**: if the caller's
test of the returned value takes the edge `e` and rule 11r's selection on it
is `dv`, every exit of the body that returns a value on that edge established
`dv`. -/
theorem a_caller_edge_selects_the_exit_disposition (paths : List PathEnd)
    (disp : PathEnd → D) (classes : List (Rel × D)) (htotal : Total (classes.map Prod.fst))
    (hv : ∀ c ∈ classes, Validates paths disp c.1 c.2) (e : Rel) (dv : D)
    (hs : select classes e = some dv) (p : PathEnd) (hp : p ∈ paths)
    (he : e.sat p.value = true) : disp p = dv := by
  obtain ⟨r, hr, hsat⟩ := htotal p.value
  obtain ⟨c, hc, hcr⟩ := List.mem_map.mp hr
  subst hcr
  have := select_sound classes e dv hs c.1 c.2 hc p.value hsat he
  rw [hv c hc p hp hsat, this]

/-- **The row covers every path**: when every class's restricted body is
safe, every path of the body lies in one checked restricted body. -/
theorem the_row_covers_every_path (cs : List Rel) (htotal : Total cs) (safe : Rel → Prop)
    (hsafe : ∀ r ∈ cs, safe r) (p : PathEnd) : ∃ r ∈ cs, kept r p = true ∧ safe r := by
  obtain ⟨r, hr, hk⟩ := htotal p.value
  exact ⟨r, hr, hk, hsafe r hr⟩

/-- **A conversion can move a value between classes**: an 8-bit truncation
sends the nonzero 256 to 0, so a wrapper whose result type differed from its
callee's would return a value of another class. -/
theorem a_conversion_can_move_a_value_between_classes :
    ∃ x : Int, (Rel.mk .ne 0).sat x = true ∧ (Rel.mk .eq 0).sat (x % 256) = true :=
  ⟨256, by decide, by decide⟩

end QCore.RelationalForwarding
