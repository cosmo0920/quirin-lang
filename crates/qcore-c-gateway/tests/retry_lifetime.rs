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

//! W45-FB-RETRY-LIFETIME — intrusive-container publication and counted users
//! (`docs/reports/w45-fb-retry-lifetime/README.md`).
//!
//! Two held-out generic families, unrelated to Fluent Bit and to the `g_*`,
//! `h_*`, `k_*`, `m_*` and `r_*` families of earlier cards:
//!
//! * `p_*`, a list API declared with `resource_published` and
//!   `resource_unpublished`;
//! * `u_*`, a counted-user API declared with `resource_user_*` operations and
//!   exit clauses.
//!
//! Accepting controls: a publication into a caller's list; a publication then
//! an unlink before the release; balanced additions and drops on every path;
//! an exit clause the body satisfies. Refusing controls: a release while
//! linked; a publication into a local list; a publication of a borrowed
//! formal's object; an unlink of an object the body does not hold; a return of
//! a published object; the C2 shape (an addition, then an early return without
//! the drop); the C3 shape (a drop without a release check, then a failure
//! that re-adds nothing); the N2 shape (a rolled-back addition reported as
//! success); an unattributed operation; an opaque call.
//!
//! `Safe` here is `Safe` for a synthetic declared projection under the premises
//! the report names, never a statement about Fluent Bit.

#[path = "support/host_abi.rs"]
mod host_abi;

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, LegacyDiagnosticKind, LegacyFunctionSummary, LegacyProgramSummary,
    ResourceContracts, analyze_legacy_c_translation_unit,
    counted_users::{ClauseRecord, ClauseVerdict},
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const CONTRACTS: &str = r#"module contract;

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

extern fn u_enqueue(q: ptr<mut, u8>, t: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(q);
    ensures normal resource_borrowed(t);
    ensures normal resource_user_neutral();
    ensures normal resource_return_none();
};

extern fn u_flush(t: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(t);
    ensures normal resource_user_exit(t, -1, restored);
    ensures normal resource_user_exit(t, 0, added);
    ensures normal resource_return_none();
};

extern fn u_log(n: i32) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(n);
    ensures normal resource_return_none();
};

extern fn a3(q: ptr<mut, u8>, t: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(q);
    ensures normal resource_borrowed(t);
    ensures normal resource_user_exit(t, -1, restored);
    ensures normal resource_user_exit(t, 0, added);
    ensures normal resource_return_none();
};

extern fn c2(q: ptr<mut, u8>, t: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(q);
    ensures normal resource_borrowed(t);
    ensures normal resource_user_exit(t, -1, restored);
    ensures normal resource_user_exit(t, 0, added);
    ensures normal resource_return_none();
};

extern fn n2(t: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(t);
    ensures normal resource_user_exit(t, -1, restored);
    ensures normal resource_user_exit(t, 0, added);
    ensures normal resource_return_none();
};

extern fn inc_body(t: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(t);
    ensures normal resource_user_added(t, refs);
    ensures normal resource_return_none();
};

extern fn dec_body(t: ptr<mut, u8>, check: i32) -> unit
contract {
    requires resource_contract_v1;
    requires resource_literal_argument(check, 0);
    ensures normal resource_borrowed(t);
    ensures normal resource_borrowed(check);
    ensures normal resource_user_dropped(t, refs);
    ensures normal resource_return_none();
};

extern fn helper_neutral(q: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(q);
    ensures normal resource_user_neutral();
    ensures normal resource_return_none();
};

// W46: scalar event/status query, with the same custody and user effects as
// the pinned Fluent Bit input/route queries.
extern fn e_status(node: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(node);
    ensures normal resource_user_neutral();
    ensures normal resource_return_none();
};
"#;

const PRELUDE: &str = concat!(
    host_abi::size_t_declaration!(),
    r#"

struct p_list { struct p_list *prev, *next; };
struct p_queue { struct p_list pending; };
struct p_node { int value; struct p_list _head; };
struct u_task { int refs; };
struct u_queue { int size; };
struct u_holder { struct u_task *task; };
void *p_malloc(size_t size);
void p_free(void *p);
void p_list_add(struct p_list *entry, struct p_list *head);
void p_list_del(struct p_list *entry);
void u_ref(struct u_task *t);
void u_unref(struct u_task *t, int check);
void u_release(struct u_task *t);
int u_enqueue(struct u_queue *q, struct u_task *t);
int u_flush(struct u_task *t);
void u_log(int n);
int u_unknown(void);
int e_status(struct p_node *node);
"#
);

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "retry_lifetime.qcore".to_owned(),
        CONTRACTS.to_owned(),
    )]))
    .expect("the fixture contracts load")
}

