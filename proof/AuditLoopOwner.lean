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

import QCore.LoopOwner
import QCore.LoopOwnerCoroPlugin

/-! Card **W47-LOOP-OWNER-CONTRACTS** (gateway rule 10p, contract
specification §C04.9): the loop-owner obligation over derived thread
identity.

Four directions are audited.

**Derived ownership.** An edge keeps the thread; a loop dispatch fixes the
mode and runs the callback on the dispatcher's thread, which is the loop's
owner; an unresolved dispatcher has no owner; an infeasible threaded-flag
branch is pruned.

**Discharge only on the owner.** A discharged site has no unresolved path, a
derived caller, and every context on its mode's single owner; one resolved
non-owner context refuses; the per-unit route derives no thread and refuses
every obligated call; only a discharge admits.

**No concurrent mutator.** A discharge derives the Cycle 18 single-dispatcher
premise (`EventDriven.Execution.sameBaseSingleDispatcher`) for this route
instead of assuming it, leaves no conflicting pair of accesses, and a foreign
access conflicts with the dispatcher's own walk (the reactor's L05).

**C2 and C3.** An enqueue never discharges "applied"; an unfixed mode claims
nothing; the paired backend collectors' state is a boundary claim only on the
owner with both results succeeded, and never for a resume without active
ingestion.

**Hop C** (card W47-LOOP-OWNER-COROUTINE-HOP). A coroutine body runs on its
derived resumers' threads; one resumed only on the owner runs on the owner;
one non-owner resumer refuses; an undetermined resumer, a body's continuation
after a yield included, never discharges. An input whose derived descriptor
flags run no coroutine is not dispatched through the hop; an underived flag
keeps it. The configured reading keeps the arm any agreeing build takes, drops
a decided-false arm, and reads nothing after a decided-true arm.

**Typed dispatch** (card W47-LOOP-OWNER-TYPED-DISPATCH). Type-keyed
dispatch is a sub-relation of field-keyed dispatch, equal to it where no typed
fact exists; a call through record `T` reaches no registrant stored only in
another record; every removed edge is cross-type or belongs to an unmatched
dispatch, whose registrants are unresolved and never discharged. A runtime
plugin copy's instance drops the coroutine route only when both the creating
plugin's flag and the copy's derive none; a copy written with the coroutine
flag, or an underived part, keeps it.

**The list-head instance** (card W47-LOOP-OWNER-INSTANCE-FLOW). From a fresh
context's empty input list, a path that creates exactly once and mutates the
list in no unmodelled way leaves exactly the created instance, so a derived
head is the one creation on every path; a second creation, an unmodelled
mutation or no creation leaves the head underived and keeps the coroutine
route. An escaping holder keeps the derivation only when what it reaches
writes neither the instance's flag nor which instance runs the callback; the
route is dropped only through one non-coroutine creation, such an escape and a
non-coroutine copy. The old test's resolved test-thread context is never
discharged.

**The list-head instance's premises** (card W47-PREMISES-INSTANCE). A
derived config initializer leaves the empty list the list-head derivation
starts from, and one without its list initialisation derives none; a
creation over a read registration derives a flag only with a registered
match, every instance it makes then runs no coroutine, and an unread
registration keeps the route; a straight-line constructor block either
returns a null pointer or appends, so a non-null creation is on the list,
while a failed one leaves the fresh list empty (the stated
P-CREATIONS-SUCCEED); a collector written only by its constructor names its
registration's instance; a refuted P-INSTANCE is never discharged, the
premised verdict never adds a discharge, and the old test's path stays
refused; a byte or whole-object write keeps the route.

**Predefined-macro arms** (card W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS). A
reserved macro decided from the recorded compiler, per command, agrees with
its definedness at every point of every unit of the file, so it behaves as an
owned macro; a condition's three-valued reading agrees with the
preprocessor's value; an arithmetic comparison is never evaluated and keeps
its arm; an absent derivation (a compiler that is unavailable or not the
recorded one) decides nothing and keeps every arm; a `-U` after a `-D`
undefines; a file's own directive keeps the opposite decision from being
made; the old test's path is refused whatever arms a reading keeps.

