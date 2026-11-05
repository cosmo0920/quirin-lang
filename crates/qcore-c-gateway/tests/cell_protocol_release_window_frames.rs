//! Card CELL-PROTOCOL-RELEASE-WINDOW-FRAMES (gateway rule 38g, grammar G05k,
//! contract specification C04.6.4; USER DECISION 2026-11-04, verbatim, on the
//! options (a) callee frame declarations, (b) a member-actual narrowing, (c)
//! keep the refusal: "(a) Callee frames").
//!
//! A Fluent Bit destroyer closes an end without storing its sentinel and
//! keeps using the record before `flb_free`: in that window the closed end
//! still looks live, so a contracted call that may reach the record refuses
//! (`descriptor-cell-protocol-broken`) unless its callee's frame over the
//! record's type stands. One fixture per window call of the three destroyers
//! (`src/flb_output.c:615-638`, `src/flb_input.c:1311-1353`,
//! `src/flb_input_thread.c:194-227` at 046accf6 / 34155955), under the
//! production declarations
//! (`providers/quirin-provider-fluent-bit/contracts/fluent_bit/teardown_destroy.qv`,
//! whose frames are `mk_list_del`'s) and, for the callees production states
//! no frame for, a frame added to the same host's text here (a frame stands
//! only beside its record's protocol, and every declaration stating the
//! protocol must state the same frames). Callee bodies are Fluent Bit's
//! (and `lib/monkey/include/monkey/mk_core/mk_list.h:139-150`), reduced to
//! what they read; a body the unit does not define is declared only.
//!
//! Outcomes: a checked frame passes the record's fact through the call
//! unchanged; a declared one passes it and holds the call for review
//! (`awaiting-human-review`: `Pending`, never `Safe`); a refuted one refuses
//! (`descriptor-cell-frame-refuted`); none refuses as before. Every test
//! asserts the complete diagnostic vector with body lines.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

/// The production declarations.
const TEARDOWN: &str = include_str!(
    "../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/teardown_destroy.qv"
);
const EVENT_LOOP: &str =
    include_str!("../../../providers/qcore-provider-libevent/contracts/monkey/event_loop.qv");

/// The production frame lines (the anchor for the test-only frames).
const PRODUCTION_FRAMES: &str = "    quant cell flb_input_collector_destroy: flb_input_instance |-> #untouched;\n    quant cell mk_list_del: flb_input_instance |-> #untouched;\n    quant cell mk_list_del: flb_output_instance |-> #untouched;\n";

/// Frames production does not state, added to the host's own text for the
/// fixtures of the window's other callees.
const FIXTURE_FRAMES: &str = "\
    quant cell flb_output_free_properties: flb_output_instance |-> #untouched;\n\
    quant cell output_flags_clear: flb_output_instance |-> #untouched;\n\
    quant cell output_notify: flb_output_instance |-> #untouched;\n\
    quant cell flb_storage_input_destroy: flb_input_instance |-> #untouched;\n\
    quant cell flb_tp_destroy: flb_input_thread_instance |-> #untouched;\n";

/// The allocator, `close`, `mk_list_del` (Monkey's intrusive-list
/// declaration, `contracts/monkey/intrusive_list.qv`), and borrowed
/// declarations of the window's other callees, as their custody would read
/// (every one keeps the record the caller's).
const LIBRARY: &str = "\
module contract;\n\
extern fn flb_free(ptr: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(ptr, nullable);\n\
           ensures normal resource_return_none(); };\n\
extern fn close(fd: i32) -> i32\n\
contract { requires resource_contract_v1; ensures normal resource_released(fd, valid);\n\
           ensures normal resource_kind(fd, descriptor); ensures normal resource_return_none(); };\n\
extern fn mk_list_del(entry: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(entry);\n\
           ensures normal resource_unpublished(entry); ensures normal resource_return_none(); };\n\
extern fn flb_output_free_properties(ins: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(ins);\n\
           ensures normal resource_return_none(); };\n\
extern fn output_flags_clear(ins: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(ins);\n\
           ensures normal resource_return_none(); };\n\
extern fn output_notify(ins: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(ins);\n\
           ensures normal resource_return_none(); };\n\
extern fn flb_processor_destroy(proc: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(proc);\n\
           ensures normal resource_return_none(); };\n\
extern fn flb_storage_input_destroy(in: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(in);\n\
           ensures normal resource_return_none(); };\n\
extern fn flb_tp_destroy(tp: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(tp);\n\
           ensures normal resource_return_none(); };";

