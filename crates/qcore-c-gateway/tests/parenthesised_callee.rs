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

//! Card W45-PARENTHESISED-CALLEE — a statement `(f)(args);` was silently
//! skipped (`docs/reports/w45-parenthesised-callee/README.md`, gateway
//! constraint 10c).
//!
//! In C, `(f)(x)` calls `f`: the parentheses only suppress a function-like
//! macro. Every lexical call reader of the gateway keys on the pair
//! `NAME (`, so before this card a parenthesised callee was invisible to all
//! of them, and a statement with no visible call, assignment or increment fell
//! through to the classifier's final `NoOp`. The card normalises
//! `(NAME)(…)` to `NAME(…)` in every function body before any reader runs,
//! and refuses every other callee expression that is not a name.
//!
//! Every fixture runs `analyze_legacy_c_translation_unit` over the synthetic
//! `q_*`, `r_*` and `k_*` surfaces. `Safe` here is `Safe` over synthetic
//! trusted test assumptions and promotes nothing; a provisional `Unsafe` is a
//! projection allegation, never an upstream defect claim.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, LegacyDiagnosticKind, LegacyFunctionSummary, LegacyInstruction,
    LegacyProgramSummary, ResourceContracts, analyze_legacy_c_translation_unit,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

/// The refusal of a callee expression that is not a name.
const REFUSAL: &str =
    "call through a callee expression that is not a name is outside the resource projection";
/// The refusal of a `for` clause whose grouping tokens the stripped spelling lost.
const FOR_CLAUSE: &str =
    "for-loop clause with a call or grouping is not classified from its stripped spelling";

const DECLARATIONS: &str = "\
void *q_alloc(void); void q_free(void *); void q_transfer(void *); \
int r_release_code(void *p); int r_consume_code(void *p); int r_borrow_code(void *p); \
void *r_wrap(void *p); int k_open(const char *path); int k_close(int fd); \
void zz(void *); typedef void (*rel_t)(void *); rel_t q_pick(void); int c;";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        (
            "q_base.qcore".to_owned(),
            include_str!("fixtures/contracts/q_base.qv").to_owned(),
        ),
        (
            "r_return_call.qcore".to_owned(),
            include_str!("fixtures/contracts/r_return_call.qv").to_owned(),
        ),
        (
            "descriptor_generic.qcore".to_owned(),
            include_str!("fixtures/contracts/descriptor_generic.qv").to_owned(),
        ),
    ]))
    .expect("the fixture contracts load")
}

fn unit(source: &str) -> LegacyProgramSummary {
    analyze_legacy_c_translation_unit(
        &format!("{DECLARATIONS}\n{source}\n"),
        "subject",
        &contracts(),
    )
    .expect("the fixture tokenizes")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent CG22 row: {reason}");
            false
        }
    }
}

fn subject(summary: &LegacyProgramSummary) -> &LegacyFunctionSummary {
    &summary.functions["subject"]
}

fn body(text: &str) -> LegacyProgramSummary {
    unit(&format!("void subject(void) {{ {text} }}"))
}

fn unsupported(summary: &LegacyFunctionSummary) -> Vec<String> {
    summary
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description } => Some(description.clone()),
            _ => None,
        })
        .collect()
}

fn count(summary: &LegacyFunctionSummary, kind: fn(&LegacyDiagnosticKind) -> bool) -> usize {
    summary
        .diagnostics
        .iter()
        .filter(|diagnostic| kind(&diagnostic.kind))
        .count()
}

fn leaks(summary: &LegacyFunctionSummary) -> usize {
    count(summary, |kind| {
        matches!(kind, LegacyDiagnosticKind::Leak { .. })
    })
}

fn double_releases(summary: &LegacyFunctionSummary) -> usize {
    count(summary, |kind| {
        matches!(kind, LegacyDiagnosticKind::DoubleRelease { .. })
    })
}

fn calls(summary: &LegacyFunctionSummary, callee: &str) -> usize {
    summary
        .cfg
        .nodes
        .iter()
        .filter(|node| {
            matches!(&node.instruction, LegacyInstruction::ContractCall { function, .. } if function == callee)
        })
        .count()
}

