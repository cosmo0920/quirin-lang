import QCore.JoinedProvenanceCallFrames

/-! Card `JOINED-PROVENANCE-CALL-FRAMES`: conditional provenance across a call.

Model of the `LegacyInstruction::ContractCall` arm of `Facts::step` in
`crates/qcore-c-gateway/src/legacy_joined_provenance.rs`, which before this card
cleared the binding map at every contracted call and now keeps the names the
call provably cannot write.

`call_preserves_sound_claims` is the rule itself. `a_call_never_adds_a_name` and
`a_smaller_frame_claims_less` are the direction invariant 1 governs: the
transfer is a restriction, so it can withdraw a name and can never manufacture
one, and an unrecognised place — a smaller frame — only weakens.

`a_written_name_is_outside_every_frame` is the fail-closed half, and it is the
theorem the Rust arm's ordering answers to: the destination and every `Out` slot
are removed by `rebound_names` before the frame test runs, and a `Reallocated`
parameter is forgotten in the arm itself, because a name the call writes cannot
be in any frame the call respects.

`the_empty_frame_is_the_old_blanket_forget` is the conservativity statement in
its exact form — the pre-card behaviour is this rule at `F = ∅` — and
`the_frame_keeps_what_the_blanket_forget_dropped` is the matching non-vacuity,
so the generalisation is shown to be a real one rather than a restatement.

`a_call_edge_is_transfer_sound` is the composition: the new arm discharges
precisely the `TransferSound` hypothesis `QCore.JoinedProvenanceCycles`'
`fixed_point_is_sound` already carries, so the acyclic uniqueness theorem and
the cyclic soundness theorem are reused unedited rather than reproved.

Premises, not conclusions, and both named in the card's report: that the token
reader's frame really is address-never-taken and automatic
(`CALL-FRAMES-FRAME-READER-PREMISE`), and the no-undefined-behaviour C object
model standing constraint 10a already rests on (`AOM-POISON-NO-UB`).
-/

#print axioms QCore.JoinedProvenanceCallFrames.call_preserves_sound_claims
#print axioms QCore.JoinedProvenanceCallFrames.a_call_never_adds_a_name
#print axioms QCore.JoinedProvenanceCallFrames.a_smaller_frame_claims_less
#print axioms QCore.JoinedProvenanceCallFrames.a_written_name_is_outside_every_frame
#print axioms QCore.JoinedProvenanceCallFrames.the_empty_frame_is_the_old_blanket_forget
#print axioms QCore.JoinedProvenanceCallFrames.the_empty_frame_stays_sound
#print axioms QCore.JoinedProvenanceCallFrames.the_frame_keeps_what_the_blanket_forget_dropped
#print axioms QCore.JoinedProvenanceCallFrames.a_call_edge_is_transfer_sound
#print axioms QCore.JoinedProvenanceCallFrames.a_graph_with_call_edges_is_sound
