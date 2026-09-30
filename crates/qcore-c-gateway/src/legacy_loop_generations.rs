//! Card **CG04-LOOP-SITE-GENERATIONS**: the route-G adapter of
//! [`crate::loop_generations`], called from thin hooks in `legacy_c.rs`.
//!
//! * [`record_exposure`] reads the settled states once the work list is
//!   stable and records, in the `QCORE_TYPED_PATH_PARITY` stream
//!   (`position: "loop-site-generation"`), every acquisition node whose own
//!   site is already present in the state it runs in: an earlier generation
//!   of the site reaches its re-acquisition, so the node lies on a CFG cycle.
//!   It decides nothing.

use serde_json::json;

use super::{
    AbstractState, Derivation, LIVE, LegacyCfg, instruction_produced_sites,
};
use crate::typed_access_path::{parity_enabled, record_row};

/// Whether anything other than the binding `destination` refers to `site`
/// in `state`: another identity binding, an aggregate edge, a pending
/// reallocation, a publication or a collection element.
pub(super) fn referenced_elsewhere(
    state: &AbstractState,
    site: usize,
    destination: Option<&str>,
) -> bool {
    state.pointers.iter().any(|(name, pointer)| {
        Some(name.as_str()) != destination
            && pointer.targets.contains(&site)
            && !matches!(pointer.derivation, Derivation::Derived { .. })
    }) || state
        .contained
        .values()
        .any(|children| children.contains(&site))
        || state
            .field_links
            .values()
            .any(|fields| fields.values().any(|children| children.contains(&site)))
        || state
            .conditional_replacements
            .values()
            .any(|replaced| replaced.contains(&site))
        || state.published.contains(&site)
}

/// The destination binding an acquisition node writes, when it has one name.
fn node_destination(instruction: &super::LegacyInstruction) -> Option<&str> {
    match instruction {
        super::LegacyInstruction::Acquire { destination, .. } => Some(destination.as_str()),
        super::LegacyInstruction::ContractCall { destination, .. } => destination.as_deref(),
        _ => None,
    }
}

/// Record every re-acquisition an earlier generation reaches (see the module
/// comment). Parity only: no diagnostic, no state change.
pub(super) fn record_exposure(cfg: &LegacyCfg, incoming: &[Option<AbstractState>]) {
    if !parity_enabled() {
        return;
    }
    for (node_id, node) in cfg.nodes.iter().enumerate() {
        let Some(state) = incoming[node_id].as_ref() else {
            continue;
        };
        for site in instruction_produced_sites(&node.instruction) {
            let Some(&status) = state.resources.get(&site) else {
                continue;
            };
            let destination = node_destination(&node.instruction);
            record_row(json!({
                "function": cfg.function,
                "line": node.line,
                "texts": [],
                "position": "loop-site-generation",
                "phase": "exposure",
                "site": site,
                "status": status,
                "live": status & LIVE != 0,
                "unresolved": state.unresolved_effect_sites.contains(&site),
                "nullable": state.nullable_sites.contains(&site),
                "referenced": referenced_elsewhere(state, site, destination),
                "old_route": "flow",
                "old_admitted": true,
                "new_admitted": true,
                "disagreement": false,
            }));
        }
    }
}
