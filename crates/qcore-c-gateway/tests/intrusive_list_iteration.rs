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

//! Card **W45-FB-BACKLOG-CUSTODY**: intrusive-list iteration, element identity
//! and checked specifications (gateway constraint 5l, contract spec §C06.3).
//!
//! The prelude spells the Monkey/CFL list macros exactly as the pinned headers
//! do, so every fixture reaches the gateway in the preprocessed form a
//! production unit does. `q_unlink` is a synthetic, custody-neutral unlink
//! (`resource_borrowed`), standing for any declared operation that detaches a
//! node; it is not `mk_list_del`, whose custody is card W45-FB-RETRY-LIFETIME's.

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
void *q_alloc(void); void q_free(void *); void q_use(void *);
void q_unlink(struct mk_list *entry); void q_link(struct mk_list *entry, struct mk_list *head);
struct item { int size; void *payload; struct mk_list _head; };
struct queue { struct mk_list items; int total; };
struct holder { struct queue *queue; };
"#;

const PLAIN_REFUSAL: &str = "the current element of an intrusive-list iteration that does not \
                             save its successor is unlinked or released";
const SAFE_REFUSAL: &str =
    "the saved successor of an intrusive-list iteration is unlinked or released";
const INDIRECT_REFUSAL: &str = "indirect memory access is outside the resource projection";
const FOR_REFUSAL: &str = "unmodeled call, aggregate or indirect access in for-loop condition";
const BRACED_REFUSAL: &str = "braced initializer, expression, or declaration is not modeled";

const FIXTURE: &str = "module contract;
extern fn q_use(p: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; ensures normal resource_borrowed(p); ensures normal resource_return_none(); };
extern fn q_unlink(entry: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; ensures normal resource_borrowed(entry); ensures normal resource_return_none(); };
extern fn q_link(entry: ptr<mut, u8>, head: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; ensures normal resource_borrowed(entry); ensures normal resource_borrowed(head); ensures normal resource_return_none(); };
";

fn contracts(extra: &str) -> ResourceContracts {
    let mut sources = vec![
        (
            "q_base.qcore".to_owned(),
            include_str!("fixtures/contracts/q_base.qv").to_owned(),
        ),
        ("list_fixture.qcore".to_owned(), FIXTURE.to_owned()),
    ];
    if !extra.is_empty() {
        sources.push(("specification.qcore".to_owned(), extra.to_owned()));
    }
    load_resource_contract_sources(&sources.into_iter().collect()).unwrap()
}

fn analyze_with(
    body: &str,
    entry: &str,
    extra: &str,
) -> (AnalysisStatus, Vec<LegacyDiagnosticKind>) {
    let source = format!("{PRELUDE}\n{body}");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            dialect: CCompilerDialect::Gnu17,
            ..Default::default()
        },
        entry,
        &contracts(extra),
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
    analyze_with(body, "subject", "")
}

fn clang_available() -> bool {
    qcore_c_gateway::clang_command().is_ok()
}

fn has(diagnostics: &[LegacyDiagnosticKind], wanted: &str) -> bool {
    diagnostics.iter().any(|kind| {
        matches!(kind, LegacyDiagnosticKind::Unsupported { description } if description == wanted)
    })
}

// ---- accepting controls --------------------------------------------------

#[test]
fn a_plain_iteration_that_only_reads_its_elements_is_safe() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) = analyze(
        "int subject(struct queue *q) { struct mk_list *it; struct item *e; int n = 0;
         mk_list_foreach(it, &q->items) { e = mk_list_entry(it, struct item, _head); n += e->size; q_use(e); }
         return n; }",
    );
    assert_eq!(status, AnalysisStatus::Safe, "{diagnostics:?}");
}

#[test]
fn a_head_reached_through_a_member_chain_is_read_with_its_obligation() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) = analyze(
        "void subject(struct holder *h) { struct mk_list *it; struct item *e;
         mk_list_foreach(it, &h->queue->items) { e = mk_list_entry(it, struct item, _head); q_use(e); } }",
    );
    assert_eq!(status, AnalysisStatus::Safe, "{diagnostics:?}");
}

