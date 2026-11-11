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

//! W45-SR5-SOUNDNESS — the pinned controls for the holes STABILITY-REEVAL-5
//! registered (`docs/reports/w45-sr5-soundness/README.md`). Every admitting
//! shape the card's one-off probe measured is pinned not `Safe` (with the
//! violation or refusal it now carries), beside its `Safe` neighbour.
//!
//! **The families are held out.** The `q_*` fixture contracts, and `p_*`/`u_*`
//! APIs declared inline: no production contract moves. Every `Safe` below is
//! `Safe` for a synthetic declared projection under the premises the report
//! names, and every `Unsafe` is provisional, never a claim about a library.

#[path = "support/host_abi.rs"]
mod host_abi;

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::counted_users::ClauseVerdict;
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, CCompilerDialect, LegacyDiagnosticKind,
    LegacyFunctionScope, LegacyProgramSummary, ResourceContracts, UnrestrictedLegacyResult,
    analyze_legacy_c_translation_unit, analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::contract_revision::PinMismatchReason;
use qcore_schemas::verdict::Verdict;

fn load(extra: &[(&str, &str)]) -> ResourceContracts {
    let mut sources = BTreeMap::from([
        (
            "q_base.qcore".to_owned(),
            include_str!("fixtures/contracts/q_base.qv").to_owned(),
        ),
        (
            "q_nullable.qcore".to_owned(),
            include_str!("fixtures/contracts/q_nullable.qv").to_owned(),
        ),
    ]);
    for (name, text) in extra {
        sources.insert((*name).to_owned(), (*text).to_owned());
    }
    load_resource_contract_sources(&sources).expect("the fixture contracts load")
}

fn analyse(source: &str, entry: &str, contracts: &ResourceContracts) -> LegacyProgramSummary {
    analyze_legacy_c_translation_unit(source, entry, contracts).expect("the fixture analyses")
}

fn status(summary: &LegacyProgramSummary, function: &str) -> AnalysisStatus {
    summary.functions[function].status
}

fn descriptions(summary: &LegacyProgramSummary, function: &str) -> Vec<String> {
    summary.functions[function]
        .diagnostics
        .iter()
        .map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Unsupported { description } => description.clone(),
            other => format!("{other:?}"),
        })
        .collect()
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent W45-SR5-SOUNDNESS test: {reason}");
            false
        }
    }
}

fn c11(
    source: &str,
    scope: LegacyFunctionScope,
    contracts: &ResourceContracts,
) -> UnrestrictedLegacyResult {
    analyze_unrestricted_c11(
        &C11TranslationUnit {
            source,
            dialect: CCompilerDialect::Gnu17,
            function_scope: scope,
            ..Default::default()
        },
        "subject",
        contracts,
    )
    .expect("the fixture is accepted")
}

fn unit_functions(result: &UnrestrictedLegacyResult) -> &LegacyProgramSummary {
    match result {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown { partial, .. } => {
            partial.as_ref().expect("the unit was analysed")
        }
    }
}

// ---- 1. Rule 5k: a member release and a callee that releases it again ----

const DECLS: &str = "void *q_alloc(void); void q_free(void *); void q_nullable_free(void *);\n";

const RECORD: &str = "struct inner { int n; };\n\
     struct node { void *buf; struct node *next; struct inner in; int n; };\n";

const DESTROY: &str = "static void node_destroy(struct node *n) {\n\
     if (n->buf) q_nullable_free(n->buf);\n\
     q_nullable_free(n);\n}\n";

const DESTROY_NEXT: &str = "static void node_destroy_next(struct node *n) {\n\
     if (n->next) q_nullable_free(n->next);\n\
     q_nullable_free(n);\n}\n";

const DROP: &str = "static void node_drop(struct node *n) {\n    node_destroy(n);\n}\n";

fn member(helpers: &str, body: &str, contracts: &ResourceContracts) -> LegacyProgramSummary {
    analyse(
        &format!("{DECLS}{RECORD}{helpers}void subject(struct node *n) {{ {body} }}\n"),
        "subject",
        contracts,
    )
}

