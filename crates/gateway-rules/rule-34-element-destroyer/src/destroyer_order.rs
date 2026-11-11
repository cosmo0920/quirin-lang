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

//! Rule 34's transfer arm `destroyer-order`: the element destroyer's unlink
//! applies before the call's own reading.

use qcore_gateway_rule_api::traits::RulePackage;
use qcore_gateway_rule_api::{Answer, DeclaredEffect, Phase, TransferArm, TransferQuery};

/// The package: rule 34 at the transfer position.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ElementDestroyer;

impl RulePackage for ElementDestroyer {
    fn id(&self) -> &'static str {
        crate::ID
    }
}

impl TransferArm for ElementDestroyer {
    fn ask(&self, query: &TransferQuery<'_>) -> Answer<Vec<Phase>> {
        order(query.effects)
    }
}

/// Whether one declared effect is an element destroyer's unlink: an unlink
/// (`resource_unpublished(p)`) of a parameter the same declaration releases
/// (`resource_released(p, …)`).
#[must_use]
pub const fn is_destroyer(effect: DeclaredEffect) -> bool {
    effect.unlinks && effect.releases
}

/// When one declared effect applies: a destroyer's unlink before the call's
/// own reading, every other effect after it.
#[must_use]
pub const fn phase(effect: DeclaredEffect) -> Phase {
    if is_destroyer(effect) {
        Phase::BeforeCall
    } else {
        Phase::AfterCall
    }
}

/// The arm's answer for one call's declared container effects: the phases,
/// when at least one effect is a destroyer's unlink; otherwise a refusal
/// (not rule 34's shape: the host's order stands, which is every phase
/// `AfterCall`). The admission carries no obligation: the premises rule 34's
/// rows rest on (P-CRR-MEMBER, P-CRR-LINKED, the declaration's own) are
/// carried by the host's unlink and by the declaration.
#[must_use]
pub fn order(effects: &[DeclaredEffect]) -> Answer<Vec<Phase>> {
    if effects.iter().copied().any(is_destroyer) {
        Answer::admit(effects.iter().copied().map(phase).collect())
    } else {
        Answer::Refuse
    }
}

/// The phase of every effect shape, `(unlinks, releases, phase)`. The Lean
/// model's `phaseVectors` (`QCore.ElementDestroyerSwitch`) is this table, and
/// a test holds the two equal.
pub const DECISION_VECTORS: &[(bool, bool, &str)] = &[
    (false, false, "after"),
    (false, true, "after"),
    (true, false, "after"),
    (true, true, "before"),
];

/// The table's spelling of a phase.
#[must_use]
pub const fn code(phase: Phase) -> &'static str {
    match phase {
        Phase::BeforeCall => "before",
        Phase::AfterCall => "after",
    }
}

#[cfg(test)]
#[path = "../tests/unit/destroyer_order.rs"]
mod tests;
