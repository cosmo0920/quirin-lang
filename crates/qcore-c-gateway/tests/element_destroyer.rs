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

//! Card **W48-SCHED-CLEANUP-PRODUCTION**: the element destroyer (gateway
//! constraint 34).
//!
//! A declaration may state both `resource_unpublished(p)` and
//! `resource_released(p, …)` for one parameter: the call unlinks the object
//! `p` designates from the container that holds it and then ends it. At a
//! call the caller reads exactly that order: rule 10d/10f's unlink (custody of
//! an object this path published, or rule 10f(2)'s foreign element of an
//! admitted iteration), then the declared release of that custody. Every
//! other entry keeps the unlink's refusal.
//!
//! The prelude spells the Monkey list macros as the pinned headers do, so
//! every fixture reaches the gateway in the preprocessed form a production
//! unit does. `item_destroy` stands for `mk_http_thread_destroy`.

use std::process::Command;

use qcore_c_gateway::gateway_rules;
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, CCompilerDialect, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const PRELUDE: &str = r#"
#include <stddef.h>
#define container_of(ptr, type, member) ({                      \
      const typeof( ((type *)0)->member ) *__mptr = (ptr);      \
      (type *)( (char *)__mptr - offsetof(type,member) );})
struct mk_list { struct mk_list *prev, *next; };
#define mk_list_foreach(curr, head) for( curr = (head)->next; curr != (head); curr = curr->next )
#define mk_list_foreach_safe(curr, n, head) \
    for (curr = (head)->next, n = curr->next; curr != (head); curr = n, n = curr->next)
#define mk_list_entry( ptr, type, member ) container_of( ptr, type, member )
#define mk_list_entry_first(ptr, type, member) container_of((ptr)->next, type, member)
void mk_list_add(struct mk_list *entry, struct mk_list *head);
void mk_list_del(struct mk_list *entry);
void *q_alloc(void); void q_free(void *); void q_use(void *);
struct item { int close; struct session *session; struct mk_list _head; };
struct session { void *conn; };
struct queue { struct mk_list items; struct mk_list done; int total; };
int item_destroy(struct item *e);
"#;

const LIST: &str = "module contract;
extern fn mk_list_add(entry: ptr<mut, u8>, head: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; ensures normal resource_borrowed(entry); ensures normal resource_borrowed(head); ensures normal resource_published(entry, head); ensures normal resource_return_none(); };
extern fn mk_list_del(entry: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; ensures normal resource_borrowed(entry); ensures normal resource_unpublished(entry); ensures normal resource_return_none(); };
extern fn q_use(p: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; ensures normal resource_borrowed(p); ensures normal resource_return_none(); };
extern fn item_destroy(e: ptr<mut, u8>) -> i32
contract { requires resource_contract_v1; ensures normal resource_unpublished(e); ensures normal resource_released(e, nonnull); ensures normal resource_return_none(); };
";

const NOT_PUBLISHED: &str =
    "unlink by `item_destroy` of `e`, which this function has not published";
const PLAIN_REFUSAL: &str = "the current element of an intrusive-list iteration that does not \
                             save its successor is unlinked or released";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(
        &[
            (
                "q_base.qcore".to_owned(),
                include_str!("fixtures/contracts/q_base.qv").to_owned(),
            ),
            ("list.qcore".to_owned(), LIST.to_owned()),
        ]
        .into_iter()
        .collect(),
    )
    .unwrap()
}

fn analyze_entry(body: &str, entry: &str) -> (AnalysisStatus, Vec<LegacyDiagnosticKind>) {
    let source = format!("{PRELUDE}\n{body}");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            dialect: CCompilerDialect::Gnu17,
            ..Default::default()
        },
        entry,
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("valid fixture rejected: {error:?}\n{source}"));
    let summary = match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.unwrap(),
    };
    let result = &summary.functions[entry];
    (
        result.status,
        result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.kind.clone())
            .collect(),
    )
}

fn analyze(body: &str) -> (AnalysisStatus, Vec<LegacyDiagnosticKind>) {
    analyze_entry(body, "subject")
}

fn clang_available() -> bool {
    qcore_c_gateway::clang_command().is_ok()
}

fn has(diagnostics: &[LegacyDiagnosticKind], wanted: &str) -> bool {
    diagnostics.iter().any(|kind| {
        matches!(kind, LegacyDiagnosticKind::Unsupported { description } if description == wanted)
    })
}

