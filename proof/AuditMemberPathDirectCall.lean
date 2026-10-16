import QCore.MemberPathDirectCall

/-! Audited companion to FB-TEARDOWN-DESTROY-DECLARATIONS (gateway rule 30 at a direct call, contract specification C04.19.6 amended; decision 2 of 2026-10-16, the parameter guard): at a direct call a `gone` path is taken exactly once and cleared, a `stays` path is never taken, a `taken` (stale) path is taken once and still designated, so a later release is the second; a set and an unstated path never decide; a guard on a constant actual selects its arm, an unknown actual gives the cautious join of both arms, which never releases and decides only what every value decides. -/

#print axioms QCore.MemberPathDirectCall.the_direct_call_state_is_sound
#print axioms QCore.MemberPathDirectCall.a_release_after_taken_is_a_double_release
#print axioms QCore.MemberPathDirectCall.a_set_never_decides_at_a_direct_call
#print axioms QCore.MemberPathDirectCall.an_unstated_path_never_decides
#print axioms QCore.MemberPathDirectCall.a_constant_selects_its_arm
#print axioms QCore.MemberPathDirectCall.an_unknown_value_gives_the_join
#print axioms QCore.MemberPathDirectCall.an_unknown_value_never_releases
#print axioms QCore.MemberPathDirectCall.the_join_is_cautious
#print axioms QCore.MemberPathDirectCall.direct_vectors_hold
#print axioms QCore.MemberPathDirectCall.guard_vectors_hold
