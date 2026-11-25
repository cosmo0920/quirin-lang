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

//! Card **JUMPED-SHADOW-FORMAL-READINGS** (gateway rule 86, extended): the
//! other consumers of a formal's spelling, and a local that shadows it.
//!
//! Rule 86 (NAME-KEYED-HOLDER-SHADOWING) made rule 12t's formal-keyed
//! readings ask Clang's declaration reading. The other consumers of
//! `CallerFormals` (container publication and its derived locals, rule 38's
//! descriptor-cell `formal` closure, slot moves, member custody attribution,
//! the counted-user pass) and W4 entry custody still read the spelling. A
//! shadow whose declaration control jumps over (before a `switch`'s first
//! label, past a `goto` target) puts the classifier's lexical guard on no
//! path, and a declaration the guard does not recognize (`__attribute__`,
//! `_Alignas`, `__typeof__`, a parenthesized declarator, an enumeration
//! constant) is refused only incidentally. Probed at base (commit
//! `66339083c`): container publication, the descriptor cells and W4 entry
//! custody reached `Safe` that only the internal certificate check stopped,
//! and a derived local's shadow was `Safe` outright.
//!
//! Now `CallerFormals` is read by declaration at its construction, the
//! counted-user pass's stable formals likewise, and a formal that is not one
//! declaration refuses its function at the entry
//! (`legacy_formal_declaration.rs`). The `p_*`, `u_*`, `sm_*`, `mk_*`,
//! `ev_*` and `r_*` declarations are synthetic trusted test assumptions: a
//! `Safe` here is `Safe` over them and promotes nothing.
//! Lean: `QCore.JumpedShadowFormalReadings`.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_legacy_c_translation_unit, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const LISTS: &str = r#"module contract;

extern fn p_malloc(size: usize) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(size);
    ensures normal resource_return_owned(nullable);
};

extern fn p_free(p: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_released(p, nullable);
    ensures normal resource_return_none();
};

extern fn p_list_add(entry: ptr<mut, u8>, head: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(entry);
    ensures normal resource_borrowed(head);
    ensures normal resource_published(entry, head);
    ensures normal resource_return_none();
};

extern fn p_list_del(entry: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(entry);
    ensures normal resource_unpublished(entry);
    ensures normal resource_return_none();
};

extern fn u_ref(t: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(t);
    ensures normal resource_user_added(t, refs);
    ensures normal resource_return_none();
};

extern fn u_unref(t: ptr<mut, u8>, check: i32) -> unit
contract {
    requires resource_contract_v1;
    requires resource_literal_argument(check, 0);
    ensures normal resource_borrowed(t);
    ensures normal resource_borrowed(check);
    ensures normal resource_user_dropped(t, refs);
    ensures normal resource_return_none();
};

extern fn u_release(t: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(t);
    ensures normal resource_user_release_check(t, refs);
    ensures normal resource_return_none();
};

extern fn u_log(n: i32) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(n);
    ensures normal resource_return_none();
};

extern fn inc_body(t: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(t);
    ensures normal resource_user_added(t, refs);
    ensures normal resource_return_none();
};
"#;

const LISTS_PRELUDE: &str = "\
struct p_list { struct p_list *prev, *next; };
struct p_queue { struct p_list pending; };
struct p_node { int value; struct p_list _head; };
struct u_task { int refs; };
void *p_malloc(__SIZE_TYPE__ size);
void p_free(void *p);
void p_list_add(struct p_list *entry, struct p_list *head);
void p_list_del(struct p_list *entry);
void u_ref(struct u_task *t);
void u_unref(struct u_task *t, int check);
void u_release(struct u_task *t);
void u_log(int n);
";

const SLOTS_PRELUDE: &str = "\
void *sm_alloc(__SIZE_TYPE__ n); void sm_free(void *p); __SIZE_TYPE__ sm_len(const char *s);
void sm_touch(void *p);
void sm_cat(char **slot, const char *text);
void *sm_child(void *parent, const char *key);
void *sm_private(void *parent);
__SIZE_TYPE__ sm_reader(void *slice, char **out);
";

