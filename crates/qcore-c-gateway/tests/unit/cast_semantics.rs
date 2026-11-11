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

//! Card CAST-FORMALIZATION (rule 12h): the conversion table and its
//! classifier, bound to `QCore.CastSemantics` by shared vectors.

use super::*;

#[test]
fn every_classify_vector_holds() {
    for (kind, from, to, class) in CLASSIFY_VECTORS {
        assert_eq!(
            classify(kind, from, to).name(),
            class,
            "{kind} {from} -> {to}"
        );
    }
}

#[test]
fn the_class_table_has_one_row_per_class_and_unique_names() {
    let mut names = CONVERSION_TABLE
        .iter()
        .map(|row| row.class.name())
        .collect::<Vec<_>>();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), CONVERSION_TABLE.len());
}

#[test]
fn the_class_table_refines_rule_11b_kind_table() {
    assert!(agrees_with_cast_table());
}

#[test]
fn identity_and_access_rows_are_the_formal_model() {
    use ConversionClass as C;
    // View and round-trip rows preserve the identity.
    for class in [C::ByteView, C::TypedView, C::PunnedView, C::Qualifier] {
        assert!(class.preserves_identity(), "{class:?}");
    }
    // Pointer to integer loses it; integer to pointer and null have none.
    assert_eq!(conversion(C::PointerToInteger).identity, Identity::Lost);
    assert_eq!(conversion(C::NullTest).identity, Identity::Lost);
    assert_eq!(conversion(C::IntegerToPointer).identity, Identity::Absent);
    assert_eq!(conversion(C::NullPointer).identity, Identity::Absent);
    // A typed view needs the effective-type premise; a punned one is refused.
    assert_eq!(conversion(C::TypedView).access, Access::EffectiveType);
    assert!(
        conversion(C::TypedView)
            .obligations
            .contains(&P_EFFECTIVE_TYPE)
    );
    assert_eq!(conversion(C::PunnedView).access, Access::Punned);
    assert_eq!(conversion(C::ByteView).access, Access::Character);
    // Function and object pointers never convert into each other.
    assert_eq!(conversion(C::FunctionObject).identity, Identity::Refused);
    // `(void) e` has no value.
    assert_eq!(conversion(C::Discard).identity, Identity::NoValue);
    // Scalars are data.
    assert_eq!(conversion(C::Scalar).identity, Identity::Data);
}

#[test]
fn type_classes_read_clang_spellings() {
    assert_eq!(
        type_class("void (*)(void *)").0,
        TypeClass::Pointer(Pointee::Function)
    );
    assert_eq!(
        type_class("int (*)[4]").0,
        TypeClass::Pointer(Pointee::Object("int[4]".into()))
    );
    // A pointer to a function pointer points to an object.
    assert!(matches!(
        type_class("void (**)(int)").0,
        TypeClass::Pointer(Pointee::Object(_))
    ));
    assert_eq!(
        type_class("const char *const").0,
        TypeClass::Pointer(Pointee::Character("char".into()))
    );
    assert!(type_class("volatile int *").1.volatile);
    assert_eq!(type_class("unsigned long").0, TypeClass::Scalar);
    assert_eq!(type_class("enum mode").0, TypeClass::Scalar);
    assert_eq!(type_class("struct S").0, TypeClass::Other);
    assert_eq!(type_class("void").0, TypeClass::Void);
}

#[test]
fn callee_arity_reads_the_last_parameter_list() {
    assert_eq!(callee_arity("int (*)(const char *, ...)"), Some((1, true)));
    assert_eq!(
        callee_arity("int (*)(FILE *, const char *, ...)"),
        Some((2, true))
    );
    assert_eq!(callee_arity("void (*)(void)"), Some((0, false)));
    assert_eq!(
        callee_arity("int (*)(int (*)(void), int)"),
        Some((2, false))
    );
}