The lexical census reading, the hop tables, P-INSTANCE and the Rust
correspondence are trust boundary 28's premises, pinned by
`crates/qcore-core/tests/unit/loop_owner.rs`,
`crates/quirin-source-analysis/tests/loop_owner_census.rs` and
`crates/quirin-source-analysis/tests/loop_owner_coroutine.rs` and
`crates/quirin-source-analysis/tests/loop_owner_typed_dispatch.rs` and
`crates/quirin-source-analysis/tests/loop_owner_instance_flow.rs` and
`crates/quirin-source-analysis/tests/loop_owner_predefined_macros.rs`. -/

-- Derived ownership.
#print axioms QCore.LoopOwner.a_step_keeps_the_thread
#print axioms QCore.LoopOwner.a_dispatched_callback_runs_on_the_owner
#print axioms QCore.LoopOwner.a_worker_callback_runs_on_the_owner
#print axioms QCore.LoopOwner.an_unresolved_dispatcher_has_no_owner
#print axioms QCore.LoopOwner.an_infeasible_branch_is_pruned

-- Discharge only on the owner.
#print axioms QCore.LoopOwner.unresolved_is_never_discharged
#print axioms QCore.LoopOwner.no_derived_caller_is_never_discharged
#print axioms QCore.LoopOwner.context_discharged_on_owner
#print axioms QCore.LoopOwner.discharged_means_every_context_runs_on_its_owner
#print axioms QCore.LoopOwner.a_non_owner_context_refuses
#print axioms QCore.LoopOwner.the_per_unit_route_refuses_every_obligated_call
#print axioms QCore.LoopOwner.only_a_discharge_admits

-- No concurrent mutator.
#print axioms QCore.LoopOwner.discharged_accesses_run_on_the_owner
#print axioms QCore.LoopOwner.the_owner_route_derives_the_dispatcher_premise
#print axioms QCore.LoopOwner.a_discharged_loop_has_no_concurrent_mutator
#print axioms QCore.LoopOwner.a_foreign_access_conflicts_with_the_dispatcher

-- C2 and C3.
#print axioms QCore.LoopOwner.an_enqueue_never_applies
#print axioms QCore.LoopOwner.a_threaded_control_only_enqueues
#print axioms QCore.LoopOwner.an_unfixed_mode_claims_nothing
#print axioms QCore.LoopOwner.a_boundary_claim_needs_both_results_and_the_owner
#print axioms QCore.LoopOwner.a_discarded_result_gives_no_conclusion
#print axioms QCore.LoopOwner.a_resume_without_ingestion_is_refused

-- Hop C: a coroutine runs on the thread that resumes it.
#print axioms QCore.LoopOwner.a_coroutine_runs_on_its_resumers
#print axioms QCore.LoopOwner.a_coroutine_resumed_only_on_the_owner_runs_on_the_owner
#print axioms QCore.LoopOwner.a_non_owner_resumer_refuses
#print axioms QCore.LoopOwner.an_undetermined_resumer_never_discharges
#print axioms QCore.LoopOwner.a_body_that_may_yield_is_never_discharged
#print axioms QCore.LoopOwner.the_dispatcher_resumes_on_its_own_thread
#print axioms QCore.LoopOwner.an_input_without_a_coroutine_is_not_dispatched_by_the_hop
#print axioms QCore.LoopOwner.an_underived_flag_keeps_the_hop

-- The configured reading of a conditional group.
#print axioms QCore.LoopOwner.taken_within_kept
#print axioms QCore.LoopOwner.the_taken_arm_is_read
#print axioms QCore.LoopOwner.a_decided_false_arm_is_not_read
#print axioms QCore.LoopOwner.nothing_after_a_decided_arm_is_read

-- Typed dispatch: a sub-relation that only removes cross-type edges.
#print axioms QCore.LoopOwner.type_keyed_is_a_sub_relation
#print axioms QCore.LoopOwner.an_untyped_dispatch_is_field_keyed
#print axioms QCore.LoopOwner.a_call_through_a_record_reaches_only_its_registrants
#print axioms QCore.LoopOwner.a_removed_edge_is_cross_type_or_refused
#print axioms QCore.LoopOwner.an_unmatched_dispatch_never_discharges

-- Step B: the instance flag of a runtime plugin copy.
#print axioms QCore.LoopOwner.no_coroutine_needs_both
#print axioms QCore.LoopOwner.a_coroutine_copy_keeps_the_route
#print axioms QCore.LoopOwner.an_underived_part_keeps_the_route
#print axioms QCore.LoopOwner.the_route_is_dropped_only_when_both_derive_none