const CELLS_PRELUDE: &str = "\
typedef __INT64_TYPE__ time_t;
struct mk_event_loop;
struct mk_event { int fd; int mask; };
struct timer { int timer_fd; struct mk_event event; };
void *ev_alloc(__SIZE_TYPE__ n); void ev_free(void *p);
int mk_event_timeout_create(struct mk_event_loop *loop, time_t sec, __INT64_TYPE__ nsec, void *data);
int mk_event_timeout_destroy(struct mk_event_loop *loop, void *data);
";

const RETAINED_PRELUDE: &str = "\
struct st { const char *base; int n; };
struct box { char *data; struct st *state; };
void *r_alloc(__SIZE_TYPE__ n); void r_free(void *p);
int r_keep(struct st *state, const char *src); void r_reset(struct st *state);
";

fn load(files: &[(&str, &str)]) -> ResourceContracts {
    load_resource_contract_sources(
        &files
            .iter()
            .map(|(name, text)| ((*name).to_owned(), (*text).to_owned()))
            .collect::<BTreeMap<_, _>>(),
    )
    .expect("the fixture contracts load")
}

fn lists() -> ResourceContracts {
    load(&[("lists.qv", LISTS)])
}

fn slots() -> ResourceContracts {
    load(&[(
        "slot_move_borrow.qv",
        include_str!("fixtures/contracts/slot_move_borrow.qv"),
    )])
}

fn cells() -> ResourceContracts {
    load(&[
        (
            "event_loop.qv",
            include_str!("fixtures/contracts/event_loop.qv"),
        ),
        (
            "monkey/event_loop.qv",
            include_str!(
                "../../../providers/qcore-provider-libevent/contracts/monkey/event_loop.qv"
            ),
        ),
    ])
}

fn retained() -> ResourceContracts {
    load(&[(
        "retained_by.qv",
        include_str!("fixtures/contracts/retained_by.qv"),
    )])
}

fn outcome(prelude: &str, source: &str, entry: &str, environment: &ResourceContracts) -> String {
    let text = format!("{prelude}\n{source}\n");
    let outcome = match analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        entry,
        environment,
    ) {
        Ok(outcome) => outcome,
        Err(error) => return format!("REJECTED {error:?}"),
    };
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome
        && let Err(error) = check_legacy_safety_certificate(certificate)
    {
        return format!("CERT-REJECTED {error:?}");
    }
    let summary = match &outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    };
    let Some(row) = summary.and_then(|summary| summary.functions.get(entry)) else {
        return format!("NO-ROW {outcome:?}");
    };
    let diagnostics = row
        .diagnostics
        .iter()
        .map(|d| match &d.kind {
            LegacyDiagnosticKind::Unsupported { description } => description.clone(),
            other => format!("{other:?}"),
        })
        .collect::<Vec<_>>();
    let clauses = row
        .counted_users
        .iter()
        .map(|record| format!("{}={:?}", record.clause, record.verdict))
        .collect::<Vec<_>>();
    let status = match row.status {
        AnalysisStatus::Safe => "Safe",
        AnalysisStatus::Unsafe => "Unsafe",
        AnalysisStatus::Unknown => "Unknown",
        _ => "Other",
    };
    format!("{status} {diagnostics:?} {clauses:?}")
}

const ONE: &str = "is not one declaration";
const OUTLIVES: &str = "does not show outlives it";

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent JUMPED-SHADOW-FORMAL-READINGS test: {reason}");
            false
        }
    }
}

type Shape<'a> = (&'a str, &'a str, &'a str);

