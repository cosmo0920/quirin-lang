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

//! Card CELL-PROTOCOL-PER-END-GUARDS (gateway rule 38f; grammar G05i as
//! amended; USER DECISION 2026-11-03, verbatim: "Per-end sentinels").
//!
//! Each channel end carries its own sentinel, read under the production
//! declarations (`providers/quirin-provider-fluent-bit/contracts/fluent_bit/teardown_destroy.qv`,
//! host `flb_config_set_program_name`) and Monkey's event loop. One fixture
//! per Fluent Bit end: the bodies are Fluent Bit's destroyers and creators
//! (`src/flb_output.c`, `src/flb_input.c`, `src/flb_input_thread.c`,
//! `src/flb_config.c`, `src/flb_engine.c` at 046accf6), reduced to the
//! statements that read the channel members, with the record prelude reduced
//! to the members they read.
//!
//! USER DIRECTION 2026-11-03 (standing, verbatim): "The Fluent Bit's caller
//! tests sometimes check the incorrect assertions so we need to pay
//! attention for the assertions and actual returning values of Fluent Bit
//! functions." Every destroyer here tests its end `> 0`, which is WRONG
//! against the creators' result sets (descriptor 0 is a success value and
//! calloc's 0 the absent value): its live edge decides the close, and its
//! dead edge never reads the end as absent. An end left there at the
//! holder's release refuses (`descriptor-cell-zero-undecided`): a candidate
//! defect, never `Unsafe`, nothing filed upstream.
//!
//! Card CELL-RESULT-SET-ALIGNMENT (USER DECISIONS 2026-11-04: "Write [1,
//! MAX]", Q3 "(α) live_set suffix"): the production ends now declare
//! `live_set([1, MAX])` and their pinned `absent_set`, so every `> 0` dead
//! edge below decides "absent" under that DECLARED bound (POSIX's set is
//! [0, MAX]; descriptor 0 is observed natively): the zero readings at the
//! releases are gone, a close on the `<= 0` edge refuses for an unrecorded
//! cell only, and a creation refuses until the creator's written set is in
//! force (`descriptor-cell-live-set-unproved`). The POSIX contrast is
//! `tests/cell_result_set_alignment.rs`.
//!
//! A `Safe` here is `Safe` over the declared assumptions, the protocol
//! included (`P-CELL-PROTOCOL-DECLARED`), and promotes nothing. Every test
//! asserts the complete diagnostic vector with lines.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::result_range::{ResultRangeFact, ResultRangeStanding, written_set_key};

/// The production declarations.
const TEARDOWN: &str = include_str!(
    "../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/teardown_destroy.qv"
);
const EVENT_LOOP: &str =
    include_str!("../../../providers/qcore-provider-libevent/contracts/monkey/event_loop.qv");

/// The allocator and `close` as the selected modules state them (libc's
/// clauses, spelled here because the test loader does not resolve `contract
/// inherit`), and two borrowed helpers standing for the destroyers' later
/// uses of the record (`flb_output_free_properties(ins)`,
/// `flb_storage_input_destroy(ins)`, `flb_tp_destroy(thi->tp)`), declared so
/// the record stays resolved and rule 38f's reading of that window shows.
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
           ensures normal resource_kind(fd, descriptor); ensures normal resource_return_none(); };\n\
extern fn uses_record(p: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(p);\n\
           ensures normal resource_return_none(); };\n\
extern fn flb_pipe_w(fd: i32, buf: ptr<mut, u8>, count: usize) -> i32\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(fd);\n\
           ensures normal resource_borrowed(buf); ensures normal resource_borrowed(count);\n\
           ensures normal resource_return_none(); };";

/// `flb_config`'s per-end text, withheld from production (the host module
/// says why; follow-up CELL-PROTOCOL-NONCELL-ACTUAL-REACHES-RECORD).
const CONFIG: &str = "\
module contract;\n\
extern fn config_ends(c: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(c);\n\
           ensures normal resource_return_none();\n\
           quant cell #c: flb_config.ch_manager[0] |-> #fresh(descriptor) when #c.ch_manager[0] > 0;\n\
           quant cell #c: flb_config.ch_manager[1] |-> #fresh(descriptor) when #c.ch_manager[1] > 0;\n\
           quant cell #c: flb_config.ch_event.fd |-> #same(ch_manager[0]) when #c.ch_manager[0] > 0;\n\
           quant cell #c: flb_config.ch_notif[0] |-> #fresh(descriptor) when #c.ch_notif[0] > 0;\n\
           quant cell #c: flb_config.ch_notif[1] |-> #fresh(descriptor) when #c.ch_notif[1] > 0; };";

