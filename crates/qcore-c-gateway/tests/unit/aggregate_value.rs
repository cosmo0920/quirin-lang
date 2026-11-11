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

//! Card CG14-AGGREGATE-VALUES-AND-INITIALIZERS (rule 11f): the decision
//! tables, their exactly-when characterisations, and their mechanical binding
//! to `QCore.AggregateValue`.

use super::*;

fn class(code: &str) -> TypeClass {
    [
        TypeClass::Scalar,
        TypeClass::Pointer,
        TypeClass::CustodyFree,
        TypeClass::PointerBearing,
        TypeClass::Unread,
    ]
    .into_iter()
    .find(|class| class.code() == code)
    .unwrap_or_else(|| panic!("unknown class {code}"))
}

fn side_of(code: &str) -> Side {
    [Side::Place, Side::Local, Side::Other]
        .into_iter()
        .find(|side| side.code() == code)
        .unwrap_or_else(|| panic!("unknown side {code}"))
}

const LEAVES: [Leaf; 15] = [
    Leaf::Zero,
    Leaf::Scalar,
    Leaf::StringArray,
    Leaf::Null,
    Leaf::StringLiteral,
    Leaf::Function,
    Leaf::CustodyFreeRecord,
    Leaf::PointerBearingRecord,
    Leaf::FormalPointer,
    Leaf::LocalPointer,
    Leaf::PointerRead,
    Leaf::Address,
    Leaf::Call,
    Leaf::CompoundLiteral,
    Leaf::Other,
];

fn leaf(code: &str) -> Leaf {
    LEAVES
        .into_iter()
        .find(|leaf| leaf.code() == code)
        .unwrap_or_else(|| panic!("unknown leaf {code}"))
}

fn use_of(code: &str) -> LiteralUse {
    [
        LiteralUse::Borrowed,
        LiteralUse::CustodyTaking,
        LiteralUse::Elsewhere,
    ]
    .into_iter()
    .find(|use_| use_.code() == code)
    .unwrap_or_else(|| panic!("unknown use {code}"))
}

fn outcome(result: Result<(), Refusal>) -> String {
    result.map_or_else(|refusal| refusal.code(), |()| "admit".to_owned())
}

#[test]
fn the_copy_vectors_hold() {
    for (spelling, class_code, destination, source, decision) in COPY_VECTORS {
        assert_eq!(
            outcome(decide_copy(
                class(class_code),
                side_of(destination),
                side_of(source)
            )),
            *decision,
            "{spelling}"
        );
    }
}

/// A copy is admitted exactly for a custody-free record between two typed
/// sides (`QCore.AggregateValue.copy_admitted_exactly_when`), over every input.
#[test]
fn a_copy_is_admitted_exactly_when_custody_free_between_typed_sides() {
    for class_code in [
        "scalar",
        "pointer",
        "custody-free",
        "pointer-bearing",
        "unread",
    ] {
        for destination in ["place", "frame", "other"] {
            for source in ["place", "frame", "other"] {
                assert_eq!(
                    decide_copy(class(class_code), side_of(destination), side_of(source)).is_ok(),
                    class_code == "custody-free" && destination != "other" && source != "other",
                    "{class_code} {destination} {source}"
                );
            }
        }
    }
}

#[test]
fn the_leaf_vectors_hold_and_cover_every_leaf() {
    assert_eq!(LEAF_VECTORS.len(), LEAVES.len());
    for (code, admitted) in LEAF_VECTORS {
        assert_eq!(leaf(code).admitted(), *admitted, "{code}");
    }
}

#[test]
fn the_literal_vectors_hold() {
    for (identity_free, reads, use_, decision) in LITERAL_VECTORS {
        assert_eq!(
            outcome(decide_literal(*identity_free, *reads, use_of(use_))),
            *decision,
            "{identity_free} {reads} {use_}"
        );
    }
}