fn analyse(body: &str, entry: &str) -> LegacyProgramSummary {
    analyze_legacy_c_translation_unit(&format!("{PRELUDE}\n{body}"), entry, &contracts())
        .expect("the fixture analyses")
}

fn function<'a>(summary: &'a LegacyProgramSummary, name: &str) -> &'a LegacyFunctionSummary {
    summary
        .functions
        .get(name)
        .unwrap_or_else(|| panic!("missing function `{name}`"))
}

fn kinds(summary: &LegacyFunctionSummary) -> Vec<String> {
    summary
        .diagnostics
        .iter()
        .map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::Leak { .. } => "leak".to_owned(),
            LegacyDiagnosticKind::ReleaseAfterEscape { .. } => "release_after_escape".to_owned(),
            LegacyDiagnosticKind::CountedUser { description } => {
                format!("counted_user:{description}")
            }
            LegacyDiagnosticKind::Unsupported { description } => {
                format!("unsupported:{description}")
            }
            LegacyDiagnosticKind::UnknownAlias { variable } => format!("unknown_alias:{variable}"),
            other => format!("{other:?}"),
        })
        .collect()
}

fn verdicts(summary: &LegacyFunctionSummary) -> Vec<(String, ClauseVerdict)> {
    summary
        .counted_users
        .iter()
        .map(
            |ClauseRecord {
                 clause, verdict, ..
             }| (clause.clone(), *verdict),
        )
        .collect()
}

// ---- container publication ----

#[test]
fn a_publication_into_a_callers_list_discharges_the_object() {
    let summary = analyse(
        r#"
int p_enqueue(struct p_queue *queue, int value)
{
    struct p_node *node;

    node = p_malloc(sizeof(struct p_node));
    if (!node) {
        return -1;
    }
    node->value = value;
    p_list_add(&node->_head, &queue->pending);
    return 0;
}
"#,
        "p_enqueue",
    );
    let enqueue = function(&summary, "p_enqueue");
    assert_eq!(enqueue.status, AnalysisStatus::Safe, "{:?}", kinds(enqueue));
}

#[test]
fn the_borrowed_reading_accused_the_same_body() {
    // Non-vacuity: the same body under the pre-card declaration.
    let borrowed = CONTRACTS.replace("    ensures normal resource_published(entry, head);\n", "");
    let contracts =
        load_resource_contract_sources(&BTreeMap::from([("borrowed.qcore".to_owned(), borrowed)]))
            .unwrap();
    let source = format!(
        "{PRELUDE}\nint p_enqueue(struct p_queue *queue)\n{{\n    struct p_node *node;\n    node = p_malloc(sizeof(struct p_node));\n    if (!node) {{\n        return -1;\n    }}\n    p_list_add(&node->_head, &queue->pending);\n    return 0;\n}}\n"
    );
    let summary = analyze_legacy_c_translation_unit(&source, "p_enqueue", &contracts).unwrap();
    let enqueue = function(&summary, "p_enqueue");
    assert_eq!(enqueue.status, AnalysisStatus::Unsafe);
    assert!(kinds(enqueue).contains(&"leak".to_owned()));
}

#[test]
fn an_unlink_before_the_release_restores_custody() {
    let summary = analyse(
        r#"
void p_cycle(struct p_queue *queue)
{
    struct p_node *node;

    node = p_malloc(sizeof(struct p_node));
    if (!node) {
        return;
    }
    p_list_add(&node->_head, &queue->pending);
    p_list_del(&node->_head);
    p_free(node);
}
"#,
        "p_cycle",
    );
    let cycle = function(&summary, "p_cycle");
    assert_eq!(cycle.status, AnalysisStatus::Safe, "{:?}", kinds(cycle));
}

