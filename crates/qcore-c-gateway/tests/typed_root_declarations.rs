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

//! Card W47-TYPED-ROOT-DECLARATION-READER (gateway rule 10l) — the never-bound,
//! `may_bind` and declared-local premises of the typed positions (rule 10k),
//! answered from the Clang AST where the lexical declaration reader answers
//! nothing, and only ever at least as conservatively as the lexical closure.
//!
//! The lexical reader refuses every declaration of a function whose parameter
//! list it cannot read; an array-of-function-pointers formal `int (*cb[1])(int)`
//! (a plain function-pointer formal is read since card
//! FB-LOOP-CHAIN-ACCESS-IDENTITY-BLOCKERS) is the
//! trigger these fixtures use, so each body is answered by the AST reader. The
//! negative controls were written before the change and run at the base
//! (`docs/reports/w47-typed-root-declaration-reader/`). Every test asserts the
//! status and the complete diagnostic vector of `w`. The fixtures use a
//! generic `c_*` surface: `Safe` here is `Safe` over synthetic trusted test
//! assumptions and promotes nothing.

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
extern fn c_hold(b: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(b);\n\
           ensures normal resource_return_none(); };\n\
extern fn c_use(v: i32) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(v);\n\
           ensures normal resource_return_none(); };";

const DECLARATIONS: &str = "\
struct c_leaf { int weight; char *name; }; \
struct c_item { int count; struct c_leaf *leaf; }; \
struct c_box { int flag; struct c_item *item; }; \
struct c_box *c_make(int n); \
void c_drop(struct c_box *b); \
void c_hold(struct c_box *b); \
void c_use(int v);";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "typed_root_declarations.qv".to_owned(),
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
            eprintln!("skipping Clang-dependent typed-root-declaration test: {reason}");
            false
        }
    }
}

