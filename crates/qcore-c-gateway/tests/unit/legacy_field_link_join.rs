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

//! Card B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED: one fixture per arm of the
//! route-G adapter (`legacy_field_link_join.rs`), on synthetic abstract
//! states.

use std::collections::{BTreeMap, BTreeSet};

use super::super::{RELEASED, loop_generations_route};
use super::*;

const OWNER: usize = 1;
const CHILD: usize = 2;
const OTHER: usize = 3;

fn linked(children: &[usize]) -> AbstractState {
    let mut state = AbstractState::default();
    state.resources.insert(OWNER, LIVE);
    for &child in children {
        state.resources.insert(child, LIVE);
    }
    if !children.is_empty() {
        state.field_links.insert(
            OWNER,
            BTreeMap::from([("item".to_owned(), children.iter().copied().collect())]),
        );
    }
    state
}

fn partial_children(state: &AbstractState) -> BTreeSet<usize> {
    state
        .partial_links
        .get(&OWNER)
        .and_then(|fields| fields.get("item"))
        .cloned()
        .unwrap_or_default()
}

/// The ledger's defect: the join of a path that linked the child with one
/// that did not recorded the child as unresolved and nothing else. The repair
/// adds no unresolved effect and records the link as partial.
#[test]
fn a_one_sided_link_joins_partial_and_adds_no_unresolved_effect() {
    let mut state = linked(&[CHILD]);
    let mut other = linked(&[]);
    other.resources.insert(CHILD, RELEASED);
    join(&mut state, &other);
    assert!(state.unresolved_effect_sites.is_empty());
    assert_eq!(partial_children(&state), BTreeSet::from([CHILD]));
    assert_eq!(
        state.field_links[&OWNER]["item"],
        BTreeSet::from([CHILD]),
        "the union is kept"
    );
    assert!(partial_field(&state, OWNER, "item"));
}

/// Linked alike on both sides: a must link, no partial triple.
#[test]
fn a_link_on_both_sides_stays_must() {
    let mut state = linked(&[CHILD]);
    join(&mut state, &linked(&[CHILD]));
    assert!(state.partial_links.is_empty());
    assert!(!partial_field(&state, OWNER, "item"));
}

/// Different children on the two sides: both are partial.
#[test]
fn different_children_are_both_partial() {
    let mut state = linked(&[CHILD]);
    join(&mut state, &linked(&[OTHER]));
    assert_eq!(partial_children(&state), BTreeSet::from([CHILD, OTHER]));
}

/// A partial triple survives a later join with a must side (it did not hold
/// on some path of its own side), and the join only grows.
#[test]
fn a_partial_triple_survives_a_join_with_a_must_side() {
    let mut state = linked(&[CHILD]);
    join(&mut state, &linked(&[]));
    let mut again = state.clone();
    join(&mut again, &linked(&[CHILD]));
    assert_eq!(partial_children(&again), BTreeSet::from([CHILD]));
    let mut reversed = linked(&[CHILD]);
    join(&mut reversed, &state);
    assert_eq!(partial_children(&reversed), BTreeSet::from([CHILD]));
}

/// A store writes the link on this path: it holds on every path from here.
#[test]
fn a_store_makes_the_link_hold() {
    let mut state = linked(&[CHILD]);
    join(&mut state, &linked(&[]));
    stored(&mut state, OWNER, "item");
    assert!(state.partial_links.is_empty());
}

/// Prune keeps partial triples a subset of the links.
#[test]
fn prune_drops_a_triple_whose_link_went() {
    let mut state = linked(&[CHILD]);
    join(&mut state, &linked(&[]));
    state.field_links.clear();
    prune(&mut state);
    assert!(state.partial_links.is_empty());
}

/// A generation shift renames the partial triples as it renames the links.
#[test]
fn a_rename_moves_the_partial_triples() {
    let mut state = linked(&[CHILD]);
    join(&mut state, &linked(&[]));
    rename(&mut state, CHILD, 40);
    assert_eq!(partial_children(&state), BTreeSet::from([40]));
    rename(&mut state, OWNER, 41);
    assert!(state.partial_links.contains_key(&41));
    assert!(!state.partial_links.contains_key(&OWNER));
}

