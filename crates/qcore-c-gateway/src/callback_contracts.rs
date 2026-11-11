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

//! Card **CG22-CALLBACK-REGISTRY-CONTRACT** (gateway rules 13u–13y): the
//! decision core of callback contracts (contract spec §C04.19), bound to
//! `QCore.CallbackContracts` by [`DECISION_VECTORS`] and
//! [`REGISTRY_VECTORS`].
//!
//! # Loop-safe composition (rules 13u, 13v)
//!
//! A synchronous invocation calls one of its candidates zero or more times.
//! The call is read as its custody contract alone exactly when every
//! candidate's own contract is the **identity** on the caller's state: as many
//! parameters as the clause has actuals, each `resource_borrowed` with no
//! kind, `resource_return_none()`, and no side clause. Then any number of
//! invocations, of any candidates, in any order, is the identity
//! (`QCore.CallbackContracts.invocations_of_identities_are_identity`), so the
//! loop the callee runs needs no fixed point. Every other case refuses, and
//! never by assuming a callback is benign:
//!
//! * an open candidate set (a formal, a member read through a pointer, a
//!   cast, a null pointer, a call's result, a table the body writes):
//!   [`UNDECIDED`];
//! * a candidate with no contract in force (no declaration, no validated
//!   summary; a withheld declaration is not one): [`UNCONTRACTED`];
//! * a candidate whose contract is not the identity: [`EFFECTFUL`].
//!
//! # Registries (rules 13w, 13x)
//!
//! A registration is in force at a holder only on a path that made it in this
//! body ([`crate::indirect_calls::callback_trace`]: an invocation needs the
//! registration in force and the context live). The registry's class is read
//! from its candidates' contracts at the element position: every candidate the
//! identity ([`RegistryClass::Neutral`]), every candidate releasing its one
//! element parameter ([`RegistryClass::Releasing`]), or no callback at all
//! ([`RegistryClass::None`]). An element added to a releasing registry moves
//! its custody into the holder (released when the registry is invoked over
//! its elements); an element added to any other registry is only retained
//! (C04.14). Anything else is [`REGISTRY_UNDECIDED`].

use std::collections::BTreeSet;

use qcore_schemas::callback::CallbackActual;
use qcore_schemas::contract::{
    ResourceFunctionContract, ResourceParameterMode, ResourceReturnMode,
};

/// The minted-name prefix is the retaining one: a callback declaration is a
/// retaining declaration ([`crate::retained_by::RETAINED_PREFIX`]).
///
/// The stable reason key of an open candidate set.
pub const UNDECIDED: &str = "callback-undecided";
/// The stable reason key of a candidate with no contract in force.
pub const UNCONTRACTED: &str = "callback-uncontracted";
/// The stable reason key of a candidate whose contract is not the identity.
pub const EFFECTFUL: &str = "callback-effectful";
/// The stable reason key of a registry use with no registration in force, or
/// a holder this body cannot name.
pub const REGISTRY_UNDECIDED: &str = "callback-registry-undecided";
/// The stable reason key of an element whose custody a registry would take
/// but which this body cannot hand it (a frame holder, a mixed registry).
pub const ELEMENT_UNDECIDED: &str = "callback-element-custody-undecided";
/// The stable reason key of a declaration whose body in this unit does not
/// validate its checked callback clause (rule 13y).
pub const WITHDRAWN: &str = "callback-withdrawn";
/// Card **CG22-OP-CALLBACKS-AND-SERVED-QUEUES** (rule 12m): a candidate that
/// does not take, on every return, the object the callee supplies it at a
/// `supplied_owned` position.
pub const SUPPLIED_KEPT: &str = "callback-supplied-kept";
/// Card **CG22-OP-CALLBACKS-AND-SERVED-QUEUES** (rule 12n): a member whose
/// stores do not close into a set of functions.
pub const MEMBER_OPEN: &str = "callback-member-open";
/// Card **CG22-OP-CALLBACKS-AND-SERVED-QUEUES** (rule 12o): a discriminant
/// value no arm declares.
pub const ARM_UNDECLARED: &str = "arm-undeclared";
/// Card **CG22-OP-CALLBACKS-AND-SERVED-QUEUES** (rule 12o): a tracked object
/// linked at a member some arms release and others do not.
pub const ARM_UNDECIDED: &str = "arm-undecided";
/// Card **CG22-OP-CALLBACKS-AND-SERVED-QUEUES** (rule 12o): a declaration
/// whose body in this unit does not validate its arms.
pub const ARM_WITHDRAWN: &str = "arm-withdrawn";