fn is_double_release(summary: &LegacyProgramSummary, callee: &str, place: &str) -> bool {
    let descriptions = descriptions(summary, "subject");
    status(summary, "subject") == AnalysisStatus::Unsafe
        && descriptions
            .iter()
            .any(|text| text.starts_with("DoubleRelease"))
        && descriptions.iter().any(|text| {
            text.starts_with(&format!("release of `{place}` before `{callee}("))
                && text.ends_with("(member-released-again-by-callee)")
        })
}

#[test]
fn a_member_released_again_by_a_unit_callee_is_a_double_release() {
    let plain = load(&[]);
    for (reason, helpers, body, callee, place) in [
        (
            "the measured shape",
            DESTROY.to_owned(),
            "q_nullable_free(n->buf); node_destroy(n);",
            "node_destroy",
            "n->buf",
        ),
        (
            "the member null-tested first",
            DESTROY.to_owned(),
            "if (n->buf) q_nullable_free(n->buf); node_destroy(n);",
            "node_destroy",
            "n->buf",
        ),
        (
            "through a wrapper",
            format!("{DESTROY}{DROP}"),
            "q_nullable_free(n->buf); node_drop(n);",
            "node_drop",
            "n->buf",
        ),
        (
            "another member the destructor releases",
            DESTROY_NEXT.to_owned(),
            "q_nullable_free(n->next); node_destroy_next(n);",
            "node_destroy_next",
            "n->next",
        ),
    ] {
        let summary = member(&helpers, body, &plain);
        assert!(
            is_double_release(&summary, callee, place),
            "{reason}: {:?}",
            descriptions(&summary, "subject")
        );
    }
}

#[test]
fn a_declaration_does_not_hide_the_body_this_unit_defines() {
    let declared = load(&[(
        "destroy.qcore",
        "module contract;\nextern fn node_destroy(n: ptr<mut, u8>) -> unit\ncontract { \
         requires resource_contract_v1; ensures normal resource_released(n, nullable); \
         ensures normal resource_return_none(); };\n",
    )]);
    let summary = member(
        DESTROY,
        "q_nullable_free(n->buf); node_destroy(n);",
        &declared,
    );
    assert!(
        is_double_release(&summary, "node_destroy", "n->buf"),
        "{:?}",
        descriptions(&summary, "subject")
    );
    // With no body here the declaration governs (P-B08-SHALLOW): unchanged.
    let summary = member(
        "void node_destroy(struct node *n);\n",
        "q_nullable_free(n->buf); node_destroy(n);",
        &declared,
    );
    assert_eq!(status(&summary, "subject"), AnalysisStatus::Safe);
}

#[test]
fn the_neighbours_of_the_hole_stay_safe() {
    let plain = load(&[]);
    for (reason, helpers, body) in [
        ("the destructor alone", DESTROY, "node_destroy(n);"),
        (
            "the destructor body itself",
            "",
            "if (n->buf) q_nullable_free(n->buf); q_nullable_free(n);",
        ),
        (
            "a member the destructor does not touch",
            DESTROY_NEXT,
            "q_nullable_free(n->buf); node_destroy_next(n);",
        ),
    ] {
        let summary = member(helpers, body, &plain);
        assert_eq!(
            status(&summary, "subject"),
            AnalysisStatus::Safe,
            "{reason}: {:?}",
            descriptions(&summary, "subject")
        );
    }
}

#[test]
fn the_mirrored_order_and_the_aliases_are_not_safe() {
    let plain = load(&[]);
    let alias = "static void node_destroy_alias(struct node *n) {\n\
                 void *b = n->buf;\n    q_nullable_free(b);\n    q_nullable_free(n);\n}\n";
    for (reason, helpers, body) in [
        (
            "the destructor, then the member",
            DESTROY,
            "node_destroy(n); q_nullable_free(n->buf); q_nullable_free(n);",
        ),
        (
            "the member through a local",
            DESTROY,
            "void *b = n->buf; q_nullable_free(b); node_destroy(n);",
        ),
        (
            "a destructor that releases the member through a local",
            alias,
            "q_nullable_free(n->buf); node_destroy_alias(n);",
        ),
    ] {
        let summary = member(helpers, body, &plain);
        assert_ne!(
            status(&summary, "subject"),
            AnalysisStatus::Safe,
            "{reason}: {:?}",
            descriptions(&summary, "subject")
        );
    }
}

