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

//! Public provider-free body comparison, with no library witness constructor.
use qcore_c_gateway::legacy_c::{C11TranslationUnit, CCompilerDialect};
use qcore_c_gateway::typed_source_match::{
    SourceFieldProfile, TypedBodyComparison, compare_typed_body,
};

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "invalid explicit Clang: {reason}"
            );
            eprintln!("Clang comparison test unavailable: {reason}");
            false
        }
    }
}
fn compare(
    source: &str,
    reference: &str,
    fields: SourceFieldProfile<'_>,
) -> Result<TypedBodyComparison, String> {
    compare_typed_body(
        &C11TranslationUnit {
            source,
            dialect: CCompilerDialect::Gnu11,
            ..Default::default()
        },
        "unverified-test-label",
        "sample",
        reference,
        fields,
    )
}
const DECLS: &str = "struct packet { int value; }; void deliver(int); void discard(int);";
const BODY: &str = "void sample(struct packet *p) { int saved = p->value; deliver(saved); }";
const REFERENCE: &str =
    "void qcore_source_reference(struct packet *item) { int value = item->value; deliver(value); }";
const FIELDS: SourceFieldProfile<'_> = &[("packet", &[("value", "int")])];

#[test]
fn public_comparison_preserves_types_calls_and_local_binding_identity() {
    if !clang_available() {
        return;
    }
    let source = format!("{DECLS}{BODY}");
    let result = compare(&source, REFERENCE, FIELDS).unwrap();
    assert_eq!(result.function(), "sample");
    assert_eq!(result.source_revision(), "unverified-test-label");
    let json = serde_json::to_value(result).unwrap();
    assert_eq!(json["reference"], REFERENCE);
    assert_eq!(json.as_object().unwrap().len(), 4);
    for changed in [
        source.replace("deliver(saved)", "discard(saved)"),
        source.replace("deliver(saved)", "deliver(1)"),
        source.replace("p->value", "p->value + 1"),
        source.replace("deliver(saved);", "deliver(saved); deliver(saved);"),
    ] {
        assert!(compare(&changed, REFERENCE, FIELDS).is_err(), "{changed}");
    }
}

#[test]
fn field_representation_is_checked_even_when_both_bodies_share_declarations() {
    if !clang_available() {
        return;
    }
    for field in [
        "unsigned value",
        "volatile int value",
        "_Atomic int value",
        "int value:3",
    ] {
        let source = format!("{}{BODY}", DECLS.replace("int value", field));
        assert!(compare(&source, REFERENCE, FIELDS).is_err(), "{field}");
    }
    assert!(
        compare(
            &format!("{DECLS}{BODY}"),
            REFERENCE,
            &[("missing", &[("value", "int")])]
        )
        .is_err()
    );
}

#[test]
fn input_macro_cannot_rewrite_the_supplied_reference_call() {
    if !clang_available() {
        return;
    }
    let source = format!("{DECLS}\n#define deliver(value) discard(value)\n{BODY}");
    assert!(compare(&source, REFERENCE, FIELDS).is_err());
    let source = format!(
        "{DECLS}\n#define identity(value) (value)\n{}",
        BODY.replace("p->value", "identity(p->value)")
    );
    // Parentheses remain AST shape: deliberately conservative, never optimized away.
    assert!(compare(&source, REFERENCE, FIELDS).is_err());
}

#[test]
fn omitted_offsetof_operands_and_attributes_are_not_erased() {
    if !clang_available() {
        return;
    }
    let decl = "struct envelope { int first; int second; };";
    let body = "unsigned long sample(void) { return __builtin_offsetof(struct envelope, first); }";
    let reference = body.replace("sample", "qcore_source_reference");
    compare(&format!("{decl}{body}"), &reference, &[]).unwrap();
    assert!(
        compare(
            &format!("{decl}{}", body.replace(", first)", ", second)")),
            &reference,
            &[]
        )
        .is_err()
    );
    assert!(
        compare(
            &format!("{DECLS}__attribute__((noinline)) {BODY}"),
            REFERENCE,
            FIELDS
        )
        .is_err()
    );
}

#[test]
fn arbitrary_matching_reference_is_not_a_safety_or_revision_claim() {
    if !clang_available() {
        return;
    }
    // Dereferencing a null pointer has matching ASTs but supplies no safety proof.
    let source = "int sample(void) { int *p = (int *)0; return *p; }";
    let reference = source.replace("sample", "qcore_source_reference");
    let report = compare(source, &reference, &[]).unwrap();
    assert_eq!(report.source_revision(), "unverified-test-label");
    assert!(compare(source, &reference.replace("return *p", "return 0"), &[]).is_err());
}