#[test]
fn a_release_while_linked_is_a_violation_candidate() {
    let summary = analyse(
        r#"
void p_linked_free(struct p_queue *queue)
{
    struct p_node *node;

    node = p_malloc(sizeof(struct p_node));
    if (!node) {
        return;
    }
    p_list_add(&node->_head, &queue->pending);
    p_free(node);
}
"#,
        "p_linked_free",
    );
    let linked = function(&summary, "p_linked_free");
    assert_eq!(linked.status, AnalysisStatus::Unsafe);
    assert!(kinds(linked).contains(&"release_after_escape".to_owned()));
}

#[test]
fn a_publication_into_a_local_list_is_refused() {
    let summary = analyse(
        r#"
void p_local(void)
{
    struct p_list local;
    struct p_node *node;

    node = p_malloc(sizeof(struct p_node));
    if (!node) {
        return;
    }
    p_list_add(&node->_head, &local);
}
"#,
        "p_local",
    );
    let local = function(&summary, "p_local");
    assert_eq!(local.status, AnalysisStatus::Unknown, "{:?}", kinds(local));
    assert!(
        kinds(local)
            .iter()
            .any(|kind| kind.contains("does not show outlives it"))
    );
}

#[test]
fn a_publication_of_a_borrowed_formal_certifies_no_caller() {
    let summary = analyse(
        r#"
static void p_link(struct p_queue *queue, struct p_node *node)
{
    p_list_add(&node->_head, &queue->pending);
}

void p_caller(struct p_queue *queue)
{
    struct p_node *node;

    node = p_malloc(sizeof(struct p_node));
    if (!node) {
        return;
    }
    p_link(queue, node);
    p_free(node);
}
"#,
        "p_caller",
    );
    let caller = function(&summary, "p_caller");
    assert_ne!(caller.status, AnalysisStatus::Safe, "{:?}", kinds(caller));
}

#[test]
fn an_unlink_of_an_object_the_body_does_not_hold_is_refused() {
    let summary = analyse(
        r#"
void p_unlink(struct p_node *node)
{
    p_list_del(&node->_head);
}
"#,
        "p_unlink",
    );
    let unlink = function(&summary, "p_unlink");
    assert_eq!(
        unlink.status,
        AnalysisStatus::Unknown,
        "{:?}",
        kinds(unlink)
    );
}

#[test]
fn a_return_of_a_published_object_is_refused() {
    let summary = analyse(
        r#"
struct p_node *p_publish_and_return(struct p_queue *queue)
{
    struct p_node *node;

    node = p_malloc(sizeof(struct p_node));
    if (!node) {
        return 0;
    }
    p_list_add(&node->_head, &queue->pending);
    return node;
}
"#,
        "p_publish_and_return",
    );
    let returned = function(&summary, "p_publish_and_return");
    assert_eq!(
        returned.status,
        AnalysisStatus::Unknown,
        "{:?}",
        kinds(returned)
    );
}

#[test]
fn a_container_reached_through_a_local_copy_of_a_formal_member_is_the_callers() {
    // `struct sched *sched = config->sched;` names memory the caller supplied,
    // exactly as `config` does: the helper links the caller's object there,
    // so it consumes it, and a caller that frees it afterwards is accused.
    let summary = analyse(
        r#"
struct p_config { struct p_queue *sched; };

static int p_wait(struct p_node *node, struct p_config *config)
{
    struct p_queue *sched = config->sched;

    p_list_add(&node->_head, &sched->pending);
    return 0;
}

void p_wait_then_free(struct p_config *config)
{
    struct p_node *node;

    node = p_malloc(sizeof(struct p_node));
    if (!node) {
        return;
    }
    p_wait(node, config);
    p_free(node);
}

void p_wait_only(struct p_config *config)
{
    struct p_node *node;

    node = p_malloc(sizeof(struct p_node));
    if (!node) {
        return;
    }
    p_wait(node, config);
}
"#,
        "p_wait_then_free",
    );
    let freed = function(&summary, "p_wait_then_free");
    assert_ne!(freed.status, AnalysisStatus::Safe, "{:?}", kinds(freed));
    let kept = function(&summary, "p_wait_only");
    assert_eq!(kept.status, AnalysisStatus::Safe, "{:?}", kinds(kept));
}

