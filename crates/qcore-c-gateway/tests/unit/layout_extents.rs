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

//! Unit tests of card CG17-LAYOUT-DERIVED-EXTENTS (gateway rule 11u): the
//! four decisions, their mechanical binding to
//! `proof/QCore/LayoutExtents.lean`, and the Clang readers on hand-built
//! nodes.

use super::*;

const LEAN: &str = include_str!("../../../../proof/QCore/LayoutExtents.lean");

/// The tuples of the Lean list `def NAME : … := [ … ]`, one per line,
/// whitespace-normalised.
fn lean_rows(name: &str) -> Vec<String> {
    let lean = LEAN.replace("\r\n", "\n");
    let start = format!("def {name} :");
    let body = lean.split(&start).nth(1).expect("the Lean vector list");
    let body = &body[body.find(":=").expect("a definition") + 2..];
    let body = &body[..body.find("\n\ntheorem").expect("the list's end")];
    body.lines()
        .map(|line| {
            line.trim()
                .trim_start_matches(['[', ','])
                .trim_end_matches(']')
                .trim()
                .to_owned()
        })
        .filter(|line| !line.is_empty())
        .collect()
}

fn lean_flex(extent: FlexExtent) -> String {
    match extent {
        FlexExtent::Elements(n) => format!(".elements {n}"),
        FlexExtent::Refused(FlexRefusal::SizeNotEstablished) => {
            ".refused .sizeNotEstablished".into()
        }
        FlexExtent::Refused(FlexRefusal::BelowMember) => ".refused .belowMember".into(),
        FlexExtent::Refused(FlexRefusal::ZeroWidth) => ".refused .zeroWidth".into(),
    }
}

#[test]
fn flex_vectors_are_leans() {
    let rust = FLEX_VECTORS
        .iter()
        .map(|(size, offset, width, extent)| {
            let size = size.map_or("none".to_owned(), |s| format!("some {s}"));
            format!("({size}, {offset}, {width}, {})", lean_flex(*extent))
        })
        .collect::<Vec<_>>();
    assert_eq!(rust, lean_rows("flexVectors"));
    for (size, offset, width, extent) in FLEX_VECTORS {
        assert_eq!(flexible_extent(*size, *offset, *width), *extent);
    }
}

fn lean_bound(bound: BoundClass) -> &'static str {
    match bound {
        BoundClass::ReadsOnly => ".readsOnly",
        BoundClass::Effect => ".effect",
        BoundClass::Unread => ".unread",
    }
}

fn lean_use(use_: VlaUse) -> &'static str {
    match use_ {
        VlaUse::Element => ".element",
        VlaUse::SizeQuery => ".sizeQuery",
        VlaUse::Address => ".address",
    }
}

fn lean_vla(decision: VlaDecision) -> &'static str {
    match decision {
        VlaDecision::Admitted => ".admitted",
        VlaDecision::ElementNotScalar => ".elementNotScalar",
        VlaDecision::BoundEffect => ".boundEffect",
        VlaDecision::BoundUnread => ".boundUnread",
        VlaDecision::AddressEscapes => ".addressEscapes",
    }
}