/// What one candidate's contract is to an invocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CandidateClass {
    /// No contract in force.
    Missing,
    /// The identity on the caller's state.
    Identity,
    /// The identity except that it releases what an `element` position
    /// receives (a registry's free callback).
    ReleasesElement,
    /// Anything else.
    Effectful,
    /// Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12m): the identity on
    /// the caller's state, taking (`resource_consumed` or
    /// `resource_released`, unconditionally) the object the callee supplies
    /// at its `supplied_owned` position.
    TakesSupplied,
    /// Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12m): the identity
    /// elsewhere, but it leaves the supplied object with the callee
    /// (`resource_borrowed` or `resource_owned` there), or takes it only on
    /// some return codes (a return-code disposition).
    KeepsSupplied,
    /// Card **SUPPLIED-OWNED-RETURN-CODE-HANDOFF** (rule 12m): a candidate
    /// that takes the object it is handed only where its result meets a
    /// guard, and whose invoker's branch on that result does not admit it
    /// (`crate::handoff_branch::admit`). An admitted one is classified by its
    /// custody contract with the handed formal read as taken (a
    /// `supplied_owned` position) or kept by the invoker (`_`, a parameter).
    HandOffUnbranched,
    /// Card **HANDOFF-CANDIDATE-OWN-CALLBACKS** (rule 13u amended): a
    /// candidate whose declaration carries its own member invocations, one
    /// of which does not decide over the run's set (or that is reached again
    /// while its own invocations are decided): no contract in force at this
    /// invocation, with the reason named
    /// ([`crate::candidate_composition::OWN_REFUSED`]).
    OwnInvocationRefused,
}

/// Whether a contract states no custody effect besides its modes.
fn plain(contract: &ResourceFunctionContract) -> bool {
    contract.parameter_kinds.is_empty()
        && contract.result_kind.is_heap()
        && contract.container_effects.is_empty()
        && contract.counted_users.is_empty()
        && contract.result == ResourceReturnMode::None
}

/// Classify a candidate's contract against the actuals of one invocation.
#[must_use]
pub fn classify(
    contract: Option<&ResourceFunctionContract>,
    actuals: &[CallbackActual],
) -> CandidateClass {
    let Some(contract) = contract else {
        return CandidateClass::Missing;
    };
    if !plain(contract) || contract.parameters.len() != actuals.len() {
        return CandidateClass::Effectful;
    }
    let mut releases = false;
    let mut takes = false;
    let mut keeps = false;
    for (mode, actual) in contract.parameters.iter().zip(actuals) {
        match (mode, actual) {
            (
                ResourceParameterMode::Consumed { .. } | ResourceParameterMode::Released { .. },
                CallbackActual::SuppliedOwned,
            ) => takes = true,
            (
                ResourceParameterMode::Borrowed | ResourceParameterMode::Owned { .. },
                CallbackActual::SuppliedOwned,
            ) => keeps = true,
            // `resource_owned` is a requirement (a live non-null object),
            // not an effect: the callee meets it with what it supplies, and
            // at a parameter position [`requirements`] puts it on the call.
            (ResourceParameterMode::Borrowed | ResourceParameterMode::Owned { .. }, _) => {}
            (ResourceParameterMode::Released { .. }, CallbackActual::Element) => releases = true,
            _ => return CandidateClass::Effectful,
        }
    }
    if keeps {
        CandidateClass::KeepsSupplied
    } else if takes {
        CandidateClass::TakesSupplied
    } else if releases {
        CandidateClass::ReleasesElement
    } else {
        CandidateClass::Identity
    }
}

/// The requirements a candidate's contract puts on the call's own
/// parameters: each parameter actual the candidate reads at a
/// `resource_owned(n)` position, with `n`. The call then reads that position
/// as `resource_owned(n)` itself (the requirement a direct call of the
/// candidate with that actual would carry); across candidates a position is
/// nullable only when every requirement on it is.
#[must_use]
pub fn requirements(
    contract: Option<&ResourceFunctionContract>,
    actuals: &[CallbackActual],
) -> Vec<(usize, bool)> {
    let Some(contract) = contract else {
        return Vec::new();
    };
    contract
        .parameters
        .iter()
        .zip(actuals)
        .filter_map(|(mode, actual)| match (mode, actual) {
            (ResourceParameterMode::Owned { nullable }, CallbackActual::Parameter { index }) => {
                Some((*index, *nullable))
            }
            _ => None,
        })
        .collect()
}

