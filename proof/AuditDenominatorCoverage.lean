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

import QCore.DenominatorCoverage

-- Unit `E01-TAIL`, residual `E01-F1-DENOM`: a whole-unit `Safe` claim needs the
-- whole selected scope to have been read. Model of the `Safe` arm of
-- `analyze_unrestricted_c11_with_inventory` in
-- `crates/qcore-c-gateway/src/legacy_c.rs`. Which definitions the analyzer's
-- body finder reports, and which the enumeration selects, are Rust boundaries
-- pinned by `crates/qcore-c-gateway/tests/e01_denominator_coverage.rs`.
-- Record: `docs/reports/remaining-e01-tail/README.md`.

-- The gate is exactly full coverage, in both directions.
#print axioms QCore.DenominatorCoverage.safe_is_published_exactly_when_clean_and_covered
#print axioms QCore.DenominatorCoverage.an_unread_definition_withholds_safe

-- The repair only ever refuses (hard invariant 1), and is not the identity.
#print axioms QCore.DenominatorCoverage.the_gate_only_refuses
#print axioms QCore.DenominatorCoverage.the_gate_is_not_vacuous
#print axioms QCore.DenominatorCoverage.only_the_uncovered_safe_arm_moves

-- The other two arms are untouched: an Unsafe finding is not retracted and an
-- analysis-derived Unknown keeps its own reason.
#print axioms QCore.DenominatorCoverage.an_unread_definition_does_not_retract_unsafe
#print axioms QCore.DenominatorCoverage.an_analysis_unknown_keeps_its_reason

-- A withheld claim costs the unit verdict and no per-function evidence.
#print axioms QCore.DenominatorCoverage.a_withheld_claim_retains_every_row

-- The two measured causes are refused; the covered control is still Safe.
#print axioms QCore.DenominatorCoverage.both_measured_causes_are_refused
#print axioms QCore.DenominatorCoverage.the_covered_control_is_still_safe

-- Relation to E01-F1's own rule: agreement on the shape it could see, and the
-- strict separation that is the residual itself.
#print axioms QCore.DenominatorCoverage.the_rules_agree_on_the_knr_cause
#print axioms QCore.DenominatorCoverage.the_coverage_gate_is_strictly_stronger
#print axioms QCore.DenominatorCoverage.the_gate_refuses_every_unanalyzable_definition