// ---------------------------------------------------------------------------
// The binding to `proof/QCore/CastSemantics.lean`.
// ---------------------------------------------------------------------------

const LEAN: &str = include_str!("../../../../proof/QCore/CastSemantics.lean");

/// The rows of the Lean list starting `start` and ending at `\n]`.
fn lean_rows(start: &str) -> Vec<String> {
    let body = LEAN.split(start).nth(1).expect(start);
    body[..body.find("\n]").expect("list end")]
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('('))
        .map(|line| line.trim_end_matches(',').to_owned())
        .collect()
}

/// A kebab-case class name as its Lean constructor (`byte-view` → `byteView`).
fn camel(name: &str) -> String {
    let mut out = String::new();
    let mut upper = false;
    for character in name.chars() {
        if character == '-' {
            upper = true;
        } else if upper {
            out.extend(character.to_uppercase());
            upper = false;
        } else {
            out.push(character);
        }
    }
    out
}

fn lean_identity(identity: Identity) -> &'static str {
    match identity {
        Identity::Data => ".data",
        Identity::Preserved => ".preserved",
        Identity::Lost => ".lost",
        Identity::Absent => ".absent",
        Identity::NoValue => ".noValue",
        Identity::Refused => ".refused",
    }
}

fn lean_access(access: Access) -> &'static str {
    match access {
        Access::NotPointer => ".notPointer",
        Access::Same => ".same",
        Access::Character => ".character",
        Access::EffectiveType => ".effectiveType",
        Access::Punned => ".punned",
        Access::Untracked => ".untracked",
        Access::Null => ".null",
        Access::Callee => ".callee",
        Access::Refused => ".refused",
    }
}

/// `conversionTable` = `CONVERSION_TABLE`, row for row.
#[test]
fn lean_conversion_table_matches_the_rust_table() {
    let rows =
        lean_rows("def conversionTable : List (ConvClass × List String × Identity × Access) := [");
    assert_eq!(rows.len(), CONVERSION_TABLE.len());
    for (row, rust) in rows.iter().zip(CONVERSION_TABLE) {
        let kinds = rust
            .kinds
            .iter()
            .map(|kind| format!("\"{kind}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let expected = format!(
            "(.{}, [{kinds}], {}, {})",
            camel(rust.class.name()),
            lean_identity(rust.identity),
            lean_access(rust.access)
        );
        assert_eq!(row, &expected);
    }
}

fn lean_pointee(pointee: &Pointee) -> String {
    match pointee {
        Pointee::Void => ".void".into(),
        Pointee::Character(spelling) => format!("(.character \"{spelling}\")"),
        Pointee::Function => ".function".into(),
        Pointee::Object(key) => format!("(.object \"{key}\")"),
    }
}

fn lean_type(text: &str) -> String {
    let (class, qualifiers) = type_class(text);
    match class {
        TypeClass::Scalar => ".scalar".into(),
        TypeClass::Void => ".void".into(),
        TypeClass::Other => ".other".into(),
        TypeClass::Pointer(pointee) => {
            format!(".pointer {} {}", lean_pointee(&pointee), qualifiers.atomic)
        }
    }
}

/// `classifyVectors` = `CLASSIFY_VECTORS`, each type text rendered as the Rust
/// reader classifies it; Lean proves its `classify` agrees on every row
/// (`classify_vectors_hold`), so the two classifiers agree on the vectors.
#[test]
fn lean_classify_vectors_are_the_rust_vectors() {
    let rows =
        lean_rows("def classifyVectors : List (String × TypeClass × TypeClass × ConvClass) := [");
    assert_eq!(rows.len(), CLASSIFY_VECTORS.len());
    for (row, (kind, from, to, class)) in rows.iter().zip(CLASSIFY_VECTORS) {
        let expected = format!(
            "(\"{kind}\", {}, {}, .{})",
            lean_type(from),
            lean_type(to),
            camel(class)
        );
        assert_eq!(row, &expected);
    }
}
