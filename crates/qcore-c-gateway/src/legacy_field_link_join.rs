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

//! Card **B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED** (gateway rule 11c): the
//! route-G adapter of [`crate::field_link_join`], called from thin hooks in
//! `legacy_c.rs` and `legacy_loop_generations.rs`.
//!
//! * [`join`] is the B08 field-link join. It keeps the union of the links
//!   and records in `partial_links` every `(owner, field, child)` that does
//!   not hold on both sides. It adds **no** unresolved effect: the child's
//!   obligation stays exactly what the two paths' statuses say, so an
//!   owner's release (which does not cascade, P-B08-SHALLOW) leaves a
//!   possibly-live child to be reported at the exit as in the straight-line
//!   form. [`stored`], [`prune`] and [`rename`] keep `partial_links` a subset
//!   of `field_links` through a store, every other transfer and a
//!   generation shift.
//! * [`escape_cascade`] is the escape of an owner through its field links:
//!   [`crate::field_link_join::escape_step`] for each child it reaches. A
//!   refused child keeps its status with an unresolved effect, and
//!   [`report_escape_refusal`] reports the refusal at the escape.
//! * [`partial_field`] tells a read through a member that its link may not
//!   hold on every path, so the read refuses.
//! * [`exit_backstop`] is the floor under every rule: a settled exit on a
//!   path no refusal reached that still withholds a possibly-live identity
//!   with an unresolved effect reports a refusal
//!   ([`crate::field_link_join::exit_refuses`]).
//! * [`record_escape_exposure`] is the card's census (parity only, it decides
//!   nothing): each child an escape reaches that may be live and is not
//!   carried on every path, or has a mixed status (`position:
//!   "field-link-escape"`).

use std::collections::{BTreeMap, BTreeSet};

use serde_json::json;

use super::{
    AbstractPointer, AbstractState, CONTAINED, ESCAPED, LIVE, LegacyDiagnostic,
    LegacyDiagnosticKind, mark_unresolved_effect,
};
use crate::field_link_join::{EscapeStep, Link, escape_step, exit_refuses, join_link};
use crate::typed_access_path::{parity_enabled, record_row};

/// The refusal an escape reports for a linked child that may be live on a
/// path the escape does not carry it, or whose status is mixed.
pub(crate) const ESCAPE_REFUSED: &str =
    "field-link escape: a linked child may still be live on a path the escape does not carry it";
/// The refusal a settled exit reports when it withholds a possibly-live
/// identity with an unresolved effect on a path no refusal reached.
pub(crate) const EXIT_WITHHELD: &str =
    "exit withholds a possibly-live obligation whose effect is unresolved and unreported";

type Links = BTreeMap<usize, BTreeMap<String, BTreeSet<usize>>>;

/// The B08 field-link join of `incoming` into `state` (see the module
/// comment). Both relations only grow over a fixed point's iterates, so the
/// work list still terminates.
pub(super) fn join(state: &mut AbstractState, incoming: &AbstractState) {
    let owners = state
        .field_links
        .keys()
        .chain(incoming.field_links.keys())
        .copied()
        .collect::<BTreeSet<_>>();
    let mut partial = Links::new();
    for owner in owners {
        let mine = state.field_links.get(&owner).cloned().unwrap_or_default();
        let theirs = incoming
            .field_links
            .get(&owner)
            .cloned()
            .unwrap_or_default();
        let fields = mine
            .keys()
            .chain(theirs.keys())
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut joined: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
        for field in fields {
            let left = mine.get(&field).cloned().unwrap_or_default();
            let right = theirs.get(&field).cloned().unwrap_or_default();
            let union = left.union(&right).copied().collect::<BTreeSet<_>>();
            if union.is_empty() {
                continue;
            }
            // Each side's link of each child (`Link`), joined by
            // `join_link`: partial when one side does not link it, or when a
            // side already did not on some path of its own.
            let side_link = |links: &BTreeSet<usize>, partial: &Links, child: usize| {
                if !links.contains(&child) {
                    Link::Absent
                } else if partial
                    .get(&owner)
                    .and_then(|fields| fields.get(&field))
                    .is_some_and(|children| children.contains(&child))
                {
                    Link::Partial
                } else {
                    Link::Must
                }
            };
            let one_sided = union
                .iter()
                .copied()
                .filter(|&child| {
                    join_link(
                        side_link(&left, &state.partial_links, child),
                        side_link(&right, &incoming.partial_links, child),
                    ) == Link::Partial
                })
                .collect::<BTreeSet<_>>();
            if !one_sided.is_empty() {
                partial
                    .entry(owner)
                    .or_default()
                    .insert(field.clone(), one_sided);
            }
            joined.insert(field, union);
        }
        if joined.is_empty() {
            state.field_links.remove(&owner);
        } else {
            state.field_links.insert(owner, joined);
        }
    }
    state.partial_links = partial;
}

/// A store into `owner`'s `field` wrote the link on this path: it holds on
/// every path from here.
pub(super) fn stored(state: &mut AbstractState, owner: usize, field: &str) {
    if let Some(fields) = state.partial_links.get_mut(&owner) {
        fields.remove(field);
        if fields.is_empty() {
            state.partial_links.remove(&owner);
        }
    }
}

/// Drop every partial triple whose link a transfer removed.
pub(super) fn prune(state: &mut AbstractState) {
    if state.partial_links.is_empty() {
        return;
    }
    let links = &state.field_links;
    state.partial_links.retain(|owner, fields| {
        fields.retain(|field, children| {
            match links.get(owner).and_then(|linked| linked.get(field)) {
                Some(linked) => children.retain(|child| linked.contains(child)),
                None => children.clear(),
            }
            !children.is_empty()
        });
        !fields.is_empty()
    });
}

