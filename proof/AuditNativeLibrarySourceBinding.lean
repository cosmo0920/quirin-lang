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

import QCore.NativeLibrarySourceBinding

-- Card `NATIVE-LIBRARY-SOURCE-BINDING`: the source-file-scoped selection of a
-- manifest-bound library, `extern library NAME;`. Model of what this card added
-- to `crates/qcore-core/src/native_library.rs` and `extern_call.rs`; the source
-- spelling, the parser, the driver's diagnostics and the per-file span
-- arithmetic are separate Rust boundaries, recorded in
-- `docs/reports/native-library-source-binding/README.md`.

-- **The binding is a function.** An unknown name, an ambiguous name and a
-- repeated name each refuse, so a resolved binding names exactly one build.
#print axioms QCore.NativeLibrarySourceBinding.an_unknown_name_refuses
#print axioms QCore.NativeLibrarySourceBinding.an_ambiguous_name_refuses
#print axioms QCore.NativeLibrarySourceBinding.a_repeated_name_refuses

-- **An unbound file is unchanged.** The card is opt-in: an empty binding list is
-- the selection-wide view, and such a file is never owed a refusal.
#print axioms QCore.NativeLibrarySourceBinding.the_empty_binding_is_unbound
#print axioms QCore.NativeLibrarySourceBinding.an_unbound_view_resolves_selection_wide
#print axioms QCore.NativeLibrarySourceBinding.an_unbound_view_is_never_foreign

-- **A binding only ever narrows, and a narrowing refuses.** The first is hard
-- invariant 1's direction; the second is the theorem that keeps this card from
-- being a weakening — exactly where a binding removes a resolution, the file is
-- owed a refusal rather than the weaker premise.
#print axioms QCore.NativeLibrarySourceBinding.a_binding_only_narrows
#print axioms QCore.NativeLibrarySourceBinding.a_narrowed_symbol_is_refused
#print axioms QCore.NativeLibrarySourceBinding.a_symbol_no_manifest_binds_is_not_foreign
#print axioms QCore.NativeLibrarySourceBinding.resolving_and_refusing_are_exclusive

-- **Bindings do not leak across compilation inputs** (constraint 13's discipline
-- applied to a binding): each file's view is a function of that file's own list.
#print axioms QCore.NativeLibrarySourceBinding.a_file_s_view_is_its_own
#print axioms QCore.NativeLibrarySourceBinding.an_unbound_sibling_is_unmoved

-- **The premise names the build**, and the identity it names is the declared
-- one — this card resolves a name to a declaration and opens no file.
#print axioms QCore.NativeLibrarySourceBinding.the_premise_names_the_build
#print axioms QCore.NativeLibrarySourceBinding.the_identity_is_the_declared_one

-- Non-vacuity: the route binds, narrows and refuses on one reachable example.
#print axioms QCore.NativeLibrarySourceBinding.the_route_binds_narrows_and_refuses
