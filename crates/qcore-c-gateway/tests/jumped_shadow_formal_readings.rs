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

//! Card JUMPED-SHADOW-FORMAL-READINGS: probes (printed, not yet pinned).

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
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

#[test]
fn probe_table() {
    if qcore_c_gateway::clang_command().is_err() {
        return;
    }
    let mut lines = Vec::new();
    let mut probe = |id: &str, prelude: &str, entry: &str, environment: &ResourceContracts, source: &str| {
        lines.push(format!("{id}: {}", outcome(prelude, source, entry, environment)));
    };
    let l = lists();
    // Container publication.
    for (id, source) in [
        ("C0", "int subject(struct p_queue *queue) { struct p_node *node = p_malloc(sizeof(struct p_node)); if (!node) return -1; p_list_add(&node->_head, &queue->pending); return 0; }"),
        ("CJ1", "int subject(struct p_queue *queue) { struct p_node *node = p_malloc(sizeof(struct p_node)); if (!node) return -1; switch (0) { struct p_queue queue[1]; default: p_list_add(&node->_head, &queue->pending); } return 0; }"),
        ("CJ2", "int subject(struct p_queue *queue) { struct p_node *node = p_malloc(sizeof(struct p_node)); if (!node) return -1; goto out; { struct p_queue queue[1]; out: p_list_add(&node->_head, &queue->pending); } return 0; }"),
        ("CJ3", "int subject(struct p_queue *queue) { struct p_queue *q = queue; struct p_node *node = p_malloc(sizeof(struct p_node)); if (!node) return -1; switch (0) { struct p_queue q[1]; default: p_list_add(&node->_head, &q->pending); } return 0; }"),
        ("CJ4", "int subject(struct p_list *head) { struct p_node *node = p_malloc(sizeof(struct p_node)); if (!node) return -1; switch (0) { struct p_list head[1]; default: p_list_add(&node->_head, head); } return 0; }"),
        ("CJ5", "struct p_holder { struct p_list *list; }; int subject(struct p_holder *holder) { struct p_list *h = holder->list; struct p_node *node = p_malloc(sizeof(struct p_node)); if (!node) return -1; switch (0) { struct p_list h[1]; default: p_list_add(&node->_head, h); } return 0; }"),
        ("CD0", "struct p_holder { struct p_list *list; }; int subject(struct p_holder *holder) { struct p_list *h = holder->list; struct p_node *node = p_malloc(sizeof(struct p_node)); if (!node) return -1; p_list_add(&node->_head, h); return 0; }"),
        ("CS0", "static struct p_list g; int subject(void) { struct p_node *node = p_malloc(sizeof(struct p_node)); if (!node) return -1; p_list_add(&node->_head, &g); return 0; }"),
        ("CSJ1", "static struct p_list g; int subject(void) { struct p_node *node = p_malloc(sizeof(struct p_node)); if (!node) return -1; switch (0) { struct p_list g; default: p_list_add(&node->_head, &g); } return 0; }"),
        ("CSU1", "static struct p_list g; int subject(void) { struct p_node *node = p_malloc(sizeof(struct p_node)); if (!node) return -1; { __attribute__((unused)) struct p_list g; p_list_add(&node->_head, &g); } return 0; }"),
        ("CE1", "int subject(struct p_queue *queue) { struct p_node *node = p_malloc(sizeof(struct p_node)); if (!node) return -1; { enum { queue = 0 }; } p_list_add(&node->_head, &queue->pending); return 0; }"),
        ("CU1", "int subject(struct p_queue *queue) { struct p_node *node = p_malloc(sizeof(struct p_node)); if (!node) return -1; { __attribute__((unused)) struct p_queue queue[1]; p_list_add(&node->_head, &queue->pending); } return 0; }"),
        ("CU2", "int subject(struct p_queue *queue) { struct p_node *node = p_malloc(sizeof(struct p_node)); if (!node) return -1; { _Alignas(8) struct p_queue queue[1]; p_list_add(&node->_head, &queue->pending); } return 0; }"),
        ("CU3", "int subject(struct p_queue *queue) { struct p_node *node = p_malloc(sizeof(struct p_node)); if (!node) return -1; { __typeof__(struct p_queue) queue[1]; p_list_add(&node->_head, &queue->pending); } return 0; }"),
        ("CU4", "typedef struct p_queue pq; int subject(struct p_queue *queue) { struct p_node *node = p_malloc(sizeof(struct p_node)); if (!node) return -1; { pq (queue)[1]; p_list_add(&node->_head, &queue->pending); } return 0; }"),
        ("CU5", "typedef struct p_list pl1[1]; int subject(struct p_list *head) { struct p_node *node = p_malloc(sizeof(struct p_node)); if (!node) return -1; { __attribute__((unused)) pl1 head; p_list_add(&node->_head, head); } return 0; }"),
        ("CU6", "int subject(struct p_list *head) { struct p_node *node = p_malloc(sizeof(struct p_node)); if (!node) return -1; { __typeof__(struct p_list [1]) head; p_list_add(&node->_head, head); } return 0; }"),
    ] {
        probe(id, LISTS_PRELUDE, "subject", &l, source);
    }
    // Counted users.
    for (id, source) in [
        ("K0", "void subject(struct u_task *t) { u_ref(t); u_log(1); u_unref(t, 0); u_release(t); }"),
        ("KJ1", "void subject(struct u_task *t) { switch (0) { struct u_task t[1]; default: u_ref(t); } u_log(1); u_unref(t, 0); u_release(t); }"),
        ("KJ2", "void subject(struct u_task *t) { u_ref(t); switch (0) { struct u_task t[1]; default: u_unref(t, 0); u_release(t); } }"),
        ("KJ3", "void subject(struct u_task *t) { u_ref(t); goto out; { struct u_task t[1]; out: u_unref(t, 0); u_release(t); } }"),
        ("KU1", "void subject(struct u_task *t) { u_ref(t); { __attribute__((unused)) struct u_task t[1]; u_unref(t, 0); u_release(t); } }"),
        ("KU2", "void subject(struct u_task *t) { u_ref(t); { __typeof__(struct u_task) t[1]; u_unref(t, 0); u_release(t); } }"),
    ] {
        probe(id, LISTS_PRELUDE, "subject", &l, source);
    }
    for (id, source) in [
        ("KD0", "void inc_body(struct u_task *t) { t->refs++; }"),
        ("KDJ1", "void inc_body(struct u_task *t) { switch (0) { struct u_task t[1]; default: t->refs++; } }"),
        ("KDU1", "void inc_body(struct u_task *t) { { __attribute__((unused)) struct u_task t[1]; t->refs++; } }"),
    ] {
        probe(id, LISTS_PRELUDE, "inc_body", &l, source);
    }
    // Slot moves.
    let s = slots();
    for (id, source) in [
        ("SM0", "void subject(char **buf) { sm_cat(buf, \"x\"); sm_cat(buf, \"y\"); }"),
        ("SMJ1", "void subject(char **buf) { switch (0) { char *buf[1]; default: sm_cat(buf, \"x\"); } }"),
        ("SMJ2", "void subject(char **buf) { char *own = sm_alloc(4); switch (0) { char *buf[1]; default: buf[0] = own; sm_cat(buf, \"x\"); } }"),
        ("SMJ3", "__SIZE_TYPE__ subject(void *slice, char **p) { switch (0) { char *p[1]; default: return sm_reader(slice, p); } }"),
        ("SMJ4", "void subject(char **buf) { goto out; { char *buf[1]; out: sm_cat(buf, \"x\"); } }"),
        ("SMU1", "void subject(char **buf) { { __attribute__((unused)) char *buf[1]; sm_cat(buf, \"x\"); } }"),
        ("SMU2", "void subject(char **buf) { { __typeof__(char *) buf[1]; sm_cat(buf, \"x\"); } }"),
        ("SMU3", "void subject(char **buf) { { char *(buf)[1]; sm_cat(buf, \"x\"); } }"),
        ("SM00", "__SIZE_TYPE__ subject(void *slice, char **p) { return sm_reader(slice, p); }"),
    ] {
        probe(id, SLOTS_PRELUDE, "subject", &s, source);
    }
    // Descriptor cells (rule 38's `formal` closure).
    let c = cells();
    for (id, source) in [
        ("DC0", "int subject(struct mk_event_loop *loop, struct timer *timer) { if (timer->timer_fd != -1) { mk_event_timeout_destroy(loop, &timer->event); timer->timer_fd = -1; } return 0; }"),
        ("DCJ1", "int subject(struct mk_event_loop *loop, struct timer *timer) { switch (0) { struct timer timer[1]; default: if (timer->timer_fd != -1) { mk_event_timeout_destroy(loop, &timer->event); timer->timer_fd = -1; } } return 0; }"),
        ("DCJ2", "int subject(struct mk_event_loop *loop, struct timer *timer) { goto out; { struct timer timer[1]; out: if (timer->timer_fd != -1) { mk_event_timeout_destroy(loop, &timer->event); timer->timer_fd = -1; } } return 0; }"),
        ("DCJ3", "int subject(struct mk_event_loop *loop, struct timer *timer) { switch (0) { struct timer timer[1]; default: mk_event_timeout_destroy(loop, &timer->event); } return 0; }"),
        ("DCU1", "int subject(struct mk_event_loop *loop, struct timer *timer) { { __attribute__((unused)) struct timer timer[1]; if (timer->timer_fd != -1) { mk_event_timeout_destroy(loop, &timer->event); timer->timer_fd = -1; } } return 0; }"),
        ("DCU2", "int subject(struct mk_event_loop *loop, struct timer *timer) { { __typeof__(struct timer) timer[1]; if (timer->timer_fd != -1) { mk_event_timeout_destroy(loop, &timer->event); timer->timer_fd = -1; } } return 0; }"),
    ] {
        probe(id, CELLS_PRELUDE, "subject", &c, source);
    }
    // Member custody attribution (a re-attribution) and W4 entry custody.
    let r = retained();
    for (id, source) in [
        ("MC0", "void subject(struct box *b, char *v) { b->data = v; }"),
        ("MCJ1", "void subject(struct box *b, char *v) { switch (0) { struct box b[1]; default: b->data = v; } }"),
        ("MCJ2", "void subject(struct box *b) { switch (0) { struct box b[1]; default: r_free(b->data); } }"),
        ("MCU1", "void subject(struct box *b, char *v) { { __attribute__((unused)) struct box b[1]; b->data = v; } }"),
    ] {
        probe(id, RETAINED_PRELUDE, "subject", &r, source);
    }
    for (id, source) in [
        ("W0", "static void drop(char *p) { r_free(p); } void subject(void) { char *q = r_alloc(4); if (!q) return; drop(q); }"),
        ("WJ1", "static void drop(char *p) { switch (0) { char *p; default: r_free(p); } } void subject(void) { char *q = r_alloc(4); if (!q) return; drop(q); }"),
        ("WJ2", "static void drop(char *p) { goto out; { char *p; out: r_free(p); } } void subject(void) { char *q = r_alloc(4); if (!q) return; drop(q); }"),
        ("WJ3", "static char *keep(char *p) { switch (0) { char *p; default: return p; } } void subject(void) { char *q = r_alloc(4); if (!q) return; q = keep(q); r_free(q); }"),
        ("WU1", "static void drop(char *p) { { __attribute__((unused)) char *p = 0; r_free(p); } } void subject(void) { char *q = r_alloc(4); if (!q) return; drop(q); }"),
        ("WU2", "static void drop(char *p) { { __typeof__(char *) p = 0; r_free(p); } } void subject(void) { char *q = r_alloc(4); if (!q) return; drop(q); }"),
        ("WU3", "static void drop(char *p) { { char *(p) = 0; r_free(p); } } void subject(void) { char *q = r_alloc(4); if (!q) return; drop(q); }"),
        ("WU4", "static void drop(char *p) { { _Alignas(8) char *p = 0; r_free(p); } } void subject(void) { char *q = r_alloc(4); if (!q) return; drop(q); }"),
    ] {
        probe(id, RETAINED_PRELUDE, "subject", &r, source);
    }
    for (id, source) in [
        ("WD0", "void drop(char *p) { r_free(p); }"),
        ("WDJ1", "void drop(char *p) { switch (0) { char *p; default: r_free(p); } }"),
        ("WDU1", "void drop(char *p) { { __attribute__((unused)) char *p = 0; r_free(p); } }"),
    ] {
        probe(id, RETAINED_PRELUDE, "drop", &r, source);
    }
    println!("PROBES\n{}", lines.join("\n"));
}
