//! Card RETAINING-CALL-MINTED-NAME-ACTUALS (gateway rule 38b): rule 12t
//! classifies a withheld call a second time under its minted name
//! (`qcore_retained__<callee>`). W44-INDIRECT-CALL-ARGUMENT-ACCESS's probe now
//! finds its admitted actuals under that name at every withheld declaration,
//! not only at the cell-bearing ones (rule 38a), for the call's own callee and
//! only at a declared `Borrowed` position. Before this card an admitted actual
//! left the gate in the outer classification and was refused again in the
//! inner one.
//!
//! One group of fixtures per declaration family, each under the provider's
//! own declarations: Monkey's event calls (`monkey/event_loop.qv`), Fluent
//! Bit's collector setters (`fluent_bit/input_collector_registration.qv`),
//! msgpack's packer (`msgpack/packer.qv`) and librdkafka's queues
//! (`librdkafka/client.qv`). Synthetic withheld declarations cover the
//! refusals no provider declaration isolates: a custody-taking position, a
//! rule-1 release, an element address at a declaration without cells, and a
//! callback's `resource_owned` requirement on an admitted position. Every
//! test asserts the complete diagnostic vector of `w` with its status, and a
//! `Safe` certificate is rechecked. A `Safe` here is `Safe` over the declared
//! assumptions and promotes nothing.
//!
//! No test here rests on rule 38 (the descriptor cells): no fixture calls a
//! cell-bearing declaration, so card MK-EVENT-CELL-CROSS-FUNCTION's changes to
//! rule 38 and the lifetime summaries do not reach these vectors.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyDiagnosticKind, LegacyProgramSummary, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

/// Synthetic callees: an allocator and its release, a function-map callee,
/// and four withheld declarations (each retains `data` in `loop`): `ev_hand`
/// takes custody of `slot`, `ev_drop` releases a nullable `p`, `ev_keep`
/// borrows a pointer `slot`, and `ev_call` invokes `cb` with `ctx`, whose only
/// candidate `ev_owner_cb` requires its argument live (`resource_owned`).
const CONTRACTS: &str = "\
module contract;\n\
extern fn ev_alloc(n: usize) -> ptr<mut, u8>\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(n);\n\
           ensures normal resource_return_owned(nullable); };\n\
extern fn ev_free(p: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_released(p, nullable);\n\
           ensures normal resource_return_none(); };\n\
extern fn ev_note(v: i32) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(v);\n\
           ensures normal resource_return_none(); };\n\
extern fn ev_hand(loop: ptr<mut, u8>, slot: ptr<mut, u8>, data: ptr<mut, u8>) -> i32\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(loop);\n\
           ensures normal resource_consumed(slot, nullable);\n\
           ensures normal resource_borrowed(data);\n\
           ensures normal resource_retained_by(data, loop);\n\
           ensures normal resource_return_none(); };\n\
extern fn ev_drop(loop: ptr<mut, u8>, p: ptr<mut, u8>, data: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(loop);\n\
           ensures normal resource_released(p, nullable);\n\
           ensures normal resource_borrowed(data);\n\
           ensures normal resource_retained_by(data, loop);\n\
           ensures normal resource_return_none(); };\n\
extern fn ev_keep(loop: ptr<mut, u8>, slot: ptr<mut, u8>, data: ptr<mut, u8>) -> i32\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(loop);\n\
           ensures normal resource_borrowed(slot);\n\
           ensures normal resource_borrowed(data);\n\
           ensures normal resource_retained_by(data, loop);\n\
           ensures normal resource_return_none(); };\n\
extern fn ev_owner_cb(ctx: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_owned(ctx, nonnull);\n\
           ensures normal resource_return_none(); };\n\
extern fn ev_call(cb: fnptr, ctx: ptr<mut, u8>) -> unit\n\
contract { requires resource_contract_v1; ensures normal resource_borrowed(cb);\n\
           ensures normal resource_borrowed(ctx);\n\
           ensures normal resource_callback_invoked(cb, ctx);\n\
           ensures normal resource_return_none(); };";

