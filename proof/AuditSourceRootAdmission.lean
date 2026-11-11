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

import QCore.SourceRootAdmission

-- Card `G03-F1`: admission of a declared source root and the manifest version
-- that gates it. Model of `qcore_contract::profile_loader`'s
-- `is_admissible_source_root` and `source_root_path`. The component lists Rust's
-- `Path::components` yields, and the audit's other path validators, are Rust
-- boundaries recorded in `docs/reports/remaining-g03-f/README.md`.

-- Version 2 is a widening, monotone in the version, and it moves exactly one
-- spelling.
#print axioms QCore.SourceRootAdmission.version_two_admits_everything_version_one_does
#print axioms QCore.SourceRootAdmission.admission_is_monotone_in_the_version
#print axioms QCore.SourceRootAdmission.only_the_checkout_root_spelling_moved
#print axioms QCore.SourceRootAdmission.the_checkout_root_is_admissible_at_version_two
#print axioms QCore.SourceRootAdmission.the_checkout_root_is_refused_at_version_one
#print axioms QCore.SourceRootAdmission.exactly_one_corpus_spelling_separates_the_versions

-- No admitted root escapes the pinned checkout, and the new spelling resolves
-- to the checkout itself rather than to a `checkout/.`.
#print axioms QCore.SourceRootAdmission.a_relative_root_never_escapes
#print axioms QCore.SourceRootAdmission.an_admissible_root_never_escapes
#print axioms QCore.SourceRootAdmission.the_checkout_root_resolves_to_the_checkout
#print axioms QCore.SourceRootAdmission.an_ordinary_root_resolves_below_the_checkout
#print axioms QCore.SourceRootAdmission.no_admitted_corpus_root_escapes

-- Traversal, absolute and empty spellings keep their refusal at every supported
-- version; the refused corpus is inhabited, so that is not vacuous; and the
-- refusal is deliberately conservative rather than minimal.
#print axioms QCore.SourceRootAdmission.traversal_and_absolute_roots_are_refused_at_every_version
#print axioms QCore.SourceRootAdmission.the_refused_corpus_is_inhabited
#print axioms QCore.SourceRootAdmission.a_refused_root_may_still_be_harmless

-- Fail-closed in the version direction: an unsupported version admits nothing,
-- a higher number buys nothing, and a version-1 profile is unaffected.
#print axioms QCore.SourceRootAdmission.an_unsupported_version_admits_nothing
#print axioms QCore.SourceRootAdmission.a_higher_version_number_does_not_buy_the_checkout_root
#print axioms QCore.SourceRootAdmission.both_supported_versions_load_an_ordinary_root
#print axioms QCore.SourceRootAdmission.a_version_one_profile_is_unaffected
