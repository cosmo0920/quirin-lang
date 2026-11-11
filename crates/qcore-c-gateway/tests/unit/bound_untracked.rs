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

//! Card CG10-BOUND-UNTRACKED-ROOTS (rule 11d): the decision table and its Lean
//! binding.

use super::*;

fn binding(code: &str) -> BindingClass {
    match code {
        "absent" => BindingClass::Absent,
        "target-free" => BindingClass::TargetFree,
        "nullable" => BindingClass::Nullable,
        "unresolved" => BindingClass::Unresolved,
        "tracked" => BindingClass::Tracked,
        other => panic!("unknown binding class {other}"),
    }
}

#[test]
fn the_decision_vectors_hold() {
    for &(eligible, class, rooted, refused, expected) in DECISION_VECTORS {
        assert_eq!(
            decide(eligible, binding(class), rooted, refused).code(),
            expected,
            "{eligible} {class} {rooted} {refused}"
        );
    }
}

#[test]
fn only_an_untracked_binding_on_an_unrefused_path_holds() {
    for eligible in [false, true] {
        for class in ["absent", "target-free", "nullable", "unresolved", "tracked"] {
            for rooted in [false, true] {
                for refused in [false, true] {
                    let holds = decide(eligible, binding(class), rooted, refused) == Premise::Holds;
                    assert_eq!(
                        holds,
                        eligible
                            && matches!(class, "absent" | "target-free")
                            && !rooted
                            && !refused
                    );
                }
            }
        }
    }
}

#[test]
fn a_place_rooted_at_the_root_is_recognised() {
    for place in ["r->f", "r.f", "r[0]", "*r", "(*r).f", "r->f->g"] {
        assert!(rooted_at(place, "r"), "{place}");
    }
    for place in ["r", "rk->f", "q->r", "*rk"] {
        assert!(!rooted_at(place, "r"), "{place}");
    }
}

/// `camelCase` Lean constructor to the Rust code (`targetFree` → `target-free`).
fn kebab(name: &str) -> String {
    let mut code = String::new();
    for character in name.trim().trim_start_matches('.').chars() {
        if character.is_ascii_uppercase() {
            code.push('-');
            code.push(character.to_ascii_lowercase());
        } else {
            code.push(character);
        }
    }
    code
}

/// The rows of one Lean list literal `def NAME … :=` up to its closing `]`.
fn lean_rows(lean: &str, name: &str) -> Vec<String> {
    let lean = lean.replace("\r\n", "\n");
    let block = lean
        .split(&format!("def {name} "))
        .nth(1)
        .unwrap_or_else(|| panic!("{name} in the Lean model"));
    let block = &block[block.find(":=").expect("definition") + 2..];
    let block = &block[..block.find(" ]\n").expect("end of list")];
    block
        .lines()
        .map(|line| line.trim().trim_start_matches(['[', ',']).trim().to_owned())
        .filter(|line| line.starts_with('('))
        .collect()
}

#[test]
fn the_lean_decision_vectors_are_the_rust_table() {
    let lean = include_str!("../../../../proof/QCore/BoundUntracked.lean");
    let rows = lean_rows(lean, "decisionVectors")
        .into_iter()
        .map(|row| {
            let fields = row
                .trim_start_matches('(')
                .trim_end_matches(')')
                .split(',')
                .map(str::trim)
                .collect::<Vec<_>>();
            assert_eq!(fields.len(), 5, "{row}");
            (
                fields[0] == "true",
                kebab(fields[1]),
                fields[2] == "true",
                fields[3] == "true",
                kebab(fields[4]),
            )
        })
        .collect::<Vec<_>>();
    let rust = DECISION_VECTORS
        .iter()
        .map(|&(e, b, r, f, p)| (e, b.to_owned(), r, f, p.to_owned()))
        .collect::<Vec<_>>();
    assert_eq!(rows, rust);
}

#[test]
fn the_lean_event_vectors_are_the_rust_table() {
    let lean = include_str!("../../../../proof/QCore/BoundUntracked.lean");
    let event = |text: &str| -> String {
        match text.split_whitespace().collect::<Vec<_>>().as_slice() {
            [".custody", mode] => kebab(mode),
            [".rebindOwned", _site] => "rebind-owned".to_owned(),
            [single] => kebab(single),
            other => panic!("unknown Lean event {other:?}"),
        }
    };
    let rows = lean_rows(lean, "eventVectors")
        .into_iter()
        .map(|row| {
            let (events, premise) = row
                .trim_start_matches("([")
                .trim_end_matches(')')
                .split_once("],")
                .expect("events and premise");
            let events = events
                .split(',')
                .filter(|text| !text.trim().is_empty())
                .map(event)
                .collect::<Vec<_>>();
            (events, kebab(premise))
        })
        .collect::<Vec<_>>();
    let rust = EVENT_VECTORS
        .iter()
        .map(|&(events, premise)| {
            (
                events
                    .iter()
                    .map(|event| (*event).to_owned())
                    .collect::<Vec<_>>(),
                premise.to_owned(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(rows, rust);
}

#[test]
fn the_integration_copy_of_the_event_vectors_is_this_table() {
    // `tests/bound_untracked_roots.rs` runs every event vector through the
    // gateway; it holds a copy because this module is crate-private.
    let integration = include_str!("../bound_untracked_roots.rs");
    let block = integration
        .split("pub const EVENT_VECTORS: &[(&[&str], &str)] = &[")
        .nth(1)
        .expect("the integration copy");
    let block = &block[..block.find("];").expect("end of the copy")];
    let copy = block
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let rust = EVENT_VECTORS
        .iter()
        .map(|&(events, premise)| {
            let events = events
                .iter()
                .map(|event| format!("\"{event}\""))
                .collect::<Vec<_>>()
                .join(", ");
            format!("(&[{events}], \"{premise}\"),")
        })
        .collect::<Vec<_>>();
    assert_eq!(copy, rust);
}
