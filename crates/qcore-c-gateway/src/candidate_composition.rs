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

//! Card **HANDOFF-CANDIDATE-OWN-CALLBACKS** (gateway rule 13u amended, with
//! rules 12m and 12n): a callback candidate whose own declaration carries
//! member invocations, decided at the invocation as the composition of its
//! own clauses. Bound to `QCore.CandidateComposition` by [`COMPOSE_VECTORS`].
//!
//! A declaration carrying a callback clause is a retaining declaration,
//! withheld from the function map, so rule 13u read such a candidate as
//! uncontracted wherever it was a candidate. The composition reads it as:
//!
//! * **its own member invocations**, each decided exactly as rule 12n decides
//!   it in the candidate's own body: over the **run's** set (the pre-pass),
//!   each inner candidate classified with the candidate itself as the invoker
//!   (rule 12m's caller branch check from the candidate's body), then
//!   [`crate::callback_contracts::decide_run`]. Only the run's set: the
//!   member fold checks the members of the declarations a row lists, and a
//!   row does not list a composed candidate, so a unit's own set is never
//!   enough here;
//! * **its formals' modes**: its custody contract, strengthened by every
//!   requirement its inner candidates put on its own parameters
//!   ([`strengthen`]);
//! * **its hand-off**: rule 12m's caller branch check at the outer invoker,
//!   after the composition;
//! * **member-path states**: a declaration stating its own is not composed;
//!   an inner candidate's state at one of the candidate's formals refuses
//!   where the outer invocation hands that formal a parameter of its caller
//!   (two levels of rule 30 are not composed), and changes nothing at a `_`
//!   or `supplied_owned` position (an object that was never the caller's).
//!
//! The candidate counts as contracted only when every own invocation
//! decides; otherwise it refuses, naming the invocation ([`OWN_REFUSED`]).
//! **Least fixed point** (as rule 15w): a candidate met again while its own
//! invocations are decided refuses ([`with_stack`]), so a cycle never
//! supports itself and an unresolved cycle refuses.

use qcore_schemas::callback::CallbackClause;
use qcore_schemas::contract::{ResourceFunctionContract, ResourceParameterMode};
use qcore_schemas::retained_by::RetainingContract;

/// The stable reason key of a candidate whose own invocations do not all
/// decide.
pub const OWN_REFUSED: &str = "callback-own-invocation-refused";

/// Whether a declaration is read by the composition: it carries callback
/// clauses, every one a member invocation, and nothing else beside its
/// custody contract and a hand-off. Any other own callback clause (a formal
/// invocation, a table, a registry, a setter) keeps the candidate
/// uncontracted, as before the card.
#[must_use]
pub fn composable(declared: &RetainingContract) -> bool {
    !declared.callbacks.is_empty()
        && declared
            .callbacks
            .iter()
            .all(|clause| matches!(clause, CallbackClause::MemberInvoked { .. }))
        && declared.retentions.is_empty()
        && declared.resets.is_empty()
        && declared.slot_moves.is_empty()
        && declared.derived_outs.is_empty()
        && declared.references.is_empty()
        && declared.discriminant.is_none()
        && declared.arms.is_empty()
        && declared.constructed.is_empty()
        && declared.member_paths.is_empty()
}

/// Decide `name`'s own invocations under the decision stack: a candidate
/// already on the stack (its own composition is being decided) refuses, so
/// only what the least fixed point contracts is ever contracted.
///
/// # Errors
/// The cycle, or what `decide` refuses.
pub fn with_stack<T>(
    stack: &mut Vec<String>,
    name: &str,
    decide: impl FnOnce(&mut Vec<String>) -> Result<T, String>,
) -> Result<T, String> {
    if stack.iter().any(|entry| entry == name) {
        let mut cycle = stack
            .iter()
            .skip_while(|entry| *entry != name)
            .cloned()
            .collect::<Vec<_>>();
        cycle.push(name.to_owned());
        return Err(format!(
            "`{name}` is reached again while its own invocations are decided ({}): a cycle \
             never supports itself",
            cycle.join(" -> ")
        ));
    }
    stack.push(name.to_owned());
    let decided = decide(stack);
    stack.pop();
    decided
}

/// Fold one requirement an inner candidate puts on the candidate's own
/// parameter `index` (rule 13u: `resource_owned(n)` there) into the
/// candidate's contract: a borrowed formal becomes `resource_owned(n)`, an
/// owned one keeps the stronger of the two, and a formal the candidate takes
/// already requires a live object.
pub fn strengthen(contract: &mut ResourceFunctionContract, index: usize, nullable: bool) {
    let Some(mode) = contract.parameters.get_mut(index) else {
        return;
    };
    match mode {
        ResourceParameterMode::Borrowed => {
            *mode = ResourceParameterMode::Owned { nullable };
        }
        ResourceParameterMode::Owned { nullable: own } => *own &= nullable,
        _ => {}
    }
}