/// Each control is `Safe` (and rechecks).
fn safe(prelude: &str, environment: &ResourceContracts, shapes: &[Shape<'_>]) {
    let refused = shapes
        .iter()
        .map(|(id, entry, source)| (id, outcome(prelude, source, entry, environment)))
        .filter(|(_, outcome)| !outcome.starts_with("Safe "))
        .map(|(id, outcome)| format!("{id}: {outcome}"))
        .collect::<Vec<_>>();
    assert!(refused.is_empty(), "{refused:#?}");
}

/// Each shape is analysed (no rejection), is not `Safe`, and its row carries
/// `fragment` (empty: any refusal).
fn refused(prelude: &str, environment: &ResourceContracts, fragment: &str, shapes: &[Shape<'_>]) {
    let admitted = shapes
        .iter()
        .map(|(id, entry, source)| (id, outcome(prelude, source, entry, environment)))
        .filter(|(_, outcome)| !outcome.starts_with("Unknown ") || !outcome.contains(fragment))
        .map(|(id, outcome)| format!("{id}: {outcome}"))
        .collect::<Vec<_>>();
    assert!(admitted.is_empty(), "{admitted:#?}");
}

const PUBLISH: &str =
    "struct p_node *node = p_malloc(sizeof(struct p_node)); if (!node) return -1;";

/// Controls: an unshadowed formal and an unshadowed derived local keep every
/// reading (a publication into a caller's container, a counted-user clause,
/// a caller's slot, a descriptor cell's formal closure, a member store, a W4
/// summary).
#[test]
fn unshadowed_names_keep_their_readings() {
    if !clang_available() {
        return;
    }
    let p = |body: &str| body.replace("PUBLISH", PUBLISH);
    let shapes = [
        ("C0", "int subject(struct p_queue *queue) { PUBLISH p_list_add(&node->_head, &queue->pending); return 0; }"),
        ("CD0", "struct p_holder { struct p_list *list; }; int subject(struct p_holder *holder) { struct p_list *h = holder->list; PUBLISH p_list_add(&node->_head, h); return 0; }"),
        ("CS0", "static struct p_list g; int subject(void) { PUBLISH p_list_add(&node->_head, &g); return 0; }"),
        ("K0", "void subject(struct u_task *t) { u_ref(t); u_log(1); u_unref(t, 0); u_release(t); }"),
    ]
    .map(|(id, source)| (id, p(source)));
    let shapes = shapes
        .iter()
        .map(|(id, source)| (*id, "subject", source.as_str()))
        .collect::<Vec<_>>();
    safe(LISTS_PRELUDE, &lists(), &shapes);
    let declared = outcome(
        LISTS_PRELUDE,
        "void inc_body(struct u_task *t) { t->refs++; }",
        "inc_body",
        &lists(),
    );
    assert!(
        declared.starts_with("Safe ") && declared.contains("resource_user_added(t, refs)=Held"),
        "{declared}"
    );
    safe(
        SLOTS_PRELUDE,
        &slots(),
        &[
            (
                "SM0",
                "subject",
                "void subject(char **buf) { sm_cat(buf, \"x\"); sm_cat(buf, \"y\"); }",
            ),
            (
                "SM00",
                "subject",
                "__SIZE_TYPE__ subject(void *slice, char **p) { return sm_reader(slice, p); }",
            ),
        ],
    );
    safe(
        CELLS_PRELUDE,
        &cells(),
        &[(
            "DC0",
            "subject",
            "int subject(struct mk_event_loop *loop, struct timer *timer) { if (timer->timer_fd != -1) { mk_event_timeout_destroy(loop, &timer->event); timer->timer_fd = -1; } return 0; }",
        )],
    );
    safe(
        RETAINED_PRELUDE,
        &retained(),
        &[
            (
                "MC0",
                "subject",
                "void subject(struct box *b, char *v) { b->data = v; }",
            ),
            (
                "W0",
                "subject",
                "static void drop(char *p) { r_free(p); } void subject(void) { char *q = r_alloc(4); if (!q) return; drop(q); }",
            ),
            ("WD0", "drop", "void drop(char *p) { r_free(p); }"),
        ],
    );
}

/// Container publication. A container rooted at a jumped shadow of a formal
/// (CJ1 `switch`, CJ2 `goto`, CJ4 a bare list head) was read as the caller's:
/// at base the route reached `Safe` and only the internal certificate check
/// rejected the unit. A jumped shadow of a derived local `h = holder->list`
/// (CJ5), or one in an inner block (CJ6), was read as the caller's container
/// and the row was `Safe` past the certificate check. Each now refuses: a
/// shadowed formal at the entry, a shadowed derived local at the
/// publication. The unrecognized declarations (CU1-CU6, an enumeration
/// constant CE1) refuse at the entry, directly.
#[test]
fn a_shadowed_container_root_is_not_the_callers() {
    if !clang_available() {
        return;
    }
    let jumped = [
        ("CJ1", "int subject(struct p_queue *queue) { PUBLISH switch (0) { struct p_queue queue[1]; default: p_list_add(&node->_head, &queue->pending); } return 0; }"),
        ("CJ2", "int subject(struct p_queue *queue) { PUBLISH goto out; { struct p_queue queue[1]; out: p_list_add(&node->_head, &queue->pending); } return 0; }"),
        ("CJ4", "int subject(struct p_list *head) { PUBLISH switch (0) { struct p_list head[1]; default: p_list_add(&node->_head, head); } return 0; }"),
        ("CU1", "int subject(struct p_queue *queue) { PUBLISH { __attribute__((unused)) struct p_queue queue[1]; p_list_add(&node->_head, &queue->pending); } return 0; }"),
        ("CU2", "int subject(struct p_queue *queue) { PUBLISH { _Alignas(8) struct p_queue queue[1]; p_list_add(&node->_head, &queue->pending); } return 0; }"),
        ("CU3", "int subject(struct p_queue *queue) { PUBLISH { __typeof__(struct p_queue) queue[1]; p_list_add(&node->_head, &queue->pending); } return 0; }"),
        ("CU4", "typedef struct p_queue pq; int subject(struct p_queue *queue) { PUBLISH { pq (queue)[1]; p_list_add(&node->_head, &queue->pending); } return 0; }"),
        ("CU5", "typedef struct p_list pl1[1]; int subject(struct p_list *head) { PUBLISH { __attribute__((unused)) pl1 head; p_list_add(&node->_head, head); } return 0; }"),
        ("CU6", "int subject(struct p_list *head) { PUBLISH { __typeof__(struct p_list [1]) head; p_list_add(&node->_head, head); } return 0; }"),
        ("CE1", "int subject(struct p_queue *queue) { PUBLISH { enum { queue = 0 }; } p_list_add(&node->_head, &queue->pending); return 0; }"),
    ]
    .map(|(id, source)| (id, source.replace("PUBLISH", PUBLISH)));
    let shapes = jumped
        .iter()
        .map(|(id, source)| (*id, "subject", source.as_str()))
        .collect::<Vec<_>>();
    refused(LISTS_PRELUDE, &lists(), ONE, &shapes);
    let derived = [
        ("CJ5", "struct p_holder { struct p_list *list; }; int subject(struct p_holder *holder) { struct p_list *h = holder->list; PUBLISH switch (0) { struct p_list h[1]; default: p_list_add(&node->_head, h); } return 0; }"),
        ("CJ6", "struct p_holder { struct p_list *list; }; int subject(struct p_holder *holder) { struct p_list *h = holder->list; PUBLISH { struct p_list h[1]; p_list_add(&node->_head, h); } return 0; }"),
    ]
    .map(|(id, source)| (id, source.replace("PUBLISH", PUBLISH)));
    let shapes = derived
        .iter()
        .map(|(id, source)| (*id, "subject", source.as_str()))
        .collect::<Vec<_>>();
    refused(LISTS_PRELUDE, &lists(), OUTLIVES, &shapes);
    // Refused at base by other routes, and still: an alias derived local
    // (CJ3), a static container's jumped or unrecognized shadow (CSJ1, CSU1).
    let other = [
        ("CJ3", "int subject(struct p_queue *queue) { struct p_queue *q = queue; PUBLISH switch (0) { struct p_queue q[1]; default: p_list_add(&node->_head, &q->pending); } return 0; }"),
        ("CSJ1", "static struct p_list g; int subject(void) { PUBLISH switch (0) { struct p_list g; default: p_list_add(&node->_head, &g); } return 0; }"),
        ("CSU1", "static struct p_list g; int subject(void) { PUBLISH { __attribute__((unused)) struct p_list g; p_list_add(&node->_head, &g); } return 0; }"),
    ]
    .map(|(id, source)| (id, source.replace("PUBLISH", PUBLISH)));
    let shapes = other
        .iter()
        .map(|(id, source)| (*id, "subject", source.as_str()))
        .collect::<Vec<_>>();
    refused(LISTS_PRELUDE, &lists(), "", &shapes);
}

/// Counted users. The pass refuses every `switch` and jump (KJ1-KJ3, KDJ1),
/// so a jumped shadow never reached a clause. But an unrecognized
/// declaration (KU1, KU2, KDU1) was attributed to the formal and its clause
/// recorded `Held`, the row `Unknown` only by an indirect access. Now the
/// pass's stable formals are one declaration each: the operations are
/// unattributed, no clause is `Held`, and the entry refuses.
#[test]
fn a_shadowed_formal_attributes_no_counted_user() {
    if !clang_available() {
        return;
    }
    let shapes: [Shape<'_>; 7] = [
        (
            "KJ1",
            "subject",
            "void subject(struct u_task *t) { switch (0) { struct u_task t[1]; default: u_ref(t); } u_log(1); u_unref(t, 0); u_release(t); }",
        ),
        (
            "KJ2",
            "subject",
            "void subject(struct u_task *t) { u_ref(t); switch (0) { struct u_task t[1]; default: u_unref(t, 0); u_release(t); } }",
        ),
        (
            "KJ3",
            "subject",
            "void subject(struct u_task *t) { u_ref(t); goto out; { struct u_task t[1]; out: u_unref(t, 0); u_release(t); } }",
        ),
        (
            "KU1",
            "subject",
            "void subject(struct u_task *t) { u_ref(t); { __attribute__((unused)) struct u_task t[1]; u_unref(t, 0); u_release(t); } }",
        ),
        (
            "KU2",
            "subject",
            "void subject(struct u_task *t) { u_ref(t); { __typeof__(struct u_task) t[1]; u_unref(t, 0); u_release(t); } }",
        ),
        (
            "KDJ1",
            "inc_body",
            "void inc_body(struct u_task *t) { switch (0) { struct u_task t[1]; default: t->refs++; } }",
        ),
        (
            "KDU1",
            "inc_body",
            "void inc_body(struct u_task *t) { { __attribute__((unused)) struct u_task t[1]; t->refs++; } }",
        ),
    ];
    refused(LISTS_PRELUDE, &lists(), ONE, &shapes);
    let held = shapes
        .iter()
        .map(|(id, entry, source)| (id, outcome(LISTS_PRELUDE, source, entry, &lists())))
        .filter(|(_, outcome)| outcome.contains("=Held"))
        .map(|(id, outcome)| format!("{id}: {outcome}"))
        .collect::<Vec<_>>();
    assert!(held.is_empty(), "{held:#?}");
}

/// Slot moves: the slot-mention census already refused each shape at base
/// (a shadow's declarator mentions the slot). Each also refuses at the entry.
#[test]
fn a_shadowed_slot_is_no_callers_slot() {
    if !clang_available() {
        return;
    }
    refused(
        SLOTS_PRELUDE,
        &slots(),
        ONE,
        &[
            (
                "SMJ1",
                "subject",
                "void subject(char **buf) { switch (0) { char *buf[1]; default: sm_cat(buf, \"x\"); } }",
            ),
            (
                "SMJ2",
                "subject",
                "void subject(char **buf) { char *own = sm_alloc(4); switch (0) { char *buf[1]; default: buf[0] = own; sm_cat(buf, \"x\"); } }",
            ),
            (
                "SMJ3",
                "subject",
                "__SIZE_TYPE__ subject(void *slice, char **p) { switch (0) { char *p[1]; default: return sm_reader(slice, p); } }",
            ),
            (
                "SMJ4",
                "subject",
                "void subject(char **buf) { goto out; { char *buf[1]; out: sm_cat(buf, \"x\"); } }",
            ),
            (
                "SMU1",
                "subject",
                "void subject(char **buf) { { __attribute__((unused)) char *buf[1]; sm_cat(buf, \"x\"); } }",
            ),
            (
                "SMU2",
                "subject",
                "void subject(char **buf) { { __typeof__(char *) buf[1]; sm_cat(buf, \"x\"); } }",
            ),
            (
                "SMU3",
                "subject",
                "void subject(char **buf) { { char *(buf)[1]; sm_cat(buf, \"x\"); } }",
            ),
        ],
    );
}

/// Rule 38's descriptor-cell `formal` closure: a cell of a jumped shadow
/// (DCJ1-DCJ3) was read as the formal's, and at base only the internal
/// certificate check rejected the unit. The unrecognized declarations
/// (DCU1, DCU2) refused incidentally. Each now refuses at the entry.
#[test]
fn a_shadowed_cell_root_is_not_the_formal() {
    if !clang_available() {
        return;
    }
    refused(
        CELLS_PRELUDE,
        &cells(),
        ONE,
        &[
            (
                "DCJ1",
                "subject",
                "int subject(struct mk_event_loop *loop, struct timer *timer) { switch (0) { struct timer timer[1]; default: if (timer->timer_fd != -1) { mk_event_timeout_destroy(loop, &timer->event); timer->timer_fd = -1; } } return 0; }",
            ),
            (
                "DCJ2",
                "subject",
                "int subject(struct mk_event_loop *loop, struct timer *timer) { goto out; { struct timer timer[1]; out: if (timer->timer_fd != -1) { mk_event_timeout_destroy(loop, &timer->event); timer->timer_fd = -1; } } return 0; }",
            ),
            (
                "DCJ3",
                "subject",
                "int subject(struct mk_event_loop *loop, struct timer *timer) { switch (0) { struct timer timer[1]; default: mk_event_timeout_destroy(loop, &timer->event); } return 0; }",
            ),
            (
                "DCU1",
                "subject",
                "int subject(struct mk_event_loop *loop, struct timer *timer) { { __attribute__((unused)) struct timer timer[1]; if (timer->timer_fd != -1) { mk_event_timeout_destroy(loop, &timer->event); timer->timer_fd = -1; } } return 0; }",
            ),
            (
                "DCU2",
                "subject",
                "int subject(struct mk_event_loop *loop, struct timer *timer) { { __typeof__(struct timer) timer[1]; if (timer->timer_fd != -1) { mk_event_timeout_destroy(loop, &timer->event); timer->timer_fd = -1; } } return 0; }",
            ),
        ],
    );
}

/// W4 entry custody and member custody attribution. A release through a
/// jumped shadow (WDJ1) was read as the formal's: the row reached `Safe`,
/// stopped only by the internal certificate check, and a caller consuming
/// that summary (WJ1, WJ2) too. Member custody attribution named a shadow
/// "the formal" (MCJ1, MCJ2, MCU1), a re-attribution only. Each refuses at
/// the entry now; a caller of a refused body finds no summary.
#[test]
fn a_shadowed_formal_has_no_entry_custody() {
    if !clang_available() {
        return;
    }
    refused(
        RETAINED_PRELUDE,
        &retained(),
        ONE,
        &[
            (
                "WDJ1",
                "drop",
                "void drop(char *p) { switch (0) { char *p; default: r_free(p); } }",
            ),
            (
                "WDJ2",
                "drop",
                "void drop(char *p) { goto out; { char *p; out: r_free(p); } }",
            ),
            (
                "WDU1",
                "drop",
                "void drop(char *p) { { __attribute__((unused)) char *p = 0; r_free(p); } }",
            ),
            (
                "WDU2",
                "drop",
                "void drop(char *p) { { char *(p) = 0; r_free(p); } }",
            ),
            (
                "MCJ1",
                "subject",
                "void subject(struct box *b, char *v) { switch (0) { struct box b[1]; default: b->data = v; } }",
            ),
            (
                "MCJ2",
                "subject",
                "void subject(struct box *b) { switch (0) { struct box b[1]; default: r_free(b->data); } }",
            ),
            (
                "MCU1",
                "subject",
                "void subject(struct box *b, char *v) { { __attribute__((unused)) struct box b[1]; b->data = v; } }",
            ),
        ],
    );
    refused(
        RETAINED_PRELUDE,
        &retained(),
        "call to uncontracted function",
        &[
            (
                "WJ1",
                "subject",
                "static void drop(char *p) { switch (0) { char *p; default: r_free(p); } } void subject(void) { char *q = r_alloc(4); if (!q) return; drop(q); }",
            ),
            (
                "WJ2",
                "subject",
                "static void drop(char *p) { goto out; { char *p; out: r_free(p); } } void subject(void) { char *q = r_alloc(4); if (!q) return; drop(q); }",
            ),
            (
                "WJ3",
                "subject",
                "static char *keep(char *p) { switch (0) { char *p; default: return p; } } void subject(void) { char *q = r_alloc(4); if (!q) return; q = keep(q); r_free(q); }",
            ),
            (
                "WU1",
                "subject",
                "static void drop(char *p) { { __attribute__((unused)) char *p = 0; r_free(p); } } void subject(void) { char *q = r_alloc(4); if (!q) return; drop(q); }",
            ),
            (
                "WU2",
                "subject",
                "static void drop(char *p) { { __typeof__(char *) p = 0; r_free(p); } } void subject(void) { char *q = r_alloc(4); if (!q) return; drop(q); }",
            ),
            (
                "WU3",
                "subject",
                "static void drop(char *p) { { char *(p) = 0; r_free(p); } } void subject(void) { char *q = r_alloc(4); if (!q) return; drop(q); }",
            ),
            (
                "WU4",
                "subject",
                "static void drop(char *p) { { _Alignas(8) char *p = 0; r_free(p); } } void subject(void) { char *q = r_alloc(4); if (!q) return; drop(q); }",
            ),
        ],
    );
}

/// Without Clang's reading of the function (a unit Clang does not accept:
/// here `r_free` and `r_alloc` are undeclared), the token route still runs.
/// A body that holds a shape the lexical guard cannot see (a `switch` or a
/// `goto` where the guard refused a declaration, an unrecognized declaration
/// opener, a parenthesized formal after a typedef name) refuses at the
/// entry; one that holds none keeps its reading.
#[test]
fn without_clangs_reading_an_unguarded_body_refuses() {
    let environment = retained();
    let run = |source: &str| {
        analyze_legacy_c_translation_unit(source, "drop", &environment)
            .map(|summary| {
                let row = &summary.functions["drop"];
                (row.status, format!("{:?}", row.diagnostics))
            })
            .unwrap_or_else(|error| panic!("{source}: {error:?}"))
    };
    let (status, diagnostics) = run("void drop(char *p) { r_free(p); }");
    assert_eq!(status, AnalysisStatus::Safe, "{diagnostics}");
    for source in [
        "void drop(char *p) { switch (0) { char *p; default: r_free(p); } }",
        "void drop(char *p) { goto out; { char *p; out: r_free(p); } }",
        "void drop(char *p) { { __attribute__((unused)) char *p = 0; r_free(p); } }",
        "typedef char ch; void drop(char *p) { { ch (p)[4]; r_free(p); } }",
    ] {
        let (status, diagnostics) = run(source);
        assert_eq!(status, AnalysisStatus::Unknown, "{source}: {diagnostics}");
        assert!(
            diagnostics.contains("the function's declarations are not read"),
            "{source}: {diagnostics}"
        );
    }
    // A declarator the lexical guard reads refuses where it stands, as before.
    let (status, diagnostics) = run("void drop(char *p) { { char *(p) = 0; r_free(p); } }");
    assert_eq!(status, AnalysisStatus::Unknown, "{diagnostics}");
}
