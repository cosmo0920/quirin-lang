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

//! Card W45-FB-LIST-FINITENESS-PREMISE: the list-length decision and its
//! mechanical binding to `proof/QCore/ListLengthBound.lean`.

use super::*;

const LEAN: &str = include_str!("../../../../proof/QCore/ListLengthBound.lean");

/// The Lean `decideVectors` rows: gates and code.
fn lean_vectors() -> Vec<(Vec<bool>, String)> {
    let table = LEAN
        .split("def decideVectors : List (List Bool × String) := [")
        .nth(1)
        .expect("table header");
    let table = &table[..table.find("\n]").expect("end of table")];
    table
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("(["))
        .map(|row| {
            let row = row.trim_end_matches(',');
            let row = &row[1..row.len() - 1];
            let close = row.find(']').expect("gates");
            let gates = row[1..close]
                .split(',')
                .map(|cell| match cell.trim() {
                    "true" => true,
                    "false" => false,
                    other => panic!("not a Bool: {other}"),
                })
                .collect();
            let code = row[close + 1..]
                .trim_start_matches([',', ' '])
                .trim_matches('"')
                .to_owned();
            (gates, code)
        })
        .collect()
}

#[test]
fn the_decide_vectors_hold_and_match_the_lean_table() {
    for (gates, code) in LIST_DECIDE_VECTORS {
        assert_eq!(
            decide(&vector_shape(*gates)).map_or("admitted", ListRefusal::code),
            *code,
            "{gates:?}"
        );
    }
    let rust = LIST_DECIDE_VECTORS
        .iter()
        .map(|(gates, code)| (gates.to_vec(), (*code).to_owned()))
        .collect::<Vec<_>>();
    assert_eq!(lean_vectors(), rust);
}

#[test]
fn every_refusal_code_is_the_lean_code() {
    for refusal in [
        ListRefusal::NotAListTraversal,
        ListRefusal::CursorWritten,
        ListRefusal::HeadNotMember,
        ListRefusal::HeadRootNotStable,
        ListRefusal::PremiseNotDeclared,
        ListRefusal::OpaqueControl,
        ListRefusal::LabelInLoop,
        ListRefusal::FrameWritesLinks,
        ListRefusal::FrameIndirectCall,
        ListRefusal::FrameCalleeUndefined,
    ] {
        assert!(
            LEAN.contains(&format!("=> \"{}\"", refusal.code())),
            "{refusal:?} has no Lean code"
        );
    }
}

/// Every shape (all 1024): admitted only when every gate passes, the theorem
/// `an_admitted_shape_meets_the_premises`.
#[test]
fn an_admitted_shape_meets_the_premises() {
    for bits in 0u16..1024 {
        let flag = |n: u16| bits & (1 << n) != 0;
        let gates = [
            flag(0),
            flag(1),
            flag(2),
            flag(3),
            flag(4),
            flag(5),
            flag(6),
            flag(7),
            flag(8),
            flag(9),
        ];
        let shape = vector_shape(gates);
        let admitted = shape.list_traversal
            && shape.cursor_stable
            && shape.head_member
            && shape.head_root_stable
            && shape.premise_declared
            && !shape.opaque_control
            && !shape.label_in_loop
            && !shape.frame_writes_links
            && !shape.frame_indirect_call
            && !shape.frame_callee_undefined;
        assert_eq!(decide(&shape).is_none(), admitted, "{shape:?}");
    }
}

#[test]
fn type_spellings_classify_link_writes() {
    assert!(list_node("struct mk_list"));
    assert!(list_node("const struct cfl_list"));
    assert!(list_pointer("struct mk_list *"));
    assert!(list_pointer("struct cfl_list *const"));
    assert!(!list_pointer("struct mk_list **"));
    assert!(record("struct flb_time"));
    assert!(record("union value"));
    assert!(!record("struct flb_time *"));
    assert!(character("unsigned char"));
    assert!(!character("unsigned char *"));
    assert_eq!(
        pointee_tag("struct flb_config *").as_deref(),
        Some("flb_config")
    );
    assert_eq!(
        pointee_tag("const struct flb_config *").as_deref(),
        Some("flb_config")
    );
    assert_eq!(pointee_tag("struct flb_config **"), None);
}