/// Card MONKEY-VHOST-PROPERTY-CUSTODY (gateway rule 12d) deliberately changed
/// this pin: an owner whose field-linked member is exactly live and
/// unencumbered now carries it into the caller's list (an unlink hands it
/// back), where rule 10d withdrew it. A member that is not exactly live keeps
/// the withdrawal, so the refusal control stays here.
#[test]
fn a_publication_of_an_object_that_owns_members_carries_only_live_ones() {
    let summary = analyse(
        r#"
struct p_owner { char *buffer; struct p_list _head; };

void p_owner_publish(struct p_queue *queue)
{
    struct p_owner *owner;

    owner = p_malloc(sizeof(struct p_owner));
    if (!owner) {
        return;
    }
    owner->buffer = p_malloc(16);
    p_list_add(&owner->_head, &queue->pending);
}

void p_owner_publish_released(struct p_queue *queue)
{
    struct p_owner *owner;
    char *buffer;

    owner = p_malloc(sizeof(struct p_owner));
    if (!owner) {
        return;
    }
    owner->buffer = p_malloc(16);
    buffer = owner->buffer;
    p_free(buffer);
    p_list_add(&owner->_head, &queue->pending);
}
"#,
        "p_owner_publish",
    );
    let owner = function(&summary, "p_owner_publish");
    assert_eq!(owner.status, AnalysisStatus::Safe, "{:?}", kinds(owner));
    let released = function(&summary, "p_owner_publish_released");
    assert_ne!(
        released.status,
        AnalysisStatus::Safe,
        "{:?}",
        kinds(released)
    );
}

// ---- counted users ----

#[test]
fn balanced_additions_and_drops_hold_on_every_path() {
    let summary = analyse(
        r#"
int a3(struct u_queue *q, struct u_task *t)
{
    int ret;

    u_ref(t);
    ret = u_enqueue(q, t);
    if (ret == -1) {
        u_unref(t, 0);
        u_release(t);
        return -1;
    }
    return 0;
}

void a4(struct u_task *t)
{
    u_ref(t);
    u_log(1);
    u_unref(t, 0);
    u_release(t);
}
"#,
        "a3",
    );
    let a3 = function(&summary, "a3");
    assert_eq!(a3.status, AnalysisStatus::Safe, "{:?}", kinds(a3));
    assert!(
        verdicts(a3)
            .iter()
            .all(|(_, verdict)| *verdict == ClauseVerdict::Held),
        "{:?}",
        verdicts(a3)
    );
    assert_eq!(verdicts(a3).len(), 3);
    let a4 = function(&summary, "a4");
    assert_eq!(a4.status, AnalysisStatus::Safe, "{:?}", kinds(a4));
}

#[test]
fn the_c2_shape_is_refuted() {
    let summary = analyse(
        r#"
int c2(struct u_queue *q, struct u_task *t)
{
    int ret;

    u_ref(t);
    ret = u_enqueue(q, t);
    if (ret == -1) {
        return -1;
    }
    return 0;
}

int c2_undeclared(struct u_queue *q, struct u_task *t)
{
    int ret;

    u_ref(t);
    ret = u_enqueue(q, t);
    if (ret == -1) {
        return -1;
    }
    u_unref(t, 0);
    u_release(t);
    return 0;
}
"#,
        "c2",
    );
    let c2 = function(&summary, "c2");
    assert_eq!(c2.status, AnalysisStatus::Unsafe, "{:?}", kinds(c2));
    assert!(verdicts(c2).contains(&(
        "resource_user_exit(t, -1, restored)".to_owned(),
        ClauseVerdict::Refuted
    )));
    assert!(verdicts(c2).contains(&(
        "resource_user_exit(t, 0, added)".to_owned(),
        ClauseVerdict::Held
    )));
    let undeclared = function(&summary, "c2_undeclared");
    assert_eq!(
        undeclared.status,
        AnalysisStatus::Unsafe,
        "{:?}",
        kinds(undeclared)
    );
}

