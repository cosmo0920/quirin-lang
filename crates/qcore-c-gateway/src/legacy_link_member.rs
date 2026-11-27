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

//! Card **LERI-HELD-AND-LINK-MEMBER-PREMISES** (gateway rule 46): the two
//! premises rule 43 stated about an element destroyer's unlink, checked.
//!
//! An element destroyer (`resource_unpublished(p)` beside
//! `resource_released(p)`, rule 34) runs `mk_list_del(&p->M)` and then frees
//! `p`. Rule 43 admitted two entries on premises it did not check:
//!
//! * **P-LERI-LINK-MEMBER.** A foreign element `e`, bound by
//!   `e = mk_list_entry(it, T, F)` over a list the caller iterates, is removed
//!   from that list by the destroyer only when the destroyer unlinks the same
//!   member (`M == F`); otherwise the element stays linked there while it is
//!   freed (`flb_tail_file_remove` unlinks `_rotate_head` only while
//!   `file->rotated > 0`, and always `_head`). The declaration now names `M`
//!   (`resource_unpublished(p, M)`, contract specification §C04.8), and the
//!   destroyer takes a foreign element only when every `container_of`
//!   binding of it names exactly `M` ([`foreign_admits`]). A declaration that
//!   names no member, or a binding through another member, refuses.
//! * **P-LERI-HELD-LINKS.** An object the path holds (live, unpublished,
//!   never unlinked) has links an unlink may remove only when something
//!   initialised them: a fresh allocation's links are indeterminate, and a
//!   zeroed one's are `NULL`, which `mk_list_del` dereferences. The obligation
//!   is now decided from the body's own initialisation: a rule 36b
//!   `TrackedSelfInit` whose path is the one member `M` and whose self-writes
//!   cover both link fields (`prev`, `next`, the fields `mk_list_del` reads)
//!   records `(site, M)` ([`initialised_member`], [`record_initialisation`]),
//!   and the held arm admits only an object with `(site, M)` recorded for the
//!   destroyer's declared member ([`held_admits`]). The fact joins by
//!   intersection (initialised on every arriving path) and is forgotten at
//!   every statement that may write the object's memory ([`retain`]): only
//!   statements that write no memory keep it (a copy, clear or acquisition
//!   only into a plain name), and a member-place copy or clear and a contract
//!   call keep it only when every written place's and place actual's root
//!   names a binding that cannot denote the object. Card
//!   **LINK-RECORD-KILL-POLICY-SCALAR-CALLS** refines the call arm: an actual
//!   that denotes no object (a literal, a predefined identifier, an admitted
//!   dereference of a neutral nested call's scalar result: [`Kind::Scalar`])
//!   adds nothing, a nested call adds what its place actuals may denote (read
//!   before the call, [`before_nested`]), and any other actual still reaches
//!   every object ([`decided`]; `QCore.ScalarCallKillPolicy`).
//!   A formal, a callee-constructed object and
//!   an object initialised any other way hold no fact, so the held arm
//!   refuses them (`Unknown`).
//!
//! Both refusals are the unlink's own refusal (an unresolved effect on the
//! entry), with a diagnostic naming the undecided obligation. Nothing here
//! admits an entry rule 43 refused; every change replaces an admission with a
//! refusal. Not switchable: a switch would re-admit an unchecked premise.
//! The Lean model is `QCore.ElementLinkPremises`.

use std::collections::BTreeSet;

use super::{AbstractState, LegacyInstruction};

/// The link fields of the intrusive list record `mk_list_del` reads before
/// it writes (`entry->prev`, `entry->next`; Monkey's `mk_list` and CFL's
/// `cfl_list`, as `monkey/intrusive_list.qv` quotes the body).
pub(crate) const LINK_FIELDS: [&str; 2] = ["next", "prev"];

/// The member a checked self-initialisation makes unlinkable: its path is one
/// member and its self-writes cover both link fields.
pub(crate) fn initialised_member<'a>(path: &'a [String], writes: &[String]) -> Option<&'a str> {
    match path {
        [member]
            if LINK_FIELDS
                .iter()
                .all(|field| writes.iter().any(|write| write == field)) =>
        {
            Some(member.as_str())
        }
        _ => None,
    }
}