/// `(f)(a)` and `f(a)` have the same effect wherever the card normalises:
/// the whole program summary — every function's status, diagnostics, CFG and
/// exit residuals — is identical.
fn assert_same(parenthesised: &str, plain: &str) -> LegacyProgramSummary {
    assert_ne!(parenthesised, plain, "the two spellings differ");
    let normalised = unit(parenthesised);
    let reference = unit(plain);
    assert_eq!(normalised, reference, "\n{parenthesised}\n{plain}");
    normalised
}

// ---------------------------------------------------------------------------
// 1. The measured counterexamples, one per row of the brief's table.
// ---------------------------------------------------------------------------

/// Row 1: the release was skipped, so the acquisition was reported leaked.
#[test]
fn row_1_a_parenthesised_release_is_the_release() {
    let run = body("void *p = q_alloc(); if (p == 0) return; (q_free)(p);");
    let result = subject(&run);
    assert_eq!(result.status, AnalysisStatus::Safe, "{result:#?}");
    assert_eq!(leaks(result), 0);
    assert_eq!(calls(result, "q_free"), 1);
}

/// Row 2: the second release was skipped, so a double release was `Safe`.
#[test]
fn row_2_a_parenthesised_second_release_is_a_double_release() {
    let run = body("void *p = q_alloc(); if (p == 0) return; q_free(p); (q_free)(p);");
    let result = subject(&run);
    assert_eq!(result.status, AnalysisStatus::Unsafe, "{result:#?}");
    assert_eq!(double_releases(result), 1);
    assert_eq!(calls(result, "q_free"), 2);
}

/// Row 3: an uncontracted call was skipped, so the body was `Safe`.
#[test]
fn row_3_a_parenthesised_uncontracted_call_is_refused() {
    let run = body("void *p = q_alloc(); if (p == 0) return; (zz)(p); q_free(p);");
    let result = subject(&run);
    assert_eq!(result.status, AnalysisStatus::Unknown, "{result:#?}");
    assert!(
        unsupported(result).contains(&"call to uncontracted function `zz`".to_owned()),
        "{:?}",
        unsupported(result)
    );
}

/// Row 4: a call through a dereferenced pointer was already refused, by the
/// lexical access gate; the card leaves that refusal byte for byte.
#[test]
fn row_4_a_dereferenced_callee_keeps_its_refusal() {
    let run = unit(
        "void subject(void (*fp)(void *)) { void *p = q_alloc(); if (p == 0) return; \
         (*fp)(p); q_free(p); }",
    );
    let result = subject(&run);
    assert_eq!(result.status, AnalysisStatus::Unknown, "{result:#?}");
    assert_eq!(
        unsupported(result),
        ["indirect memory access is outside the resource projection"]
    );
}

// ---------------------------------------------------------------------------
// 2. Normalised positions: `(f)(a)` is decided exactly as `f(a)`.
// ---------------------------------------------------------------------------

#[test]
fn every_row_is_decided_as_its_unparenthesised_spelling() {
    for (parenthesised, plain) in [
        ("(q_free)(p);", "q_free(p);"),
        ("q_free(p); (q_free)(p);", "q_free(p); q_free(p);"),
        ("(zz)(p); q_free(p);", "zz(p); q_free(p);"),
    ] {
        let prefix = "void subject(void) { void *p = q_alloc(); if (p == 0) return; ";
        assert_same(
            &format!("{prefix}{parenthesised} }}"),
            &format!("{prefix}{plain} }}"),
        );
    }
}

