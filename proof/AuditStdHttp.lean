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

import QCore.StdHttp

-- Card `HTTP-QUIRIN-V1`: `source.http.v1`, the `std.http` bundled module's
-- checked HTTP/1.1 message subset. An **executed HTTP/1.1 subset with the
-- boundary named**, never HTTP/1.1 conformance.

-- The message grammar's admission. RFC 9110 §5.6.2's `token` and RFC 9110
-- §5.5's field-content, decided over literals whose bytes are known at check
-- time.
#print axioms QCore.StdHttp.the_version_is_exactly_http_1_1
#print axioms QCore.StdHttp.a_status_outside_the_registered_range_is_refused

-- The framing invariants. No source program writes a CRLF: the builder writes
-- them, and no admitted component may carry one, so every CRLF in an admitted
-- head is one this module wrote. Header injection is therefore not expressible
-- on this route rather than merely not intended.
#print axioms QCore.StdHttp.no_framing_byte_is_a_field_byte
#print axioms QCore.StdHttp.no_framing_byte_is_a_tchar
#print axioms QCore.StdHttp.an_admitted_component_carries_no_framing_byte
#print axioms QCore.StdHttp.an_admitted_token_carries_no_framing_byte
#print axioms QCore.StdHttp.an_admitted_field_carries_no_framing_byte

-- The shape of a message on the wire, as equations rather than as comments:
-- RFC 9112 §2.1's head-then-content, the head's decomposition into a start line
-- and one line per field, and the empty line that ends every head.
#print axioms QCore.StdHttp.the_wire_is_the_head_then_the_content
#print axioms QCore.StdHttp.a_message_with_no_content_is_its_head
#print axioms QCore.StdHttp.the_head_is_the_start_line_then_the_fields_then_the_empty_line
#print axioms QCore.StdHttp.every_field_line_ends_in_crlf
#print axioms QCore.StdHttp.the_field_lines_are_crlf_terminated
#print axioms QCore.StdHttp.the_head_ends_in_an_empty_line
#print axioms QCore.StdHttp.a_fieldless_head_is_the_start_line_and_an_empty_line
#print axioms QCore.StdHttp.one_more_field_adds_exactly_one_line

-- The `Content-Length` equality: the card's signature claim, proved in both
-- directions so the check is a gate and not a description. A literal's byte
-- count is known at check time, which is the whole reason this is decidable.
#print axioms QCore.StdHttp.a_declared_length_is_the_content_length
#print axioms QCore.StdHttp.a_length_that_is_not_the_content_length_is_refused
#print axioms QCore.StdHttp.content_without_a_declared_length_is_refused
#print axioms QCore.StdHttp.no_content_and_no_length_is_admitted
#print axioms QCore.StdHttp.a_length_on_an_empty_message_must_be_zero

-- The effect discipline: the HTTP vocabulary derives nothing at all, stated
-- over the whole vocabulary so a widening changes a theorem.
#print axioms QCore.StdHttp.no_http_statement_derives_an_effect
#print axioms QCore.StdHttp.the_statement_forms_are_exactly_three

-- The module gate, and the composition rule this card had to argue: `std.http`
-- grants neither the transport nor the text module, and neither grants it.
#print axioms QCore.StdHttp.the_http_capability_needs_its_module
#print axioms QCore.StdHttp.the_http_module_grants_neither_transport_nor_text

-- The registry's own new rows (this card grows it from nine modules to ten).
#print axioms QCore.ModuleImports.empty_imports_grant_no_http
#print axioms QCore.ModuleImports.admitted_http_requires_its_module
#print axioms QCore.ModuleImports.the_http_import_alone_grants_no_socket
#print axioms QCore.ModuleImports.the_http_module_is_independent_of_the_text_module
#print axioms QCore.ModuleImports.the_http_import_alone_grants_no_stream
#print axioms QCore.ModuleImports.the_neighbouring_modules_grant_no_http
#print axioms QCore.ModuleImports.concurrency_grants_no_http_capability
#print axioms QCore.ModuleImports.neither_extern_module_grants_http
#print axioms QCore.ModuleImports.code_that_declares_no_message_needs_no_http_import
#print axioms QCore.ModuleImports.duplicate_http_import_is_idempotent
#print axioms QCore.ModuleImports.the_http_module_is_independent_of_its_neighbours
#print axioms QCore.ModuleImports.a_near_miss_http_spelling_rejects
