#!/usr/bin/env python3
"""write_contracts.py -- OP-APP-RK-CALLBACKS-VIA-POLL-CB: write the card's
contracts (idempotent).

* `qcore-provider-librdkafka/contracts/librdkafka/client.qv`: the card's
  section between its markers (after OP-CALLBACK-CANDIDATE-CONTRACTS'): the
  seventeen dedicated rk_conf callback setters (registrations, user decision
  Q2), `rd_kafka_conf_new` with its stated function defaults (D1), the
  invocation sites `rd_kafka_consume_cb` and
  `rd_kafka_background_queue_serve`, the op callback
  `rd_kafka_oauthbearer_refresh_op` (OP-CALLBACK-CANDIDATES-APP-CALLBACKS),
  and the librdkafka candidates (`rd_kafka_log_print`, the five OAUTHBEARER
  refreshers). Every body read by hand at `66910c10` and `046accf6`
  (`scripts/body.py`; librdkafka's `src/` is byte-identical,
  `artifacts/rk-conf-census.json`).
* The same module: the premises `rk_log_callback` and
  `rk_closesocket_callback` of OP-CALLBACK-CANDIDATE-CONTRACTS' declarations
  are restated as `rk_no_plugin_library`, the premise the user chose (U2),
  since the log and close-socket callbacks are now closed over the run's
  registrations and their candidates declared.
* `quirin-provider-fluent-bit/contracts/fluent_bit/logging.qv` (appended between
  the card's markers, so no module is added): the three Fluent Bit candidates (`cb_kafka_msg`, `cb_kafka_logger`,
  `oauthbearer_token_refresh_cb`), declared assumptions with each body read
  at both revisions (placement: Fluent Bit functions in the Fluent Bit
  provider)."""
import pathlib
import re

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
CLIENT = REPO / "providers/qcore-provider-librdkafka/contracts/librdkafka/client.qv"
FLUENT = REPO / "providers/quirin-provider-fluent-bit/contracts/fluent_bit/logging.qv"
BEGIN = "// BEGIN OP-APP-RK-CALLBACKS-VIA-POLL-CB (scripts/write_contracts.py)"
END = "// END OP-APP-RK-CALLBACKS-VIA-POLL-CB"
AFTER = "// END OP-CALLBACK-CANDIDATE-CONTRACTS\n"

PLUGIN = ("requires resource_premise(rk_no_plugin_library, \"P-RK-NO-PLUGIN-LIBRARY (user "
          "decision 2026-10-14, U2): no plugin.library.paths is configured, so no library "
          "outside the run (rd_kafka_plugin_new's dlsym conf_init) registers a callback or an "
          "interceptor into a conf, and every rk_conf callback a call may run, the log callback "
          "rd_kafka_log_buf calls included, is one the run registers or a stated default, each "
          "declared (OP-APP-RK-CALLBACKS-VIA-POLL-CB)\");")
WRITER = ("requires resource_premise(conf_property_writer, \"the setter stores through "
          "rd_kafka_anyconf_set_prop0's _RK_C_PTR case (rdkafka_conf.c:2087-2088), a "
          "type-punned store through the property table, which writes a callback member only "
          "from a dedicated rd_kafka_conf_set_*_cb setter (each declared a registration), a "
          "pdef default (stated on rd_kafka_conf_new), rd_kafka_anyconf_copy of the same "
          "property of another conf, or the string API for an _RK_HIDDEN property (only "
          "ut_handle_ProduceResponse); Clang's member facts see none of these stores\");")
ALLOCATION = ("requires resource_premise(asserted_allocation, \"the result is non-null because "
              "it is allocated with rd_calloc (rd.h:139-143), which asserts its result, and the "
              "audited compile commands define no NDEBUG, so it never returns NULL in the "
              "audited build\");")