const PRELUDE: &str = r#"
#define NULL ((void *)0)
#define mk_event_closesocket(s) close(s)
struct mk_list { struct mk_list *prev, *next; };
struct mk_event { int fd; int type; int mask; int status; };
struct flb_tp;
struct flb_task_queue;
struct flb_input_collector;
struct flb_output_instance {
    struct mk_event event;
    struct mk_list _head;
    struct mk_list properties;
    struct flb_task_queue *singleplex_queue;
    void *processor;
    int ch_events[2];
    int flags;
};
struct flb_input_instance {
    struct mk_event event;
    struct mk_list _head;
    struct mk_list chunks;
    void *storage;
    int ch_events[2];
    int ingress_queue_channels[2];
};
struct flb_input_thread_instance {
    struct mk_event event;
    struct mk_event event_local;
    int ch_parent_events[2];
    int ch_thread_events[2];
    int notification_channels_initialized;
    int notification_channels[2];
    struct mk_event notification_event;
    struct flb_tp *tp;
};
void flb_free(void *p);
int close(int fd);
void flb_kv_release(struct mk_list *list);
void flb_processor_destroy(void *proc);
void flb_storage_input_destroy(struct flb_input_instance *in);
void flb_tp_destroy(struct flb_tp *tp);
void flb_task_queue_destroy(struct flb_task_queue *queue);
int flb_input_collector_destroy(struct flb_input_collector *coll);
int flb_pipe_w(int fd, void *buf, unsigned long count);
static inline void __mk_list_del(struct mk_list *prev, struct mk_list *next)
{
    prev->next = next;
    next->prev = prev;
}
static inline void mk_list_del(struct mk_list *entry)
{
    __mk_list_del(entry->prev, entry->next);
    entry->prev = NULL;
    entry->next = NULL;
}
static void flb_output_free_properties(struct flb_output_instance *ins)
{
    flb_kv_release(&ins->properties);
}
static void output_flags_clear(struct flb_output_instance *ins)
{
    ins->flags = 0;
}
static void output_notify(struct flb_output_instance *ins)
{
    int v = 1;
    flb_pipe_w(ins->ch_events[1], &v, sizeof(v));
}
"#;

fn environment(frames: &str) -> ResourceContracts {
    assert!(
        TEARDOWN.contains(PRODUCTION_FRAMES),
        "the production frames"
    );
    let teardown = TEARDOWN.replace(PRODUCTION_FRAMES, frames);
    let sources = BTreeMap::from([
        ("library.qv".to_owned(), LIBRARY.to_owned()),
        ("teardown_destroy.qv".to_owned(), teardown),
        ("event_loop.qv".to_owned(), EVENT_LOOP.to_owned()),
    ]);
    load_resource_contract_sources(&sources).expect("the contracts load")
}

/// Production's frames and the fixtures'.
fn framed() -> ResourceContracts {
    environment(&format!("{PRODUCTION_FRAMES}{FIXTURE_FRAMES}"))
}

