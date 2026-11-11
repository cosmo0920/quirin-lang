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

//! Card W47-LEXICAL-CLOSURE-NESTED-ASSIGNMENT (gateway rule 10o) — the
//! never-bound premise reads every assignment of the body, wherever it
//! stands, and the typed positions serve the join of the lexical and the AST
//! closures.
//!
//! Every fixture here is read by the lexical declaration reader (no formal it
//! cannot read), so before this card the lexical answer was served unchanged.
//! Each negative control binds its root in a form the old statement reader
//! skipped; exploratory outcomes print with `--nocapture`.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const CONTRACTS: &str = "\
module contract;\n\
extern fn c_make(n: i32) -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(n);\n\
           ensures normal resource_return_owned(nullable); };\n\
extern fn c_drop(b: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(b, nonnull);\n\
           ensures normal resource_return_none(); };\n\
extern fn c_use(v: i32) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(v);\n\
           ensures normal resource_return_none(); };\n\
extern fn c_look(p: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(p);\n\
           ensures normal resource_return_none(); };";

const DECLARATIONS: &str = "\
struct c_leaf { int weight; char *name; }; \
struct c_item { int count; struct c_leaf *leaf; }; \
struct c_box { int flag; struct c_item *item; }; \
struct c_box *c_make(int n); \
void c_drop(struct c_box *b); \
void c_use(int v); \
void c_look(struct c_item *p);";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "lexical_closure_nested_assignment.qv".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the synthetic contracts load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent nested-assignment test: {reason}");
            false
        }
    }
}

fn program(result: &UnrestrictedLegacyResult) -> Option<&LegacyProgramSummary> {
    match result {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial: None, .. } => None,
    }
}

/// The status and the complete, sorted, de-duplicated diagnostic vector of
/// `w`.
fn outcome_with(prefix: &str, body: &str) -> (&'static str, Vec<String>) {
    let source = format!("{DECLARATIONS}\n{prefix}\n{body}");
    let Ok(result) = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "w",
        &contracts(),
    ) else {
        return ("Rejected", Vec::new());
    };
    let status = match &result {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(certificate)
                .expect("a Safe result carries a certificate that rechecks");
            "Safe"
        }
        UnrestrictedLegacyResult::Unsafe(_) => "Unsafe",
        UnrestrictedLegacyResult::Unknown { .. } => "Unknown",
    };
    let Some(function) = program(&result).and_then(|summary| summary.functions.get("w")) else {
        eprintln!("outcome: {prefix} || {body} => Unprojected");
        return ("Unprojected", Vec::new());
    };
    let mut keys = function
        .diagnostics
        .iter()
        .map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::UnknownAlias { variable } => format!("unknown_alias:{variable}"),
            LegacyDiagnosticKind::Leak { .. } => "leak".to_owned(),
            LegacyDiagnosticKind::DoubleRelease { .. } => "double_release".to_owned(),
            LegacyDiagnosticKind::UseAfterRelease { .. } => "use_after_release".to_owned(),
            LegacyDiagnosticKind::Unsupported { description } => {
                format!("unsupported:{description}")
            }
            other => format!("{other:?}"),
        })
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    eprintln!("outcome: {prefix} || {body} => {status} {keys:?}");
    (status, keys)
}

fn outcome(body: &str) -> (&'static str, Vec<String>) {
    outcome_with("", body)
}

const CONDITION: &str =
    "unsupported:unmodeled call, aggregate or indirect access in branch condition";
const RETURN: &str =
    "unsupported:unmodeled call, aggregate or indirect access in return expression";

fn unknown(keys: &[&str]) -> (&'static str, Vec<String>) {
    (
        "Unknown",
        keys.iter().map(|key| (*key).to_owned()).collect(),
    )
}

