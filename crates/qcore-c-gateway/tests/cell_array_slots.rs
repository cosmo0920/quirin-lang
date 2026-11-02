//! Card MK-EVENT-CELL-ARRAY-SLOTS (gateway rule 38a): a constant-index
//! element of an integer array member, read (`X->f[K]`) or addressed
//! (`&X->f[K]`) at a cell-bearing withheld declaration's `Borrowed` position,
//! leaves the lexical access gate under W44-INDIRECT-CALL-ARGUMENT-ACCESS's
//! base obligations. The call's other admitted actuals, such as the loop
//! `s->evl`, leave it too, so rule 38 decides the cell
//! (`&sched->ch_events[0]`, `config->ch_self_events[0]`).
//!
//! Almost every fixture's base is a record this body allocates (`ev_alloc`),
//! so no older route admits the element: rule 10n/14c's typed address needs a
//! root the body never binds. The teardown fixture is a caller's record, as
//! in `flb_sched_destroy`. Every test asserts the complete diagnostic vector
//! of `w` with its status, and a `Safe` certificate is rechecked. The
//! declarations are the provider's own restated Monkey declarations
//! (`providers/qcore-provider-libevent/contracts/monkey/event_loop.qv`) and
//! synthetic test callees, so a `Safe` here is `Safe` over those declared
//! assumptions and promotes nothing.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

/// Synthetic test callees: an allocator and its release, a function-map
/// callee with a `Borrowed` slot (not withheld), and a cell-bearing withheld
/// declaration (it retains `data` and creates a descriptor in `*r_fd`) whose
/// `slot` position takes custody.
const CONTRACTS: &str = "\
module contract;\n\
extern fn ev_alloc(n: usize) -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(n);\n\
           ensures normal resource_return_owned(nullable); };\n\
extern fn ev_free(p: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(p, nullable);\n\
           ensures normal resource_return_none(); };\n\
extern fn ev_note(slot: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(slot);\n\
           ensures normal resource_return_none(); };\n\
extern fn ev_hand(loop: ptr<mut, u8>, r_fd: ptr<mut, u8>, slot: ptr<mut, u8>, data: ptr<mut, u8>) -> i32\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(loop);\n\
           ensures normal resource_borrowed(r_fd);\n\
           ensures normal resource_consumed(slot, nullable);\n\
           ensures normal resource_borrowed(data);\n\
           ensures normal resource_retained_by(data, loop);\n\
           quant live #r_fd: #pointee |-> #fresh(descriptor) when #result == 0;\n\
           ensures normal resource_return_none(); };";

const PRELUDE: &str = "\
struct mk_event_loop;
struct mk_event { int fd; int mask; };
struct chan { int ch[2]; int *p; double d[2]; char k[2]; struct mk_event_loop *evl; struct mk_event event; };
void *ev_alloc(__SIZE_TYPE__ n); void ev_free(void *p);
void ev_note(int *slot);
int ev_hand(struct mk_event_loop *loop, int *r_fd, int *slot, void *data);
struct mk_event_loop *mk_event_loop_create(int size);
void mk_event_loop_destroy(struct mk_event_loop *loop);
int mk_event_add(struct mk_event_loop *loop, int fd, int type, unsigned int mask, void *data);
int mk_event_del(struct mk_event_loop *loop, struct mk_event *event);
int mk_event_channel_create(struct mk_event_loop *loop, int *r_fd, int *w_fd, void *data);
int mk_event_channel_destroy(struct mk_event_loop *loop, int r_fd, int w_fd, void *data);
";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        ("cell_array_slots.qv".to_owned(), CONTRACTS.to_owned()),
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
            eprintln!("skipping Clang-dependent MK-EVENT-CELL-ARRAY-SLOTS test: {reason}");
            false
        }
    }
}

fn program(result: &UnrestrictedLegacyResult) -> &LegacyProgramSummary {
    match result {
        UnrestrictedLegacyResult::Safe(certificate) => &certificate.summary,
        UnrestrictedLegacyResult::Unsafe(summary) => summary,
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        } => summary,
        other => panic!("no projection at all: {other:?}"),
    }
}