#[test]
fn the_unit_verdict_of_the_hole_is_not_safe() {
    if !clang_available() {
        return;
    }
    let result = c11(
        &format!(
            "{DECLS}{RECORD}{DESTROY}void subject(struct node *n) {{ q_nullable_free(n->buf); node_destroy(n); }}\n"
        ),
        LegacyFunctionScope::default(),
        &load(&[]),
    );
    assert_eq!(result.verdict(), Verdict::Unsafe);
}

// ---- 2. A reserved prefix the source spells ----

fn pinned() -> ResourceContracts {
    let mut contracts = load(&[(
        "k.qcore",
        "module contract;\nextern fn k_close(h: ptr<mut, u8>) -> unit\ncontract { \
         requires resource_contract_v1; ensures normal resource_released(h, nullable); \
         ensures normal resource_return_none(); };\n",
    )]);
    contracts
        .withhold_for_pin_mismatch(
            "k_close",
            "k.qcore".into(),
            PinMismatchReason::SourceChanged,
            Some("contracts.lock".into()),
            "the audited source's sites differ from the lock".into(),
        )
        .unwrap();
    contracts
}

const HELD_DECLS: &str = "void k_close(void *); void zz(void *); void qcore_pin_mismatch__zz(void *); \
     void qcore_review_pending__zz(void *); void qcore_pin_mismatch__k_close(void *);\n";

fn held(body: &str, contracts: &ResourceContracts) -> LegacyProgramSummary {
    analyse(
        &format!("{HELD_DECLS}void subject(void) {{ {body} }}\n"),
        "subject",
        contracts,
    )
}

#[test]
fn a_source_spelled_prefix_is_unknown_never_pending() {
    let plain = load(&[]);
    for (reason, body, contracts) in [
        ("the pin prefix", "qcore_pin_mismatch__zz(0);", &plain),
        ("the review prefix", "qcore_review_pending__zz(0);", &plain),
        (
            "a spelling that collides with a withheld name",
            "qcore_pin_mismatch__k_close(0);",
            &pinned(),
        ),
    ] {
        let summary = held(body, contracts);
        assert_eq!(
            status(&summary, "subject"),
            AnalysisStatus::Unknown,
            "{reason}"
        );
        assert!(
            descriptions(&summary, "subject")
                .iter()
                .all(|text| text.ends_with("(reserved-prefix-in-source)")),
            "{reason}: {:?}",
            descriptions(&summary, "subject")
        );
    }
    let summary = analyse(
        &format!(
            "{HELD_DECLS}static void qcore_pin_mismatch__zz(void *p) {{ }}\n\
             void subject(void) {{ qcore_pin_mismatch__zz(0); }}\n"
        ),
        "subject",
        &plain,
    );
    assert_eq!(status(&summary, "subject"), AnalysisStatus::Unknown);
    // Its control: the same call to a name the source spells plainly.
    assert_eq!(
        status(&held("zz(0);", &plain), "subject"),
        AnalysisStatus::Unknown
    );
}

#[test]
fn a_genuine_hold_is_still_pending() {
    // Card W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL: `k_close(0);` was this
    // test's held call until that card, and it is one of the card's own
    // instances: with the declaration in force the literal actual does not
    // match the contract (`arguments do not match resource contract`), so no
    // re-pin alone makes it `Safe` and it is now `Unknown`. A formal actual is
    // the genuine hold.
    let summary = held("k_close(0);", &pinned());
    assert_eq!(status(&summary, "subject"), AnalysisStatus::Unknown);
    let summary = analyse(
        &format!("{HELD_DECLS}void subject(void *p) {{ k_close(p); }}\n"),
        "subject",
        &pinned(),
    );
    assert_eq!(status(&summary, "subject"), AnalysisStatus::Pending);
    // Beside a source-spelled prefix the function joins to `Unknown`.
    let summary = analyse(
        &format!(
            "{HELD_DECLS}void subject(void *p) {{ k_close(p); qcore_pin_mismatch__zz(0); }}\n"
        ),
        "subject",
        &pinned(),
    );
    assert_eq!(status(&summary, "subject"), AnalysisStatus::Unknown);
}

