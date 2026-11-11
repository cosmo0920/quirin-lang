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

import QCore.DerivedResult

/-! Card **W43-LIBC-RESULT-ALIAS**: the derived-result mode
(`resource_return_alias`, `resource_return_interior`) and the gateway's derived
binding.

Six directions are audited. **Non-owning**: deriving changes no status, adds no
diagnostic and binds only its destination, to its base's target.
**Releasing a derived pointer is not a release of its base**: no status moves;
an interior pointer is accused (`invalidRelease`), an exact alias and a mixed
binding are refused. **Releasing the base invalidates every derived pointer**:
a later use or release is a `useAfterRelease` candidate, and the composition
with the base's own release is stated. **A derived null test narrows only
itself**, beside the identity rule it must not inherit. **No `Safe`**: both
candidates are safety issues and a refusal is never `Safe`. **Conservativity**:
on identity bindings every other instruction is the pre-card transfer.
The join, the `const`-dropped flag and five witness programs complete it.

What this is not: a proof about the Rust transfer (trust boundary 28's
correspondence premise) or about libc (the libc crate's trusted premises). A
candidate is never a validated upstream defect. -/

-- Non-owning.
#print axioms QCore.DerivedResult.deriving_changes_no_status
#print axioms QCore.DerivedResult.deriving_binds_only_its_destination
#print axioms QCore.DerivedResult.a_derived_pointer_targets_its_base

-- Releasing a derived pointer is not a release of its base.
#print axioms QCore.DerivedResult.releasing_a_derived_pointer_never_releases_its_base
#print axioms QCore.DerivedResult.releasing_an_interior_pointer_is_a_candidate
#print axioms QCore.DerivedResult.releasing_a_live_exact_alias_is_refused
#print axioms QCore.DerivedResult.releasing_a_mixed_pointer_is_refused

-- Releasing the base invalidates every derived pointer.
#print axioms QCore.DerivedResult.a_use_after_the_base_is_released_is_a_candidate
#print axioms QCore.DerivedResult.a_release_after_the_base_is_released_is_a_candidate
#print axioms QCore.DerivedResult.releasing_the_base_invalidates_every_derived_pointer

-- A derived null test narrows only itself.
#print axioms QCore.DerivedResult.a_derived_null_test_keeps_every_status
#print axioms QCore.DerivedResult.a_derived_null_test_keeps_every_other_binding
#print axioms QCore.DerivedResult.an_identity_null_test_removes_its_site

-- Never delivered through an out slot.
#print axioms QCore.DerivedResult.a_derived_pointer_is_never_delivered
#print axioms QCore.DerivedResult.an_identity_is_delivered

-- No `Safe` from any of it.
#print axioms QCore.DerivedResult.the_derived_candidates_are_safety_issues
#print axioms QCore.DerivedResult.a_candidate_is_never_safe
#print axioms QCore.DerivedResult.a_refusal_is_never_safe

-- Conservativity.
#print axioms QCore.DerivedResult.identity_bindings_are_unaffected

-- The join and the `const`-dropped flag.
#print axioms QCore.DerivedResult.join_of_equal_is_itself
#print axioms QCore.DerivedResult.identity_and_derived_join_to_mixed
#print axioms QCore.DerivedResult.a_null_path_keeps_the_other_relation
#print axioms QCore.DerivedResult.mixed_absorbs
#print axioms QCore.DerivedResult.const_dropped_survives_derivation
#print axioms QCore.DerivedResult.an_interior_step_is_interior

-- Witnesses.
#print axioms QCore.DerivedResult.the_use_after_release_witness
#print axioms QCore.DerivedResult.the_invalid_release_witness
#print axioms QCore.DerivedResult.the_null_derived_pointer_leaves_its_base_live
#print axioms QCore.DerivedResult.an_identity_null_test_would_have_hidden_it
#print axioms QCore.DerivedResult.the_exact_alias_witness
#print axioms QCore.DerivedResult.scanning_an_untracked_string_is_safe