/// The decision of one invocation clause over its candidates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    /// Every candidate is the identity: the call reads as its custody
    /// contract.
    Identity,
    /// Every candidate releases the element it receives (a registry
    /// invocation only).
    ReleasesElements,
    /// Refused, with the reason key and the candidate it names.
    Refused(&'static str, Option<String>),
}

/// Card **OP-APP-RK-CALLBACKS-VIA-POLL-CB** (rule 12n, user decision E1): a
/// member invocation decided over the run's set (the pre-pass): an open set
/// is refused; a closed, empty set invokes nothing, so the call is the
/// identity; a closed non-empty set is decided as [`decide`] decides it
/// (`QCore.AppRkCallbacks.runDecision`). Only the run's set reads an empty
/// set so: a unit's own empty set is still refused by [`decide`].
#[must_use]
pub fn decide_run(candidates: Result<&[(String, CandidateClass)], ()>) -> Decision {
    match candidates {
        Ok([]) => Decision::Identity,
        other => decide(other, false),
    }
}

/// Decide an invocation: `candidates` is the resolution (`Err` an open set),
/// each with its class. `elements` admits [`CandidateClass::ReleasesElement`]
/// (a registry invocation over its elements).
#[must_use]
pub fn decide(candidates: Result<&[(String, CandidateClass)], ()>, elements: bool) -> Decision {
    let Ok(candidates) = candidates else {
        return Decision::Refused(UNDECIDED, None);
    };
    if candidates.is_empty() {
        return Decision::Refused(UNDECIDED, None);
    }
    if let Some((name, _)) = candidates
        .iter()
        .find(|(_, class)| *class == CandidateClass::Missing)
    {
        return Decision::Refused(UNCONTRACTED, Some(name.clone()));
    }
    // Card HANDOFF-CANDIDATE-OWN-CALLBACKS (rule 13u amended): a composed
    // candidate one of whose own invocations does not decide.
    if let Some((name, _)) = candidates
        .iter()
        .find(|(_, class)| *class == CandidateClass::OwnInvocationRefused)
    {
        return Decision::Refused(
            crate::candidate_composition::OWN_REFUSED,
            Some(name.clone()),
        );
    }
    // Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF (rule 12m): a guarded hand-off
    // the invoker's branch does not admit keeps today's refusal.
    if let Some((name, _)) = candidates
        .iter()
        .find(|(_, class)| *class == CandidateClass::HandOffUnbranched)
    {
        return Decision::Refused(crate::handoff_branch::UNBRANCHED, Some(name.clone()));
    }
    if let Some((name, _)) = candidates.iter().find(|(_, class)| {
        *class == CandidateClass::Effectful
            || (!elements && *class == CandidateClass::ReleasesElement)
            || (elements && *class == CandidateClass::TakesSupplied)
    }) {
        return Decision::Refused(EFFECTFUL, Some(name.clone()));
    }
    // Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (rule 12m): a callback that
    // leaves the supplied object with the callee is refused, never read as
    // taking it.
    if let Some((name, _)) = candidates
        .iter()
        .find(|(_, class)| *class == CandidateClass::KeepsSupplied)
    {
        return Decision::Refused(SUPPLIED_KEPT, Some(name.clone()));
    }
    let releasing = candidates
        .iter()
        .filter(|(_, class)| *class == CandidateClass::ReleasesElement)
        .count();
    if releasing == 0 {
        Decision::Identity
    } else if releasing == candidates.len() {
        Decision::ReleasesElements
    } else {
        // Some candidates release the elements and some do not: whether an
        // element is still owed afterwards is not decided.
        Decision::Refused(ELEMENT_UNDECIDED, None)
    }
}

/// What a registration in force holds, for element custody.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RegistryClass {
    /// No callback (a null pointer was registered).
    None,
    /// Every candidate is the identity over one element.
    Neutral,
    /// Every candidate releases its one element.
    Releasing,
    /// Anything else: a mix, an effectful or uncontracted candidate.
    Undecided,
}