// ---- 3. A container root released after a publication (10d) ----

const LIST: &str = r#"module contract;
extern fn p_malloc(size: usize) -> ptr<mut, u8>
contract { requires resource_contract_v1; ensures normal resource_borrowed(size); ensures normal resource_return_owned(nullable); };
extern fn p_free(p: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; ensures normal resource_released(p, nullable); ensures normal resource_return_none(); };
extern fn p_list_add(entry: ptr<mut, u8>, head: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; ensures normal resource_borrowed(entry); ensures normal resource_borrowed(head); ensures normal resource_published(entry, head); ensures normal resource_return_none(); };
"#;

const LIST_PRELUDE: &str = concat!(
    host_abi::size_t_declaration!(),
    "\n\
     struct p_list { struct p_list *prev, *next; };\n\
     struct p_queue { struct p_list pending; };\n\
     struct p_node { int value; struct p_list _head; };\n\
     void *p_malloc(size_t size); void p_free(void *p);\n\
     void p_list_add(struct p_list *entry, struct p_list *head);\n"
);

fn publish(body: &str) -> LegacyProgramSummary {
    let contracts = load_resource_contract_sources(&BTreeMap::from([(
        "list.qcore".to_owned(),
        LIST.to_owned(),
    )]))
    .expect("the list contracts load");
    analyse(&format!("{LIST_PRELUDE}{body}\n"), "subject", &contracts)
}

const PUBLISH: &str = "struct p_node *e = p_malloc(sizeof(struct p_node)); if (!e) return; \
     p_list_add(&e->_head, &q->pending);";

#[test]
fn a_released_container_root_discharges_no_publication() {
    for (reason, body) in [
        (
            "released directly",
            format!("void subject(struct p_queue *q) {{ {PUBLISH} p_free(q); }}"),
        ),
        (
            "released before the publication",
            "void subject(struct p_queue *q) { struct p_node *e = p_malloc(sizeof(struct p_node)); \
             if (!e) return; p_free(q); p_list_add(&e->_head, &q->pending); }"
                .to_owned(),
        ),
        (
            "released by a summary",
            format!(
                "static void queue_destroy(struct p_queue *q) {{ p_free(q); }}\n\
                 void subject(struct p_queue *q) {{ {PUBLISH} queue_destroy(q); }}"
            ),
        ),
        (
            "released through a local initialised from the formal",
            format!(
                "void subject(struct p_queue *q) {{ struct p_queue *r = q; {PUBLISH} p_free(r); }}"
            ),
        ),
    ] {
        let summary = publish(&body);
        assert_ne!(
            status(&summary, "subject"),
            AnalysisStatus::Safe,
            "{reason}: {:?}",
            descriptions(&summary, "subject")
        );
    }
    let summary = publish(&format!(
        "void subject(struct p_queue *q) {{ {PUBLISH} p_free(q); }}"
    ));
    assert!(
        descriptions(&summary, "subject")
            .iter()
            .any(|text| text.contains("into a container this function does not show outlives it")),
        "{:?}",
        descriptions(&summary, "subject")
    );
}

#[test]
fn a_publication_into_a_root_the_body_keeps_is_still_safe() {
    let summary = publish(&format!("void subject(struct p_queue *q) {{ {PUBLISH} }}"));
    assert_eq!(
        status(&summary, "subject"),
        AnalysisStatus::Safe,
        "{:?}",
        descriptions(&summary, "subject")
    );
}

// ---- 4. An element bound over a formal (10d with 5l(2)) ----

const CONTAINER_PRELUDE: &str = r#"
#include <stddef.h>
#define container_of(ptr, type, member) ({                      \
      const typeof( ((type *)0)->member ) *__mptr = (ptr);      \
      (type *)( (char *)__mptr - offsetof(type,member) );})
struct mk_list { struct mk_list *prev, *next; };
void mk_list_add(struct mk_list *entry, struct mk_list *head);
void *q_alloc(void); void q_free(void *);
struct item { int size; void *payload; struct mk_list _head; };
struct queue { struct mk_list items; };
"#;

