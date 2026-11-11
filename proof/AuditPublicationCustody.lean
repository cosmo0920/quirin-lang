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

import QCore.PublicationCustody

/-! Card CTWIN-PUBLISH: publication into an object that outlives the call.

Card CT-CTWIN recorded, as the open finding `CT-CTWIN-PUBLISH`, that a store of
an owned pointer into a file-scope object is invisible to the unrestricted-C
resource projection: publish-then-release was **`Safe` with no diagnostic**, and
the dangling published reference had no representation at all. This audit covers
the decision that closes it, in both directions, and the two-direction argument
for refusing the store rather than modelling it as an escape.

* **The decision, positively** — an alias, an acquisition and a custody-yielding
  call bound to a file-scope object are each refused, as is a store into a
  body's own `static` local; and the refusal withdraws a certificate.
* **Why refusal, not an escape** — under the escape model the
  published-and-not-released body certifies `safe`, which the implemented model
  refuses. The promotion is exhibited concretely and then stated as an
  existential, and the implemented model is shown to certify nothing and to
  manufacture no violation on either compared input.
* **Every way it fails closed** — a null store, an opaque value, a call with no
  custody result, an unbound source, a caller-held (borrowed) identity, a
  formal, a shadowing local declaration, and a name no reader recognised. The
  last is the recorded residual `CTWIN-PUBLISH-SCAN`, stated rather than hidden.
* **What the decision reads** — the destination half is a function of the four
  name sets alone, so no declared qualifier can move it (the model half of the
  `CT-CTWIN-SYNC` decision); the value half reads no per-site status, so a
  dangling identity is refused exactly as a live one is.
* **Monotonicity and the load-bearing conjuncts** — recognising more
  static-storage names only adds refusals, and each of the three conjuncts is
  shown moving a fixture.
* **The published verdict class** — a refusal never certifies, never
  manufactures a `violation`, never softens an existing one, and not firing
  changes nothing.
* **Non-vacuity** — the rule both refuses and admits.

The readers' faithfulness — that the file-scope declarators, the body's own
declarations, the formal names and the flow-sensitive bindings are extracted
correctly from C source — is a premise here, exactly as in
`QCore.BorrowedActual` and `QCore.BorrowLiveness`. -/

-- The decision, positively.
#print axioms
  QCore.PublicationCustody.a_store_of_a_body_acquired_identity_into_a_file_scope_object_is_refused
#print axioms QCore.PublicationCustody.an_acquisition_bound_to_such_an_object_is_refused
#print axioms QCore.PublicationCustody.a_custody_yielding_call_bound_to_such_an_object_is_refused
#print axioms QCore.PublicationCustody.a_static_local_is_a_destination
#print axioms QCore.PublicationCustody.a_refusal_withdraws_a_certificate
#print axioms QCore.PublicationCustody.a_publication_followed_by_a_release_never_certifies

-- Why refusal, and not an escape.
#print axioms QCore.PublicationCustody.the_escape_model_certifies_a_body_the_refusal_withdraws
#print axioms QCore.PublicationCustody.the_escape_model_promotes_on_some_input
#print axioms QCore.PublicationCustody.the_refusal_model_certifies_nothing
#print axioms QCore.PublicationCustody.a_publication_is_never_a_validated_violation

-- Every way the decision fails closed.
#print axioms QCore.PublicationCustody.a_null_store_is_never_refused
#print axioms QCore.PublicationCustody.an_untracked_value_is_never_refused
#print axioms QCore.PublicationCustody.a_call_with_no_custody_result_is_never_refused
#print axioms QCore.PublicationCustody.an_unbound_source_is_never_refused
#print axioms QCore.PublicationCustody.a_declared_scalar_source_is_never_refused
#print axioms QCore.PublicationCustody.a_formal_is_not_a_destination
#print axioms QCore.PublicationCustody.a_shadowed_name_is_not_a_destination
#print axioms QCore.PublicationCustody.an_unrecognised_name_is_not_a_destination

-- What the decision reads, and what it does not.
#print axioms QCore.PublicationCustody.the_destination_decision_reads_only_the_four_name_sets
#print axioms QCore.PublicationCustody.the_rule_reads_no_status

-- Monotonicity and the load-bearing conjuncts.
#print axioms QCore.PublicationCustody.a_wider_destination_set_only_adds_refusals
#print axioms QCore.PublicationCustody.a_wider_scan_only_adds_destinations
#print axioms QCore.PublicationCustody.the_static_duration_conjunct_is_load_bearing
#print axioms QCore.PublicationCustody.the_tracked_value_conjunct_is_load_bearing
#print axioms QCore.PublicationCustody.the_scalar_exclusion_is_load_bearing

