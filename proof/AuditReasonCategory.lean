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

import QCore.ReasonCategory

-- Card `REASON-CODES`: the closed L21.1 reason-code vocabulary. Model of
-- `crates/qcore-schemas/src/reason.rs`. The two carriers themselves —
-- `DerivationIncomplete::category` and the audit report's `reason_code` — and
-- the agreement between the Rust marker table and this relation are Rust
-- boundaries, recorded in `docs/reports/remaining-reason-codes/README.md`.

-- The vocabulary is closed and its enumeration is complete and duplicate-free.
#print axioms QCore.ReasonCategory.all_is_complete
#print axioms QCore.ReasonCategory.all_has_no_duplicates
#print axioms QCore.ReasonCategory.spellings_are_pairwise_distinct
#print axioms QCore.ReasonCategory.Family.all_is_complete

-- §L21.1 rule 1, structurally: the subject partition, and the fact that
-- `environment-unavailable` is its sole environment member.
#print axioms QCore.ReasonCategory.environment_subject_is_inhabited_only_by_environment_unavailable
#print axioms QCore.ReasonCategory.no_category_is_both_environment_and_source
#print axioms QCore.ReasonCategory.environment_unavailable_says_nothing_about_the_source
#print axioms QCore.ReasonCategory.an_environment_row_never_supports_unsafe
#print axioms QCore.ReasonCategory.every_subject_is_reached
#print axioms QCore.ReasonCategory.exactly_two_categories_describe_the_analysed_source

-- Card W45-USER-CONTRACT-PLUGINS: the three human-resolvable categories, the
-- only reasons that may make a row `Pending`, never support `Unsafe`.
#print axioms QCore.ReasonCategory.a_human_resolvable_category_never_supports_unsafe
#print axioms QCore.ReasonCategory.exactly_four_categories_are_human_resolvable

-- The mapping over the real reason corpus: total, single-valued, non-vacuous.
#print axioms QCore.ReasonCategory.the_mapping_is_total
#print axioms QCore.ReasonCategory.every_category_is_reached
#print axioms QCore.ReasonCategory.classification_is_single_valued
#print axioms QCore.ReasonCategory.every_known_input_classifies

-- §L21.1 rule 1 over that corpus, in both directions: an environment family is
-- never relabeled a source statement, and the category is not over-applied.
#print axioms QCore.ReasonCategory.an_environment_family_never_becomes_a_source_statement
#print axioms QCore.ReasonCategory.only_an_environment_family_is_labeled_environment
#print axioms QCore.ReasonCategory.an_environment_family_never_supports_unsafe
#print axioms QCore.ReasonCategory.a_source_statement_has_a_recognised_non_environment_origin

-- Fail-closed: refusal yields no category at all, never a defaulted one.
#print axioms QCore.ReasonCategory.an_unrecognised_reason_is_refused
#print axioms QCore.ReasonCategory.refusal_never_yields_a_source_statement