#[test]
fn vla_vectors_are_leans() {
    let rust = VLA_VECTORS
        .iter()
        .map(|(scalar, bound, uses, decision)| {
            let uses = uses
                .iter()
                .map(|u| lean_use(*u))
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "({scalar}, {}, [{uses}], {})",
                lean_bound(*bound),
                lean_vla(*decision)
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(rust, lean_rows("vlaVectors"));
    for (scalar, bound, uses, decision) in VLA_VECTORS {
        assert_eq!(vla_decision(*scalar, *bound, uses), *decision);
    }
    // `obligationsOf`: only an admitted declaration carries the numeric
    // obligations, and it carries all three.
    assert_eq!(
        obligations_of(VlaDecision::Admitted)
            .iter()
            .map(|o| o.code())
            .collect::<Vec<_>>(),
        ["bound-defined", "bound-positive", "size-representable"]
    );
    assert!(LEAN.contains("| .admitted => [.boundDefined, .boundPositive, .sizeRepresentable]"));
    assert!(obligations_of(VlaDecision::AddressEscapes).is_empty());
}

fn lean_offset(offset: Offset) -> String {
    match offset {
        Offset::Const(k) if k < 0 => format!(".const ({k})"),
        Offset::Const(k) => format!(".const {k}"),
        Offset::Scaled(s) => format!(".scaled {s}"),
        Offset::Unknown => ".unknown".into(),
    }
}

#[test]
fn alignment_vectors_are_leans() {
    let rust = ALIGNMENT_VECTORS
        .iter()
        .map(|(required, base, offset, answer)| {
            format!(
                "({required}, {base}, {}, .{})",
                lean_offset(*offset),
                match answer {
                    Alignment::Aligned => "aligned",
                    Alignment::Misaligned => "misaligned",
                    Alignment::Undecided => "undecided",
                }
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(rust, lean_rows("alignmentVectors"));
    for (required, base, offset, answer) in ALIGNMENT_VECTORS {
        assert_eq!(alignment(*required, *base, *offset), *answer);
    }
    assert_eq!(alignment_verdict(&[]), PropertyVerdict::Safe);
    assert_eq!(
        alignment_verdict(&[Alignment::Aligned, Alignment::Misaligned]),
        PropertyVerdict::Unknown
    );
    assert_eq!(
        alignment_verdict(&[Alignment::Undecided]),
        PropertyVerdict::Unknown
    );
}

#[test]
fn container_vectors_are_leans() {
    let path = |p: &[u32]| {
        format!(
            "[{}]",
            p.iter().map(u32::to_string).collect::<Vec<_>>().join(", ")
        )
    };
    let rust = container_vectors()
        .iter()
        .map(|(provenance, record, q, decision)| {
            let provenance = match provenance {
                Provenance::MemberOf {
                    base,
                    record,
                    path: p,
                } => {
                    format!(".memberOf {base} {record} {}", path(p))
                }
                Provenance::Unestablished(_) => ".unestablished".into(),
            };
            let decision = match decision {
                ContainerDecision::Enclosing(base) => format!(".enclosing {base}"),
                ContainerDecision::Unestablished(_) => ".unestablished".into(),
                ContainerDecision::RecordMismatch => ".recordMismatch".into(),
                ContainerDecision::MemberMismatch => ".memberMismatch".into(),
            };
            format!("({provenance}, {record}, {}, {decision})", path(q))
        })
        .collect::<Vec<_>>();
    assert_eq!(rust, lean_rows("containerVectors"));
    for (provenance, record, q, decision) in container_vectors() {
        assert_eq!(container_of(&provenance, &record, q), decision);
    }
}

#[test]
fn type_texts_are_read_fail_closed() {
    assert_eq!(vla_type("int[n]"), Some(("int", "n")));
    assert_eq!(
        vla_type("unsigned long[n * 2]"),
        Some(("unsigned long", "n * 2"))
    );
    assert_eq!(vla_type("int[n][m]"), None);
    assert!(builtin_scalar("unsigned long"));
    assert!(!builtin_scalar("size_t"));
    assert!(!builtin_scalar("void *"));
    assert!(!builtin_scalar("const"));
    assert_eq!(pointee("struct S *"), Some("struct S"));
    assert_eq!(pointee("const int *const"), Some("const int"));
    assert_eq!(pointee("int"), None);
    assert_eq!(pointee("void (*)(void)"), None);
    assert!(byte_aligned("const unsigned char"));
    assert!(byte_aligned("void"));
    assert!(!byte_aligned("int"));
}

#[test]
fn decide_cast_takes_clangs_numbers_and_refuses_without_them() {
    let cast = DerivedCast {
        begin: None,
        target: "int".into(),
        base: BaseAddress::PointerValue("char".into()),
        offset: OffsetTerm::Const {
            elements: 1,
            element: "char".into(),
        },
    };
    let numbers = |query: &str| match query {
        "_Alignof(int)" => Some(4),
        "sizeof(int)" => Some(4),
        _ => None,
    };
    assert_eq!(decide_cast(&cast, &numbers).3, Alignment::Undecided);
    let aligned = DerivedCast {
        base: BaseAddress::PointerValue("int".into()),
        offset: OffsetTerm::Scaled {
            element: "int".into(),
        },
        ..cast.clone()
    };
    assert_eq!(decide_cast(&aligned, &numbers).3, Alignment::Aligned);
    let misaligned = DerivedCast {
        base: BaseAddress::Object("int".into()),
        ..cast.clone()
    };
    assert_eq!(decide_cast(&misaligned, &numbers).3, Alignment::Misaligned);
    // No number from Clang: never decided.
    assert_eq!(decide_cast(&aligned, &|_| None).3, Alignment::Undecided);
    assert_eq!(
        cast_queries(&aligned),
        ["_Alignof(int)", "_Alignof(int)", "sizeof(int)"]
    );
}
