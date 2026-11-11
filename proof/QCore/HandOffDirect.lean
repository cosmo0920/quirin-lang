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

import QCore.ReturnCodeHandOff
import QCore.RelationalPost

/-!
# A direct call of a hand-off declaration (card HANDOFF-DIRECT-CALL-READING)

The model of gateway rule 12m's amendment: a direct call of a declaration
stating `quant live #P: #owner |-> #callback when #result OP @K;` forks the
caller's state for the handed object, **taken** where the guard holds and
**kept** where it fails, and joins the two only at the caller's own test of
the call's result, read as rule 11r reads a return-code disposition. Abstract
rule theorems, not an extraction of the Rust analyzer
(`crates/qcore-c-gateway/src/handoff_direct.rs`: `classes`, `edge_owner`,
`split`; `handoff_branch.rs`: `direct_admission`;
`legacy_handoff_direct.rs`: the route hooks).

* **The hand-off is two exit classes.** `eq K` and `ne K`, each with the
  owner after the call; every value of a class has that owner
  (`a_class_owner_is_the_guard`).
* **The split state is sound per result class.** On whichever edge of the
  caller's test the returned value takes, an accepted edge's owner is the
  guard's owner for that value (`the_split_state_is_sound_per_class`,
  through `QCore.RelationalPost.the_edge_disposition_is_the_returned_class`).
* **A caller that disagrees with the guard never decides.** If the calling
  body's reading names the object again on some value the guard gives the
  callback, the direct call is not decided
  (`a_caller_that_disagrees_never_decides`); nor is it where the body never
  compares the result with the guard's own constant
  (`an_unbranched_direct_call_never_decides`).
* **A decided call never touches a taken object** on any value the guard
  gives the callback (`a_decided_call_never_touches_a_taken_object`), and
  every value lands on an edge whose owner is the guard's
  (`a_decided_split_is_the_guard_on_every_value`).
* **Vectors.** `direct_vectors_hold` (Rust `DIRECT_VECTORS`).

## What this is not

