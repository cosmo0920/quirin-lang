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

//! The registry's routes for rule 33's two arms: `selection_route::expand`
//! (statement rewrite, `removed-selection-expansion`) and
//! `selection_route::row` (classification, `removed-selection-entry`). Each
//! is the arm's decision; the gateway asks them from `legacy_selection`.

use qcore_gateway_rule_api::{Answer, BodyQuery, EntryRow, Expanded, StatementExpansion};

use crate::{Frame, RemovedSelection, ValidationRun};

/// The expansion arm: a checked frame's own body, or its decimal-constant
/// caller statements, become the constituent calls; refuses when nothing
/// changes.
#[must_use]
pub fn expand(query: &BodyQuery<'_, Frame>) -> Answer<Expanded> {
    StatementExpansion::ask(&RemovedSelection, query)
}

/// The entry arm: admits a validated run only.
#[must_use]
pub fn row(run: &ValidationRun) -> Answer<()> {
    EntryRow::ask(&RemovedSelection, run)
}
