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

namespace QCore.ScalarRouteExitEdge

/-! Card `SCALAR-ROUTE-EXIT-EDGE`: a loop header's exit edge is never removed.

`refine_pointer_null` in `crates/qcore-c-gateway/src/legacy_c.rs` answers
`PointerRefinement::Infeasible` for a branch edge whose null assumption
contradicts the abstract state, and the custody work list then does not
propagate that successor. It is the only place in that file where a successor
is dropped.

For an `if` this is ordinary path sensitivity: the two arms rejoin at the same
continuation, so pruning one orphans nothing. For a **loop header** the two
successors do not rejoin — everything after the loop hangs off the exit edge
alone — so deleting it leaves the function's exit node with no incoming state,
`report_leaks` never runs there, and `classify_diagnostics` returns `Safe`.

Card `C-GATEWAY-CONSTANT-DO-WHILE` proved `the_exit_edge_is_never_removed` for
its own *token* rule and did not check its neighbour. This module makes the
same statement true of the **value** route, and states why the old rule could
not be proved instead of restricted.

The deletion would be exact only if the loop really cannot terminate, and this
route's must-fact is a modelled value rather than a token, so its exactness
rests on two premises the analyzer does not carry:

* `the_abstract_state_does_not_decide_the_exit` — two concrete situations agree
  on everything the analyzer reads and disagree on whether the exit is
  reachable. The witness is a `volatile` object: C11 §5.1.2.3 lets it be
  modified by means outside the program, so `while (!flag) { }` terminates in
  well-defined C while the abstract state still says `flag` is null.
* `a_spared_write_leaves_the_fact_standing` — the fact also survives any body
  write that constraint 10a's refusal *spares*, a role that constraint does not
  record (`SCALAR-ROUTE-POISON-DEPENDENCE`).

The restriction is stated in both directions, because *adding* an edge is the
accusing direction. `propagation_only_grows`, `base_reach_implies_reach` and
`no_diagnostic_is_withdrawn` say nothing is lost;
`the_rule_never_promotes_to_safe` is invariant 1 itself. `a_branch_is_unchanged`
and `the_body_edge_is_unchanged` are the boundary: an `if`'s path sensitivity
and the sibling back-edge arm (`SCALAR-ROUTE-BACK-EDGE-UNDEFENDED`) are
untouched. The non-vacuity claims say the rule is not decoration — there is a
header the old rule really does strand.
-/

/-! ### The pruning site -/

/-- The three answers `refine_pointer_null` gives for one edge. -/
inductive Refinement where
  | refined
  | infeasible
  | unknown
  deriving DecidableEq, Repr

/-- A branch node of the custody work list's CFG. `target` reads an edge index
to its successor node; `loopExitEdge` is `LegacyCfgNode::loop_exit_edge`, the
static index of the successor that leaves the loop this node heads, and `none`
on every node that is not a loop header — including a `for (;;)` header, which
carries no exit successor at all. -/
structure Header where
  edgeCount : Nat
  target : Nat → Nat
  loopExitEdge : Option Nat
  refine : Nat → Refinement

/-- The propagation rule **before** this card: an edge is taken unless the
value route called it infeasible. -/
def basePropagates (h : Header) (edge : Nat) : Prop :=
  edge < h.edgeCount ∧ h.refine edge ≠ Refinement.infeasible

/-- The propagation rule this card lands: a loop header's exit edge is taken
whatever the value route says. -/
def propagates (h : Header) (edge : Nat) : Prop :=
  edge < h.edgeCount ∧
    (h.refine edge ≠ Refinement.infeasible ∨ h.loopExitEdge = some edge)

/-! ### The rule -/

/-- The rule itself, and the property card `C-GATEWAY-CONSTANT-DO-WHILE` proved
of the token route, now true of the value route: a loop header's exit edge is
always propagated. -/
theorem the_exit_edge_is_never_removed (h : Header) (e : Nat)
    (hx : h.loopExitEdge = some e) (hlt : e < h.edgeCount) :
    propagates h e :=
  ⟨hlt, Or.inr hx⟩

/-- Conservativity, at the edge level. The propagated set only ever grows, so
this card can create a refusal and can never withdraw one — the direction
invariant 1 permits. -/
theorem propagation_only_grows (h : Header) (e : Nat) (hb : basePropagates h e) :
    propagates h e :=
  ⟨hb.1, Or.inl hb.2⟩

/-! ### The boundary: what did not move -/

