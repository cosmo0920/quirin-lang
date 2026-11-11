// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Arm `sizeof-operand-exemption` (typed reader), the registry's route
//! `sizeof_operand::decide`. The gateway asks it from
//! `aggregate_store::store_bases` through `result_constructor_rule`.

use qcore_gateway_rule_api::{Answer, CheckedOperandArm, CheckedOperandQuery};

use crate::PACKAGE;

/// The arm's decision: the checked operand spans when they are well formed,
/// else a refusal of the whole answer.
#[must_use]
pub fn decide(query: &CheckedOperandQuery<'_>) -> Answer<Vec<(usize, usize)>> {
    CheckedOperandArm::ask(&PACKAGE, query)
}