#[test]
fn nested_parentheses_and_a_cast_to_void_are_normalised() {
    let prefix = "void subject(void) { void *p = q_alloc(); if (p == 0) return; ";
    for parenthesised in ["((q_free))(p);", "(((q_free)))(p);", "(void)(q_free)(p);"] {
        let plain = if parenthesised.starts_with("(void)") {
            "(void) q_free(p);"
        } else {
            "q_free(p);"
        };
        let run = assert_same(
            &format!("{prefix}{parenthesised} }}"),
            &format!("{prefix}{plain} }}"),
        );
        if parenthesised.starts_with("(void)") {
            // Flipped by CG05-ELABORATOR-CALL-FOOTPRINTS (rule 11n): the
            // plain spelling's refusal (`resource call with a formal embedded
            // in an unmodeled expression`) is answered by the elaborator,
            // whose call event is applied through the one application point,
            // and both spellings are decided alike: the release is applied.
            assert!(unsupported(subject(&run)).is_empty(), "{parenthesised}");
            assert_eq!(
                subject(&run).status,
                AnalysisStatus::Safe,
                "{parenthesised}"
            );
        } else {
            assert_eq!(
                subject(&run).status,
                AnalysisStatus::Safe,
                "{parenthesised}"
            );
        }
    }
}

#[test]
fn an_initialiser_and_an_assignment_right_hand_side_are_normalised() {
    let run = assert_same(
        "void subject(void) { void *p = (q_alloc)(); if (p == 0) return; q_free(p); }",
        "void subject(void) { void *p = q_alloc(); if (p == 0) return; q_free(p); }",
    );
    assert_eq!(subject(&run).status, AnalysisStatus::Safe);
    let run = assert_same(
        "void subject(void) { void *p; p = (q_alloc)(); if (p == 0) return; }",
        "void subject(void) { void *p; p = q_alloc(); if (p == 0) return; }",
    );
    // The acquisition is seen, and so is its leak.
    assert_eq!(subject(&run).status, AnalysisStatus::Unsafe);
    assert_eq!(leaks(subject(&run)), 1);
}

/// Gateway constraint 5h: a call in a controlling expression is applied
/// exactly where C evaluates it, parenthesised or not.
#[test]
fn a_condition_is_normalised() {
    for (parenthesised, plain) in [
        (
            "if ((r_release_code)(p)) return;",
            "if (r_release_code(p)) return;",
        ),
        (
            "while ((r_borrow_code)(p)) { } q_free(p);",
            "while (r_borrow_code(p)) { } q_free(p);",
        ),
        (
            "if (c && (r_release_code)(p)) return; q_free(p);",
            "if (c && r_release_code(p)) return; q_free(p);",
        ),
        (
            "switch ((r_borrow_code)(p)) { default: break; } q_free(p);",
            "switch (r_borrow_code(p)) { default: break; } q_free(p);",
        ),
        (
            "do { } while ((r_borrow_code)(p)); q_free(p);",
            "do { } while (r_borrow_code(p)); q_free(p);",
        ),
    ] {
        let prefix = "void subject(void) { void *p = q_alloc(); if (p == 0) return; ";
        let run = assert_same(
            &format!("{prefix}{parenthesised} }}"),
            &format!("{prefix}{plain} }}"),
        );
        assert!(
            calls(subject(&run), "r_release_code") + calls(subject(&run), "r_borrow_code") > 0,
            "the condition's call is modeled: {parenthesised}"
        );
    }
}

/// Gateway constraint 5j and the W4 return-call route.
#[test]
fn a_return_is_normalised() {
    let run = assert_same(
        "int subject(void) { void *p = q_alloc(); if (p == 0) return 0; \
         return (r_release_code)(p); }",
        "int subject(void) { void *p = q_alloc(); if (p == 0) return 0; \
         return r_release_code(p); }",
    );
    assert_eq!(subject(&run).status, AnalysisStatus::Safe);
    assert_same(
        "void *subject(void) { return (q_alloc)(); }",
        "void *subject(void) { return q_alloc(); }",
    );
}

#[test]
fn a_call_argument_is_normalised() {
    assert_same(
        "void subject(void) { void *p = q_alloc(); if (p == 0) return; \
         (void) r_borrow_code((r_wrap)(p)); q_free(p); }",
        "void subject(void) { void *p = q_alloc(); if (p == 0) return; \
         (void) r_borrow_code(r_wrap(p)); q_free(p); }",
    );
    assert_same(
        "void subject(void) { void *p = q_alloc(); if (p == 0) return; \
         q_free(((p))); }",
        "void subject(void) { void *p = q_alloc(); if (p == 0) return; \
         q_free(((p))); }"
            .replace("(((p)))", "((p))")
            .as_str(),
    );
}

