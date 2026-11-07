//! Card **CG04-LOOP-SITE-GENERATIONS**: the route-G adapter of
//! [`crate::loop_generations`], called from thin hooks in `legacy_c.rs`.
//!
//! * [`shift`] runs before a node's transfer (and before the per-node reads
//!   the driver makes of the state it runs in). For every site the node
//!   produces that the state already mentions, an earlier generation reaches
//!   its re-acquisition: the previous generation is merged into the older
//!   summary, the current one is renamed previous, and the node's own
//!   transfer then creates a fresh current generation. A merge that
//!   [`crate::loop_generations::decide`] refuses reports `Unsupported` and
//!   leaves the summary with an unresolved effect.
//! * [`record_exposure`] reads the settled states once the work list is
//!   stable and records every re-acquisition, with its step, in the
//!   `QCORE_TYPED_PATH_PARITY` stream (`position: "loop-site-generation"`).
//!   It decides nothing.
//! * [`withheld`] and [`record_withheld`] are a census for the card's sibling
//!   search: a `Safe` result whose settled exit still holds a possibly-live
//!   identity with an unresolved effect, which no diagnostic reported
//!   (`position: "withheld-obligation"`). They decide nothing either.

use std::collections::BTreeSet;

use serde_json::json;

use super::{
    AbstractState, LIVE, LegacyCfg, LegacyCfgNode, LegacyDiagnostic, LegacyDiagnosticKind,
    LegacyInstruction, ResourceParameterMode, ResourceReturnMode, instruction_produced_sites,
    is_indirect_place, mark_unresolved_effect,
};
use crate::loop_generations::{Earlier, Step, decide, indexable, older, previous};
use crate::typed_access_path::{parity_enabled, record_row};

/// The refusal a merge into a held, possibly live summary reports.
pub(crate) const SHIFT_REFUSED: &str =
    "loop re-acquisition: an earlier generation that may be live is still held";
/// The refusal for a site the generation tags cannot index.
pub(crate) const SITE_NOT_INDEXABLE: &str =
    "loop re-acquisition: the acquisition site cannot be generation-indexed";

/// The names a node binds to the sites it produces: they stop holding
/// whatever they held before, so they do not hold an earlier generation past
/// the node.
fn overwritten_names(instruction: &LegacyInstruction) -> BTreeSet<&str> {
    match instruction {
        LegacyInstruction::Acquire { destination, .. } => BTreeSet::from([destination.as_str()]),
        LegacyInstruction::ContractCall {
            destination,
            arguments,
            contract,
            ..
        } => {
            let mut names = BTreeSet::new();
            if matches!(
                contract.result,
                ResourceReturnMode::Owned { .. } | ResourceReturnMode::Reallocated { .. }
            ) && let Some(destination) = destination
            {
                names.insert(destination.as_str());
            }
            for (index, mode) in contract.parameters.iter().enumerate() {
                if matches!(mode, ResourceParameterMode::Out { .. })
                    && let Some(Some(name)) = arguments.get(index)
                {
                    names.insert(name.as_str());
                }
            }
            names
        }
        _ => BTreeSet::new(),
    }
}

/// Whether anything in `state` mentions `identity`.
fn mentioned(state: &AbstractState, identity: usize) -> bool {
    state.resources.contains_key(&identity)
        || state.nullable_sites.contains(&identity)
        || state.unresolved_effect_sites.contains(&identity)
        || state.published.contains(&identity)
        || state.collections.contains_key(&identity)
        || state.conditional_replacements.contains_key(&identity)
        || state.contained.contains_key(&identity)
        || state.field_links.contains_key(&identity)
        || held_by_edge(state, identity)
        || state
            .pointers
            .values()
            .any(|pointer| pointer.targets.contains(&identity))
}