const PRELUDE: &str = "\
struct mk_event_loop;
struct mk_event { int fd; int type; unsigned int mask; unsigned char status; void *data; int priority; };
struct mk_event_loop *mk_event_loop_create(int size);
void mk_event_loop_destroy(struct mk_event_loop *loop);
int mk_event_add(struct mk_event_loop *loop, int fd, int type, unsigned int mask, void *data);
int mk_event_del(struct mk_event_loop *loop, struct mk_event *event);
void *ev_alloc(__SIZE_TYPE__ n); void ev_free(void *p);
void ev_note(int v);
int ev_hand(struct mk_event_loop *loop, void *slot, void *data);
void ev_drop(struct mk_event_loop *loop, void *p, void *data);
int ev_keep(struct mk_event_loop *loop, int *slot, void *data);
void ev_owner_cb(void *ctx);
void ev_call(void (*cb)(void *), void *ctx);
struct chan { int ch[2]; int fd; struct mk_event_loop *evl; struct mk_event event; void *p; void *ctx; struct chan *next; };
struct flb_config { int x; struct mk_event_loop *evl; };
struct flb_input_instance { int id; struct flb_config *config; };
struct tail_ctx { int ch_manager[2]; int fd_notify; long refresh_sec; long refresh_nsec; };
int flb_input_set_collector_event(struct flb_input_instance *ins,
    int (*cb)(struct flb_input_instance *, struct flb_config *, void *), int fd, struct flb_config *config);
int flb_input_set_collector_time(struct flb_input_instance *ins,
    int (*cb)(struct flb_input_instance *, struct flb_config *, void *), long seconds, long nanoseconds,
    struct flb_config *config);
