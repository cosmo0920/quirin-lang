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

import QCore.ReasonCarrier

-- Card `GEN-REASON` (follow-ons `REASON-CODES-B1`/`B2`/`B3`): how a reason row
-- carries its closed L21.1 category. Model of the two carriers in
-- `crates/qcore-audit/src/report.rs` (`UnitRefusal::classified` /
-- `UnitRefusal::environment`, published through `reason_code_value`) and of
-- `crates/qcore-c-gateway/src/boolean_source.rs`'s
-- `ChunkSourceResult::unknown`. Which concrete Rust sites use which carrier is
-- a code census, recorded in `docs/reports/wave9-gen-reason/README.md`.

-- The text route is the unchanged classification relation: no second
-- vocabulary was introduced, and the site route publishes one fixed category.
#print axioms QCore.ReasonCarrier.the_text_route_is_the_unchanged_classification
#print axioms QCore.ReasonCarrier.the_site_route_publishes_environment_unavailable

-- Fail-closed on the text route is preserved: an unrecognised reason still
-- publishes no category, and in particular no source-subject one.
#print axioms QCore.ReasonCarrier.an_unrecognised_text_still_publishes_no_category
#print axioms QCore.ReasonCarrier.an_unrecognised_text_publishes_no_source_statement

-- L21.1 rule 1 across the site route: a site-named row says nothing about the
-- analysed source, never supports a provisional Unsafe, and its subject is the
-- environment.
#print axioms QCore.ReasonCarrier.the_site_route_never_states_anything_about_the_source
#print axioms QCore.ReasonCarrier.the_site_route_never_supports_unsafe
#print axioms QCore.ReasonCarrier.the_site_route_speaks_about_the_environment

-- The safety argument for adding the second carrier: naming at the site is a
-- narrowing, not a widening, in both halves of rule 1.
#print axioms QCore.ReasonCarrier.naming_at_the_site_never_widens_a_source_claim
#print axioms QCore.ReasonCarrier.naming_at_the_site_never_widens_unsafe_support

-- Publication is a function, which is what lets a producer-supplied category be
-- republished rather than reclassified; and the two carriers agree wherever
-- both decide the same fact.
#print axioms QCore.ReasonCarrier.publication_is_single_valued
#print axioms QCore.ReasonCarrier.the_carriers_agree_on_environment_text

-- Non-vacuity, so the refusal theorems above are not proved of an empty set.
#print axioms QCore.ReasonCarrier.the_text_route_can_publish_a_source_statement
#print axioms QCore.ReasonCarrier.the_site_route_publishes_something

-- Totality, and the fact that only a refused text can leave a row uncoded.
#print axioms QCore.ReasonCarrier.publication_is_total
#print axioms QCore.ReasonCarrier.only_a_refused_text_publishes_nothing
