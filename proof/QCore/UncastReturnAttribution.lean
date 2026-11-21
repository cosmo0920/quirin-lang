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

import QCore.UncontractedAccessAttribution
import QCore.ReturnCastView

/-!
# Rule 12k asked of rule 71a's uncast call (card CG10-SMALL-BUNDLE, gateway rule 74)

Rule 12k re-attributes the gate's refusal of a whole-call return to its
uncontracted callee, reading the spelled actuals; a cast actual is one
neither reader recognises, so `return f((T *)p->m)` kept the gate's text.
Rule 74 asks rule 12k's reading a second time, of rule 71a's uncast call,
only where the spelled reading declined and rule 71a reads the uncast call
(`ReturnCastView.readUncast`). The callee is the same call's.

The model proves that the combined rule is still a re-attribution (the
verdict of every node and the status of every function are unchanged, the
writes are kept, the answer is the gate's refusal or the statement's own
callee), that a view rule 71a keeps leaves rule 12k's spelled answer, and
that the uncast call hands its callee the cast call's identities
(`ReturnCastView.the_same_identities_are_handed`).

## What this is not

A model of the classifier: that the uncast reading answers the uncontracted
refusal is the Rust's (`legacy_uncontracted_access.rs`,
`uncast_uncontracted_reading`), bound by `tests/small_bundle.rs`; the
token/AST match is trust boundary 28.
-/

namespace QCore.UncastReturnAttribution

open QCore.UncontractedAccessAttribution
open QCore.ReturnCastView (ViewPointee readUncast Actual handed uncast)

/-- Rule 74: rule 12k's answer on the spelled call; where it leaves the node
unchanged and rule 71a reads the uncast call, rule 12k's answer on it. -/
def attributeReturn (call : Call) (spelled uncastReading : Option Node) (wholeCall : Bool)
    (views : List ViewPointee) (n : Node) : Node :=
  if reattribute call spelled n ≠ n then reattribute call spelled n
  else if readUncast wholeCall views then reattribute call uncastReading n
  else n

/-- The verdict of every node is unchanged. -/
theorem the_verdict_of_every_node_is_unchanged (call : Call) (s u : Option Node) (w : Bool)
    (vs : List ViewPointee) (n : Node) :
    verdict (attributeReturn call s u w vs n) = verdict n := by
  unfold attributeReturn
  split
  · exact QCore.UncontractedAccessAttribution.the_verdict_of_every_node_is_unchanged call s n
  · split
    · exact QCore.UncontractedAccessAttribution.the_verdict_of_every_node_is_unchanged call u n
    · rfl

/-- The status of every function is unchanged. -/
theorem the_status_of_a_function_is_unchanged (call : Node → Call) (s u : Node → Option Node)
    (w : Node → Bool) (vs : Node → List ViewPointee) (nodes : List Node) :
    status (nodes.map fun n => attributeReturn (call n) (s n) (u n) (w n) (vs n) n) = status nodes := by
  unfold status
  suffices ∀ v : Verdict, List.foldl (fun (v : Verdict) n => v.join (verdict n)) v
      (nodes.map fun n => attributeReturn (call n) (s n) (u n) (w n) (vs n) n) =
      List.foldl (fun (v : Verdict) n => v.join (verdict n)) v nodes from this _
  induction nodes with
  | nil => intro v; rfl
  | cons n rest ih =>
      intro v
      simp only [List.map_cons, List.foldl_cons, the_verdict_of_every_node_is_unchanged]
      exact ih _

/-- The gate's refusal keeps its writes and becomes the gate's refusal or the
statement's own uncontracted callee. -/
theorem the_result_is_the_gate_refusal_or_the_callees (call : Call) (s u : Option Node)
    (w : Bool) (vs : List ViewPointee) (ws : List String) :
    attributeReturn call s u w vs (.refusal .gate ws) = .refusal .gate ws ∨
      attributeReturn call s u w vs (.refusal .gate ws) = .refusal (.uncontracted call.callee) ws := by
  unfold attributeReturn
  split
  · exact QCore.UncontractedAccessAttribution.the_result_is_the_gate_refusal_or_the_callees call s ws
  · split
    · exact QCore.UncontractedAccessAttribution.the_result_is_the_gate_refusal_or_the_callees call u ws
    · exact Or.inl rfl

/-- A view rule 71a keeps (a union, a record typedef) leaves rule 12k's
spelled answer: the uncast call is never asked. -/
theorem a_kept_view_keeps_the_spelled_answer (call : Call) (s u : Option Node) (w : Bool)
    (vs : List ViewPointee) (h : .other ∈ vs) (n : Node) :
    attributeReturn call s u w vs n = reattribute call s n := by
  unfold attributeReturn
  rw [QCore.ReturnCastView.a_kept_view_keeps_the_refusal w vs h]
  split
  · rfl
  · next hne => exact (Decidable.of_not_not hne).symm

/-- A return inside an expression is never read through an uncast call. -/
theorem only_a_whole_call_is_read (call : Call) (s u : Option Node) (vs : List ViewPointee)
    (n : Node) : attributeReturn call s u false vs n = reattribute call s n := by
  unfold attributeReturn
  rw [QCore.ReturnCastView.only_a_whole_call_is_read vs]
  split
  · rfl
  · next hne => exact (Decidable.of_not_not hne).symm

/-- A contracted or held callee keeps the gate's refusal on both readings. -/
theorem a_contracted_callee_keeps_the_gate_refusal (call : Call) (s u : Option Node) (w : Bool)
    (vs : List ViewPointee) (ws : List String) (h : call.contracted = true) :
    attributeReturn call s u w vs (.refusal .gate ws) = .refusal .gate ws := by
  simp [attributeReturn, QCore.UncontractedAccessAttribution.a_contracted_callee_keeps_the_gate_refusal
    call _ ws h]

theorem a_held_callee_keeps_the_gate_refusal (call : Call) (s u : Option Node) (w : Bool)
    (vs : List ViewPointee) (ws : List String) (h : call.heldCallee = true) :
    attributeReturn call s u w vs (.refusal .gate ws) = .refusal .gate ws := by
  simp [attributeReturn, QCore.UncontractedAccessAttribution.a_held_callee_keeps_the_gate_refusal
    call _ ws h]

/-- An earlier attribution is kept: rule 74 never changes rule 12k's answer
where rule 12k already named the callee. -/
theorem an_attributed_spelling_is_kept (call : Call) (s u : Option Node) (w : Bool)
    (vs : List ViewPointee) (n : Node) (h : reattribute call s n ≠ n) :
    attributeReturn call s u w vs n = reattribute call s n := by
  simp [attributeReturn, h]

/-- The uncast call hands its callee exactly the identities the cast call
hands, so the callee that decides the uncast call decides the cast one. -/
theorem the_uncast_call_hands_the_same_identities (as : List Actual) :
    (as.map uncast).map handed = as.map handed :=
  QCore.ReturnCastView.the_same_identities_are_handed as

end QCore.UncastReturnAttribution