/// A destroyer that tests the holder's own `-1` sentinel (the shape an
/// upstream `>= 0` / `-1` fix would have), for contrast.
const FIXED: &str = "\
module contract;\n\
extern fn fixed_ends(c: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(c);\n\
           ensures normal resource_return_none();\n\
           quant cell #f: fixed_out.ch[0] |-> #fresh(descriptor) when #f.ch[0] != -1;\n\
           quant cell #f: fixed_out.ch[1] |-> #fresh(descriptor) when #f.ch[1] != -1; };";

const PRELUDE: &str = r#"
#define NULL ((void *)0)
#define FLB_TRUE 1
#define FLB_FALSE 0
#define mk_event_closesocket(s) close(s)
struct mk_event_loop;
struct mk_event { int fd; int type; int mask; int status; };
struct flb_tp;
struct flb_output_instance {
    struct mk_event event;
    void *processor;
    int ch_events[2];
    int flags;
};
struct flb_input_instance {
    struct mk_event event;
    void *processor;
    int ch_events[2];
    int ingress_queue_channels[2];
};
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
struct flb_config {
    struct mk_event ch_event;
    struct mk_event_loop *evl;
    int ch_manager[2];
    int ch_notif[2];
};
struct fixed_out { int ch[2]; };
void *flb_calloc(__SIZE_TYPE__ count, __SIZE_TYPE__ size);
void flb_free(void *p);
int close(int fd);
void uses_record(void *p);
void opaque(void *p);
void flb_tp_destroy(struct flb_tp *tp);
int flb_pipe_w(int fd, void *buf, __SIZE_TYPE__ count);
void mk_event_loop_destroy(struct mk_event_loop *loop);
int mk_event_channel_create(struct mk_event_loop *loop, int *r_fd, int *w_fd, void *data);
int mk_event_channel_destroy(struct mk_event_loop *loop, int r_fd, int w_fd, void *data);
"#;

fn environment() -> ResourceContracts {
    let sources = BTreeMap::from([
        ("library.qv".to_owned(), LIBRARY.to_owned()),
        ("teardown_destroy.qv".to_owned(), TEARDOWN.to_owned()),
        ("event_loop.qv".to_owned(), EVENT_LOOP.to_owned()),
        ("config.qv".to_owned(), CONFIG.to_owned()),
        ("fixed.qv".to_owned(), FIXED.to_owned()),
    ]);
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
            eprintln!("skipping Clang-dependent CELL-PROTOCOL-PER-END-GUARDS test: {reason}");
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

/// The line the analyzer reports for a body's first line (its coordinate
/// counts the preprocessed text, so it is measured, not derived; every
/// fixture's vector pins it, the uncontracted `opaque(ins)` on body line 2
/// first of all).
const BODY_LINE_ONE: usize = 44;

/// The entry row's status and its sorted diagnostics (`line:kind:text`,
/// the line counted from the body's first line, 1-based), under a given
/// environment (card CELL-RESULT-SET-WRITTEN-SET-DERIVATION: the route's
/// written sets).
fn outcome_in(body: &str, contracts: &ResourceContracts) -> (String, Vec<String>) {
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
            format!("{}:{kind}", d.line + 1 - BODY_LINE_ONE)
        })
        .collect::<Vec<_>>();
    diagnostics.sort();
    diagnostics.dedup();
    (format!("{:?}", row.status), diagnostics)
}

fn expect(label: &str, body: &str, status: &str, diagnostics: &[&str]) {
    expect_in(label, body, &environment(), status, diagnostics);
}