-- The list-head instance, the holder escape and the old test's path
-- (card W47-LOOP-OWNER-INSTANCE-FLOW).
#print axioms QCore.LoopOwner.a_derived_head_is_the_one_creation
#print axioms QCore.LoopOwner.every_path_reads_the_one_creation
#print axioms QCore.LoopOwner.a_second_creator_keeps_the_route
#print axioms QCore.LoopOwner.an_unmodelled_mutation_keeps_the_route
#print axioms QCore.LoopOwner.a_path_without_a_creation_derives_nothing
#print axioms QCore.LoopOwner.an_underived_head_keeps_the_route
#print axioms QCore.LoopOwner.an_escape_that_writes_neither_keeps_the_derivation
#print axioms QCore.LoopOwner.an_escape_that_writes_the_flag_or_the_instance_keeps_the_route
#print axioms QCore.LoopOwner.the_route_is_dropped_only_through_one_non_coroutine_creation
#print axioms QCore.LoopOwner.the_old_test_path_is_never_discharged

-- The keys a property setter carries (card W47-SERVICE-PROPERTY-KEYS).
#print axioms QCore.LoopOwner.literalKeys_cons_str
#print axioms QCore.LoopOwner.a_read_key_list_is_null_terminated_and_paired
#print axioms QCore.LoopOwner.an_unreadable_argument_keeps_every_branch
#print axioms QCore.LoopOwner.a_non_literal_key_is_unreadable
#print axioms QCore.LoopOwner.an_odd_list_is_unreadable
#print axioms QCore.LoopOwner.an_unterminated_list_is_unreadable
#print axioms QCore.LoopOwner.a_key_absent_from_the_table_keeps_every_branch
#print axioms QCore.LoopOwner.a_literal_key_sequence_reaches_only_its_keys_effects
#print axioms QCore.LoopOwner.the_key_reading_covers_the_run
#print axioms QCore.LoopOwner.a_matched_hazard_stays_read
#print axioms QCore.LoopOwner.a_case_variant_matches_the_same_entries
#print axioms QCore.LoopOwner.a_key_reach_holding_a_hazard_keeps_the_route
#print axioms QCore.LoopOwner.a_key_reach_without_a_hazard_leaves_the_one_creation

-- Predefined-macro arms (card W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS).
#print axioms QCore.LoopOwner.definedAfter_without_undef
#print axioms QCore.LoopOwner.definedAfter_without_define
#print axioms QCore.LoopOwner.a_predefined_macro_behaves_as_an_owned_macro
#print axioms QCore.LoopOwner.a_compiler_mismatch_decides_nothing
#print axioms QCore.LoopOwner.a_minus_u_undefines
#print axioms QCore.LoopOwner.a_file_directive_wins
#print axioms QCore.LoopOwner.agrees_neg
#print axioms QCore.LoopOwner.agrees_conj
#print axioms QCore.LoopOwner.the_condition_reading_agrees_with_the_preprocessor
#print axioms QCore.LoopOwner.keptArms_unknown_head
#print axioms QCore.LoopOwner.an_undecidable_comparison_keeps_the_arm
#print axioms QCore.LoopOwner.readCond_without_decisions
#print axioms QCore.LoopOwner.keptArms_all_unknown
#print axioms QCore.LoopOwner.a_compiler_mismatch_keeps_every_arm
#print axioms QCore.LoopOwner.the_old_test_path_is_never_discharged_whatever_the_arms

