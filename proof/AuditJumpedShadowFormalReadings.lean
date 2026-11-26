import QCore.JumpedShadowFormalReadings

/-! Card **JUMPED-SHADOW-FORMAL-READINGS** (gateway rule 86, extended): every
consumer of `CallerFormals` reads a formal or a derived local only where its
name is one declaration, and a formal that is not one refuses its function at
the entry. A `Safe` verdict under the extension was `Safe` under the census.
Which declaration a reference designates is read off Clang (trust boundary
28), pinned by `crates/qcore-c-gateway/tests/jumped_shadow_formal_readings.rs`. -/

-- The narrowed reading.
#print axioms QCore.JumpedShadowFormalReadings.local86_census
#print axioms QCore.JumpedShadowFormalReadings.reaches_narrows
#print axioms QCore.JumpedShadowFormalReadings.admits_narrows
#print axioms QCore.JumpedShadowFormalReadings.safe_narrows
-- Only refusals are added.
#print axioms QCore.JumpedShadowFormalReadings.no_verdict_becomes_safe
#print axioms QCore.JumpedShadowFormalReadings.a_shadowed_formal_refuses_at_entry
#print axioms QCore.JumpedShadowFormalReadings.a_keyed_event_needs_one_declaration
-- Witness programs.
#print axioms QCore.JumpedShadowFormalReadings.probe_cj1_was_safe_and_is_refused
#print axioms QCore.JumpedShadowFormalReadings.probe_cj5_was_safe_and_is_refused
#print axioms QCore.JumpedShadowFormalReadings.probe_wdj1_was_safe_and_is_refused
#print axioms QCore.JumpedShadowFormalReadings.the_controls_are_safe_under_both
