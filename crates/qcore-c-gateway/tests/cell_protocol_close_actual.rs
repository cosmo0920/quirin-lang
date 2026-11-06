//! Card CELL-PROTOCOL-CLOSE-ACTUAL (gateway rule 38d; registered by
//! MK-EVENT-CELL-CROSS-FUNCTION; USER DECISION 2026-11-02, verbatim: "B as
//! recommended", Q3: a close of a declared cell member is that cell's
//! `#stale`).
//!
//! The access gate admits exactly one more actual: an integer member read of
//! a declared protocol record's one-segment cell, at a descriptor-kind release
//! position of a function-map callee, with rule 38c's close as its custody
//! effect. Every other shape keeps the gate's refusal byte for byte, and an
//! admitted close rule 38c does not decide refuses (never a silent release).
//! Card CELL-PROTOCOL-PER-END-GUARDS (rule 38f) changed the `> 0` vectors
//! here: that guard's dead edge leaves its cells undecided (descriptor 0),
//! so a touched record joined with it is broken and its cells escape; a
//! holder's release with a live cell refuses. Every change adds a refusal.
//! One fixture per admitted and refused shape; each asserts the complete
//! diagnostic vector, and every `Safe` certificate is rechecked. The hosts are
//! synthetic: a `Safe` here is `Safe` over the declared protocol (rule 13i,
//! `P-CELL-PROTOCOL-DECLARED`) and the synthetic `close`, and promotes nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

/// The protocol hosts: `struct pos_chan` carries a `> 0` sentinel protocol
/// (the census's C5, C6, C8, C9 shape), `struct chan` a `!= -1` one (C1),
/// `struct flag_chan` a `== 1` flag (C4, C7).
const PROTOCOLS: &str = "\
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
           quant cell #p: pos_chan.event.fd |-> #same(e[0]) when #p.e[0] > 0;\n\
           quant cell #q: one_chan.fd |-> #fresh(descriptor) when #q.fd > 0;\n\
           quant cell #b: fb_chan.ch_events[0] |-> #fresh(descriptor) when #b.ch_events[0] > 0;\n\
           quant cell #b: fb_chan.ch_events[1] |-> #fresh(descriptor) when #b.ch_events[0] > 0;\n\
           quant cell #b: fb_chan.event.fd |-> #same(ch_events[0]) when #b.ch_events[0] > 0; };";

/// `close` as libc's provider declares it (a function-map release of a
/// descriptor, `providers/quirin-provider-libc/contracts/libc/unistd.qv`).
const CLOSE: &str = "\
module contract;\n\
extern fn close(fd: i32) -> i32\n\
contract { requires resource_contract_v1; ensures normal resource_released(fd, valid);\n\
           ensures normal resource_kind(fd, descriptor); ensures normal resource_return_none(); };";

/// `close` declared at a custody position that is not a release.
const CLOSE_CONSUMED: &str = "\
module contract;\n\
extern fn close(fd: i32) -> i32\n\
contract { requires resource_contract_v1; ensures normal resource_consumed(fd, valid);\n\
           ensures normal resource_kind(fd, descriptor); ensures normal resource_return_none(); };";

/// `close` declared as a heap release (no descriptor kind).
const CLOSE_HEAP: &str = "\
module contract;\n\
extern fn close(fd: ptr<mut, u8>) -> i32\n\
contract { requires resource_contract_v1; ensures normal resource_released(fd, nonnull);\n\
           ensures normal resource_return_none(); };";

const PRELUDE: &str = "\
struct mk_event_loop;
struct mk_event { int fd; int mask; };
struct chan { int ch[2]; struct mk_event_loop *evl; struct mk_event event; };
struct flag_chan { int n[2]; int on; struct mk_event_loop *evl; struct mk_event event; };
struct pos_chan { int e[2]; int aux; struct mk_event_loop *evl; struct mk_event event; };
struct one_chan { int fd; int aux; };
struct other { int e[2]; struct mk_event_loop *evl; struct mk_event event; }; typedef int flb_pipefd_t; struct fb_chan { flb_pipefd_t ch_events[2]; struct mk_event event; };
void *ev_alloc(__SIZE_TYPE__ n); void ev_free(void *p); void ev_note(void *p);
int close(int fd);
int closesocket(int fd);
void chan_protocols(void *c);
#define mk_event_closesocket(s) close(s)
";