-- P-CORO-HANDLE, P-PLUGIN-DIR and the immutable threaded flag derived (card
-- W47-PREMISES-CORO-PLUGIN).
#print axioms QCore.LoopOwner.CoroPlugin.a_confined_handle_has_only_derived_resumers
#print axioms QCore.LoopOwner.CoroPlugin.an_escaping_handle_never_discharges
#print axioms QCore.LoopOwner.CoroPlugin.a_use_outside_the_set_escapes
#print axioms QCore.LoopOwner.CoroPlugin.a_census_discharge_covers_every_possible_resumer
#print axioms QCore.LoopOwner.CoroPlugin.a_parameter_chain_keeps_the_directory
#print axioms QCore.LoopOwner.CoroPlugin.a_runtime_flags_write_keeps_the_route
#print axioms QCore.LoopOwner.CoroPlugin.a_registration_on_another_plugins_instance_keeps_the_route
#print axioms QCore.LoopOwner.CoroPlugin.a_derived_directory_drops_the_route
#print axioms QCore.LoopOwner.CoroPlugin.valueAt_before_all
#print axioms QCore.LoopOwner.CoroPlugin.valueAt_after_all
#print axioms QCore.LoopOwner.CoroPlugin.no_write_after_start_fixes_the_flag
#print axioms QCore.LoopOwner.CoroPlugin.another_literal_key_cannot_write_the_flag
#print axioms QCore.LoopOwner.CoroPlugin.a_runtime_threaded_write_never_discharges
#print axioms QCore.LoopOwner.CoroPlugin.the_old_test_path_is_never_discharged_whatever_the_premises
-- The list-head instance's premises (card W47-PREMISES-INSTANCE).
#print axioms QCore.LoopOwner.initStep_initList
#print axioms QCore.LoopOwner.initStep_other
#print axioms QCore.LoopOwner.initStep_others
#print axioms QCore.LoopOwner.a_derived_fresh_context_is_empty
#print axioms QCore.LoopOwner.initStep_none_without_init
#print axioms QCore.LoopOwner.an_initializer_without_its_list_init_is_not_fresh
#print axioms QCore.LoopOwner.a_fresh_context_starts_the_head_derivation
#print axioms QCore.LoopOwner.an_unread_registration_derives_nothing
#print axioms QCore.LoopOwner.a_derived_registration_has_a_match
#print axioms QCore.LoopOwner.every_created_instance_runs_no_coroutine
#print axioms QCore.LoopOwner.an_unread_registration_keeps_the_route
#print axioms QCore.LoopOwner.a_straight_block_appends_or_returns_null
#print axioms QCore.LoopOwner.a_non_null_creation_is_on_the_list
#print axioms QCore.LoopOwner.listStep_others
#print axioms QCore.LoopOwner.a_failed_creation_leaves_the_fresh_list_empty
#print axioms QCore.LoopOwner.a_constructor_only_collector_names_its_registration
#print axioms QCore.LoopOwner.a_repointed_collector_may_name_another
#print axioms QCore.LoopOwner.a_refuted_instance_is_never_discharged
#print axioms QCore.LoopOwner.the_premises_never_add_a_discharge
#print axioms QCore.LoopOwner.a_derived_premise_keeps_the_verdict
#print axioms QCore.LoopOwner.the_old_test_path_is_never_discharged_under_the_premises
#print axioms QCore.LoopOwner.allowed_member_writes_keep_the_derivation
#print axioms QCore.LoopOwner.a_non_member_write_keeps_the_route
-- The list-head region's macro invocations and the holder's opacity (card
-- W47-PREMISES-HOLDER-REGION).
#print axioms QCore.LoopOwner.an_unexpandable_invocation_refuses
#print axioms QCore.LoopOwner.a_loud_expansion_refuses
#print axioms QCore.LoopOwner.created_quiet
#print axioms QCore.LoopOwner.mutates_quiet
#print axioms QCore.LoopOwner.created_append
#print axioms QCore.LoopOwner.mutates_append
#print axioms QCore.LoopOwner.the_region_reading_derives_what_the_expansion_runs
#print axioms QCore.LoopOwner.a_derived_region_head_is_the_programs_one_creation
#print axioms QCore.LoopOwner.a_region_without_invocations_reads_lexically
#print axioms QCore.LoopOwner.the_included_definition_is_the_one_read
#print axioms QCore.LoopOwner.an_opaque_holder_writes_nothing_the_route_reads
#print axioms QCore.LoopOwner.an_opaque_holder_keeps_the_derivation
#print axioms QCore.LoopOwner.a_carrier_that_writes_keeps_the_route
#print axioms QCore.LoopOwner.a_data_reader_that_writes_the_flag_keeps_the_route
#print axioms QCore.LoopOwner.the_route_is_dropped_only_through_read_macros_and_an_opaque_holder
#print axioms QCore.LoopOwner.the_old_test_path_is_never_discharged_whatever_the_region
#print axioms QCore.LoopOwner.a_passing_root_reaches_a_hazard_only_as_counted
#print axioms QCore.LoopOwner.reachesWithin_more_edges
#print axioms QCore.LoopOwner.a_root_reaching_only_through_a_macro_call_refuses
#print axioms QCore.LoopOwner.the_holder_instance_runs_its_copy
