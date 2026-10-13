import QCore.RequestConsumption

-- Card `HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING`: the verifier host's
-- decision on a provider's successful exit. Model of
-- `crates/qcore-verifier-host/src/request_consumption.rs` `classify` and the
-- decision branch of `ProcessProvider::run_process`.
--
-- Proved: the outcome is a function of consumption alone, so two schedules of
-- one consumption agree; a non-consuming provider is never decoded; a
-- consuming one is; a broken pipe and unread bytes are the same refusal; and
-- the legacy decision gave a non-reading provider two outcomes. Not proved, and
-- measured instead (docs/reports/host-unread-request-acceptance/README.md):
-- that FIONREAD on the probe reports the bytes no reader took, and that the
-- Rust function transcribes `decide` (pinned by its unit tests).

#print axioms QCore.RequestConsumption.decide_is_a_function_of_consumption
#print axioms QCore.RequestConsumption.the_outcome_does_not_depend_on_the_schedule
#print axioms QCore.RequestConsumption.a_non_consuming_provider_is_never_decoded
#print axioms QCore.RequestConsumption.a_consuming_provider_is_decoded
#print axioms QCore.RequestConsumption.a_broken_pipe_is_an_unconsumed_request
#print axioms QCore.RequestConsumption.an_unsuccessful_exit_is_terminated
#print axioms QCore.RequestConsumption.an_unobservable_consumption_is_refused
#print axioms QCore.RequestConsumption.the_legacy_decision_depended_on_the_schedule