# (setter, callback parameter, member path below rd_kafka_conf_s, line, result)
SETTERS = [
    ("rd_kafka_conf_set_background_event_cb", "event_cb", "background_event_cb", 2988, "unit"),
    ("rd_kafka_conf_set_dr_cb", "dr_cb", "dr_cb", 2996, "unit"),
    ("rd_kafka_conf_set_dr_msg_cb", "dr_msg_cb", "dr_msg_cb", 3007, "unit"),
    ("rd_kafka_conf_set_consume_cb", "consume_cb", "consume_cb", 3016, "unit"),
    ("rd_kafka_conf_set_rebalance_cb", "rebalance_cb", "rebalance_cb", 3023, "unit"),
    ("rd_kafka_conf_set_offset_commit_cb", "offset_commit_cb", "offset_commit_cb", 3033, "unit"),
    ("rd_kafka_conf_set_error_cb", "error_cb", "error_cb", 3044, "unit"),
    ("rd_kafka_conf_set_throttle_cb", "throttle_cb", "throttle_cb", 3053, "unit"),
    ("rd_kafka_conf_set_log_cb", "log_cb", "log_cb", 3064, "unit"),
    ("rd_kafka_conf_set_stats_cb", "stats_cb", "stats_cb", 3077, "unit"),
    ("rd_kafka_conf_set_oauthbearer_token_refresh_cb", "oauthbearer_token_refresh_cb",
     "sasl.oauthbearer.token_refresh_cb", 3085, "unit"),
    ("rd_kafka_conf_set_socket_cb", "socket_cb", "socket_cb", 3102, "unit"),
    ("rd_kafka_conf_set_connect_cb", "connect_cb", "connect_cb", 3108, "unit"),
    ("rd_kafka_conf_set_closesocket_cb", "closesocket_cb", "closesocket_cb", 3118, "unit"),
    ("rd_kafka_conf_set_open_cb", "open_cb", "open_cb", 3128, "unit"),
    ("rd_kafka_conf_set_resolve_cb", "resolve_cb", "resolve_cb", 3137, "unit"),
    ("rd_kafka_conf_set_ssl_cert_verify_cb", "ssl_cert_verify_cb", "ssl.cert_verify_cb", 3148,
     "i32"),
]

INTRO = """// # Application rk_conf callbacks
//
// Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (gateway rule 12n, contract
// specification C04.19.3 and C04.19.4), on the user's decisions of 2026-10-14
// ("Closed over FB's registrations"; then "Build the agent's recommended
// set": D1, Q2, Q3, U2, R1, F1, E1, S1). Every body was read at `66910c10` and
// `046accf6`; librdkafka's `src/` is byte-identical at both (the card's
// `artifacts/rk-conf-census.json`). Each declaration is pinned (C06.2).
//
// * **Registrations (Q2).** No dedicated setter stores into its member: each
//   is `rd_kafka_anyconf_set_internal(_RK_GLOBAL, conf, "NAME", cb)`
//   (`rdkafka_conf.c:2606-2615`), a property-table lookup ending in
//   `rd_kafka_anyconf_set_prop0`'s type-punned `_RK_C_PTR` store
//   (`:2087-2088`). Each setter is declared
//   `resource_callback_member_registered(conf, rd_kafka_conf_s.FIELD, cb)`:
//   for rule 12n's run closure every call the run makes is a store of its
//   actual; the premise `conf_property_writer` names what else that writer
//   serves. A declared assumption (no body check reads it).
// * **Defaults (D1).** `rd_kafka_conf_new` (`:2523-2530`) runs
//   `rd_kafka_defaultconf_set` (`:2500-2521`), which stores each `pdef` of
//   `rd_kafka_properties[]`: `log_cb` `rd_kafka_log_print` (`:691-693`),
//   `socket_cb` `rd_kafka_socket_cb_linux` (`:724-731`), `open_cb`
//   `rd_kafka_open_cb_linux` (`:747-754`); every other callback member stays
//   NULL from `rd_calloc`. They are stated with `|->` and are candidates.
// * **The copy (Q3).** `rd_kafka_new` struct-copies the conf
//   (`rk->rk_conf = *conf;`, `rdkafka.c:2574`); a member is keyed by its
//   innermost named record, so `rk->rk_conf.dr_msg_cb` and `conf->dr_msg_cb`
//   are both `rd_kafka_conf_s.dr_msg_cb`, and the declarations below spell the
//   exact path `rd_kafka_s.rk_conf.FIELD`.
// * **Plugins (U2).** librdkafka is built `WITH_PLUGINS` and Fluent Bit
//   forwards every `rdkafka.*` property (`src/flb_kafka.c:84-86`), so a
//   configured `plugin.library.paths` would hand the conf to a `conf_init`
//   outside the run (`rdkafka_plugin.c:90-101`). Every declaration resting on
//   the closure names the premise `rk_no_plugin_library`.
//
// Not declared, by the decision S1 and the vocabulary: `rd_kafka_poll_cb`
// (`rdkafka.c:5813-6096`) keeps the op on `PASS` (seven paths) and takes it
// on `HANDLED`, `KEEP` and `YIELD`: a return-code split C04.13 does not state
// over the unsigned `rd_kafka_op_res_t`, and no disposition stands beside a
// callback clause, so it stays uncontracted and rule 12m keeps refusing it at
// the serves; `rd_kafka_destroy_app` (`rdkafka.c:1150-1287`): it releases `rk`
// only without `RD_KAFKA_DESTROY_F_IMMEDIATE`, closes the consumer through
// `rd_kafka_poll_cb` and runs the interceptors' `on_destroy`, a list member
// read through a union (`rdkafka_interceptor.c:116`, `:412`); the socket and
// open defaults' invocation sites (transport, offset file), which no
// declaration states.
"""