/// P-LERI-LINK-MEMBER: a destroyer takes a foreign element only when it
/// names its link member and every `container_of` binding of the element
/// names that member.
pub(crate) fn foreign_admits(link: Option<&str>, members: &BTreeSet<String>) -> bool {
    link.is_some_and(|link| !members.is_empty() && members.iter().all(|member| member == link))
}

/// [`foreign_admits`] for a foreign element, when there is one (`None`: no
/// element, so nothing to refuse here).
pub(crate) fn admits(foreign: Option<&super::Foreign>, link: Option<&str>) -> bool {
    foreign.is_none_or(|foreign| foreign_admits(link, &foreign.members))
}

/// The refusal of a foreign element a destroyer may not take.
pub(crate) fn foreign_refusal(
    called: &str,
    name: &str,
    foreign: Option<&super::Foreign>,
    link: Option<&str>,
) -> String {
    let members = foreign
        .map(|foreign| {
            foreign
                .members
                .iter()
                .map(|member| format!("`{member}`"))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let declared = link.map_or_else(
        || "names no link member".to_owned(),
        |link| format!("unlinks `{link}`"),
    );
    format!(
        "unlink by `{called}` of `{name}`, whose list links it through {members}, while the declaration {declared}"
    )
}

/// The refusal of a held object whose links this body did not initialise.
pub(crate) fn held_refusal(called: &str, name: &str, link: Option<&str>) -> String {
    let reason = link.map_or_else(
        || "the declaration names no link member".to_owned(),
        |link| format!("this function has not initialised its `{link}` links"),
    );
    format!("unlink by `{called}` of `{name}`, which this function holds, but {reason}")
}

/// P-LERI-HELD-LINKS, the decision: a held object's declared link member is
/// one this path initialised.
pub(crate) fn held_decides(link: Option<&str>, ready: &BTreeSet<String>) -> bool {
    link.is_some_and(|link| ready.contains(link))
}

/// P-LERI-HELD-LINKS at `site`.
pub(in crate::legacy_c) fn held_admits(
    state: &AbstractState,
    site: usize,
    link: Option<&str>,
) -> bool {
    let ready = state
        .link_ready
        .iter()
        .filter(|(held, _)| *held == site)
        .map(|(_, member)| member.clone())
        .collect::<BTreeSet<_>>();
    held_decides(link, &ready)
}

/// An admitted rule 36b self-initialisation of `base`'s member `path`.
pub(in crate::legacy_c) fn record_initialisation(
    state: &mut AbstractState,
    base: &str,
    path: &[String],
    writes: &[String],
) {
    let Some(member) = initialised_member(path, writes) else {
        return;
    };
    let Some(pointer) = state.pointers.get(base) else {
        return;
    };
    if pointer.unknown
        || pointer.may_be_null
        || !pointer.derivation.is_identity()
        || pointer.targets.len() != 1
    {
        return;
    }
    let site = *pointer.targets.first().expect("one target");
    if state.resources.get(&site) == Some(&super::LIVE) {
        state.link_ready.insert((site, member.to_owned()));
    }
}

/// The root name of an actual's spelling (`x` of `&x->f`, `(x)`, `x`), or
/// `None` when it starts with no identifier (a literal).
fn root(actual: &str) -> Option<&str> {
    let trimmed = actual.trim_start_matches(['&', '(', '*', ' ']);
    let end = trimmed
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(trimmed.len());
    let name = &trimmed[..end];
    (!name.is_empty() && !name.starts_with(|c: char| c.is_ascii_digit())).then_some(name)
}

/// Whether `place` is a plain name (a variable), whose write changes no
/// object's memory.
fn plain(place: &str) -> bool {
    !place.is_empty()
        && place
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

/// What a statement may write, for the link records it may invalidate.
/// Card FB-RETRY-OR6-ELEMENT-IDENTITY (gateway rule 91) reads the same
/// policy for its carried records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Reach {
    /// No object's memory.
    Nothing,
    /// Only objects at these sites.
    Sites(BTreeSet<usize>),
    /// Possibly any object.
    Everything,
}

/// The objects one of `places` may reach: the sites its root's binding
/// denotes, or every object when a root has no binding or an unknown one. A
/// place with no identifier root (a literal) reaches none.
pub(in crate::legacy_c) fn reach_places<'a>(
    state: &AbstractState,
    places: impl IntoIterator<Item = &'a str>,
) -> Reach {
    let mut reached = BTreeSet::new();
    for place in places {
        let Some(name) = root(place) else {
            continue;
        };
        match state.pointers.get(name) {
            Some(pointer) if !pointer.unknown => reached.extend(pointer.targets.iter().copied()),
            _ => return Reach::Everything,
        }
    }
    Reach::Sites(reached)
}

/// One actual of a contract call as the kill policy reads it (card
/// LINK-RECORD-KILL-POLICY-SCALAR-CALLS; `QCore.ScalarCallKillPolicy.Kind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A place: the call may write the objects its root binding denotes.
    Place,
    /// A value that denotes no object: a literal or predefined identifier
    /// (premise T6), or an admitted dereference of a neutral nested call's
    /// scalar result (premises N4-D, N10).
    Scalar,
    /// Anything else (a cast, an address, an expression, a nested call's
    /// undereferenced result, an inner actual read as data): it may carry a
    /// pointer to any object.
    Undecided,
}

