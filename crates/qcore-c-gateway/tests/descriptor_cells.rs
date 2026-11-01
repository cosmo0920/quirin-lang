//! Card MK-EVENT-DESCRIPTOR-QUANT (gateway rule 38, contract specification
//! §C04.6.2; USER DECISION 2026-11-01, "I approve the changes of D1-D5."):
//! the descriptor cells of Monkey's timeout and channel operations, read on
//! route G under the provider's own restated declarations
//! (`providers/qcore-provider-libevent/contracts/monkey/event_loop.qv`) and the
//! synthetic allocator of `fixtures/contracts/event_loop.qv`. A `Safe` here is
//! `Safe` over those declared assumptions and promotes nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary,
    ResourceContracts, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

const PRELUDE: &str = "\
typedef long time_t;
struct mk_event_loop;
struct mk_event { int fd; int mask; };
struct timer { int timer_fd; struct mk_event event; };
struct chan { int r; int w; struct mk_event event; };
struct wrapper { struct mk_event event; int other; };
void *ev_alloc(__SIZE_TYPE__ n); void ev_free(void *p);
struct mk_event_loop *mk_event_loop_create(int size);
void mk_event_loop_destroy(struct mk_event_loop *loop);
int mk_event_timeout_create(struct mk_event_loop *loop, time_t sec, long nsec, void *data);
int mk_event_timeout_destroy(struct mk_event_loop *loop, void *data);
int mk_event_del(struct mk_event_loop *loop, struct mk_event *event);
int mk_event_channel_create(struct mk_event_loop *loop, int *r_fd, int *w_fd, void *data);
int mk_event_channel_destroy(struct mk_event_loop *loop, int r_fd, int w_fd, void *data);
";

fn environment() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        (
            "event_loop.qv".to_owned(),
            include_str!("fixtures/contracts/event_loop.qv").to_owned(),
        ),
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
            eprintln!("skipping Clang-dependent MK-EVENT-DESCRIPTOR-QUANT test: {reason}");
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
                                format!("{} @{}", description, d.line)
                            }
                            other => format!("{other:?} @{}", d.line),
                        })
                        .collect(),
                },
            )
        })
        .collect()
}

/// D2: a destroy on a cell of a caller's object is the identity on custody
/// (`P-CELL-STATE-NOT-FORWARDED`), as in `flb_sched_timer_cb_disable`.
#[test]
fn a_destroy_of_a_callers_cell_is_the_identity() {
    if !clang_available() {
        return;
    }
    let rows = rows(
        "int disable(struct mk_event_loop *loop, struct timer *timer) { if (timer->timer_fd != -1) { mk_event_timeout_destroy(loop, &timer->event); timer->timer_fd = -1; } return 0; }",
    );
    assert!(rows["disable"].safe(), "{:?}", rows["disable"]);
}

const TIMER_CYCLE: &str = "\
int cycle(void) {
    struct mk_event_loop *loop = mk_event_loop_create(8);
    if (!loop) { return -1; }
    struct timer *t = ev_alloc(sizeof(struct timer));
    if (!t) { mk_event_loop_destroy(loop); return -1; }
    int fd = mk_event_timeout_create(loop, 1, 0, &t->event);
    if (fd == -1) { mk_event_loop_destroy(loop); ev_free(t); return -1; }
    t->timer_fd = fd;
    BODY
    ev_free(t);
    mk_event_loop_destroy(loop);
    return 0;
}";

/// D1 and D2 together: a descriptor created into a cell of a tracked object,
/// its class decided by the witness, closed by the destroy, is discharged.
#[test]
fn a_witnessed_creation_closed_by_its_destroy_is_safe() {
    if !clang_available() {
        return;
    }
    let rows = rows(&TIMER_CYCLE.replace("BODY", "mk_event_timeout_destroy(loop, &t->event);"));
    assert!(rows["cycle"].safe(), "{:?}", rows["cycle"]);
}

#[test]
fn a_live_cell_refuses_its_owners_release_and_a_second_destroy() {
    if !clang_available() {
        return;
    }
    let leaked = rows(&TIMER_CYCLE.replace("BODY", "mk_event_del(loop, &t->event);"));
    assert!(!leaked["cycle"].safe(), "{:?}", leaked["cycle"]);
    assert!(
        leaked["cycle"].refused_with("descriptor-cell-owner-released"),
        "{:?}",
        leaked["cycle"]
    );
    let twice = rows(&TIMER_CYCLE.replace(
        "BODY",
        "mk_event_timeout_destroy(loop, &t->event); mk_event_timeout_destroy(loop, &t->event);",
    ));
    assert!(!twice["cycle"].safe(), "{:?}", twice["cycle"]);
    assert!(
        twice["cycle"].refused_with("descriptor-cell-double-release"),
        "{:?}",
        twice["cycle"]
    );
}

