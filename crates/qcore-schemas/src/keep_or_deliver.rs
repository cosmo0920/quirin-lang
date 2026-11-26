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

//! Card **OUT-SLOT-CONDITIONAL-DELIVERY** (gateway rule 85): the
//! keep-or-deliver summary of an in-out slot (`if (!*out) *out = make();`).
//!
//! USER DECISION 2026-11-25: "lanich four of queued cards", choosing the
//! return-family and this-lane set, which includes this card: build the
//! keep-or-deliver mode with Lean parity.
//!
//! The slot formal at [`KeepOrDeliverSummary::slot`] is written only on the
//! exits where it held null at entry: there it receives what the `delivered`
//! contract's `Out` mode states; where it held the caller's object, that
//! object is still there, still the caller's, and the call is `delivered`
//! with the slot read as `Owned { nullable: false }` of the object itself
//! ([`KeepOrDeliverSummary::kept`]). No single [`ResourceParameterMode`]
//! states both, and either one read alone is unsound (an `Out` read of a kept
//! object mints a second custody; a `Borrowed` read of a delivered one hides
//! a leak), so the summary lives in a table of its own
//! ([`crate::contract::ResourceContracts::keep_or_deliver`]), never in the
//! function table: every consumer that does not read it (cross-unit records,
//! callback candidates, indirect calls) sees an uncontracted function and
//! fails closed, exactly as for a member effect.
//!
//! The Lean model is `QCore.OutSlotKeepOrDeliver`.

use serde::{Deserialize, Serialize};

use crate::contract::{ResourceFunctionContract, ResourceParameterMode};

/// A validated keep-or-deliver summary.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeepOrDeliverSummary {
    /// The position of the slot formal (`T **out`).
    pub slot: usize,
    /// The call when the slot held null at entry: the slot position is
    /// `Out { nullable }`, every other position as validated.
    pub delivered: ResourceFunctionContract,
}

impl KeepOrDeliverSummary {
    /// The call when the slot held the caller's live object at entry: the
    /// same contract with the slot position read on that object, which the
    /// caller keeps (`Owned { nullable: false }`). `None` when `delivered`
    /// is not a keep-or-deliver contract at all (the slot is not `Out`).
    #[must_use]
    pub fn kept(&self) -> Option<ResourceFunctionContract> {
        if !matches!(
            self.delivered.parameters.get(self.slot),
            Some(ResourceParameterMode::Out { .. })
        ) {
            return None;
        }
        let mut kept = self.delivered.clone();
        kept.parameters[self.slot] = ResourceParameterMode::Owned { nullable: false };
        Some(kept)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::ResourceReturnMode;

    #[test]
    fn the_kept_reading_owns_the_slot_object_and_changes_nothing_else() {
        let summary = KeepOrDeliverSummary {
            slot: 1,
            delivered: ResourceFunctionContract::new(
                [
                    ResourceParameterMode::Borrowed,
                    ResourceParameterMode::Out { nullable: true },
                ],
                ResourceReturnMode::None,
            ),
        };
        let kept = summary.kept().expect("an out slot");
        assert_eq!(
            kept.parameters,
            [
                ResourceParameterMode::Borrowed,
                ResourceParameterMode::Owned { nullable: false }
            ]
        );
        assert_eq!(kept.result, ResourceReturnMode::None);
        let not_out = KeepOrDeliverSummary { slot: 0, ..summary };
        assert!(not_out.kept().is_none());
    }
}
