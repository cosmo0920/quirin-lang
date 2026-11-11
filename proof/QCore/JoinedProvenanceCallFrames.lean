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

import QCore.JoinedProvenanceCycles

/-! Abstract rules for **preservation of conditional provenance across a call**
(card JOINED-PROVENANCE-CALL-FRAMES).

Before this card the pre-pass forgot every `site -> names` relation at every
contracted call: the call arm cleared the binding map outright. The corpus
measurement that motivated the change attributes the lost name to exactly that
rule at every site whose relation ever named the released pointer.

The rule this module states is the bounded replacement. A call carries a
**frame** `F` — the closed list of names whose slot no lvalue but the name
itself designates — and keeps a claim only for names in `F`; every other name
is forgotten exactly as before. In the analyzer `F` is `conditional_names`,
which is `unique_local_names ∩ frame_protected − formals`: an unshadowed object
of automatic storage duration this body declared, with no `&` anywhere in the
body taking its address.

As with every gateway module these are **abstract rule theorems, not extraction
of the Rust analyzer** (`GATEWAY-LEAN-ABSTRACT-NOT-EXTRACTION`). Two things are
premises here rather than conclusions, and both are named in the report: that
the token reader computes a frame that really is address-never-taken and
automatic (`FrameRespecting` is assumed of the call, not derived from C), and
that the C object model's "a callee reaches only what its arguments and
file-scope objects give it" holds — the same no-undefined-behaviour reading
standing constraint 10a already rests on (`AOM-POISON-NO-UB`).

What *is* proved: that a call which respects the frame preserves soundness of
the kept claim; that the rule only ever weakens, so it cannot manufacture a
fact; that a name the call actually writes is outside every frame, which is why
the destination, the `Out` slots and a `Reallocated` parameter must be — and
are — forgotten; that the empty frame reproduces the old blanket forget exactly,
so the change is a generalisation and not a replacement; and that a call edge
carrying this transfer satisfies precisely the `TransferSound` premise
`QCore.JoinedProvenanceCycles.fixed_point_is_sound` already requires, so both
the acyclic pass and the cyclic fixed point keep their theorems unedited. -/
namespace QCore.JoinedProvenanceCallFrames

open QCore.CGatewayRefinements
open QCore.JoinedProvenanceCycles

/-- A call **respects the frame** `F` when it changes the value of no name in
`F`. This is the premise the analyzer discharges by construction: a name in
`conditional_names` has no `&` anywhere in the body, so no pointer to its slot
exists for a callee to hold, and it has automatic storage duration, so it has
no linkage either. -/
def FrameRespecting (F : Nat → Prop) (e : World → World) : Prop :=
  ∀ w n, F n → (e w).value n = w.value n

/-- A call **does not bring `site` into existence**. The analyzer's call arm
adds only `produced_sites` to the presence set, and clears those sites' names
before re-establishing at most the freshly returned one, so a claim carried
across a call is never a claim about a site the call created. -/
def PreservesAbsence (site : Nat) (e : World → World) : Prop :=
  ∀ w, (e w).present site = true → w.present site = true

/-- The abstract transfer this card lands: keep exactly the framed names. -/
def callOut (F : Nat → Prop) (c : Claim) : Claim := fun n => c n ∧ F n

/-- **Preservation across a call that cannot write the name.** The card's
headline rule: a sound claim, restricted to the frame, is still sound after the
call. -/
theorem call_preserves_sound_claims (site : Nat) (F : Nat → Prop)
    (e : World → World) (frame : FrameRespecting F e)
    (absence : PreservesAbsence site e) (w : World) (c : Claim)
    (sound : soundAt site w c) :
    soundAt site (e w) (callOut F c) := by
  intro n hn hpresent
  obtain ⟨hc, hF⟩ := hn
  rw [frame w n hF]
  exact sound n hc (absence w hpresent)

/-- **The rule only ever weakens.** Whatever the frame, the call's outgoing
claim is contained in its incoming one, so no call can manufacture a name the
relation did not already carry. -/
theorem a_call_never_adds_a_name (F : Nat → Prop) (c : Claim) (n : Nat)
    (h : callOut F c n) : c n := h.1