/// The registry class of a registration's candidates, read at one `element`
/// position (the free callback's shape).
#[must_use]
pub fn registry_class(candidates: &[(String, CandidateClass)]) -> RegistryClass {
    if candidates.is_empty() {
        return RegistryClass::None;
    }
    if candidates
        .iter()
        .all(|(_, class)| *class == CandidateClass::Identity)
    {
        RegistryClass::Neutral
    } else if candidates
        .iter()
        .all(|(_, class)| *class == CandidateClass::ReleasesElement)
    {
        RegistryClass::Releasing
    } else {
        RegistryClass::Undecided
    }
}

/// The refusal text of one callback clause at a call.
#[must_use]
pub fn refusal(callee: &str, detail: &str, reason: &str) -> String {
    format!("call to `{callee}`: {detail} ({reason})")
}

/// The detail of an invocation refusal.
#[must_use]
pub fn invocation_detail(reason: &str, candidate: Option<&str>, actual: &str) -> String {
    match (reason, candidate) {
        (UNCONTRACTED, Some(name)) => {
            format!("callback `{name}` (from `{actual}`) has no contract in force")
        }
        (EFFECTFUL, Some(name)) => {
            format!("callback `{name}` (from `{actual}`) is not custody-neutral per call")
        }
        (ELEMENT_UNDECIDED, _) => {
            format!("callbacks from `{actual}` do not agree on the elements' custody")
        }
        (SUPPLIED_KEPT, Some(name)) => format!(
            "callback `{name}` (from `{actual}`) does not take, on every return, the object the \
             callee supplies it"
        ),
        (crate::candidate_composition::OWN_REFUSED, Some(name)) => {
            format!("callback `{name}` (from `{actual}`) does not decide its own invocations")
        }
        (crate::handoff_branch::UNBRANCHED, Some(name)) => format!(
            "callback `{name}` (from `{actual}`) takes the object it is handed only on some \
             return codes, and the invoker's branch on its result does not admit that"
        ),
        _ => format!("the callback `{actual}` is not a closed set of functions"),
    }
}

/// One event of a registry's life on a path, for [`REGISTRY_VECTORS`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistryEvent {
    /// A registration of a callback of the class.
    Register(RegistryClass),
    /// A tracked element added.
    Add,
    /// The registry invoked over its elements.
    Invoke,
    /// The registry's elements dropped and its registration removed.
    Clear,
    /// The function exits.
    Exit,
}

/// The verdict of one registry trace (the model of rules 13w and 13x for
/// one holder that is a tracked identity and one tracked element):
/// `Some(true)` safe, `Some(false)` refused, `None` an element still owed.
#[must_use]
pub fn registry_trace(events: &[RegistryEvent]) -> Option<bool> {
    let mut registered: Option<RegistryClass> = None;
    // The element's state: owned by the function, held by the registry
    // (custody moved), retained by it, released.
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Element {
        Owned,
        Moved,
        Retained,
        Released,
        Absent,
    }
    let mut element = Element::Absent;
    for event in events {
        match event {
            RegistryEvent::Register(class) => registered = Some(*class),
            RegistryEvent::Add => {
                if element != Element::Absent {
                    return Some(false);
                }
                element = match registered {
                    Some(RegistryClass::Releasing) => Element::Moved,
                    Some(RegistryClass::None | RegistryClass::Neutral) => Element::Retained,
                    _ => return Some(false),
                };
            }
            RegistryEvent::Invoke => match registered {
                Some(RegistryClass::Releasing) => {
                    if element == Element::Moved {
                        element = Element::Released;
                    }
                }
                Some(RegistryClass::None | RegistryClass::Neutral) => {}
                _ => return Some(false),
            },
            RegistryEvent::Clear => {
                if registered.is_none() {
                    return Some(false);
                }
                if element == Element::Retained {
                    element = Element::Owned;
                }
                if element == Element::Moved {
                    // Dropped from a releasing registry without its
                    // invocation: lost.
                    return None;
                }
                registered = None;
            }
            RegistryEvent::Exit => {
                return match element {
                    Element::Moved => None,
                    // A retained element at the exit: the 12t exit refusal
                    // (a frame holder dies with the frame; a tracked holder
                    // still holding refuses).
                    Element::Retained => Some(false),
                    Element::Owned | Element::Released | Element::Absent => Some(true),
                };
            }
        }
    }
    Some(true)
}