/// Whether an aggregate edge, a pending reallocation or a publication holds
/// `identity`: a place that can update it without going through a binding.
fn held_by_edge(state: &AbstractState, identity: usize) -> bool {
    state
        .contained
        .values()
        .any(|children| children.contains(&identity))
        || state
            .field_links
            .values()
            .any(|fields| fields.values().any(|children| children.contains(&identity)))
        || state
            .conditional_replacements
            .values()
            .any(|replaced| replaced.contains(&identity))
        || state.published.contains(&identity)
}

/// Whether a place other than a local binding holds `identity`: an edge
/// ([`held_by_edge`]) or an out slot this node does not overwrite.
fn held(state: &AbstractState, identity: usize, overwritten: &BTreeSet<&str>) -> bool {
    held_by_edge(state, identity)
        || state.pointers.iter().any(|(name, pointer)| {
            is_indirect_place(name)
                && !overwritten.contains(name.as_str())
                && pointer.targets.contains(&identity)
        })
}

/// Whether `identity` may be live.
fn live(state: &AbstractState, identity: usize) -> bool {
    state
        .resources
        .get(&identity)
        .is_some_and(|status| status & LIVE != 0)
}

/// What the node at hand knows about `site`'s earlier generations.
fn earlier(state: &AbstractState, site: usize, overwritten: &BTreeSet<&str>) -> Earlier {
    let (previous, older) = (previous(site), older(site));
    Earlier {
        present: mentioned(state, site),
        previous_live: live(state, previous),
        previous_held: held(state, previous, overwritten),
        previous_unresolved: state.unresolved_effect_sites.contains(&previous),
        older_live: live(state, older),
        older_held: held(state, older, overwritten),
        older_unresolved: state.unresolved_effect_sites.contains(&older),
    }
}

fn replace_in(set: &mut BTreeSet<usize>, from: usize, into: usize) -> bool {
    let found = set.remove(&from);
    if found {
        set.insert(into);
    }
    found
}

/// Move every mention of `from` to `into`. When `into` is a summary
/// ([`crate::loop_generations::is_summary`]) it may already stand for other
/// objects: statuses and sets are merged, a binding
/// that named `from` becomes unknown (no precise update may discharge a
/// summary), and a collection meeting another one, or a present summary,
/// becomes `Unknown`. Otherwise `into` must be unmentioned and the move is
/// a pure renaming.
fn rename(state: &mut AbstractState, from: usize, into: usize) {
    let summary = crate::loop_generations::is_summary(into);
    if !mentioned(state, from) {
        return;
    }
    debug_assert!(
        summary || !mentioned(state, into),
        "a renaming target is fresh"
    );
    let into_present = state.resources.contains_key(&into);
    let from_present = state.resources.contains_key(&from);
    if let Some(status) = state.resources.remove(&from) {
        *state.resources.entry(into).or_insert(0) |= status;
    }
    if state.nullable_sites.remove(&from) {
        state.nullable_sites.insert(into);
    }
    if state.unresolved_effect_sites.remove(&from) {
        state.unresolved_effect_sites.insert(into);
    }
    replace_in(&mut state.published, from, into);
    match state.collections.remove(&from) {
        // A summary's element ranges would describe several arrays at once:
        // two generations' ranges (or one generation's ranges and another
        // generation with none stored, all unfilled) are not joined.
        Some(_) if summary && (into_present || state.collections.contains_key(&into)) => {
            state
                .collections
                .insert(into, crate::indexed_collection::Collection::Unknown);
        }
        Some(collection) => {
            state.collections.insert(into, collection);
        }
        None if summary && from_present && state.collections.contains_key(&into) => {
            state
                .collections
                .insert(into, crate::indexed_collection::Collection::Unknown);
        }
        None => {}
    }
    if let Some(replaced) = state.conditional_replacements.remove(&from) {
        state
            .conditional_replacements
            .entry(into)
            .or_default()
            .extend(replaced);
    }
    for replaced in state.conditional_replacements.values_mut() {
        replace_in(replaced, from, into);
    }
    if let Some(children) = state.contained.remove(&from) {
        state.contained.entry(into).or_default().extend(children);
    }
    for children in state.contained.values_mut() {
        replace_in(children, from, into);
    }
    if let Some(fields) = state.field_links.remove(&from) {
        let merged = state.field_links.entry(into).or_default();
        for (field, children) in fields {
            merged.entry(field).or_default().extend(children);
        }
    }
    for fields in state.field_links.values_mut() {
        for children in fields.values_mut() {
            replace_in(children, from, into);
        }
    }
    // Card B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED: the partial triples follow.
    super::field_link_join_route::rename(state, from, into);
    // Card REFCOUNT-RESOURCE-KIND (rule 13b): a reference family's count
    // follows its site; two families merged into one site have no count.
    if let Some(held) = state.references.remove(&from) {
        state
            .references
            .entry(into)
            .and_modify(|current| current.held = crate::reference_count::Held::Undecided)
            .or_insert(held);
    }
    for pointer in state.pointers.values_mut() {
        if replace_in(&mut pointer.targets, from, into) && summary {
            pointer.unknown = true;
        }
    }
}