-- The published verdict class.
#print axioms QCore.PublicationCustody.the_rule_never_certifies
#print axioms QCore.PublicationCustody.a_refusal_is_never_the_violation_class
#print axioms QCore.PublicationCustody.an_existing_violation_survives
#print axioms QCore.PublicationCustody.not_firing_changes_nothing

-- Non-vacuity.
#print axioms QCore.PublicationCustody.the_rule_both_refuses_and_admits

/-! Card CTWIN-RESIDUALS: the publication shapes wave 16 left open.

Three of card CTWIN-PUBLISH's four recorded residuals carried a **live false
`Safe`** on the measured base — a publish-then-free body certified with no
diagnostic through a parenthesised `_Atomic` specifier, through a helper that
publishes a borrowed formal, and through a declared `Out` slot naming the
object. The fourth, `CTWIN-PUBLISH-INDIRECT`, carried none: every indirect
shape was already refused, and what it kept instead was a residual `violation`
claim the direct rule withdraws.

* **`CTWIN-PUBLISH-BORROWED`, withdrawn** — the rule reads no provenance, and a
  caller-held identity is now refused. Why the exclusion had to go is argued at
  the *summary* boundary, where the launder actually lived: `borrowed` lets the
  caller's release certify while the same body inlined is refused, and the
  faithful alternative `escaped` promotes the unreleased caller exactly as the
  escape model does inside one body. A refused helper publishes no summary, so
  neither caller certifies; a non-publishing helper still does.
* **`CTWIN-PUBLISH-OUT`, withdrawn** — a declared `Out` slot naming such an
  object is refused, into a file-scope object and into a `static` local alike,
  with an ordinary local as the control; and, being refused before the call's
  effects apply, it is never a validated violation.
* **`CTWIN-PUBLISH-INDIRECT`, residual claim withdrawn** — a continued-access
  store into such an object withdraws its `violation` residual, a local base
  keeps its claim, an untracked value withdraws nothing, and the record can
  never certify anything on any input.
* **`CTWIN-PUBLISH-SCAN`, monotonicity preserved** — dropping a `_Atomic
  ( type-name )` specifier only adds names, never reaches a declarator
  parenthesis, recognises a name the old reader hid, and makes the two legal
  spellings of one object answer alike.
* **The card whole** — no landed shape certifies, and the widened rule still
  admits.

Two theorems of card CTWIN-PUBLISH are retired **by name** —
`a_caller_held_identity_is_never_refused` and
`the_caller_site_exclusion_is_load_bearing` — because the admission they pinned
was measured to launder a caller's release. `the_rule_reads_no_status` is
strengthened rather than retired: its caller-held hypothesis is deleted.

That the reader splits a declarator group's *specifier* parenthesis from its
*declarator* parenthesis faithfully is a premise, exactly as every other
reader's extraction is. -/

-- CTWIN-PUBLISH-BORROWED, withdrawn.
#print axioms QCore.PublicationCustody.the_rule_reads_no_provenance
#print axioms QCore.PublicationCustody.a_caller_held_identity_is_now_refused
#print axioms QCore.PublicationCustody.publishing_borrowed_launders_the_callers_release
#print axioms QCore.PublicationCustody.an_escaping_summary_promotes_the_unreleased_caller
#print axioms QCore.PublicationCustody.a_refused_helper_certifies_no_caller
#print axioms QCore.PublicationCustody.a_non_publishing_helper_still_certifies_its_caller

-- CTWIN-PUBLISH-OUT, withdrawn.
#print axioms QCore.PublicationCustody.an_out_slot_naming_such_an_object_is_refused
#print axioms QCore.PublicationCustody.an_out_slot_naming_a_static_local_is_refused
#print axioms QCore.PublicationCustody.an_out_slot_naming_an_ordinary_local_is_not_refused
#print axioms QCore.PublicationCustody.an_out_publication_is_never_a_validated_violation

-- CTWIN-PUBLISH-INDIRECT, the residual claim withdrawn.
#print axioms QCore.PublicationCustody.an_indirect_publication_withdraws_its_leak_claim
#print axioms QCore.PublicationCustody.an_indirect_store_into_a_local_keeps_its_claim
#print axioms
  QCore.PublicationCustody.an_indirect_store_of_an_untracked_value_withdraws_nothing
#print axioms QCore.PublicationCustody.the_indirect_record_never_certifies

-- CTWIN-PUBLISH-SCAN, monotonicity preserved.
#print axioms QCore.PublicationCustody.the_specifier_drop_only_adds_names
#print axioms QCore.PublicationCustody.a_declarator_parenthesis_still_skips_the_group
#print axioms QCore.PublicationCustody.the_drop_recognises_a_name_the_old_reader_hid
#print axioms QCore.PublicationCustody.both_spellings_of_one_object_are_recognised_alike

-- The card whole.
#print axioms QCore.PublicationCustody.no_landed_shape_certifies
#print axioms QCore.PublicationCustody.the_widened_rule_both_refuses_and_admits