/// `DECISION_VECTORS`: (open?, candidate classes, elements?, decision kind:
/// 0 identity, 1 releases elements, 2 refused), shared with
/// `QCore.CallbackContracts.decisionVectors`.
#[cfg(test)]
pub(crate) const DECISION_VECTORS: &[(bool, &[CandidateClass], bool, u8)] = &[
    (true, &[CandidateClass::Identity], false, 2),
    (false, &[], false, 2),
    (false, &[CandidateClass::Identity], false, 0),
    (
        false,
        &[CandidateClass::Identity, CandidateClass::Identity],
        false,
        0,
    ),
    (
        false,
        &[CandidateClass::Identity, CandidateClass::Missing],
        false,
        2,
    ),
    (false, &[CandidateClass::Effectful], false, 2),
    (false, &[CandidateClass::ReleasesElement], false, 2),
    (false, &[CandidateClass::ReleasesElement], true, 1),
    (
        false,
        &[CandidateClass::ReleasesElement, CandidateClass::Identity],
        true,
        2,
    ),
    (false, &[CandidateClass::Identity], true, 0),
];

/// Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES: `SUPPLIED_DECISION_VECTORS`
/// (open?, candidate classes, decision kind), shared with
/// `QCore.OpCallbacks.suppliedDecisionVectors`; outside a registry.
#[cfg(test)]
pub(crate) const SUPPLIED_DECISION_VECTORS: &[(bool, &[CandidateClass], u8)] = &[
    (false, &[CandidateClass::TakesSupplied], 0),
    (
        false,
        &[CandidateClass::TakesSupplied, CandidateClass::TakesSupplied],
        0,
    ),
    (
        false,
        &[CandidateClass::TakesSupplied, CandidateClass::KeepsSupplied],
        2,
    ),
    (
        false,
        &[CandidateClass::TakesSupplied, CandidateClass::Missing],
        2,
    ),
    (false, &[CandidateClass::Effectful], 2),
    (true, &[CandidateClass::TakesSupplied], 2),
    (false, &[], 2),
];

/// `REGISTRY_VECTORS`: (trace, verdict), shared with
/// `QCore.CallbackContracts.registryVectors`.
#[cfg(test)]
pub(crate) const REGISTRY_VECTORS: &[(&[RegistryEvent], Option<bool>)] = {
    use RegistryClass as C;
    use RegistryEvent as E;
    &[
        (
            &[
                E::Register(C::Releasing),
                E::Add,
                E::Invoke,
                E::Clear,
                E::Exit,
            ],
            Some(true),
        ),
        (&[E::Register(C::Releasing), E::Add, E::Exit], None),
        (
            &[E::Register(C::Releasing), E::Add, E::Clear, E::Exit],
            None,
        ),
        (&[E::Add, E::Exit], Some(false)),
        (&[E::Register(C::Undecided), E::Add, E::Exit], Some(false)),
        (&[E::Register(C::None), E::Add, E::Exit], Some(false)),
        (
            &[E::Register(C::None), E::Add, E::Clear, E::Exit],
            Some(true),
        ),
        (
            &[E::Register(C::Neutral), E::Invoke, E::Clear, E::Exit],
            Some(true),
        ),
        (&[E::Invoke, E::Exit], Some(false)),
        (
            &[E::Register(C::Undecided), E::Invoke, E::Exit],
            Some(false),
        ),
    ]
};

/// The kinds of the decision, as the vectors spell them.
#[cfg(test)]
fn decision_kind(decision: &Decision) -> u8 {
    match decision {
        Decision::Identity => 0,
        Decision::ReleasesElements => 1,
        Decision::Refused(..) => 2,
    }
}

