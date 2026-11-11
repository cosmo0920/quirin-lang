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

import QCore.NativeLibraryHeader

-- Card `NATIVE-LIBRARY-HEADER`: the exported-header level of
-- `source.native-library.v1` — reading the library's real `.h` instead of
-- believing the manifest's transcription of it. Model of the parts this card
-- added to `crates/qcore-core/src/native_library_header.rs`,
-- `native_library.rs` and the driver; the header's concrete lexical subset, its
-- directive layer and the total-or-reject text reader are separate Rust
-- boundaries, recorded with their subset census in
-- `docs/reports/wave22-native-library-header/README.md`.

-- **The agreement relation is the one card LOADER-ABI's gateway reader
-- compares with.** The crate layering forbids sharing the Rust code —
-- `qcore-core` has no dependencies at all — so this is where the two readers
-- are said to compare by one relation rather than two that could drift.
#print axioms QCore.NativeLibraryHeader.the_agreement_relation_is_the_declared_abi_one
#print axioms QCore.NativeLibraryHeader.a_read_position_that_does_not_admit_disagrees

-- **The level is opt-in and cannot reach a manifest that did not opt in**, and
-- it is not a no-op where one did.
#print axioms QCore.NativeLibraryHeader.a_manifest_naming_no_header_is_unchanged
#print axioms QCore.NativeLibraryHeader.the_level_is_not_a_no_op

-- **The digest is verified, not pinned**, and it decides before any declaration
-- is read — so a substituted header that agrees in every declaration is still
-- refused, which is the whole content of verifying an identity.
#print axioms QCore.NativeLibraryHeader.a_mismatched_digest_refuses
#print axioms QCore.NativeLibraryHeader.a_substituted_header_that_agrees_is_still_refused
#print axioms QCore.NativeLibraryHeader.an_unsupplied_header_refuses
#print axioms QCore.NativeLibraryHeader.a_reference_that_is_not_the_one_named_is_not_supplied

-- **Totality, one-directional.** A bound symbol the header does not declare
-- refuses; a declaration the manifest does not name changes nothing, stated
-- over arbitrary readings of the unnamed symbols. That is deliberately the
-- opposite of the contract slice's both-directions rule, and the reason is in
-- the module documentation.
#print axioms QCore.NativeLibraryHeader.a_bound_symbol_the_header_does_not_declare_refuses
#print axioms QCore.NativeLibraryHeader.unnamed_declarations_are_not_read

-- **A disagreement refuses and never withdraws**, and the weaker admissible
-- reading is exhibited beside the refusal so the claim is not a statement about
-- an empty set. Binding a header only removes admissions, and removes at least
-- one.
#print axioms QCore.NativeLibraryHeader.the_weaker_reading_is_admitted
#print axioms QCore.NativeLibraryHeader.a_contradicting_header_refuses_and_the_weaker_reading_is_admitted
#print axioms QCore.NativeLibraryHeader.a_header_binding_only_removes_admissions
#print axioms QCore.NativeLibraryHeader.a_header_binding_removes_something

-- **Read and unspellable are different facts.** `fnptr`, `f32` and `f64` are
-- §C04.1 spellings a manifest entry cannot write; collapsing them into
-- "unreadable" would report a vocabulary gap as a reader gap, and the two have
-- different follow-ons.
#print axioms QCore.NativeLibraryHeader.the_manifest_vocabulary_excludes_exactly_the_unspellable_spellings
#print axioms QCore.NativeLibraryHeader.an_unspellable_position_never_agrees
#print axioms QCore.NativeLibraryHeader.unreadable_and_unspellable_are_different_refusals

-- **The two typedef shapes are `QCore.TypedefProjection`'s**, imported rather
-- than restated: a function-pointer typedef is read and unspellable, the
-- aggregate beside it is read and spellable, and an aggregate value stays
-- unreadable.
#print axioms QCore.NativeLibraryHeader.a_function_pointer_typedef_is_read_and_unspellable
#print axioms QCore.NativeLibraryHeader.an_aggregate_pointer_typedef_is_read_and_spellable
#print axioms QCore.NativeLibraryHeader.an_aggregate_value_is_unreadable

-- **The `HEADER-STDINT-PRELUDE` is C11's own reading.** The exact-width names
-- are single readings; the names whose width the standard leaves open are the
-- pairs the manifest's declared `data_model lp64` premise fixes; a name the
-- standard leaves open is absent; and every prelude reading is spellable, which
-- is why the prelude is what lets this level decide anything at all.
#print axioms QCore.NativeLibraryHeader.the_exact_width_names_are_single_readings
#print axioms QCore.NativeLibraryHeader.the_open_width_names_are_the_lp64_pairs
#print axioms QCore.NativeLibraryHeader.a_name_the_standard_leaves_open_is_absent
#print axioms QCore.NativeLibraryHeader.every_prelude_reading_is_spellable

-- **Refusal completeness over the shape vocabulary.** Every shape a header can
-- present for a bound symbol refuses by its own name; an arity difference is a
-- disagreement rather than a refusal; and `static` refuses even where every
-- projection agrees, because internal linkage is a statement about the artifact
-- and not a reader boundary.
#print axioms QCore.NativeLibraryHeader.every_non_declaration_shape_refuses_by_name
#print axioms QCore.NativeLibraryHeader.every_incomparable_declaration_refuses_by_name
#print axioms QCore.NativeLibraryHeader.an_arity_difference_disagrees
#print axioms QCore.NativeLibraryHeader.an_internal_declaration_refuses_even_when_it_agrees

-- **Non-vacuity.**
#print axioms QCore.NativeLibraryHeader.an_agreeing_binding_is_admitted