const MK_LIST: &str = "module contract;
extern fn mk_list_add(entry: ptr<mut, u8>, head: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; ensures normal resource_borrowed(entry); ensures normal resource_borrowed(head); ensures normal resource_published(entry, head); ensures normal resource_return_none(); };
";

#[test]
fn an_element_bound_over_a_formal_is_not_laundered() {
    if !clang_available() {
        return;
    }
    let contracts = load(&[("list.qcore", MK_LIST)]);
    let helper = "static void link_it(void *arg, struct queue *q) { \
                  struct item *ctx = container_of(arg, struct item, _head); \
                  mk_list_add(&ctx->_head, &q->items); }\n";
    // The caller frees the element it linked: never `Safe`.
    let result = c11(
        &format!(
            "{CONTAINER_PRELUDE}{helper}void subject(struct queue *q) {{ \
             struct item *e = q_alloc(); link_it(&e->_head, q); q_free(e); }}\n"
        ),
        LegacyFunctionScope::default(),
        &contracts,
    );
    assert_ne!(result.verdict(), Verdict::Safe);
    let summary = unit_functions(&result);
    assert_ne!(status(summary, "link_it"), AnalysisStatus::Safe);
    assert_ne!(status(summary, "subject"), AnalysisStatus::Safe);
    // The control: a publication of a formal's own object, spelled as the
    // formal, keeps its pre-card answer.
    let result = c11(
        &format!(
            "{CONTAINER_PRELUDE}void subject(struct item *arg, struct queue *q) {{ \
             mk_list_add(&arg->_head, &q->items); }}\n"
        ),
        LegacyFunctionScope::default(),
        &contracts,
    );
    assert_eq!(result.verdict(), Verdict::Safe);
}

// ---- 5. Counted users (10e) ----

const USERS: &str = r#"module contract;
extern fn u_ref(t: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; ensures normal resource_borrowed(t); ensures normal resource_user_added(t, refs); ensures normal resource_return_none(); };
extern fn u_unref(t: ptr<mut, u8>, check: i32) -> unit
contract { requires resource_contract_v1; requires resource_literal_argument(check, 0); ensures normal resource_borrowed(t); ensures normal resource_borrowed(check); ensures normal resource_user_dropped(t, refs); ensures normal resource_return_none(); };
extern fn u_release(t: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; ensures normal resource_borrowed(t); ensures normal resource_user_release_check(t, refs); ensures normal resource_return_none(); };
extern fn hx(t: ptr<mut, u8>) -> i32
contract { requires resource_contract_v1; ensures normal resource_borrowed(t); ensures normal resource_user_exit(t, 0, restored); ensures normal resource_return_none(); };
extern fn cx(t: ptr<mut, u8>) -> i32
contract { requires resource_contract_v1; ensures normal resource_borrowed(t); ensures normal resource_user_exit(t, 0, restored); ensures normal resource_return_none(); };
extern fn helper_neutral(q: ptr<mut, u8>) -> i32
contract { requires resource_contract_v1; ensures normal resource_borrowed(q); ensures normal resource_user_neutral(); ensures normal resource_return_none(); };
"#;

const USERS_PRELUDE: &str = "struct u_task { int refs; };\n\
     void u_ref(struct u_task *t); void u_unref(struct u_task *t, int check); \
     void u_release(struct u_task *t);\nint c;\n";

fn users(body: &str) -> LegacyProgramSummary {
    let contracts = load_resource_contract_sources(&BTreeMap::from([(
        "users.qcore".to_owned(),
        USERS.to_owned(),
    )]))
    .expect("the counted-user contracts load");
    analyse(&format!("{USERS_PRELUDE}{body}\n"), "cx", &contracts)
}

fn clause(summary: &LegacyProgramSummary, function: &str, clause: &str) -> Option<ClauseVerdict> {
    summary.functions[function]
        .counted_users
        .iter()
        .find(|record| record.clause == clause)
        .map(|record| record.verdict)
}

const CALLER: &str = "int cx(struct u_task *t) { hx(t); return 0; }";

