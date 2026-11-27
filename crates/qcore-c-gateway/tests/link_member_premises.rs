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

//! Card **LERI-HELD-AND-LINK-MEMBER-PREMISES** (gateway rule 46): rule 43's
//! P-LERI-HELD-LINKS and P-LERI-LINK-MEMBER, checked.
//!
//! An element destroyer names the member its body unlinks
//! (`resource_unpublished(p, MEMBER)`). It takes a foreign element only when
//! every `container_of` binding of the element names that member, and an
//! object the path holds only when this body initialised that member's links
//! itself (a rule 36b self-initialisation covering `next` and `prev`) and no
//! later statement may have written them. Every other entry refuses
//! (`Unknown`), and nothing here accuses.
//!
//! The prelude is `tests/element_release_summary.rs`'s, with a self-link
//! initialiser leaf (`mk_list_init`, the header's own body) and a record
//! with two links (`struct pair`, as `struct flb_tail_file` has `_head` and
//! `_rotate_head`).

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
#define mk_list_foreach_safe(curr, n, head) \
    for (curr = (head)->next, n = curr->next; curr != (head); curr = n, n = curr->next)
#define mk_list_entry( ptr, type, member ) container_of( ptr, type, member )
static void mk_list_init(struct mk_list *list) { list->next = list; list->prev = list; }
static void mk_list_half_init(struct mk_list *list) { list->next = list; }
void mk_list_add(struct mk_list *entry, struct mk_list *head);
void mk_list_del(struct mk_list *entry);
void *q_alloc(void); void q_free(void *); void q_use(void *);
struct item { int close; struct mk_list _head; void *payload; };
struct pair { struct mk_list _head; struct mk_list _rotate; };
struct queue { struct mk_list items; struct mk_list rotated; };
int item_destroy(struct item *e);
int pair_destroy(struct pair *p);
int bare_destroy(struct item *e);
"#;

const LIST: &str = "module contract;
extern fn mk_list_add(entry: ptr<mut, u8>, head: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; ensures normal resource_borrowed(entry); ensures normal resource_borrowed(head); ensures normal resource_published(entry, head); ensures normal resource_return_none(); };
extern fn mk_list_del(entry: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; ensures normal resource_borrowed(entry); ensures normal resource_unpublished(entry); ensures normal resource_return_none(); };
extern fn q_use(p: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; ensures normal resource_borrowed(p); ensures normal resource_return_none(); };
extern fn item_destroy(e: ptr<mut, u8>) -> i32
contract { requires resource_contract_v1; ensures normal resource_unpublished(e, _head); ensures normal resource_released(e, nonnull); ensures normal resource_return_none(); };
extern fn pair_destroy(p: ptr<mut, u8>) -> i32
contract { requires resource_contract_v1; ensures normal resource_unpublished(p, _head); ensures normal resource_released(p, nonnull); ensures normal resource_return_none(); };
extern fn bare_destroy(e: ptr<mut, u8>) -> i32
contract { requires resource_contract_v1; ensures normal resource_unpublished(e); ensures normal resource_released(e, nonnull); ensures normal resource_return_none(); };
";

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

fn clang_available() -> bool {
    qcore_c_gateway::clang_command().is_ok()
}

fn analyze(body: &str) -> (AnalysisStatus, Vec<LegacyDiagnosticKind>) {
    let source = format!("{PRELUDE}\n{body}");
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            dialect: CCompilerDialect::Gnu17,
            ..Default::default()
        },
        "subject",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("valid fixture rejected: {error:?}\n{source}"));
    let summary = match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.unwrap(),
    };
    let result = &summary.functions["subject"];
    (
        result.status,
        result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.kind.clone())
            .collect(),
    )
}

fn has(diagnostics: &[LegacyDiagnosticKind], wanted: &str) -> bool {
    diagnostics.iter().any(|kind| {
        matches!(kind, LegacyDiagnosticKind::Unsupported { description } if description == wanted)
    })
}

fn accusations(diagnostics: &[LegacyDiagnosticKind]) -> usize {
    diagnostics
        .iter()
        .filter(|kind| {
            matches!(
                kind,
                LegacyDiagnosticKind::Leak { .. } | LegacyDiagnosticKind::DoubleRelease { .. }
            )
        })
        .count()
}

