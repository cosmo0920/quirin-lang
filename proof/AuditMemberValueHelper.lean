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

import QCore.MemberValueHelper
#print axioms QCore.MemberValueHelper.the_slot_abstracts_the_path
#print axioms QCore.MemberValueHelper.established_iff_taken_once
#print axioms QCore.MemberValueHelper.the_reading_is_the_abstract_run
#print axioms QCore.MemberValueHelper.an_escape_never_establishes
#print axioms QCore.MemberValueHelper.an_uncomposed_callee_never_establishes
#print axioms QCore.MemberValueHelper.no_take_never_establishes
#print axioms QCore.MemberValueHelper.a_second_take_never_establishes
#print axioms QCore.MemberValueHelper.a_copy_not_yet_written_is_no_take
#print axioms QCore.MemberValueHelper.the_destroyers_path_establishes
#print axioms QCore.MemberValueHelper.a_join_is_established_iff_both_are
#print axioms QCore.MemberValueHelper.distinct_identities_each_end_once
#print axioms QCore.MemberValueHelper.one_identity_refuses_the_second_end