def setter(name, callback, path, line, result):
    member = f"rd_kafka_conf_s.{path}"
    return f"""// `{name}` (`rdkafka_conf.c:{line}`): `rd_kafka_anyconf_set_internal`
// stores `{callback}` into `{member}` through the property table.
extern fn {name}(conf: ptr<mut, u8>, {callback}: fnptr) -> {result}
contract {{
    requires resource_contract_v1;
    {WRITER}
    {PLUGIN}
    ensures normal resource_borrowed(conf);
    ensures normal resource_borrowed({callback});
    ensures normal resource_callback_member_registered(conf, {member}, {callback});
    ensures normal resource_return_none();
}};
"""


CONF_NEW = f"""// `rd_kafka_conf_new()` (`rdkafka_conf.c:2523-2530`): `rd_calloc` (asserts its
// result), `rd_kafka_defaultconf_set(_RK_GLOBAL, conf)` storing each `pdef`,
// `rd_kafka_anyconf_clear_all_is_modified(conf)`; returns the conf, owned.
extern fn rd_kafka_conf_new() -> ptr<mut, u8>
contract {{
    requires resource_contract_v1;
    {ALLOCATION}
    {WRITER}
    {PLUGIN}
    quant live #result: rd_kafka_conf_s.log_cb |-> rd_kafka_log_print;
    quant live #result: rd_kafka_conf_s.socket_cb |-> rd_kafka_socket_cb_linux;
    quant live #result: rd_kafka_conf_s.open_cb |-> rd_kafka_open_cb_linux;
    ensures normal resource_return_owned(nonnull);
}};
"""

CONSUME_CB = f"""// `rd_kafka_consume_cb(rk, rkq, rko, cb_type, opaque)` (`rdkafka.c:4932-4955`,
// static): an outdated or barrier op is destroyed (`HANDLED`); otherwise
// `rd_kafka_message_get(rko)`, `rd_kafka_fetch_op_app_prepare(rk, rko)`, then
// `ctx->consume_cb(rkmessage, ctx->opaque)` with `ctx = opaque`, a
// `struct consume_ctx` (`:4923-4926`), then `rd_kafka_op_destroy(rko)`;
// `HANDLED` on every return: the op is taken on every return.
extern fn rd_kafka_consume_cb(rk: ptr<mut, u8>, rkq: ptr<mut, u8>, rko: ptr<mut, u8>, cb_type: u32, opaque: ptr<mut, u8>) -> u32
contract {{
    requires resource_contract_v1;
    {PLUGIN}
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(rkq);
    ensures normal resource_consumed(rko, nonnull);
    ensures normal resource_borrowed(cb_type);
    ensures normal resource_borrowed(opaque);
    ensures normal resource_callback_member_invoked(opaque, consume_ctx.consume_cb, _, _);
    ensures normal resource_return_none();
}};
"""

