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

import QCore.RequestConsumption

-- Cards `HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING` and
-- `HOST-REQUEST-READ-BY-DETACHED-DESCENDANT`: the verifier host's decision on
-- a provider's successful exit. Model of
-- `crates/qcore-verifier-host/src/request_consumption.rs` `classify` and
-- `holders`, and of the decision branch of `ProcessProvider::run_process`.
--
-- Proved: the outcome is a function of the holders at the decision and the
-- consumption alone, so two schedules that agree on both agree; a pipe another
-- process holds is never decoded; a non-consuming provider is never decoded; a
-- consuming one is, once no other process holds the pipe; a broken pipe and
-- unread bytes are the same refusal; the legacy decision gave a non-reading
-- provider two outcomes. Over time: the wait for the last holder decides from
-- the final consumption whenever the host checks, refuses a holder that
-- outlives the deadline, decodes nothing while the pipe is held, and an
-- immediate decision (on the count, or refusing a held pipe) would still have
-- depended on the schedule. These are the Linux host's. Off Linux, where the
-- holders cannot be observed and the host decides as before the card (the
-- user's decision): the decision is the count at the direct child's exit, and a
-- descendant that reads late still gets either outcome. Not proved, and
-- measured instead
-- (docs/reports/host-unread-request-acceptance/README.md,
-- docs/reports/host-request-detached-descendant/README.md): that FIONREAD
-- reports the bytes no reader took, that POLLERR on a reopened write end
-- reports that no other reader is left, and that the Rust functions
-- transcribe `decide` (pinned by their unit tests).

#print axioms QCore.RequestConsumption.decide_is_a_function_of_consumption_and_holders
#print axioms QCore.RequestConsumption.decide_is_a_function_of_consumption
#print axioms QCore.RequestConsumption.the_outcome_does_not_depend_on_the_schedule
#print axioms QCore.RequestConsumption.a_held_request_is_never_decoded
#print axioms QCore.RequestConsumption.a_non_consuming_provider_is_never_decoded
#print axioms QCore.RequestConsumption.a_consuming_provider_is_decoded
#print axioms QCore.RequestConsumption.a_broken_pipe_is_an_unconsumed_request
#print axioms QCore.RequestConsumption.an_unsuccessful_exit_is_terminated
#print axioms QCore.RequestConsumption.an_unobservable_consumption_is_refused
#print axioms QCore.RequestConsumption.the_legacy_decision_depended_on_the_schedule
#print axioms QCore.RequestConsumption.the_wait_decides_from_the_final_consumption
#print axioms QCore.RequestConsumption.the_wait_does_not_depend_on_when_the_host_checks
#print axioms QCore.RequestConsumption.a_holder_past_the_deadline_is_refused
#print axioms QCore.RequestConsumption.the_wait_decodes_only_after_the_release
#print axioms QCore.RequestConsumption.an_immediate_decision_would_still_depend_on_the_schedule
#print axioms QCore.RequestConsumption.off_linux_decides_on_the_count_at_the_decision
#print axioms QCore.RequestConsumption.off_linux_a_late_reader_still_depends_on_the_schedule