/// Shift the generations of every site `node` produces that `state` already
/// mentions (see the module comment), and return the sites shifted. Every
/// other state is returned unchanged.
pub(super) fn shift(
    function: &str,
    node: &LegacyCfgNode,
    mut state: AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) -> (AbstractState, Vec<usize>) {
    let sites = instruction_produced_sites(&node.instruction);
    if sites.iter().all(|site| !mentioned(&state, *site)) {
        return (state, Vec::new());
    }
    let overwritten = overwritten_names(&node.instruction);
    let mut shifted = Vec::new();
    for site in sites {
        if !mentioned(&state, site) {
            continue;
        }
        let refuse = |state: &mut AbstractState,
                      diagnostics: &mut BTreeSet<LegacyDiagnostic>,
                      description: &str,
                      identity: usize| {
            diagnostics.insert(LegacyDiagnostic {
                function: function.to_owned(),
                line: node.line,
                kind: LegacyDiagnosticKind::Unsupported {
                    description: description.to_owned(),
                },
            });
            mark_unresolved_effect(
                state,
                &super::AbstractPointer {
                    targets: BTreeSet::from([identity]),
                    ..super::AbstractPointer::default()
                },
            );
            state.unmodeled = true;
        };
        if !indexable(site) {
            refuse(&mut state, diagnostics, SITE_NOT_INDEXABLE, site);
            continue;
        }
        let step = decide(earlier(&state, site, &overwritten));
        rename(&mut state, previous(site), older(site));
        rename(&mut state, site, previous(site));
        if step == Step::ShiftRefused {
            refuse(&mut state, diagnostics, SHIFT_REFUSED, older(site));
        }
        shifted.push(site);
    }
    (state, shifted)
}

/// After the transfer of a node that shifted `shifted` and was refused (a
/// diagnostic in `diagnostics` poisons the path): a shifted site the refused
/// transfer did not create may still have been produced by the call C ran,
/// so it is present, possibly live, with an unresolved effect. Before this
/// card the site stood there for the earlier generation, with the refusal's
/// unresolved effect; a fresh generation never produced must not leave a
/// join with another iteration's owed generation claiming more than that.
pub(super) fn settle_refused(
    shifted: &[usize],
    state: &mut AbstractState,
    diagnostics: &BTreeSet<LegacyDiagnostic>,
) {
    if shifted.is_empty() || !diagnostics.iter().any(super::is_path_poisoning) {
        return;
    }
    for &site in shifted {
        if let std::collections::btree_map::Entry::Vacant(entry) = state.resources.entry(site) {
            entry.insert(LIVE);
            state.unresolved_effect_sites.insert(site);
        }
    }
}

