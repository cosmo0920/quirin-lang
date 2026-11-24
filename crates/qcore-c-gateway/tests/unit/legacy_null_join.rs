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

//! Card UNTRACKED-BASE-NULL-JOIN-DISCHARGE (rule 83): the join and transfer
//! of `null_on_path`, bound to `QCore.NullJoinDischarge.joinVectors`. End to
//! end: `tests/untracked_base_null_join.rs`.

use super::super::super::null_premise_route::premised;
use super::super::indirect_scalar_base_resolves;
use super::*;

const LEAN: &str = include_str!("../../../../proof/QCore/NullJoinDischarge.lean");

/// One path's state binding `p` as the Lean `PathBinding` names it.
fn path(binding: &str) -> AbstractState {
    let pointer = match binding {
        "unbound" => return AbstractState::default(),
        "nullConst" => AbstractPointer {
            may_be_null: true,
            ..AbstractPointer::default()
        },
        "pathRead" => AbstractPointer {
            unknown: true,
            may_be_null: true,
            ..AbstractPointer::default()
        },
        "nonNull" => AbstractPointer {
            unknown: true,
            may_be_null: false,
            ..AbstractPointer::default()
        },
        other => panic!("unknown Lean path binding {other}"),
    };
    let mut state = AbstractState::default();
    state.pointers.insert("p".to_owned(), pointer);
    state
}

/// The rows of `joinVectors`: label, paths, discharged at the base, under
/// rule 83.
fn lean_rows() -> Vec<(String, Vec<String>, bool, bool)> {
    let block = LEAN
        .split("def joinVectors")
        .nth(1)
        .and_then(|rest| rest.split("\n]").next())
        .expect("joinVectors in the Lean module");
    block
        .lines()
        .filter_map(|line| {
            let line = line.trim().strip_prefix("(\"")?;
            let (label, rest) = line.split_once("\", [")?;
            let (paths, flags) = rest.split_once("], ")?;
            let flags = flags.trim_end_matches(',').trim_end_matches(')');
            let (old, new) = flags.split_once(", ")?;
            Some((
                label.to_owned(),
                paths
                    .split(", ")
                    .map(|path| path.trim_start_matches('.').to_owned())
                    .collect(),
                old == "true",
                new == "true",
            ))
        })
        .collect()
}

fn discharged(state: &AbstractState) -> (bool, bool) {
    let resolves = indirect_scalar_base_resolves(state, "p");
    let old = state.pointers.get("p").is_some_and(|pointer| {
        crate::null_premise::premised_shape(
            pointer.targets.is_empty(),
            pointer.unknown,
            pointer.may_be_null,
            pointer.derivation.is_identity(),
            state.widened,
        )
    });
    (resolves || old, resolves || premised(state, "p"))
}

#[test]
fn rule_83_join_vectors_hold() {
    let rows = lean_rows();
    assert_eq!(rows.len(), 14, "every joinVectors row is read");
    for (label, paths, old, new) in rows {
        let mut joined = path(&paths[0]);
        for next in &paths[1..] {
            joined.join(&path(next), &BTreeSet::new());
        }
        assert_eq!(discharged(&joined), (old, new), "{label}: {paths:?}");
    }
}

#[test]
fn a_definite_null_side_flags_the_joined_name() {
    let mut joined = path("nullConst");
    joined.join(&AbstractState::default(), &BTreeSet::new());
    assert!(refuses_premise(&joined, "p"));
    let mut clean = path("pathRead");
    clean.join(&AbstractState::default(), &BTreeSet::new());
    assert!(!refuses_premise(&clean, "p"));
    // Two unbound sides keep nothing.
    let mut none = AbstractState::default();
    none.join(&AbstractState::default(), &BTreeSet::new());
    assert!(none.null_on_path.is_empty());
}

#[test]
fn a_copy_carries_the_flag_and_a_rebinding_forgets_it() {
    let flagged = BTreeSet::from(["p".to_owned()]);
    let mut output = path("pathRead");
    output.pointers.insert(
        "q".to_owned(),
        output.pointers.get("p").cloned().expect("p bound"),
    );
    let copy = LegacyInstruction::Alias {
        destination: "q".to_owned(),
        source: "p".to_owned(),
    };
    transfer(&copy, &flagged, &mut output);
    assert_eq!(
        output.null_on_path,
        BTreeSet::from(["p".to_owned(), "q".to_owned()])
    );
    // `p = f`: the copy of a name not in the set rebinds `p`.
    let rebind = LegacyInstruction::Alias {
        destination: "p".to_owned(),
        source: "f".to_owned(),
    };
    let mut output = path("pathRead");
    transfer(&rebind, &flagged, &mut output);
    assert!(output.null_on_path.is_empty());
    // `p = 0` again: the binding is itself definitely null, so the flag is
    // not needed until a join.
    let clear = LegacyInstruction::Clear {
        destination: "p".to_owned(),
    };
    let mut output = path("nullConst");
    transfer(&clear, &flagged, &mut output);
    assert!(output.null_on_path.is_empty());
    // A name the output no longer binds is dropped.
    let mut output = AbstractState::default();
    transfer(&LegacyInstruction::NoOp, &flagged, &mut output);
    assert!(output.null_on_path.is_empty());
}

#[test]
fn the_flag_withholds_only_the_premise() {
    // A guard-narrowed non-null binding discharges whatever the flag says.
    let mut state = path("nonNull");
    state.null_on_path.insert("p".to_owned());
    assert!(indirect_scalar_base_resolves(&state, "p"));
    // A premised binding with the flag refuses; without it, it is premised.
    let mut state = path("pathRead");
    assert!(premised(&state, "p"));
    state.null_on_path.insert("p".to_owned());
    assert!(!premised(&state, "p"));
    assert!(definitely_null(&AbstractPointer {
        may_be_null: true,
        ..AbstractPointer::default()
    }));
}