/// Rule 46's kill policy before this card: a contract call's writes are
/// decided from its actuals only when it has no nested call and every actual
/// is a place.
#[cfg(test)]
pub(crate) fn decided_before(actuals: &[Kind], nested: &[&[Kind]]) -> bool {
    nested.is_empty() && actuals.iter().all(|kind| *kind == Kind::Place)
}

/// The refined policy: every actual is a place or a scalar, and every
/// nested call's own actuals are places. A scalar adds no reached object,
/// and a nested call adds the objects its places denote.
pub(crate) fn decided(actuals: &[Kind], nested: &[&[Kind]]) -> bool {
    actuals.iter().all(|kind| *kind != Kind::Undecided)
        && nested
            .iter()
            .all(|inner| inner.iter().all(|kind| *kind == Kind::Place))
}

/// The positions of a call's actuals that denote no object
/// ([`Kind::Scalar`]), recorded at classification: an actual that is a
/// literal or predefined identifier (premise T6; the reader spells a string
/// literal as one identifier-shaped token, so such a position may carry a
/// place that names no binding), or, at a position with no place, an
/// admitted nested call's dereferenced result, or a whole parenthesised
/// `sizeof` ([`is_sizeof_actual`]). Every other position keeps its reading.
pub(crate) fn scalar_actuals(
    raw: Option<&[&[&str]]>,
    arguments: &[Option<String>],
    nested: &[crate::variadic_nested::NestedCallActual],
) -> Vec<usize> {
    arguments
        .iter()
        .enumerate()
        .filter(|(index, place)| {
            raw.and_then(|raw| raw.get(*index)).is_some_and(|actual| {
                is_literal_or_predefined_actual(actual) || is_sizeof_actual(actual)
            }) || (place.is_none()
                && nested
                    .iter()
                    .any(|nested| nested.argument == *index && nested.dereferenced))
        })
        .map(|(index, _)| index)
        .collect()
}

/// Premise T6 (`variadic_nested`'s `is_literal_or_predefined_actual`,
/// `QCore.LiteralTailActual`), restated here because that module's source is
/// a pinned reading-site artifact: every token is a string or character
/// literal (one identifier-shaped token), a numeric literal, or a C11
/// 6.4.2.2 predefined identifier. Neither names a declaration or binding.
fn is_literal_or_predefined_actual(actual: &[&str]) -> bool {
    !actual.is_empty()
        && actual.iter().all(|text| {
            *text == crate::custody_free_copy::LITERAL
                || crate::retained_by::PREDEFINED_IDENTIFIERS.contains(text)
                || (text.as_bytes().first().is_some_and(u8::is_ascii_digit)
                    && text.bytes().all(|byte| byte.is_ascii_alphanumeric()))
        })
}