/-- An `if` is not a loop header, so its path sensitivity is exactly what it
was. This is what keeps every `if (p)` after an acquisition as precise as
before. -/
theorem a_branch_is_unchanged (h : Header) (hn : h.loopExitEdge = none) (e : Nat) :
    propagates h e ↔ basePropagates h e := by
  constructor
  · rintro ⟨hlt, hr | hx⟩
    · exact ⟨hlt, hr⟩
    · rw [hn] at hx
      exact absurd hx (by simp)
  · exact propagation_only_grows h e

/-- A loop header's **other** edge — the body edge, which a positively spelled
header such as `while (flag)` is the one to lose — is propagated exactly as
before. The sibling back-edge arm is deliberately untouched by this card
(`SCALAR-ROUTE-BACK-EDGE-UNDEFENDED`). -/
theorem the_body_edge_is_unchanged (h : Header) (e x : Nat)
    (hx : h.loopExitEdge = some x) (hne : e ≠ x) :
    propagates h e ↔ basePropagates h e := by
  constructor
  · rintro ⟨hlt, hr | he⟩
    · exact ⟨hlt, hr⟩
    · rw [hx] at he
      exact absurd (Option.some.inj he).symm hne
  · exact propagation_only_grows h e

/-! ### Nothing is lost -/

/-- Reachability under the old rule. -/
inductive BaseReach (c : Nat → Header) (entry : Nat) : Nat → Prop where
  | start : BaseReach c entry entry
  | step {n e : Nat} : BaseReach c entry n → basePropagates (c n) e →
      BaseReach c entry ((c n).target e)

/-- Reachability under this card's rule. -/
inductive Reach (c : Nat → Header) (entry : Nat) : Nat → Prop where
  | start : Reach c entry entry
  | step {n e : Nat} : Reach c entry n → propagates (c n) e →
      Reach c entry ((c n).target e)

/-- No destination is lost. Every node the analysis reached before it reaches
now, so every exit node at which `report_leaks` ran still runs. -/
theorem base_reach_implies_reach (c : Nat → Header) (entry n : Nat)
    (h : BaseReach c entry n) : Reach c entry n := by
  induction h with
  | start => exact Reach.start
  | step _ hp ih => exact Reach.step ih (propagation_only_grows _ _ hp)

/-- A diagnostic is emitted at a node the analysis reaches. No diagnostic the
old rule reported is withdrawn by this card. -/
theorem no_diagnostic_is_withdrawn (c : Nat → Header) (entry : Nat)
    (emits : Nat → Nat → Prop) (d : Nat)
    (h : ∃ n, BaseReach c entry n ∧ emits n d) :
    ∃ n, Reach c entry n ∧ emits n d := by
  obtain ⟨n, hr, he⟩ := h
  exact ⟨n, base_reach_implies_reach c entry n hr, he⟩

/-- Invariant 1, stated. `Safe` is "no diagnostic at all", so if the function
is `Safe` after this card it was `Safe` before it: the rule cannot promote
anything. -/
theorem the_rule_never_promotes_to_safe (c : Nat → Header) (entry : Nat)
    (emits : Nat → Nat → Prop)
    (safeAfter : ¬ ∃ d n, Reach c entry n ∧ emits n d) :
    ¬ ∃ d n, BaseReach c entry n ∧ emits n d := by
  rintro ⟨d, n, hr, he⟩
  exact safeAfter ⟨d, n, base_reach_implies_reach c entry n hr, he⟩

/-! ### Non-vacuity: the old rule really did strand a header -/

/-- A loop header whose exit edge the value route calls infeasible: two edges,
edge `1` leaves the loop, and the refinement refuses exactly that edge. This is
`int flag = 0; while (!flag) { }` — the shape measured in
`crates/qcore-c-gateway/tests/scalar_route_exit_edge.rs`. -/
def strandedHeader : Header where
  edgeCount := 2
  target := fun e => if e = 1 then 20 else 10
  loopExitEdge := some 1
  refine := fun e => if e = 1 then Refinement.infeasible else Refinement.refined

/-- The old rule drops that exit edge. -/
theorem the_old_rule_strands_the_exit (h : basePropagates strandedHeader 1) : False := by
  exact h.2 (by simp [strandedHeader])

/-- This card's rule keeps it. -/
theorem the_new_rule_keeps_the_exit : propagates strandedHeader 1 :=
  the_exit_edge_is_never_removed strandedHeader 1 rfl (by decide)

