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

import QCore.ReferenceCount
import QCore.RetainedBy

/-!
# Returned references and result holders (card REFCOUNT-REFERENCE-RETURNING-DECLARATIONS, rules 13d and 13e)

The abstract rules implemented by `crates/qcore-c-gateway/src/legacy_reference_result.rs`
(rule 13d, the body check of a returned-reference declaration) and by the
result-holder reading of `legacy_retained_by.rs` and `legacy_reference_count.rs`
(rule 13e), contract specification §C04.17 and §C04.14.2.

USER DECISION 2026-10-05 (`W48-CALLEE-DECISION-TABLE` class 3): a reference
is a resource kind; each function balances the references it takes or hands
them off. A **returned reference** (`resource_return_owned(n)` with
`resource_return_kind(reference)`) is how a lookup or constructor hands its
caller one reference. For class 1 (b): a pointer a callee retains stays the
caller's, and the caller must show its object outlives the holder;
otherwise the result is never `Safe`. Rule 13e spells the holder as the
call's **result**.

## The theorems

* Rule 13d: `a_validated_body_never_returns_a_formal`,
  `a_validated_body_returns_only_obtained_references`,
  `a_validated_non_null_body_never_returns_null`,
  `a_withdrawn_declaration_is_never_safe`, `body_vectors_hold`.
* Rule 13e: `a_result_holder_released_first_is_safe`,
  `a_result_holder_dropped_first_is_safe`,
  `a_result_holder_handed_on_is_never_safe`,
  `a_result_holder_without_a_tracked_result_is_undecided`,
  `an_object_released_while_its_handle_holds_it_is_refused`,
  `a_borrowed_familys_last_drop_keeps_its_records`,
  `a_predefined_identifier_outlives_every_holder`.
* The transfer (a declared drop seen from its caller):
  `a_transferred_reference_leaves_the_callers_family`,
  `a_drop_after_a_transfer_is_an_over_drop`,
  `a_use_after_a_transfer_is_refused`.

## What this is not

A model over abstract shapes. That the gateway reads each `return` and each
assignment as the classes say, and the correspondence with the compiled Rust,
are review premises (trust boundary 28), pinned by the Rust unit tests of
`legacy_reference_result.rs` and the route-G controls
(`tests/refcount_reference_returning.rs`). That a declared body performs the
count arithmetic and the nullability it declares stays the declared
assumption rule 13d checks only structurally. That the last drop of the
function's own references on an object it acquired ends what that object
holds, as far as the function can see, is premise P-RESULT-HOLDER-LAST-DROP:
a reference some callee took through a position declared borrowed is that
declaration's assumption (constraint 4).
-/

namespace QCore.ReferenceResult

/-! ## Rule 13d: the body check -/