A model of the decisions over the mathematical integers. That the route
reads the caller's test exactly (rule 11r's `read_test`, with an enumerator
read as the value Clang gives it in the callee's result type and checked
against the constants Clang reads the body comparing the result with), that
the walk's per-class reading is the body's (rule 12m's caller branch check),
and that a non-negative constant compared with an unsigned or enumeration
result means the same comparison over the integers, are review premises
(trust boundary 28).
-/

namespace QCore.HandOffDirect

open QCore.ReturnCodeHandOff (Guard GuardOp Owner ownerAfter Reading admit admitted
  branchesOn touches)

/-- The two exit classes of a hand-off: `eq K` and `ne K`, each with the
owner after the call (Rust `classes`). -/
def classes (g : Guard) : List (RelationalPost.Rel × Owner) :=
  match g.op with
  | .eq => [(⟨.eq, g.value⟩, .callback), (⟨.ne, g.value⟩, .caller)]
  | .ne => [(⟨.eq, g.value⟩, .caller), (⟨.ne, g.value⟩, .callback)]

/-- Every value of a class has the class's owner. -/
theorem a_class_owner_is_the_guard (g : Guard) (c : RelationalPost.Rel) (o : Owner)
    (hc : (c, o) ∈ classes g) (x : Int) (hx : c.sat x = true) : o = ownerAfter g x := by
  obtain ⟨op, k⟩ := g
  cases op <;>
    simp only [classes, List.mem_cons, List.not_mem_nil, or_false, Prod.mk.injEq] at hc <;>
    rcases hc with ⟨rfl, rfl⟩ | ⟨rfl, rfl⟩ <;>
    simp_all [RelationalPost.Rel.sat, RelationalPost.holds, ownerAfter,
      QCore.ReturnCodeHandOff.holds]

/-- Every value lies in one of the two classes. -/
theorem every_value_has_a_class (g : Guard) (x : Int) :
    ∃ c o, (c, o) ∈ classes g ∧ c.sat x = true := by
  obtain ⟨op, k⟩ := g
  by_cases h : x = k
  · cases op
    · exact ⟨⟨.eq, k⟩, .callback, by simp [classes], by simp [RelationalPost.Rel.sat, RelationalPost.holds, h]⟩
    · exact ⟨⟨.eq, k⟩, .caller, by simp [classes], by simp [RelationalPost.Rel.sat, RelationalPost.holds, h]⟩
  · cases op
    · exact ⟨⟨.ne, k⟩, .caller, by simp [classes], by simp [RelationalPost.Rel.sat, RelationalPost.holds, h]⟩
    · exact ⟨⟨.ne, k⟩, .callback, by simp [classes], by simp [RelationalPost.Rel.sat, RelationalPost.holds, h]⟩

/-- The owner one edge of the caller's test selects (Rust `edge_owner`). -/
def edgeOwner (g : Guard) (e : RelationalPost.Rel) : Option Owner :=
  RelationalPost.select (classes g) e

/-- The edge a returned value takes on the test whose true edge is `t`. -/
def edgeOf (t : RelationalPost.Rel) (x : Int) : RelationalPost.Rel :=
  if t.sat x then t else ⟨t.op.neg, t.k⟩

/-- **The split state is sound per result class**: whatever value the call
returns, an accepted edge it takes has the guard's owner for that value. -/
theorem the_split_state_is_sound_per_class (g : Guard) (t : RelationalPost.Rel) (x : Int)
    (o : Owner) (h : edgeOwner g (edgeOf t x) = some o) : o = ownerAfter g x := by
  obtain ⟨c, d, hc, hx⟩ := every_value_has_a_class g x
  have hd : d = o :=
    RelationalPost.the_edge_disposition_is_the_returned_class (classes g) t c d hc x hx o h
  rw [← hd]
  exact a_class_owner_is_the_guard g c d hc x hx

/-- The direct call is decided: the calling body's reading admits it (no
class the guard admits touches the object, the guard's constant compared)
and both edges of the caller's test select an owner. -/
def directDecides (g : Guard) (rd : Reading) (t : RelationalPost.Rel) : Bool :=
  admitted g rd && (edgeOwner g t).isSome && (edgeOwner g ⟨t.op.neg, t.k⟩).isSome

/-- **A caller that disagrees with the guard never decides.** -/
theorem a_caller_that_disagrees_never_decides (g : Guard) (rd : Reading)
    (t : RelationalPost.Rel) (r : Int) (hg : QCore.ReturnCodeHandOff.holds g r = true)
    (ht : touches rd r = true) : directDecides g rd t = false := by
  cases ha : admit g rd with
  | false => simp [directDecides, admitted, ha]
  | true =>
    have := QCore.ReturnCodeHandOff.an_admitted_branch_never_touches_a_taken_object
      g rd r ha hg
    rw [this] at ht
    exact absurd ht (by decide)

/-- **A body that never compares the guard's constant never decides.** -/
theorem an_unbranched_direct_call_never_decides (g : Guard) (rd : Reading)
    (t : RelationalPost.Rel) (h : branchesOn g rd = false) : directDecides g rd t = false := by
  simp [directDecides, QCore.ReturnCodeHandOff.an_unbranched_guard_never_admits g rd h]

/-- **A direct call whose result is stored through a conversion never decides.** -/
theorem a_converted_direct_call_never_decides (g : Guard) (rd : Reading)
    (t : RelationalPost.Rel) (h : rd.resultConverted = true) : directDecides g rd t = false := by
  simp [directDecides, QCore.ReturnCodeHandOff.a_converted_result_never_admits g rd h]

/-- A decided call never touches an object the callback took. -/
theorem a_decided_call_never_touches_a_taken_object (g : Guard) (rd : Reading)
    (t : RelationalPost.Rel) (h : directDecides g rd t = true) (r : Int)
    (hg : QCore.ReturnCodeHandOff.holds g r = true) : touches rd r = false := by
  simp only [directDecides, admitted, Bool.and_eq_true] at h
  exact QCore.ReturnCodeHandOff.an_admitted_branch_never_touches_a_taken_object
    g rd r h.1.1.1.2 hg

/-- **A decided split is the guard on every value**: every returned value
takes an edge that selects an owner, and it is the guard's. -/
theorem a_decided_split_is_the_guard_on_every_value (g : Guard) (rd : Reading)
    (t : RelationalPost.Rel) (h : directDecides g rd t = true) (x : Int) :
    ∃ o, edgeOwner g (edgeOf t x) = some o ∧ o = ownerAfter g x := by
  simp only [directDecides, Bool.and_eq_true] at h
  obtain ⟨⟨_, hT⟩, hF⟩ := h
  have hsome : (edgeOwner g (edgeOf t x)).isSome = true := by
    unfold edgeOf
    split
    · exact hT
    · exact hF
  obtain ⟨o, ho⟩ := Option.isSome_iff_exists.mp hsome
  exact ⟨o, ho, the_split_state_is_sound_per_class g t x o ho⟩

/-! ### Shared vectors -/

/-- `DIRECT_VECTORS` of `handoff_direct.rs`, row by row: (guard is `!=`,
guard value, test operator, test constant, the true edge's owner, the false
edge's owner). -/
def directVectors :
    List (Bool × Int × RelationalPost.Op × Int × Option Owner × Option Owner) := [
  (true, 0, .eq, 0, some .caller, some .callback),
  (true, 0, .ne, 0, some .callback, some .caller),
  (true, 0, .eq, 1, some .callback, none),
  (true, 0, .gt, 0, some .callback, none),
  (false, 2, .eq, 2, some .callback, some .caller),
  (false, 2, .eq, 0, some .caller, none),
  (false, 2, .lt, 2, some .caller, none)
]

theorem direct_vectors_hold :
    directVectors.all (fun (ne, v, o, k, a, b) =>
      let g : Guard := ⟨if ne then .ne else .eq, v⟩
      edgeOwner g ⟨o, k⟩ == a && edgeOwner g ⟨o.neg, k⟩ == b) = true := by
  decide

end QCore.HandOffDirect