#[test]
fn a_safe_iteration_may_unlink_its_current_element() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) = analyze(
        "void subject(struct queue *q) { struct mk_list *it; struct mk_list *tmp; struct item *e;
         mk_list_foreach_safe(it, tmp, &q->items) { e = mk_list_entry(it, struct item, _head); q_unlink(&e->_head); } }",
    );
    assert_eq!(status, AnalysisStatus::Safe, "{diagnostics:?}");
}

#[test]
fn a_safe_iteration_that_unlinks_and_frees_refuses_only_on_the_elements_custody() {
    if !clang_available() {
        return;
    }
    // The iteration admits the unlink and the release; what stays open is who
    // owns an element of a borrowed container, which version 1 cannot state.
    let (status, diagnostics) = analyze(
        "void subject(struct queue *q) { struct mk_list *it; struct mk_list *tmp; struct item *e;
         mk_list_foreach_safe(it, tmp, &q->items) { e = mk_list_entry(it, struct item, _head); q_unlink(&e->_head); q_free(e); } }",
    );
    assert_eq!(status, AnalysisStatus::Unknown, "{diagnostics:?}");
    assert!(!has(&diagnostics, PLAIN_REFUSAL) && !has(&diagnostics, SAFE_REFUSAL));
    assert!(!has(&diagnostics, FOR_REFUSAL) && !has(&diagnostics, BRACED_REFUSAL));
    assert!(diagnostics.iter().all(|kind| matches!(
        kind,
        LegacyDiagnosticKind::UnknownAlias { variable } if variable == "e"
    )));
}

#[test]
fn a_body_acquisition_inside_an_iteration_keeps_its_leak() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) = analyze(
        "void subject(struct queue *q) { struct mk_list *it; void *p;
         mk_list_foreach(it, &q->items) { p = q_alloc(); } }",
    );
    assert_eq!(status, AnalysisStatus::Unsafe, "{diagnostics:?}");
    assert!(
        diagnostics
            .iter()
            .any(|kind| matches!(kind, LegacyDiagnosticKind::Leak { .. }))
    );
}

#[test]
fn the_first_element_of_a_head_is_an_element_read() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) = analyze(
        "void subject(struct queue *q) { struct item *e;
         e = mk_list_entry_first(&q->items, struct item, _head); q_use(e); }",
    );
    assert_eq!(status, AnalysisStatus::Safe, "{diagnostics:?}");
}

#[test]
fn removal_before_close_of_a_tracked_resource_is_safe() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) = analyze(
        "void subject(struct queue *q) { struct mk_list *it; struct mk_list *tmp; struct item *e; void *chunk = q_alloc();
         mk_list_foreach_safe(it, tmp, &q->items) { e = mk_list_entry(it, struct item, _head); q_unlink(&e->_head); }
         q_free(chunk); }",
    );
    assert_eq!(status, AnalysisStatus::Safe, "{diagnostics:?}");
}

// ---- refusing controls ---------------------------------------------------

#[test]
fn unlinking_the_current_element_of_a_plain_iteration_is_refused() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) = analyze(
        "void subject(struct queue *q) { struct mk_list *it; struct item *e;
         mk_list_foreach(it, &q->items) { e = mk_list_entry(it, struct item, _head); q_unlink(&e->_head); } }",
    );
    assert_eq!(status, AnalysisStatus::Unknown, "{diagnostics:?}");
    assert!(has(&diagnostics, PLAIN_REFUSAL), "{diagnostics:?}");
}

#[test]
fn unlinking_the_cursor_of_a_plain_iteration_is_refused() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) = analyze(
        "void subject(struct queue *q) { struct mk_list *it;
         mk_list_foreach(it, &q->items) { q_unlink(it); } }",
    );
    assert_eq!(status, AnalysisStatus::Unknown, "{diagnostics:?}");
    assert!(has(&diagnostics, PLAIN_REFUSAL), "{diagnostics:?}");
}

#[test]
fn freeing_the_current_element_of_a_plain_iteration_is_refused() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) = analyze(
        "void subject(struct queue *q) { struct mk_list *it; struct item *e;
         mk_list_foreach(it, &q->items) { e = mk_list_entry(it, struct item, _head); q_free(e); } }",
    );
    assert_eq!(status, AnalysisStatus::Unknown, "{diagnostics:?}");
    assert!(has(&diagnostics, PLAIN_REFUSAL), "{diagnostics:?}");
}