/// One own invocation of a vector candidate: (admits apart from its inner
/// candidates, inner candidates).
#[cfg(test)]
pub(crate) type VectorInvocation = (bool, &'static [usize]);

/// `COMPOSE_VECTORS`, shared with `QCore.CandidateComposition.composeVectors`:
/// (graph: per candidate, its own reading and its own invocations; the
/// candidate; contracted).
#[cfg(test)]
#[allow(clippy::type_complexity)]
pub(crate) const COMPOSE_VECTORS: &[(&[(bool, &[VectorInvocation])], usize, bool)] = &[
    (&[(true, &[])], 0, true),
    (&[(false, &[])], 0, false),
    (
        &[(true, &[(true, &[1, 2])]), (true, &[]), (true, &[])],
        0,
        true,
    ),
    (&[(true, &[(false, &[1])]), (true, &[])], 0, false),
    (&[(true, &[(true, &[1])]), (false, &[])], 0, false),
    (
        &[(true, &[(true, &[1]), (false, &[])]), (true, &[])],
        0,
        false,
    ),
    (&[(true, &[(true, &[0])])], 0, false),
    (
        &[(true, &[(true, &[1])]), (true, &[(true, &[0])])],
        0,
        false,
    ),
    (
        &[(true, &[(true, &[1])]), (true, &[(true, &[0])])],
        1,
        false,
    ),
    (
        &[
            (true, &[(true, &[1])]),
            (true, &[(true, &[2])]),
            (true, &[]),
        ],
        0,
        true,
    ),
    (
        &[
            (true, &[(true, &[1])]),
            (true, &[(true, &[2])]),
            (true, &[(true, &[1])]),
        ],
        0,
        false,
    ),
    (&[(true, &[(true, &[])])], 0, true),
];

#[cfg(test)]
mod tests {
    use super::*;

    /// The production recursion ([`with_stack`]) over a vector graph, as
    /// `Parser::classified` runs it over the run's candidates.
    fn contracted(
        graph: &[(bool, &[VectorInvocation])],
        candidate: usize,
        stack: &mut Vec<String>,
    ) -> bool {
        let Some((base, own)) = graph.get(candidate) else {
            return false;
        };
        if !base {
            return false;
        }
        if own.is_empty() {
            return true;
        }
        with_stack(stack, &candidate.to_string(), |stack| {
            for (admits, inner) in *own {
                if !admits {
                    return Err("an own invocation does not admit".into());
                }
                for next in *inner {
                    if !contracted(graph, *next, stack) {
                        return Err("an inner candidate is not contracted".into());
                    }
                }
            }
            Ok(())
        })
        .is_ok()
    }

    #[test]
    fn compose_vectors_hold() {
        for (graph, candidate, expected) in COMPOSE_VECTORS {
            assert_eq!(
                contracted(graph, *candidate, &mut Vec::new()),
                *expected,
                "{graph:?} {candidate}"
            );
        }
    }

    /// The Rust vectors are the Lean vectors, entry for entry.
    #[test]
    fn compose_vectors_are_the_lean_vectors() {
        let lean = include_str!("../../../proof/QCore/CandidateComposition.lean");
        let start = lean.find("def composeVectors").expect("the Lean vectors");
        let text = &lean[start
            ..lean[start..]
                .find("theorem")
                .map_or(lean.len(), |end| start + end)];
        let mut rendered = Vec::new();
        for (graph, candidate, expected) in COMPOSE_VECTORS {
            let candidates = graph
                .iter()
                .map(|(base, own)| {
                    let own = own
                        .iter()
                        .map(|(admits, inner)| {
                            format!(
                                "⟨{admits}, [{}]⟩",
                                inner
                                    .iter()
                                    .map(ToString::to_string)
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("⟨{base}, [{own}]⟩")
                })
                .collect::<Vec<_>>()
                .join(", ");
            rendered.push(format!("([{candidates}], {candidate}, {expected})"));
        }
        let lean_entries = text
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with("(["))
            .map(|line| line.trim_end_matches(" ]").trim_end_matches(',').to_owned())
            .collect::<Vec<_>>();
        assert_eq!(lean_entries, rendered);
    }

    #[test]
    fn a_requirement_strengthens_a_borrowed_formal() {
        let mut contract = ResourceFunctionContract {
            parameters: vec![
                ResourceParameterMode::Borrowed,
                ResourceParameterMode::Owned { nullable: true },
                ResourceParameterMode::Consumed { nullable: false },
            ],
            ..ResourceFunctionContract::default()
        };
        strengthen(&mut contract, 0, true);
        strengthen(&mut contract, 1, false);
        strengthen(&mut contract, 2, true);
        strengthen(&mut contract, 7, true);
        assert_eq!(
            contract.parameters,
            vec![
                ResourceParameterMode::Owned { nullable: true },
                ResourceParameterMode::Owned { nullable: false },
                ResourceParameterMode::Consumed { nullable: false },
            ]
        );
    }

    #[test]
    fn a_cycle_names_itself() {
        let mut stack = vec!["a".to_owned(), "b".to_owned()];
        let refused = with_stack(&mut stack, "a", |_| Ok(())).expect_err("a cycle");
        assert!(refused.contains("a -> b -> a"), "{refused}");
        assert_eq!(stack.len(), 2);
    }
}
