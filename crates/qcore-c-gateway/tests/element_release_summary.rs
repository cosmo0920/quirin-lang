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

//! Card **LIST-ENTRY-RELEASE-IDENTITY**: the released list element and its
//! summary (gateway rule 43).
//!
//! Part 1: an element destroyer (rule 34) handed an object the path already
//! holds as one live, unpublished identity no path unlinked takes nothing at
//! its unlink, and its release ends that custody. Part 2: a body whose every
//! element custody comes from containers rooted at its formals publishes its
//! summary with `ElementsReleased` at those formals; a caller applying it
//! forgets every object it published, and carries the clause on when its
//! actual is rooted at its own formal.
//!
//! The prelude is `tests/element_destroyer.rs`'s: Monkey's list macros as the
//! pinned headers spell them. `item_destroy` stands for a Fluent Bit
//! destroyer such as `flb_tail_file_remove`.

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
struct item { int close; struct mk_list _head; void *payload; };
struct queue { struct mk_list items; struct mk_list done; int total; };
int item_destroy(struct item *e);
void item_close(struct item *e);
static struct mk_list registry;
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
extern fn item_close(e: ptr<mut, u8>) -> unit
contract { requires resource_contract_v1; ensures normal resource_unpublished(e); ensures normal resource_released(e, nonnull); ensures normal resource_return_none(); quant live #e: item.payload |-> #released; };
";

const NOT_PUBLISHED: &str =
    "unlink by `item_destroy` of `e`, which this function has not published";

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

fn uncontracted(diagnostics: &[LegacyDiagnosticKind], callee: &str) -> bool {
    let wanted = format!("call to uncontracted function `{callee}`");
    diagnostics.iter().any(|kind| {
        matches!(kind, LegacyDiagnosticKind::Unsupported { description } if *description == wanted)
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

/// The draining helper: a safe iteration over its formal's list, each
/// element handed to the destroyer (rule 34 over rule 10f(2)'s element).
const DRAIN: &str = "static int drain(struct queue *q) { int c = 0; struct mk_list *it, *tmp;
     struct item *e; mk_list_foreach_safe(it, tmp, &q->items) {
     e = mk_list_entry(it, struct item, _head); item_destroy(e); c++; } return c; }";

/// The same with `mk_list_del` and `q_free` (rule 10f(2) alone).
const DRAIN_UNLINK: &str =
    "static void drain_unlink(struct mk_list *list) { struct mk_list *it, *tmp;
     struct item *e; mk_list_foreach_safe(it, tmp, list) {
     e = mk_list_entry(it, struct item, _head); mk_list_del(&e->_head); q_free(e); } }";

/// A drain of a static list: no clause can name its root.
const DRAIN_STATIC: &str = "static void drain_static(void) { struct mk_list *it, *tmp;
     struct item *e; mk_list_foreach_safe(it, tmp, &registry) {
     e = mk_list_entry(it, struct item, _head); item_destroy(e); } }";

/// Every fixture with its verdict with rule 43 on and with it off.
const FIXTURES: &[(&str, &str, AnalysisStatus, AnalysisStatus)] = &[
    // Part 2: the helper keeps its row; its caller reads the summary.
    (
        "drain-helper",
        "",
        AnalysisStatus::Safe,
        AnalysisStatus::Safe,
    ),
    (
        "drain-caller",
        "void subject(struct queue *q) { drain(q); }",
        AnalysisStatus::Safe,
        AnalysisStatus::Unknown,
    ),
    (
        "drain-unlink-caller-member",
        "void subject(struct queue *q) { drain_unlink(&q->done); }",
        AnalysisStatus::Safe,
        AnalysisStatus::Unknown,
    ),
    (
        "drain-caller-derived-local",
        "void subject(void *data) { struct queue *q = data; drain(q); }",
        AnalysisStatus::Safe,
        AnalysisStatus::Unknown,
    ),
    // The clause carried on through a wrapper rooted at its own formal.
    (
        "drain-wrapper-caller",
        "static void wrapper(struct queue *q) { (void) drain(q); }
         void subject(struct queue *q) { wrapper(q); }",
        AnalysisStatus::Safe,
        AnalysisStatus::Unknown,
    ),
    // Refusing: an object published before the drain is forgotten, so its
    // later unlink refuses.
    (
        "published-then-unlinked",
        "void subject(struct queue *q) { struct item *x = q_alloc();
         mk_list_add(&x->_head, &q->items); drain(q); mk_list_del(&x->_head); q_free(x); }",
        AnalysisStatus::Unknown,
        AnalysisStatus::Unknown,
    ),
    // A static container: the helper's summary is withheld.
    (
        "static-drain-caller",
        "void subject(void) { drain_static(); }",
        AnalysisStatus::Unknown,
        AnalysisStatus::Unknown,
    ),
    // A caller whose actual is a local list: its own row reads the clause,
    // its own summary is withheld, so its caller stays uncontracted.
    (
        "local-actual-caller-of-caller",
        "static void local(void) { struct mk_list l; l.next = &l; l.prev = &l; drain_unlink(&l); }
         void subject(void) { local(); }",
        AnalysisStatus::Unknown,
        AnalysisStatus::Unknown,
    ),
    // A member-path destroyer (rule 30's states beside rule 34's unlink,
    // §C04.8 as amended by this card) over a formal-rooted element. The
    // amendment is the loader's, not rule 43's: Safe with rule 43 off too.
    (
        "member-path-destroyer-element",
        "void subject(struct queue *q) { struct mk_list *it, *tmp; struct item *e;
         mk_list_foreach_safe(it, tmp, &q->items) { e = mk_list_entry(it, struct item, _head);
         item_close(e); } }",
        AnalysisStatus::Safe,
        AnalysisStatus::Safe,
    ),
    // ... and a caller of a drain through it reads the summary (rule 43).
    (
        "member-path-drain-caller",
        "static void close_all(struct queue *q) { struct mk_list *it, *tmp; struct item *e;
         mk_list_foreach_safe(it, tmp, &q->items) { e = mk_list_entry(it, struct item, _head);
         item_close(e); } }
         void subject(struct queue *q) { close_all(q); }",
        AnalysisStatus::Safe,
        AnalysisStatus::Unknown,
    ),
    // Part 1: the held object at an element destroyer.
    (
        "held-acquisition-destroyed",
        "void subject(void) { struct item *e = q_alloc(); item_destroy(e); }",
        AnalysisStatus::Safe,
        AnalysisStatus::Unknown,
    ),
    (
        "formal-handed-to-destroyer-caller",
        "static void hand(struct item *e) { item_destroy(e); }
         void subject(void) { struct item *e = q_alloc(); hand(e); }",
        AnalysisStatus::Safe,
        AnalysisStatus::Unknown,
    ),
    // Refusing: an object this path already unlinked.
    (
        "held-unlinked-then-destroyed",
        "void subject(struct queue *q) { struct item *e = q_alloc();
         mk_list_add(&e->_head, &q->items); mk_list_del(&e->_head); item_destroy(e); }",
        AnalysisStatus::Unknown,
        AnalysisStatus::Unknown,
    ),
    // Accusing control: a held object destroyed, then released again (off,
    // rule 34 refuses the unlink but the call's release still applies, so
    // the second release is accused either way).
    (
        "held-destroyed-then-freed",
        "void subject(void) { struct item *e = q_alloc(); item_destroy(e); q_free(e); }",
        AnalysisStatus::Unsafe,
        AnalysisStatus::Unsafe,
    ),
];

