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

//! Provider-side readings and refusals for the two teardown families.
//!
//! The two fixtures are the *real* projections the gateway produces for the
//! bodies the in-process routes check; a conformance test asserts the live
//! import still equals them, so these cases cannot drift into a shape the
//! importer no longer emits. Every reading here is computed from the paths,
//! never read out of the projection.
use super::*;

const UNLINK: &str = include_str!("../../fixtures/list_unlink_projection.json");
const DESTRUCTOR: &str = include_str!("../../fixtures/field_destructor_projection.json");

fn unlink() -> TeardownSourceInput {
    serde_json::from_str(UNLINK).unwrap()
}

fn destructor() -> TeardownSourceInput {
    serde_json::from_str(DESTRUCTOR).unwrap()
}

fn unlink_parts(
    input: &mut TeardownSourceInput,
) -> (
    &mut TeardownFunctionRecord,
    &mut TeardownFunctionRecord,
    &mut String,
) {
    match &mut input.family {
        TeardownFamilyInput::ListUnlink {
            entry,
            helper,
            node_c_type,
        } => (entry, helper, node_c_type),
        TeardownFamilyInput::FieldDestructor { .. } => panic!("unlink projection expected"),
    }
}

fn destructor_parts(
    input: &mut TeardownSourceInput,
) -> (&mut TeardownFunctionRecord, &mut String, &mut Vec<String>) {
    match &mut input.family {
        TeardownFamilyInput::FieldDestructor {
            destructor,
            root_c_type,
            declared_owned_fields,
        } => (destructor, root_c_type, declared_owned_fields),
        TeardownFamilyInput::ListUnlink { .. } => panic!("destructor projection expected"),
    }
}

fn write(node: u32, field: &str, value: Option<u32>) -> LinkWriteRecord {
    LinkWriteRecord {
        node,
        field: field.into(),
        value,
    }
}

#[test]
fn reads_both_alias_classes_of_the_unlink_from_the_paths() {
    let Some(TeardownProposalRecord::ListUnlink(reading)) = derive(&unlink()) else {
        panic!("the real unlink projection must be readable")
    };
    assert_eq!(reading.function, "mk_list_del");
    assert_eq!(reading.helper_function, "__mk_list_del");
    assert_eq!(reading.node_c_type, "struct mk_list *");
    assert_eq!(reading.cases.len(), 2);
    // Distinct neighbors: the helper relinks 2 <-> 3, then the entry is nulled.
    assert!(!reading.cases[0].neighbors_alias);
    assert_eq!(reading.cases[0].helper_calls, 1);
    assert_eq!(
        reading.cases[0].writes,
        vec![
            write(2, "next", Some(3)),
            write(3, "prev", Some(2)),
            write(1, "prev", None),
            write(1, "next", None),
        ]
    );
    // Aliasing neighbors: the same node is both sides of the singleton ring.
    assert!(reading.cases[1].neighbors_alias);
    assert_eq!(
        reading.cases[1].writes,
        vec![
            write(2, "next", Some(2)),
            write(2, "prev", Some(2)),
            write(1, "prev", None),
            write(1, "next", None),
        ]
    );
}

#[test]
fn reads_every_realizable_nullable_state_of_the_destructor() {
    let Some(TeardownProposalRecord::FieldDestructor(reading)) = derive(&destructor()) else {
        panic!("the real destructor projection must be readable")
    };
    assert_eq!(reading.function, "cio_memfs_close");
    assert_eq!(reading.root_c_type, "struct cio_chunk *");
    // Four declared fields, but a null backend has no live children, so only
    // the nine realizable states appear.
    assert_eq!(reading.cases.len(), 9);
    assert_eq!(reading.cases[0].initially_live, Vec::<String>::new());
    assert_eq!(reading.cases[0].released_in_order, Vec::<String>::new());
    assert_eq!(reading.cases[0].source_path, 0);
    let all = reading.cases.last().unwrap();
    assert_eq!(
        all.initially_live,
        [
            "backend",
            "backend__buf_data",
            "backend__meta_data",
            "backend__name"
        ]
    );
    // Children are released before their parent, in source order.
    assert_eq!(
        all.released_in_order,
        [
            "backend__name",
            "backend__buf_data",
            "backend__meta_data",
            "backend"
        ]
    );
    assert_eq!(all.source_path, 1);
    assert!(
        reading
            .cases
            .iter()
            .all(|case| case.initially_live.is_empty() == case.released_in_order.is_empty())
    );
}

