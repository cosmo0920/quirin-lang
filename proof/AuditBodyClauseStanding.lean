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

import QCore.BodyClauseStanding

/-! Audited companion to VERDICT-BODY-STANDING-CLAUSE-SCOPE: per-clause body standing, the clauses each piece of body evidence covers, and the Safe roll-up's validated-only subset (gateway rules 13i-13k). -/

#print axioms QCore.BodyClauseStanding.a_structural_check_covers_only_what_it_reads
#print axioms QCore.BodyClauseStanding.a_structural_check_never_covers_a_formal
#print axioms QCore.BodyClauseStanding.nothing_covers_these_clauses
#print axioms QCore.BodyClauseStanding.a_held_declaration_is_never_validated
#print axioms QCore.BodyClauseStanding.standing_eq_validated
#print axioms QCore.BodyClauseStanding.standing_eq_partly
#print axioms QCore.BodyClauseStanding.validated_needs_every_clause
#print axioms QCore.BodyClauseStanding.an_uncovered_clause_is_never_validated
#print axioms QCore.BodyClauseStanding.a_partly_validated_declaration_has_an_uncovered_clause
#print axioms QCore.BodyClauseStanding.validated_needs_the_evidence_standing
#print axioms QCore.BodyClauseStanding.validated_implies_previously_validated
#print axioms QCore.BodyClauseStanding.a_declaration_with_a_premise_is_never_validated
#print axioms QCore.BodyClauseStanding.structural_evidence_alone_never_validates_a_formal
#print axioms QCore.BodyClauseStanding.the_validated_only_rollup_is_a_subset
#print axioms QCore.BodyClauseStanding.coverage_vectors_hold
#print axioms QCore.BodyClauseStanding.clause_standing_vectors_hold

#print axioms QCore.BodyClauseStanding.legacy_evidence_never_validates_a_lifetime_protocol