/// The status and the complete, sorted, de-duplicated diagnostic vector of `w`.
fn outcome(body: &str) -> (&'static str, Vec<String>) {
    let source = format!("{PRELUDE}\n{body}\n");
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "w",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{source}"));
    let status = match &result {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(certificate)
                .expect("a Safe result carries a certificate that rechecks");
            "Safe"
        }
        UnrestrictedLegacyResult::Unsafe(_) => "Unsafe",
        UnrestrictedLegacyResult::Unknown { .. } => "Unknown",
    };
    let mut keys = program(&result)
        .functions
        .get("w")
        .expect("missing function `w`")
        .diagnostics
        .iter()
        .map(|diagnostic| match &diagnostic.kind {
            LegacyDiagnosticKind::UnknownAlias { variable } => format!("unknown_alias:{variable}"),
            LegacyDiagnosticKind::UseAfterRelease { .. } => "use_after_release".to_owned(),
            LegacyDiagnosticKind::Unsupported { description } => {
                format!("unsupported:{description}")
            }
            other => format!("{other:?}"),
        })
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    (status, keys)
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
        (actual_status, &actual),
        (status, &wanted),
        "{label}\n{body}"
    );
}

const GATE: &str = "unsupported:indirect memory access is outside the resource projection";

/// A channel created into the slots of a record this body allocates, then
/// destroyed through `SLOTS` and freed. `CREATE` and `SLOTS` are replaced.
const CYCLE: &str = "\
int w(void) {
    struct mk_event_loop *loop = mk_event_loop_create(8);
    if (!loop) { return -1; }
    struct chan *c = ev_alloc(sizeof(struct chan));
    if (!c) { mk_event_loop_destroy(loop); return -1; }
    int ret = mk_event_channel_create(loop, CREATE, &c->event);
    if (ret != 0) { mk_event_loop_destroy(loop); ev_free(c); return -1; }
    mk_event_channel_destroy(loop, SLOTS, &c->event);
    mk_event_loop_destroy(loop);
    ev_free(c);
    return 0;
}";

fn cycle(create: &str, slots: &str) -> String {
    CYCLE.replace("CREATE", create).replace("SLOTS", slots)
}

const ESCAPES_0: &str = "unsupported:the descriptor in cell `(an object this body tracks as int).ch[0] [Site(1) as int]` may outlive this function's frame (descriptor-cell-escapes)";
const ESCAPES_1: &str = "unsupported:the descriptor in cell `(an object this body tracks as int).ch[1] [Site(1) as int]` may outlive this function's frame (descriptor-cell-escapes)";
const ESCAPES_EVENT: &str = "unsupported:the descriptor in cell `(an object this body tracks as mk_event).fd [SiteInterior(1, [\"event\"]) as mk_event]` may outlive this function's frame (descriptor-cell-escapes)";
const OWNER_RELEASED: &str = "unsupported:`c` is released while a cell of it may name a live descriptor (descriptor-cell-owner-released)";

// -- Admitted: rule 38 decides the cell --------------------------------------

/// D3 and D4 over array slots: the creation's slots `&c->ch[0]`, `&c->ch[1]`
/// and the destroy's by-value reads `c->ch[0]`, `c->ch[1]` leave the gate, the
/// member is a second name of the read end, so the guard is decided equal and
/// both ends close before `c` is freed.
#[test]
fn a_channel_in_tracked_slots_closed_through_its_read_end_is_safe() {
    expect(
        "the channel cycle over `int ch[2]`",
        &cycle("&c->ch[0], &c->ch[1]", "c->ch[0], c->ch[1]"),
        "Safe",
        &[],
    );
}

/// The same cycle with the ends swapped: rule 38 decides the guard distinct,
/// nothing closes, the live cells escape and refuse the owner's release.
#[test]
fn a_destroy_through_the_other_end_refuses_in_rule_38() {
    expect(
        "the ends swapped at the destroy",
        &cycle("&c->ch[0], &c->ch[1]", "c->ch[1], c->ch[0]"),
        "Unknown",
        &[ESCAPES_0, ESCAPES_1, ESCAPES_EVENT, OWNER_RELEASED],
    );
}