fn expect_in(
    label: &str,
    body: &str,
    contracts: &ResourceContracts,
    status: &str,
    diagnostics: &[&str],
) {
    if !clang_available() {
        return;
    }
    let (actual_status, actual) = outcome_in(body, contracts);
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

const GATE: &str = "indirect memory access is outside the resource projection";

fn broken(line: usize, root: &str, record: &str) -> String {
    format!(
        "{line}:`{root}` (a `struct {record}`) does not keep its declared cell protocol here (descriptor-cell-protocol-broken)"
    )
}

fn zero(line: usize, root: &str) -> String {
    format!(
        "{line}:`{root}` is released while a cell of it may hold descriptor 0, which a `> 0` test read as absent (descriptor-cell-zero-undecided)"
    )
}

fn undecided(line: usize, place: &str) -> String {
    format!("{line}:descriptor cell `{place}` is not decided (descriptor-cell-undecided)")
}

// -- The production inventory ---------------------------------------------------

/// The production module states one protocol per guard: the thread
/// instance's flag group beside its four per-end sentinels, and the input and
/// output instances' per-end sentinels; every one `> 0` but the flag.
#[test]
fn the_production_module_states_per_end_sentinels() {
    let contracts = environment();
    let protocols = contracts.cell_protocols();
    let mut stated = protocols
        .iter()
        .filter(|(_, (hosts, _))| hosts.contains("flb_config_set_program_name"))
        .flat_map(|(record, (_, protocols))| {
            protocols.iter().map(move |protocol| {
                (
                    record.clone(),
                    protocol.guard.join("."),
                    protocol.op.spelling(),
                    protocol.cells.len(),
                    protocol.per_end(),
                )
            })
        })
        .collect::<Vec<_>>();
    stated.sort();
    let row = |record: &str, guard: &str, op: &'static str, cells: usize, per_end: bool| {
        (record.to_owned(), guard.to_owned(), op, cells, per_end)
    };
    assert_eq!(
        stated,
        [
            row("flb_input_instance", "ch_events[0]", "> 0", 2, true),
            row("flb_input_instance", "ch_events[1]", "> 0", 1, true),
            row(
                "flb_input_instance",
                "ingress_queue_channels[0]",
                "> 0",
                1,
                true
            ),
            row(
                "flb_input_instance",
                "ingress_queue_channels[1]",
                "> 0",
                1,
                true
            ),
            row(
                "flb_input_thread_instance",
                "ch_parent_events[0]",
                "> 0",
                2,
                true
            ),
            row(
                "flb_input_thread_instance",
                "ch_parent_events[1]",
                "> 0",
                1,
                true
            ),
            row(
                "flb_input_thread_instance",
                "ch_thread_events[0]",
                "> 0",
                2,
                true
            ),
            row(
                "flb_input_thread_instance",
                "ch_thread_events[1]",
                "> 0",
                1,
                true
            ),
            row(
                "flb_input_thread_instance",
                "notification_channels_initialized",
                "== 1",
                3,
                false
            ),
            row("flb_output_instance", "ch_events[0]", "> 0", 2, true),
            row("flb_output_instance", "ch_events[1]", "> 0", 1, true),
        ]
    );
}

// -- flb_output_instance_destroy (flb_output.c:615-638) -------------------------

/// Both ends under their own `> 0`, the record used, then released. Each live
/// edge decides its close; each dead edge leaves its end undecided (it may
/// be descriptor 0); the use of the record between the closes and the
/// release refuses (the closed ends still look live); the release ends the
/// protocol and names the zero reading.
#[test]
fn the_output_instance_destroyer_names_descriptor_zero_at_the_release() {
    expect(
        "flb_output_instance_destroy",
        "int flb_output_instance_destroy(struct flb_output_instance *ins) {\n\
         if (ins->ch_events[0] > 0) {\n\
         mk_event_closesocket(ins->ch_events[0]);\n\
         }\n\
         if (ins->ch_events[1] > 0) {\n\
         mk_event_closesocket(ins->ch_events[1]);\n\
         }\n\
         uses_record(ins);\n\
         flb_free(ins);\n\
         return 0;\n\
         }",
        "Unknown",
        &[
            &broken(8, "ins", "flb_output_instance"),
            "9:unknown_alias:ins",
        ],
    );
}

/// The same ends with no use of the record before its release: only the
/// zero reading refuses.
#[test]
fn the_output_ends_released_at_once_refuse_only_for_descriptor_zero() {
    expect(
        "closes then flb_free",
        "int d(struct flb_output_instance *ins) {\n\
         if (ins->ch_events[0] > 0) {\n\
         mk_event_closesocket(ins->ch_events[0]);\n\
         }\n\
         if (ins->ch_events[1] > 0) {\n\
         mk_event_closesocket(ins->ch_events[1]);\n\
         }\n\
         flb_free(ins);\n\
         return 0;\n\
         }",
        "Unknown",
        &["8:unknown_alias:ins"],
    );
}