#[test]
fn unlinking_the_saved_successor_of_a_safe_iteration_is_refused() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) = analyze(
        "void subject(struct queue *q) { struct mk_list *it; struct mk_list *tmp;
         mk_list_foreach_safe(it, tmp, &q->items) { q_unlink(tmp); } }",
    );
    assert_eq!(status, AnalysisStatus::Unknown, "{diagnostics:?}");
    assert!(has(&diagnostics, SAFE_REFUSAL), "{diagnostics:?}");
}

#[test]
fn a_body_that_writes_its_cursor_keeps_the_old_refusals() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) = analyze(
        "void subject(struct queue *q) { struct mk_list *it;
         mk_list_foreach(it, &q->items) { it = it->next; } }",
    );
    assert_eq!(status, AnalysisStatus::Unknown, "{diagnostics:?}");
    // Re-pinned deliberately by W47-ACCESS-PATH-POSITION-RESIDUALS (rule
    // 10n): the iteration route still declines, and the loop condition
    // `it != (&q->items)` it leaves refused is now read by the typed
    // fallback as a comparison with an address (a hop on `q`, no read of
    // `items`); the body's own cursor write keeps the function `Unknown`.
    assert!(!has(&diagnostics, FOR_REFUSAL), "{diagnostics:?}");
    assert!(has(&diagnostics, INDIRECT_REFUSAL), "{diagnostics:?}");
}

#[test]
fn linking_a_tracked_object_into_the_iterated_list_declines_the_iteration() {
    if !clang_available() {
        return;
    }
    // P-ILI-UNTRACKED's guard: the element could be the tracked object.
    let (status, diagnostics) = analyze(
        "void subject(struct queue *q) { struct mk_list *it; struct item *e; struct item *x = q_alloc();
         q_link(&x->_head, &q->items);
         mk_list_foreach(it, &q->items) { e = mk_list_entry(it, struct item, _head); q_use(e); } }",
    );
    assert_ne!(status, AnalysisStatus::Safe, "{diagnostics:?}");
    // Re-pinned deliberately by W47-ACCESS-PATH-POSITION-RESIDUALS (rule
    // 10n): the iteration is still declined; its loop condition, a
    // comparison with `&q->items`, is now read by the typed fallback, and the
    // element read in the body keeps its refusal, so the guard's outcome —
    // never `Safe` — is unchanged.
    assert!(!has(&diagnostics, FOR_REFUSAL), "{diagnostics:?}");
    assert!(
        has(&diagnostics, INDIRECT_REFUSAL) || has(&diagnostics, BRACED_REFUSAL),
        "{diagnostics:?}"
    );
}

#[test]
fn closing_a_chunk_a_wrapper_still_refers_to_is_never_safe() {
    if !clang_available() {
        return;
    }
    // The partial-fan-out shape: a wrapper is linked for the chunk, then the
    // chunk is closed. Version 1 has no reference relation, so the pair is not
    // decided; what this pins is that the shape does not certify.
    let (status, diagnostics) = analyze(
        "struct wrapper { void *chunk; struct mk_list _head; };
         void subject(struct queue *q, void *chunk) { struct wrapper *w = q_alloc();
         if (w == NULL) { return; } w->chunk = chunk; q_link(&w->_head, &q->items); q_free(chunk); }",
    );
    assert_ne!(status, AnalysisStatus::Safe, "{diagnostics:?}");
}

#[test]
fn a_stale_wrapper_read_after_its_close_is_never_safe() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) = analyze(
        "struct wrapper { void *chunk; struct mk_list _head; };
         void subject(struct queue *q) { struct mk_list *it; struct mk_list *tmp; struct wrapper *w;
         mk_list_foreach_safe(it, tmp, &q->items) { w = mk_list_entry(it, struct wrapper, _head); q_free(w->chunk); q_use(w->chunk); } }",
    );
    assert_ne!(status, AnalysisStatus::Safe, "{diagnostics:?}");
}

// ---- checked specifications ----------------------------------------------

const DESTROY_SPEC_NONNULL: &str = "module contract;
extern fn destroy_item(item: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; requires resource_body_checked; ensures normal resource_released(item, nonnull); ensures normal resource_return_none(); };
";

