import QCore.MemberRelease

/-! Card **W45-CALLEE-SUMMARY-CASCADE**: the destructor-body family of corpus
callees (`crates/qcore-c-gateway/src/legacy_member_release.rs`).

Four directions are audited. **Rule 1 admits only a pointer read at a nullable
release**: a non-null release, a consuming position, a scalar value and a read
with no dereferenced base never admit. **Its lowering releases no tracked
site**: under the projection's construction premise, a read whose second
obligation passes denotes no tracked identity, and reading its position as
`Borrowed` is then exactly the call's effect on every tracked site; a linked
owner or a bound place refuses. **A member null test is a read**: it changes
no binding, and a declined one keeps the refusal. **The guard**: a passing
guard leaves no caller-tracked site reachable through an actual, it refuses
every owner of sub-custody, every body whose summary chain reaches a rule-1
call is guarded, and inserting it keeps every node's identity and routes every
edge into the call through it. Witnesses complete it.

What this is not: a proof about the Rust readers (trust boundary 28), or of
the two construction premises it names, which are stated as hypotheses. -/

-- Rule 1: admission.
#print axioms QCore.MemberRelease.only_a_nullable_release_admits
#print axioms QCore.MemberRelease.a_non_null_release_never_admits
#print axioms QCore.MemberRelease.a_consuming_position_never_admits
#print axioms QCore.MemberRelease.a_scalar_value_never_admits
#print axioms QCore.MemberRelease.a_baseless_read_never_admits

-- Rule 1: the lowering releases no tracked site.
#print axioms QCore.MemberRelease.the_rewrite_changes_no_tracked_effect
#print axioms QCore.MemberRelease.an_admitted_release_names_no_tracked_identity
#print axioms QCore.MemberRelease.an_admitted_member_release_releases_no_tracked_site
#print axioms QCore.MemberRelease.a_linked_owner_refuses_the_release
#print axioms QCore.MemberRelease.a_bound_place_refuses_the_release

-- Rule 2: a member null test is a read.
#print axioms QCore.MemberRelease.a_member_null_test_changes_no_binding
#print axioms QCore.MemberRelease.a_declined_null_test_keeps_the_refusal
#print axioms QCore.MemberRelease.an_admitted_null_test_obliges_its_base

-- Rule 3: the guard.
#print axioms QCore.MemberRelease.a_passing_guard_hides_no_caller_release
#print axioms QCore.MemberRelease.the_guard_refuses_an_owner_of_sub_custody
#print axioms QCore.MemberRelease.the_guard_is_appended
#print axioms QCore.MemberRelease.the_guard_leads_to_the_call
#print axioms QCore.MemberRelease.every_existing_node_keeps_its_id
#print axioms QCore.MemberRelease.every_edge_into_the_call_passes_the_guard
#print axioms QCore.MemberRelease.every_summary_chain_to_a_member_release_is_guarded

-- Witnesses.
#print axioms QCore.MemberRelease.the_witness_admissions
#print axioms QCore.MemberRelease.the_witness_insertion