POLL_MEMBERS = [
    ("rebalance_cb", "rk, _, _, _"),
    ("error_cb", "rk, _, _, _"),
    ("dr_msg_cb", "rk, _, _"),
    ("dr_cb", "rk, _, _, _, _, _"),
    ("throttle_cb", "rk, _, _, _, _"),
    ("stats_cb", "rk, _, _, _"),
    ("log_cb", "rk, _, _, _"),
    ("consume_cb", "_, _"),
]

BACKGROUND = f"""// `rd_kafka_background_queue_serve(rk, rkq, rko, cb_type, opaque)`
// (`rdkafka_background.c:67-103`, static): with `background_event_cb` set and
// an eventable op, `rd_kafka_call_background_event_cb(rk, rko)` hands the op
// to `rk->rk_conf.background_event_cb(rk, rko, rk->rk_conf.opaque)` ("Event
// must be destroyed by application") and returns `HANDLED`; otherwise
// `rd_kafka_poll_cb(rk, rkq, rko, RD_KAFKA_Q_CB_CALLBACK, opaque)`, returning
// its `HANDLED` or `YIELD`, and destroying the op on any other result: the op
// is taken on every return. `rd_kafka_poll_cb` invokes the application's
// rk_conf callbacks below (`rdkafka.c:5849-6001`, `consume_cb` through
// `rd_kafka_consume_cb`); its op-carried callbacks (`rko_u.offset_commit.cb`,
// `rko_u.terminated.cb`, `rko_op_cb` through `rd_kafka_op_call`) are not
// stated here: a member invocation reads a borrowed or released object, and
// the op is consumed.
extern fn rd_kafka_background_queue_serve(rk: ptr<mut, u8>, rkq: ptr<mut, u8>, rko: ptr<mut, u8>, cb_type: u32, opaque: ptr<mut, u8>) -> u32
contract {{
    requires resource_contract_v1;
    {PLUGIN}
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(rkq);
    ensures normal resource_consumed(rko, nonnull);
    ensures normal resource_borrowed(cb_type);
    ensures normal resource_borrowed(opaque);
    ensures normal resource_callback_member_invoked(rk, rd_kafka_s.rk_conf.background_event_cb, rk, rko, _);
""" + "".join(
    f"    ensures normal resource_callback_member_invoked(rk, rd_kafka_s.rk_conf.{field}, {actuals});\n"
    for field, actuals in POLL_MEMBERS
) + """    ensures normal resource_return_none();
};
"""

REFRESH_OP = f"""// `rd_kafka_oauthbearer_refresh_op(rk, rkq, rko)`
// (`rdkafka_sasl_oauthbearer.c:170-183`, static), an `rko_op_cb` candidate:
// unless `rko->rko_err == RD_KAFKA_RESP_ERR__DESTROY`, calls
// `rk->rk_conf.sasl.oauthbearer.token_refresh_cb(rk,
// rk->rk_conf.sasl.oauthbearer_config, rk->rk_conf.opaque)` when set;
// returns `HANDLED`: the op is only read (OP-CALLBACK-CANDIDATE-CONTRACTS'
// protocol: borrowed).
extern fn rd_kafka_oauthbearer_refresh_op(rk: ptr<mut, u8>, rkq: ptr<mut, u8>, rko: ptr<mut, u8>) -> u32
contract {{
    requires resource_contract_v1;
    {PLUGIN}
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(rkq);
    ensures normal resource_borrowed(rko);
    ensures normal resource_callback_member_invoked(rk, rd_kafka_s.rk_conf.sasl.oauthbearer.token_refresh_cb, rk, _, _);
    ensures normal resource_return_none();
}};
"""

LOG_PRINT = """// `rd_kafka_log_print(rk, level, fac, buf)` (`rdkafka.c:379-390`), `log_cb`'s
// default: `rd_gettimeofday`, one `fprintf(stderr, …)` reading `rk->rk_name`
// when `rk` is not NULL; keeps nothing.
extern fn rd_kafka_log_print(rk: ptr<const, u8>, level: i32, fac: ptr<const, u8>, buf: ptr<const, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(level);
    ensures normal resource_borrowed(fac);
    ensures normal resource_borrowed(buf);
    ensures normal resource_return_none();
};
"""

