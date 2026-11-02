//! Card CELL-PROTOCOL-FB-DECLARATIONS (gateway rule 38c, contract
//! specification §C04.6.3, grammar G05i; USER DECISION 2026-11-02, verbatim:
//! "B as recommended"): the production `quant cell` declaration of Fluent
//! Bit's `struct flb_input_thread_instance`
//! (`providers/quirin-provider-fluent-bit/contracts/fluent_bit/teardown_destroy.qv`,
//! host `flb_config_set_program_name`), read on route G under the provider's
//! own modules. The fixture bodies are Fluent Bit's (`src/flb_input_thread.c`,
//! `src/flb_scheduler.c`, `src/flb_engine.c`, byte-identical at 66910c10,
//! 046accf6 and 34155955 in the statements kept here), with the prelude
//! reduced to the members they read. A `Safe` here is `Safe` over the
//! declared assumptions, the protocol included, and promotes nothing. Every
//! test asserts the complete diagnostic vector.
//!
//! The `> 0` closes of `input_thread_instance_destroy` (`ch_parent_events`,
//! `ch_thread_events`) are left out of its fixture: they are no protocol
//! cell, and their refusal is the access gate's (W44 premise 1), the area of
//! the concurrent card CELL-PROTOCOL-CLOSE-ACTUAL.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

/// The production declarations: the protocol's host module and the Monkey
/// event loop.
const TEARDOWN: &str = include_str!(
    "../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/teardown_destroy.qv"
);
const EVENT_LOOP: &str =
    include_str!("../../../providers/qcore-provider-libevent/contracts/monkey/event_loop.qv");

/// The allocator and `close` as the selected modules state them
/// (`fluent_bit/resource_memory.qv` inherits libc's `calloc`/`free`, whose
/// clauses these are; `libc/unistd.qv`'s `close`), spelled here because the
/// test loader does not resolve `contract inherit`.
const LIBRARY: &str = "\
module contract;\n\
extern fn flb_calloc(count: usize, size: usize) -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(count);\n\
           ensures normal resource_borrowed(size); ensures normal resource_return_owned(nullable); };\n\
extern fn flb_free(ptr: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(ptr, nullable);\n\
           ensures normal resource_return_none(); };\n\
extern fn close(fd: i32) -> i32\n\
contract { requires resource_contract_v1; ensures normal resource_released(fd, valid);\n\
           ensures normal resource_kind(fd, descriptor); ensures normal resource_return_none(); };";

/// The protocols the census found kept by every store but withheld from
/// production (the host module's comment says why); used only by the ignored
/// `withheld_protocols_reading`, which records rule 38c's reading of them.
const WITHHELD: &str = "\
module contract;\n\
extern fn withheld_protocols(c: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(c);\n\
           ensures normal resource_return_none();\n\
           quant cell #s: flb_sched.ch_events[0] |-> #fresh(descriptor) when #s.ch_events[0] != -1;\n\
           quant cell #s: flb_sched.ch_events[1] |-> #fresh(descriptor) when #s.ch_events[0] != -1;\n\
           quant cell #s: flb_sched.event.fd |-> #same(ch_events[0]) when #s.ch_events[0] != -1;\n\
           quant cell #c: flb_config.notification_channels[0] |-> #fresh(descriptor) when #c.notification_channels_initialized == 1;\n\
           quant cell #c: flb_config.notification_channels[1] |-> #fresh(descriptor) when #c.notification_channels_initialized == 1;\n\
           quant cell #c: flb_config.notification_event.fd |-> #same(notification_channels[0]) when #c.notification_channels_initialized == 1; };";

/// Fluent Bit's records, reduced to the members the bodies read, in their
/// declared order (`include/fluent-bit/flb_input_thread.h:61-69`,
/// `flb_scheduler.h:89-139`, `flb_config.h`), and the list macros of
/// `lib/monkey/include/monkey/mk_core/mk_list.h:206-214`. `FLB_TRUE` and
/// `FLB_FALSE` are `true` and `false`, 1 and 0 after preprocessing.
/// `struct flb_out_thread_instance` carries the same channel members and no
/// protocol.
const PRELUDE: &str = r#"
#define NULL ((void *)0)
#define FLB_TRUE 1
#define FLB_FALSE 0
#define container_of(ptr, type, member) ((type *)((char *)(ptr) - (char *) &((type *)0)->member))
#define mk_list_foreach_safe(curr, n, head) \
    for (curr = (head)->next, n = curr->next; curr != (head); curr = n, n = curr->next)