/// `flb_sched_destroy`'s shape: a caller's record whose loop is a member
/// (`s->evl`) and whose slots are read by value. The call reaches rule 38,
/// whose guard no body here decides (the creation is in another function).
#[test]
fn a_callers_teardown_reaches_rule_38_and_its_guard_refuses() {
    expect(
        "`mk_event_channel_destroy(s->evl, s->ch[0], s->ch[1], &s->event)`",
        "int w(struct chan *s) { if (s->ch[0] != -1) { \
         mk_event_channel_destroy(s->evl, s->ch[0], s->ch[1], &s->event); \
         s->ch[0] = -1; s->ch[1] = -1; } ev_free(s); return 0; }",
        "Unknown",
        &[
            "unknown_alias:s",
            "unsupported:call to `mk_event_channel_destroy`: descriptor cell \
             `(Interior(\"s\", [\"event\"], true) as mk_event).fd` is not decided \
             (descriptor-cell-guard-undecided)",
        ],
    );
}

// -- Accused -----------------------------------------------------------------

/// The element read keeps its base's obligation: after `ev_free(c)` the read
/// `c->ch[0]` is a use after release.
#[test]
fn an_element_read_after_its_base_is_released_is_accused() {
    expect(
        "`c->ch[0]` after `ev_free(c)`",
        "int w(struct mk_event_loop *loop) { struct chan *c = ev_alloc(sizeof(struct chan)); \
         if (!c) { return -1; } ev_free(c); \
         mk_event_channel_destroy(loop, c->ch[0], c->ch[1], &c->event); return 0; }",
        "Unsafe",
        &["unknown_alias:c", "use_after_release"],
    );
}

// -- Refused: the gate's refusal is kept -------------------------------------

/// The refusal every slot shape below keeps, with the unresolved names the
/// refused creation leaves behind.
const KEPT: &[&str] = &["unknown_alias:c", "unknown_alias:loop", GATE];

fn refused_slot(label: &str, slot: &str) {
    expect(
        label,
        &cycle(&format!("{slot}, &c->ch[1]"), "c->ch[0], c->ch[1]"),
        "Unknown",
        KEPT,
    );
}

#[test]
fn a_variable_index_keeps_the_refusal() {
    refused_slot("`&c->ch[ret]`", "&c->ch[ret]");
}

#[test]
fn an_out_of_bounds_index_keeps_the_refusal() {
    refused_slot("`&c->ch[2]` over `int ch[2]`", "&c->ch[2]");
}

#[test]
fn a_negative_index_keeps_the_refusal() {
    refused_slot("`&c->ch[-1]`", "&c->ch[-1]");
}

#[test]
fn a_non_canonical_index_keeps_the_refusal() {
    refused_slot("`&c->ch[00]` (an octal literal)", "&c->ch[00]");
}

#[test]
fn a_pointer_member_keeps_the_refusal() {
    refused_slot("`&c->p[0]` over `int *p`", "&c->p[0]");
}

#[test]
fn a_non_integer_element_keeps_the_refusal() {
    refused_slot("`&c->d[0]` over `double d[2]`", "&c->d[0]");
    refused_slot("`&c->k[0]` over a plain `char k[2]`", "&c->k[0]");
}

#[test]
fn a_custody_taking_position_keeps_the_refusal() {
    expect(
        "`&c->ch[1]` at `ev_hand`'s consumed `slot`",
        "int w(struct mk_event_loop *loop) { struct chan *c = ev_alloc(sizeof(struct chan)); \
         if (!c) { return -1; } ev_hand(loop, &c->ch[0], &c->ch[1], &c->event); \
         ev_free(c); return 0; }",
        "Unknown",
        &["unknown_alias:c", GATE],
    );
}

#[test]
fn a_function_map_callee_keeps_the_refusal() {
    expect(
        "`ev_note(&c->ch[0])`, a `Borrowed` slot of a declaration that is not withheld",
        "int w(void) { struct chan *c = ev_alloc(sizeof(struct chan)); if (!c) { return -1; } \
         ev_note(&c->ch[0]); ev_free(c); return 0; }",
        "Unknown",
        &["unknown_alias:c", GATE],
    );
}

