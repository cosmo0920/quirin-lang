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

//! Card CG04-LOOP-SITE-GENERATIONS: one fixture per arm of the route-G shift
//! (`legacy_loop_generations.rs`), on synthetic abstract states.

use std::collections::{BTreeMap, BTreeSet};

use super::super::{
    AbstractPointer, AbstractState, Derivation, LegacyCfgNode, LegacyDiagnostic,
    LegacyDiagnosticKind, LegacyInstruction, PointerRefinement, RELEASED, refine_pointer_null,
};
use super::*;
use crate::loop_generations::{PREVIOUS_TAG, older, previous};

const SITE: usize = 3;

fn acquire_node() -> LegacyCfgNode {
    LegacyCfgNode {
        lookup: None,
        lookup_read: None,
        instruction: LegacyInstruction::Acquire {
            destination: "p".to_owned(),
            site: SITE,
        },
        successors: Vec::new(),
        line: 7,
        is_exit: false,
        loop_exit_edge: None,
    }
}

fn binding(targets: &[usize]) -> AbstractPointer {
    AbstractPointer {
        targets: targets.iter().copied().collect(),
        unknown: false,
        may_be_null: false,
        derivation: Derivation::Identity,
    }
}

fn run(state: AbstractState) -> (AbstractState, Vec<usize>, BTreeSet<LegacyDiagnostic>) {
    let mut diagnostics = BTreeSet::new();
    let (state, shifted) = shift("f", &acquire_node(), state, &mut diagnostics);
    (state, shifted, diagnostics)
}

fn refused(diagnostics: &BTreeSet<LegacyDiagnostic>, description: &str) -> bool {
    diagnostics.iter().any(|diagnostic| {
        matches!(&diagnostic.kind, LegacyDiagnosticKind::Unsupported { description: d } if d == description)
    })
}

/// Fresh: the site is not mentioned, so the state is returned unchanged.
#[test]
fn a_first_acquisition_shifts_nothing() {
    let mut state = AbstractState::default();
    state.resources.insert(9, LIVE);
    state.pointers.insert("q".to_owned(), binding(&[9]));
    let (after, shifted, diagnostics) = run(state.clone());
    assert_eq!(after, state);
    assert!(shifted.is_empty() && diagnostics.is_empty());
}

/// Shift: the current generation becomes the previous one, precisely; a
/// binding to it keeps its precision, and the site is free for the fresh one.
#[test]
fn the_current_generation_becomes_the_previous_one_precisely() {
    let mut state = AbstractState::default();
    state.resources.insert(SITE, LIVE);
    state.nullable_sites.insert(SITE);
    state.pointers.insert("p".to_owned(), binding(&[SITE]));
    state.pointers.insert("q".to_owned(), binding(&[SITE]));
    let (after, shifted, diagnostics) = run(state);
    assert_eq!(shifted, vec![SITE]);
    assert!(diagnostics.is_empty());
    assert!(!after.resources.contains_key(&SITE));
    assert_eq!(after.resources.get(&previous(SITE)), Some(&LIVE));
    assert!(after.nullable_sites.contains(&previous(SITE)));
    assert_eq!(after.pointers["q"], binding(&[previous(SITE)]));
}

/// The previous generation merges into the older summary: statuses join,
/// and every binding that named it becomes unknown (no precise update may
/// discharge a summary).
#[test]
fn the_previous_generation_merges_into_the_summary() {
    let mut state = AbstractState::default();
    state.resources.insert(SITE, LIVE);
    state.resources.insert(previous(SITE), RELEASED);
    state.resources.insert(older(SITE), LIVE);
    state
        .pointers
        .insert("old".to_owned(), binding(&[previous(SITE)]));
    let (after, _, diagnostics) = run(state);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    assert_eq!(after.resources.get(&older(SITE)), Some(&(LIVE | RELEASED)));
    assert_eq!(after.resources.get(&previous(SITE)), Some(&LIVE));
    let old = &after.pointers["old"];
    assert!(old.unknown && old.targets == BTreeSet::from([older(SITE)]));
}

/// ShiftRefused (held): a live previous generation a field link holds merges
/// into the summary only with a refusal, and the summary carries an
/// unresolved effect.
#[test]
fn a_held_live_generation_is_refused_at_the_merge() {
    let mut state = AbstractState::default();
    state.resources.insert(SITE, LIVE);
    state.resources.insert(previous(SITE), LIVE);
    state.resources.insert(9, LIVE);
    state.field_links.insert(
        9,
        BTreeMap::from([("next".to_owned(), BTreeSet::from([previous(SITE)]))]),
    );
    let (after, _, diagnostics) = run(state);
    assert!(refused(&diagnostics, SHIFT_REFUSED), "{diagnostics:?}");
    assert!(after.unresolved_effect_sites.contains(&older(SITE)));
    assert!(after.unmodeled);
    assert_eq!(
        after.field_links[&9]["next"],
        BTreeSet::from([older(SITE)]),
        "the edge follows the generation"
    );
}