/-- What a returned or assigned expression is. -/
inductive Source where
  | null
  /-- A call of a declared keep (its argument) or of a declared returned reference. -/
  | reference
  /-- A call of a declared heap allocation (a constructor's fresh object). -/
  | allocation
  | other
  deriving DecidableEq, Repr

/-- One `return` of the body. -/
inductive Ret where
  /-- `return E;` with `E` read as a source. -/
  | expr (s : Source)
  /-- `return x;` for a local `x`: its assignments, whether it is ever given
  away (an actual at a declared release, consumption or drop), and whether
  its address is taken. -/
  | loc (assigns : List Source) (givenAway addressTaken : Bool)
  /-- `return p;` for a formal: the caller's object, no reference taken. -/
  | formal
  deriving DecidableEq, Repr

/-- Whether a source is a reference the body obtained. -/
def obtained (s : Source) : Bool := s == .reference || s == .allocation

/-- A local returned as a reference: obtained, never given away. -/
def admitsLocal (nullable : Bool) (assigns : List Source) (givenAway addressTaken : Bool) : Bool :=
  !givenAway && !addressTaken && assigns.any obtained &&
    assigns.all (fun s => obtained s || (s == .null && nullable))

/-- Whether one `return` is admitted. -/
def admits (nullable : Bool) : Ret → Bool
  | .expr .reference => true
  | .expr .null => nullable
  | .expr _ => false
  | .loc a g ad => admitsLocal nullable a g ad
  | .formal => false

/-- `legacy_reference_result::validated`. -/
def validated (nullable hasGoto : Bool) (rs : List Ret) : Bool :=
  !hasGoto && !rs.isEmpty && rs.all (admits nullable)

theorem admitted_of_validated (n g : Bool) (rs : List Ret) (h : validated n g rs = true) :
    ∀ r ∈ rs, admits n r = true := by
  simp only [validated, Bool.and_eq_true, List.all_eq_true] at h
  exact h.2

theorem a_validated_body_never_returns_a_formal (n g : Bool) (rs : List Ret)
    (h : validated n g rs = true) : Ret.formal ∉ rs := by
  intro hm
  have := admitted_of_validated n g rs h _ hm
  simp [admits] at this

/-- Every `return` of a validated body hands back a reference the body
obtained: a reference call, a null constant of a nullable declaration, or a
local obtained and never given away. -/
theorem a_validated_body_returns_only_obtained_references (n g : Bool) (rs : List Ret)
    (h : validated n g rs = true) :
    ∀ r ∈ rs, r = .expr .reference ∨ (r = .expr .null ∧ n = true) ∨
      ∃ a ad, r = .loc a false ad ∧ ad = false ∧ a.any obtained = true := by
  intro r hr
  have hadm := admitted_of_validated n g rs h r hr
  cases r with
  | expr s => cases s <;> simp_all [admits]
  | loc a g' ad =>
      right; right
      simp only [admits, admitsLocal, Bool.and_eq_true, Bool.not_eq_true'] at hadm
      exact ⟨a, ad, by rw [hadm.1.1.1], hadm.1.1.2, hadm.1.2⟩
  | formal => simp [admits] at hadm

/-- A non-null declaration's validated body returns no null constant and
assigns none to a returned local. -/
theorem a_validated_non_null_body_never_returns_null (g : Bool) (rs : List Ret)
    (h : validated false g rs = true) :
    Ret.expr .null ∉ rs ∧ ∀ a g' ad, Ret.loc a g' ad ∈ rs → Source.null ∉ a := by
  have hall := admitted_of_validated false g rs h
  refine ⟨fun hm => by simpa [admits] using hall _ hm, ?_⟩
  intro a g' ad hm hn
  have hadm := hall _ hm
  simp only [admits, admitsLocal, Bool.and_eq_true, List.all_eq_true] at hadm
  have := hadm.2 _ hn
  simp [obtained] at this

/-- A declaration its body does not validate is withdrawn: every call refuses
(`reference-withdrawn`), so no path through it is `Safe`. -/
theorem a_withdrawn_declaration_is_never_safe (pre post : List ReferenceCount.Event) (n : Nat) :
    ReferenceCount.outcome ((pre ++ .withdrawn n :: post).foldl ReferenceCount.step
      ReferenceCount.State.empty) ≠ .safe := by
  have hr : (ReferenceCount.step (pre.foldl ReferenceCount.step ReferenceCount.State.empty)
      (.withdrawn n)).refused = true := rfl
  have := ReferenceCount.foldl_keeps_refused _ post hr
  rw [List.foldl_append, List.foldl_cons]
  unfold ReferenceCount.outcome
  generalize List.foldl ReferenceCount.step (ReferenceCount.step (List.foldl
    ReferenceCount.step ReferenceCount.State.empty pre) (.withdrawn n)) post = st at this
  cases hv : st.violation <;> simp [this]

/-- The measured shapes (`rd_kafka_toppar_get0`, `_get2`, `rd_kafka_q_new0`)
and the refused ones, as `(returns, nullable, validated)`. -/
def bodyVectors : List (List Ret × Bool × Bool) := [
  -- get0: a keep returned, or NULL.
  ([.expr .reference, .expr .null], true, true),
  ([.expr .reference, .expr .null], false, false),
  -- get2: a local from a returned reference, or NULL.
  ([.expr .null, .loc [.reference] false false], true, true),
  -- q_new0: a local from an allocation.
  ([.loc [.allocation] false false], false, true),
  -- a formal handed back.
  ([.formal], true, false),
  -- an undeclared call, a member read.
  ([.expr .other], true, false),
  -- a bare allocation returned (no local the count is initialised on).
  ([.expr .allocation], true, false),
  -- a local given away (dropped) before it is returned.
  ([.loc [.reference] true false], true, false),
  -- a local whose address is taken.
  ([.loc [.reference] false true], true, false),
  -- a local assigned only NULL.
  ([.loc [.null] false false], true, false),
  -- no return.
  ([], true, false)
]

theorem body_vectors_hold :
    bodyVectors.all (fun v => validated v.2.1 false v.1 == v.2.2) = true := by
  decide

theorem a_jump_withdraws (n : Bool) (rs : List Ret) : validated n true rs = false := by
  simp [validated]

/-! ## Rule 13e: the result as holder -/

/-- The holder class of `result`: the identity the call bound its plain
destination to, if there is one; otherwise a holder the body cannot name. -/
def resultHolder : Option Nat → RetainedBy.Holder
  | some s => .tracked s
  | none => .foreign

/-- The last drop of an acquired holder family ends what it holds
(P-RESULT-HOLDER-LAST-DROP), read as the holder's release; a borrowed
family's last drop ends nothing. -/
def lastDrop (acquired : Bool) (s : Nat) : List RetainedBy.Event :=
  if acquired then [.release s] else []

/-- A caller's object retained by the returned handle, the handle released
first: `Safe`. -/
theorem a_result_holder_released_first_is_safe :
    RetainedBy.replay [.retain (RetainedBy.decide (.callerObject 7) (resultHolder (some 2))),
      .release 2] = true := by
  decide

theorem a_result_holder_dropped_first_is_safe :
    RetainedBy.replay ([.retain (RetainedBy.decide (.callerObject 7) (resultHolder (some 2)))] ++
      lastDrop true 2) = true := by
  decide

/-- A handle returned, stored or leaked still holds at the exit: never `Safe`. -/
theorem a_result_holder_handed_on_is_never_safe :
    RetainedBy.replay [.retain (RetainedBy.decide (.callerObject 7) (resultHolder (some 2)))] =
      false := by
  decide

/-- A borrowed family's last drop is not the object's end (the caller's
reference keeps it alive), so what it holds is still held at the exit. -/
theorem a_borrowed_familys_last_drop_keeps_its_records :
    RetainedBy.replay ([.retain (RetainedBy.decide (.callerObject 7) (resultHolder (some 2)))] ++
      lastDrop false 2) = false := by
  decide

/-- A result the call did not bind to a plain tracked destination is a holder
the body cannot name: every non-static retention is undecided. -/
theorem a_result_holder_without_a_tracked_result_is_undecided (r : RetainedBy.Retained)
    (hr : r ≠ .static) : RetainedBy.decide r (resultHolder none) = .undecided :=
  RetainedBy.foreign_holder_is_undecided r hr

/-- A tracked object released while the handle still holds it is refused,
never accused; released after the handle, it is `Safe`. -/
theorem an_object_released_while_its_handle_holds_it_is_refused :
    RetainedBy.replay [.retain (RetainedBy.decide (.tracked 1) (resultHolder (some 2))),
      .release 1, .release 2] = false ∧
    RetainedBy.replay [.retain (RetainedBy.decide (.tracked 1) (resultHolder (some 2))),
      .release 2, .release 1] = true := by
  decide

/-- `__func__`, `__FUNCTION__` and `__PRETTY_FUNCTION__` are read as static
storage (P-RETAINED-PREDEFINED, C11 6.4.2.2p1): they outlive every holder,
the result included. -/
theorem a_predefined_identifier_outlives_every_holder (s : Option Nat) :
    RetainedBy.decide .static (resultHolder s) = .outlives :=
  RetainedBy.static_outlives_every_holder _

/-! ## The transfer: a declared drop seen from its caller -/

/-- A drop wrapper takes over the caller's reference: on an acquired family
it is the caller's last drop, and the family is discharged. -/
theorem a_transferred_reference_leaves_the_callers_family :
    ReferenceCount.outcome (ReferenceCount.run [.acquire 0, .drop 0]) = .safe := by
  decide

/-- A drop after the transfer takes a reference the caller no longer holds: an
over-drop, a violation candidate (provisional). -/
theorem a_drop_after_a_transfer_is_an_over_drop :
    ReferenceCount.outcome (ReferenceCount.run [.acquire 0, .drop 0, .drop 0]) = .violation := by
  decide

/-- A use after the transfer is refused, never accused. -/
theorem a_use_after_a_transfer_is_refused :
    ReferenceCount.outcome (ReferenceCount.run [.acquire 0, .drop 0, .use 0]) = .refused := by
  decide

end QCore.ReferenceResult
