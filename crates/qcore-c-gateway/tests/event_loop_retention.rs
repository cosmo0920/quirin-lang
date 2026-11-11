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

//! Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION (gateway rules 12t and 12n;
//! USER DECISIONS 2026-10-16: "Tracked holder path", "R3: quant state",
//! "Result + list"): the brief's controls on route G — a path holder, an
//! unresolved path, a loop destroyed before deregistration, a removal of one
//! registration, and a registration forwarded through an id. The `ev_*` /
//! `e_*` callees (`fixtures/contracts/event_loop.qv`) are synthetic trusted
//! test assumptions: a `Safe` here is `Safe` over them and promotes nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const PRELUDE: &str = "\
struct ev_loop; struct obj { int x; };
struct cfg { struct ev_loop *evl; };
struct timer { int fd; struct cfg *sched; struct obj event; };
struct coll { int id; struct ev_loop *evl; struct obj event; struct obj timer; struct coll *next; };
void *ev_alloc(__SIZE_TYPE__ n); void ev_free(void *p);
struct ev_loop *ev_loop_create(void); void ev_loop_destroy(struct ev_loop *loop);
int ev_add(struct ev_loop *loop, void *data); int ev_del(struct ev_loop *loop, void *data);
int e_start(struct coll *coll, struct cfg *config); void ev_touch(struct coll *coll);
struct list { struct list *prev, *next; };
void e_list_add(struct list *entry, struct list *head);
struct e_coll { int id; int (*cb_collect)(void); long seconds; struct list _head; };
struct ins { struct list collectors; };
int e_set(struct ins *ins, int (*cb)(void), __INT64_TYPE__ seconds, struct cfg *config);
int e_set_bad(struct ins *ins, int (*cb)(void), __INT64_TYPE__ seconds, struct cfg *config);
";

fn environment() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([(
        "event_loop.qcore".to_owned(),
        include_str!("fixtures/contracts/event_loop.qv").to_owned(),
    )]))
    .expect("the fixture contracts load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent FB-EVENT-LOOP-RETENTION test: {reason}");
            false
        }
    }
}

#[derive(Debug)]
struct Row {
    status: AnalysisStatus,
    unsupported: Vec<String>,
}

impl Row {
    fn safe(&self) -> bool {
        self.status == AnalysisStatus::Safe
    }
    fn refused_with(&self, key: &str) -> bool {
        self.unsupported.iter().any(|d| d.contains(key))
    }
}

fn summary_of(outcome: &UnrestrictedLegacyResult) -> Option<&LegacyProgramSummary> {
    match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    }
}

