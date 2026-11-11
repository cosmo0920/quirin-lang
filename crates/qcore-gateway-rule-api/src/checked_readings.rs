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

//! Card **RULE37-PILOT**: two host capabilities whose queries need none of the
//! host's vocabulary, added for rule 37's migration (plan
//! `docs/plans/pluggable-cards/README.md` §3 B: "a migration card that needs a
//! further reading adds it here as a host capability").
//!
//! * [`CheckedOperandArm`] (§7, typed reader): the host's own typed,
//!   non-variably-modified `sizeof` operand reader has already decided which
//!   operand spans of a body are unevaluated; the arm decides which of those
//!   spans a scan may skip. Refusal keeps the scan's old reading (every
//!   mention counts).
//! * [`SpecificationPremiseArm`] (§2.2 and §6, the contracted arm of a call):
//!   a checked specification (contract spec §C06.3) that names a premise a
//!   package owns is in force only while that package admits it, and the
//!   admission carries the premise as an [`Obligation::Premise`]. Refusal
//!   withdraws the specification, so a switched-off package only removes
//!   specifications.
//!
//! Both answer [`Answer::Admit`] or [`Answer::Refuse`] and compose through
//! [`crate::consult`] like every other position.
//!
//! [`Obligation::Premise`]: crate::Obligation::Premise

use crate::answer::Answer;
use crate::traits::RulePackage;

/// One body asked of a [`CheckedOperandArm`].
#[derive(Clone, Copy, Debug)]
pub struct CheckedOperandQuery<'q> {
    /// The body's token spellings.
    pub tokens: &'q [&'q str],
    /// The half-open spans `[start, end)` of `tokens` that the host's typed
    /// operand reader admitted: each starts at a `sizeof` keyword and covers
    /// its whole operand. A span the reader did not admit is never here.
    pub checked: &'q [(usize, usize)],
}

/// §7: an arm of the typed operand reader. The admission's lowering is the
/// set of spans whose mentions a scan reads as unevaluated; refusal keeps the
/// old reading, in which every mention counts.
pub trait CheckedOperandArm: RulePackage {
    fn ask(&self, query: &CheckedOperandQuery<'_>) -> Answer<Vec<(usize, usize)>>;
}

/// One checked specification asked of a [`SpecificationPremiseArm`].
#[derive(Clone, Copy, Debug)]
pub struct SpecificationQuery<'q> {
    /// The specified function.
    pub function: &'q str,
    /// The named premises its declaration states
    /// (`requires resource_premise(NAME, "STATEMENT");`), in clause order.
    pub premises: &'q [&'q str],
}

/// §2.2 and §6: an arm of the checked-specification settlement. The host asks
/// it only for a checked specification whose declaration names
/// [`Self::premise`]; an admission must carry that premise as an
/// `Obligation::Premise` the declaration states, and anything else (a refusal,
/// a disabled package, an obligation the declaration does not state, an open
/// obligation) withdraws the specification.
pub trait SpecificationPremiseArm: RulePackage {
    /// The named premise the package owns.
    fn premise(&self) -> &'static str;
    fn ask(&self, query: &SpecificationQuery<'_>) -> Answer<()>;
}

#[cfg(test)]
mod tests {
    use super::{
        CheckedOperandArm, CheckedOperandQuery, SpecificationPremiseArm, SpecificationQuery,
    };
    use crate::answer::{Admission, Answer, Obligation, consult};
    use crate::traits::RulePackage;

    struct Skip;
    impl RulePackage for Skip {
        fn id(&self) -> &'static str {
            "skip"
        }
    }
    impl CheckedOperandArm for Skip {
        fn ask(&self, query: &CheckedOperandQuery<'_>) -> Answer<Vec<(usize, usize)>> {
            Answer::admit(query.checked.to_vec())
        }
    }
    impl SpecificationPremiseArm for Skip {
        fn premise(&self) -> &'static str {
            "owned"
        }
        fn ask(&self, query: &SpecificationQuery<'_>) -> Answer<()> {
            if query.premises.contains(&self.premise()) {
                Answer::Admit(Admission {
                    lowering: (),
                    obligations: vec![Obligation::Premise(self.premise().into())],
                })
            } else {
                Answer::Refuse
            }
        }
    }

    #[test]
    fn a_disabled_arm_keeps_the_old_reading_and_withdraws_the_specification() {
        let tokens = ["sizeof", "(", "*", "e", ")"];
        let query = CheckedOperandQuery {
            tokens: &tokens,
            checked: &[(0, 5)],
        };
        let packages = [Skip];
        let on = consult(
            &packages,
            |_| true,
            |_, p| CheckedOperandArm::ask(p, &query),
        );
        assert_eq!(on.expect("admits").admission.lowering, [(0, 5)]);
        assert!(
            consult(
                &packages,
                |_| false,
                |_, p| CheckedOperandArm::ask(p, &query)
            )
            .is_none()
        );
        let specification = SpecificationQuery {
            function: "f",
            premises: &["owned"],
        };
        let admitted = consult(
            &packages,
            |_| true,
            |_, p| SpecificationPremiseArm::ask(p, &specification),
        )
        .expect("admits");
        assert_eq!(
            admitted.admission.obligations,
            [Obligation::Premise("owned".into())]
        );
        assert!(
            consult(
                &packages,
                |_| false,
                |_, p| SpecificationPremiseArm::ask(p, &specification)
            )
            .is_none()
        );
    }
}