/-- **Monotone in the refusing direction.** Recognising *fewer* names is always
available: a claim kept under the smaller frame is kept under the larger one, so
shrinking the frame — which is what an unrecognised place does — only weakens.
The analyzer builds `conditional_names` positively for exactly this reason. -/
theorem a_smaller_frame_claims_less (F G : Nat → Prop) (sub : ∀ n, F n → G n)
    (c : Claim) (n : Nat) (h : callOut F c n) : callOut G c n :=
  ⟨h.1, sub n h.2⟩

/-- **A name the call writes is outside every frame it respects.** This is the
fail-closed half, and it is why the destination, every `Out` slot and a
`Reallocated` parameter are forgotten before the frame test runs: if the call
changes what the name denotes anywhere, no frame containing it can be
respected. -/
theorem a_written_name_is_outside_every_frame (F : Nat → Prop)
    (e : World → World) (frame : FrameRespecting F e) (w : World) (n : Nat)
    (written : (e w).value n ≠ w.value n) : ¬ F n :=
  fun hF => written (frame w n hF)

/-- **Conservative extension, exactly.** At the empty frame the new rule *is*
the old one: nothing survives a call. The pre-card analyzer is the `F = ∅`
instance of this card's rule, so the change is a generalisation of the previous
behaviour and not a different rule placed beside it. -/
theorem the_empty_frame_is_the_old_blanket_forget (c : Claim) (n : Nat) :
    ¬ callOut (fun _ => False) c n := fun h => h.2

/-- Declining at a call stays sound, as the extreme case. -/
theorem the_empty_frame_stays_sound (site : Nat) (w : World) (c : Claim) :
    soundAt site w (callOut (fun _ => False) c) := by
  intro n hn
  exact hn.2.elim

/-- **Non-vacuity.** The rule genuinely parts from the old one: there is a
claim, a frame and a call under which the new transfer keeps a name the blanket
forget dropped. Without this the theorems above would hold of a rule that never
fires. -/
theorem the_frame_keeps_what_the_blanket_forget_dropped :
    ∃ (F : Nat → Prop) (c : Claim) (n : Nat),
      callOut F c n ∧ ¬ callOut (fun _ => False) c n :=
  ⟨fun _ => True, fun _ => True, 0, ⟨trivial, trivial⟩, fun h => h.2⟩

/-- **The new arm plugs into the fixed point unchanged.** A call edge whose
concrete effect respects the frame and creates no presence satisfies precisely
the `TransferSound` premise `fixed_point_is_sound` already demands. Neither the
acyclic pass's uniqueness theorem nor the cyclic soundness theorem is edited or
reproved: this card supplies a new way to discharge their existing hypothesis. -/
theorem a_call_edge_is_transfer_sound (g : Graph) (site : Nat) (p n : Node)
    (F : Nat → Prop)
    (frame : FrameRespecting F (g.transfer p))
    (absence : PreservesAbsence site (g.transfer p))
    (edge : g.abstractOut p n = callOut F) :
    TransferSound g site p n := by
  intro w c sound
  rw [edge]
  exact call_preserves_sound_claims site F (g.transfer p) frame absence w c sound

/-- The composition stated the other way round, so the report can cite one
theorem: a graph whose every edge is either a call edge of this shape or is
already transfer-sound derives only claims sound on every reachable path. -/
theorem a_graph_with_call_edges_is_sound
    (g : Graph) (w₀ : World) (site : Nat) (value : Node → Claim)
    (transfers : ∀ p n, TransferSound g site p n)
    (entry_sound : soundAt site w₀ (value g.entry))
    (fp : ∀ p n, n ∈ g.succ p → ∀ m, value n m → g.abstractOut p n (value p) m) :
    ∀ n w, Reach g w₀ n w → soundAt site w (value n) :=
  fixed_point_is_sound g w₀ site value transfers entry_sound fp

end QCore.JoinedProvenanceCallFrames
