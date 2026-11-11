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

//! Card CG04-CONDITION-ACQUISITION (rule 10x): the route-G adapter's lexical
//! readers, and one test per refusal arm of the condition desugaring
//! (`CONDITION-DESUGAR-REFUSAL-ARMS-UNPINNED`), each pinned to the variant the
//! desugaring answered.

use super::{CONSULTATIONS, holds_real_call, leaf_features, type_operand_spans};
use crate::legacy_c::{
    C11TranslationUnit, LegacyDiagnosticKind, Token, UnrestrictedLegacyResult,
    analyze_unrestricted_c11,
};

fn texts(text: &str) -> Vec<&str> {
    text.split_whitespace().collect()
}

#[test]
fn a_type_operand_is_set_apart() {
    assert_eq!(
        type_operand_spans(&texts("n < sizeof ( buf ) + sizeof ( int )")),
        Some(vec![(2, 5), (7, 10)])
    );
    assert_eq!(type_operand_spans(&texts("n < sizeof * p")), None);
    let features = leaf_features(&texts("f ( p ) == sizeof ( int )")).unwrap();
    assert!(features.call && !features.call_in_type_operand);
    let features = leaf_features(&texts("n == sizeof ( f ( p ) )")).unwrap();
    assert!(!features.call && features.call_in_type_operand);
    let features = leaf_features(&texts("f ( a & & b ) == i + +")).unwrap();
    assert!(features.call && features.increment && !features.connective);
    let features = leaf_features(&texts("( a & & b ) == f ( x )")).unwrap();
    assert!(features.call && features.connective);
    let tokens = |text: &str| {
        text.split_whitespace()
            .map(|text| Token {
                text: text.to_owned(),
                line: 1,
            })
            .collect::<Vec<_>>()
    };
    assert!(!holds_real_call(&tokens("n < sizeof ( int )")));
    assert!(holds_real_call(&tokens("f ( n ) < sizeof ( int )")));
    assert!(holds_real_call(&tokens("n < sizeof * f ( p )")));
}

#[test]
fn cycle_ranges_cover_loops_and_backward_gotos() {
    let tokens = |text: &str| {
        text.split_whitespace()
            .map(|text| Token {
                text: text.to_owned(),
                line: 1,
            })
            .collect::<Vec<_>>()
    };
    let body = tokens("{ a ; while ( x ) { b ; } c ; L : d ; goto L ; e ; goto M ; f ; M : g ; }");
    let ranges = super::CycleRanges::read(&body, 0);
    let at = |text: &str| body.iter().position(|token| token.text == text).unwrap();
    assert!(!ranges.contains(at("a")));
    assert!(ranges.contains(at("b")));
    assert!(!ranges.contains(at("c")));
    assert!(ranges.contains(at("d")));
    assert!(!ranges.contains(at("e")));
    assert!(!ranges.contains(at("f")));
    assert!(!ranges.contains(at("g")));
    // A label the reader does not find makes the whole body a cycle.
    let body = tokens("{ a ; goto Z ; }");
    assert!(super::CycleRanges::read(&body, 0).contains(1));
    // A braceless loop body is bounded by its enclosing block.
    let body = tokens("{ { for ( ; ; ) b ; c ; } d ; }");
    let ranges = super::CycleRanges::read(&body, 0);
    let at = |text: &str| body.iter().position(|token| token.text == text).unwrap();
    assert!(ranges.contains(at("b")));
    assert!(ranges.contains(at("c")));
    assert!(!ranges.contains(at("d")));
}

const DECLARATIONS: &str = "\
void *q_alloc(void); void q_free(void *p);
int r_borrow_code(void *p);
struct F { int (*cb)(void *); int a; };
";

fn contracts() -> crate::legacy_c::ResourceContracts {
    let sources = [
        (
            "q_base.qv".to_owned(),
            include_str!("../fixtures/contracts/q_base.qv").to_owned(),
        ),
        (
            "r_return_call.qv".to_owned(),
            include_str!("../fixtures/contracts/r_return_call.qv").to_owned(),
        ),
    ]
    .into_iter()
    .collect();
    crate::contract_loader::load_resource_contract_sources(&sources).unwrap()
}

/// The desugaring's answers for `subject`'s controlling expressions, and
/// whether the function kept the position's refusal.
fn consultations(body: &str) -> (Vec<String>, bool) {
    CONSULTATIONS.with(|consultations| consultations.borrow_mut().clear());
    let source = format!("{DECLARATIONS}void subject(struct F *f, int flag, int n) {{ {body} }}\n");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "subject",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("valid fixture rejected by Clang: {error:?}\n{source}"));
    let summary = match &outcome {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown { partial, .. } => {
            partial.as_ref().expect("a partial summary")
        }
    };
    let refused = summary.functions["subject"].diagnostics.iter().any(|d| {
        matches!(&d.kind, LegacyDiagnosticKind::Unsupported { description }
            if description.starts_with("unmodeled call, aggregate or indirect access in"))
    });
    let answers = CONSULTATIONS.with(|consultations| consultations.borrow().clone());
    (answers, refused)
}

/// `CONDITION-DESUGAR-REFUSAL-ARMS-UNPINNED`: one fixture per variant the
/// stability review found untested, each answered by exactly that arm, and
/// each keeping the position's refusal.
#[test]
fn every_refusal_arm_is_pinned() {
    if !crate::test_support::clang_available() {
        return;
    }
    for (body, arm) in [
        (
            "void *p = q_alloc(); if (flag && f->cb(p)) { } q_free(p);",
            "IndirectCallee",
        ),
        (
            "void *p = q_alloc(); if (flag && ({ r_borrow_code(p); })) { } q_free(p);",
            "Braces",
        ),
        (
            "void *p = q_alloc(); if ((n = r_borrow_code(p)) != 0) { } q_free(p);",
            "AssignmentDestination(\"n\")",
        ),
        (
            "void *p = q_alloc(); int a[2] = {0}; if (r_borrow_code(p) == a[n]) { } q_free(p);",
            "ResidualRefused",
        ),
        // Card CG08-CASTS-AND-TYPE-QUERIES (rule 11b) admits the unevaluated
        // `sizeof(buf)` this arm's fixture used; an evaluated operand with a
        // store (`char[n++]`) is still refused by the elaborator.
        (
            "void *p = q_alloc(); if (r_borrow_code(p) && n >= sizeof(char[n++])) { } q_free(p);",
            "CallFreeLeafRefused",
        ),
        (
            "void *p = q_alloc(); switch (flag && r_borrow_code(p)) { default: break; } q_free(p);",
            "SwitchShape",
        ),
    ] {
        let (answers, refused) = consultations(body);
        assert!(
            answers.iter().any(|answer| answer == arm),
            "{body}: {answers:?}"
        );
        assert!(refused, "{body}: {answers:?}");
    }
}