/// The production state: an earlier uncontracted call leaves `ins` an
/// unresolved identity for the member reads, yet the declared protocol is
/// still read at the tests (rule 38c's premise covers stores by
/// uncontracted callees): each live edge's close is decided (no gate
/// refusal), and the record, left broken and possibly holding descriptor 0,
/// refuses at the exit with its ends escaping.
#[test]
fn the_output_ends_after_an_opaque_call_are_still_read() {
    expect(
        "opaque call first",
        "int d(struct flb_output_instance *ins) {\n\
         opaque(ins);\n\
         if (ins->ch_events[0] > 0) {\n\
         mk_event_closesocket(ins->ch_events[0]);\n\
         }\n\
         if (ins->ch_events[1] > 0) {\n\
         mk_event_closesocket(ins->ch_events[1]);\n\
         }\n\
         return 0;\n\
         }",
        "Unknown",
        &[
            "2:call to uncontracted function `opaque`",
            "3:unknown_alias:ins",
            "6:unknown_alias:ins",
            &broken(9, "ins", "flb_output_instance"),
        ],
    );
}

/// Each live edge's close decided and its sentinel stored dead, the record
/// kept: forwarded under its protocol, the dead edge's undecided end with it
/// (the caller still holds the record).
#[test]
fn closed_ends_with_their_sentinels_reset_keep_the_protocol() {
    expect(
        "close and reset each end",
        "int d(struct flb_output_instance *ins) {\n\
         if (ins->ch_events[0] > 0) {\n\
         mk_event_closesocket(ins->ch_events[0]);\n\
         ins->ch_events[0] = 0;\n\
         }\n\
         if (ins->ch_events[1] > 0) {\n\
         mk_event_closesocket(ins->ch_events[1]);\n\
         ins->ch_events[1] = 0;\n\
         }\n\
         return 0;\n\
         }",
        "Safe",
        &[],
    );
}

/// A close on the dead edge (`<= 0`) refuses: the end may be descriptor 0,
/// never read as absent.
#[test]
fn a_close_on_a_positive_dead_edge_refuses() {
    expect(
        "close where the end tests <= 0",
        "int d(struct flb_output_instance *ins) {\n\
         if (ins->ch_events[1] <= 0) {\n\
         mk_event_closesocket(ins->ch_events[1]);\n\
         }\n\
         return 0;\n\
         }",
        "Unknown",
        &[&undecided(3, "ins->ch_events[1]")],
    );
}

// -- flb_input_instance_destroy (flb_input.c:1311-1353) -------------------------

/// Four ends, two channel pairs, each its own sentinel (the decision's
/// "a record with several channel pairs"); `ingress_queue_channels`' absent
/// value is -1, which `> 0`'s dead edge holds too, beside descriptor 0.
#[test]
fn the_input_instance_destroyer_reads_four_ends() {
    expect(
        "flb_input_instance_destroy",
        "int flb_input_instance_destroy(struct flb_input_instance *ins) {\n\
         if (ins->ch_events[0] > 0) {\n\
         mk_event_closesocket(ins->ch_events[0]);\n\
         }\n\
         if (ins->ch_events[1] > 0) {\n\
         mk_event_closesocket(ins->ch_events[1]);\n\
         }\n\
         if (ins->ingress_queue_channels[0] > 0) {\n\
         mk_event_closesocket(ins->ingress_queue_channels[0]);\n\
         }\n\
         if (ins->ingress_queue_channels[1] > 0) {\n\
         mk_event_closesocket(ins->ingress_queue_channels[1]);\n\
         }\n\
         uses_record(ins);\n\
         flb_free(ins);\n\
         return 0;\n\
         }",
        "Unknown",
        &[
            &broken(14, "ins", "flb_input_instance"),
            "15:unknown_alias:ins",
        ],
    );
}

