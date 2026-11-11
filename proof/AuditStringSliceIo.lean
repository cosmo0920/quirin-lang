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

import QCore.StringSliceIo

-- Card `STRING-SLICE-IO`: `std.string` composed with the output modules, so a
-- string view can reach a stream (amendment §7 of
-- `docs/design/STRING_SLICE_V1.md`).

-- The vocabulary the composition adds, printed so that a later change to any of
-- it is visible in this surface rather than only in a proof.
#print axioms QCore.StringSliceIo.written
#print axioms QCore.StringSliceIo.erase
#print axioms QCore.StringSliceIo.eraseProgram
#print axioms QCore.StringSliceIo.composedEffects
#print axioms QCore.StringSliceIo.admitsText

-- **Neutrality.** A text line erases to the base route's own line, and the
-- effect row of a composed program is the row of its erasure — so every theorem
-- `QCore.StdString` proves about a program is a theorem about the same program
-- with rendered text in it. A program with no text in it *is* its erasure,
-- which is the model's form of the measured byte-identity of everything that
-- existed before this card.
#print axioms QCore.StringSliceIo.erasing_the_text_lines_leaves_the_effect_row
#print axioms QCore.StringSliceIo.a_program_with_no_text_is_its_own_erasure

-- Amendment §7 for the composed route: rendering derives `io` and nothing else.
#print axioms QCore.StringSliceIo.a_text_line_derives_io
#print axioms QCore.StringSliceIo.a_text_line_derives_no_alloc
#print axioms QCore.StringSliceIo.a_text_line_derives_no_mut

-- **The rendering rule is the identity on bytes**, framed by the sink's own
-- line discipline and by nothing else: the body is the view's payload, a
-- `print` line is unprefixed on standard output, a `log` line carries its
-- bracketed tag, and the framing appends exactly one newline.
#print axioms QCore.StringSliceIo.the_rendered_body_is_the_views_own_bytes
#print axioms QCore.StringSliceIo.a_print_line_is_unprefixed_on_standard_output
#print axioms QCore.StringSliceIo.a_logged_line_carries_its_level_tag
#print axioms QCore.StringSliceIo.the_framing_appends_exactly_one_newline
#print axioms QCore.StringSliceIo.no_level_tag_contains_a_newline
#print axioms QCore.StringSliceIo.the_print_framing_contains_no_newline
#print axioms QCore.StringSliceIo.the_empty_view_writes_only_the_framing

-- **One framing, two line kinds.** A rendered scalar line and a rendered text
-- line with the same body are the same written line, by `rfl`: there is one
-- framing rather than two implementations that agree.
#print axioms QCore.StringSliceIo.the_two_line_kinds_share_one_framing

-- **The NUL decision, as a disagreement.** On a payload this route admits, a
-- scan-to-terminator write and this model disagree — two bytes against five —
-- which is why the emitted C writes with a length and why no `printf`
-- conversion appears in an emitted line.
#print axioms QCore.StringSliceIo.every_byte_of_the_view_is_written
#print axioms QCore.StringSliceIo.a_terminator_scan_would_write_fewer_bytes
#print axioms QCore.StringSliceIo.an_embedded_nul_reaches_the_stream

-- **One decision for a slice, wherever it is written**, and fail-closed on a
-- name this entry bound no string for. The operand wall states the three forms
-- as one list, so a widening changes a theorem rather than slipping past it.
#print axioms QCore.StringSliceIo.an_inline_slice_is_admitted_exactly_when_the_bound_one_is
#print axioms QCore.StringSliceIo.a_name_with_no_view_refuses
#print axioms QCore.StringSliceIo.the_text_operand_forms_are_exactly_three

-- **The selector decision and the module gate it decides.** A text line names
-- no `std.fmt` rendering, so a module that renders only text needs no
-- formatting module; a scalar line still does, so the narrowing is not a
-- weakening.
#print axioms QCore.StringSliceIo.a_text_line_names_no_rendering
#print axioms QCore.StringSliceIo.a_text_line_needs_no_formatting_module
#print axioms QCore.StringSliceIo.a_scalar_line_still_needs_the_formatting_module

-- **Both modules, and neither grants the other.** This card did not make
-- `std.io` accept text; it made a module naming *both* able to render it, which
-- is amendment §7's own sentence as a registry fact.
#print axioms QCore.StringSliceIo.a_rendered_string_needs_both_modules
#print axioms QCore.StringSliceIo.a_logged_string_needs_both_modules
#print axioms QCore.StringSliceIo.the_output_imports_alone_still_grant_no_string
#print axioms QCore.StringSliceIo.the_string_import_alone_still_grants_no_stream