fn unit(body: &str) -> String {
    format!("{DRAIN}\n{DRAIN_UNLINK}\n{DRAIN_STATIC}\n{body}")
}

fn verdict(name: &str, body: &str) -> (AnalysisStatus, Vec<LegacyDiagnosticKind>) {
    if name == "drain-helper" {
        analyze_entry(&unit("void subject(void) {}"), "drain")
    } else {
        analyze(&unit(body))
    }
}

#[test]
fn with_rule_43_on_every_fixture_has_its_verdict() {
    if std::env::var_os(CHILD).is_some() || !clang_available() {
        return;
    }
    assert!(gateway_rules::effective_off().is_empty());
    for (name, body, on, _) in FIXTURES {
        let (status, diagnostics) = verdict(name, body);
        assert_eq!(status, *on, "{name}\n{diagnostics:?}");
    }
}

#[test]
fn a_summary_with_the_clause_reaches_the_caller() {
    if std::env::var_os(CHILD).is_some() || !clang_available() {
        return;
    }
    let (_, diagnostics) = verdict(
        "drain-caller",
        "void subject(struct queue *q) { drain(q); }",
    );
    assert!(!uncontracted(&diagnostics, "drain"), "{diagnostics:?}");
}

#[test]
fn a_withheld_summary_leaves_the_call_uncontracted() {
    if std::env::var_os(CHILD).is_some() || !clang_available() {
        return;
    }
    let (_, diagnostics) = verdict(
        "static-drain-caller",
        "void subject(void) { drain_static(); }",
    );
    assert!(
        uncontracted(&diagnostics, "drain_static"),
        "{diagnostics:?}"
    );
    let (_, diagnostics) = verdict(
        "local-actual-caller-of-caller",
        "static void local(void) { struct mk_list l; l.next = &l; l.prev = &l; drain_unlink(&l); }
         void subject(void) { local(); }",
    );
    assert!(uncontracted(&diagnostics, "local"), "{diagnostics:?}");
}

