//! Card **B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED** (gateway rule 11c): the
//! route-G adapter, called from thin hooks in `legacy_c.rs`.
//!
//! * [`record_escape_exposure`] is a census (parity only, it decides
//!   nothing): each child an escape of its owner reaches through a B08 field
//!   link that may be live while the escape does not account for it on every
//!   path (`position: "field-link-escape"`).

use std::collections::BTreeSet;

use serde_json::json;

use super::{AbstractState, CONTAINED, LIVE};
use crate::typed_access_path::{parity_enabled, record_row};

/// The children an escape of `owner` reaches through field links (and
/// through the containment and field links of what it reaches), exactly the
/// traversal of `cascade_field_links_on_escape`.
fn reached(state: &AbstractState, owner: usize) -> BTreeSet<usize> {
    let mut pending = state
        .field_links
        .get(&owner)
        .into_iter()
        .flat_map(|fields| fields.values().flatten().copied())
        .collect::<BTreeSet<_>>();
    let mut visited = BTreeSet::new();
    while let Some(site) = pending.pop_first() {
        if !visited.insert(site) {
            continue;
        }
        if let Some(fields) = state.field_links.get(&site) {
            pending.extend(fields.values().flatten().copied());
        }
        if let Some(children) = state.contained.get(&site) {
            pending.extend(children.iter().copied());
        }
    }
    visited
}

/// Record the escape of `owner` at `line` when a child it reaches through a
/// field link may be live (or contained) and either carries an unresolved
/// effect already (on the base, every child linked on one side of a join) or
/// has a mixed status. Parity only; runs on every iterate, so it
/// over-approximates the settled states.
pub(super) fn record_escape_exposure(function: &str, line: usize, state: &AbstractState, owner: usize) {
    if !parity_enabled() {
        return;
    }
    for child in reached(state, owner) {
        let status = state.resources.get(&child).copied().unwrap_or(0);
        let unresolved = state.unresolved_effect_sites.contains(&child);
        let possibly_live = status & LIVE != 0;
        let contained = status & CONTAINED != 0;
        let mixed = status != LIVE && (possibly_live || contained);
        if !((possibly_live && unresolved) || mixed) {
            continue;
        }
        record_row(json!({
            "function": function,
            "line": line,
            "texts": [],
            "position": "field-link-escape",
            "child": format!("{child:#x}"),
            "status": status,
            "unresolved": unresolved,
            "mixed": mixed,
            "old_route": "flow",
            "old_admitted": true,
            "new_admitted": true,
            "disagreement": false,
        }));
    }
}