#[test]
fn an_operation_c_may_skip_decides_no_clause() {
    for (reason, hx) in [
        (
            "behind `?:`",
            "int hx(struct u_task *t) { u_ref(t); c ? u_unref(t, 0) : (void)0; return 0; }",
        ),
        (
            "behind `&&`",
            "int hx(struct u_task *t) { u_ref(t); c && (u_unref(t, 0), 1); return 0; }",
        ),
        (
            "through a call's result",
            "typedef void (*fn_t)(struct u_task *);\nstatic fn_t pick(int n) { return 0; }\n\
             int hx(struct u_task *t) { u_ref(t); u_unref(t, 0); pick(1)(t); return 0; }",
        ),
    ] {
        let summary = users(&format!("{hx}\n{CALLER}"));
        assert_eq!(
            clause(&summary, "hx", "resource_user_exit(t, 0, restored)"),
            Some(ClauseVerdict::Undecided),
            "{reason}"
        );
        assert_ne!(status(&summary, "hx"), AnalysisStatus::Safe, "{reason}");
        assert_ne!(status(&summary, "cx"), AnalysisStatus::Safe, "{reason}");
    }
    // The control: the same operations unconditionally.
    let summary = users(&format!(
        "int hx(struct u_task *t) {{ u_ref(t); u_unref(t, 0); return 0; }}\n{CALLER}"
    ));
    assert_eq!(
        clause(&summary, "hx", "resource_user_exit(t, 0, restored)"),
        Some(ClauseVerdict::Held)
    );
    assert_eq!(status(&summary, "cx"), AnalysisStatus::Safe);
}

#[test]
fn a_declared_neutrality_is_applied_only_when_its_body_verifies_it() {
    let caller = "int cx(struct u_task *t) { u_ref(t); helper_neutral(t); u_unref(t, 0); \
                  u_release(t); return 0; }";
    for (reason, helper) in [
        (
            "a counter write in a loop",
            "static int helper_neutral(struct u_task *q) { while (c) { q->refs++; } return 0; }",
        ),
        (
            "an operation in a loop",
            "static int helper_neutral(struct u_task *q) { while (c) { u_ref(q); } return 0; }",
        ),
    ] {
        let summary = users(&format!("{helper}\n{caller}"));
        assert_ne!(status(&summary, "cx"), AnalysisStatus::Safe, "{reason}");
        assert_eq!(
            clause(&summary, "cx", "resource_user_exit(t, 0, restored)"),
            Some(ClauseVerdict::Undecided),
            "{reason}"
        );
    }
    let summary = users(&format!(
        "static int helper_neutral(struct u_task *q) {{ return q->refs; }}\n{caller}"
    ));
    assert_eq!(status(&summary, "cx"), AnalysisStatus::Safe);
}

// ---- 6. A `Pending` unit and the `Safe` gates ----

#[test]
fn a_held_call_beside_an_unread_definition_is_unknown() {
    if !clang_available() {
        return;
    }
    let scope = LegacyFunctionScope::MainSourceDefinitions;
    // Card W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL: the held call takes a
    // formal; `k_close(0)` does not match the declaration in force and is
    // `Unknown` since that card (see `a_genuine_hold_is_still_pending`).
    let unit = |body: &str| {
        c11(
            &format!("void k_close(void *);\n{body}\n"),
            scope,
            &pinned(),
        )
    };
    assert_eq!(
        unit("void subject(void *p) { k_close(p); }").verdict(),
        Verdict::Pending
    );
    for (reason, body) in [
        (
            "an `#if 0` definition",
            "#if 0\nint dead(void) { return 0; }\n#endif\nvoid subject(void *p) { k_close(p); }",
        ),
        (
            "a K&R definition",
            "int kr(a) int a; { return a; }\nvoid subject(void *p) { k_close(p); }",
        ),
    ] {
        let result = unit(body);
        assert_eq!(result.verdict(), Verdict::Unknown, "{reason}");
        // The function row keeps its own held reason.
        assert_eq!(
            status(unit_functions(&result), "subject"),
            AnalysisStatus::Pending,
            "{reason}"
        );
    }
    // The control: the same unread definition without the hold.
    assert_eq!(
        unit("#if 0\nint dead(void) { return 0; }\n#endif\nvoid subject(void) { }").verdict(),
        Verdict::Unknown
    );
}