#[test]
fn a_forgotten_publication_refuses_and_accuses_nothing() {
    if std::env::var_os(CHILD).is_some() || !clang_available() {
        return;
    }
    let (status, diagnostics) = verdict(
        "published-then-unlinked",
        "void subject(struct queue *q) { struct item *x = q_alloc();
         mk_list_add(&x->_head, &q->items); drain(q); mk_list_del(&x->_head); q_free(x); }",
    );
    assert_eq!(status, AnalysisStatus::Unknown, "{diagnostics:?}");
    assert_eq!(accusations(&diagnostics), 0, "{diagnostics:?}");
}

#[test]
fn an_unlinked_held_object_keeps_rule_34s_refusal() {
    if std::env::var_os(CHILD).is_some() || !clang_available() {
        return;
    }
    let (_, diagnostics) = verdict(
        "held-unlinked-then-destroyed",
        "void subject(struct queue *q) { struct item *e = q_alloc();
         mk_list_add(&e->_head, &q->items); mk_list_del(&e->_head); item_destroy(e); }",
    );
    assert!(has(&diagnostics, NOT_PUBLISHED), "{diagnostics:?}");
}

// ---- the switch ------------------------------------------------------------
//
// The switch is read once per process, so the switched case runs this binary
// again as a child with `QCORE_GATEWAY_RULES_OFF=43` (`child_*` tests do
// nothing unless `ELEMENT_RELEASE_SUMMARY_CHILD` names them).

const CHILD: &str = "ELEMENT_RELEASE_SUMMARY_CHILD";

#[test]
fn switching_rule_43_off_restores_the_earlier_verdicts_and_never_widens() {
    if std::env::var_os(CHILD).is_some() || !clang_available() {
        return;
    }
    let test = "child_rule_43_off";
    let status = Command::new(std::env::current_exe().expect("the test binary"))
        .args([test, "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD, test)
        .env(gateway_rules::RULES_OFF_VARIABLE, "43")
        .status()
        .expect("the child test runs");
    assert!(status.success(), "{test}: {status}");
}

#[test]
fn child_rule_43_off() {
    if std::env::var(CHILD).is_ok_and(|named| named != "child_rule_43_off")
        || std::env::var_os(CHILD).is_none()
    {
        return;
    }
    assert_eq!(
        gateway_rules::effective_off()
            .into_iter()
            .collect::<Vec<_>>(),
        ["43"]
    );
    for (name, body, on, off) in FIXTURES {
        let (status, diagnostics) = verdict(name, body);
        assert_eq!(status, *off, "{name}\n{diagnostics:?}");
        // Off never widens: no fixture is Safe off and not Safe on.
        assert!(
            *off != AnalysisStatus::Safe || *on == AnalysisStatus::Safe,
            "{name}"
        );
    }
}