const NOT_INITIALISED: &str = "unlink by `item_destroy` of `e`, which this function holds, but this function has not initialised its `_head` links";

/// Every fixture, its verdict and the refusal it must carry (if any).
const FIXTURES: &[(&str, &str, AnalysisStatus, Option<&str>)] = &[
    // P-LERI-HELD-LINKS.
    (
        "held-initialised-destroyed",
        "void subject(void) { struct item *e = q_alloc(); mk_list_init(&e->_head); item_destroy(e); }",
        AnalysisStatus::Safe,
        None,
    ),
    (
        "held-uninitialised-destroyed",
        "void subject(void) { struct item *e = q_alloc(); item_destroy(e); }",
        AnalysisStatus::Unknown,
        Some(NOT_INITIALISED),
    ),
    (
        "held-half-initialised-destroyed",
        "void subject(void) { struct item *e = q_alloc(); mk_list_half_init(&e->_head); item_destroy(e); }",
        AnalysisStatus::Unknown,
        None,
    ),
    (
        "held-initialised-then-handed-on",
        "void subject(void) { struct item *e = q_alloc(); mk_list_init(&e->_head); q_use(e); item_destroy(e); }",
        AnalysisStatus::Unknown,
        Some(NOT_INITIALISED),
    ),
    (
        "held-initialised-then-link-stored",
        "void subject(void) { struct item *e = q_alloc(); mk_list_init(&e->_head);
         e->_head.next = 0; item_destroy(e); }",
        AnalysisStatus::Unknown,
        None,
    ),
    (
        "held-initialised-another-object-handed-on",
        "void subject(void) { struct item *f = q_alloc(); struct item *e = q_alloc();
         mk_list_init(&e->_head); q_use(f); item_destroy(e); q_free(f); }",
        AnalysisStatus::Safe,
        None,
    ),
    (
        "held-initialised-on-one-path",
        "void subject(int c) { struct item *e = q_alloc(); if (c) { mk_list_init(&e->_head); }
         item_destroy(e); }",
        AnalysisStatus::Unknown,
        Some(NOT_INITIALISED),
    ),
    (
        "held-other-member-initialised",
        "void subject(void) { struct pair *p = q_alloc(); mk_list_init(&p->_rotate); pair_destroy(p); }",
        AnalysisStatus::Unknown,
        Some(
            "unlink by `pair_destroy` of `p`, which this function holds, but this function has not initialised its `_head` links",
        ),
    ),
    (
        "held-memberless-destroyer",
        "void subject(void) { struct item *e = q_alloc(); mk_list_init(&e->_head); bare_destroy(e); }",
        AnalysisStatus::Unknown,
        Some(
            "unlink by `bare_destroy` of `e`, which this function holds, but the declaration names no link member",
        ),
    ),
    // Card FB-RETRY-OR6-ELEMENT-IDENTITY (gateway rule 91): the formal's
    // links are its caller's obligation, carried by the derived summary (an
    // element destroyer of `e` through `_head`) and checked at every call;
    // Unknown under rule 46 alone.
    (
        "formal-handed-to-destroyer",
        "void subject(struct item *e) { item_destroy(e); }",
        AnalysisStatus::Safe,
        None,
    ),
    // P-LERI-LINK-MEMBER.
    (
        "foreign-element-same-member",
        "void subject(struct queue *q) { struct mk_list *it, *tmp; struct pair *p;
         mk_list_foreach_safe(it, tmp, &q->items) { p = mk_list_entry(it, struct pair, _head);
         pair_destroy(p); } }",
        AnalysisStatus::Safe,
        None,
    ),
    (
        "foreign-element-other-member",
        "void subject(struct queue *q) { struct mk_list *it, *tmp; struct pair *p;
         mk_list_foreach_safe(it, tmp, &q->rotated) { p = mk_list_entry(it, struct pair, _rotate);
         pair_destroy(p); } }",
        AnalysisStatus::Unknown,
        Some(
            "unlink by `pair_destroy` of `p`, whose list links it through `_rotate`, while the declaration unlinks `_head`",
        ),
    ),
    (
        "foreign-element-memberless-destroyer",
        "void subject(struct queue *q) { struct mk_list *it, *tmp; struct item *e;
         mk_list_foreach_safe(it, tmp, &q->items) { e = mk_list_entry(it, struct item, _head);
         bare_destroy(e); } }",
        AnalysisStatus::Unknown,
        Some(
            "unlink by `bare_destroy` of `e`, whose list links it through `_head`, while the declaration names no link member",
        ),
    ),
];

