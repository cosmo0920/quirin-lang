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
//!   call keep it only when (the call having no nested call) every written
//!   place's and actual's root names a binding that cannot denote the
//!   object. A formal, a callee-constructed object and
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

/// Forget the records of every object one of `places` may reach: the sites
/// its root's binding denotes, or every record when a root has no binding or
/// an unknown one. A place with no identifier root (a literal) reaches none.
fn forget_reached<'a>(state: &mut AbstractState, places: impl IntoIterator<Item = &'a str>) {
    let mut reached = BTreeSet::new();
    for place in places {
        let Some(name) = root(place) else {
            continue;
        };
        match state.pointers.get(name) {
            Some(pointer) if !pointer.unknown => reached.extend(pointer.targets.iter().copied()),
            _ => {
                state.link_ready.clear();
                return;
            }
        }
    }
    state.link_ready.retain(|(site, _)| !reached.contains(site));
}

/// Forget every initialisation fact a statement may invalidate, after its
/// transfer. Statements that write no memory keep every fact (a copy, a
/// clear or an acquisition into a plain name, a branch, a rule 36b
/// initialisation); a copy or a clear into a member place, and a contract
/// call with no nested call, forget the records of the objects their written
/// place or their actuals may denote; everything else forgets them all.
pub(in crate::legacy_c) fn retain(instruction: &LegacyInstruction, state: &mut AbstractState) {
    if state.link_ready.is_empty() {
        return;
    }
    match instruction {
        LegacyInstruction::NoOp
        | LegacyInstruction::BranchOnNull { .. }
        | LegacyInstruction::BranchOnValidity { .. }
        | LegacyInstruction::BranchOnResult { .. }
        | LegacyInstruction::TrackedSelfInit { .. } => {}
        LegacyInstruction::Acquire { destination, .. }
        | LegacyInstruction::Alias { destination, .. }
        | LegacyInstruction::Clear { destination } => {
            if !plain(destination) {
                forget_reached(state, [destination.as_str()]);
            }
        }
        LegacyInstruction::ContractCall {
            arguments,
            nested,
            destination,
            ..
        } if nested.is_empty() && arguments.iter().all(Option::is_some) => {
            let written = destination.as_deref().filter(|place| !plain(place));
            forget_reached(
                state,
                arguments
                    .iter()
                    .flatten()
                    .map(String::as_str)
                    .chain(written),
            );
        }
        _ => state.link_ready.clear(),
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

#[cfg(test)]
mod tests {
    use super::*;

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