/-- And the body edge of that same header is propagated by both, so the
witness separates the two rules on the exit edge alone. -/
theorem the_witness_agrees_on_the_body_edge :
    basePropagates strandedHeader 0 ∧ propagates strandedHeader 0 :=
  ⟨⟨by decide, by simp [strandedHeader]⟩, ⟨by decide, Or.inl (by simp [strandedHeader])⟩⟩

/-! ### Why the old rule was restricted rather than proved -/

/-- What the abstract state carries about the deciding name at the header —
the whole of what `refine_pointer_null` reads. -/
inductive Fact where
  | knownNull
  | knownNonNull
  | unknown
  deriving DecidableEq, Repr

/-- A concrete situation: the fact the analyzer holds, and whether the object
may be modified by means outside the writes the analyzer models — a `volatile`
object (C11 §5.1.2.3), one reached through an escaped address, or one a body
write the refusal *spared* has changed. The analyzer reads `fact` and nothing
else. -/
structure Concrete where
  fact : Fact
  asynchronous : Bool
  deriving DecidableEq, Repr

/-- Whether the loop's exit is genuinely reachable. Under an inverted header a
`knownNull` fact forbids it through the modelled value alone — but an
asynchronous modification restores it. -/
def exitReachable (c : Concrete) : Bool :=
  match c.fact with
  | Fact.unknown => true
  | _ => c.asynchronous

/-- The finding, formally. Two situations agree on **everything the analyzer
reads** and disagree on whether the exit is reachable, so no function of the
abstract state can decide it. The old rule deleted the edge for both; for one
of them the exit is reachable and the code after the loop really runs. -/
theorem the_abstract_state_does_not_decide_the_exit :
    ∃ a b : Concrete, a.fact = b.fact ∧ exitReachable a ≠ exitReachable b :=
  ⟨⟨Fact.knownNull, true⟩, ⟨Fact.knownNull, false⟩, rfl, by decide⟩

/-- The same statement for the second arm, so the finding is not about `int`
bindings: a known **non-null** pointer in `while (p)` carries the identical
gap. -/
theorem the_second_arm_has_the_same_gap :
    ∃ a b : Concrete, a.fact = b.fact ∧ a.fact = Fact.knownNonNull ∧
      exitReachable a ≠ exitReachable b :=
  ⟨⟨Fact.knownNonNull, true⟩, ⟨Fact.knownNonNull, false⟩, rfl, rfl, by decide⟩

/-- This card's rule is independent of the premise it cannot check: the exit
edge is retained for both members of the indistinguishable pair, because
`propagates` reads no `Concrete` at all. That is the whole content of
"restricted rather than proved". -/
theorem retention_does_not_read_the_unchecked_premise
    (h : Header) (e : Nat) (hx : h.loopExitEdge = some e) (hlt : e < h.edgeCount)
    (a b : Concrete) (_hfact : a.fact = b.fact) :
    propagates h e :=
  the_exit_edge_is_never_removed h e hx hlt

/-! ### The write-detection premise the old rule also rested on -/

/-- How a body write reaches the header. A write the analyzer models, or a
refusal that poisons the binding, widens the fact to `unknown`; a write that
constraint 10a's `Sparing` list *spares* leaves it exactly as it was. -/
def factAfterBodyWrite (wrote spared : Bool) (f : Fact) : Fact :=
  if wrote && !spared then Fact.unknown else f

/-- The premise `SCALAR-ROUTE-POISON-DEPENDENCE` names: a spared write leaves
the deciding fact standing, so the old rule's exactness depended on
`Sparing` being complete for the names a loop header reads — a role constraint
10a does not record. -/
theorem a_spared_write_leaves_the_fact_standing (f : Fact) :
    factAfterBodyWrite true true f = f := by
  simp [factAfterBodyWrite]

/-- The arm that made the old rule look safe wherever it was tested: an
unspared write widens the fact, the header then takes
`PointerRefinement::Unknown`, and both edges propagate. It is a real arm — it
is simply not the only one. -/
theorem an_unspared_write_widens_the_fact (f : Fact) :
    factAfterBodyWrite true false f = Fact.unknown := by
  simp [factAfterBodyWrite]

/-- And a body that does not write the name at all leaves it standing, which is
the empty-body case the corpus census counts. -/
theorem no_write_leaves_the_fact_standing (spared : Bool) (f : Fact) :
    factAfterBodyWrite false spared f = f := by
  simp [factAfterBodyWrite]

end QCore.ScalarRouteExitEdge