#define mk_list_entry( ptr, type, member ) container_of( ptr, type, member )
struct mk_list { struct mk_list *prev, *next; };
struct mk_event_loop;
struct mk_event { int fd; int type; int mask; int status; };
struct flb_tp;
struct flb_config;
struct flb_input_thread_instance {
    struct mk_event event;
    struct mk_event event_local;
    struct mk_event_loop *evl;
    int ch_parent_events[2];
    int ch_thread_events[2];
    int notification_channels_initialized;
    int notification_channels[2];
    struct mk_event notification_event;
    struct flb_tp *tp;
};
struct flb_out_thread_instance {
    struct mk_event event;
    struct mk_event_loop *evl;
    int ch_parent_events[2];
    int ch_thread_events[2];
    int notification_channels_initialized;
    int notification_channels[2];
    struct mk_event notification_event;
};
struct flb_sched_timer { struct mk_event event; struct mk_list _head; };
struct flb_sched {
    struct mk_event event;
    struct mk_list requests;
    struct mk_list requests_wait;
    struct mk_list timers;
    struct mk_list timers_drop;
    int frame_fd;
    struct mk_event_loop *evl;
    struct flb_config *config;
    int ch_events[2];
};
struct flb_config {
    int is_running;
    struct mk_event_loop *evl;
    int ch_self_events[2];
    int notification_channels_initialized;
    int notification_channels[2];
    struct mk_event notification_event;
    struct mk_event event_thread_init;
};
void *flb_calloc(__SIZE_TYPE__ count, __SIZE_TYPE__ size);
void flb_free(void *p);
int close(int fd);
void flb_tp_destroy(struct flb_tp *tp);
void mk_event_loop_destroy(struct mk_event_loop *loop);
int mk_event_timeout_destroy(struct mk_event_loop *loop, void *data);
int mk_event_channel_create(struct mk_event_loop *loop, int *r_fd, int *w_fd, void *data);
int mk_event_channel_destroy(struct mk_event_loop *loop, int r_fd, int w_fd, void *data);
"#;