/// No frame at all: the reading before this card.
fn unframed() -> ResourceContracts {
    environment("")
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!(
                "skipping Clang-dependent CELL-PROTOCOL-RELEASE-WINDOW-FRAMES test: {reason}"
            );
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

/// The line the analyzer reports for a body's first line (measured: it
/// counts the preprocessed text).
const BODY_LINE_ONE: usize = 66;

fn outcome(body: &str, contracts: &ResourceContracts) -> (String, Vec<String>) {
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
        contracts,
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
        .map(|d| {
            let kind = match &d.kind {
                LegacyDiagnosticKind::Unsupported { description } => description.clone(),
                LegacyDiagnosticKind::UnknownAlias { variable } => {
                    format!("unknown_alias:{variable}")
                }
                other => format!("{other:?}"),
            };
            format!("{}:{kind}", (d.line + 1).saturating_sub(BODY_LINE_ONE))
        })
        .collect::<Vec<_>>();
    diagnostics.sort();
    diagnostics.dedup();
    (format!("{:?}", row.status), diagnostics)
}

fn expect(
    label: &str,
    contracts: &ResourceContracts,
    body: &str,
    status: &str,
    diagnostics: &[&str],
) {
    if !clang_available() {
        return;
    }
    let (actual_status, actual) = outcome(body, contracts);
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

fn broken(line: usize, root: &str, record: &str) -> String {
    format!(
        "{line}:`{root}` (a `struct {record}`) does not keep its declared cell protocol here (descriptor-cell-protocol-broken)"
    )
}

fn held(line: usize, callee: &str, record: &str) -> String {
    format!(
        "{line}:call to `{callee}` reaching a `struct {record}` that does not keep its cell protocol rests on its declared frame, which awaits human review (awaiting-human-review)"
    )
}

fn refuted(line: usize, root: &str, record: &str, callee: &str) -> String {
    format!(
        "{line}:`{root}` (a `struct {record}`) does not keep its declared cell protocol here, and a body this unit analyses refutes the declared frame of `{callee}` over it (descriptor-cell-frame-refuted)"
    )
}

fn unlink(line: usize, root: &str) -> String {
    format!("{line}:unlink by `mk_list_del` of `{root}`, which this function has not published")
}

// -- The production inventory ---------------------------------------------------

/// Production states, on the protocol host, `mk_list_del`'s frame over the
/// input and output instances (checked wherever its mk_list.h body is in the
/// unit) and the declared frame of the other contracted window callee read
/// there, `flb_input_collector_destroy`.
#[test]
fn the_production_module_states_mk_list_del_frames() {
    let contracts = environment(PRODUCTION_FRAMES);
    let frames = contracts
        .cell_frames()
        .into_iter()
        .map(|(record, callees)| (record, callees.into_iter().collect::<Vec<_>>()))
        .collect::<Vec<_>>();
    assert_eq!(
        frames,
        [
            (
                "flb_input_instance".to_owned(),
                vec![
                    "flb_input_collector_destroy".to_owned(),
                    "mk_list_del".to_owned()
                ]
            ),
            (
                "flb_output_instance".to_owned(),
                vec!["mk_list_del".to_owned()]
            ),
        ]
    );
    assert!(!contracts.cell_frames_accepted());
}

// -- flb_output_instance_destroy (flb_output.c:615-638) -------------------------

/// `mk_list_del(&ins->_head)` (:631) in the window: its body (mk_list.h) is
/// in the unit and names only `struct mk_list`'s members, so the frame is
/// checked and the call no longer refuses the protocol; `mk_list_del`'s own
/// unlink refusal and the release's unresolved caller record stay.
#[test]
fn mk_list_del_in_the_output_window_is_body_checked() {
    let body = "int flb_output_instance_destroy(struct flb_output_instance *ins) {\n\
         if (ins->ch_events[0] > 0) {\n\
         mk_event_closesocket(ins->ch_events[0]);\n\
         }\n\
         if (ins->ch_events[1] > 0) {\n\
         mk_event_closesocket(ins->ch_events[1]);\n\
         }\n\
         mk_list_del(&ins->_head);\n\
         flb_free(ins);\n\
         return 0;\n\
         }";
    expect(
        "framed",
        &environment(PRODUCTION_FRAMES),
        body,
        "Unknown",
        &[&unlink(8, "ins"), "9:unknown_alias:ins"],
    );
    expect(
        "unframed (the reading before this card)",
        &unframed(),
        body,
        "Unknown",
        &[
            &broken(8, "ins", "flb_output_instance"),
            &unlink(8, "ins"),
            "9:unknown_alias:ins",
        ],
    );
}

/// `flb_output_free_properties(ins)` (:624): the unit defines its body, but
/// the body calls `flb_kv_release`, which the unit does not define, so the
/// frame is declared only and the call is held for review; the record's fact
/// passes the call, and the sentinel store after it keeps the protocol, so
/// the hold is the row's one reason: `Pending`.
#[test]
fn flb_output_free_properties_is_declared_only_and_held() {
    expect(
        "close, release properties, reset",
        &framed(),
        "int d(struct flb_output_instance *ins) {\n\
         if (ins->ch_events[0] > 0) {\n\
         mk_event_closesocket(ins->ch_events[0]);\n\
         flb_output_free_properties(ins);\n\
         ins->ch_events[0] = 0;\n\
         }\n\
         return 0;\n\
         }",
        "Pending",
        &[&held(
            4,
            "flb_output_free_properties",
            "flb_output_instance",
        )],
    );
}

/// A callee whose whole body the unit defines and which names no protocol
/// member: the frame is checked and the window call is invisible to the
/// protocol, so the same shape is `Safe` (and its certificate rechecks).
#[test]
fn a_checked_frame_passes_the_window_call() {
    expect(
        "close, clear flags, reset",
        &framed(),
        "int d(struct flb_output_instance *ins) {\n\
         if (ins->ch_events[0] > 0) {\n\
         mk_event_closesocket(ins->ch_events[0]);\n\
         output_flags_clear(ins);\n\
         ins->ch_events[0] = 0;\n\
         }\n\
         return 0;\n\
         }",
        "Safe",
        &[],
    );
}

/// A declared frame whose body reads an end (`ch_events[1]`, as the flush
/// notification does) is refuted by the body.
#[test]
fn a_frame_whose_body_reads_an_end_is_refuted() {
    expect(
        "close, notify, reset",
        &framed(),
        "int d(struct flb_output_instance *ins) {\n\
         if (ins->ch_events[0] > 0) {\n\
         mk_event_closesocket(ins->ch_events[0]);\n\
         output_notify(ins);\n\
         ins->ch_events[0] = 0;\n\
         }\n\
         return 0;\n\
         }",
        "Unknown",
        &[&refuted(4, "ins", "flb_output_instance", "output_notify")],
    );
}

/// `flb_processor_destroy(ins->processor)` (:635) states no frame: the call
/// refuses (fail-closed). Its actual is a member value, which option (b)
/// would read as reaching no end; rule 38g reads every call in the window as
/// reaching the record (`container_of`, a back pointer or a global reach it
/// too), so only a frame admits it.
#[test]
fn a_callee_without_a_frame_keeps_refusing() {
    expect(
        "close, destroy the processor",
        &framed(),
        "int d(struct flb_output_instance *ins) {\n\
         if (ins->ch_events[0] > 0) {\n\
         mk_event_closesocket(ins->ch_events[0]);\n\
         flb_processor_destroy(ins->processor);\n\
         ins->ch_events[0] = 0;\n\
         }\n\
         return 0;\n\
         }",
        "Unknown",
        &[&broken(4, "ins", "flb_output_instance")],
    );
}

/// Without any frame the declared-only and checked shapes refuse as before.
#[test]
fn without_frames_the_window_refuses_as_before() {
    for callee in ["flb_output_free_properties", "output_flags_clear"] {
        expect(
            callee,
            &unframed(),
            &format!(
                "int d(struct flb_output_instance *ins) {{\n\
                 if (ins->ch_events[0] > 0) {{\n\
                 mk_event_closesocket(ins->ch_events[0]);\n\
                 {callee}(ins);\n\
                 ins->ch_events[0] = 0;\n\
                 }}\n\
                 return 0;\n\
                 }}"
            ),
            "Unknown",
            &[&broken(4, "ins", "flb_output_instance")],
        );
    }
}

// -- flb_input_instance_destroy (flb_input.c:1311-1353) -------------------------

/// The input destroyer's window: `flb_storage_input_destroy(ins)` (:1338,
/// body in src/flb_storage.c, not in this unit: declared only, held) and
/// `mk_list_del(&ins->_head)` (:1340, checked). The release's unresolved
/// caller record stays.
#[test]
fn the_input_window_holds_storage_and_checks_the_unlink() {
    expect(
        "flb_input_instance_destroy",
        &framed(),
        "void flb_input_instance_destroy(struct flb_input_instance *ins) {\n\
         if (ins->ch_events[0] > 0) {\n\
         mk_event_closesocket(ins->ch_events[0]);\n\
         }\n\
         if (ins->ingress_queue_channels[0] > 0) {\n\
         mk_event_closesocket(ins->ingress_queue_channels[0]);\n\
         }\n\
         flb_storage_input_destroy(ins);\n\
         mk_list_del(&ins->_head);\n\
         flb_free(ins);\n\
         }",
        "Unknown",
        &[
            &held(8, "flb_storage_input_destroy", "flb_input_instance"),
            &unlink(9, "ins"),
            "10:unknown_alias:ins",
        ],
    );
}

/// The storage destroy alone between the close and the sentinel store: the
/// hold is the row's one reason (`Pending`); with every held declaration in
/// force the row carries no other reason, so it stays `Pending`.
#[test]
fn flb_storage_input_destroy_is_declared_only_and_held() {
    expect(
        "close, destroy storage, reset",
        &framed(),
        "void d(struct flb_input_instance *ins) {\n\
         if (ins->ingress_queue_channels[0] > 0) {\n\
         mk_event_closesocket(ins->ingress_queue_channels[0]);\n\
         flb_storage_input_destroy(ins);\n\
         ins->ingress_queue_channels[0] = -1;\n\
         }\n\
         }",
        "Pending",
        &[&held(4, "flb_storage_input_destroy", "flb_input_instance")],
    );
}

// -- input_thread_instance_destroy (flb_input_thread.c:194-227) -----------------

/// `flb_tp_destroy(thi->tp)` (:225; body in src/flb_thread_pool.c, not in
/// this unit): declared only, held; the actual `thi->tp` is read through the
/// record, so the call may reach it.
#[test]
fn flb_tp_destroy_is_declared_only_and_held() {
    expect(
        "close, destroy the pool, reset",
        &framed(),
        "void d(struct flb_input_thread_instance *thi) {\n\
         if (thi->ch_parent_events[0] > 0) {\n\
         mk_event_closesocket(thi->ch_parent_events[0]);\n\
         flb_tp_destroy(thi->tp);\n\
         thi->ch_parent_events[0] = 0;\n\
         }\n\
         }",
        "Pending",
        &[&held(4, "flb_tp_destroy", "flb_input_thread_instance")],
    );
}

/// A held row with another reason is `Unknown`: the second analysis (every
/// held declaration in force, the frame accepted) still carries the release's
/// unresolved caller record.
#[test]
fn a_hold_never_masks_another_reason() {
    if !clang_available() {
        return;
    }
    let (status, diagnostics) = outcome(
        "void d(struct flb_input_thread_instance *thi) {\n\
         if (thi->ch_parent_events[0] > 0) {\n\
         mk_event_closesocket(thi->ch_parent_events[0]);\n\
         }\n\
         flb_tp_destroy(thi->tp);\n\
         flb_free(thi);\n\
         }",
        &framed(),
    );
    assert_eq!(status, "Unknown", "{diagnostics:?}");
    assert!(
        diagnostics.contains(&held(5, "flb_tp_destroy", "flb_input_thread_instance")),
        "{diagnostics:?}"
    );
}

// -- The production frames declared for review ---------------------------------

/// `flb_task_queue_destroy(ins->singleplex_queue)` (flb_output.c:628): the
/// release of a member value refuses at the access gate first, so no call is
/// lowered and no frame would be read (production declares none): the
/// statement's refusal is the gate's, as at the baseline.
#[test]
fn flb_task_queue_destroy_refuses_at_the_gate_first() {
    expect(
        "close, destroy the singleplex queue, reset",
        &environment(PRODUCTION_FRAMES),
        "int d(struct flb_output_instance *ins) {\n\
         if (ins->ch_events[0] > 0) {\n\
         mk_event_closesocket(ins->ch_events[0]);\n\
         flb_task_queue_destroy(ins->singleplex_queue);\n\
         ins->ch_events[0] = 0;\n\
         }\n\
         return 0;\n\
         }",
        "Unknown",
        &["4:indirect memory access is outside the resource projection"],
    );
}

/// `flb_input_collector_destroy(collector)` (flb_input.c:1331): the call is
/// handed another pointer, which reaches the instance through
/// `collector->instance`; the production frame is declared (the body calls
/// Monkey out of the unit), so the call is held. The release of the caller's
/// collector is its own refusal (an unresolved identity), so the row is
/// `Unknown`.
#[test]
fn flb_input_collector_destroy_is_declared_only_and_held() {
    expect(
        "close, destroy a collector, reset",
        &environment(PRODUCTION_FRAMES),
        "void d(struct flb_input_instance *ins, struct flb_input_collector *collector) {\n\
         if (ins->ch_events[0] > 0) {\n\
         mk_event_closesocket(ins->ch_events[0]);\n\
         flb_input_collector_destroy(collector);\n\
         ins->ch_events[0] = 0;\n\
         }\n\
         }",
        "Unknown",
        &[
            &held(4, "flb_input_collector_destroy", "flb_input_instance"),
            "4:unknown_alias:collector",
        ],
    );
}