/// The acquisition sites whose re-acquisition the settled analysis refuses
/// ([`Step::ShiftRefused`]), read from the settled incoming states. The
/// decision is monotone in the state, so the settled state decides as the
/// last iterate of each node did, and that iterate reported
/// [`SHIFT_REFUSED`] there. Reads only.
pub(super) fn refused_reacquisitions(
    cfg: &LegacyCfg,
    incoming: &[Option<AbstractState>],
) -> BTreeSet<usize> {
    let mut refused = BTreeSet::new();
    for (node_id, node) in cfg.nodes.iter().enumerate() {
        let Some(state) = incoming[node_id].as_ref() else {
            continue;
        };
        let overwritten = overwritten_names(&node.instruction);
        for site in instruction_produced_sites(&node.instruction) {
            if indexable(site)
                && mentioned(state, site)
                && decide(earlier(state, site, &overwritten)) == Step::ShiftRefused
            {
                refused.insert(site);
            }
        }
    }
    refused
}

/// Record every re-acquisition an earlier generation reaches, with its step
/// (see the module comment). Parity only: no diagnostic, no state change.
pub(super) fn record_exposure(cfg: &LegacyCfg, incoming: &[Option<AbstractState>]) {
    if !parity_enabled() {
        return;
    }
    for (node_id, node) in cfg.nodes.iter().enumerate() {
        let Some(state) = incoming[node_id].as_ref() else {
            continue;
        };
        let overwritten = overwritten_names(&node.instruction);
        for site in instruction_produced_sites(&node.instruction) {
            if !mentioned(state, site) {
                continue;
            }
            let earlier = earlier(state, site, &overwritten);
            let status = state.resources.get(&site).copied().unwrap_or(0);
            record_row(json!({
                "function": cfg.function,
                "line": node.line,
                "texts": [],
                "position": "loop-site-generation",
                "site": site,
                "status": status,
                "live": status & LIVE != 0,
                "unresolved": state.unresolved_effect_sites.contains(&site),
                "held": held(state, site, &overwritten),
                "referenced": state.pointers.iter().any(|(name, pointer)| {
                    !overwritten.contains(name.as_str()) && pointer.targets.contains(&site)
                }),
                "nullable": state.nullable_sites.contains(&site),
                "previous_live": earlier.previous_live,
                "previous_held": earlier.previous_held,
                "previous_unresolved": earlier.previous_unresolved,
                "older_live": earlier.older_live,
                "older_held": earlier.older_held,
                "older_unresolved": earlier.older_unresolved,
                "step": if indexable(site) { decide(earlier).code() } else { "not-indexable" },
                "old_route": "flow",
                "old_admitted": true,
                "new_admitted": true,
                "disagreement": false,
            }));
        }
    }
}

/// The identities an exit state holds possibly live, with an unresolved
/// effect, that the caller does not receive (census only).
pub(super) fn withheld(state: &AbstractState, exempt: &BTreeSet<usize>) -> BTreeSet<usize> {
    state
        .resources
        .iter()
        .filter(|(site, status)| {
            **status & LIVE != 0
                && state.unresolved_effect_sites.contains(site)
                && !exempt.contains(site)
        })
        .map(|(site, _)| *site)
        .collect()
}

/// Record every exit of a `Safe` result that withholds an obligation (see
/// [`withheld`]). Parity only.
pub(super) fn record_withheld(
    function: &str,
    status: super::AnalysisStatus,
    withheld: &[(usize, BTreeSet<usize>)],
    acquisition_lines: &std::collections::BTreeMap<usize, usize>,
) {
    if !parity_enabled() || status != super::AnalysisStatus::Safe {
        return;
    }
    for (line, sites) in withheld {
        if sites.is_empty() {
            continue;
        }
        record_row(json!({
            "function": function,
            "line": line,
            "texts": [],
            "position": "withheld-obligation",
            "sites": sites.iter().map(|site| format!("{site:#x}")).collect::<Vec<_>>(),
            "acquisition_lines": sites
                .iter()
                .map(|site| super::acquisition_line(acquisition_lines, *site))
                .collect::<Vec<_>>(),
            "old_route": "flow",
            "old_admitted": true,
            "new_admitted": true,
            "disagreement": false,
        }));
    }
}

#[cfg(test)]
#[path = "../tests/unit/legacy_loop_generations.rs"]
mod tests;