/// ShiftRefused (unresolved): an owed previous generation meeting a summary
/// with an unresolved effect would be hidden by the merge, so it refuses.
#[test]
fn an_owed_generation_meeting_an_unresolved_summary_is_refused() {
    let mut state = AbstractState::default();
    state.resources.insert(SITE, LIVE);
    state.resources.insert(previous(SITE), LIVE);
    state.resources.insert(older(SITE), LIVE);
    state.unresolved_effect_sites.insert(older(SITE));
    let (_, _, diagnostics) = run(state);
    assert!(refused(&diagnostics, SHIFT_REFUSED), "{diagnostics:?}");
}

/// An out slot (an indirect place) holds a generation past the binding's
/// life: a live one merging into the summary refuses.
#[test]
fn an_out_slot_holding_a_live_generation_is_refused() {
    let mut state = AbstractState::default();
    state.resources.insert(SITE, LIVE);
    state.resources.insert(previous(SITE), LIVE);
    state
        .pointers
        .insert("*out".to_owned(), binding(&[previous(SITE)]));
    let (_, _, diagnostics) = run(state);
    assert!(refused(&diagnostics, SHIFT_REFUSED), "{diagnostics:?}");
}

/// Collections: two generations' element ranges are not joined; the summary's
/// becomes `Unknown`.
#[test]
fn two_generations_element_ranges_become_unknown() {
    let mut state = AbstractState::default();
    state.resources.insert(SITE, LIVE);
    state.resources.insert(previous(SITE), RELEASED);
    state.resources.insert(older(SITE), RELEASED);
    state.collections.insert(
        previous(SITE),
        crate::indexed_collection::Collection::default(),
    );
    state.collections.insert(
        older(SITE),
        crate::indexed_collection::Collection::default(),
    );
    let (after, _, _) = run(state);
    assert_eq!(
        after.collections.get(&older(SITE)),
        Some(&crate::indexed_collection::Collection::Unknown)
    );
}

/// A site the tags cannot index is refused, never shifted.
#[test]
fn an_unindexable_site_is_refused() {
    let mut state = AbstractState::default();
    let site = PREVIOUS_TAG;
    state.resources.insert(site, LIVE);
    let node = LegacyCfgNode {
        instruction: LegacyInstruction::Acquire {
            destination: "p".to_owned(),
            site,
        },
        ..acquire_node()
    };
    let mut diagnostics = BTreeSet::new();
    let (after, shifted) = shift("f", &node, state, &mut diagnostics);
    assert!(shifted.is_empty());
    assert!(refused(&diagnostics, SITE_NOT_INDEXABLE));
    assert!(after.unresolved_effect_sites.contains(&site));
}

/// The failure edge of the fresh acquisition removes only the fresh
/// generation: the earlier one, now previous, stays live.
#[test]
fn the_failure_edge_removes_only_the_fresh_generation() {
    let mut state = AbstractState::default();
    state.resources.insert(SITE, LIVE);
    state.nullable_sites.insert(SITE);
    state.pointers.insert(
        "p".to_owned(),
        AbstractPointer {
            may_be_null: true,
            ..binding(&[SITE])
        },
    );
    let (mut after, _, _) = run(state);
    // The node's own transfer: the fresh nullable generation at `p`.
    after.resources.insert(SITE, LIVE);
    after.nullable_sites.insert(SITE);
    after.pointers.insert(
        "p".to_owned(),
        AbstractPointer {
            may_be_null: true,
            ..binding(&[SITE])
        },
    );
    assert!(matches!(
        refine_pointer_null(&mut after, "p", true),
        PointerRefinement::Refined
    ));
    assert!(!after.resources.contains_key(&SITE));
    assert_eq!(after.resources.get(&previous(SITE)), Some(&LIVE));
}

/// A refused re-acquisition leaves the generation the call may have produced
/// present, possibly live and unresolved; an unrefused one is untouched.
#[test]
fn a_refused_reacquisition_leaves_its_generation_unresolved() {
    let mut state = AbstractState::default();
    let mut diagnostics = BTreeSet::new();
    settle_refused(&[SITE], &mut state, &diagnostics);
    assert!(state.resources.is_empty(), "no refusal, nothing settled");
    diagnostics.insert(LegacyDiagnostic {
        function: "f".to_owned(),
        line: 7,
        kind: LegacyDiagnosticKind::UnknownAlias {
            variable: "p".to_owned(),
        },
    });
    settle_refused(&[SITE], &mut state, &diagnostics);
    assert_eq!(state.resources.get(&SITE), Some(&LIVE));
    assert!(state.unresolved_effect_sites.contains(&SITE));
}

/// The withheld-obligation census reads exactly the possibly-live identities
/// with an unresolved effect that the caller does not receive.
#[test]
fn the_withheld_census_reads_live_unresolved_unexempt_identities() {
    let mut state = AbstractState::default();
    state.resources.insert(1, LIVE);
    state.resources.insert(2, LIVE);
    state.resources.insert(3, RELEASED);
    state.resources.insert(4, LIVE);
    state.unresolved_effect_sites.extend([1, 3, 4]);
    assert_eq!(withheld(&state, &BTreeSet::from([4])), BTreeSet::from([1]));
}