// ---- 7. The two candidates the probe measured fail-closed, pinned ----

const ITERATION_PRELUDE: &str = r#"
#include <stddef.h>
#define container_of(ptr, type, member) ({                      \
      const typeof( ((type *)0)->member ) *__mptr = (ptr);      \
      (type *)( (char *)__mptr - offsetof(type,member) );})
struct mk_list { struct mk_list *prev, *next; };
#define mk_list_entry( ptr, type, member ) container_of( ptr, type, member )
#define mk_list_foreach_safe(curr, n, head) \
    for (curr = (head)->next, n = curr->next; curr != (head); curr = n, n = curr->next)
void mk_list_add(struct mk_list *entry, struct mk_list *head);
void mk_list_del(struct mk_list *entry);
void *q_alloc(void); void q_free(void *);
struct item { int size; void *payload; struct mk_list _head; };
struct queue { struct mk_list items; };
"#;

const MK_LIST_DEL: &str = "module contract;
extern fn mk_list_add(entry: ptr<mut, u8>, head: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; ensures normal resource_borrowed(entry); ensures normal resource_borrowed(head); ensures normal resource_published(entry, head); ensures normal resource_return_none(); };
extern fn mk_list_del(entry: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; ensures normal resource_borrowed(entry); ensures normal resource_unpublished(entry); ensures normal resource_return_none(); };
";

/// `GATEWAY-5L-ELEMENT-TRACKED-AFTER-FOREIGN-UNLINK`: a foreign-unlinked
/// element re-published into the iterated list under another head spelling.
#[test]
fn a_foreign_element_republished_under_an_alias_head_is_not_safe() {
    if !clang_available() {
        return;
    }
    let contracts = load(&[("list.qcore", MK_LIST_DEL)]);
    let result = c11(
        &format!(
            "{ITERATION_PRELUDE}void subject(struct queue *q) {{ struct mk_list *c, *n; \
             struct mk_list *h = &q->items; struct item *e;\n\
             mk_list_foreach_safe(c, n, &q->items) {{ e = mk_list_entry(c, struct item, _head); \
             mk_list_del(&e->_head); mk_list_add(&e->_head, h); }} }}\n"
        ),
        LegacyFunctionScope::default(),
        &contracts,
    );
    assert_ne!(result.verdict(), Verdict::Safe);
    // The control: unlinked and released under the iterated spelling.
    let result = c11(
        &format!(
            "{ITERATION_PRELUDE}void subject(struct queue *q) {{ struct mk_list *c, *n; \
             struct item *e;\n\
             mk_list_foreach_safe(c, n, &q->items) {{ e = mk_list_entry(c, struct item, _head); \
             mk_list_del(&e->_head); q_free(e); }} }}\n"
        ),
        LegacyFunctionScope::default(),
        &contracts,
    );
    assert_eq!(result.verdict(), Verdict::Safe);
}

/// `STATIC-ROOT-SHADOW-READER`: a global shadowed by a declaration that
/// contains `(` is never read as the static root.
#[test]
fn a_shadowed_static_root_is_not_a_root() {
    if !clang_available() {
        return;
    }
    let contracts = load(&[("list.qcore", MK_LIST_DEL)]);
    let publish = "struct item *e = q_alloc(); mk_list_add(&e->_head, &g);";
    for (reason, declaration) in [
        (
            "a parenthesised initialiser",
            "struct mk_list g = { (struct mk_list *)0, (struct mk_list *)0 };",
        ),
        ("a parenthesised declarator", "struct mk_list (g);"),
    ] {
        let result = c11(
            &format!(
                "{ITERATION_PRELUDE}struct mk_list g;\nvoid subject(void) {{ {declaration} {publish} }}\n"
            ),
            LegacyFunctionScope::default(),
            &contracts,
        );
        assert_ne!(result.verdict(), Verdict::Safe, "{reason}");
    }
    // The control: the unshadowed global is a static root.
    let result = c11(
        &format!("{ITERATION_PRELUDE}struct mk_list g;\nvoid subject(void) {{ {publish} }}\n"),
        LegacyFunctionScope::default(),
        &contracts,
    );
    assert_eq!(result.verdict(), Verdict::Safe);
}