REFRESHERS = [
    ("rd_kafka_oauthbearer_unsecured_token", "rdkafka_sasl_oauthbearer.c:1010-1030",
     "registered by `rd_kafka_new` for an unsecured JWT (`rdkafka.c:2651-2655`): builds a token "
     "from `oauthbearer_config` into a local, `rd_kafka_oauthbearer_set_token` (copies) or "
     "`_set_token_failure`, then frees the local token"),
    ("rd_kafka_oidc_token_jwt_bearer_refresh_cb", "rdkafka_sasl_oauthbearer_oidc.c:778-917",
     "registered by `rd_kafka_new` for OIDC JWT bearer (`rdkafka.c:2689-2690`): builds and posts "
     "an assertion, validates the reply, sets the token or its failure, and frees every local "
     "(`sub`, the assertion, the body, the headers, the JSON, the extension arrays)"),
    ("rd_kafka_oidc_token_client_credentials_refresh_cb",
     "rdkafka_sasl_oauthbearer_oidc.c:918-1017",
     "registered by `rd_kafka_new` for OIDC client credentials (`rdkafka.c:2684-2686`): posts the "
     "credentials, validates the reply, sets the token or its failure, frees every local"),
    ("rd_kafka_oidc_token_metadata_azure_imds_refresh_cb",
     "rdkafka_sasl_oauthbearer_oidc.c:1018-1134",
     "registered by `rd_kafka_new` for Azure IMDS (`rdkafka.c:2672-2674`): fetches the token "
     "endpoint, validates the reply, sets the token or its failure, frees every local"),
    ("rd_kafka_oidc_token_metadata_aws_iam_refresh_cb",
     "rdkafka_sasl_oauthbearer_oidc.c:1135-1149",
     "registered by `rd_kafka_new` for AWS IAM metadata (`rdkafka.c:2678-2680`): sets the token "
     "failure only"),
]


def refresher(name, where, what):
    return f"""// `{name}(rk, oauthbearer_config, opaque)` (`{where}`), a
// `sasl.oauthbearer.token_refresh_cb` candidate {what}; keeps nothing it is
// handed.
extern fn {name}(rk: ptr<mut, u8>, oauthbearer_config: ptr<const, u8>, opaque: ptr<mut, u8>) -> unit
contract {{
    requires resource_contract_v1;
    {PLUGIN}
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(oauthbearer_config);
    ensures normal resource_borrowed(opaque);
    ensures normal resource_return_none();
}};
"""


def section():
    parts = [BEGIN, "\n", INTRO, "\n"]
    for row in SETTERS:
        parts.append(setter(*row))
        parts.append("\n")
    parts += [CONF_NEW, "\n", CONSUME_CB, "\n", BACKGROUND, "\n", REFRESH_OP, "\n", LOG_PRINT,
              "\n"]
    for row in REFRESHERS:
        parts.append(refresher(*row))
        parts.append("\n")
    parts.append(END + "\n")
    return "".join(parts)


PREMISE_LINE = re.compile(
    r"^    requires resource_premise\((rk_log_callback|rk_closesocket_callback), \"[^\"]*\"\);\n",
    re.M)


def restate(text):
    """Restate the sibling card's two premises as `rk_no_plugin_library`,
    once per declaration."""
    out = []
    for block in re.split(r"(?=^extern fn )", text, flags=re.M):
        if PREMISE_LINE.search(block):
            block = PREMISE_LINE.sub("", block)
            block = block.replace("    requires resource_contract_v1;\n",
                                  "    requires resource_contract_v1;\n    " + PLUGIN + "\n", 1)
        out.append(block)
    return "".join(out).replace(
        "// declaration states the dependency as the premise `rk_log_callback`, which\n"
        "// `OP-APP-RK-CALLBACKS-VIA-POLL-CB` discharges by contracting the logger.\n",
        "// declaration stated the dependency as the premise `rk_log_callback`;\n"
        "// `OP-APP-RK-CALLBACKS-VIA-POLL-CB` contracted the logger's run-closed\n"
        "// candidates and restated it as `rk_no_plugin_library` (and the mock\n"
        "// cluster's `rk_closesocket_callback` with it: `closesocket_cb` closes empty).\n",
    )


