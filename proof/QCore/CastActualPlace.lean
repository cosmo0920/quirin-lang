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

import QCore.CastSemantics

/-!
# A pointer view of a member place at a call actual (card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 3, rule 12h amended)

Rule 12h's hook H-U removed an identity-preserving cast of a **plain name**
at a value position and lowered the uncast statement. Round 3 extends the
removal, at a **call actual only**, to a member-chain place over a plain
name (`(const char *)a->name`), which hook H-U0 already reads as that place.
The decision (`place_view_removed` in `legacy_cast_semantics.rs`) is bound
to `PLACE_VIEW_VECTORS`. The soundness argument is
`QCore.CastSemantics.view_preserves_identity`: the converted value carries
the operand's identity, so the uncast actual hands the callee exactly the
identity the cast one does, and the older routes decide it at its position.

## What this is not

A model of the classifier: that the uncast statement is lowered by the
older routes, which decide the place at the contract's position, is the
gateway's; faithful extraction is trust boundary 28.
-/

namespace QCore.CastActualPlace

open QCore.CastSemantics

/-- The amendment's decision: an identity-preserving class, at a call actual,
of a member-chain place, with no `volatile`/`_Atomic` result. -/
def placeViewRemoved (identityPreserving callActual memberPlace qualified : Bool) : Bool :=
  identityPreserving && callActual && memberPlace && !qualified

/-- The Rust's `PLACE_VIEW_VECTORS`. -/
def placeViewVectors : List (Bool × Bool × Bool × Bool × Bool) :=
  [(true, true, true, false, true), (false, true, true, false, false),
   (true, false, true, false, false), (true, true, false, false, false),
   (true, true, true, true, false)]

theorem place_view_vectors_hold :
    placeViewVectors.all (fun (i, a, m, q, r) => placeViewRemoved i a m q == r) = true := by
  decide

/-- Only at a call actual (never an initializer, assignment value or pointer
step, where a plain name alone is removed). -/
theorem only_a_call_actual_is_uncast (i m q : Bool) : placeViewRemoved i false m q = false := by
  simp [placeViewRemoved]

/-- A qualified (`volatile`/`_Atomic`) view is never removed. -/
theorem a_qualified_view_is_kept (i a m : Bool) : placeViewRemoved i a m true = false := by
  simp [placeViewRemoved]

/-- A removed view preserves the place's identity: the uncast actual is the
same identity (`view_preserves_identity`), so removing it changes nothing a
custody route reads. -/
theorem a_removed_view_is_the_place (transfer : Int → Int) (c : ConvClass) (target : Pointee)
    (v v' : Val) (hc : c = .byteView ∨ c = .typedView ∨ c = .punnedView ∨ c = .qualifier)
    (h : convert transfer c target v = .value v') : identityOf v' = identityOf v :=
  view_preserves_identity transfer c target v v' hc h

end QCore.CastActualPlace
