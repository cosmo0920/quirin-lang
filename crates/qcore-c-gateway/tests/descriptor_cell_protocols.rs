//! Card MK-EVENT-CELL-CROSS-FUNCTION (gateway rule 38c, contract
//! specification §C04.6.3, grammar G05e; USER DECISION 2026-11-02, verbatim:
//! "B as recommended"): declared record-type cell protocols, read on route G
//! under the provider's own Monkey declarations
//! (`providers/qcore-provider-libevent/contracts/monkey/event_loop.qv`) and the
//! synthetic declarations below, one fixture per census shape of
//! `docs/reports/mk-event-cell-cross-function/PROPOSAL.md` §1. A `Safe` here
//! is `Safe` over those declared assumptions, the protocol included, and
//! promotes nothing. Every test asserts the complete diagnostic vector.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

/// Synthetic callees and the protocol hosts: `struct chan` carries a `!= -1`
/// sentinel protocol (C1's shape), `struct flag_chan` a `== 1` flag (C4, C7),
/// `struct pos_chan` a `> 0` sentinel closed with `close` (C5, C8, C9).
const CONTRACTS: &str = "\
module contract;\n\
extern fn ev_alloc(n: usize) -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(n);\n\
           ensures normal resource_return_owned(nullable); };\n\
extern fn ev_free(p: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(p, nullable);\n\
           ensures normal resource_return_none(); };\n\
extern fn ev_note(p: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(p);\n\
           ensures normal resource_return_none(); };\n\
extern fn close(fd: i32) -> i32\n\
contract { requires resource_contract_v1; ensures normal resource_kind(fd, descriptor);\n\
           ensures normal resource_released(fd, valid); ensures normal resource_return_none(); };\n\
extern fn chan_protocols(c: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(c);\n\
           ensures normal resource_return_none();\n\
           quant cell #s: chan.ch[0] |-> #fresh(descriptor) when #s.ch[0] != -1;\n\
           quant cell #s: chan.ch[1] |-> #fresh(descriptor) when #s.ch[0] != -1;\n\
           quant cell #s: chan.event.fd |-> #same(ch[0]) when #s.ch[0] != -1;\n\
           quant cell #f: flag_chan.n[0] |-> #fresh(descriptor) when #f.on == 1;\n\
           quant cell #f: flag_chan.n[1] |-> #fresh(descriptor) when #f.on == 1;\n\
           quant cell #f: flag_chan.event.fd |-> #same(n[0]) when #f.on == 1;\n\
           quant cell #p: pos_chan.e[0] |-> #fresh(descriptor) when #p.e[0] > 0;\n\
           quant cell #p: pos_chan.e[1] |-> #fresh(descriptor) when #p.e[0] > 0;\n\
           quant cell #p: pos_chan.event.fd |-> #same(e[0]) when #p.e[0] > 0; };";

const PRELUDE: &str = "\
struct mk_event_loop;
struct mk_event { int fd; int mask; };
struct chan { int ch[2]; struct mk_event_loop *evl; struct mk_event event; };
struct flag_chan { int n[2]; int on; struct mk_event_loop *evl; struct mk_event event; };
struct pos_chan { int e[2]; struct mk_event_loop *evl; struct mk_event event; };
struct other { int ch[2]; struct mk_event_loop *evl; struct mk_event event; };
void *ev_alloc(__SIZE_TYPE__ n); void ev_free(void *p); void ev_note(void *p);
int close(int fd);
void chan_protocols(void *c);
struct mk_event_loop *mk_event_loop_create(int size);
void mk_event_loop_destroy(struct mk_event_loop *loop);
int mk_event_channel_create(struct mk_event_loop *loop, int *r_fd, int *w_fd, void *data);
int mk_event_channel_destroy(struct mk_event_loop *loop, int r_fd, int w_fd, void *data);
";

fn environment() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        ("protocols.qv".to_owned(), CONTRACTS.to_owned()),
        (
            "monkey/event_loop.qv".to_owned(),
            include_str!(
                "../../../providers/qcore-provider-libevent/contracts/monkey/event_loop.qv"
            )
            .to_owned(),
        ),
    ]))
    .expect("the contracts load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent MK-EVENT-CELL-CROSS-FUNCTION test: {reason}");
            false
        }
    }
}

