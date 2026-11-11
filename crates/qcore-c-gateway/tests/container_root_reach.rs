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

//! Card **W45-CONTAINER-ROOT-REACH**: static containers and the foreign unlink
//! (gateway constraint 10d, second paragraph).
//!
//! The prelude spells the Monkey list macros as the pinned headers do, and
//! `mk_list_add` / `mk_list_del` carry the declared container clauses of
//! `monkey/intrusive_list.qv`, so every fixture reaches the gateway in the
//! preprocessed form a production unit does.

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
struct queue *q_get(void);
struct item { int size; void *payload; struct mk_list _head; };
struct queue { struct mk_list items; struct mk_list done; int total; };
struct holder { struct queue *queue; };
"#;

const LIST: &str = "module contract;
extern fn mk_list_add(entry: ptr<mut, u8>, head: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; ensures normal resource_borrowed(entry); ensures normal resource_borrowed(head); ensures normal resource_published(entry, head); ensures normal resource_return_none(); };
extern fn mk_list_del(entry: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; ensures normal resource_borrowed(entry); ensures normal resource_unpublished(entry); ensures normal resource_return_none(); };
extern fn q_use(p: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; ensures normal resource_borrowed(p); ensures normal resource_return_none(); };
";

const NOT_PUBLISHED: &str = "unlink by `mk_list_del` of `e`, which this function has not published";
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

fn has_prefix(diagnostics: &[LegacyDiagnosticKind], wanted: &str) -> bool {
    diagnostics.iter().any(|kind| {
        matches!(kind, LegacyDiagnosticKind::Unsupported { description } if description.starts_with(wanted))
    })
}

fn leaks(diagnostics: &[LegacyDiagnosticKind]) -> usize {
    diagnostics
        .iter()
        .filter(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. }))
        .count()
}

// ---- accepting controls ---------------------------------------------------

#[test]
fn a_publication_into_a_static_list_discharges() {
    if !clang_available() {
        return;
    }
    for body in [
        "static struct mk_list registry;
         void subject(void) { struct item *e = q_alloc(); mk_list_add(&e->_head, &registry); }",
        "struct mk_list registry;
         void subject(void) { struct item *e = q_alloc(); mk_list_add(&e->_head, &registry); }",
        "void subject(void) { static struct mk_list local_static; struct item *e = q_alloc();
         mk_list_add(&e->_head, &local_static); }",
    ] {
        let (status, diagnostics) = analyze(body);
        assert_eq!(status, AnalysisStatus::Safe, "{body}\n{diagnostics:?}");
    }
}

#[test]
fn a_member_of_a_static_record_reaches_the_container_rule() {
    if !clang_available() {
        return;
    }
    // `&G.F` reaches the container rule only once the call's member-address
    // gate reads a static record's member. Rule 10n's typed address fallback
    // does not (its root must be never bound, and since rule 10o the closure
    // counts the address taken of `global_queue`; the class is
    // W47-JOIN-AST-CONSERVATIVE-CLASSES). CG15 flip (rule 11h): the address
    // of a member of a static-duration record is a pointer with no identity,
    // admitted at the `Borrowed` head position, so the publication reaches the
    // container rule and discharges into the static list exactly as
    // `&registry` does above. BEFORE: refused (Unknown), nothing accused.
    let (status, diagnostics) = analyze(
        "struct queue global_queue;
         void subject(void) { struct item *e = q_alloc(); mk_list_add(&e->_head, &global_queue.items); }",
    );
    assert_eq!(status, AnalysisStatus::Safe, "{diagnostics:?}");
    assert_eq!(leaks(&diagnostics), 0, "{diagnostics:?}");
    // A local record's member is not static storage: never `Safe`, nothing
    // accused.
    let (status, diagnostics) = analyze(
        "void subject(void) { struct queue local; struct item *e = q_alloc();
         mk_list_add(&e->_head, &local.items); }",
    );
    assert_ne!(status, AnalysisStatus::Safe, "{diagnostics:?}");
    assert_eq!(leaks(&diagnostics), 0, "{diagnostics:?}");
}

#[test]
fn a_publication_into_a_caller_member_chain_discharges() {
    if !clang_available() {
        return;
    }
    for body in [
        "void subject(struct queue *q) { struct item *e = q_alloc();
         mk_list_add(&e->_head, &q->items); }",
        "void subject(struct holder *h) { struct queue *q = h->queue; struct item *e = q_alloc();
         mk_list_add(&e->_head, &q->done); }",
    ] {
        let (status, diagnostics) = analyze(body);
        assert_eq!(status, AnalysisStatus::Safe, "{body}\n{diagnostics:?}");
    }
}