#[test]
fn every_fixture_has_its_verdict_and_refusal() {
    if !clang_available() {
        return;
    }
    for (name, body, status, refusal) in FIXTURES {
        let (got, diagnostics) = analyze(body);
        assert_eq!(got, *status, "{name}\n{diagnostics:?}");
        if let Some(refusal) = refusal {
            assert!(has(&diagnostics, refusal), "{name}\n{diagnostics:?}");
        }
        // The checks only refuse: no fixture is accused.
        assert_eq!(accusations(&diagnostics), 0, "{name}\n{diagnostics:?}");
    }
}

#[test]
fn the_loader_reads_the_link_member_of_a_destroyer_only() {
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
    let loaded = load(
        "ensures normal resource_unpublished(e, _head); ensures normal resource_released(e, nonnull);",
    )
    .expect("a destroyer names its link member");
    let contract = loaded.contract("d").expect("declared");
    assert_eq!(
        contract.link_members.get(&0).map(String::as_str),
        Some("_head")
    );
    // The link member is not part of the custody projection.
    assert!(contract.custody_projection().link_members.is_empty());
    // Without a member the declaration loads as before, naming none.
    let bare = load(
        "ensures normal resource_unpublished(e); ensures normal resource_released(e, nonnull);",
    )
    .expect("a destroyer without a member still loads");
    assert!(
        bare.contract("d")
            .expect("declared")
            .link_members
            .is_empty()
    );
    // A plain unlink's entry is the link itself: a member there rejects.
    assert!(
        load("ensures normal resource_borrowed(e); ensures normal resource_unpublished(e, _head);")
            .is_err()
    );
    // Malformed spellings reject the module.
    for refused in [
        "ensures normal resource_unpublished(e, _head, x); ensures normal resource_released(e, nonnull);",
        "ensures normal resource_unpublished(); ensures normal resource_released(e, nonnull);",
    ] {
        assert!(load(refused).is_err(), "{refused}");
    }
}

/// Card **FB-RETRY-OR6-ELEMENT-IDENTITY** (gateway rule 91): a formal handed
/// to an element destroyer carries its link obligation to the caller.
///
/// Rule 46 (3) decides P-LERI-HELD-LINKS only from the body's own
/// initialisation, so a body handing its formal to a destroyer refuses
/// (`flb_sched_timer_cb_destroy`'s shape). Rule 91 publishes such a body's
/// summary as an element destroyer of that formal through the same member
/// (`resource_unpublished(p, M)` beside `resource_released(p)`), so every
/// caller's call is checked by rules 34, 43 and 46: its own initialisation,
/// an element bound through `M`, or its own formal (carried again). Every
/// other actual refuses at the caller, and nothing here accuses.
mod carried_link_obligation {
    use qcore_c_gateway::legacy_c::{
        AnalysisStatus, C11TranslationUnit, CCompilerDialect, LegacyDiagnosticKind,
        ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    };
    use qcore_contract::contract_loader::load_resource_contract_sources;