/// `ingress_queue_channels`' `-1` stores (flb_input.c:549-550,
/// flb_input_ingest.c:493-494) are its dead value: they keep the protocol.
#[test]
fn the_ingress_sentinel_stores_keep_the_protocol() {
    expect(
        "ingress -1 stores",
        "int d(struct flb_input_instance *ins) {\n\
         ins->ingress_queue_channels[0] = -1;\n\
         ins->ingress_queue_channels[1] = -1;\n\
         return 0;\n\
         }",
        "Safe",
        &[],
    );
}

// -- input_thread_instance_destroy (flb_input_thread.c:194-227) -----------------

/// The flag group and four per-end sentinels in one record. The flag's
/// teardown decides; each `> 0` end reads as the output instance's;
/// `flb_tp_destroy(thi->tp)` reaches the record after the closes.
#[test]
fn the_thread_instance_destroyer_reads_its_flag_and_four_ends() {
    expect(
        "input_thread_instance_destroy",
        "void input_thread_instance_destroy(struct flb_input_thread_instance *thi) {\n\
         if (thi->notification_channels_initialized == FLB_TRUE) {\n\
         mk_event_channel_destroy(thi->evl, thi->notification_channels[0], thi->notification_channels[1], &thi->notification_event);\n\
         thi->notification_channels_initialized = FLB_FALSE;\n\
         }\n\
         if (thi->ch_parent_events[0] > 0) {\n\
         mk_event_closesocket(thi->ch_parent_events[0]);\n\
         }\n\
         if (thi->ch_parent_events[1] > 0) {\n\
         mk_event_closesocket(thi->ch_parent_events[1]);\n\
         }\n\
         if (thi->ch_thread_events[0] > 0) {\n\
         mk_event_closesocket(thi->ch_thread_events[0]);\n\
         }\n\
         if (thi->ch_thread_events[1] > 0) {\n\
         mk_event_closesocket(thi->ch_thread_events[1]);\n\
         }\n\
         uses_record(thi);\n\
         flb_free(thi);\n\
         }",
        "Unknown",
        &[
            &broken(18, "thi", "flb_input_thread_instance"),
            "19:unknown_alias:thi",
        ],
    );
}

/// The production state: `mk_event_loop_destroy(thi->evl)` is refused at the
/// access gate (:206); the `> 0` test after it is still read, its close is
/// decided, and the release names descriptor 0. `flb_tp_destroy` is
/// uncontracted, as in production, so it reaches no fact.
#[test]
fn the_thread_ends_after_the_gate_refusal_are_read() {
    expect(
        "after mk_event_loop_destroy",
        "void d(struct flb_input_thread_instance *thi) {\n\
         if (thi->evl) {\n\
         mk_event_loop_destroy(thi->evl);\n\
         }\n\
         if (thi->ch_parent_events[0] > 0) {\n\
         mk_event_closesocket(thi->ch_parent_events[0]);\n\
         }\n\
         flb_tp_destroy(thi->tp);\n\
         flb_free(thi);\n\
         }",
        "Unknown",
        &[
            // Card MEMBER-EFFECT-SUMMARY (rule 38i) re-pins this deliberately: the member release names its premise.
            "3:release of `thi->evl`, a member of the formal `thi` \
             (member-release-through-formal)",
            "5:unknown_alias:thi",
            "8:call to uncontracted function `flb_tp_destroy`",
            "9:unknown_alias:thi",
        ],
    );
}

// -- flb_config_exit (flb_config.c:606-631), withheld text ----------------------

/// `flb_config_exit`'s ends: `ch_event.fd` (a multi-segment close, the
/// gate's), `ch_manager[0] > 0` (decided on its live edge), `ch_manager[1]`
/// under `!=` of the two ends (no test of its own sentinel: the gate's),
/// `ch_notif[0] > 0`, and `ch_notif[1]` under `> 0 && !=` (two deciding
/// nodes: not read, the gate's).
#[test]
fn the_config_exit_ends_under_the_withheld_text() {
    expect(
        "flb_config_exit",
        "void flb_config_exit(struct flb_config *config) {\n\
         if (config->ch_event.fd) {\n\
         mk_event_closesocket(config->ch_event.fd);\n\
         }\n\
         if (config->ch_manager[0] > 0) {\n\
         mk_event_closesocket(config->ch_manager[0]);\n\
         if (config->ch_manager[0] != config->ch_manager[1]) {\n\
         mk_event_closesocket(config->ch_manager[1]);\n\
         }\n\
         }\n\
         if (config->ch_notif[0] > 0) {\n\
         mk_event_closesocket(config->ch_notif[0]);\n\
         if (config->ch_notif[1] > 0 &&\n\
         config->ch_notif[0] != config->ch_notif[1]) {\n\
         mk_event_closesocket(config->ch_notif[1]);\n\
         }\n\
         }\n\
         flb_free(config);\n\
         }",
        "Unknown",
        &[
            &format!("3:{GATE}"),
            "5:unknown_alias:config",
            "7:unknown_alias:config",
            &format!("8:{GATE}"),
            "11:unknown_alias:config",
            "13:unknown_alias:config",
            &format!("15:{GATE}"),
            &zero(18, "config"),
            "18:unknown_alias:config",
        ],
    );
}

