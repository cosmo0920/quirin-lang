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

import QCore.CleanupVocabulary

-- Card `P04-B1`: the cleanup-outcome contracts' declared referent vocabulary.
-- Model of `crates/qcore-verification/src/cleanup_vocabulary.rs`.
--
-- Two boundaries stay outside every theorem below and are recorded rather than
-- modelled: that `String::starts_with`/`split("__")` realise the segmentation
-- the spelling theorems constrain, and — decisively — that a declared name is
-- *bound by the pinned C source*, which is decided by the existing replay in
-- `crates/qcore-verification/src/cleanup_outcome.rs` over a real Clang import
-- and is measured in `crates/qcore-verification/tests/cleanup_vocabulary.rs`,
-- not proved here. Declaring `close`, `munmap` or `unlink` remains a statement
-- about a name, never about a descriptor, a mapping or a file.

-- The spelling layer: a prefix code with distinct, separator-free tokens.
#print axioms QCore.CleanupVocabulary.the_prefixes_are_pairwise_incomparable
#print axioms QCore.CleanupVocabulary.the_prefixes_are_distinct
#print axioms QCore.CleanupVocabulary.every_prefix_ends_with_the_separator
#print axioms QCore.CleanupVocabulary.the_scalar_tokens_are_pairwise_distinct
#print axioms QCore.CleanupVocabulary.the_scalar_spellings_are_pairwise_distinct
#print axioms QCore.CleanupVocabulary.every_scalar_token_is_separator_free
#print axioms QCore.CleanupVocabulary.no_scalar_token_looks_like_a_record
#print axioms QCore.CleanupVocabulary.every_reset_and_index_token_is_separator_free

-- Total or reject at the spelling layer: a payload the grammar cannot express
-- has no declaration at all, rather than an approximated one.
#print axioms QCore.CleanupVocabulary.a_name_carrying_the_separator_has_no_spelling
#print axioms QCore.CleanupVocabulary.a_record_tag_carrying_the_separator_has_no_token
#print axioms QCore.CleanupVocabulary.a_record_tag_starting_with_a_digit_has_no_token
#print axioms QCore.CleanupVocabulary.an_empty_payload_has_no_spelling
#print axioms QCore.CleanupVocabulary.an_unspellable_index_is_refused

-- Non-vacuity: the real clauses spell exactly as the pinned contracts write
-- them, including the verbatim `__errno_location` payload.
#print axioms QCore.CleanupVocabulary.the_errno_helper_spells_verbatim
#print axioms QCore.CleanupVocabulary.the_pinned_clauses_spell_as_the_contracts_write_them

-- The clause layer: reading is single-valued and fails closed, and binding
-- admits exactly the declared shape.
#print axioms QCore.CleanupVocabulary.a_declaration_never_yields_two_shapes
#print axioms QCore.CleanupVocabulary.an_unreadable_declaration_refuses_every_profile
#print axioms QCore.CleanupVocabulary.binding_admits_exactly_the_declared_shape
#print axioms QCore.CleanupVocabulary.a_declaration_without_the_marker_is_refused
#print axioms QCore.CleanupVocabulary.a_repeated_marker_is_refused

-- Bounded over an exhaustively enumerated domain whose size and variety are
-- pinned: spelling round-trips, is injective, and binds one profile per
-- declaration; and the marker, a duplicated entry, an index gap and a second
-- eligibility field each refuse across the whole domain.
#print axioms QCore.CleanupVocabulary.the_sample_domain_is_inhabited_and_varied
#print axioms QCore.CleanupVocabulary.spelling_round_trips_on_the_sample_domain
#print axioms QCore.CleanupVocabulary.spelling_is_injective_on_the_sample_domain
#print axioms QCore.CleanupVocabulary.one_declaration_binds_one_profile_on_the_sample_domain
#print axioms QCore.CleanupVocabulary.dropping_the_marker_refuses_the_whole_domain
#print axioms QCore.CleanupVocabulary.duplicating_a_field_clause_refuses_the_whole_domain
#print axioms QCore.CleanupVocabulary.an_inventory_gap_refuses_the_whole_domain
#print axioms QCore.CleanupVocabulary.two_mapping_clauses_are_refused_on_the_whole_domain

-- The real unmap policy: every profile member is load-bearing, the reset
-- values and the syscall argument order included.
#print axioms QCore.CleanupVocabulary.the_pinned_unmap_shape_reads_back
#print axioms QCore.CleanupVocabulary.swapping_the_syscall_arguments_is_refused
#print axioms QCore.CleanupVocabulary.changing_a_reset_value_is_refused
#print axioms QCore.CleanupVocabulary.resizing_the_inventory_is_refused
#print axioms QCore.CleanupVocabulary.the_mapping_field_is_declared_exactly_when_present
#print axioms QCore.CleanupVocabulary.retyping_a_field_is_refused
#print axioms QCore.CleanupVocabulary.the_named_referents_are_load_bearing
#print axioms QCore.CleanupVocabulary.the_pinned_unmap_declaration_is_spellable
#print axioms QCore.CleanupVocabulary.the_pinned_unmap_spellings_are_distinct