/// Card CG04-LOOP-SITE-GENERATIONS' shift renames `from` to `into`: the
/// partial triples follow, as the field links do.
pub(super) fn rename(state: &mut AbstractState, from: usize, into: usize) {
    if let Some(fields) = state.partial_links.remove(&from) {
        let merged = state.partial_links.entry(into).or_default();
        for (field, children) in fields {
            merged.entry(field).or_default().extend(children);
        }
    }
    for fields in state.partial_links.values_mut() {
        for children in fields.values_mut() {
            if children.remove(&from) {
                children.insert(into);
            }
        }
    }
}

/// Whether a read through `owner`'s `field` may not denote the linked child
/// on every path.
pub(super) fn partial_field(state: &AbstractState, owner: usize, field: &str) -> bool {
    state
        .partial_links
        .get(&owner)
        .and_then(|fields| fields.get(field))
        .is_some_and(|children| !children.is_empty())
}

/// The children an escape of `owner` reaches through field links (and
/// through the containment and field links of what it reaches), with
/// whether some link that reaches each may not hold on every path.
fn reached(state: &AbstractState, owner: usize) -> BTreeMap<usize, bool> {
    let mut reached = BTreeMap::<usize, bool>::new();
    let mut pending = vec![(owner, false)];
    let mut expanded = BTreeSet::new();
    while let Some((site, partial)) = pending.pop() {
        if !expanded.insert((site, partial)) {
            continue;
        }
        if site != owner || partial {
            let entry = reached.entry(site).or_insert(false);
            *entry |= partial;
        }
        for (field, children) in state.field_links.get(&site).into_iter().flatten() {
            for &child in children {
                let one_sided = partial
                    || state
                        .partial_links
                        .get(&site)
                        .and_then(|fields| fields.get(field))
                        .is_some_and(|partial| partial.contains(&child));
                pending.push((child, one_sided));
            }
        }
        if site != owner {
            for &child in state.contained.get(&site).into_iter().flatten() {
                pending.push((child, partial));
            }
        }
    }
    reached
}

/// Escape `owner`'s linked children (the B08 escape cascade): each child it
/// reaches is decided by [`escape_step`]. Returns the refused children;
/// the caller reports them ([`report_escape_refusal`]).
pub(super) fn escape_cascade(state: &mut AbstractState, owner: usize) -> BTreeSet<usize> {
    let reached = reached(state, owner);
    // The links are consumed by the escape, as before: the owner's and every
    // reached child's field links, and every reached child's containment.
    state.field_links.remove(&owner);
    for site in reached.keys() {
        state.field_links.remove(site);
        state.contained.remove(site);
    }
    prune(state);
    let mut withdrawn = BTreeSet::new();
    let mut refused = BTreeSet::new();
    for (&site, &partial) in &reached {
        match escape_step(partial, state.resources.get(&site).copied()) {
            EscapeStep::Keep => {}
            EscapeStep::Escape => {
                state.resources.insert(site, ESCAPED);
            }
            EscapeStep::Withdraw => {
                withdrawn.insert(site);
            }
            EscapeStep::Refuse => {
                refused.insert(site);
            }
        }
    }
    state.unresolved_effect_sites.extend(withdrawn);
    if !refused.is_empty() {
        mark_unresolved_effect(
            state,
            &AbstractPointer {
                targets: refused.clone(),
                ..AbstractPointer::default()
            },
        );
    }
    refused
}

/// Report an escape's refused children (see [`escape_cascade`]).
pub(super) fn report_escape_refusal(
    function: &str,
    line: usize,
    refused: &BTreeSet<usize>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    if refused.is_empty() {
        return;
    }
    diagnostics.insert(LegacyDiagnostic {
        function: function.to_owned(),
        line,
        kind: LegacyDiagnosticKind::Unsupported {
            description: ESCAPE_REFUSED.to_owned(),
        },
    });
}

/// The floor under every rule, at a settled exit (see the module comment).
pub(super) fn exit_backstop(
    function: &str,
    line: usize,
    state: &AbstractState,
    exempt: &BTreeSet<usize>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    let withheld = super::loop_generations_route::withheld(state, exempt);
    if exit_refuses(!withheld.is_empty(), !state.unmodeled) {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::Unsupported {
                description: EXIT_WITHHELD.to_owned(),
            },
        });
    }
}

/// Record the escape of `owner` at `line` when a child it reaches may be
/// live and is not carried on every path, or has a mixed status (census,
/// parity only; runs on every iterate, so it over-approximates the settled
/// states).
pub(super) fn record_escape_exposure(
    function: &str,
    line: usize,
    state: &AbstractState,
    owner: usize,
) {
    if !parity_enabled() {
        return;
    }
    for (child, partial) in reached(state, owner) {
        let status = state.resources.get(&child).copied().unwrap_or(0);
        let unresolved = state.unresolved_effect_sites.contains(&child);
        let possibly_live = status & LIVE != 0;
        let mixed = status != LIVE && (possibly_live || status & CONTAINED != 0);
        if !((possibly_live && (partial || unresolved)) || mixed) {
            continue;
        }
        record_row(json!({
            "function": function,
            "line": line,
            "texts": [],
            "position": "field-link-escape",
            "child": format!("{child:#x}"),
            "status": status,
            "partial": partial,
            "unresolved": unresolved,
            "mixed": mixed,
            "step": escape_step(partial, state.resources.get(&child).copied()).code(),
            "old_route": "flow",
            "old_admitted": true,
            "new_admitted": true,
            "disagreement": false,
        }));
    }
}

#[cfg(test)]
#[path = "../tests/unit/legacy_field_link_join.rs"]
mod tests;
