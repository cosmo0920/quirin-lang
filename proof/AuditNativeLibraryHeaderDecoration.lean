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

import QCore.NativeLibraryHeaderDecoration

-- Card `NATIVE-LIBRARY-HEADER-DECORATION`: making a **decorated** exported
-- header readable without creating an unchecked premise about its text. Model
-- of the parts this card added to
-- `crates/qcore-core/src/native_library_header.rs` and `native_library.rs`; the
-- concrete lexical layer — splicing, guard removal, tokenising — is a Rust
-- boundary tested there and censused in
-- `docs/reports/native-library-header-decoration/README.md`.
--
-- Card `NATIVE-LIBRARY-HEADER` named the obvious repair, a manifest-declared
-- erasure list, and declined it: it converts a reader boundary into an
-- unchecked premise about the header's own text. The theorems below are the
-- derived route instead — a manifest declares which hash-identified texts may be
-- read, and every erasure is read out of those texts' own `#define`s.

-- **Erasure is expansion.** The justification of the whole derivation, as an
-- equality rather than an appeal: a body all of whose tokens the vocabulary
-- erases contributes nothing, so deleting the macro name is splicing its body
-- in. The Rust side makes it literal by erasing the written form and the derived
-- name with the one vocabulary.
#print axioms QCore.NativeLibraryHeaderDecoration.expanding_then_erasing_equals_erasing
#print axioms QCore.NativeLibraryHeaderDecoration.erasing_the_name_is_expanding_it
#print axioms QCore.NativeLibraryHeaderDecoration.erase_keeps_exactly_the_code
#print axioms QCore.NativeLibraryHeaderDecoration.erase_append
#print axioms QCore.NativeLibraryHeaderDecoration.an_empty_body_is_erasable

-- **The derivation can only widen what is readable, never what is admitted.** A
-- name it does not establish survives into the specifier prefix, where the
-- declaration keeps card `NATIVE-LIBRARY-HEADER`'s refusal; erasure deletes no
-- token the comparison reads; and applying the vocabulary twice changes nothing.
#print axioms QCore.NativeLibraryHeaderDecoration.an_underived_name_survives
#print axioms QCore.NativeLibraryHeaderDecoration.erasure_preserves_every_code_token
#print axioms QCore.NativeLibraryHeaderDecoration.erasure_is_idempotent

-- **No condition is evaluated, and none is guessed.** A group establishes
-- definedness only when it wrote an `#else` — so that no implicit empty arm
-- escapes — and only when every one of its arms establishes it. Whichever arm a
-- consumer's configuration selects, the name is defined.
#print axioms QCore.NativeLibraryHeaderDecoration.a_group_with_no_else_establishes_nothing
#print axioms QCore.NativeLibraryHeaderDecoration.a_group_with_no_else_is_never_derivable
#print axioms QCore.NativeLibraryHeaderDecoration.an_arm_that_does_not_define_withdraws_the_group

-- **Every definition must be erasable for the name to be**, a function-like
-- definition is never derivable because this reader evaluates no expansion, and
-- an `#undef` anywhere withdraws the derivation because this reader tracks no
-- scope for a macro.
#print axioms QCore.NativeLibraryHeaderDecoration.a_function_like_definition_is_not_derivable
#print axioms QCore.NativeLibraryHeaderDecoration.an_unerasable_body_is_not_derivable
#print axioms QCore.NativeLibraryHeaderDecoration.one_unerasable_definition_withdraws_the_derivation
#print axioms QCore.NativeLibraryHeaderDecoration.an_undefined_name_is_not_derivable

-- **And the derivation is not vacuous**, and is a function of the text alone —
-- there is no manifest input to it, which is the whole difference between this
-- card and the declared erasure list that was declined.
#print axioms QCore.NativeLibraryHeaderDecoration.the_real_visibility_shape_is_derivable
#print axioms QCore.NativeLibraryHeaderDecoration.the_derivation_is_a_function_of_the_text

-- **The texts read are exactly the declared ones.** A declared text must be
-- supplied, must measure the declared digest — decided *before* it is read, so a
-- substituted text that would have been read alike is still refused — and must
-- be reached; a text nobody declares is refused rather than read.
#print axioms QCore.NativeLibraryHeaderDecoration.an_unsupplied_declared_text_refuses
#print axioms QCore.NativeLibraryHeaderDecoration.a_substituted_declared_text_refuses
#print axioms QCore.NativeLibraryHeaderDecoration.an_unreached_declared_text_refuses
#print axioms QCore.NativeLibraryHeaderDecoration.an_undeclared_source_refuses

-- **Declaring no text is card `NATIVE-LIBRARY-HEADER`'s reading**, unchanged —
-- and the accepted configuration is reachable, so the level is not a refusal
-- machine.
#print axioms QCore.NativeLibraryHeaderDecoration.declaring_no_text_binds_trivially
#print axioms QCore.NativeLibraryHeaderDecoration.a_declared_supplied_and_reached_text_binds