fn summary_of(outcome: &UnrestrictedLegacyResult) -> Option<&LegacyProgramSummary> {
    match outcome {
        UnrestrictedLegacyResult::Safe(certificate) => Some(&certificate.summary),
        UnrestrictedLegacyResult::Unsafe(summary) => Some(summary),
        UnrestrictedLegacyResult::Unknown { partial, .. } => partial.as_ref(),
    }
}

/// The entry row's status and its sorted diagnostics (`kind:text`).
fn outcome(body: &str) -> (String, Vec<String>) {
    let text = format!("{PRELUDE}\n{body}\n");
    let entry = body
        .split('(')
        .next()
        .and_then(|head| head.split_whitespace().last())
        .expect("a definition")
        .trim_start_matches('*')
        .to_owned();
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        &entry,
        &environment(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &result {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let summary = summary_of(&result).unwrap_or_else(|| panic!("no summary: {result:?}"));
    let row = &summary.functions[&entry];
    let mut diagnostics = row
        .diagnostics
        .iter()
        .map(|d| match &d.kind {
            LegacyDiagnosticKind::Unsupported { description } => {
                format!("unsupported:{description}")
            }
            LegacyDiagnosticKind::UnknownAlias { variable } => format!("unknown_alias:{variable}"),
            other => format!("{other:?}"),
        })
        .collect::<Vec<_>>();
    diagnostics.sort();
    diagnostics.dedup();
    (format!("{:?}", row.status), diagnostics)
}

fn expect(label: &str, body: &str, status: &str, diagnostics: &[&str]) {
    if !clang_available() {
        return;
    }
    let (actual_status, actual) = outcome(body);
    let mut wanted = diagnostics
        .iter()
        .map(|text| (*text).to_owned())
        .collect::<Vec<_>>();
    wanted.sort();
    assert_eq!(
        (actual_status.as_str(), &actual),
        (status, &wanted),
        "{label}\n{body}"
    );
}

const GUARD_UNDECIDED: &str = "unsupported:call to `mk_event_channel_destroy`: descriptor cell `(Interior(\"s\", [\"event\"], true) as mk_event).fd` is not decided (descriptor-cell-guard-undecided)";

fn broken(root: &str, record: &str) -> String {
    format!(
        "unsupported:`{root}` (a `struct {record}`) does not keep its declared cell protocol here (descriptor-cell-protocol-broken)"
    )
}

// -- C1: a `!= -1` sentinel, destroyed in another function -------------------

/// `flb_sched_destroy`'s teardown over a caller's record: the test's live
/// edge introduces the caller's three names (one identity for `ch[0]` and
/// `event.fd`), so the member-equality guard is decided equal, both ends
/// close, and the sentinel stores restore the protocol. The dead edge does
/// nothing. The record keeps its protocol at the exit.
#[test]
fn a_sentinel_teardown_of_a_callers_record_is_decided() {
    expect(
        "C1 destroy",
        "int d(struct chan *s) { if (s->ch[0] != -1) { \
         mk_event_channel_destroy(s->evl, s->ch[0], s->ch[1], &s->event); \
         s->ch[0] = -1; s->ch[1] = -1; } return 0; }",
        "Safe",
        &[],
    );
}

/// The complement spelling of the test reads the same edges.
#[test]
fn the_complement_test_reads_the_other_edge() {
    expect(
        "C1 destroy, `== -1`",
        "int d(struct chan *s) { if (s->ch[0] == -1) { return 0; } \
         mk_event_channel_destroy(s->evl, s->ch[0], s->ch[1], &s->event); \
         s->ch[0] = -1; return 0; }",
        "Safe",
        &[],
    );
}

/// The ends close but the sentinel is not restored: the guard still says
/// live over closed cells, so the protocol is broken at the exit.
#[test]
fn a_teardown_that_leaves_the_sentinel_refuses() {
    expect(
        "C1 destroy without the sentinel store",
        "int d(struct chan *s) { if (s->ch[0] != -1) { \
         mk_event_channel_destroy(s->evl, s->ch[0], s->ch[1], &s->event); } return 0; }",
        "Unknown",
        &[&broken("s", "chan")],
    );
}

/// A second destroy after the first never decides: its guard names two
/// stale identities, so rule 38 (5) refuses (never two closes).
#[test]
fn a_second_destroy_never_decides() {
    expect(
        "C1 destroy twice",
        "int d(struct chan *s) { if (s->ch[0] != -1) { \
         mk_event_channel_destroy(s->evl, s->ch[0], s->ch[1], &s->event); \
         mk_event_channel_destroy(s->evl, s->ch[0], s->ch[1], &s->event); \
         s->ch[0] = -1; } return 0; }",
        "Unknown",
        &[GUARD_UNDECIDED],
    );
}

/// Without the test the caller's state is unknown: the guard keeps rule
/// 38's refusal (an unresolved forward never decides). The refusal ends
/// the path, so nothing after it is read.
#[test]
fn an_untested_teardown_keeps_the_guard_refusal() {
    expect(
        "C1 destroy without a test",
        "int d(struct chan *s) { \
         mk_event_channel_destroy(s->evl, s->ch[0], s->ch[1], &s->event); \
         s->ch[0] = -1; return 0; }",
        "Unknown",
        &[GUARD_UNDECIDED],
    );
}

/// A record type with no protocol is read as before: the guard refuses.
#[test]
fn a_record_without_a_protocol_is_unchanged() {
    expect(
        "the same teardown over `struct other`",
        "int d(struct other *s) { if (s->ch[0] != -1) { \
         mk_event_channel_destroy(s->evl, s->ch[0], s->ch[1], &s->event); \
         s->ch[0] = -1; } return 0; }",
        "Unknown",
        &[GUARD_UNDECIDED],
    );
}

/// The teardown on one arm only, without its sentinel store, joins with the
/// untouched arm into a conflict: refused at the exit.
#[test]
fn a_conditional_teardown_that_breaks_the_protocol_refuses() {
    expect(
        "C1 destroy on one arm",
        "int d(struct chan *s, int k) { if (s->ch[0] != -1) { if (k) { \
         mk_event_channel_destroy(s->evl, s->ch[0], s->ch[1], &s->event); } } return 0; }",
        "Unknown",
        &[&broken("s", "chan")],
    );
}

// -- C1's creator --------------------------------------------------------------

/// A creator into a caller's record whose test says no cell is live: the
/// channel's cells are forwarded under the protocol on both classes (no
/// `descriptor-cell-escapes`). The registration in the caller's loop keeps
/// rule 12t's refusal, the registration side of
/// `FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME`, which this card does not touch.
#[test]
fn a_creator_into_a_dead_record_forwards_the_cells() {
    expect(
        "C1 create",
        "int mk(struct chan *s, struct mk_event_loop *loop) { \
         if (s->ch[0] != -1) { return -1; } \
         int ret = mk_event_channel_create(loop, &s->ch[0], &s->ch[1], &s->event); \
         if (ret != 0) { return -1; } return 0; }",
        "Unknown",
        &[
            "unsupported:call to `mk_event_channel_create` retains `& s - > event` in `loop`, whose lifetime this body does not decide against it (retention-undecided)",
        ],
    );
}

/// `flb_sched_create`'s shape over a record this body allocates: the member
/// stores of the sentinels keep their existing refusal (the stored base is
/// an unresolved identity), which ends the path before the creation. The
/// refusal is not rule 38c's and holds with or without the protocol.
#[test]
fn a_tracked_record_creator_keeps_the_member_store_refusal() {
    expect(
        "C1 create into an allocated record",
        "struct chan *mk(struct mk_event_loop *loop) { \
         struct chan *c = ev_alloc(sizeof(struct chan)); if (!c) { return 0; } \
         c->ch[0] = -1; c->ch[1] = -1; \
         int ret = mk_event_channel_create(loop, &c->ch[0], &c->ch[1], &c->event); \
         if (ret != 0) { ev_free(c); return 0; } return c; }",
        "Unknown",
        &["unknown_alias:c"],
    );
}

/// Without the dead stores the allocated record's prior state is unknown,
/// so the creation may overwrite a live descriptor and refuses
/// (`descriptor-cell-undecided`); the failure arm's release of the record
/// and the exit then refuse too. Nothing here is `Safe`.
#[test]
fn a_creator_that_does_not_kill_the_sentinel_first_refuses() {
    expect(
        "C1 create without the stores",
        "struct chan *mk(struct mk_event_loop *loop) { \
         struct chan *c = ev_alloc(sizeof(struct chan)); if (!c) { return 0; } \
         int ret = mk_event_channel_create(loop, &c->ch[0], &c->ch[1], &c->event); \
         if (ret != 0) { ev_free(c); return 0; } return c; }",
        "Unknown",
        &[
            &broken("an object this body tracks", "chan"),
            "unsupported:`c` is released while a cell of it may name a live descriptor (descriptor-cell-owner-released)",
            "unsupported:call to `mk_event_channel_create` retains `& c - > event` in `loop`, whose lifetime this body does not decide against it (retention-undecided)",
            "unsupported:call to `mk_event_channel_create`: descriptor cell `(Name(\"c\") as int).ch[0]` is not decided (descriptor-cell-undecided)",
            "unsupported:call to `mk_event_channel_create`: descriptor cell `(Name(\"c\") as int).ch[1]` is not decided (descriptor-cell-undecided)",
        ],
    );
}

// -- C4, C7: a `== 1` flag ---------------------------------------------------

/// `input_thread_instance_destroy`'s teardown: the flag's live edge decides
/// the guard, and storing the flag dead after the close keeps the protocol.
#[test]
fn a_flag_teardown_is_decided() {
    expect(
        "C7 destroy",
        "int d(struct flag_chan *s) { if (s->on == 1) { \
         mk_event_channel_destroy(s->evl, s->n[0], s->n[1], &s->event); s->on = 0; } return 0; }",
        "Safe",
        &[],
    );
}

/// Storing the flag dead while the cells are still live breaks it.
#[test]
fn a_flag_stored_dead_over_live_cells_refuses() {
    expect(
        "C7 flag cleared first",
        "int d(struct flag_chan *s) { if (s->on == 1) { s->on = 0; } return 0; }",
        "Unknown",
        &[
            "unsupported:descriptor cell `s->on` is not decided (descriptor-cell-protocol-broken)",
            &broken("s", "flag_chan"),
        ],
    );
}

/// A creator into a caller's record must know the flag dead first; then the
/// flag stored live over the new cells keeps the protocol on both classes.
#[test]
fn a_flag_creator_keeps_the_protocol() {
    expect(
        "C4 create",
        "int mk(struct flag_chan *s, struct mk_event_loop *loop) { \
         if (s->on == 1) { return -1; } \
         int ret = mk_event_channel_create(loop, &s->n[0], &s->n[1], &s->event); \
         if (ret != 0) { return -1; } s->on = 1; return 0; }",
        "Unknown",
        &[
            "unsupported:call to `mk_event_channel_create` retains `& s - > event` in `loop`, whose lifetime this body does not decide against it (retention-undecided)",
        ],
    );
}

// -- C5, C8, C9: a `> 0` sentinel closed with `close` (Q3) -------------------

/// `flb_input_instance_destroy`'s ends. Rule 38c reads each `close` of a
/// cell as its `#stale` (decision Q3; the package vector `release`), but the
/// access gate refuses an integer member at `close`'s custody position
/// (W44 premise 1) before that, and the refusal ends the path. This rests
/// on the gate's refusal, not on rule 38c: admitting the actual there is
/// the follow-up recorded in the report.
#[test]
fn a_close_of_a_protocol_cell_keeps_the_access_gate_refusal() {
    expect(
        "C8 closes",
        "int d(struct pos_chan *s) { if (s->e[0] > 0) { close(s->e[0]); close(s->e[1]); \
         s->e[0] = -1; } return 0; }",
        "Unknown",
        &["unsupported:indirect memory access is outside the resource projection"],
    );
}

// -- Transport at a call ------------------------------------------------------

/// A record that keeps its protocol is forwarded under it at a call that
/// may reach it; after the call the body knows nothing of it (the callee may
/// have closed it), so a destroy keeps rule 38's refusal.
#[test]
fn a_call_forwards_a_record_that_keeps_its_protocol() {
    expect(
        "forwarded at `ev_note(s)`",
        "int d(struct chan *s) { if (s->ch[0] != -1) { ev_note(s); \
         mk_event_channel_destroy(s->evl, s->ch[0], s->ch[1], &s->event); \
         s->ch[0] = -1; } return 0; }",
        "Unknown",
        &[GUARD_UNDECIDED],
    );
}

/// A record that does not keep its protocol refuses at the call.
#[test]
fn a_call_reaching_a_broken_record_refuses() {
    expect(
        "broken at `ev_note(s)`",
        "int d(struct chan *s) { if (s->ch[0] != -1) { \
         mk_event_channel_destroy(s->evl, s->ch[0], s->ch[1], &s->event); ev_note(s); \
         s->ch[0] = -1; } return 0; }",
        "Unknown",
        &[&broken("s", "chan")],
    );
}