/// A descriptor release: the tracked descriptor is released by `(k_close)`.
#[test]
fn a_descriptor_release_is_normalised() {
    let run = assert_same(
        "void subject(void) { int fd = k_open(\"x\"); if (fd < 0) return; (k_close)(fd); }",
        "void subject(void) { int fd = k_open(\"x\"); if (fd < 0) return; k_close(fd); }",
    );
    assert_eq!(calls(subject(&run), "k_close"), 1);
}

/// Gateway constraint 5k: a member release through a formal, then the formal.
#[test]
fn a_member_release_is_normalised() {
    let records = "struct n { void *buf; };";
    let run = assert_same(
        &format!(
            "{records} void subject(struct n *b) {{ if (b == 0) return; \
             (q_free)(b->buf); (q_free)(b); }}"
        ),
        &format!(
            "{records} void subject(struct n *b) {{ if (b == 0) return; \
             q_free(b->buf); q_free(b); }}"
        ),
    );
    assert_eq!(
        calls(subject(&run), "q_free"),
        calls(
            subject(&unit(&format!(
                "{records} void subject(struct n *b) {{ if (b == 0) return; \
             q_free(b->buf); q_free(b); }}"
            ))),
            "q_free"
        )
    );
}

/// W4: a helper whose only statement is a parenthesised release validates
/// the same summary as the plain helper, and its caller is decided by it.
#[test]
fn a_summary_is_derived_through_a_parenthesised_call() {
    let run = assert_same(
        "static void helper(void *p) { (q_free)(p); } \
         void subject(void) { void *p = q_alloc(); if (p == 0) return; helper(p); }",
        "static void helper(void *p) { q_free(p); } \
         void subject(void) { void *p = q_alloc(); if (p == 0) return; helper(p); }",
    );
    assert_eq!(subject(&run).status, AnalysisStatus::Safe);
    let run = assert_same(
        "static void helper(void *p) { q_free(p); } \
         void subject(void) { void *p = q_alloc(); if (p == 0) return; (helper)(p); }",
        "static void helper(void *p) { q_free(p); } \
         void subject(void) { void *p = q_alloc(); if (p == 0) return; helper(p); }",
    );
    assert_eq!(subject(&run).status, AnalysisStatus::Safe);
}

/// The contract-withdrawal rewrites (LOADER-ABI-MODES) key on `NAME (`, so
/// the normalisation runs before them: a parenthesised call of a withdrawn
/// declaration is withdrawn too.
#[test]
fn a_withdrawn_declaration_is_withdrawn_at_a_parenthesised_call() {
    // The unit's own `q_free` body releases nothing, contradicting the
    // declared `Released`.
    let run = assert_same(
        "void q_free(void *p) { } \
         void subject(void) { void *p = q_alloc(); if (p == 0) return; (q_free)(p); }",
        "void q_free(void *p) { } \
         void subject(void) { void *p = q_alloc(); if (p == 0) return; q_free(p); }",
    );
    assert_eq!(subject(&run).status, AnalysisStatus::Unknown);
}

// ---------------------------------------------------------------------------
// 3. The class: every other callee expression that is not a name refuses.
// ---------------------------------------------------------------------------

#[test]
fn every_other_callee_expression_refuses() {
    // A conditional of designators and `&designator` are closed candidate
    // sets under rule 11p (CG22) when Clang reads the unit: see
    // `a_closed_callee_expression_is_its_candidate_calls`.
    for statement in [
        // A cast callee.
        "((rel_t) q_free)(p);",
        // A comma callee.
        "(c, q_free)(p);",
        // A call through a call's result, after a modeled call.
        "(void) r_borrow_code(p), q_pick()(p);",
    ] {
        let run = body(&format!(
            "void *p = q_alloc(); if (p == 0) return; {statement} q_free(p);"
        ));
        let result = subject(&run);
        assert_eq!(
            result.status,
            AnalysisStatus::Unknown,
            "{statement}: {result:#?}"
        );
        assert!(!unsupported(result).is_empty(), "{statement}");
    }
}