    const PRELUDE: &str = r#"
    #include <stddef.h>
    #define container_of(ptr, type, member) ({                      \
          const typeof( ((type *)0)->member ) *__mptr = (ptr);      \
          (type *)( (char *)__mptr - offsetof(type,member) );})
    struct mk_list { struct mk_list *prev, *next; };
    #define mk_list_foreach_safe(curr, n, head) \
        for (curr = (head)->next, n = curr->next; curr != (head); curr = n, n = curr->next)
    #define mk_list_entry( ptr, type, member ) container_of( ptr, type, member )
    static void mk_list_init(struct mk_list *list) { list->next = list; list->prev = list; }
    void mk_list_add(struct mk_list *entry, struct mk_list *head);
    void mk_list_del(struct mk_list *entry);
    void *q_alloc(void); void *q_try_alloc(void); void q_free(void *); void q_use(void *);
    struct item { int close; struct mk_list _head; struct mk_list _rotate; void *payload; };
    struct holder { struct item *r; struct mk_list _head; };
    struct queue { struct mk_list items; struct mk_list rotated; struct mk_list pending; };
    int item_destroy(struct item *e);
    "#;

    const LIST: &str = "module contract;
    extern fn mk_list_add(entry: ptr<mut, u8>, head: ptr<mut, u8>) -> unit
    contract { requires resource_contract_v1; ensures normal resource_borrowed(entry); ensures normal resource_borrowed(head); ensures normal resource_published(entry, head); ensures normal resource_return_none(); };
    extern fn mk_list_del(entry: ptr<mut, u8>) -> unit
    contract { requires resource_contract_v1; ensures normal resource_borrowed(entry); ensures normal resource_unpublished(entry); ensures normal resource_return_none(); };
    extern fn q_use(p: ptr<mut, u8>) -> unit
    contract { requires resource_contract_v1; ensures normal resource_borrowed(p); ensures normal resource_return_none(); };
    extern fn q_try_alloc() -> ptr<mut, u8>
    contract { requires resource_contract_v1; ensures normal resource_return_owned(nullable); };
    extern fn item_destroy(e: ptr<mut, u8>) -> i32
    contract { requires resource_contract_v1; ensures normal resource_unpublished(e, _head); ensures normal resource_released(e, nonnull); ensures normal resource_return_none(); };
    ";

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

    fn analyze(body: &str, entry: &str) -> (AnalysisStatus, Vec<LegacyDiagnosticKind>) {
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

    fn has(diagnostics: &[LegacyDiagnosticKind], wanted: &str) -> bool {
        diagnostics.iter().any(|kind| {
            matches!(kind, LegacyDiagnosticKind::Unsupported { description } if description == wanted)
        })
    }

    fn accusations(diagnostics: &[LegacyDiagnosticKind]) -> usize {
        diagnostics
            .iter()
            .filter(|kind| {
                matches!(
                    kind,
                    LegacyDiagnosticKind::Leak { .. } | LegacyDiagnosticKind::DoubleRelease { .. }
                )
            })
            .count()
    }

    /// `flb_sched_timer_cb_destroy`'s shape: the formal handed to a destroyer.
    const WRAPPER: &str = "int tdestroy(struct item *t) { item_destroy(t); return 0; }";

    /// `flb_output_task_queue_enqueue`'s shape: the formal handed to a
    /// destroyer on the allocation failure only, stored on the other exit.
    const ENQUEUE: &str = "static int enq(struct queue *q, struct item *r) {
        struct holder *h = q_try_alloc();
        if (!h) { if (r) { item_destroy(r); } return -1; }
        h->r = r; mk_list_add(&h->_head, &q->pending); return 0; }";

    const WRAPPER_HELD: &str =
        "unlink by `item_destroy` of `t`, which this function has not published";
    const CALLER_NOT_INITIALISED: &str = "unlink by `tdestroy` of `e`, which this function holds, but this function has not initialised its `_head` links";
    const CALLER_ROTATE: &str = "unlink by `tdestroy` of `e`, whose list links it through `_rotate`, while the declaration unlinks `_head`";
    const UNCONTRACTED_WRAPPER: &str = "call to uncontracted function `tdestroy`";

    /// Every fixture: name, unit, the function whose row is read, its verdict,
    /// and a refusal it must carry (if any).
    fn fixtures() -> Vec<(
        &'static str,
        String,
        &'static str,
        AnalysisStatus,
        Option<&'static str>,
    )> {
        vec![
            // The wrapper's own row: its summary carries the obligation.
            (
                "wrapper",
                WRAPPER.to_owned(),
                "tdestroy",
                AnalysisStatus::Safe,
                None,
            ),
            // Callers discharging the carried obligation.
            (
                "caller-initialised",
                format!(
                    "{WRAPPER}\nvoid subject(void) {{ struct item *e = q_alloc(); mk_list_init(&e->_head); tdestroy(e); }}"
                ),
                "subject",
                AnalysisStatus::Safe,
                None,
            ),
            (
                "caller-element-through-head",
                format!(
                    "{WRAPPER}\nvoid subject(struct queue *q) {{ struct mk_list *c, *n; struct item *e; mk_list_foreach_safe(c, n, &q->items) {{ e = mk_list_entry(c, struct item, _head); tdestroy(e); }} }}"
                ),
                "subject",
                AnalysisStatus::Safe,
                None,
            ),
            (
                "caller-formal-carried-again",
                format!("{WRAPPER}\nvoid subject(struct item *e) {{ tdestroy(e); }}"),
                "subject",
                AnalysisStatus::Safe,
                None,
            ),
            // Callers that cannot discharge it refuse there.
            (
                "caller-not-initialised",
                format!("{WRAPPER}\nvoid subject(void) {{ struct item *e = q_alloc(); tdestroy(e); }}"),
                "subject",
                AnalysisStatus::Unknown,
                Some(CALLER_NOT_INITIALISED),
            ),
            (
                "caller-element-through-rotate",
                format!(
                    "{WRAPPER}\nvoid subject(struct queue *q) {{ struct mk_list *c, *n; struct item *e; mk_list_foreach_safe(c, n, &q->rotated) {{ e = mk_list_entry(c, struct item, _rotate); tdestroy(e); }} }}"
                ),
                "subject",
                AnalysisStatus::Unknown,
                Some(CALLER_ROTATE),
            ),
            (
                "caller-of-carrier-not-initialised",
                format!(
                    "{WRAPPER}\nstatic void hand(struct item *e) {{ tdestroy(e); }}\nvoid subject(void) {{ struct item *e = q_alloc(); hand(e); }}"
                ),
                "subject",
                AnalysisStatus::Unknown,
                None,
            ),
            // Bodies whose summary may not carry it.
            (
                "wrapper-writes-before",
                "int tdestroy(struct item *t) { q_use(t); item_destroy(t); return 0; }".to_owned(),
                "tdestroy",
                AnalysisStatus::Unknown,
                Some(WRAPPER_HELD),
            ),
            (
                "wrapper-frees-on-one-path",
                "int tdestroy(struct item *t, int k) { if (k) { item_destroy(t); } else { q_free(t); } return 0; }"
                    .to_owned(),
                "tdestroy",
                AnalysisStatus::Unknown,
                Some(WRAPPER_HELD),
            ),
            (
                "wrapper-initialises-its-formal",
                "int tdestroy(struct item *t) { mk_list_init(&t->_head); item_destroy(t); return 0; }"
                    .to_owned(),
                "tdestroy",
                AnalysisStatus::Unknown,
                None,
            ),
            (
                "caller-of-withheld-wrapper",
                "int tdestroy(struct item *t, int k) { if (k) { item_destroy(t); } else { q_free(t); } return 0; }\nvoid subject(void) { struct item *e = q_alloc(); mk_list_init(&e->_head); tdestroy(e, 1); }"
                    .to_owned(),
                "subject",
                AnalysisStatus::Unknown,
                Some(UNCONTRACTED_WRAPPER),
            ),
            // A declared destroyer's own body is checked against its clause:
            // releasing the formal without the unlink no longer validates.
            (
                "declared-destroyer-frees-without-unlink",
                "int item_destroy(struct item *e) { q_free(e); return 0; }".to_owned(),
                "item_destroy",
                AnalysisStatus::Unknown,
                None,
            ),
            // The measured boundary: the obligation is needed on one exit class
            // only, and no summary states a precondition without the unlink.
            (
                "enqueue",
                ENQUEUE.to_owned(),
                "enq",
                AnalysisStatus::Unknown,
                None,
            ),
        ]
    }

    #[test]
    fn every_fixture_has_its_verdict_and_accuses_nothing() {
        if qcore_c_gateway::clang_command().is_err() {
            return;
        }
        for (name, body, entry, status, refusal) in fixtures() {
            let (got, diagnostics) = analyze(&body, entry);
            assert_eq!(got, status, "{name}\n{diagnostics:?}");
            if let Some(refusal) = refusal {
                assert!(has(&diagnostics, refusal), "{name}\n{diagnostics:?}");
            }
            assert_eq!(accusations(&diagnostics), 0, "{name}\n{diagnostics:?}");
        }
    }
}