#[test]
fn the_c3_shape_is_refuted() {
    let summary = analyse(
        r#"
int c3(struct u_task *t)
{
    int ret;

    u_unref(t, 0);
    ret = u_flush(t);
    if (ret == -1) {
        return -1;
    }
    return 0;
}

int c3_checked(struct u_task *t)
{
    int ret;

    u_unref(t, 0);
    ret = u_flush(t);
    if (ret == -1) {
        u_release(t);
        return -1;
    }
    return 0;
}
"#,
        "c3",
    );
    let c3 = function(&summary, "c3");
    assert_eq!(c3.status, AnalysisStatus::Unsafe, "{:?}", kinds(c3));
    assert!(
        kinds(c3)
            .iter()
            .any(|kind| kind.contains("without a release check"))
    );
    // The control: the same body with the release check on the failure arm.
    // Its failure arm drops a user it did not add (net -1), which no clause
    // covers, so it is refuted as an uncovered residual rather than the C3
    // shape; the C3 diagnostic itself is gone.
    let checked = function(&summary, "c3_checked");
    assert!(
        !kinds(checked)
            .iter()
            .any(|kind| kind.contains("without a release check"))
    );
}

#[test]
fn a_rolled_back_addition_reported_as_success_is_refuted() {
    let summary = analyse(
        r#"
int n2(struct u_task *t)
{
    int ret;

    u_ref(t);
    ret = u_enqueue(0, t);
    if (ret == -1) {
        u_unref(t, 0);
    }
    return 0;
}
"#,
        "n2",
    );
    let n2 = function(&summary, "n2");
    assert_eq!(n2.status, AnalysisStatus::Unsafe, "{:?}", kinds(n2));
    assert!(verdicts(n2).contains(&(
        "resource_user_exit(t, 0, added)".to_owned(),
        ClauseVerdict::Refuted
    )));
    // A rollback of this path's own addition owes no release check.
    assert!(
        !kinds(n2)
            .iter()
            .any(|kind| kind.contains("without a release check"))
    );
}

#[test]
fn an_unattributed_operation_and_an_opaque_call_are_undecided() {
    let summary = analyse(
        r#"
void u_member(struct u_holder *h)
{
    u_ref(h->task);
}

void u_opaque(struct u_task *t)
{
    u_ref(t);
    u_unknown();
    u_unref(t, 0);
    u_release(t);
}
"#,
        "u_member",
    );
    let member = function(&summary, "u_member");
    assert_eq!(
        member.status,
        AnalysisStatus::Unknown,
        "{:?}",
        kinds(member)
    );
    assert!(
        kinds(member)
            .iter()
            .any(|kind| kind.contains("not a formal"))
    );
    let opaque = function(&summary, "u_opaque");
    assert_eq!(
        opaque.status,
        AnalysisStatus::Unknown,
        "{:?}",
        kinds(opaque)
    );
    assert!(
        verdicts(opaque)
            .iter()
            .any(|(_, verdict)| *verdict == ClauseVerdict::Undecided)
    );
}

#[test]
fn a_declared_operation_is_checked_against_its_body() {
    let summary = analyse(
        r#"
void inc_body(struct u_task *t)
{
    t->refs++;
}

void dec_body(struct u_task *t, int check)
{
    t->refs--;
    if (check == 1) {
        u_release(t);
    }
}

static int helper_neutral(struct u_queue *q)
{
    u_log(q->size);
    return 0;
}
"#,
        "inc_body",
    );
    let inc = function(&summary, "inc_body");
    assert!(
        verdicts(inc).contains(&(
            "resource_user_added(t, refs)".to_owned(),
            ClauseVerdict::Held
        )),
        "{:?} {:?}",
        verdicts(inc),
        kinds(inc)
    );
    let dec = function(&summary, "dec_body");
    assert!(
        verdicts(dec).contains(&(
            "resource_user_dropped(t, refs)".to_owned(),
            ClauseVerdict::Held
        )),
        "{:?} {:?}",
        verdicts(dec),
        kinds(dec)
    );
    let neutral = function(&summary, "helper_neutral");
    assert!(
        verdicts(neutral).contains(&("resource_user_neutral()".to_owned(), ClauseVerdict::Held))
    );
}

#[test]
fn a_declared_operation_the_body_contradicts_is_refuted() {
    let summary = analyse(
        r#"
void inc_body(struct u_task *t)
{
    t->refs--;
}
"#,
        "inc_body",
    );
    let inc = function(&summary, "inc_body");
    assert_eq!(inc.status, AnalysisStatus::Unsafe, "{:?}", kinds(inc));
}