fn count(diagnostics: &[LegacyDiagnosticKind], wanted: fn(&LegacyDiagnosticKind) -> bool) -> usize {
    diagnostics.iter().filter(|kind| wanted(kind)).count()
}

// ---- the loader ------------------------------------------------------------

#[test]
fn an_unlink_beside_a_release_loads_and_beside_any_other_custody_mode_refuses() {
    let load = |clauses: &str| {
        load_resource_contract_sources(
            &[(
                "d.qcore".to_owned(),
                format!(
                    "module contract;\nextern fn d(e: ptr<mut, u8>) -> unit\ncontract {{ requires resource_contract_v1; {clauses} ensures normal resource_return_none(); }};\n"
                ),
            )]
            .into_iter()
            .collect(),
        )
    };
    for accepted in [
        "ensures normal resource_unpublished(e); ensures normal resource_released(e, nonnull);",
        "ensures normal resource_released(e, nullable); ensures normal resource_unpublished(e);",
        "ensures normal resource_borrowed(e); ensures normal resource_unpublished(e);",
    ] {
        assert!(load(accepted).is_ok(), "{accepted}");
    }
    for refused in [
        "ensures normal resource_unpublished(e); ensures normal resource_consumed(e, nonnull);",
        "ensures normal resource_unpublished(e); ensures normal resource_owned(e, nonnull);",
        // a publication still needs a borrowed entry
        "ensures normal resource_published(e, e); ensures normal resource_released(e, nonnull);",
    ] {
        assert!(load(refused).is_err(), "{refused}");
    }
    let refused_publication = load_resource_contract_sources(
        &[(
            "p.qcore".to_owned(),
            "module contract;\nextern fn p(e: ptr<mut, u8>, h: ptr<mut, u8>) -> unit\ncontract { requires resource_contract_v1; ensures normal resource_released(e, nonnull); ensures normal resource_borrowed(h); ensures normal resource_published(e, h); ensures normal resource_return_none(); };\n".to_owned(),
        )]
        .into_iter()
        .collect(),
    );
    assert!(refused_publication.is_err());
}

// ---- accepting controls ----------------------------------------------------

#[test]
fn destroying_each_element_of_a_safe_iteration_over_a_list_formal_is_safe() {
    if !clang_available() {
        return;
    }
    for body in [
        // sched_thread_cleanup (mk_scheduler.c:614-632), verbatim but for names
        "static int subject(struct queue *sched, struct mk_list *list) { int c = 0;
         struct mk_list *tmp; struct mk_list *head; struct item *mth; (void) sched;
         mk_list_foreach_safe(head, tmp, list) { mth = mk_list_entry(head, struct item, _head);
         item_destroy(mth); c++; } return c; }",
        // a member of a formal's record
        "int subject(struct queue *q) { struct mk_list *it, *tmp; struct item *e;
         mk_list_foreach_safe(it, tmp, &q->items) { e = mk_list_entry(it, struct item, _head);
         item_destroy(e); } return 0; }",
        // mk_sched_threads_destroy_conn (mk_scheduler.c:657-673): a condition
        // read through the element decides whether it is destroyed
        "int subject(struct queue *q, void *conn) { int c = 0; struct mk_list *it, *tmp; struct item *e;
         mk_list_foreach_safe(it, tmp, &q->items) { e = mk_list_entry(it, struct item, _head);
         if (e->session->conn == conn) { item_destroy(e); c++; } } return c; }",
        // a first-element lookup
        "void subject(struct queue *q) { struct item *e;
         e = mk_list_entry_first(&q->items, struct item, _head); item_destroy(e); }",
    ] {
        let (status, diagnostics) = analyze(body);
        assert_eq!(status, AnalysisStatus::Safe, "{body}\n{diagnostics:?}");
    }
}

#[test]
fn destroying_an_object_this_body_published_is_safe() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) = analyze(
        "void subject(struct queue *q) { struct item *e = q_alloc();
         mk_list_add(&e->_head, &q->items); item_destroy(e); }",
    );
    assert_eq!(status, AnalysisStatus::Safe, "{diagnostics:?}");
}

// ---- refusing controls -----------------------------------------------------

