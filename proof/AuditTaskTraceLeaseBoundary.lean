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

import QCore.TaskTraceLeaseBoundary

-- Card `TRACE-ALPHABET-LEASE-BOUNDARY`: the arm of
-- `qcore_core::task_trace_relation::reference_trace` that refuses card
-- `WRITE-LEND-TASKS`' two range-write-lease events — written at wave-31
-- integration to resolve a semantic conflict with `CT03-PROOF-ALPHABET`, and
-- recorded by the residual `TRACE-ALPHABET-LEASE-REFUSAL-UNEXERCISED` as
-- unproven and unexercised. CT03's conservativity theorems are about the
-- *observable* alphabet; this module is about the **observation** alphabet on
-- the other side of the same crossing. The exercise half of the discharge is
-- `compiler/tests/trace_alphabet_lease_boundary.rs` (five named tests, each
-- shown non-vacuous by mutation).

-- The vocabulary the module adds, printed so a later change to any of it is
-- visible in this surface rather than only inside a proof.
#print axioms QCore.TaskTraceLeaseBoundary.isLease
#print axioms QCore.TaskTraceLeaseBoundary.isRefusable
#print axioms QCore.TaskTraceLeaseBoundary.encodeEventAt
#print axioms QCore.TaskTraceLeaseBoundary.traceFrom
#print axioms QCore.TaskTraceLeaseBoundary.referenceTrace
#print axioms QCore.TaskTraceLeaseBoundary.Refusal.index

-- **The crossing is decided by the events alone, exactly**: a run encodes iff
-- none of its events is refusable. The forward direction is the one the
-- unexercised arm was asserting — no lease slips through, on any shape of run;
-- the backward direction is that the refusal blocks nothing else.
#print axioms QCore.TaskTraceLeaseBoundary.succeeds_iff_no_event_is_refusable
#print axioms QCore.TaskTraceLeaseBoundary.a_lease_carrying_observation_is_refused
#print axioms QCore.TaskTraceLeaseBoundary.no_lease_reaches_an_encoded_trace
#print axioms QCore.TaskTraceLeaseBoundary.encodeEventAt_refuses_a_lease
#print axioms QCore.TaskTraceLeaseBoundary.lease_is_refusable

-- **Refused, not silently dropped** — the formal content of the in-code claim.
-- On success the output is, position for position, the per-event encoding of
-- the input: nothing elided, nothing reordered, nothing invented.
#print axioms QCore.TaskTraceLeaseBoundary.encoding_preserves_length
#print axioms QCore.TaskTraceLeaseBoundary.encoding_is_positionwise

-- **The index is the offending event's own, and the first such.** The Rust pin
-- `UnsupportedObservedEvent { index: 2 }` on the `WRITE-LEND-TASKS` flagship is
-- an instance of this, not a coincidence.
#print axioms QCore.TaskTraceLeaseBoundary.encodeEventAt_error_index
#print axioms QCore.TaskTraceLeaseBoundary.refusal_points_at_a_refused_event
#print axioms QCore.TaskTraceLeaseBoundary.refusal_is_the_first

-- Non-vacuity in both directions on the card's real runs, in the manner of
-- CT03's `a_stale_read_is_refused` / `a_rewritten_payload_is_admitted`: the
-- flagship lease run is refused at its lease, and its read-share twin crosses
-- into an admitted causal order of the extended relation.
#print axioms QCore.TaskTraceLeaseBoundary.the_flagship_run_is_refused_at_its_lease
#print axioms QCore.TaskTraceLeaseBoundary.the_lease_free_twin_is_an_admitted_causal_order

-- **Why refusing and not dropping**, measured rather than asserted: the
-- lease-dropped remainder is not a lossy trace but an *admitted* one — an
-- admitted causal order for a run in which two workers held outstanding write
-- authority the order never mentions. Dropping would have been a move in the
-- admitting direction (invariant 1); refusing only blocks a comparison.
#print axioms QCore.TaskTraceLeaseBoundary.the_dropped_encoding_would_have_been_admitted
#print axioms QCore.TaskTraceLeaseBoundary.the_dropped_run_loses_exactly_the_four_lease_events