#[test]
fn an_environment_without_counted_users_is_untouched() {
    let contracts = load_resource_contract_sources(&BTreeMap::from([(
        "plain.qcore".to_owned(),
        "module contract;\nextern fn u_log(n: i32) -> unit\ncontract {\n    requires resource_contract_v1;\n    ensures normal resource_borrowed(n);\n    ensures normal resource_return_none();\n};\n".to_owned(),
    )]))
    .unwrap();
    let source =
        format!("{PRELUDE}\nvoid plain(struct u_task *t)\n{{\n    t->refs++;\n    u_log(1);\n}}\n");
    let summary = analyze_legacy_c_translation_unit(&source, "plain", &contracts).unwrap();
    let plain = function(&summary, "plain");
    assert!(plain.counted_users.is_empty());
}

#[test]
fn the_loader_refuses_misplaced_clauses() {
    let refuse = |clause: &str, signature: &str| {
        let text = format!(
            "module contract;\nextern fn f({signature}) -> i32\ncontract {{\n    requires resource_contract_v1;\n{clause}    ensures normal resource_return_none();\n}};\n"
        );
        load_resource_contract_sources(&BTreeMap::from([("bad.qcore".to_owned(), text)])).is_err()
    };
    // A counted-user position must be a borrowed pointer.
    assert!(refuse(
        "    ensures normal resource_released(t, nonnull);\n    ensures normal resource_user_added(t, refs);\n",
        "t: ptr<mut, u8>"
    ));
    assert!(refuse(
        "    ensures normal resource_borrowed(n);\n    ensures normal resource_user_added(n, refs);\n",
        "n: i32"
    ));
    // Neutral stands alone.
    assert!(refuse(
        "    ensures normal resource_borrowed(t);\n    ensures normal resource_user_neutral();\n    ensures normal resource_user_exit(t, -1, restored);\n",
        "t: ptr<mut, u8>"
    ));
    // Canonical literals and delta words only.
    assert!(refuse(
        "    ensures normal resource_borrowed(t);\n    ensures normal resource_user_exit(t, -0, restored);\n",
        "t: ptr<mut, u8>"
    ));
    assert!(refuse(
        "    ensures normal resource_borrowed(t);\n    ensures normal resource_user_exit(t, 1, twice);\n",
        "t: ptr<mut, u8>"
    ));
    // A duplicate exit clause.
    assert!(refuse(
        "    ensures normal resource_borrowed(t);\n    ensures normal resource_user_exit(t, 0, added);\n    ensures normal resource_user_exit(t, 0, restored);\n",
        "t: ptr<mut, u8>"
    ));
    // A publication's container is a distinct borrowed pointer.
    assert!(refuse(
        "    ensures normal resource_borrowed(t);\n    ensures normal resource_published(t, t);\n",
        "t: ptr<mut, u8>"
    ));
}

#[test]
fn a_scalar_status_helper_keeps_custody_and_liveness_obligations() {
    let summary = analyse(
        r#"
int e_status(struct p_node *node) { return node->value; }

void e_live(void)
{
    struct p_node *node;
    node = p_malloc(sizeof(struct p_node));
    if (!node) { return; }
    e_status(node);
    p_free(node);
}

void e_dead(void)
{
    struct p_node *node;
    node = p_malloc(sizeof(struct p_node));
    if (!node) { return; }
    p_free(node);
    e_status(node);
}
"#,
        "e_live",
    );
    let live = function(&summary, "e_live");
    assert_eq!(live.status, AnalysisStatus::Safe, "{:?}", kinds(live));
    let dead = function(&summary, "e_dead");
    assert_ne!(dead.status, AnalysisStatus::Safe, "{:?}", kinds(dead));
}

#[test]
fn a_status_contract_with_the_wrong_c_declaration_is_withdrawn() {
    let source = PRELUDE.replace(
        "int e_status(struct p_node *node);",
        "int e_status(int value);",
    );
    let source = format!("{source}\nint e_bad(void) {{ return e_status(1); }}");
    let summary = analyze_legacy_c_translation_unit(&source, "e_bad", &contracts())
        .expect("the mismatched declaration is valid C");
    let bad = function(&summary, "e_bad");
    assert_ne!(bad.status, AnalysisStatus::Safe, "{:?}", kinds(bad));
    assert!(
        kinds(bad)
            .iter()
            .any(|kind| kind.contains("withdrawn against its C declaration")),
        "{:?}",
        kinds(bad)
    );
}