#[test]
fn every_departure_from_the_supported_unlink_refuses() {
    let mutate = |change: &dyn Fn(&mut TeardownSourceInput)| {
        let mut input = unlink();
        change(&mut input);
        derive(&input)
    };
    // Schema, identity and signature bindings.
    assert!(mutate(&|i| i.schema_version = 0).is_none());
    assert!(
        mutate(&|i| {
            let (entry, ..) = unlink_parts(i);
            entry.function.clear();
        })
        .is_none()
    );
    assert!(
        mutate(&|i| {
            let (entry, helper, _) = unlink_parts(i);
            entry.function = helper.function.clone();
        })
        .is_none()
    );
    assert!(
        mutate(&|i| {
            let (entry, ..) = unlink_parts(i);
            entry.source_revision = "other".into();
        })
        .is_none()
    );
    assert!(
        mutate(&|i| {
            let (entry, ..) = unlink_parts(i);
            entry.signature = "void (struct other *)".into();
        })
        .is_none()
    );
    assert!(
        mutate(&|i| {
            let (_, helper, _) = unlink_parts(i);
            helper.signature = "void (struct mk_list *)".into();
        })
        .is_none()
    );
    assert!(
        mutate(&|i| {
            let (.., node) = unlink_parts(i);
            node.clear();
        })
        .is_none()
    );
    // Path shape: an empty body, a missing return, a second path, and an
    // oversized body all refuse instead of being partially read.
    assert!(
        mutate(&|i| {
            let (entry, ..) = unlink_parts(i);
            entry.paths[0].operations.clear();
        })
        .is_none()
    );
    assert!(
        mutate(&|i| {
            let (entry, ..) = unlink_parts(i);
            entry.paths[0].operations.pop();
        })
        .is_none()
    );
    assert!(
        mutate(&|i| {
            let (entry, ..) = unlink_parts(i);
            let extra = entry.paths[0].clone();
            entry.paths.push(extra);
        })
        .is_none()
    );
    assert!(
        mutate(&|i| {
            let (entry, ..) = unlink_parts(i);
            let operation = entry.paths[0].operations[0].clone();
            entry.paths[0].operations = vec![operation; MAX_OPERATIONS + 1];
        })
        .is_none()
    );
    // The helper call must be the co-transported helper, at its own signature,
    // with the entry's own original neighbors in their own order.
    assert!(
        mutate(&|i| {
            let (entry, ..) = unlink_parts(i);
            let TeardownOperationRecord::Call { function, .. } = &mut entry.paths[0].operations[2]
            else {
                panic!("call at index 2")
            };
            *function = "memcpy".into();
        })
        .is_none()
    );
    assert!(
        mutate(&|i| {
            let (entry, ..) = unlink_parts(i);
            let TeardownOperationRecord::Call { arguments, .. } = &mut entry.paths[0].operations[2]
            else {
                panic!("call at index 2")
            };
            arguments.swap(0, 1);
        })
        .is_none()
    );
    assert!(
        mutate(&|i| {
            let (entry, ..) = unlink_parts(i);
            let TeardownOperationRecord::Call { arguments, .. } = &mut entry.paths[0].operations[2]
            else {
                panic!("call at index 2")
            };
            arguments.pop();
        })
        .is_none()
    );
    assert!(
        mutate(&|i| {
            let (entry, ..) = unlink_parts(i);
            let TeardownOperationRecord::Call { result_type, .. } =
                &mut entry.paths[0].operations[2]
            else {
                panic!("call at index 2")
            };
            *result_type = "int".into();
        })
        .is_none()
    );
    // An access to a cell with no supplied live binding, and a guard the
    // unlink family does not model, both refuse.
    assert!(
        mutate(&|i| {
            let (entry, ..) = unlink_parts(i);
            let TeardownOperationRecord::Read { place } = &mut entry.paths[0].operations[0] else {
                panic!("read at index 0")
            };
            let TeardownExpressionRecord::Field { name, .. } = &mut place.expression else {
                panic!("field read")
            };
            *name = "other".into();
        })
        .is_none()
    );
    assert!(
        mutate(&|i| {
            let (entry, ..) = unlink_parts(i);
            entry.paths[0].operations.insert(
                0,
                TeardownOperationRecord::Guard {
                    condition: TeardownValueRecord {
                        c_type: "int".into(),
                        expression: TeardownExpressionRecord::Integer { value: "1".into() },
                    },
                    taken: true,
                },
            );
        })
        .is_none()
    );
    // A second helper body is not a second helper call: the helper must have
    // exactly one complete path.
    assert!(
        mutate(&|i| {
            let (_, helper, _) = unlink_parts(i);
            let extra = helper.paths[0].clone();
            helper.paths.push(extra);
        })
        .is_none()
    );
}