fn analyze(prefix: &str, body: &str) -> Option<UnrestrictedLegacyResult> {
    let source = format!("{DECLARATIONS}\n{prefix}\n{body}");
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "w",
        &contracts(),
    )
    .ok()
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
/// `w`; `("Rejected", [])` when the frontend refused the unit and
/// `("Unprojected", [])` for an `Unknown` with no projection of `w` at all.
fn outcome_with(prefix: &str, body: &str) -> (&'static str, Vec<String>) {
    let Some(result) = analyze(prefix, body) else {
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
        assert_eq!(status, "Unknown");
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
    // Captured by the harness; `--nocapture` prints the fixture table the
    // report records for the base and the change.
    eprintln!("outcome: {prefix} || {body} => {status} {keys:?}");
    (status, keys)
}

fn outcome(body: &str) -> (&'static str, Vec<String>) {
    outcome_with("", body)
}

fn safe() -> (&'static str, Vec<String>) {
    ("Safe", Vec::new())
}

const RETURN: &str =
    "unsupported:unmodeled call, aggregate or indirect access in return expression";

/// A control that must never certify, and whose return keeps its refusal
/// (or whose unit the frontend rejects).
fn assert_refused(prefix: &str, body: &str) {
    let (status, keys) = outcome_with(prefix, body);
    if matches!(status, "Rejected" | "Unprojected") {
        return;
    }
    assert_ne!(status, "Safe", "{prefix} {body}: {keys:?}");
    assert!(
        keys.contains(&RETURN.to_owned()),
        "{prefix} {body}: {keys:?}"
    );
}

/// The lexical reader cannot read `int (*cb[1])(int)` and answers nothing for
/// the whole function; the AST reader answers `b` never bound and not
/// `may_bind`, so the pointer read is returned with a `NoOp` exit.
#[test]
fn an_untouched_root_certifies_when_only_the_ast_answers() {
    if !clang_available() {
        return;
    }
    for body in [
        "char *w(struct c_box *b, int (*cb[1])(int)) { return b->item->leaf->name; }",
        "struct c_item *w(struct c_box *b, int (*cb[1])(int)) { return b->item; }",
    ] {
        assert_eq!(outcome(body), safe(), "{body}");
    }
    // Rule 10l serves the typed positions only: a statement the older
    // routes read keeps the lexical answer, whose unread declarations leave
    // `b` not never bound there. Card CG10-BOUND-UNTRACKED-ROOTS flip (rule
    // 11d): the flow's per-path premise, which reads no declaration answer,
    // reads `b` at `b->flag` as an untracked borrowed object (no binding, no
    // refusal on the path, never address-taken). BEFORE:
    // `("Unknown", ["unknown_alias:b"])`.
    assert_eq!(
        outcome(
            "char *w(struct c_box *b, int (*cb[1])(int)) { int n = b->flag; c_use(n); \
             return b->item->leaf->name; }"
        ),
        safe()
    );
    // The never-bound answer is unchanged: a custody position on `b` before
    // the read still refuses it (the path is refused there).
    let (status, keys) = outcome(
        "char *w(struct c_box *b, int (*cb[1])(int)) { c_drop(b); int n = b->flag; c_use(n); \
         return 0; }",
    );
    assert_ne!(status, "Safe", "{keys:?}");
}

/// The same body with a formal the lexical reader reads is the lexical
/// answer's, unchanged: it certified before this card.
#[test]
fn the_lexical_answer_is_kept_where_it_answers() {
    if !clang_available() {
        return;
    }
    assert_eq!(
        outcome("char *w(struct c_box *b) { return b->item->leaf->name; }"),
        safe()
    );
}

/// A root whose address is taken, written from a call, released, bound by a
/// macro-expanded assignment, or handed to a `Borrowed` callee (never bound,
/// but `may_bind`: rule 5b's premise) keeps the refusal.
#[test]
fn a_bound_or_may_bind_root_keeps_the_refusal() {
    if !clang_available() {
        return;
    }
    for body in [
        // `&root`
        "char *w(struct c_box *b, int (*cb[1])(int)) { struct c_box **pb = &b; return b->item->leaf->name; }",
        // written from a call
        "char *w(struct c_box *b, int (*cb[1])(int)) { b = c_make(1); return b->item->leaf->name; }",
        // copied from a written root
        "char *w(struct c_box *b, int (*cb[1])(int)) { struct c_box *a = c_make(1); b = a; return b->item->leaf->name; }",
        // released at a custody position
        "char *w(struct c_box *b, int (*cb[1])(int)) { c_drop(b); return b->item->leaf->name; }",
        // a `Borrowed` callee: never bound, but `may_bind`
        "char *w(struct c_box *b, int (*cb[1])(int)) { c_hold(b); return b->item->leaf->name; }",
        // incremented
        "char *w(struct c_box *b, int (*cb[1])(int)) { b++; return b->item->leaf->name; }",
    ] {
        assert_refused("", body);
    }
    assert_refused(
        "#define C_SET(p) p = c_make(1)",
        "char *w(struct c_box *b, int (*cb[1])(int)) { C_SET(b); return b->item->leaf->name; }",
    );
}

/// A name declared twice (a nested block redeclares the root) is not answered
/// by the AST reader: the inner object is bound, and the outer one keeps the
/// refusal.
#[test]
fn a_shadowed_root_keeps_the_refusal() {
    if !clang_available() {
        return;
    }
    assert_refused(
        "",
        "char *w(struct c_box *b, int (*cb[1])(int)) { \
         { struct c_box *b = c_make(1); if (b) c_drop(b); } \
         return b->item->leaf->name; }",
    );
    assert_refused(
        "",
        "char *w(struct c_box *b, int (*cb[1])(int)) { \
         { struct c_box *b = 0; (void)b; } \
         return b->item->leaf->name; }",
    );
}

/// A typedef redeclared as C11 permits (one type, two declarations) is one
/// type to Clang and certifies; a block-scope typedef of the same name for a
/// distinct type is not answered.
#[test]
fn a_redeclared_typedef_certifies_only_when_it_is_one_type() {
    if !clang_available() {
        return;
    }
    let twice = "typedef struct c_box c_box_t; typedef struct c_box c_box_t;";
    assert_eq!(
        outcome_with(
            twice,
            "char *w(c_box_t *b, int (*cb[1])(int)) { return b->item->leaf->name; }"
        ),
        safe()
    );
    assert_refused(
        "typedef struct c_box c_t;",
        "char *w(c_t *b, int (*cb[1])(int)) { typedef struct c_leaf c_t; \
         return b->item->leaf->name; }",
    );
}

/// The P3 control (now `a_redeclared_typedef_formal_reads_its_const_pointer_return`)
/// refused for a reason outside the declaration reader: `node->host` through
/// a `const` pointee is a `char *const` value, which the typed reader did not
/// read as a pointer until W47-TYPED-CONST-POINTER-VALUE (rule 10m). With a
/// non-`const` formal the same doubly declared typedef certifies under the
/// lexical answer, and since rule 10m so does the `const` one: the premise
/// answer is the same in both, only the value's qualifier differed.
#[test]
fn the_redeclared_typedef_control_certifies_with_its_const_pointee() {
    if !clang_available() {
        return;
    }
    let twice = "typedef struct g_s g_t; typedef struct g_s { char *host; } g_t;";
    assert_eq!(
        outcome_with(twice, "char *w(g_t *node) { return node->host; }"),
        safe()
    );
    assert_eq!(
        outcome_with(
            twice,
            "const char *w(const g_t *node) { return node->host; }"
        ),
        safe()
    );
}

/// A K&R definition, an implicitly declared callee (which C11 Clang rejects)
/// and a root that is not a declaration of this function (a file-scope
/// object) are not answered.
#[test]
fn an_unprototyped_definition_or_a_non_local_root_keeps_the_refusal() {
    if !clang_available() {
        return;
    }
    assert_refused(
        "",
        "char *w(b, cb) struct c_box *b; int (*cb[1])(int); { return b->item->leaf->name; }",
    );
    assert_refused(
        "",
        "char *w(struct c_box *b, int (*cb[1])(int)) { c_mystery(b); return b->item->leaf->name; }",
    );
    assert_refused(
        "struct c_box *c_global;",
        "char *w(int (*cb[1])(int)) { return c_global->item->leaf->name; }",
    );
}

/// A declared local the lexical reader does not type is answered by the AST:
/// `it` is one automatic `struct c_item *` of this body, so `it = b->item` is
/// P2's borrowed read followed by the null branch, which keeps its identity
/// refusal (rule 10b). A formal, a `static` local and a shadowed local are not
/// declared locals.
#[test]
fn an_assignment_operand_reads_the_ast_declared_local() {
    if !clang_available() {
        return;
    }
    let local = "int w(struct c_box *b, int (*cb[1])(int)) { struct c_item *it; \
        if (!(it = b->item)) return 0; return 1; }";
    assert_eq!(
        outcome(local),
        ("Unknown", vec!["unknown_alias:it".to_owned()])
    );
    for body in [
        "int w(struct c_box *b, int (*cb[1])(int)) { static struct c_item *it; \
         if (!(it = b->item)) return 0; return 1; }",
        "int w(struct c_box *b, int (*cb[1])(int)) { struct c_item *it = 0; \
         { struct c_item *it; (void)it; } if (!(it = b->item)) return 0; return 1; }",
    ] {
        let (status, keys) = outcome(body);
        assert_eq!(status, "Unknown", "{body}");
        assert!(
            keys.iter().any(|key| key.contains("branch condition")),
            "{body}: {keys:?}"
        );
    }
}