fn environment(withheld: bool) -> ResourceContracts {
    let mut sources = BTreeMap::from([
        ("library.qv".to_owned(), LIBRARY.to_owned()),
        (
            "fluent_bit/teardown_destroy.qv".to_owned(),
            TEARDOWN.to_owned(),
        ),
        ("monkey/event_loop.qv".to_owned(), EVENT_LOOP.to_owned()),
    ]);
    if withheld {
        sources.insert("withheld.qv".to_owned(), WITHHELD.to_owned());
    }
    load_resource_contract_sources(&sources).expect("the contracts load")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent CELL-PROTOCOL-FB-DECLARATIONS test: {reason}");
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
fn outcome(entry: &str, body: &str, withheld: bool) -> (String, Vec<String>) {
    let text = format!("{PRELUDE}\n{body}\n");
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &text,
            ..Default::default()
        },
        entry,
        &environment(withheld),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{text}"));
    if let UnrestrictedLegacyResult::Safe(certificate) = &result {
        check_legacy_safety_certificate(certificate).expect("a Safe result rechecks");
    }
    let summary = summary_of(&result).unwrap_or_else(|| panic!("no summary: {result:?}"));
    let row = &summary.functions[entry];
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

fn expect(label: &str, entry: &str, body: &str, status: &str, diagnostics: &[&str]) {
    if !clang_available() {
        return;
    }
    let (actual_status, actual) = outcome(entry, body, false);
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

fn guard_undecided(root: &str, member: &str) -> String {
    format!(
        "unsupported:call to `mk_event_channel_destroy`: descriptor cell `(Interior(\"{root}\", [\"{member}\"], true) as mk_event).fd` is not decided (descriptor-cell-guard-undecided)"
    )
}

fn broken(root: &str, record: &str) -> String {
    format!(
        "unsupported:`{root}` (a `struct {record}`) does not keep its declared cell protocol here (descriptor-cell-protocol-broken)"
    )
}

/// The production teardown (`input_thread_instance_destroy`,
/// src/flb_input_thread.c:194-227 without the `> 0` closes): the flag test's
/// live edge introduces the caller's three names, the member-equality guard
/// is decided, both ends close and the flag store restores the protocol; the
/// record keeps it at `mk_event_loop_destroy(thi->evl)` and at its release.
const DESTROY: &str = r#"
static void input_thread_instance_destroy(struct flb_input_thread_instance *thi)
{
    if (thi->notification_channels_initialized == FLB_TRUE) {
        mk_event_channel_destroy(thi->evl,
                                 thi->notification_channels[0],
                                 thi->notification_channels[1],
                                 &thi->notification_event);

        thi->notification_channels_initialized = FLB_FALSE;
    }

    if (thi->evl) {
        mk_event_loop_destroy(thi->evl);
    }

    flb_tp_destroy(thi->tp);
    flb_free(thi);
}
"#;

/// The production module states exactly one protocol: the thread instance's
/// notification channel, hosted by `flb_config_set_program_name`, which
/// leaves the function map for the lifetime table.
#[test]
fn the_production_module_declares_the_thread_instance_protocol_only() {
    let contracts = environment(false);
    let protocols = contracts.cell_protocols();
    assert_eq!(
        protocols.keys().map(String::as_str).collect::<Vec<_>>(),
        ["flb_input_thread_instance"]
    );
    let (hosts, protocol) = &protocols["flb_input_thread_instance"];
    assert_eq!(
        hosts.iter().map(String::as_str).collect::<Vec<_>>(),
        ["flb_config_set_program_name"]
    );
    assert_eq!(protocol.guard, ["notification_channels_initialized"]);
    assert_eq!(protocol.cells.len(), 3);
    assert!(contracts.contract("flb_config_set_program_name").is_none());
    assert!(
        contracts
            .retaining()
            .contains_key("flb_config_set_program_name")
    );
}

/// Forwarded: the production teardown decides its guard; no descriptor-cell
/// leaf stays. What stays is not rule 38c's: the access gate's refusal of
/// the member `thi->evl` at `mk_event_loop_destroy`'s release position,
/// `flb_tp_destroy`, which no module declares, and the release of the
/// caller's `thi`.
#[test]
fn the_thread_instance_teardown_decides_its_guard() {
    expect(
        "input_thread_instance_destroy",
        "input_thread_instance_destroy",
        DESTROY,
        "Unknown",
        &[
            "unknown_alias:thi",
            "unsupported:call to uncontracted function `flb_tp_destroy`",
            "unsupported:indirect memory access is outside the resource projection",
        ],
    );
}

/// The complement test reads the other edge.
#[test]
fn the_complement_test_reads_the_dead_edge() {
    expect(
        "complement",
        "d",
        "int d(struct flb_input_thread_instance *thi) { \
         if (thi->notification_channels_initialized != 1) { return 0; } \
         mk_event_channel_destroy(thi->evl, thi->notification_channels[0], \
         thi->notification_channels[1], &thi->notification_event); \
         thi->notification_channels_initialized = 0; return 0; }",
        "Safe",
        &[],
    );
}

/// Broken: the ends close but the flag still says live at the exit.
#[test]
fn a_teardown_that_keeps_the_flag_breaks_the_protocol() {
    expect(
        "flag kept",
        "d",
        "int d(struct flb_input_thread_instance *thi) { \
         if (thi->notification_channels_initialized == 1) { \
         mk_event_channel_destroy(thi->evl, thi->notification_channels[0], \
         thi->notification_channels[1], &thi->notification_event); } return 0; }",
        "Unknown",
        &[&broken("thi", "flb_input_thread_instance")],
    );
}

/// Broken: the flag is set live over cells the test read dead.
#[test]
fn a_flag_set_over_dead_cells_breaks_the_protocol() {
    expect(
        "flag set dead",
        "d",
        "int d(struct flb_input_thread_instance *thi) { \
         if (thi->notification_channels_initialized != 1) { \
         thi->notification_channels_initialized = 1; } return 0; }",
        "Unknown",
        &[
            "unsupported:descriptor cell `thi->notification_channels_initialized` is not decided (descriptor-cell-protocol-broken)",
            &broken("thi", "flb_input_thread_instance"),
        ],
    );
}

/// Forwarded: a creator into a caller's record that reads the flag dead,
/// creates, tests the result as the Monkey declaration states it
/// (`#result == 0`) and sets the flag keeps the protocol; only the
/// registration side (rule 12t) stays.
#[test]
fn a_creation_under_the_dead_flag_forwards_the_cells() {
    expect(
        "creator",
        "c",
        "int c(struct flb_input_thread_instance *thi, struct mk_event_loop *loop) { \
         if (thi->notification_channels_initialized == 1) { return -1; } \
         int ret = mk_event_channel_create(loop, &thi->notification_channels[0], \
         &thi->notification_channels[1], &thi->notification_event); \
         if (ret != 0) { return -1; } \
         thi->notification_channels_initialized = 1; return 0; }",
        "Unknown",
        &[
            "unsupported:call to `mk_event_channel_create` retains `& thi - > notification_event` in `loop`, whose lifetime this body does not decide against it (retention-undecided)",
        ],
    );
}

/// Broken: `input_thread_instance_create` tests the creation's result as
/// `ret == -1` (src/flb_input_thread.c:291). The declaration decides the
/// cells only under `#result == 0`, so the other edge does not establish
/// them, and the flag store over them refuses, as does each exit. In
/// production the body also allocates `thi` itself (an unresolved identity at
/// the store), so rule 38c reads nothing there.
#[test]
fn the_production_result_test_does_not_establish_the_cells() {
    let escapes = |cell: &str| {
        format!(
            "unsupported:the descriptor in cell `{cell}` may outlive this function's frame (descriptor-cell-escapes)"
        )
    };
    expect(
        "creator, ret == -1",
        "c",
        "int c(struct flb_input_thread_instance *thi, struct mk_event_loop *loop) { \
         if (thi->notification_channels_initialized == 1) { return -1; } \
         int ret = mk_event_channel_create(loop, &thi->notification_channels[0], \
         &thi->notification_channels[1], &thi->notification_event); \
         if (ret == -1) { return -1; } \
         thi->notification_channels_initialized = 1; return 0; }",
        "Unknown",
        &[
            &broken("thi", "flb_input_thread_instance"),
            "unsupported:call to `mk_event_channel_create` retains `& thi - > notification_event` in `loop`, whose lifetime this body does not decide against it (retention-undecided)",
            "unsupported:descriptor cell `thi->notification_channels_initialized` is not decided (descriptor-cell-protocol-broken)",
            &escapes("(thi as int).notification_channels[0] [Name(\"thi\") as int]"),
            &escapes("(thi as int).notification_channels[1] [Name(\"thi\") as int]"),
            &escapes(
                "(thi.notification_event as mk_event).fd [Name(\"thi.notification_event\") as mk_event]",
            ),
        ],
    );
}

/// Unprotocolled: the same teardown over a record without a protocol keeps
/// rule 38's guard refusal.
#[test]
fn a_record_without_a_protocol_keeps_the_guard_refusal() {
    expect(
        "out thread instance",
        "d",
        "int d(struct flb_out_thread_instance *th) { \
         if (th->notification_channels_initialized == 1) { \
         mk_event_channel_destroy(th->evl, th->notification_channels[0], \
         th->notification_channels[1], &th->notification_event); \
         th->notification_channels_initialized = 0; } return 0; }",
        "Unknown",
        &[&guard_undecided("th", "notification_event")],
    );
}

/// `flb_sched_destroy`'s channel teardown (src/flb_scheduler.c:855-862).
const SCHED_TEARDOWN: &str = r#"
int sched_teardown(struct flb_sched *sched)
{
    if (sched->ch_events[0] != -1) {
        mk_event_channel_destroy(sched->evl,
                                 sched->ch_events[0],
                                 sched->ch_events[1],
                                 &sched->event);
        sched->ch_events[0] = -1;
        sched->ch_events[1] = -1;
    }
    return 0;
}
"#;

/// `flb_engine_shutdown`'s two channel teardowns (src/flb_engine.c:1710-1724
/// at 046accf6 and 34155955, :1674-1688 at 66910c10).
const SHUTDOWN_TEARDOWN: &str = r#"
int shutdown_teardown(struct flb_config *config)
{
    if (config->evl) {
        mk_event_channel_destroy(config->evl,
                                 config->ch_self_events[0],
                                 config->ch_self_events[1],
                                 &config->event_thread_init);
    }

    if (config->notification_channels_initialized == FLB_TRUE) {
        mk_event_channel_destroy(config->evl,
                                 config->notification_channels[0],
                                 config->notification_channels[1],
                                 &config->notification_event);

        config->notification_channels_initialized = FLB_FALSE;
    }

    return 0;
}
"#;

/// Unprotocolled: the scheduler and the engine configuration carry no
/// production protocol and keep rule 38's guard refusals.
#[test]
fn the_withheld_records_keep_rule_38s_refusal() {
    expect(
        "flb_sched",
        "sched_teardown",
        SCHED_TEARDOWN,
        "Unknown",
        &[&guard_undecided("sched", "event")],
    );
    expect(
        "flb_config",
        "shutdown_teardown",
        SHUTDOWN_TEARDOWN,
        "Unknown",
        &[
            &guard_undecided("config", "event_thread_init"),
            &guard_undecided("config", "notification_event"),
        ],
    );
}

/// The reading behind withholding `flb_sched` and `flb_config` (the report's
/// census section). Under their census protocols, a cell-bearing call whose
/// actual is rooted at the record (`sched->evl` in the timer loop, C2's
/// `config->evl` destroy) gives it a fact before the guard test, which is
/// then not read, and the record that keeps its protocol is reported as not
/// keeping it. It pins rule 38c's current host reading, which the follow-up
/// CELL-PROTOCOL-NONCELL-ACTUAL-REACHES-RECORD would change, so the gate
/// does not run it; `--ignored` reproduces the report.
#[test]
#[ignore = "records rule 38c's reading of the withheld protocols; see the report"]
fn withheld_protocols_reading() {
    if !clang_available() {
        return;
    }
    let after_loop = SCHED_TEARDOWN.replace(
        "    if (sched->ch_events[0] != -1) {",
        "    struct mk_list *tmp;\n    struct mk_list *head;\n    struct flb_sched_timer *timer;\n    \
         mk_list_foreach_safe(head, tmp, &sched->timers) {\n        \
         timer = mk_list_entry(head, struct flb_sched_timer, _head);\n        \
         mk_event_timeout_destroy(sched->evl, &timer->event);\n    }\n    \
         if (sched->ch_events[0] != -1) {",
    );
    let mut readings = Vec::new();
    for (label, entry, body) in [
        (
            "teardown alone",
            "sched_teardown",
            SCHED_TEARDOWN.to_owned(),
        ),
        ("after the timer loop", "sched_teardown", after_loop),
        (
            "engine shutdown",
            "shutdown_teardown",
            SHUTDOWN_TEARDOWN.to_owned(),
        ),
    ] {
        let reading = outcome(entry, &body, true);
        eprintln!("{label}: {}", reading.0);
        for d in &reading.1 {
            eprintln!("  {d}");
        }
        readings.push(reading);
    }
    assert_eq!(readings[0], ("Safe".to_owned(), Vec::new()));
    assert!(readings[1].1.contains(&guard_undecided("sched", "event")));
    assert!(readings[1].1.contains(&broken("sched", "flb_sched")));
    assert!(
        readings[2]
            .1
            .contains(&guard_undecided("config", "notification_event"))
    );
    assert!(readings[2].1.contains(&broken("config", "flb_config")));
}
