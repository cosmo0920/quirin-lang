import QCore.CallbackTaskHandoff

open QCore.CallbackTaskHandoff

#print axioms borrowed_frame_cannot_be_retained
#print axioms dropped_argument_identity_cannot_be_reused
#print axioms owned_argument_identity_is_exclusive
#print axioms shared_argument_identity_can_be_shared
#print axioms unknown_argument_cannot_be_dropped
#print axioms undropped_owned_argument_remains_explicit_custody
#print axioms returned_frame_borrow_is_not_owned_custody
#print axioms cancellation_request_is_not_joinable
#print axioms cancellation_acknowledgement_can_be_joined
#print axioms running_task_blocks_teardown
#print axioms registered_deferred_work_blocks_teardown
#print axioms post_return_join_and_deferred_completion_are_quiescent
#print axioms callback_exit_alone_does_not_discharge_handoff
#print axioms spawned_task_cannot_skip_start
#print axioms registered_deferred_work_cannot_skip_start
#print axioms cancellation_completion_race_is_outside_profile
#print axioms taskful_trace_constructs_exact_certificate
#print axioms duplicate_callback_identity_refuses
#print axioms duplicate_task_identity_refuses
#print axioms duplicate_deferred_identity_refuses
#print axioms wrong_task_owner_refuses
#print axioms retained_argument_cannot_drop
#print axioms borrowed_frame_cannot_drop_after_return
#print axioms teardown_requires_join_and_completion
#print axioms undropped_argument_is_in_constructed_certificate
#print axioms remaining_custody_is_canonical_by_argument_identity

example : taskStep .running .cancelRequested = true := by decide
example : taskStep .cancelRequested .joined = false := by decide
example : deferredStep .running .completed = true := by decide
example : quiescent ⟨false, [], []⟩ = false := by decide
example : checkTrace taskfulAcceptedTrace = some ⟨[1], [10], [20], [100], []⟩ := by decide
example : run [.callbackEnter 1 100 .borrowedFrame, .callbackExit 1,
  .dropArgument 100] = none := by decide