/// The escape carries an exactly-live child linked on every path.
#[test]
fn an_escape_carries_a_must_linked_live_child() {
    let mut state = linked(&[CHILD]);
    let refused = escape_cascade(&mut state, OWNER);
    assert!(refused.is_empty());
    assert_eq!(state.resources[&CHILD], ESCAPED);
    assert!(state.field_links.is_empty());
}

/// The escape refuses a live child linked on one side only: the base escaped
/// it silently, hiding the other side's leak.
#[test]
fn an_escape_refuses_a_one_sided_live_child() {
    let mut state = linked(&[CHILD]);
    join(&mut state, &linked(&[]).tap_child(LIVE));
    let refused = escape_cascade(&mut state, OWNER);
    assert_eq!(refused, BTreeSet::from([CHILD]));
    assert_eq!(state.resources[&CHILD], LIVE, "the obligation is kept");
    assert!(state.unresolved_effect_sites.contains(&CHILD));
    assert!(state.partial_links.is_empty() && state.field_links.is_empty());
}

/// The escape refuses a must-linked child of mixed status (live on one path,
/// released on the other): the base left it unresolved silently.
#[test]
fn an_escape_refuses_a_mixed_child() {
    let mut state = linked(&[CHILD]);
    state.resources.insert(CHILD, LIVE | RELEASED);
    assert_eq!(escape_cascade(&mut state, OWNER), BTreeSet::from([CHILD]));
}

/// A released child is owed nothing: withdrawn as before, not refused.
#[test]
fn an_escape_withdraws_a_released_child() {
    let mut state = linked(&[CHILD]);
    state.resources.insert(CHILD, RELEASED);
    assert!(escape_cascade(&mut state, OWNER).is_empty());
    assert!(state.unresolved_effect_sites.contains(&CHILD));
    assert_eq!(state.resources[&CHILD], RELEASED);
}

/// A child reached through a partial link of a reached child is partial too.
#[test]
fn partiality_follows_the_reach() {
    let mut state = linked(&[CHILD]);
    state.resources.insert(OTHER, LIVE);
    state.field_links.insert(
        CHILD,
        BTreeMap::from([("next".to_owned(), BTreeSet::from([OTHER]))]),
    );
    let mut side = linked(&[]).tap_child(LIVE);
    side.resources.insert(OTHER, LIVE);
    join(&mut state, &side);
    let refused = escape_cascade(&mut state, OWNER);
    assert_eq!(refused, BTreeSet::from([CHILD, OTHER]));
}

/// The floor: a withheld identity on a modeled path refuses; on an unmodeled
/// path (a refusal already reported) it adds nothing; a caller's identity is
/// never withheld.
#[test]
fn the_exit_floor_refuses_a_withheld_obligation_on_a_modeled_path() {
    let mut state = AbstractState::default();
    state.resources.insert(CHILD, LIVE);
    state.unresolved_effect_sites.insert(CHILD);
    let mut diagnostics = BTreeSet::new();
    exit_backstop("f", 9, &state, &BTreeSet::new(), &mut diagnostics);
    assert!(diagnostics.iter().any(|diagnostic| matches!(
        &diagnostic.kind,
        LegacyDiagnosticKind::Unsupported { description } if description == EXIT_WITHHELD
    )));
    let mut quiet = BTreeSet::new();
    exit_backstop("f", 9, &state, &BTreeSet::from([CHILD]), &mut quiet);
    state.unmodeled = true;
    exit_backstop("f", 9, &state, &BTreeSet::new(), &mut quiet);
    assert!(quiet.is_empty());
    assert_eq!(
        loop_generations_route::withheld(&state, &BTreeSet::new()),
        BTreeSet::from([CHILD])
    );
}

trait TapChild {
    fn tap_child(self, status: u8) -> Self;
}

impl TapChild for AbstractState {
    fn tap_child(mut self, status: u8) -> Self {
        self.resources.insert(CHILD, status);
        self
    }
}