/// `flb_engine_started`'s `ch_notif[1] <= 0` (flb_engine.c:1034): the true
/// edge holds descriptor 0, read as no channel. Untouched on both edges, so
/// the record is forwarded and nothing refuses here; the candidate is
/// recorded by the census, not by a refusal.
#[test]
fn the_engine_started_test_reads_nothing_it_touches() {
    expect(
        "flb_engine_started",
        "int flb_engine_started(struct flb_config *config) {\n\
         int val = 1;\n\
         if (config->ch_notif[1] <= 0) {\n\
         return -1;\n\
         }\n\
         return flb_pipe_w(config->ch_notif[1], &val, sizeof(val));\n\
         }",
        "Safe",
        &[],
    );
}

// -- Creators -------------------------------------------------------------------

/// `input_instance_channel_events_init` (flb_input.c:1372-1399): a creation
/// never establishes `> 0` (the new end may be descriptor 0), so the
/// creation into an end the path holds no dead reading of refuses.
#[test]
fn a_creator_never_establishes_a_positive_sentinel() {
    expect(
        "input_instance_channel_events_init",
        "int input_instance_channel_events_init(struct flb_input_instance *ins, struct mk_event_loop *evl) {\n\
         int ret;\n\
         ret = mk_event_channel_create(evl, &ins->ch_events[0], &ins->ch_events[1], ins);\n\
         if (ret != 0) {\n\
         return -1;\n\
         }\n\
         return 0;\n\
         }",
        "Unknown",
        &[
            "3:call to `mk_event_channel_create` retains `ins` in `evl`, whose lifetime this body does not decide against it (retention-undecided)",
            "3:call to `mk_event_channel_create`: descriptor cell `(Formal(\"ins\") as int).ch_events[0]` is not decided (descriptor-cell-live-set-unproved)",
            "3:call to `mk_event_channel_create`: descriptor cell `(Formal(\"ins\") as int).ch_events[1]` is not decided (descriptor-cell-live-set-unproved)",
            &broken(5, "ins", "flb_input_instance"),
        ],
    );
}

/// The creator's body, as `input_instance_channel_events_init` spells it.
const CREATOR: &str = "int input_instance_channel_events_init(struct flb_input_instance *ins, struct mk_event_loop *evl) {\n\
         int ret;\n\
         ret = mk_event_channel_create(evl, &ins->ch_events[0], &ins->ch_events[1], ins);\n\
         if (ret != 0) {\n\
         return -1;\n\
         }\n\
         return 0;\n\
         }";

/// The environment with `mk_event_channel_create`'s written sets of `r_fd`
/// and `w_fd` (positions 1 and 2) in the route's table, keyed as the route
/// keys them (`FUNCTION#P[K]==C`).
fn with_written(set: (i64, i64), class: i64, standing: ResultRangeStanding) -> ResourceContracts {
    let mut contracts = environment();
    let fact = ResultRangeFact {
        intervals: vec![set],
        universe: (i64::from(i32::MIN), i64::from(i32::MAX)),
        standing,
        detail: "fixture".to_owned(),
    };
    contracts.set_result_ranges(BTreeMap::from([
        (
            written_set_key("mk_event_channel_create", 1, 0, class),
            fact.clone(),
        ),
        (
            written_set_key("mk_event_channel_create", 2, 0, class),
            fact,
        ),
    ]));
    contracts
}