/// A literal is admitted exactly at a borrowed formal, identity-free and
/// reading nothing (`QCore.AggregateValue.literal_admitted_exactly_when`).
#[test]
fn a_literal_is_admitted_exactly_when_borrowed_identity_free_and_read_free() {
    for identity_free in [false, true] {
        for reads in [false, true] {
            for use_ in ["borrowed", "custody-taking", "elsewhere"] {
                assert_eq!(
                    decide_literal(identity_free, reads, use_of(use_)).is_ok(),
                    identity_free && !reads && use_ == "borrowed",
                    "{identity_free} {reads} {use_}"
                );
            }
        }
    }
}

/// The mechanical binding of `proof/QCore/AggregateValue.lean`: its three
/// vector tables are the Rust tables, row by row.
#[test]
fn cg14_lean_vectors_match_the_rust_tables() {
    let lean = include_str!("../../../../proof/QCore/AggregateValue.lean");
    let table = |name: &str| -> Vec<Vec<String>> {
        let start = lean
            .find(&format!("def {name}"))
            .unwrap_or_else(|| panic!("{name} in the Lean model"));
        let body = &lean[start..];
        let open = body.find(":= [").expect("a list literal") + 4;
        let close = body[open..].find("\n]").expect("the list's end") + open;
        body[open..close]
            .split("),")
            .map(|row| {
                row.trim()
                    .trim_start_matches('(')
                    .trim_end_matches(')')
                    .split(',')
                    .map(|field| field.trim().trim_matches('"').to_owned())
                    .filter(|field| !field.is_empty())
                    .collect::<Vec<_>>()
            })
            .filter(|row| !row.is_empty())
            .collect()
    };
    let copies = table("copyVectors");
    assert_eq!(copies.len(), COPY_VECTORS.len());
    for (row, (_, class_code, destination, source, decision)) in copies.iter().zip(COPY_VECTORS) {
        assert_eq!(
            row,
            &[
                format!(".{}", lean_name(class_code)),
                format!(".{destination}"),
                format!(".{source}"),
                (*decision).to_owned(),
            ],
            "copy row {row:?}"
        );
    }
    let leaves = table("leafVectors");
    assert_eq!(leaves.len(), LEAF_VECTORS.len());
    for (row, (code, admitted)) in leaves.iter().zip(LEAF_VECTORS) {
        assert_eq!(
            row,
            &[format!(".{}", lean_name(code)), admitted.to_string()],
            "leaf row {row:?}"
        );
    }
    let literals = table("literalVectors");
    assert_eq!(literals.len(), LITERAL_VECTORS.len());
    for (row, (identity_free, reads, use_, decision)) in literals.iter().zip(LITERAL_VECTORS) {
        assert_eq!(
            row,
            &[
                identity_free.to_string(),
                reads.to_string(),
                format!(".{}", lean_name(use_)),
                (*decision).to_owned(),
            ],
            "literal row {row:?}"
        );
    }
}

/// A Rust code (`custody-free`) as the Lean constructor (`custodyFree`).
fn lean_name(code: &str) -> String {
    let mut out = String::new();
    let mut upper = false;
    for c in code.chars() {
        if c == '-' {
            upper = true;
        } else if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

#[test]
fn string_literal_lengths_decode_or_decline() {
    assert_eq!(decoded_length("\"abc\""), Some(3));
    assert_eq!(decoded_length("\"a\\nb\""), Some(3));
    assert_eq!(decoded_length("\"\\x41\\0\""), Some(2));
    assert_eq!(decoded_length("\"\""), Some(0));
    assert_eq!(decoded_length("\"\\101\""), None);
    assert_eq!(decoded_length("L\"abc\""), None);
    let extent = |array_bytes, literal_bytes| StringExtent {
        array_bytes,
        literal_bytes,
    };
    assert_eq!(extent(4, Some(3)).terminated(), Some(true));
    assert_eq!(extent(3, Some(3)).terminated(), Some(false));
    assert_eq!(extent(8, None).terminated(), None);
}
