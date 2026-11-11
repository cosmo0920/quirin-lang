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

//! Card CG24-ALIAS-EXTENT-FRAME-VOCABULARY: the frame and derived-relation
//! clauses of bounds contract v1 (grammar G05b, contract specification C09.1)
//! load total-or-reject with their static conditions, and the resource-v1
//! consumer still rejects them.

use std::collections::BTreeMap;

use qcore_contract::bounds_contract::{
    BoundsClause, BoundsCount, BoundsUnit, ParameterKind, RESERVED_SPELLINGS,
    load_bounds_contract_sources,
};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_core::index_extent_frames::FrameMode;

fn one(text: &str) -> BTreeMap<String, String> {
    BTreeMap::from([("frames.qv".to_owned(), format!("module contract;\n{text}"))])
}

const COPY: &str = "extern fn copy(dst: ptr<mut, u8>, src: ptr<const, u8>, n: usize) -> unit
contract {
    requires bounds_contract_v1;
    requires bounds_extent(dst, n, bytes);
    requires bounds_extent(src, n, bytes);
    ensures normal bounds_frame_writes(dst);
    ensures normal bounds_frame_reads(src);
};
";

const SDS: &str =
    "extern fn cfl_sds_cat_safe(buf: ptr<mut, ptr<mut, u8>>, str: ptr<const, u8>, len: i32) -> unit
contract {
    requires bounds_contract_v1;
    requires bounds_extent(str, len, bytes);
    ensures normal bounds_frame_moves(buf);
    ensures normal bounds_frame_reads(str);
};
";

const SKIP: &str = "extern fn skip(p: ptr<mut, u8>, off: usize) -> ptr<mut, u8>
contract {
    requires bounds_contract_v1;
    ensures normal bounds_frame_reads(p);
    ensures normal bounds_return_derived(p, off, 4, elements);
};
";

const PICK: &str = "extern fn pick(a: ptr<mut, u8>, b: ptr<mut, u8>) -> ptr<mut, u8>
contract {
    requires bounds_contract_v1;
    ensures normal bounds_return_derived(a, 0, 4, elements);
    ensures normal bounds_return_derived(b, 0, 4, elements);
};
";