/// Card CELL-RESULT-SET-WRITTEN-SET-DERIVATION (rule 38e as amended): with
/// the pin-derived written sets in force inside the declared
/// `live_set([1, MAX])`, the creation passes the live-set check and reads as
/// rule 38f reads any creation: the path holds no dead reading of the ends,
/// so each refuses `descriptor-cell-undecided`. The admission rests on
/// pipe's DECLARED [1, MAX] (POSIX's set is [0, MAX]).
#[test]
fn a_creator_with_its_written_sets_in_force_passes_the_live_set_check() {
    let in_force = with_written((1, i64::from(i32::MAX)), 0, ResultRangeStanding::InForce);
    expect_in(
        "input_instance_channel_events_init, written sets in force",
        CREATOR,
        &in_force,
        "Unknown",
        &[
            "3:call to `mk_event_channel_create` retains `ins` in `evl`, whose lifetime this body does not decide against it (retention-undecided)",
            "3:call to `mk_event_channel_create`: descriptor cell `(Formal(\"ins\") as int).ch_events[0]` is not decided (descriptor-cell-undecided)",
            "3:call to `mk_event_channel_create`: descriptor cell `(Formal(\"ins\") as int).ch_events[1]` is not decided (descriptor-cell-undecided)",
            &broken(5, "ins", "flb_input_instance"),
        ],
    );
}

/// A written set out of force (held, unchecked), of another class than the
/// `#fresh` state's (`when #result == 0`), or outside the live set (POSIX's
/// [0, MAX]) proves nothing: the creation still refuses.
#[test]
fn a_written_set_out_of_force_of_another_class_or_wider_proves_nothing() {
    let unproved = [
        "3:call to `mk_event_channel_create` retains `ins` in `evl`, whose lifetime this body does not decide against it (retention-undecided)",
        "3:call to `mk_event_channel_create`: descriptor cell `(Formal(\"ins\") as int).ch_events[0]` is not decided (descriptor-cell-live-set-unproved)",
        "3:call to `mk_event_channel_create`: descriptor cell `(Formal(\"ins\") as int).ch_events[1]` is not decided (descriptor-cell-live-set-unproved)",
    ];
    let broken = broken(5, "ins", "flb_input_instance");
    let vector = unproved
        .iter()
        .copied()
        .chain([broken.as_str()])
        .collect::<Vec<_>>();
    for (label, contracts) in [
        (
            "held",
            with_written((1, i64::from(i32::MAX)), 0, ResultRangeStanding::Held),
        ),
        (
            "unchecked",
            with_written((1, i64::from(i32::MAX)), 0, ResultRangeStanding::Unchecked),
        ),
        (
            "another class",
            with_written((1, i64::from(i32::MAX)), -1, ResultRangeStanding::InForce),
        ),
        (
            "POSIX's set",
            with_written((0, i64::from(i32::MAX)), 0, ResultRangeStanding::InForce),
        ),
    ] {
        expect_in(label, CREATOR, &contracts, "Unknown", &vector);
    }
}

// -- The holder's release with a decided sentinel --------------------------------

/// A destroyer testing the holder's own `-1` sentinel: each dead edge decides
/// the end absent, each live edge's close is decided, and the release ends
/// the protocol with no cell live. What stays is route G's refusal of a
/// caller's record released here.
#[test]
fn a_minus_one_sentinel_destroyer_ends_its_protocol_at_the_release() {
    expect(
        "fixed destroyer",
        "int d(struct fixed_out *o) {\n\
         if (o->ch[0] != -1) {\n\
         close(o->ch[0]);\n\
         }\n\
         if (o->ch[1] != -1) {\n\
         close(o->ch[1]);\n\
         }\n\
         flb_free(o);\n\
         return 0;\n\
         }",
        "Unknown",
        &["8:unknown_alias:o"],
    );
}

/// The same with a use of the record between the closes and the release:
/// the closed ends still look live there, so the use refuses.
#[test]
fn a_use_between_the_closes_and_the_release_refuses() {
    expect(
        "fixed destroyer with a use",
        "int d(struct fixed_out *o) {\n\
         if (o->ch[0] != -1) {\n\
         close(o->ch[0]);\n\
         }\n\
         uses_record(o);\n\
         flb_free(o);\n\
         return 0;\n\
         }",
        "Unknown",
        &[&broken(5, "o", "fixed_out"), "6:unknown_alias:o"],
    );
}