#[test]
fn a_changed_unlink_body_changes_the_reading_instead_of_being_ignored() {
    // `prev->next = prev` instead of `= next` is one of the bodies the
    // in-process route refuses. It stays readable, and the reading records the
    // wrong link, so the core's comparison and its own contract evaluation
    // both see the change rather than a silently normalized result.
    let mut input = unlink();
    {
        let (_, helper, _) = unlink_parts(&mut input);
        let TeardownOperationRecord::Write { value, .. } = &mut helper.paths[0].operations[0]
        else {
            panic!("write at index 0")
        };
        value.expression = TeardownExpressionRecord::Parameter { index: 0 };
    }
    let Some(TeardownProposalRecord::ListUnlink(reading)) = derive(&input) else {
        panic!("a well-formed changed body is still readable")
    };
    assert_eq!(reading.cases[0].writes[0], write(2, "next", Some(2)));
    assert_ne!(
        reading.cases[0].writes,
        vec![
            write(2, "next", Some(3)),
            write(3, "prev", Some(2)),
            write(1, "prev", None),
            write(1, "next", None),
        ]
    );
}

#[test]
fn a_field_the_body_never_releases_is_reported_as_unreleased() {
    // A declared field no path touches does not make the reading refuse: the
    // states are still enumerated and the field is simply never released. The
    // core's own contract evaluation is what rejects the retained field, so
    // the reading must show it rather than hide it.
    let mut input = destructor();
    {
        let (_, _, fields) = destructor_parts(&mut input);
        fields.push("absent".into());
    }
    let Some(TeardownProposalRecord::FieldDestructor(reading)) = derive(&input) else {
        panic!("an untouched declared field is still readable")
    };
    let live_absent: Vec<_> = reading
        .cases
        .iter()
        .filter(|case| case.initially_live.iter().any(|field| field == "absent"))
        .collect();
    assert!(!live_absent.is_empty());
    assert!(
        live_absent
            .iter()
            .all(|case| !case.released_in_order.iter().any(|field| field == "absent"))
    );
}

