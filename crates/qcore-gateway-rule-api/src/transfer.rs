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

//! Card **RULE34-PILOT**: the transfer position ([`crate::Position::Transfer`],
//! route map §10), added for rule 34's migration (plan
//! `docs/plans/pluggable-cards/README.md` §3 B: "a migration card that needs a
//! further reading adds it here as a host capability";
//! `docs/reports/gateway-rule-api/README.md` §7: no route-map position covered
//! the transfer).
//!
//! The parser and every earlier position lower a call to an admitted contract
//! call. The transfer (`legacy_transfer.rs`, its `ContractCall` arm) then
//! applies the call: its own reading (the declared parameter modes and
//! result), and its declared container effects (`resource_published`,
//! `resource_unpublished`). The host's order applies every container effect
//! **after** the call's own reading. A [`TransferArm`] may admit another
//! order: one [`Phase`] per declared effect, in the declaration's position
//! order. Refusal keeps the host's order, and the host decides alone what a
//! refused call it cannot read in that order becomes (for rule 34, a refusal
//! of the call: see `docs/agents/rules/34.md`).
//!
//! The query needs none of the host's token or node vocabulary, so the arm is
//! not generic over [`crate::Host`] (like RULE37-PILOT's capabilities). It
//! answers [`Answer::Admit`] or [`Answer::Refuse`] and composes through
//! [`crate::consult`] like every other position.

use crate::answer::Answer;
use crate::traits::RulePackage;

/// One declared container effect of an admitted contract call, as the
/// transfer reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DeclaredEffect {
    /// The effect is `resource_unpublished(p)`: the call unlinks the object
    /// `p` designates from a container. Otherwise it is
    /// `resource_published(p, c)`: the call links it.
    pub unlinks: bool,
    /// The same declaration gives `p` the mode `resource_released(p, …)`:
    /// the call ends the object.
    pub releases: bool,
}

/// When one declared container effect applies, relative to the call's own
/// reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Phase {
    /// Before the call's own reading.
    BeforeCall,
    /// After it: the host's order for every effect no transfer rule moves.
    AfterCall,
}

/// One admitted contract call asked of a [`TransferArm`].
#[derive(Clone, Copy, Debug)]
pub struct TransferQuery<'q> {
    /// The called function.
    pub callee: &'q str,
    /// Its declared container effects, in the declaration's position order.
    pub effects: &'q [DeclaredEffect],
}

/// §10: an arm of the transfer. An admission's lowering is one [`Phase`] per
/// effect of the query, in its order; the host refuses an admission of
/// another length, or one carrying an obligation it cannot place. Refusal
/// keeps the host's order.
pub trait TransferArm: RulePackage {
    fn ask(&self, query: &TransferQuery<'_>) -> Answer<Vec<Phase>>;
}

#[cfg(test)]
mod tests {
    use super::{DeclaredEffect, Phase, TransferArm, TransferQuery};
    use crate::answer::{Answer, consult};
    use crate::traits::RulePackage;

    /// Moves every unlink before the call; refuses a call without one.
    struct UnlinkFirst;

    impl RulePackage for UnlinkFirst {
        fn id(&self) -> &'static str {
            "unlink-first"
        }
    }

    impl TransferArm for UnlinkFirst {
        fn ask(&self, query: &TransferQuery<'_>) -> Answer<Vec<Phase>> {
            if !query.effects.iter().any(|effect| effect.unlinks) {
                return Answer::Refuse;
            }
            Answer::admit(
                query
                    .effects
                    .iter()
                    .map(|effect| {
                        if effect.unlinks {
                            Phase::BeforeCall
                        } else {
                            Phase::AfterCall
                        }
                    })
                    .collect(),
            )
        }
    }

    #[test]
    fn a_transfer_arm_composes_and_a_disabled_one_keeps_the_host_order() {
        let effects = [
            DeclaredEffect {
                unlinks: false,
                releases: false,
            },
            DeclaredEffect {
                unlinks: true,
                releases: true,
            },
        ];
        let query = TransferQuery {
            callee: "destroy",
            effects: &effects,
        };
        let packages = [UnlinkFirst];
        let on = consult(&packages, |_| true, |_, package| package.ask(&query))
            .expect("the unlink is ordered");
        assert_eq!(on.admission.lowering, [Phase::AfterCall, Phase::BeforeCall]);
        assert!(on.admission.obligations.is_empty());
        assert!(consult(&packages, |_| false, |_, package| package.ask(&query)).is_none());
        let plain = TransferQuery {
            callee: "link",
            effects: &effects[..1],
        };
        assert!(consult(&packages, |_| true, |_, package| package.ask(&plain)).is_none());
    }
}
