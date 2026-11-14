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

import QCore.GuardCorrelatedCensus

-- Card TAIL-FILE-DEFINITION-CENSUS-INCOMPLETE (rule 49): the raw scan's
-- guard-correlation reading, modelling
-- `crates/qcore-c-gateway/src/legacy_guard_correlation.rs`'s
-- `correlated_source_scan`. The directive spellings `guard_of` accepts, the
-- report of every other directive as a reset, `_Pragma`, and the `extern "C"`
-- linkage braces are Rust boundaries recorded in
-- `docs/reports/tail-file-definition-census-incomplete/README.md`.

-- The assumption bookkeeping keeps the worlds covering every valuation.
#print axioms QCore.GuardCorrelatedCensus.a_split_covers_every_value
#print axioms QCore.GuardCorrelatedCensus.a_merge_only_weakens
#print axioms QCore.GuardCorrelatedCensus.a_clear_assumes_nothing

-- The measured `tag_compose` shape: refused by E01-RAW-COND, decided and
-- sound under guard correlation.
#print axioms QCore.GuardCorrelatedCensus.e01_raw_cond_refuses_the_measured_shape
#print axioms QCore.GuardCorrelatedCensus.the_measured_shape_is_decided
#print axioms QCore.GuardCorrelatedCensus.the_measured_shape_is_sound

-- Negatives: an expression guard or an `#elif` keeps the refusal; a reset
-- between the groups costs precision, not soundness; different guards are
-- followed in every configuration.
#print axioms QCore.GuardCorrelatedCensus.an_uncorrelated_shape_is_refused
#print axioms QCore.GuardCorrelatedCensus.an_elif_group_is_refused
#print axioms QCore.GuardCorrelatedCensus.a_reset_between_is_still_sound
#print axioms QCore.GuardCorrelatedCensus.a_reset_between_is_decided
#print axioms QCore.GuardCorrelatedCensus.different_guards_collect_every_configuration

-- The exhaustive domain: every stream of length at most four, sound whenever
-- decided, and the checksum the Rust side reproduces.
#print axioms QCore.GuardCorrelatedCensus.the_domain_is_exhaustive
#print axioms QCore.GuardCorrelatedCensus.the_domain_is_sound
#print axioms QCore.GuardCorrelatedCensus.the_domain_checksum_is_pinned