#[test]
fn every_departure_from_the_supported_destructor_refuses() {
    let mutate = |change: &dyn Fn(&mut TeardownSourceInput)| {
        let mut input = destructor();
        change(&mut input);
        derive(&input)
    };
    assert!(mutate(&|i| i.schema_version = 7).is_none());
    assert!(
        mutate(&|i| {
            let (function, ..) = destructor_parts(i);
            function.signature = "void (struct other *)".into();
        })
        .is_none()
    );
    assert!(
        mutate(&|i| {
            let (_, root, _) = destructor_parts(i);
            root.clear();
        })
        .is_none()
    );
    // The declared inventory must be a nonempty, duplicate-free, bounded set
    // of nonempty names; the core separately requires it to be the pinned
    // contract's own inventory.
    assert!(
        mutate(&|i| {
            let (_, _, fields) = destructor_parts(i);
            fields.clear();
        })
        .is_none()
    );
    assert!(
        mutate(&|i| {
            let (_, _, fields) = destructor_parts(i);
            let first = fields[0].clone();
            fields.push(first);
        })
        .is_none()
    );
    assert!(
        mutate(&|i| {
            let (_, _, fields) = destructor_parts(i);
            fields.push(String::new());
        })
        .is_none()
    );
    assert!(
        mutate(&|i| {
            let (_, _, fields) = destructor_parts(i);
            *fields = (0..=MAX_FIELDS).map(|n| format!("field{n}")).collect();
        })
        .is_none()
    );
    // Dropping a declared field makes the freeing path read an unmodeled
    // field, which refuses instead of being silently skipped.
    assert!(
        mutate(&|i| {
            let (_, _, fields) = destructor_parts(i);
            fields.retain(|field| field != "backend__name");
        })
        .is_none()
    );
    // Path shape and bounds.
    assert!(
        mutate(&|i| {
            let (function, ..) = destructor_parts(i);
            function.paths.truncate(1);
        })
        .is_none()
    );
    assert!(
        mutate(&|i| {
            let (function, ..) = destructor_parts(i);
            let extra = function.paths[1].clone();
            function.paths.push(extra);
        })
        .is_none()
    );
    assert!(
        mutate(&|i| {
            let (function, ..) = destructor_parts(i);
            function.paths[1].operations.pop();
        })
        .is_none()
    );
    assert!(
        mutate(&|i| {
            let (function, ..) = destructor_parts(i);
            let operation = function.paths[0].operations[0].clone();
            function.paths[0].operations = vec![operation; MAX_OPERATIONS + 1];
        })
        .is_none()
    );
    // Releasing the parent while a declared child is still live is refused at
    // replay, not reported as a reading: the core's own replay refuses it the
    // same way, so neither route can describe a use-after-free ordering.
    assert!(
        mutate(&|i| {
            let (function, ..) = destructor_parts(i);
            let TeardownOperationRecord::Call { arguments, .. } =
                &mut function.paths[1].operations[8]
            else {
                panic!("free at index 8")
            };
            arguments[0] = TeardownValueRecord {
                c_type: "void *".into(),
                expression: TeardownExpressionRecord::Integer { value: "0".into() },
            };
        })
        .is_none()
    );
    // A duplicated release of one field, and a write inside a destructor path,
    // are both outside the family and refuse.
    assert!(
        mutate(&|i| {
            let (function, ..) = destructor_parts(i);
            let call = function.paths[1].operations[4].clone();
            function.paths[1].operations.insert(5, call);
        })
        .is_none()
    );
    assert!(
        mutate(&|i| {
            let (function, ..) = destructor_parts(i);
            let TeardownOperationRecord::Read { place } = function.paths[1].operations[3].clone()
            else {
                panic!("read at index 3")
            };
            function.paths[1].operations.insert(
                4,
                TeardownOperationRecord::Write {
                    place: place.clone(),
                    value: place,
                },
            );
        })
        .is_none()
    );
    // Flipping a guard makes both paths realizable for some state, so no
    // single complete path exists and the reading refuses.
    assert!(
        mutate(&|i| {
            let (function, ..) = destructor_parts(i);
            let TeardownOperationRecord::Guard { taken, .. } = &mut function.paths[1].operations[2]
            else {
                panic!("guard at index 2")
            };
            *taken = true;
        })
        .is_none()
    );
}

#[test]
fn a_destructor_that_stops_releasing_a_field_is_read_as_such() {
    // Freeing a null instead of `mf->meta_data` and `mf` leaves both live.
    // Symbol indices are positional, so the operations are retargeted in place
    // rather than removed; the reading changes and the core's own contract
    // evaluation then refuses the retained fields.
    let mut input = destructor();
    {
        let (function, ..) = destructor_parts(&mut input);
        for index in [8, 9] {
            let TeardownOperationRecord::Call { arguments, .. } =
                &mut function.paths[1].operations[index]
            else {
                panic!("free at index {index}")
            };
            arguments[0] = TeardownValueRecord {
                c_type: "void *".into(),
                expression: TeardownExpressionRecord::Integer { value: "0".into() },
            };
        }
    }
    let Some(TeardownProposalRecord::FieldDestructor(reading)) = derive(&input) else {
        panic!("a well-formed changed body is still readable")
    };
    let all = reading.cases.last().unwrap();
    assert_eq!(
        all.released_in_order,
        ["backend__name", "backend__buf_data"]
    );
    assert!(
        !all.released_in_order
            .contains(&"backend__meta_data".to_owned())
    );
    assert!(!all.released_in_order.contains(&"backend".to_owned()));
}