#[test]
fn a_destroyed_element_is_ended_custody() {
    if !clang_available() {
        return;
    }
    // A use after the destroyer ended it: the released binding's read refuses,
    // exactly as after `mk_list_del(&e->_head); q_free(e);`.
    for body in [
        "void subject(struct queue *q) { struct item *e;
         e = mk_list_entry_first(&q->items, struct item, _head); item_destroy(e); q_use(e); }",
        "void subject(struct queue *q) { struct item *e;
         e = mk_list_entry_first(&q->items, struct item, _head);
         mk_list_del(&e->_head); q_free(e); q_use(e); }",
    ] {
        let (status, diagnostics) = analyze(body);
        assert_ne!(status, AnalysisStatus::Safe, "{body}\n{diagnostics:?}");
    }
    // A release after the destroyer ended it is a double release.
    let (status, diagnostics) = analyze(
        "void subject(struct queue *q) { struct item *e;
         e = mk_list_entry_first(&q->items, struct item, _head); item_destroy(e); q_free(e); }",
    );
    assert_eq!(status, AnalysisStatus::Unsafe, "{diagnostics:?}");
    assert_eq!(
        count(&diagnostics, |kind| matches!(
            kind,
            LegacyDiagnosticKind::DoubleRelease { .. }
        )),
        1,
        "{diagnostics:?}"
    );
    // So is a second destroyer of the same element: the second unlink finds
    // nothing published and refuses.
    let (status, diagnostics) = analyze(
        "void subject(struct queue *q) { struct item *e;
         e = mk_list_entry_first(&q->items, struct item, _head); item_destroy(e); item_destroy(e); }",
    );
    assert_ne!(status, AnalysisStatus::Safe, "{diagnostics:?}");
    assert!(has(&diagnostics, NOT_PUBLISHED), "{diagnostics:?}");
}

#[test]
fn an_element_unlinked_before_its_destroyer_is_refused() {
    if !clang_available() {
        return;
    }
    // The destroyer unlinks again: after the first unlink the element is
    // this body's custody, published nowhere, so the second unlink refuses.
    let (status, diagnostics) = analyze(
        "void subject(struct queue *q) { struct item *e;
         e = mk_list_entry_first(&q->items, struct item, _head);
         mk_list_del(&e->_head); item_destroy(e); }",
    );
    assert_eq!(status, AnalysisStatus::Unknown, "{diagnostics:?}");
    assert!(has(&diagnostics, NOT_PUBLISHED), "{diagnostics:?}");
}

#[test]
fn every_entry_rule_10f_does_not_admit_keeps_the_unlink_refusal() {
    if !clang_available() {
        return;
    }
    // Card LIST-ENTRY-RELEASE-IDENTITY (gateway rule 43) admits two entries
    // this list refused: a formal's own object and a body acquisition never
    // published, each one live identity the path holds and never unlinked
    // (`tests/element_release_summary.rs`, `held-acquisition-destroyed` and
    // `formal-handed-to-destroyer-caller`, with rule 43 off as the control).
    for body in [
        // a local list object
        "void subject(void) { struct mk_list *it, *tmp; struct item *e; struct queue local;
         mk_list_foreach_safe(it, tmp, &local.items) { e = mk_list_entry(it, struct item, _head);
         item_destroy(e); } }",
        // a rebound formal
        "void subject(struct queue *q, struct queue *r) { struct mk_list *it, *tmp; struct item *e;
         q = r;
         mk_list_foreach_safe(it, tmp, &q->items) { e = mk_list_entry(it, struct item, _head);
         item_destroy(e); } }",
        // an element name written another way
        "void subject(struct queue *q) { struct mk_list *it, *tmp; struct item *e = NULL;
         mk_list_foreach_safe(it, tmp, &q->items) { e = mk_list_entry(it, struct item, _head);
         item_destroy(e); } }",
    ] {
        let (status, diagnostics) = analyze(body);
        assert_ne!(status, AnalysisStatus::Safe, "{body}\n{diagnostics:?}");
        assert!(has(&diagnostics, NOT_PUBLISHED), "{body}\n{diagnostics:?}");
        assert_eq!(
            count(&diagnostics, |kind| matches!(
                kind,
                LegacyDiagnosticKind::Leak { .. }
            )),
            0,
            "{body}\n{diagnostics:?}"
        );
    }
}

#[test]
fn a_plain_iteration_destroying_its_current_element_stays_refused() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) = analyze(
        "void subject(struct queue *q) { struct mk_list *it; struct item *e;
         mk_list_foreach(it, &q->items) { e = mk_list_entry(it, struct item, _head);
         item_destroy(e); } }",
    );
    assert_eq!(status, AnalysisStatus::Unknown, "{diagnostics:?}");
    assert!(has(&diagnostics, PLAIN_REFUSAL), "{diagnostics:?}");
}