/// Every name in a candidate list (for reports).
#[must_use]
pub fn names(candidates: &[(String, CandidateClass)]) -> BTreeSet<&str> {
    candidates.iter().map(|(name, _)| name.as_str()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decision_vectors_hold() {
        for (open, classes, elements, kind) in DECISION_VECTORS {
            let candidates = classes
                .iter()
                .enumerate()
                .map(|(at, class)| (format!("f{at}"), *class))
                .collect::<Vec<_>>();
            let resolution = if *open {
                Err(())
            } else {
                Ok(candidates.as_slice())
            };
            assert_eq!(
                decision_kind(&decide(resolution, *elements)),
                *kind,
                "{open} {classes:?} {elements}"
            );
        }
        assert_eq!(DECISION_VECTORS.len(), 10);
    }

    #[test]
    fn registry_vectors_hold() {
        for (trace, verdict) in REGISTRY_VECTORS {
            assert_eq!(registry_trace(trace), *verdict, "{trace:?}");
        }
        assert_eq!(REGISTRY_VECTORS.len(), 10);
    }

    const LEAN: &str = include_str!("../../../proof/QCore/CallbackContracts.lean");

    fn lean_class(class: CandidateClass) -> &'static str {
        match class {
            CandidateClass::Missing => ".missing",
            CandidateClass::Identity => ".identity",
            CandidateClass::ReleasesElement => ".releasesElement",
            CandidateClass::Effectful => ".effectful",
            CandidateClass::TakesSupplied => ".takesSupplied",
            CandidateClass::KeepsSupplied => ".keepsSupplied",
            // Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF: modelled by
            // `QCore.ReturnCodeHandOff`, in no vector of this module.
            CandidateClass::HandOffUnbranched => ".handOffUnbranched",
            // Card HANDOFF-CANDIDATE-OWN-CALLBACKS: modelled by
            // `QCore.CandidateComposition`, in no vector of this module.
            CandidateClass::OwnInvocationRefused => ".ownInvocationRefused",
        }
    }

    fn lean_registry(class: RegistryClass) -> &'static str {
        match class {
            RegistryClass::None => ".none",
            RegistryClass::Neutral => ".neutral",
            RegistryClass::Releasing => ".releasing",
            RegistryClass::Undecided => ".undecided",
        }
    }

    /// The rows of the Lean list that opens with `header`, one per line.
    fn lean_rows(header: &str) -> Vec<String> {
        let start = LEAN.find(header).expect("the Lean vector list") + header.len();
        let body = &LEAN[start..];
        let end = body.find("\n]").expect("the list's end");
        body[..end]
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| line.trim_end_matches(',').to_owned())
            .collect()
    }

    #[test]
    fn decision_vectors_are_the_lean_vectors() {
        let rows =
            lean_rows("def decisionVectors : List (Bool × List CandidateClass × Bool × Nat) := [");
        let rust = DECISION_VECTORS
            .iter()
            .map(|(open, classes, elements, kind)| {
                format!(
                    "({open}, [{}], {elements}, {kind})",
                    classes
                        .iter()
                        .map(|class| lean_class(*class))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(rows, rust);
    }

    #[test]
    fn registry_vectors_are_the_lean_vectors() {
        let rows = lean_rows("def registryVectors : List (List Event × Option Bool) := [");
        let rust = REGISTRY_VECTORS
            .iter()
            .map(|(trace, verdict)| {
                let events = trace
                    .iter()
                    .map(|event| match event {
                        RegistryEvent::Register(class) => {
                            format!(".register {}", lean_registry(*class))
                        }
                        RegistryEvent::Add => ".add".to_owned(),
                        RegistryEvent::Invoke => ".invoke".to_owned(),
                        RegistryEvent::Clear => ".clear".to_owned(),
                        RegistryEvent::Exit => ".exit".to_owned(),
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                let verdict = match verdict {
                    Some(value) => format!("some {value}"),
                    None => "Option.none".to_owned(),
                };
                format!("([{events}], {verdict})")
            })
            .collect::<Vec<_>>();
        assert_eq!(rows, rust);
    }

    /// `CLASSIFY_VECTORS` (`QCore.CallbackContracts.classifyVectors`): the
    /// positions of a plain contract of the right arity and its class. A mode
    /// is `borrowed`, `owned` (`resource_owned(nonnull)`), `released`
    /// (`resource_released(nullable)`) or `other` (`resource_consumed`).
    const CLASSIFY_VECTORS: &[(&[(&str, CallbackActual)], CandidateClass)] = &[
        (
            &[
                ("borrowed", CallbackActual::Parameter { index: 0 }),
                ("borrowed", CallbackActual::Supplied),
                ("borrowed", CallbackActual::Parameter { index: 2 }),
            ],
            CandidateClass::Identity,
        ),
        (
            &[
                ("owned", CallbackActual::Supplied),
                ("owned", CallbackActual::Supplied),
            ],
            CandidateClass::Identity,
        ),
        (
            &[
                ("owned", CallbackActual::Parameter { index: 0 }),
                ("borrowed", CallbackActual::Supplied),
            ],
            CandidateClass::Identity,
        ),
        (
            &[("released", CallbackActual::Element)],
            CandidateClass::ReleasesElement,
        ),
        (
            &[("released", CallbackActual::Supplied)],
            CandidateClass::Effectful,
        ),
        (
            &[("released", CallbackActual::Parameter { index: 0 })],
            CandidateClass::Effectful,
        ),
        (
            &[("other", CallbackActual::Supplied)],
            CandidateClass::Effectful,
        ),
        (&[], CandidateClass::Identity),
    ];

    /// Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES: `SUPPLIED_CLASSIFY_VECTORS`
    /// (`QCore.OpCallbacks.suppliedClassifyVectors`).
    const SUPPLIED_CLASSIFY_VECTORS: &[(&[(&str, CallbackActual)], CandidateClass)] = &[
        (
            &[
                ("borrowed", CallbackActual::Supplied),
                ("released", CallbackActual::SuppliedOwned),
                ("borrowed", CallbackActual::Parameter { index: 5 }),
            ],
            CandidateClass::TakesSupplied,
        ),
        (
            &[("other", CallbackActual::SuppliedOwned)],
            CandidateClass::TakesSupplied,
        ),
        (
            &[("borrowed", CallbackActual::SuppliedOwned)],
            CandidateClass::KeepsSupplied,
        ),
        (
            &[("owned", CallbackActual::SuppliedOwned)],
            CandidateClass::KeepsSupplied,
        ),
        (
            &[
                ("released", CallbackActual::SuppliedOwned),
                ("released", CallbackActual::Supplied),
            ],
            CandidateClass::Effectful,
        ),
        (
            &[
                ("borrowed", CallbackActual::Parameter { index: 0 }),
                ("owned", CallbackActual::Supplied),
            ],
            CandidateClass::Identity,
        ),
    ];

    const OP_LEAN: &str = include_str!("../../../proof/QCore/OpCallbacks.lean");

    fn op_lean_rows(header: &str) -> Vec<String> {
        let start = OP_LEAN.find(header).expect("the Lean vector list") + header.len();
        let body = &OP_LEAN[start..];
        let end = body.find("\n]").expect("the list's end");
        body[..end]
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| line.trim_end_matches(',').to_owned())
            .collect()
    }

    fn render_actual(actual: &CallbackActual) -> String {
        match actual {
            CallbackActual::Parameter { index } => format!(".parameter {index}"),
            CallbackActual::Supplied => ".supplied".to_owned(),
            CallbackActual::Element => ".element".to_owned(),
            CallbackActual::SuppliedOwned => ".suppliedOwned".to_owned(),
        }
    }

    #[test]
    fn supplied_vectors_hold_and_are_the_lean_vectors() {
        let rows = op_lean_rows(
            "def suppliedClassifyVectors : List (List (Mode × SActual) × SuppliedClass) := [",
        );
        let mut rendered = Vec::new();
        for (positions, class) in SUPPLIED_CLASSIFY_VECTORS {
            let contract = ResourceFunctionContract {
                parameters: positions.iter().map(|(m, _)| mode(m)).collect(),
                result: ResourceReturnMode::None,
                ..ResourceFunctionContract::default()
            };
            let actuals = positions.iter().map(|(_, a)| *a).collect::<Vec<_>>();
            assert_eq!(classify(Some(&contract), &actuals), *class, "{positions:?}");
            rendered.push(format!(
                "([{}], {})",
                positions
                    .iter()
                    .map(|(m, a)| format!("(.{m}, {})", render_actual(a)))
                    .collect::<Vec<_>>()
                    .join(", "),
                lean_class(*class)
            ));
        }
        assert_eq!(rows, rendered);
        let rows = op_lean_rows(
            "def suppliedDecisionVectors : List (Bool × List SuppliedClass × Nat) := [",
        );
        let mut rendered = Vec::new();
        for (open, classes, kind) in SUPPLIED_DECISION_VECTORS {
            let candidates = classes
                .iter()
                .enumerate()
                .map(|(at, class)| (format!("f{at}"), *class))
                .collect::<Vec<_>>();
            let resolution = if *open {
                Err(())
            } else {
                Ok(candidates.as_slice())
            };
            assert_eq!(decision_kind(&decide(resolution, false)), *kind);
            rendered.push(format!(
                "({open}, [{}], {kind})",
                classes
                    .iter()
                    .map(|class| lean_class(*class))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        assert_eq!(rows, rendered);
    }

    fn mode(spelling: &str) -> ResourceParameterMode {
        match spelling {
            "borrowed" => ResourceParameterMode::Borrowed,
            "owned" => ResourceParameterMode::Owned { nullable: false },
            "released" => ResourceParameterMode::Released { nullable: true },
            _ => ResourceParameterMode::Consumed { nullable: false },
        }
    }

    #[test]
    fn classify_vectors_hold_and_are_the_lean_vectors() {
        let rows =
            lean_rows("def classifyVectors : List (List (Mode × Actual) × CandidateClass) := [");
        let mut rendered = Vec::new();
        for (positions, class) in CLASSIFY_VECTORS {
            let contract = ResourceFunctionContract {
                parameters: positions.iter().map(|(m, _)| mode(m)).collect(),
                result: ResourceReturnMode::None,
                ..ResourceFunctionContract::default()
            };
            let actuals = positions.iter().map(|(_, a)| *a).collect::<Vec<_>>();
            assert_eq!(classify(Some(&contract), &actuals), *class, "{positions:?}");
            rendered.push(format!(
                "([{}], {})",
                positions
                    .iter()
                    .map(|(m, a)| format!(
                        "(.{m}, {})",
                        match a {
                            CallbackActual::Parameter { index } => format!(".parameter {index}"),
                            CallbackActual::Supplied => ".supplied".to_owned(),
                            CallbackActual::Element => ".element".to_owned(),
                            CallbackActual::SuppliedOwned => ".suppliedOwned".to_owned(),
                        }
                    ))
                    .collect::<Vec<_>>()
                    .join(", "),
                lean_class(*class)
            ));
        }
        assert_eq!(rows, rendered);
        // The requirement of the third vector: parameter 0, non-null.
        let owned = ResourceFunctionContract {
            parameters: vec![
                ResourceParameterMode::Owned { nullable: false },
                ResourceParameterMode::Borrowed,
            ],
            result: ResourceReturnMode::None,
            ..ResourceFunctionContract::default()
        };
        assert_eq!(
            requirements(
                Some(&owned),
                &[
                    CallbackActual::Parameter { index: 0 },
                    CallbackActual::Supplied
                ]
            ),
            [(0, false)]
        );
    }

    #[test]
    fn classification_reads_the_modes() {
        let borrowed = ResourceFunctionContract {
            parameters: vec![ResourceParameterMode::Borrowed; 3],
            result: ResourceReturnMode::None,
            ..ResourceFunctionContract::default()
        };
        let actuals = [
            CallbackActual::Parameter { index: 0 },
            CallbackActual::Supplied,
            CallbackActual::Parameter { index: 2 },
        ];
        assert_eq!(
            classify(Some(&borrowed), &actuals),
            CandidateClass::Identity
        );
        assert_eq!(
            classify(Some(&borrowed), &actuals[..2]),
            CandidateClass::Effectful
        );
        assert_eq!(classify(None, &actuals), CandidateClass::Missing);
        let free = ResourceFunctionContract {
            parameters: vec![ResourceParameterMode::Released { nullable: true }],
            result: ResourceReturnMode::None,
            ..ResourceFunctionContract::default()
        };
        assert_eq!(
            classify(Some(&free), &[CallbackActual::Element]),
            CandidateClass::ReleasesElement
        );
        assert_eq!(
            classify(Some(&free), &[CallbackActual::Supplied]),
            CandidateClass::Effectful
        );
        let owned_result = ResourceFunctionContract {
            parameters: vec![ResourceParameterMode::Borrowed],
            result: ResourceReturnMode::Owned { nullable: true },
            ..ResourceFunctionContract::default()
        };
        assert_eq!(
            classify(Some(&owned_result), &[CallbackActual::Supplied]),
            CandidateClass::Effectful
        );
    }
}