const DESTROY_SPEC_BORROWED: &str = "module contract;
extern fn destroy_item(item: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; requires resource_body_checked; ensures normal resource_borrowed(item); ensures normal resource_return_none(); };
";

const DESTROY_DECLARED_BORROWED: &str = "module contract;
extern fn destroy_item(item: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; ensures normal resource_borrowed(item); ensures normal resource_return_none(); };
";

const DESTROY_UNIT: &str = "static void destroy_item(struct item *item) { q_free(item); }
void caller(void) { struct item *x = q_alloc(); destroy_item(x); }";

#[test]
fn a_specification_its_body_establishes_is_held() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) = analyze_with(DESTROY_UNIT, "destroy_item", DESTROY_SPEC_NONNULL);
    assert_eq!(status, AnalysisStatus::Safe, "{diagnostics:?}");
    let (status, diagnostics) = analyze_with(DESTROY_UNIT, "caller", DESTROY_SPEC_NONNULL);
    assert_eq!(status, AnalysisStatus::Safe, "{diagnostics:?}");
}

#[test]
fn a_specification_its_body_contradicts_is_withdrawn_not_trusted() {
    if !clang_available() {
        return;
    }
    // Declared `Borrowed` as a trusted premise, the caller leaks `x` on the
    // declaration's word; as a checked specification the body refutes it, the
    // declaration is withdrawn, and the caller sees what the body validates.
    let (status, diagnostics) = analyze_with(DESTROY_UNIT, "destroy_item", DESTROY_SPEC_BORROWED);
    assert_eq!(status, AnalysisStatus::Unknown, "{diagnostics:?}");
    assert!(diagnostics.iter().any(|kind| matches!(
        kind,
        LegacyDiagnosticKind::Unsupported { description }
            if description.starts_with("declared specification of `destroy_item` is not established")
    )));
    let (status, diagnostics) = analyze_with(DESTROY_UNIT, "caller", DESTROY_SPEC_BORROWED);
    assert_eq!(status, AnalysisStatus::Safe, "{diagnostics:?}");
    let (status, _) = analyze_with(DESTROY_UNIT, "caller", DESTROY_DECLARED_BORROWED);
    assert_ne!(status, AnalysisStatus::Safe);
}

#[test]
fn a_specification_without_a_body_in_the_unit_is_never_applied() {
    if !clang_available() {
        return;
    }
    let unit = "void destroy_item(struct item *item);
                void caller(void) { struct item *x = q_alloc(); destroy_item(x); }";
    let (status, diagnostics) = analyze_with(unit, "caller", DESTROY_SPEC_NONNULL);
    assert_eq!(status, AnalysisStatus::Unknown, "{diagnostics:?}");
    assert!(
        has(&diagnostics, "call to uncontracted function `destroy_item`"),
        "{diagnostics:?}"
    );
}

#[test]
fn a_recursive_specification_is_withdrawn() {
    if !clang_available() {
        return;
    }
    let unit = "static void destroy_item(struct item *item) { if (item->size > 0) { destroy_item(item); } q_free(item); }";
    let (status, diagnostics) = analyze_with(unit, "destroy_item", DESTROY_SPEC_NONNULL);
    assert_ne!(status, AnalysisStatus::Safe, "{diagnostics:?}");
    assert!(diagnostics.iter().any(|kind| matches!(
        kind,
        LegacyDiagnosticKind::Unsupported { description }
            if description.contains("recursive component")
    )));
}

#[test]
fn a_call_handed_only_the_iterated_head_links_nothing() {
    if !clang_available() {
        return;
    }
    // `mk_list_size(&ctx->backlogs)` beside `mk_list_foreach(it, &ctx->backlogs)`:
    // one actual, the head itself, so the guard does not decline.
    let (_, diagnostics) = analyze(
        "void subject(void *data) { struct queue *q = (struct queue *) data; struct mk_list *it; struct item *e;
         q_use(&q->items);
         mk_list_foreach(it, &q->items) { e = mk_list_entry(it, struct item, _head); q_use(e); } }",
    );
    assert!(!has(&diagnostics, FOR_REFUSAL), "{diagnostics:?}");
    assert!(!has(&diagnostics, BRACED_REFUSAL), "{diagnostics:?}");
}