#[test]
fn a_callee_expression_refuses_with_its_own_reason_where_nothing_else_did() {
    let closed = clang_available();
    for statement in [
        "(c ? q_free : zz)(p);",
        "((rel_t) q_free)(p);",
        "(&q_free)(p);",
    ] {
        if closed && !statement.contains("rel_t") {
            continue;
        }
        let run = body(&format!(
            "void *p = q_alloc(); if (p == 0) return; {statement} q_free(p);"
        ));
        assert_eq!(unsupported(subject(&run)), [REFUSAL], "{statement}");
    }
}

/// Card CG22-INDIRECT-CALLS-AND-CALLBACKS (rule 11p) flipped two rows of
/// this card's class: where Clang reads the unit, a conditional of
/// designators with an effect-free condition and `&designator` close their
/// candidate set, and the call is rewritten to the calls C may make (a
/// branch arm per candidate; a singleton to its direct call). Each is
/// exactly its explicit spelling, so the double release on the `q_free`
/// path is found; without Clang the set is unread and the older refusal
/// stands.
#[test]
fn a_closed_callee_expression_is_its_candidate_calls() {
    for (statement, explicit) in [
        ("(c ? q_free : zz)(p);", "if (c) q_free(p); else zz(p);"),
        ("(&q_free)(p);", "q_free(p);"),
    ] {
        let run = body(&format!(
            "void *p = q_alloc(); if (p == 0) return; {statement} q_free(p);"
        ));
        let result = subject(&run);
        if !clang_available() {
            assert_eq!(result.status, AnalysisStatus::Unknown, "{statement}");
            assert_eq!(unsupported(result), [REFUSAL], "{statement}");
            continue;
        }
        let spelled = body(&format!(
            "void *p = q_alloc(); if (p == 0) return; {explicit} q_free(p);"
        ));
        let spelled = subject(&spelled);
        assert_eq!(
            result.status,
            AnalysisStatus::Unsafe,
            "{statement}: {result:#?}"
        );
        assert_eq!(result.status, spelled.status, "{statement}");
        assert_eq!(
            double_releases(result),
            double_releases(spelled),
            "{statement}"
        );
        assert_eq!(unsupported(result), unsupported(spelled), "{statement}");
        assert!(
            !unsupported(result).iter().any(|r| r == REFUSAL),
            "{statement}"
        );
    }
}

#[test]
fn a_callee_expression_refuses_in_every_expression_position() {
    for text in [
        "if ((c ? q_free : zz)(p)) return; q_free(p);",
        "while ((&zz)(p)) { } q_free(p);",
        "switch ((&zz)(p)) { default: break; } q_free(p);",
        "do { } while ((&zz)(p)); q_free(p);",
        "for (; (&zz)(p); ) { } q_free(p);",
        "int v = (&zz)(p); q_free(p);",
    ] {
        let run = body(&format!("void *p = q_alloc(); if (p == 0) return; {text}"));
        assert_eq!(subject(&run).status, AnalysisStatus::Unknown, "{text}");
    }
    let run = unit(
        "int subject(void) { void *p = q_alloc(); if (p == 0) return 0; q_free(p); \
         return (&zz)(0); }",
    );
    assert_eq!(subject(&run).status, AnalysisStatus::Unknown);
}