#[derive(Clone, Copy)]
enum Close {
    /// libc's `close`.
    Release,
    /// `close` at a `resource_consumed` position.
    Consumed,
    /// `close` as a heap release.
    Heap,
    /// No `close` contract.
    Uncontracted,
}

fn environment(close: Close) -> ResourceContracts {
    let mut sources = BTreeMap::from([("protocols.qv".to_owned(), PROTOCOLS.to_owned())]);
    let close = match close {
        Close::Release => Some(CLOSE),
        Close::Consumed => Some(CLOSE_CONSUMED),
        Close::Heap => Some(CLOSE_HEAP),
        Close::Uncontracted => None,
    };
    if let Some(close) = close {
        sources.insert("close.qv".to_owned(), close.to_owned());
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
            eprintln!("skipping Clang-dependent CELL-PROTOCOL-CLOSE-ACTUAL test: {reason}");
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

/// The entry row's status and its sorted diagnostics (`line:kind:text`).
fn outcome(close: Close, body: &str) -> (String, Vec<String>) {
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
        &environment(close),
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
                LegacyDiagnosticKind::Unsupported { description } => {
                    format!("unsupported:{description}")
                }
                LegacyDiagnosticKind::UnknownAlias { variable } => {
                    format!("unknown_alias:{variable}")
                }
                other => format!("{other:?}"),
            };
            format!("{}:{kind}", d.line)
        })
        .collect::<Vec<_>>();
    diagnostics.sort();
    diagnostics.dedup();
    (format!("{:?}", row.status), diagnostics)
}