/// Each form binds `b` from the tracked `a` (released afterwards) in a spot
/// the old statement reading skipped. At the base the read `b->item->count`
/// after the release was admitted through `b` as an untracked root (the
/// refused binding left `b` unknown, and the premise called it never bound);
/// now the read needs `b`'s identity and reports it. Base outcomes:
/// `docs/reports/w47-lexical-closure-nested-assignment/artifacts/controls/`.
#[test]
fn a_nested_binding_makes_the_read_through_its_root_need_an_identity() {
    if !clang_available() {
        return;
    }
    let prelude = "struct c_box *a = c_make(1); struct c_box *b; if (!a) return 0;";
    for (binding, refusal) in [
        ("if ((b = a)) c_use(1);", CONDITION),
        ("if (!(b = a)) return 0;", CONDITION),
        (
            "while ((b = a)) break;",
            "unsupported:unmodeled call, aggregate or indirect access in loop condition",
        ),
        ("int n; if ((n = 0, b = a)) c_use(n);", CONDITION),
    ] {
        let body =
            format!("int w(void) {{ {prelude} {binding} c_drop(a); return b->item->count; }}");
        assert_eq!(
            outcome(&body),
            unknown(&["unknown_alias:b", "unknown_alias:b/a", refusal]),
            "{body}"
        );
    }
    // A later declarator.
    assert_eq!(
        outcome(
            "int w(void) { struct c_box *a = c_make(1), *b = a; if (!a) return 0; c_drop(a); \
             return b->item->count; }"
        ),
        unknown(&[
            "unknown_alias:a",
            "unknown_alias:b",
            "unknown_alias:b/a",
            "unsupported:indirect memory access is outside the resource projection",
        ])
    );
    // An assignment in a `?:` arm.
    assert_eq!(
        outcome(
            "int w(int k) { struct c_box *a = c_make(1); struct c_box *b = 0; if (!a) return 0; \
             c_use(k ? ((b = a) != 0) : 0); c_drop(a); return b->item->count; }"
        ),
        unknown(&[
            "unknown_alias:b",
            "unknown_alias:b/a",
            "unsupported:unmodeled expression may mutate a formal resource identity",
        ])
    );
    // A macro-expanded assignment: a parenthesised destination.
    assert_eq!(
        outcome_with(
            "#define C_TAKE(d, s) ((d) = (s))",
            "int w(void) { struct c_box *a = c_make(1); struct c_box *b; if (!a) return 0; \
             if (!C_TAKE(b, a)) return 0; c_drop(a); return b->item->count; }"
        ),
        unknown(&["unknown_alias:b", "unknown_alias:b/a", CONDITION])
    );
    // A call actual reads the same way.
    assert_eq!(
        outcome(
            "void w(void) { struct c_box *a = c_make(1); struct c_box *b; if (!a) return; \
             if ((b = a)) c_use(1); c_drop(a); c_use(b->item->count); }"
        ),
        unknown(&["unknown_alias:b", "unknown_alias:b/a", CONDITION])
    );
}

/// A pointer read handed out through a root bound from a call in a
/// condition, a `for` clause or a macro-expanded assignment keeps the return
/// refusal (rule 5b's premise). At the base each of these returns was
/// admitted with a `NoOp` exit.
#[test]
fn a_pointer_return_through_a_nested_binding_keeps_the_return_refusal() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        outcome(
            "struct c_item *w(struct c_box *b) { if (!(b = c_make(1))) return 0; return b->item; }"
        ),
        unknown(&[CONDITION, RETURN])
    );
    assert_eq!(
        outcome(
            "struct c_item *w(struct c_box *b) { for (b = c_make(1); b; ) break; return b->item; }"
        ),
        unknown(&[RETURN, "unsupported:unsupported assignment expression"])
    );
    assert_eq!(
        outcome_with(
            "#define C_TAKE(d, s) ((d) = (s))",
            "struct c_item *w(struct c_box *b) { if (!C_TAKE(b, c_make(1))) return 0; \
             return b->item; }"
        ),
        unknown(&[CONDITION, RETURN])
    );
}

/// A compound assignment in a condition was seeded before this card and
/// still is; a parenthesised copy the flow tracks keeps its accusation.
#[test]
fn a_compound_assignment_and_a_tracked_copy_are_unchanged() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        outcome("int w(struct c_box *b) { if ((b += 1)) c_use(1); return b->item->count; }"),
        unknown(&["unknown_alias:b", CONDITION])
    );
    let (status, keys) = outcome(
        "int w(void) { struct c_box *a = c_make(1); struct c_box *b; if (!a) return 0; \
         (b) = a; c_drop(a); return b->item->count; }",
    );
    assert_eq!(status, "Unsafe");
    assert!(keys.contains(&"use_after_release".to_owned()), "{keys:?}");
}

/// Positive controls: an untouched root still certifies, and a nested
/// assignment from an admitted field read binds nothing.
#[test]
fn a_never_bound_root_still_certifies() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        outcome("int w(struct c_box *b) { return b->item->count; }"),
        ("Safe", Vec::new())
    );
    assert_eq!(
        outcome("char *w(struct c_box *b) { return b->item->leaf->name; }"),
        ("Safe", Vec::new())
    );
    // `it = b->item` in the condition keeps its own identity refusal (rule
    // 10b) and leaves `b` untouched, so the return's read needs nothing.
    assert_eq!(
        outcome(
            "int w(struct c_box *b) { struct c_item *it; if ((it = b->item)) c_use(it->count); \
             return b->item->count; }"
        ),
        unknown(&["unknown_alias:it"])
    );
}