/// An actual that is exactly `sizeof ( … )`: the operator applies to the
/// whole parenthesised operand, which closes the actual, and the operand
/// carries no call, assignment, increment or decrement. Its value is a
/// `size_t` that denotes no object. The operand is not evaluated unless its
/// type is variably modified (C11 6.5.3.4p2), and evaluating an operand with
/// no call, assignment, increment or decrement writes no memory, so the
/// actual writes nothing either way. `sizeof (T) + p`, `sizeof x` and every
/// other spelling stay undecided.
pub(crate) fn is_sizeof_actual(actual: &[&str]) -> bool {
    let [first, open, operand @ .., close] = actual else {
        return false;
    };
    if *first != "sizeof" || *open != "(" || *close != ")" || operand.is_empty() {
        return false;
    }
    // The opening parenthesis must close only at the end of the actual.
    let mut depth = 1usize;
    for token in operand {
        match *token {
            "(" => depth += 1,
            ")" => {
                depth -= 1;
                if depth == 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 1
        && !operand.contains(&"=")
        && !operand
            .windows(2)
            .any(|pair| matches!(pair, ["+", "+"] | ["-", "-"]))
        && !operand.windows(2).any(|pair| {
            pair[1] == "("
                && crate::typed_field_access::is_name(pair[0])
                && !crate::legacy_c::TYPE_OPERATORS.contains(&pair[0])
        })
}

/// A nested call's actuals: a place, or anything else.
fn inner_kinds(nested: &crate::legacy_c::NestedContractCall) -> Vec<Kind> {
    nested
        .arguments
        .iter()
        .map(|place| {
            if place.is_some() {
                Kind::Place
            } else {
                Kind::Undecided
            }
        })
        .collect()
}

/// What a contract call may write ([`decided`]): the objects its written
/// place, its place actuals and its nested calls' place actuals may denote,
/// or every object.
fn call_reach(
    state: &AbstractState,
    arguments: &[Option<String>],
    scalar_actuals: &[usize],
    nested: &[crate::legacy_c::NestedContractCall],
    destination: Option<&str>,
) -> Reach {
    let actuals = arguments
        .iter()
        .enumerate()
        .map(|(index, place)| {
            if scalar_actuals.contains(&index) {
                Kind::Scalar
            } else if place.is_some() {
                Kind::Place
            } else {
                Kind::Undecided
            }
        })
        .collect::<Vec<_>>();
    let inner = nested.iter().map(inner_kinds).collect::<Vec<_>>();
    let inner = inner.iter().map(Vec::as_slice).collect::<Vec<_>>();
    if !decided(&actuals, &inner) {
        return Reach::Everything;
    }
    let places = arguments
        .iter()
        .zip(&actuals)
        .filter(|(_, kind)| **kind == Kind::Place)
        .map(|(place, _)| place)
        .chain(nested.iter().flat_map(|nested| &nested.arguments))
        .flatten()
        .map(String::as_str);
    let written = destination.filter(|place| !plain(place));
    reach_places(state, places.chain(written))
}

/// What `instruction` may write, after its transfer. Statements that write
/// no memory reach nothing (a copy, a clear or an acquisition into a plain
/// name, a branch, a rule 36b initialisation); a copy or a clear into a
/// member place reaches the objects its written place may denote; a
/// contract call whose actuals are places or scalars and whose nested
/// calls' actuals are places ([`decided`]) reaches the objects its written
/// place, its places and its nested calls' places may denote (a contract
/// states every effect of the call, so it writes nothing it is not handed);
/// everything else may reach every object.
pub(in crate::legacy_c) fn reach(instruction: &LegacyInstruction, state: &AbstractState) -> Reach {
    match instruction {
        LegacyInstruction::NoOp
        | LegacyInstruction::BranchOnNull { .. }
        | LegacyInstruction::BranchOnValidity { .. }
        | LegacyInstruction::BranchOnResult { .. }
        | LegacyInstruction::TrackedSelfInit { .. } => Reach::Nothing,
        LegacyInstruction::Acquire { destination, .. }
        | LegacyInstruction::Alias { destination, .. }
        | LegacyInstruction::Clear { destination } => {
            if plain(destination) {
                Reach::Nothing
            } else {
                reach_places(state, [destination.as_str()])
            }
        }
        LegacyInstruction::ContractCall {
            arguments,
            nested,
            destination,
            scalar_actuals,
            ..
        } => call_reach(
            state,
            arguments,
            scalar_actuals,
            nested,
            destination.as_deref(),
        ),
        _ => Reach::Everything,
    }
}

/// What a call's nested calls may write before the call's own effect: the
/// objects their place actuals may denote, or every object when one of
/// their actuals is not a place. [`Reach::Nothing`] without a nested call.
pub(in crate::legacy_c) fn nested_reach(
    instruction: &LegacyInstruction,
    state: &AbstractState,
) -> Reach {
    let LegacyInstruction::ContractCall { nested, .. } = instruction else {
        return Reach::Nothing;
    };
    if nested.is_empty() {
        return Reach::Nothing;
    }
    if !nested
        .iter()
        .all(|nested| inner_kinds(nested).iter().all(|kind| *kind == Kind::Place))
    {
        return Reach::Everything;
    }
    reach_places(
        state,
        nested
            .iter()
            .flat_map(|nested| &nested.arguments)
            .flatten()
            .map(String::as_str),
    )
}

/// Before a call's own effect, forget every link record (rule 46's
/// initialisation facts and rule 91's unspent carried records) of an object
/// its nested calls may write ([`nested_reach`]).
pub(in crate::legacy_c) fn before_nested(
    instruction: &LegacyInstruction,
    state: &mut AbstractState,
) {
    match nested_reach(instruction, state) {
        Reach::Nothing => {}
        Reach::Sites(reached) => {
            state.link_ready.retain(|(site, _)| !reached.contains(site));
            state.carried_links.retain(|site, record| {
                matches!(record, super::carried_link::CarriedLink::Spent(_))
                    || !reached.contains(site)
            });
        }
        Reach::Everything => {
            state.link_ready.clear();
            super::carried_link::forget_ready(state);
        }
    }
}

/// Forget every initialisation fact a statement may invalidate, after its
/// transfer ([`reach`]).
pub(in crate::legacy_c) fn retain(instruction: &LegacyInstruction, state: &mut AbstractState) {
    if state.link_ready.is_empty() {
        return;
    }
    match reach(instruction, state) {
        Reach::Nothing => {}
        Reach::Sites(reached) => state.link_ready.retain(|(site, _)| !reached.contains(site)),
        Reach::Everything => state.link_ready.clear(),
    }
}

/// The vectors `QCore.ElementLinkPremises.vectors_hold` checks: the foreign
/// decision (declared member, binding members, admitted), and the
/// initialisation (path, writes, member).
#[cfg(test)]
pub(crate) type ForeignVector = (Option<&'static str>, &'static [&'static str], bool);
#[cfg(test)]
pub(crate) type InitVector = (
    &'static [&'static str],
    &'static [&'static str],
    Option<&'static str>,
);
#[cfg(test)]
pub(crate) type HeldVector = (Option<&'static str>, &'static [&'static str], bool);

#[cfg(test)]
pub(crate) const FOREIGN_VECTORS: &[ForeignVector] = &[
    (Some("_head"), &["_head"], true),
    (Some("_head"), &["_rotate_head"], false),
    (Some("_head"), &["_head", "_rotate_head"], false),
    (None, &["_head"], false),
    (Some("_head"), &[], false),
];

#[cfg(test)]
pub(crate) const INIT_VECTORS: &[InitVector] = &[
    (&["_head"], &["next", "prev"], Some("_head")),
    (&["_head"], &["next"], None),
    (&["_head"], &["prev"], None),
    (&[], &["next", "prev"], None),
    (&["inner", "_head"], &["next", "prev"], None),
];

#[cfg(test)]
pub(crate) const HELD_VECTORS: &[HeldVector] = &[
    (Some("_head"), &["_head"], true),
    (Some("_head"), &[], false),
    (Some("_head"), &["_rotate_head"], false),
    (None, &["_head"], false),
];

/// The vectors `QCore.ScalarCallKillPolicy.vectors_hold` checks: a call's
/// actuals, its nested calls' actuals, and whether its writes are decided
/// before and after the refinement.
#[cfg(test)]
pub(crate) type KillVector = (&'static [Kind], &'static [&'static [Kind]], bool, bool);

#[cfg(test)]
pub(crate) const KILL_VECTORS: &[KillVector] = {
    use Kind::{Place, Scalar, Undecided};
    &[
        // Places only, no nested call: decided on both sides.
        (&[Place, Place], &[], true, true),
        (&[], &[], true, true),
        // `flb_errno_print(*__errno_location(), __FILE__, __LINE__)`.
        (&[Scalar, Scalar, Scalar], &[&[]], false, true),
        // Literals beside a place.
        (&[Place, Scalar], &[], false, true),
        // A nested call reading a place.
        (&[Scalar], &[&[Place]], false, true),
        // An undecided actual, or a nested call's undecided actual.
        (&[Undecided, Scalar], &[], false, false),
        (&[Place, Undecided], &[], false, false),
        (&[Scalar], &[&[Undecided]], false, false),
        (&[Place], &[&[Place], &[Undecided]], false, false),
    ]
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_whole_call_free_sizeof_is_scalar() {
        let admitted: &[&[&str]] = &[
            &["sizeof", "(", "struct", "flb_task_enqueued", ")"],
            &["sizeof", "(", "*", "v", ")"],
            &["sizeof", "(", "int", ")"],
            &["sizeof", "(", "(", "char", "*", ")", "p", ")"],
        ];
        for actual in admitted {
            assert!(is_sizeof_actual(actual), "{actual:?}");
        }
        let refused: &[&[&str]] = &[
            &["sizeof", "x"],
            &["sizeof", "(", "T", ")", "+", "p"],
            &["(", "sizeof", "(", "T", ")", ")"],
            &["sizeof", "(", "T", ")", "*", "(", "n", ")"],
            &["sizeof", "(", "f", "(", ")", ")"],
            &["sizeof", "(", "x", "=", "y", ")"],
            &["sizeof", "(", "x", "+", "+", ")"],
            &["sizeof", "(", "-", "-", "x", ")"],
            &["sizeof", "(", ")"],
            &["n"],
        ];
        for actual in refused {
            assert!(!is_sizeof_actual(actual), "{actual:?}");
        }
    }

    #[test]
    fn the_kill_vectors_hold() {
        for (actuals, nested, before, after) in KILL_VECTORS {
            assert_eq!(
                decided_before(actuals, nested),
                *before,
                "{actuals:?} {nested:?}"
            );
            assert_eq!(decided(actuals, nested), *after, "{actuals:?} {nested:?}");
        }
    }

    #[test]
    fn the_refinement_removes_only_scalar_or_nested_kills() {
        // Every vector decided before stays decided; one decided only after
        // carries a scalar or a nested call, and nothing undecided.
        for (actuals, nested, before, after) in KILL_VECTORS {
            if *before {
                assert!(*after);
            }
            if *after && !*before {
                assert!(
                    !nested.is_empty() || actuals.contains(&Kind::Scalar),
                    "{actuals:?} {nested:?}"
                );
                assert!(!actuals.contains(&Kind::Undecided));
            }
        }
    }

    fn set(members: &[&str]) -> BTreeSet<String> {
        members.iter().map(|member| (*member).to_owned()).collect()
    }

    #[test]
    fn the_lean_vectors_hold() {
        for (link, members, admitted) in FOREIGN_VECTORS {
            assert_eq!(
                foreign_admits(*link, &set(members)),
                *admitted,
                "{link:?} {members:?}"
            );
        }
        for (path, writes, member) in INIT_VECTORS {
            let path = path.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
            let writes = writes.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
            assert_eq!(
                initialised_member(&path, &writes),
                *member,
                "{path:?} {writes:?}"
            );
        }
        for (link, ready, admitted) in HELD_VECTORS {
            assert_eq!(
                held_decides(*link, &set(ready)),
                *admitted,
                "{link:?} {ready:?}"
            );
        }
    }

    #[test]
    fn a_member_place_is_not_plain() {
        assert!(plain("e"));
        assert!(!plain("e->_head.next"));
        assert!(!plain("*slot"));
    }

    #[test]
    fn an_actual_names_its_root() {
        assert_eq!(root("&x->_head"), Some("x"));
        assert_eq!(root("x"), Some("x"));
        assert_eq!(root("(x)->f"), Some("x"));
        assert_eq!(root("0"), None);
    }
}