fn expect_in(close: Close, label: &str, body: &str, status: &str, diagnostics: &[&str]) {
    if !clang_available() {
        return;
    }
    let (actual_status, actual) = outcome(close, body);
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

fn expect(label: &str, body: &str, status: &str, diagnostics: &[&str]) {
    expect_in(Close::Release, label, body, status, diagnostics);
}

/// The gate's refusal at `line`, the reading every refused shape keeps.
fn gate(line: usize) -> String {
    format!("{line}:unsupported:indirect memory access is outside the resource projection")
}

/// The line every statement of a one-line fixture body is reported on.
const L: usize = 12;

fn undecided(line: usize, place: &str) -> String {
    format!(
        "{line}:unsupported:descriptor cell `{place}` is not decided (descriptor-cell-undecided)"
    )
}

fn double_release(line: usize, place: &str) -> String {
    format!(
        "{line}:unsupported:descriptor cell `{place}` is not decided (descriptor-cell-double-release)"
    )
}

/// Card CELL-PROTOCOL-PER-END-GUARDS (rule 38f): a cell that may still name
/// a live descriptor when its record does not keep its protocol at the exit.
fn escapes(line: usize, root: &str, cell: &str) -> String {
    format!(
        "{line}:unsupported:the descriptor in cell `({root} as int).{cell} [Name(\"{root}\") as int]` may outlive this function's frame (descriptor-cell-escapes)"
    )
}

fn broken(line: usize, root: &str, record: &str) -> String {
    format!(
        "{line}:unsupported:`{root}` (a `struct {record}`) does not keep its declared cell protocol here (descriptor-cell-protocol-broken)"
    )
}

// -- Admitted: an integer member read of a protocol cell at close's release ---

/// `flb_input_instance_destroy`'s ends over one guard test: each `close` is
/// its cell's `#stale` (Q3; the event's `#same` name is staled with the
/// first), and the sentinel store restores the protocol, which the record
/// keeps at the exit.
#[test]
fn a_close_of_a_protocol_cell_is_its_stale() {
    expect(
        "C8 closes under one test",
        "int d(struct pos_chan *s) { if (s->e[0] > 0) { close(s->e[0]); close(s->e[1]); \
         s->e[0] = -1; } return 0; }",
        "Safe",
        &[],
    );
}

/// The same ends spelled `mk_event_closesocket`, Monkey's Linux macro over
/// `close` (`#define mk_event_closesocket(s) close(s)`): the unit the
/// gateway reads is preprocessed, so the actual is the same.
#[test]
fn mk_event_closesocket_is_the_same_close() {
    expect(
        "C8 closes through the macro",
        "int d(struct pos_chan *s) { if (s->e[0] > 0) { mk_event_closesocket(s->e[0]); \
         mk_event_closesocket(s->e[1]); s->e[0] = -1; } return 0; }",
        "Safe",
        &[],
    );
}

/// A `!= -1` sentinel closed with `close`, both sentinels stored dead.
#[test]
fn a_sentinel_cell_closed_with_close_is_decided() {
    expect(
        "C1 shape with closes",
        "int d(struct chan *s) { if (s->ch[0] != -1) { close(s->ch[0]); close(s->ch[1]); \
         s->ch[0] = -1; s->ch[1] = -1; } return 0; }",
        "Safe",
        &[],
    );
}

/// A `== 1` flag's cells closed with `close`, then the flag stored dead.
#[test]
fn a_flag_protocols_cells_closed_with_close_are_decided() {
    expect(
        "C7 shape with closes",
        "int d(struct flag_chan *s) { if (s->on == 1) { close(s->n[0]); close(s->n[1]); \
         s->on = 0; } return 0; }",
        "Safe",
        &[],
    );
}

/// A plain (non-array) one-segment cell.
#[test]
fn a_scalar_member_cell_is_admitted() {
    expect(
        "one cell",
        "int d(struct one_chan *s) { if (s->fd > 0) { close(s->fd); s->fd = -1; } return 0; }",
        "Safe",
        &[],
    );
}

/// A cell whose integer type is a typedef (`flb_pipefd_t`), as Fluent Bit
/// spells its channel members.
#[test]
fn a_typedef_integer_cell_is_admitted() {
    expect(
        "typedef cells",
        "int d(struct fb_chan *s) { if (s->ch_events[0] > 0) { \
         mk_event_closesocket(s->ch_events[0]); mk_event_closesocket(s->ch_events[1]); \
         s->ch_events[0] = -1; } return 0; }",
        "Safe",
        &[],
    );
}

// -- Admitted, then not decided by rule 38c: the path refuses -----------------

/// `flb_output_instance_destroy`'s spelling exactly (flb_output.c:615-621 at
/// 046accf6): each end under its own `> 0` test and no sentinel store. The
/// second test is no protocol guard; on the first test's dead edge the second
/// close names no live descriptor, and the record is broken at the exit. The
/// gate's refusal is gone; rule 38c's two refusals stand. This is what the
/// merged tree reads at that end once CELL-PROTOCOL-FB-DECLARATIONS declares
/// the record.
#[test]
fn fluent_bits_output_instance_ends_refuse_after_admission() {
    expect(
        "C9 shape",
        "int d(struct fb_chan *s) { if (s->ch_events[0] > 0) { \
         mk_event_closesocket(s->ch_events[0]); } if (s->ch_events[1] > 0) { \
         mk_event_closesocket(s->ch_events[1]); } return 0; }",
        "Unknown",
        &[
            &broken(L, "s", "fb_chan"),
            &undecided(L, "s->ch_events[1]"),
            // Rule 38f: the first test's dead edge leaves both ends undecided
            // (either may be descriptor 0), and the record is broken, so
            // they escape.
            &escapes(L, "s", "ch_events[0]"),
            &escapes(L, "s", "ch_events[1]"),
        ],
    );
}

/// A close through a record the body already released refuses: releasing a
/// caller's record is an unresolved identity, after which `s` names no
/// caller's object with a fact, so rule 38c reads nothing and the gate's
/// refusal stands. Card CELL-PROTOCOL-PER-END-GUARDS (rule 38f): the release
/// ends the record's protocol, and since its cells are live there it refuses.
#[test]
fn a_close_through_a_released_record_refuses() {
    expect(
        "close after the holder's release",
        "int d(struct pos_chan *s) { if (s->e[0] > 0) { ev_free(s); close(s->e[0]); } \
         return 0; }",
        "Unknown",
        &[
            "12:unknown_alias:s",
            "12:unsupported:`s` is released while a cell of it may name a live descriptor (descriptor-cell-owner-released)",
            &gate(L),
        ],
    );
}

/// Fluent Bit's spelling: each end under its own `> 0` test. Only the first
/// test is the protocol's guard; on its dead edge no cell is live, so the
/// second close names no live descriptor and refuses. The admitted close is
/// never a silent release.
#[test]
fn a_close_on_the_dead_edge_refuses() {
    expect(
        "C8/C9 with a test per end",
        "int d(struct pos_chan *s) { if (s->e[0] > 0) { close(s->e[0]); } \
         if (s->e[1] > 0) { close(s->e[1]); } return 0; }",
        "Unknown",
        &[
            &broken(L, "s", "pos_chan"),
            &undecided(L, "s->e[1]"),
            &escapes(L, "s", "e[0]"),
            &escapes(L, "s", "e[1]"),
        ],
    );
}

/// A close under the complement edge. Card CELL-PROTOCOL-PER-END-GUARDS
/// (rule 38f): the `<= 0` edge holds descriptor 0, so its cells are
/// undecided, not absent: the close refuses, and the record, touched, is
/// broken with its undecided cell escaping.
#[test]
fn a_close_under_the_dead_guard_refuses() {
    expect(
        "close where the guard says dead",
        "int d(struct pos_chan *s) { if (s->e[0] <= 0) { close(s->e[0]); } return 0; }",
        "Unknown",
        &[
            &broken(L, "s", "pos_chan"),
            &undecided(L, "s->e[0]"),
            &escapes(L, "s", "e[0]"),
        ],
    );
}

/// A second close of one cell, and a close of the `#same` name's group after
/// its first name: never two closes of one descriptor.
#[test]
fn a_second_close_refuses() {
    expect(
        "close twice",
        "int d(struct pos_chan *s) { if (s->e[0] > 0) { close(s->e[0]); close(s->e[0]); \
         s->e[0] = -1; } return 0; }",
        "Unknown",
        &[
            &broken(L, "s", "pos_chan"),
            &double_release(L, "s->e[0]"),
            &escapes(L, "s", "e[0]"),
        ],
    );
}

/// The ends close but the sentinel still says live: broken at the exit.
#[test]
fn closes_that_leave_the_sentinel_break_the_protocol() {
    expect(
        "closes without the sentinel store",
        "int d(struct pos_chan *s) { if (s->e[0] > 0) { close(s->e[0]); close(s->e[1]); } \
         return 0; }",
        "Unknown",
        &[
            &broken(L, "s", "pos_chan"),
            &escapes(L, "s", "e[0]"),
            &escapes(L, "s", "e[1]"),
        ],
    );
}

/// Without a guard test the path holds no fact of the record, so rule 38c
/// reads nothing and the admitted close keeps the gate's refusal, byte for
/// byte.
#[test]
fn an_untested_close_keeps_the_gate_refusal() {
    expect(
        "close without a test",
        "int d(struct pos_chan *s) { close(s->e[0]); s->e[0] = -1; return 0; }",
        "Unknown",
        &[&gate(L)],
    );
}

/// After a call that may reach the record the fact is forgotten (rule 38c
/// (4)): the close keeps the gate's refusal.
#[test]
fn a_close_after_a_forwarding_call_keeps_the_gate_refusal() {
    expect(
        "close after `ev_note(s)`",
        "int d(struct pos_chan *s) { if (s->e[0] > 0) { ev_note(s); close(s->e[0]); \
         close(s->e[1]); s->e[0] = -1; } return 0; }",
        "Unknown",
        &[&gate(L)],
    );
}

/// A record this body allocates is tracked, never a caller's: the close keeps
/// the gate's refusal (rule 38c reads a close only on a caller's record).
#[test]
fn a_close_on_a_tracked_record_keeps_the_gate_refusal() {
    expect(
        "close on an allocated record",
        "int d(void) { struct pos_chan *c = ev_alloc(sizeof(struct pos_chan)); \
         if (!c) { return 0; } close(c->e[0]); ev_free(c); return 0; }",
        "Unknown",
        &[&gate(L)],
    );
}

// -- Refused at the gate, byte for byte ---------------------------------------

/// A record type with no declared protocol.
#[test]
fn an_undeclared_record_keeps_the_gate_refusal() {
    expect(
        "close over `struct other`",
        "int d(struct other *s) { if (s->e[0] > 0) { close(s->e[0]); close(s->e[1]); \
         s->e[0] = -1; } return 0; }",
        "Unknown",
        &[&gate(L)],
    );
}

/// A member of a protocol record that is not one of its cells.
#[test]
fn a_non_cell_member_keeps_the_gate_refusal() {
    expect(
        "close of `aux`",
        "int d(struct pos_chan *s) { if (s->e[0] > 0) { close(s->aux); } return 0; }",
        "Unknown",
        // Card MEMBER-EFFECT-SUMMARY (rule 38i) re-pins this deliberately: the release of a formal's member names
        // its premise; still `Unknown`.
        &[&format!(
            "{L}:unsupported:release of `s->aux`, a member of the formal `s` \
             (member-release-through-formal)"
        )],
    );
}

/// A multi-segment cell (`event.fd`, the `#same` name).
#[test]
fn a_multi_segment_cell_keeps_the_gate_refusal() {
    expect(
        "close of `event.fd`",
        "int d(struct pos_chan *s) { if (s->e[0] > 0) { close(s->event.fd); } return 0; }",
        "Unknown",
        &[&gate(L)],
    );
}

/// A copied number is not a name of the cell (Q4): the close of the copy is
/// read as before, and the record's cell is not staled.
#[test]
fn a_copied_number_is_not_a_cell_close() {
    expect(
        "close of a copy",
        "int d(struct pos_chan *s) { if (s->e[0] > 0) { int fd = s->e[0]; close(fd); \
         s->e[0] = -1; } return 0; }",
        "Unknown",
        &[
            "12:unknown_alias:fd",
            &broken(L, "s", "pos_chan"),
            "12:unsupported:descriptor cell `s->e[0]` is not decided (descriptor-cell-overwritten)",
        ],
    );
}

/// A cell inside a larger actual, or through a cast: not an exact member read.
#[test]
fn a_cell_inside_an_expression_keeps_the_gate_refusal() {
    expect(
        "close of an expression",
        "int d(struct pos_chan *s) { if (s->e[0] > 0) { close(s->e[0] + 0); } return 0; }",
        "Unknown",
        &[&gate(L)],
    );
}

/// `close` declared at a custody position that is not a release.
#[test]
fn a_non_release_position_keeps_the_gate_refusal() {
    expect_in(
        Close::Consumed,
        "close consumes",
        "int d(struct pos_chan *s) { if (s->e[0] > 0) { close(s->e[0]); } return 0; }",
        "Unknown",
        &[
            &broken(L, "s", "pos_chan"),
            &gate(L),
            &escapes(L, "s", "e[0]"),
        ],
    );
}

/// `close` declared as a heap release: no descriptor kind.
#[test]
fn a_non_descriptor_release_keeps_the_gate_refusal() {
    expect_in(
        Close::Heap,
        "close releases a heap object",
        "int d(struct pos_chan *s) { if (s->e[0] > 0) { close(s->e[0]); } return 0; }",
        "Unknown",
        &[
            "12:unsupported:call to `close` whose declared contract was withdrawn against its C declaration in this translation unit (abi-parameter-type-disagreement)",
        ],
    );
}

/// No contract for `close`.
#[test]
fn an_uncontracted_close_keeps_the_gate_refusal() {
    expect_in(
        Close::Uncontracted,
        "close has no contract",
        "int d(struct pos_chan *s) { if (s->e[0] > 0) { close(s->e[0]); } return 0; }",
        "Unknown",
        &[
            &broken(L, "s", "pos_chan"),
            "12:unsupported:call to uncontracted function `close`",
            &escapes(L, "s", "e[0]"),
        ],
    );
}

/// Windows' `closesocket` (Monkey's `mk_event_closesocket` there) has no
/// contract in any provider: uncontracted, and no rule-38c close.
#[test]
fn windows_closesocket_has_no_contract_and_keeps_the_gate_refusal() {
    expect(
        "closesocket",
        "int d(struct pos_chan *s) { if (s->e[0] > 0) { closesocket(s->e[0]); } return 0; }",
        "Unknown",
        &["12:unsupported:call to uncontracted function `closesocket`"],
    );
}

/// A close whose result is used is no rule-38c close statement.
#[test]
fn a_close_whose_result_is_used_keeps_the_gate_refusal() {
    expect(
        "the result stored",
        "int d(struct pos_chan *s) { int r = 0; if (s->e[0] > 0) { r = close(s->e[0]); } \
         return r; }",
        "Unknown",
        &[&gate(L)],
    );
}

// -- The switch never widens -------------------------------------------------

const CHILD: &str = "RULE38D_SWITCH_CHILD";

/// Rule 38's switch (`QCORE_GATEWAY_RULES_OFF`, read once per process, so the
/// switched case runs this binary again as a child): off, rule 38c reads no
/// protocol and the gate admits nothing, so the close keeps the gate's
/// refusal byte for byte.
#[test]
fn switching_rule_38_off_keeps_the_gate_refusal() {
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
    expect(
        "C8 closes with rule 38 off",
        "int d(struct pos_chan *s) { if (s->e[0] > 0) { close(s->e[0]); close(s->e[1]); \
         s->e[0] = -1; } return 0; }",
        "Unknown",
        &[&gate(L)],
    );
}