static int collect(struct flb_input_instance *ins, struct flb_config *config, void *in_context) { return 0; }
typedef struct msgpack_packer { void *data; int (*callback)(void *data, const char *buf, __SIZE_TYPE__ len); } msgpack_packer;
void msgpack_packer_init(msgpack_packer *pk, void *data, int (*callback)(void *data, const char *buf, __SIZE_TYPE__ len));
static int pack_write(void *data, const char *buf, __SIZE_TYPE__ len) { return 0; }
struct chunk { int busy; msgpack_packer mp_pck; void *in; };
typedef struct rd_kafka_q_s { int q; } rd_kafka_q_t;
typedef struct rd_kafka_s { int t; rd_kafka_q_t *rk_rep; } rd_kafka_t;
typedef struct rd_kafka_queue_s { rd_kafka_q_t *rkqu_q; rd_kafka_t *rkqu_rk; } rd_kafka_queue_t;
rd_kafka_queue_t *rd_kafka_queue_new0(rd_kafka_t *rk, rd_kafka_q_t *rkq);
void rd_kafka_q_destroy_owner(rd_kafka_q_t *rkq);
";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        ("retaining_call_minted_name.qv".to_owned(), CONTRACTS.to_owned()),
        (
            "monkey/event_loop.qv".to_owned(),
            include_str!(
                "../../../providers/qcore-provider-libevent/contracts/monkey/event_loop.qv"
            )
            .to_owned(),
        ),
        (
            "fluent_bit/input_collector_registration.qv".to_owned(),
            include_str!(
                "../../../providers/quirin-provider-fluent-bit/contracts/fluent_bit/input_collector_registration.qv"
            )
            .to_owned(),
        ),
        (
            "msgpack/packer.qv".to_owned(),
            include_str!("../../../providers/quirin-provider-msgpack/contracts/msgpack/packer.qv")
                .to_owned(),
        ),
        (
            "librdkafka/client.qv".to_owned(),
            include_str!(
                "../../../providers/qcore-provider-librdkafka/contracts/librdkafka/client.qv"
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
            eprintln!("skipping Clang-dependent RETAINING-CALL-MINTED-NAME-ACTUALS test: {reason}");
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

/// The status of `w` and its complete, sorted, de-duplicated diagnostic
/// vector. The unit holds only `w` and the prelude's static callbacks, which
/// are analysed `Safe`, so the program's status is `w`'s.
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

/// The access gate's refusal: the vector of every fixture whose refusal is
/// the one this card does not remove.
const GATE: &str = "unsupported:indirect memory access is outside the resource projection";

// -- Monkey's event calls (monkey/event_loop.qv) -----------------------------

#[test]
fn mk_event_add_and_del_through_a_formal_record_s_loop_is_safe() {
    expect(
        "`mk_event_add(s->evl, s->fd, …, &s->event)` then `mk_event_del(s->evl, &s->event)`",
        "int w(struct chan *s) { mk_event_add(s->evl, s->fd, 0, 1, &s->event); \
         mk_event_del(s->evl, &s->event); return 0; }",
        "Safe",
        &[],
    );
}

#[test]
fn mk_event_add_of_an_element_fd_reaches_the_retention_reading() {
    expect(
        "`mk_event_add(loop, c->ch[0], …, &c->event)` over a tracked record: the element \
         is W44's scalar read, never a cell",
        "int w(struct mk_event_loop *loop) { struct chan *c = ev_alloc(sizeof(struct chan)); \
         if (!c) { return -1; } mk_event_add(loop, c->ch[0], 0, 0, &c->event); \
         mk_event_del(loop, &c->event); ev_free(c); return 0; }",
        "Unknown",
        &[
            "unknown_alias:c",
            "unsupported:call to `mk_event_add` retains `& c - > event` in `loop`, whose lifetime \
             this body does not decide against it (retention-undecided)",
        ],
    );
}

/// `flb_input_collector_destroy`'s shape (slice, `flb_input.c:3150`) without
/// its cell-bearing timeout arm: the removal through a local's loop is read,
/// and `coll`'s identity at the release stays unresolved.
#[test]
fn mk_event_del_through_a_local_config_s_loop_reaches_the_removal() {
    expect(
        "`mk_event_del(config->evl, &coll->event)` then `ev_free(coll)`",
        "struct coll { struct mk_event event; int type; struct flb_input_instance *instance; };\n\
         int w(struct coll *coll) { struct flb_config *config = coll->instance->config; \
         mk_event_del(config->evl, &coll->event); ev_free(coll); return 0; }",
        "Unknown",
        &["unknown_alias:coll"],
    );
}

#[test]
fn an_actual_the_probe_refuses_keeps_the_refusal() {
    expect(
        "`mk_event_add(loop, s->fd, …, &s->p)`: W44 refuses the address of a pointer member, \
         so the call keeps the refusal although `s->fd` is admitted",
        "int w(struct mk_event_loop *loop) { struct chan *s = ev_alloc(sizeof(struct chan)); \
         if (!s) { return -1; } mk_event_add(loop, s->fd, 0, 1, &s->p); ev_free(s); return 0; }",
        "Unknown",
        &["unknown_alias:s", GATE],
    );
}

// -- Fluent Bit's collector setters (fluent_bit/input_collector_registration.qv)

#[test]
fn a_collector_event_on_an_element_fd_is_safe() {
    expect(
        "`in_tail_init`'s `flb_input_set_collector_event(in, cb, ctx->ch_manager[0], config)`",
        "int w(struct flb_input_instance *in, struct tail_ctx *ctx, struct flb_config *config) { \
         return flb_input_set_collector_event(in, collect, ctx->ch_manager[0], config); }",
        "Safe",
        &[],
    );
}

#[test]
fn a_collector_time_on_member_intervals_is_safe() {
    expect(
        "`flb_input_set_collector_time(in, cb, ctx->refresh_sec, ctx->refresh_nsec, config)`",
        "int w(struct flb_input_instance *in, struct tail_ctx *ctx, struct flb_config *config) { \
         return flb_input_set_collector_time(in, collect, ctx->refresh_sec, ctx->refresh_nsec, \
         config); }",
        "Safe",
        &[],
    );
}

// -- msgpack's packer (msgpack/packer.qv) -------------------------------------

#[test]
fn a_packer_inside_a_tracked_chunk_keeps_the_refusal() {
    expect(
        "`msgpack_packer_init(&ic->mp_pck, ic, cb)`: rule 12t refuses the call, so the \
         gate's refusal is restored",
        "struct chunk *w(void *in) { struct chunk *ic = ev_alloc(sizeof(struct chunk)); \
         if (!ic) { return 0; } msgpack_packer_init(&ic->mp_pck, ic, pack_write); return ic; }",
        "Unknown",
        &["unknown_alias:ic", GATE],
    );
}

#[test]
fn a_packer_in_a_formal_record_keeps_the_refusal() {
    expect(
        "`msgpack_packer_init(&ic->mp_pck, ic, cb)` over a caller's record: rule 12t refuses \
         the call, so the gate's refusal is restored",
        "void w(struct chunk *ic) { msgpack_packer_init(&ic->mp_pck, ic, pack_write); }",
        "Unknown",
        &[GATE],
    );
}

// -- librdkafka's queues (librdkafka/client.qv) ------------------------------

#[test]
fn a_queue_over_a_member_queue_reaches_the_retention_reading() {
    expect(
        "`rd_kafka_queue_get_main`'s `rd_kafka_queue_new0(rk, rk->rk_rep)`",
        "rd_kafka_queue_t *w(rd_kafka_t *rk) { return rd_kafka_queue_new0(rk, rk->rk_rep); }",
        "Unknown",
        &[
            "unsupported:a holder retaining `rk` may outlive this function's frame (retention-undecided)",
        ],
    );
}

#[test]
fn an_owner_drop_of_a_member_queue_reaches_the_reference_reading() {
    expect(
        "`rd_kafka_queue_destroy`'s `rd_kafka_q_destroy_owner(rkqu->rkqu_q)`",
        "void w(rd_kafka_queue_t *rkqu) { rd_kafka_q_destroy_owner(rkqu->rkqu_q); }",
        "Unknown",
        &[
            "unsupported:call to `rd_kafka_q_destroy_owner` with `rkqu - > rkqu_q`: a drop of a \
             reference this function does not hold, or whose count is undecided \
             (reference-drop-unheld)",
        ],
    );
}

// -- Refusals no provider declaration isolates --------------------------------

#[test]
fn a_custody_taking_position_keeps_the_refusal() {
    expect(
        "`ev_hand(loop, s->p, s)`: `slot` is consumed",
        "int w(struct mk_event_loop *loop, struct chan *s) { return ev_hand(loop, s->p, s); }",
        "Unknown",
        &[GATE],
    );
}

#[test]
fn a_rule_1_release_at_a_withheld_declaration_keeps_the_refusal() {
    expect(
        "`ev_drop(loop, s->p, s)`: `p` is a nullable release, a custody position",
        "void w(struct mk_event_loop *loop, struct chan *s) { ev_drop(loop, s->p, s); }",
        "Unknown",
        &[GATE],
    );
}

#[test]
fn an_element_address_at_a_declaration_without_cells_keeps_the_refusal() {
    expect(
        "`ev_keep(loop, &s->ch[0], s)`: rule 38a's element admission stays cell-bearing",
        "int w(struct mk_event_loop *loop, struct chan *s) { return ev_keep(loop, &s->ch[0], s); }",
        "Unknown",
        &[GATE],
    );
}

#[test]
fn a_callback_requirement_on_an_admitted_position_keeps_the_refusal() {
    expect(
        "`ev_call(ev_owner_cb, s->ctx)` over a tracked `s`: rule 12t reads `ctx` as \
         `resource_owned` after the inner classification",
        "void w(void) { struct chan *s = ev_alloc(sizeof(struct chan)); if (!s) { return; } \
         ev_call(ev_owner_cb, s->ctx); ev_free(s); }",
        "Unknown",
        &[GATE],
    );
}

#[test]
fn an_uncontracted_callee_keeps_the_refusal() {
    expect(
        "`ev_unknown(s->fd)`: no contract in force",
        "void ev_unknown(int fd);\nvoid w(struct chan *s) { ev_unknown(s->fd); }",
        "Unknown",
        &["unsupported:call to uncontracted function `ev_unknown`"],
    );
}

#[test]
fn a_function_map_callee_is_read_as_before() {
    expect(
        "`ev_note(s->fd)`: a function-map callee, which rule 12t never classifies under a \
         minted name; W44 admits the read as before this card",
        "void w(struct chan *s) { ev_note(s->fd); }",
        "Safe",
        &[],
    );
}

#[test]
fn a_released_base_is_still_accused() {
    expect(
        "`mk_event_del(s->evl, &c->event)` after `ev_free(c)`: W44's accusation applies under \
         the minted name",
        "int w(struct chan *s) { struct chan *c = ev_alloc(sizeof(struct chan)); \
         if (!c) { return -1; } ev_free(c); mk_event_del(c->evl, &s->event); return 0; }",
        "Unsafe",
        &["unknown_alias:c", "unknown_alias:s/c", "use_after_release"],
    );
}