#[test]
fn a_foreign_unlink_then_free_is_safe() {
    if !clang_available() {
        return;
    }
    for body in [
        // the current element of a safe iteration over a formal's list
        "void subject(struct queue *q) { struct mk_list *it, *tmp; struct item *e;
         mk_list_foreach_safe(it, tmp, &q->items) { e = mk_list_entry(it, struct item, _head);
         mk_list_del(&e->_head); q_free(e); } }",
        // a list pointer formal
        "void subject(struct mk_list *list) { struct mk_list *it, *tmp; struct item *e;
         mk_list_foreach_safe(it, tmp, list) { e = mk_list_entry(it, struct item, _head);
         mk_list_del(&e->_head); q_free(e); } }",
        // a first-element lookup
        "void subject(struct queue *q) { struct item *e;
         e = mk_list_entry_first(&q->items, struct item, _head);
         mk_list_del(&e->_head); q_free(e); }",
        // two iterations over two lists of the same formal
        "void subject(struct queue *q) { struct mk_list *it, *tmp; struct item *e;
         mk_list_foreach_safe(it, tmp, &q->items) { e = mk_list_entry(it, struct item, _head);
         mk_list_del(&e->_head); q_free(e); }
         mk_list_foreach_safe(it, tmp, &q->done) { e = mk_list_entry(it, struct item, _head);
         mk_list_del(&e->_head); q_free(e); } }",
    ] {
        let (status, diagnostics) = analyze(body);
        assert_eq!(status, AnalysisStatus::Safe, "{body}\n{diagnostics:?}");
    }
}

#[test]
fn a_foreign_unlink_then_republication_is_safe() {
    if !clang_available() {
        return;
    }
    // flb_output_task_queue_flush_one's move from `pending` to `in_progress`.
    let (status, diagnostics) = analyze(
        "void subject(struct queue *q) { struct item *e;
         e = mk_list_entry_first(&q->items, struct item, _head);
         mk_list_del(&e->_head); mk_list_add(&e->_head, &q->done); }",
    );
    assert_eq!(status, AnalysisStatus::Safe, "{diagnostics:?}");
}

// ---- refusing controls ----------------------------------------------------

#[test]
fn a_publication_into_a_local_only_container_is_refused() {
    if !clang_available() {
        return;
    }
    for body in [
        "void subject(void) { struct mk_list local; struct item *e = q_alloc();
         mk_list_add(&e->_head, &local); }",
        // a list object inside the body's own acquisition
        "void subject(void) { struct queue *q = q_alloc(); struct item *e = q_alloc();
         mk_list_add(&e->_head, &q->items); q_free(q); }",
    ] {
        let (status, diagnostics) = analyze(body);
        assert_eq!(status, AnalysisStatus::Unknown, "{body}\n{diagnostics:?}");
        assert!(
            has_prefix(
                &diagnostics,
                "publication by `mk_list_add` into a container"
            ),
            "{body}\n{diagnostics:?}"
        );
        assert_eq!(leaks(&diagnostics), 0, "{body}\n{diagnostics:?}");
    }
    // A pointer-valued global is not static storage for its pointee: the
    // call is refused, nothing discharged, nothing accused.
    let (status, diagnostics) = analyze(
        "struct queue *global_pointer;
         void subject(void) { struct item *e = q_alloc(); mk_list_add(&e->_head, &global_pointer->items); }",
    );
    assert_eq!(status, AnalysisStatus::Unknown, "{diagnostics:?}");
    assert_eq!(leaks(&diagnostics), 0, "{diagnostics:?}");
}

#[test]
fn a_foreign_unlink_then_leak_is_a_leak() {
    if !clang_available() {
        return;
    }
    for body in [
        "void subject(struct queue *q) { struct mk_list *it, *tmp; struct item *e;
         mk_list_foreach_safe(it, tmp, &q->items) { e = mk_list_entry(it, struct item, _head);
         mk_list_del(&e->_head); } }",
        "void subject(struct queue *q, int destroy) { struct item *e;
         e = mk_list_entry_first(&q->items, struct item, _head);
         mk_list_del(&e->_head); if (destroy) { q_free(e); } }",
    ] {
        let (status, diagnostics) = analyze(body);
        assert_eq!(status, AnalysisStatus::Unsafe, "{body}\n{diagnostics:?}");
        assert_eq!(leaks(&diagnostics), 1, "{body}\n{diagnostics:?}");
    }
}

#[test]
fn a_foreign_unlink_then_double_free_is_a_violation() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) = analyze(
        "void subject(struct queue *q) { struct item *e;
         e = mk_list_entry_first(&q->items, struct item, _head);
         mk_list_del(&e->_head); q_free(e); q_free(e); }",
    );
    assert_eq!(status, AnalysisStatus::Unsafe, "{diagnostics:?}");
    assert!(
        diagnostics
            .iter()
            .any(|kind| matches!(kind, LegacyDiagnosticKind::DoubleRelease { .. })),
        "{diagnostics:?}"
    );
}

#[test]
fn releasing_a_foreign_element_after_republishing_it_is_accused() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) = analyze(
        "void subject(struct queue *q) { struct item *e;
         e = mk_list_entry_first(&q->items, struct item, _head);
         mk_list_del(&e->_head); mk_list_add(&e->_head, &q->done); q_free(e); }",
    );
    assert_eq!(status, AnalysisStatus::Unsafe, "{diagnostics:?}");
    assert!(
        diagnostics
            .iter()
            .any(|kind| matches!(kind, LegacyDiagnosticKind::ReleaseAfterEscape { .. })),
        "{diagnostics:?}"
    );
}