#[test]
fn destroying_the_saved_successor_is_refused() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) = analyze(
        "void subject(struct queue *q) { struct mk_list *it, *tmp; struct item *e;
         mk_list_foreach_safe(it, tmp, &q->items) { e = mk_list_entry(tmp, struct item, _head);
         item_destroy(e); } }",
    );
    assert_ne!(status, AnalysisStatus::Safe, "{diagnostics:?}");
}

#[test]
fn a_destroying_body_publishes_no_summary() {
    if !clang_available() {
        return;
    }
    // Rule 10f(3) unchanged: the drain is Safe on its own, but its caller
    // must not read `Borrowed` for the list: its own published object may be
    // among those the drain destroyed.
    let unit = "static int drain(struct mk_list *list) { struct mk_list *it, *tmp; struct item *e;
         mk_list_foreach_safe(it, tmp, list) { e = mk_list_entry(it, struct item, _head);
         item_destroy(e); } return 0; }
         void subject(struct queue *q) { struct item *x = q_alloc();
         mk_list_add(&x->_head, &q->items); drain(&q->items); mk_list_del(&x->_head); q_free(x); }";
    let (status, diagnostics) = analyze_entry(unit, "drain");
    assert_eq!(status, AnalysisStatus::Safe, "{diagnostics:?}");
    let (status, diagnostics) = analyze(unit);
    assert_eq!(status, AnalysisStatus::Unknown, "{diagnostics:?}");
    // Card LIST-ENTRY-RELEASE-IDENTITY (gateway rule 43) amends rule 10f(3):
    // the drain publishes `ElementsReleased` at `list`, the caller forgets
    // `x`, which it had published, and the later unlink refuses on `x`.
    assert!(
        diagnostics.iter().any(|kind| matches!(
            kind,
            LegacyDiagnosticKind::UnknownAlias { variable } if variable == "x"
        )) || diagnostics.iter().any(|kind| matches!(
            kind,
            LegacyDiagnosticKind::Unsupported { description } if description.contains("drain")
        )),
        "{diagnostics:?}"
    );
}

// ---- the switch (card RULE34-PILOT) ----------------------------------------
//
// Rule 34 is asked at the transfer position from its own package. The switch
// is read once per process, so each switched case runs this binary again as a
// child with `QCORE_GATEWAY_RULES_OFF` set (`child_*` tests do nothing unless
// `ELEMENT_DESTROYER_CHILD` names them).

const CHILD: &str = "ELEMENT_DESTROYER_CHILD";

/// The refusal of a destroyer no transfer rule orders.
const UNORDERED: &str = "element destroyer `item_destroy`: no transfer rule orders its unlink \
                         before its release (gateway rule 34 is off)";

/// Every fixture above that calls the destroyer, with its verdict with rule
/// 34 on.
const DESTROYING: &[(&str, AnalysisStatus)] = &[
    (
        "static int subject(struct queue *sched, struct mk_list *list) { int c = 0;
         struct mk_list *tmp; struct mk_list *head; struct item *mth; (void) sched;
         mk_list_foreach_safe(head, tmp, list) { mth = mk_list_entry(head, struct item, _head);
         item_destroy(mth); c++; } return c; }",
        AnalysisStatus::Safe,
    ),
    (
        "int subject(struct queue *q, void *conn) { int c = 0; struct mk_list *it, *tmp; struct item *e;
         mk_list_foreach_safe(it, tmp, &q->items) { e = mk_list_entry(it, struct item, _head);
         if (e->session->conn == conn) { item_destroy(e); c++; } } return c; }",
        AnalysisStatus::Safe,
    ),
    (
        "void subject(struct queue *q) { struct item *e;
         e = mk_list_entry_first(&q->items, struct item, _head); item_destroy(e); }",
        AnalysisStatus::Safe,
    ),
    (
        "void subject(struct queue *q) { struct item *e = q_alloc();
         mk_list_add(&e->_head, &q->items); item_destroy(e); }",
        AnalysisStatus::Safe,
    ),
    (
        "void subject(struct queue *q) { struct item *e;
         e = mk_list_entry_first(&q->items, struct item, _head); item_destroy(e); q_free(e); }",
        AnalysisStatus::Unsafe,
    ),
    (
        "void subject(struct queue *q) { struct item *e;
         e = mk_list_entry_first(&q->items, struct item, _head);
         mk_list_del(&e->_head); item_destroy(e); }",
        AnalysisStatus::Unknown,
    ),
    // Unknown before card LIST-ENTRY-RELEASE-IDENTITY; rule 43's held arm.
    (
        "void subject(void) { struct item *e = q_alloc(); item_destroy(e); }",
        AnalysisStatus::Safe,
    ),
    (
        "void subject(struct queue *q) { struct mk_list *it; struct item *e;
         mk_list_foreach(it, &q->items) { e = mk_list_entry(it, struct item, _head);
         item_destroy(e); } }",
        AnalysisStatus::Unknown,
    ),
];