def write_client():
    text = CLIENT.read_text()
    if BEGIN in text:
        start = text.index(BEGIN)
        end = text.index(END, start) + len(END) + 1
        text = text[:start] + text[end:]
        text = text.rstrip("\n") + "\n"
    text = restate(text)
    at = text.index(AFTER) + len(AFTER)
    text = text[:at] + "\n" + section() + text[at:]
    CLIENT.write_text(text)


FLUENT_TEXT = f"""// # Fluent Bit's registered rk_conf callbacks
//
// Card OP-APP-RK-CALLBACKS-VIA-POLL-CB (gateway rule 12n), on the user's
// decisions of 2026-10-14 ("Closed over FB's registrations"; "Each candidate
// needs its own contract"; F1: declared-assumption standing for the three
// Fluent Bit candidates, each body read). The callbacks Fluent Bit registers
// through `rd_kafka_conf_set_*_cb`: `plugins/out_kafka/kafka_config.c:108`
// (`dr_msg_cb`), `:111` (`log_cb`) and `src/aws/flb_aws_msk_iam.c:775`
// (`sasl.oauthbearer.token_refresh_cb`). Each body was read at `66910c10` and
// `046accf6` (identical); no analysed route defines them, so each is a
// declared assumption (rule 13i). Each is pinned (C06.2).

// `cb_kafka_msg(rk, rkmessage, opaque)` (`plugins/out_kafka/kafka.c:63-78`):
// reads the opaque's plugin context and the message's error, length and
// partition, and logs through `flb_plg_warn` / `flb_plg_debug`; keeps nothing.
extern fn cb_kafka_msg(rk: ptr<mut, u8>, rkmessage: ptr<const, u8>, opaque: ptr<mut, u8>) -> unit
contract {{
    requires resource_contract_v1;
    {PLUGIN}
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(rkmessage);
    ensures normal resource_borrowed(opaque);
    ensures normal resource_return_none();
}};

// `cb_kafka_logger(rk, level, fac, buf)` (`plugins/out_kafka/kafka.c:80-104`):
// `rd_kafka_opaque(rk)` for the plugin context, `rd_kafka_name(rk)`, and one
// `flb_plg_*` line by level; keeps nothing.
extern fn cb_kafka_logger(rk: ptr<const, u8>, level: i32, fac: ptr<const, u8>, buf: ptr<const, u8>) -> unit
contract {{
    requires resource_contract_v1;
    {PLUGIN}
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(level);
    ensures normal resource_borrowed(fac);
    ensures normal resource_borrowed(buf);
    ensures normal resource_return_none();
}};

// `oauthbearer_token_refresh_cb(rk, oauthbearer_config, opaque)`
// (`src/aws/flb_aws_msk_iam.c:609-717`, static): validates the opaque's MSK
// IAM context, builds a payload (`build_msk_iam_payload`), creates a
// temporary credentials provider and reads credentials, sets the token
// (`rd_kafka_oauthbearer_set_token`, which copies) or its failure, then
// destroys the credentials, the provider and the payload on every path that
// made them; keeps nothing it is handed.
extern fn oauthbearer_token_refresh_cb(rk: ptr<mut, u8>, oauthbearer_config: ptr<const, u8>, opaque: ptr<mut, u8>) -> unit
contract {{
    requires resource_contract_v1;
    {PLUGIN}
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(oauthbearer_config);
    ensures normal resource_borrowed(opaque);
    ensures normal resource_return_none();
}};
"""


def main():
    write_client()
    text = FLUENT.read_text()
    if BEGIN in text:
        start = text.index(BEGIN)
        end = text.index(END, start) + len(END) + 1
        text = text[:start].rstrip("\n") + "\n" + text[end:]
    FLUENT.write_text(text.rstrip("\n") + "\n\n" + BEGIN + "\n\n" + FLUENT_TEXT + "\n" + END + "\n")
    print("wrote", CLIENT.relative_to(REPO), "and", FLUENT.relative_to(REPO))


if __name__ == "__main__":
    main()