#[test]
fn an_untested_witness_never_discharges() {
    if !clang_available() {
        return;
    }
    let source = TIMER_CYCLE.replace(
        "    if (fd == -1) { mk_event_loop_destroy(loop); ev_free(t); return -1; }\n",
        "",
    );
    let rows = rows(&source.replace("BODY", "mk_event_timeout_destroy(loop, &t->event);"));
    assert!(!rows["cycle"].safe(), "{:?}", rows["cycle"]);
    assert!(
        rows["cycle"].refused_with("descriptor-cell-undecided"),
        "{:?}",
        rows["cycle"]
    );
}

#[test]
fn a_record_whose_first_member_is_the_event_refuses() {
    if !clang_available() {
        return;
    }
    let rows = rows(
        "int disable(struct mk_event_loop *loop, struct wrapper *w) { mk_event_timeout_destroy(loop, w); return 0; }",
    );
    assert!(!rows["disable"].safe(), "{:?}", rows["disable"]);
    assert!(
        rows["disable"].refused_with("descriptor-cell-undecided"),
        "{:?}",
        rows["disable"]
    );
}

const CHANNEL_CYCLE: &str = "\
int cycle(struct chan *c) {
    struct mk_event_loop *loop = mk_event_loop_create(8);
    if (!loop) { return -1; }
    int ret = mk_event_channel_create(loop, &c->r, &c->w, &c->event);
    if (ret != 0) { mk_event_loop_destroy(loop); return -1; }
    mk_event_channel_destroy(loop, c->R, c->w, &c->event);
    mk_event_loop_destroy(loop);
    return 0;
}";

/// D3 and D4: the member is a second name of the read end, so the guard is
/// decided equal and both ends close.
#[test]
fn a_channel_closed_through_its_own_read_end_is_safe() {
    if !clang_available() {
        return;
    }
    let rows = rows(&CHANNEL_CYCLE.replace("->R", "->r"));
    assert!(rows["cycle"].safe(), "{:?}", rows["cycle"]);
}

/// D4: a destroy naming another descriptor is decided distinct: nothing
/// closes, and the live cells refuse at the exit.
#[test]
fn a_channel_destroy_through_the_wrong_end_closes_nothing() {
    if !clang_available() {
        return;
    }
    let rows = rows(&CHANNEL_CYCLE.replace("->R", "->w"));
    assert!(!rows["cycle"].safe(), "{:?}", rows["cycle"]);
    assert!(
        rows["cycle"].refused_with("descriptor-cell-escapes"),
        "{:?}",
        rows["cycle"]
    );
}

/// D4: a destroy whose guard the body does not decide refuses.
#[test]
fn an_undecided_channel_guard_refuses() {
    if !clang_available() {
        return;
    }
    let rows = rows(
        "int teardown(struct mk_event_loop *loop, struct chan *c) { mk_event_channel_destroy(loop, c->r, c->w, &c->event); return 0; }",
    );
    assert!(!rows["teardown"].safe(), "{:?}", rows["teardown"]);
    assert!(
        rows["teardown"].refused_with("descriptor-cell-guard-undecided"),
        "{:?}",
        rows["teardown"]
    );
}

const CHILD: &str = "RULE38_SWITCH_CHILD";

/// Rule 38's switch (`QCORE_GATEWAY_RULES_OFF`, read once per process, so the
/// switched case runs this binary again as a child): off, the reading is not
/// asked and every call of a cell-bearing declaration refuses; off never
/// certifies what on refuses.
#[test]
fn switching_rule_38_off_refuses_every_cell_call() {
    if std::env::var_os(CHILD).is_some() || !clang_available() {
        return;
    }
    let status = std::process::Command::new(std::env::current_exe().expect("the test binary"))
        .args([
            "child_rule_38_off",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD, "child_rule_38_off")
        .env(qcore_c_gateway::gateway_rules::RULES_OFF_VARIABLE, "38")
        .status()
        .expect("the child test runs");
    assert!(status.success(), "child_rule_38_off: {status}");
}

#[test]
fn child_rule_38_off() {
    if std::env::var(CHILD).as_deref() != Ok("child_rule_38_off") {
        return;
    }
    let off = qcore_c_gateway::gateway_rules::check().expect("38 is registered");
    assert!(off.ids().contains("38"));
    let rows = rows(&TIMER_CYCLE.replace("BODY", "mk_event_timeout_destroy(loop, &t->event);"));
    assert!(!rows["cycle"].safe(), "{:?}", rows["cycle"]);
    assert!(
        rows["cycle"].refused_with("rule 38 is switched off"),
        "{:?}",
        rows["cycle"]
    );
}