/// A fixture with no destroyer: rule 10f's unlink, then a release.
const UNLINK_THEN_FREE: &str = "void subject(struct queue *q) { struct item *e;
         e = mk_list_entry_first(&q->items, struct item, _head);
         mk_list_del(&e->_head); q_free(e); }";

fn run_child(test: &str, list: &str) {
    let status = Command::new(std::env::current_exe().expect("the test binary"))
        .args([test, "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD, test)
        .env(gateway_rules::RULES_OFF_VARIABLE, list)
        .status()
        .expect("the child test runs");
    assert!(status.success(), "{test} with {list:?}: {status}");
}

fn is_child(test: &str) -> bool {
    std::env::var(CHILD).is_ok_and(|named| named == test)
}

#[test]
fn with_rule_34_on_every_destroying_fixture_keeps_its_verdict() {
    if std::env::var_os(CHILD).is_some() || !clang_available() {
        return;
    }
    assert!(gateway_rules::effective_off().is_empty());
    for (body, on) in DESTROYING {
        let (status, diagnostics) = analyze(body);
        assert_eq!(status, *on, "{body}\n{diagnostics:?}");
        assert!(!has(&diagnostics, UNORDERED), "{body}\n{diagnostics:?}");
    }
    assert_eq!(analyze(UNLINK_THEN_FREE).0, AnalysisStatus::Safe);
}

#[test]
fn switching_rule_34_off_refuses_every_destroyer_and_never_widens() {
    if std::env::var_os(CHILD).is_some() || !clang_available() {
        return;
    }
    run_child("child_rule_34_off", "34");
}

#[test]
fn child_rule_34_off() {
    if !is_child("child_rule_34_off") {
        return;
    }
    assert_eq!(
        gateway_rules::effective_off()
            .into_iter()
            .collect::<Vec<_>>(),
        ["34"]
    );
    for (body, on) in DESTROYING {
        // The call is refused before any of its effects apply: Unknown, as
        // an uncontracted call was before rule 34, never Safe, and never an
        // accusation (no leak of what the call was going to release, no
        // double release of what it was going to end). A body whose
        // iteration rule 5l already refuses at the lowering (the plain
        // iteration) never reaches the transfer and keeps that refusal.
        let (status, diagnostics) = analyze(body);
        assert_eq!(status, AnalysisStatus::Unknown, "{body}\n{diagnostics:?}");
        if *on != AnalysisStatus::Unknown {
            assert!(has(&diagnostics, UNORDERED), "{body}\n{diagnostics:?}");
        }
        if !has(&diagnostics, UNORDERED) {
            assert!(has(&diagnostics, PLAIN_REFUSAL), "{body}\n{diagnostics:?}");
        }
        assert_eq!(
            count(&diagnostics, |kind| matches!(
                kind,
                LegacyDiagnosticKind::Leak { .. } | LegacyDiagnosticKind::DoubleRelease { .. }
            )),
            0,
            "{body}\n{diagnostics:?}"
        );
    }
    // A call without a destroyer is not rule 34's: unchanged.
    let (status, diagnostics) = analyze(UNLINK_THEN_FREE);
    assert_eq!(status, AnalysisStatus::Safe, "{diagnostics:?}");
    // Rule 10f(3)'s summary gate is upstream of the transfer and unchanged;
    // the drain itself is now refused at its destroyer.
    let unit = "static int drain(struct mk_list *list) { struct mk_list *it, *tmp; struct item *e;
         mk_list_foreach_safe(it, tmp, list) { e = mk_list_entry(it, struct item, _head);
         item_destroy(e); } return 0; }
         void subject(struct queue *q) { struct item *x = q_alloc();
         mk_list_add(&x->_head, &q->items); drain(&q->items); mk_list_del(&x->_head); q_free(x); }";
    let (status, diagnostics) = analyze_entry(unit, "drain");
    assert_eq!(status, AnalysisStatus::Unknown, "{diagnostics:?}");
    assert_ne!(analyze(unit).0, AnalysisStatus::Safe);
}
