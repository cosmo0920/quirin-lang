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

//! Arm `asserted-allocation-premise` (classification), the registry's route
//! `asserted_allocation::decide`. The gateway asks it from
//! `legacy_checked_spec::settle` through `result_constructor_rule`.

use qcore_gateway_rule_api::{Answer, SpecificationPremiseArm, SpecificationQuery};

use crate::PACKAGE;

/// The arm's decision: admit a specification naming `asserted_allocation`
/// with that premise as its obligation, else refuse.
#[must_use]
pub fn decide(query: &SpecificationQuery<'_>) -> Answer<()> {
    SpecificationPremiseArm::ask(&PACKAGE, query)
}