#[test]
fn a_closed_frame_loads_one_mode_per_pointer_parameter() {
    let contracts = load_bounds_contract_sources(&one(COPY)).expect("loads");
    let copy = contracts.contract("copy").expect("copy");
    assert_eq!(
        copy.frame(),
        Some(vec![Some(FrameMode::Writes), Some(FrameMode::Reads), None])
    );
    assert!(copy.derived_bases().is_empty());
    // A declaration without frame clauses states no frame (CG16's rule).
    let plain = load_bounds_contract_sources(&one("extern fn plain(p: ptr<mut, u8>) -> unit
contract {
    requires bounds_contract_v1;
};
"))
    .expect("loads");
    assert_eq!(plain.contract("plain").unwrap().frame(), None);
}

#[test]
fn the_sds_slot_declaration_loads() {
    let contracts = load_bounds_contract_sources(&one(SDS)).expect("loads");
    let sds = contracts.contract("cfl_sds_cat_safe").expect("sds");
    assert_eq!(sds.parameters[0].kind, ParameterKind::Slot);
    assert!(sds.parameters[0].kind.is_pointer());
    assert_eq!(
        sds.frame(),
        Some(vec![Some(FrameMode::Moves), Some(FrameMode::Reads), None])
    );
}

#[test]
fn derived_relations_load_with_one_or_several_bases() {
    let contracts = load_bounds_contract_sources(&one(SKIP)).expect("loads");
    let skip = contracts.contract("skip").unwrap();
    assert!(skip.clauses.iter().any(|entry| entry.clause
        == BoundsClause::ReturnDerived {
            parameter: "p".into(),
            offset: BoundsCount::Parameter("off".into()),
            length: BoundsCount::Const(4),
            unit: BoundsUnit::Elements,
        }));
    assert_eq!(skip.derived_bases(), vec!["p"]);
    let contracts = load_bounds_contract_sources(&one(PICK)).expect("loads");
    assert_eq!(
        contracts.contract("pick").unwrap().derived_bases(),
        vec!["a", "b"]
    );
}

#[test]
fn malformed_frames_and_relations_reject_the_whole_module() {
    for (why, text) in [
        (
            "a frame names every pointer parameter",
            COPY.replace("    ensures normal bounds_frame_reads(src);\n", ""),
        ),
        (
            "one frame per parameter",
            COPY.replace(
                "bounds_frame_reads(src);",
                "bounds_frame_reads(src);\n    ensures normal bounds_frame_writes(src);",
            ),
        ),
        (
            "a frame names a pointer parameter",
            COPY.replace(
                "bounds_frame_reads(src);",
                "bounds_frame_reads(src);\n    ensures normal bounds_frame_reads(n);",
            ),
        ),
        (
            "a frame is a postcondition",
            COPY.replace(
                "ensures normal bounds_frame_reads(src);",
                "requires bounds_frame_reads(src);",
            ),
        ),
        (
            "a slot is stated only with a frame",
            SDS.replace("    ensures normal bounds_frame_moves(buf);\n", "")
                .replace("    ensures normal bounds_frame_reads(str);\n", ""),
        ),
        (
            "a slot has no extent of its own",
            SDS.replace(
                "requires bounds_extent(str, len, bytes);",
                "requires bounds_extent(buf, 1, elements);",
            ),
        ),
        (
            "preserving every extent contradicts a move",
            SDS.replace(
                "ensures normal bounds_frame_reads(str);",
                "ensures normal bounds_frame_reads(str);\n    ensures normal bounds_preserves_extents();",
            ),
        ),
        (
            "a derived base is a pointer parameter",
            SKIP.replace("bounds_return_derived(p, off,", "bounds_return_derived(off, off,"),
        ),
        (
            "a derived result is a pointer",
            SKIP.replace(") -> ptr<mut, u8>", ") -> unit"),
        ),
        (
            "a derived result and a return extent exclude each other",
            SKIP.replace(
                "ensures normal bounds_return_derived(p, off, 4, elements);",
                "ensures normal bounds_return_derived(p, off, 4, elements);\n    ensures normal bounds_return_extent(4, elements);",
            ),
        ),
        (
            "a derived result cannot alias a base the call may move",
            SKIP.replace("bounds_frame_reads(p);", "bounds_frame_moves(p);"),
        ),
        (
            "one relation per base",
            SKIP.replace(
                "ensures normal bounds_return_derived(p, off, 4, elements);",
                "ensures normal bounds_return_derived(p, off, 4, elements);\n    ensures normal bounds_return_derived(p, 0, 4, elements);",
            ),
        ),
        (
            "a relation's offset is a count",
            SKIP.replace("bounds_return_derived(p, off,", "bounds_return_derived(p, p,"),
        ),
        (
            "a relation names its unit",
            SKIP.replace("4, elements);", "4, words);"),
        ),
    ] {
        let result = load_bounds_contract_sources(&one(&text));
        assert!(result.is_err(), "{why}: {text}");
    }
}

/// Acceptance control: the resource-v1 consumer presented with a frame or a
/// derived relation rejects it, exactly as it rejects every CG16 clause
/// (CG00's K04/K06 controls: resource v1 states neither).
#[test]
fn the_resource_v1_consumer_rejects_frames_and_relations() {
    for clause in [
        "ensures normal bounds_frame_reads(p);",
        "ensures normal bounds_return_derived(p, 0, 4, elements);",
    ] {
        let mixed = format!(
            "extern fn q_look(p: ptr<mut, u8>) -> ptr<mut, u8>
contract {{
    requires resource_contract_v1;
    ensures normal resource_borrowed(p);
    ensures normal resource_return_none();
    {clause}
}};
"
        );
        assert!(
            load_resource_contract_sources(&one(&mixed)).is_err(),
            "{clause}"
        );
    }
    assert!(load_resource_contract_sources(&one(COPY)).is_err());
}

#[test]
fn the_new_spellings_are_reserved() {
    for spelling in [
        "bounds_frame_reads",
        "bounds_frame_writes",
        "bounds_frame_moves",
        "bounds_return_derived",
    ] {
        assert!(RESERVED_SPELLINGS.contains(&spelling), "{spelling}");
    }
}