#[test]
fn unlinking_from_a_container_the_function_cannot_modify_is_refused() {
    if !clang_available() {
        return;
    }
    for body in [
        // a container a callee returned (the TLS lists of Monkey)
        "void subject(void) { struct mk_list *it, *tmp; struct item *e; struct queue *q = q_get();
         mk_list_foreach_safe(it, tmp, &q->items) { e = mk_list_entry(it, struct item, _head);
         mk_list_del(&e->_head); q_free(e); } }",
        // a local list object
        "void subject(void) { struct mk_list *it, *tmp; struct item *e; struct queue local;
         mk_list_foreach_safe(it, tmp, &local.items) { e = mk_list_entry(it, struct item, _head);
         mk_list_del(&e->_head); q_free(e); } }",
        // a rebound formal
        "void subject(struct queue *q, struct queue *r) { struct mk_list *it, *tmp; struct item *e;
         q = r;
         mk_list_foreach_safe(it, tmp, &q->items) { e = mk_list_entry(it, struct item, _head);
         mk_list_del(&e->_head); q_free(e); } }",
        // a static list: the container rule admits it, but rule 5l reads no
        // static head yet, so the element is never bound and stays refused
        "static struct mk_list registry;
         void subject(void) { struct mk_list *it, *tmp; struct item *e;
         mk_list_foreach_safe(it, tmp, &registry) { e = mk_list_entry(it, struct item, _head);
         mk_list_del(&e->_head); q_free(e); } }",
        // an element name also written another way
        "void subject(struct queue *q) { struct mk_list *it, *tmp; struct item *e = NULL;
         mk_list_foreach_safe(it, tmp, &q->items) { e = mk_list_entry(it, struct item, _head);
         mk_list_del(&e->_head); q_free(e); } }",
    ] {
        let (status, diagnostics) = analyze(body);
        assert_eq!(status, AnalysisStatus::Unknown, "{body}\n{diagnostics:?}");
        assert!(has(&diagnostics, NOT_PUBLISHED), "{body}\n{diagnostics:?}");
        assert_eq!(leaks(&diagnostics), 0, "{body}\n{diagnostics:?}");
    }
}

#[test]
fn a_plain_iteration_unlinking_its_current_element_stays_refused() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) = analyze(
        "void subject(struct queue *q) { struct mk_list *it; struct item *e;
         mk_list_foreach(it, &q->items) { e = mk_list_entry(it, struct item, _head);
         mk_list_del(&e->_head); q_free(e); } }",
    );
    assert_eq!(status, AnalysisStatus::Unknown, "{diagnostics:?}");
    assert!(has(&diagnostics, PLAIN_REFUSAL), "{diagnostics:?}");
}

#[test]
fn a_formal_whose_container_is_unnamed_stays_refused() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) =
        analyze("void subject(struct item *e) { mk_list_del(&e->_head); q_free(e); }");
    assert_ne!(status, AnalysisStatus::Safe, "{diagnostics:?}");
    assert!(has(&diagnostics, NOT_PUBLISHED), "{diagnostics:?}");
}

#[test]
fn a_foreign_unlink_publishes_no_summary() {
    if !clang_available() {
        return;
    }
    // The helper is Safe on its own, but its callers must not read `Borrowed`
    // for the queue: it released what the queue held.
    let unit = "static void drain(struct queue *q) { struct mk_list *it, *tmp; struct item *e;
         mk_list_foreach_safe(it, tmp, &q->items) { e = mk_list_entry(it, struct item, _head);
         mk_list_del(&e->_head); q_free(e); } }
         void subject(struct queue *q) { struct item *x = q_alloc();
         mk_list_add(&x->_head, &q->items); drain(q); mk_list_del(&x->_head); q_free(x); }";
    let (status, diagnostics) = analyze_entry(unit, "drain");
    assert_eq!(status, AnalysisStatus::Safe, "{diagnostics:?}");
    let (status, diagnostics) = analyze(unit);
    assert_eq!(status, AnalysisStatus::Unknown, "{diagnostics:?}");
    assert!(
        diagnostics.iter().any(|kind| matches!(
            kind,
            LegacyDiagnosticKind::Unsupported { description } if description.contains("drain")
        )),
        "{diagnostics:?}"
    );
}

#[test]
fn a_foreign_element_unlinked_twice_is_refused() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) = analyze(
        "void subject(struct queue *q) { struct item *e;
         e = mk_list_entry_first(&q->items, struct item, _head);
         mk_list_del(&e->_head); mk_list_del(&e->_head); q_free(e); }",
    );
    assert_eq!(status, AnalysisStatus::Unknown, "{diagnostics:?}");
    assert!(has(&diagnostics, NOT_PUBLISHED), "{diagnostics:?}");
}