/// A `for` header's init and increment clauses reach the classifier with
/// their grouping tokens stripped (`W4-EXPR-POS-FOR-CLAUSES`), so
/// `q_free(p)` there read as the two names `q_free p` and fell through to the
/// same final `NoOp`. Such a clause now refuses unless the stripped spelling
/// was already refused.
#[test]
fn a_for_clause_call_is_never_a_no_op() {
    // The init clause always runs: before the card its release vanished and
    // the acquisition was reported leaked.
    let run = body("void *p = q_alloc(); if (p == 0) return; for (q_free(p); c; ) { }");
    let result = subject(&run);
    assert_eq!(result.status, AnalysisStatus::Unknown, "{result:#?}");
    assert_eq!(unsupported(result), [FOR_CLAUSE]);
    // The increment clause is refused too (the zero-iteration path keeps its
    // own leak, which the refusal does not reach).
    let run = body("void *p = q_alloc(); int i; for (i = 0; i < 1; q_free(p)) { i = 1; }");
    assert!(unsupported(subject(&run)).contains(&FOR_CLAUSE.to_owned()));
    assert_ne!(subject(&run).status, AnalysisStatus::Safe);
    let run = body("void *p; int i; for (p = q_alloc(); p != 0; ) { q_free(p); p = 0; }");
    assert_eq!(subject(&run).status, AnalysisStatus::Unknown);
    // Clauses without grouping keep their exact classification, and so do
    // clauses whose only groups wrap one name (`TAILQ_FOREACH_SAFE`'s
    // `(elm) = (tmpelm)`): stripping those changes no meaning.
    let run = body("int i; for (i = 0; i < 3; i++) { }");
    assert_eq!(subject(&run).status, AnalysisStatus::Safe);
    let run = body("int i; int j = 1; for ((i) = 0; i < 3; (i) = (j)) { }");
    assert_eq!(
        subject(&run).status,
        AnalysisStatus::Safe,
        "{:?}",
        unsupported(subject(&run))
    );
}

// ---------------------------------------------------------------------------
// 4. What is not a call stays what it was.
// ---------------------------------------------------------------------------

#[test]
fn a_cast_of_a_parenthesised_operand_is_not_a_call() {
    let run = body("int n = 1; (void)(n); (long)(n);");
    assert_eq!(
        subject(&run).status,
        AnalysisStatus::Safe,
        "{:#?}",
        subject(&run)
    );
    // A typedef name is a type, so `(T)(x)` is a cast.
    let run = unit("typedef long word_t; void subject(void) { int n = 1; (word_t)(n); }");
    assert_eq!(
        subject(&run).status,
        AnalysisStatus::Safe,
        "{:#?}",
        subject(&run)
    );
    // The scalar assignment with a pure cast now passes the statement reader;
    // the cast still names no call and carries no resource effect.
    let run =
        unit("typedef long word_t; void subject(void) { int n = 1; word_t m; m = (word_t)(n); }");
    assert_eq!(subject(&run).status, AnalysisStatus::Safe);
    assert!(unsupported(subject(&run)).is_empty());
}

/// A typedef name a body also declares as an object is not trusted as a type:
/// `(T)(x)` there may be a call through the object, and it is read as one.
#[test]
fn a_shadowed_typedef_name_is_not_a_cast() {
    let run = unit(
        "typedef long word_t; void subject(rel_t word_t) { void *p = q_alloc(); \
         if (p == 0) return; (word_t)(p); q_free(p); }",
    );
    assert_eq!(subject(&run).status, AnalysisStatus::Unknown);
    assert!(
        unsupported(subject(&run)).contains(&"call to uncontracted function `word_t`".to_owned()),
        "{:?}",
        unsupported(subject(&run))
    );
}

/// A parenthesised name that is a call's *argument* is not a callee:
/// `g(f)(x)` calls `g(f)`'s result, and is refused as such.
#[test]
fn an_argument_list_is_never_normalised() {
    let run = body("void *p = q_alloc(); if (p == 0) return; q_pick(q_free)(p); q_free(p);");
    let result = subject(&run);
    assert_eq!(result.status, AnalysisStatus::Unknown);
    assert_eq!(calls(result, "q_free"), 1);
}

/// A function-pointer declarator is a declaration, not a call.
#[test]
fn a_function_pointer_declarator_is_not_a_call() {
    let run = body("void *(*make)(void); void *p = q_alloc(); if (p == 0) return; q_free(p);");
    let result = subject(&run);
    assert_eq!(result.status, AnalysisStatus::Safe, "{result:#?}");
}