/// Card RETAINING-CALL-MINTED-NAME-ACTUALS (rule 38b): at a withheld
/// declaration without cells the element is never read as a cell (rule 38a
/// stays cell-bearing), but the by-value `c->ch[0]` is W44's typed scalar
/// read, which now also leaves the inner classification's gate. So the call
/// reaches rule 12t, which refuses the retention of `&c->event` in `loop`.
/// Before that card this was the gate's refusal.
#[test]
fn a_withheld_declaration_without_cells_reads_the_element_value_as_a_scalar() {
    expect(
        "`mk_event_add(loop, c->ch[0], …)`: withheld, but no descriptor-cell state",
        "int w(struct mk_event_loop *loop) { struct chan *c = ev_alloc(sizeof(struct chan)); \
         if (!c) { return -1; } mk_event_add(loop, c->ch[0], 0, 0, &c->event); \
         mk_event_del(loop, &c->event); ev_free(c); return 0; }",
        "Unknown",
        &[
            "unknown_alias:c",
            "unsupported:call to `mk_event_add` retains `& c - > event` in `loop`, whose \
             lifetime this body does not decide against it (retention-undecided)",
        ],
    );
}

// -- The switch never widens -------------------------------------------------

const CHILD: &str = "RULE38A_SWITCH_CHILD";

/// Rule 38's switch (`QCORE_GATEWAY_RULES_OFF`, read once per process, so the
/// switched case runs this binary again as a child): off, the admitted
/// element's call refuses as every cell call does.
#[test]
fn switching_rule_38_off_refuses_the_admitted_cycle() {
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
    let (status, diagnostics) = outcome(&cycle("&c->ch[0], &c->ch[1]", "c->ch[0], c->ch[1]"));
    assert_ne!(status, "Safe", "{diagnostics:?}");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("rule 38 is switched off")),
        "{diagnostics:?}"
    );
    assert!(
        !diagnostics.iter().any(|diagnostic| diagnostic == GATE),
        "the element still leaves the gate: {diagnostics:?}"
    );
}

// -- A record passed whole beside its member address -------------------------

/// MK-EVENT-FIRST-MEMBER-CELL's `MK-EVENT-CELL-WHOLE-AND-MEMBER-ACTUALS`: a
/// caller's record passed whole to one cell call and spelled `&c->event` at
/// another kept the access gate's refusal at the member address. That
/// refusal was the minted-name gap, so under rule 38a the member address
/// leaves the gate and rule 38 answers. With the first-member reading
/// (rule 38 (1)) the whole record and `&c->event` key one cell, so each
/// cycle closes what it created and is `Safe` (integration of the two
/// cards); a destroy through a record whose first member is not the event
/// keeps refusing (`descriptor_cells.rs`). Fluent Bit's shape, `ins` whole
/// beside `&ins->ch_events[0]`, is covered too.
#[test]
fn a_record_passed_whole_never_keeps_the_gate_at_its_member_address() {
    if !clang_available() {
        return;
    }
    let channel = |create: &str, destroy: &str| {
        format!(
            "struct wrapper {{ struct mk_event event; int ch[2]; }};
struct pair {{ int r; int w; }};
int w(struct wrapper *c, struct pair *s) {{
    struct mk_event_loop *loop = mk_event_loop_create(8);
    if (!loop) {{ return -1; }}
    int ret = mk_event_channel_create(loop, {create});
    if (ret != 0) {{ mk_event_loop_destroy(loop); return -1; }}
    mk_event_channel_destroy(loop, {destroy});
    mk_event_loop_destroy(loop);
    return 0;
}}"
        )
    };
    for (create, destroy) in [
        ("&s->r, &s->w, c", "s->r, s->w, &c->event"),
        ("&s->r, &s->w, &c->event", "s->r, s->w, c"),
        ("&c->ch[0], &c->ch[1], c", "c->ch[0], c->ch[1], c"),
    ] {
        let body = channel(create, destroy);
        let (status, diagnostics) = outcome(&body);
        assert!(
            !diagnostics.iter().any(|diagnostic| diagnostic == GATE),
            "{body}\n{diagnostics:?}"
        );
        assert_eq!(status, "Safe", "{body}\n{diagnostics:?}");
        assert!(diagnostics.is_empty(), "{body}\n{diagnostics:?}");
    }
}