fn rows(source: &str) -> BTreeMap<String, Row> {
    let text = format!("{PRELUDE}\n{source}\n");
    let entry = source
        .split('(')
        .next()
        .and_then(|head| head.split_whitespace().last())
        .expect("a definition")
        .trim_start_matches('*')
        .to_owned();
    let outcome = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        &entry,
        &environment(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &outcome {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let summary = summary_of(&outcome).unwrap_or_else(|| panic!("no summary: {outcome:?}"));
    summary
        .functions
        .iter()
        .map(|(name, row)| {
            (
                name.clone(),
                Row {
                    status: row.status,
                    unsupported: row
                        .diagnostics
                        .iter()
                        .map(|d| match &d.kind {
                            LegacyDiagnosticKind::Unsupported { description } => {
                                description.clone()
                            }
                            other => format!("{other:?}"),
                        })
                        .collect(),
                },
            )
        })
        .collect()
}

#[test]
fn a_path_holder_removed_from_exactly_its_path_is_safe() {
    if !clang_available() {
        return;
    }
    let rows = rows(
        "void cycle(struct coll *coll) { ev_add(coll->evl, &coll->event); ev_del(coll->evl, &coll->event); }",
    );
    assert!(rows["cycle"].safe(), "{:?}", rows["cycle"]);
}

#[test]
fn a_path_holder_still_holding_at_the_exit_is_refused() {
    if !clang_available() {
        return;
    }
    let rows = rows(
        "void start(struct coll *coll) { ev_add(coll->evl, &coll->event); }\n\
         void elsewhere(struct coll *coll, struct cfg *config) { ev_add(coll->evl, &coll->event); ev_del(config->evl, &coll->event); }\n\
         void one_of_two(struct coll *coll) { ev_add(coll->evl, &coll->event); ev_add(coll->evl, &coll->timer); ev_del(coll->evl, &coll->event); }",
    );
    assert!(
        rows["start"].refused_with("retention-undecided"),
        "{:?}",
        rows["start"]
    );
    // Card RETENTION-UNMATCHED-REMOVAL-POISONS: a removal through another
    // path poisons the record; a removal of another caller's member poisons
    // the remaining one (two caller objects are not known to be distinct).
    for name in ["start", "elsewhere", "one_of_two"] {
        assert!(!rows[name].safe(), "{name}: {:?}", rows[name]);
    }
    for name in ["elsewhere", "one_of_two"] {
        assert!(
            rows[name].refused_with("retention-removal-poisoned"),
            "{name}: {:?}",
            rows[name]
        );
    }
}

#[test]
fn an_unresolved_or_restored_path_is_undecided() {
    if !clang_available() {
        return;
    }
    let rows = rows(
        "void through_local(struct coll *coll) { struct coll *c = coll->next; ev_add(c->evl, &c->event); ev_del(c->evl, &c->event); }\n\
         void restored(struct coll *coll, struct ev_loop *other) { ev_add(coll->evl, &coll->event); coll->evl = other; ev_del(coll->evl, &coll->event); }\n\
         void handed(struct coll *coll) { ev_add(coll->evl, &coll->event); ev_touch(coll); ev_del(coll->evl, &coll->event); }",
    );
    // A path through a local is no holder this body can name (here an older
    // route already refuses the local first); a store into the path's member
    // makes the path name no one object.
    assert!(!rows["through_local"].safe(), "{:?}", rows["through_local"]);
    assert!(!rows["restored"].safe(), "{:?}", rows["restored"]);
    // A formal handed whole to a call may have its member rewritten there:
    // the path names no one object, so the registration is undecided (its
    // refusal leaves the access gate's own refusal of the member actual in
    // place, rule 12t's description at the call is not reported).
    assert!(!rows["handed"].safe(), "{:?}", rows["handed"]);
}

#[test]
fn a_removal_with_no_record_is_the_identity() {
    if !clang_available() {
        return;
    }
    let rows = rows(
        "void stop(struct coll *coll) { ev_del(coll->evl, &coll->event); }\n\
         void stop_unresolved(struct coll *coll) { struct coll *c = coll->next; ev_del(c->evl, &c->event); }\n\
         void disable(struct timer *timer) { ev_del(timer->sched->evl, &timer->event); }\n\
         void disable_then_store(struct timer *timer) { if (timer->fd != -1) { ev_del(timer->sched->evl, &timer->event); timer->fd = -1; } }",
    );
    assert!(rows["stop"].safe(), "{:?}", rows["stop"]);
    // The loop two members deep: the removal with no record is the identity.
    assert!(rows["disable"].safe(), "{:?}", rows["disable"]);
    // Deliberate pin update, LEXICAL-ROUTES-PREEMPT-TYPED-FALLBACKS:
    // the no-record removal is still the identity, and its following scalar
    // store now has an independently discharged Clang plan. W42's old base
    // refusal no longer preempts that plan; no retention record is erased.
    assert!(
        rows["disable_then_store"].safe(),
        "{:?}",
        rows["disable_then_store"]
    );
}

#[test]
fn an_interior_of_a_frame_object_is_never_a_callers_object() {
    if !clang_available() {
        return;
    }
    // `&h.event` points into this frame's own object: neither a caller's
    // object (which would outlive a frame holder) nor a tracked identity, so
    // the retention is undecided.
    let rows = rows(
        "void frame_interior(void) { struct ev_loop *l = ev_loop_create(); if (!l) return; struct coll h; ev_add(l, &h.event); ev_loop_destroy(l); }",
    );
    assert!(
        !rows["frame_interior"].safe(),
        "{:?}",
        rows["frame_interior"]
    );
}

#[test]
fn a_loop_destroyed_first_ends_its_records_and_a_release_while_held_refuses() {
    if !clang_available() {
        return;
    }
    let rows = rows(
        "void loop_first(void) { struct ev_loop *l = ev_loop_create(); if (!l) return; void *o = ev_alloc(8); if (!o) { ev_loop_destroy(l); return; } ev_add(l, o); ev_loop_destroy(l); ev_free(o); }\n\
         void object_first(void) { struct ev_loop *l = ev_loop_create(); if (!l) return; void *o = ev_alloc(8); if (!o) { ev_loop_destroy(l); return; } ev_add(l, o); ev_free(o); ev_loop_destroy(l); }\n\
         void destroyed_then_removed(void) { struct ev_loop *l = ev_loop_create(); if (!l) return; void *o = ev_alloc(8); if (!o) { ev_loop_destroy(l); return; } ev_add(l, o); ev_loop_destroy(l); ev_del(l, o); ev_free(o); }",
    );
    assert!(rows["loop_first"].safe(), "{:?}", rows["loop_first"]);
    assert!(
        rows["object_first"].refused_with("retention-released-while-held"),
        "{:?}",
        rows["object_first"]
    );
    assert!(
        !rows["destroyed_then_removed"].safe(),
        "{:?}",
        rows["destroyed_then_removed"]
    );
}

#[test]
fn a_declared_path_holder_refuses_at_its_caller() {
    if !clang_available() {
        return;
    }
    let rows =
        rows("int caller(struct coll *coll, struct cfg *config) { return e_start(coll, config); }");
    assert!(!rows["caller"].safe(), "{:?}", rows["caller"]);
    assert!(
        rows["caller"].refused_with("retention-undecided"),
        "{:?}",
        rows["caller"]
    );
}

const SETTERS: &str = "\
static struct e_coll *e_create(struct ins *ins, int (*cb)(void), struct cfg *config) { struct e_coll *coll; coll = ev_alloc(sizeof(struct e_coll)); if (!coll) { return (void *)0; } coll->id = 1; coll->cb_collect = cb; e_list_add(&coll->_head, &ins->collectors); return coll; }
int e_set(struct ins *ins, int (*cb)(void), __INT64_TYPE__ seconds, struct cfg *config) { struct e_coll *coll; coll = e_create(ins, cb, config); if (!coll) { return -1; } coll->seconds = seconds; return coll->id; }
int e_set_bad(struct ins *ins, int (*cb)(void), __INT64_TYPE__ seconds, struct cfg *config) { struct e_coll *coll; coll = e_create(ins, cb, config); if (!coll) { return -1; } coll->seconds = seconds; return 0; }
static int collect(void) { return 0; }
";

#[test]
fn a_registration_forwarded_through_an_id_stands_only_on_its_body() {
    if !clang_available() {
        return;
    }
    let source = format!(
        "{SETTERS}int plugin_init(struct ins *ins, struct cfg *config) {{ return e_set(ins, collect, 1, config); }}\n\
         int plugin_init_bad(struct ins *ins, struct cfg *config) {{ return e_set_bad(ins, collect, 1, config); }}"
    );
    let rows = rows(&source);
    assert!(rows["plugin_init"].safe(), "{:?}", rows["plugin_init"]);
    assert!(
        rows["plugin_init_bad"].refused_with("callback-withdrawn")
            || rows["plugin_init_bad"]
                .unsupported
                .iter()
                .any(|d| d.contains("does not validate the declared callback clause")),
        "{:?}",
        rows["plugin_init_bad"]
    );
}

/// Card RETENTION-UNMATCHED-REMOVAL-POISONS (rule 12t, USER DECISION
/// 2026-10-17): two tracked loops `l` and `m` and one tracked object `o`
/// registered in `l`. The prefix allocates and unwinds every failure.
const TWO_LOOPS: &str = "struct ev_loop *l = ev_loop_create(); if (!l) return; \
     struct ev_loop *m = ev_loop_create(); if (!m) { ev_loop_destroy(l); return; } \
     void *o = ev_alloc(8); if (!o) { ev_loop_destroy(m); ev_loop_destroy(l); return; } \
     ev_add(l, o);";

#[test]
fn an_unmatched_removal_poisons_the_record() {
    if !clang_available() {
        return;
    }
    let source = format!(
        "void old_sequence(void) {{ {TWO_LOOPS} ev_del(m, o); ev_del(l, o); ev_free(o); ev_loop_destroy(m); ev_loop_destroy(l); }}\n\
         void matched(void) {{ {TWO_LOOPS} ev_del(l, o); ev_free(o); ev_loop_destroy(m); ev_loop_destroy(l); }}\n\
         void destroyed(void) {{ {TWO_LOOPS} ev_loop_destroy(l); ev_free(o); ev_loop_destroy(m); }}\n\
         void poisoned_then_destroyed(void) {{ {TWO_LOOPS} ev_del(m, o); ev_loop_destroy(l); ev_free(o); ev_loop_destroy(m); }}\n\
         void poisoned_removed_then_destroyed(void) {{ {TWO_LOOPS} ev_del(m, o); ev_del(l, o); ev_loop_destroy(l); ev_free(o); ev_loop_destroy(m); }}"
    );
    let rows = rows(&source);
    // Unmatched removal, matched removal, release: the right loop may still
    // hold `o`, so its release refuses (never Unsafe).
    assert!(!rows["old_sequence"].safe(), "{:?}", rows["old_sequence"]);
    assert!(
        rows["old_sequence"].refused_with("retention-released-while-held"),
        "{:?}",
        rows["old_sequence"]
    );
    assert_ne!(rows["old_sequence"].status, AnalysisStatus::Unsafe);
    // Matched removal, then release: decides.
    assert!(rows["matched"].safe(), "{:?}", rows["matched"]);
    // The loop's destroy, then release: decides.
    assert!(rows["destroyed"].safe(), "{:?}", rows["destroyed"]);
    // Poisoned, then the loop destroyed, then release: decides.
    assert!(
        rows["poisoned_then_destroyed"].safe(),
        "{:?}",
        rows["poisoned_then_destroyed"]
    );
    assert!(
        rows["poisoned_removed_then_destroyed"].safe(),
        "{:?}",
        rows["poisoned_removed_then_destroyed"]
    );
}

#[test]
fn a_poisoned_path_record_names_its_reason_at_the_exit() {
    if !clang_available() {
        return;
    }
    let rows = rows(
        "void old_path(struct coll *coll, struct cfg *config) { ev_add(coll->evl, &coll->event); ev_del(config->evl, &coll->event); ev_del(coll->evl, &coll->event); }\n\
         void own_path(struct coll *coll) { ev_add(coll->evl, &coll->event); ev_del(coll->evl, &coll->event); }\n\
         void other_member(struct coll *coll, struct cfg *config) { ev_add(coll->evl, &coll->event); ev_del(config->evl, &coll->timer); ev_del(coll->evl, &coll->event); }",
    );
    assert!(!rows["old_path"].safe(), "{:?}", rows["old_path"]);
    assert!(
        rows["old_path"].refused_with("retention-removal-poisoned"),
        "{:?}",
        rows["old_path"]
    );
    assert!(rows["own_path"].safe(), "{:?}", rows["own_path"]);
    // A caller's objects are not known to be distinct (a union, an alias):
    // a removal of another member through another loop poisons too.
    assert!(
        rows["other_member"].refused_with("retention-removal-poisoned"),
        "{:?}",
        rows["other_member"]
    );
}

#[test]
fn a_loop_destroy_at_a_path_stays_refused() {
    if !clang_available() {
        return;
    }
    // The destroy releases a member actual: a custody-taking position the
    // access gate keeps refused, so the call decides nothing on route G.
    let rows = rows(
        "void path_destroy(struct coll *coll) { ev_add(coll->evl, &coll->event); ev_loop_destroy(coll->evl); }",
    );
    assert!(!rows["path_destroy"].safe(), "{:?}", rows["path_destroy"]);
    assert_ne!(rows["path_destroy"].status, AnalysisStatus::Unsafe);
}

#[test]
fn the_shared_vectors_hold() {
    use qcore_c_gateway::retained_by::{
        Outcome, PATH_DECISION_VECTORS, PATH_TRACE_VECTORS, path_trace, replay,
    };
    assert_eq!(PATH_DECISION_VECTORS.len(), 9);
    assert_eq!(PATH_TRACE_VECTORS.len(), 17);
    for (name, safe) in PATH_TRACE_VECTORS {
        assert_eq!(replay(&path_trace(name)) == Outcome::Safe, *safe, "{name}");
    }
    assert_eq!(qcore_c_gateway::registration_by_id::VECTORS.len(), 7);
}
