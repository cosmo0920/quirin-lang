#!/usr/bin/env python3
"""write_contracts.py -- CG22-OP-CALLBACKS-AND-SERVED-QUEUES: write the card's
section of `qcore-provider-librdkafka/contracts/librdkafka/client.qv` (between
its markers, replacing a previous run's): the served-queue declarations, by
hand below, and `rd_kafka_op_destroy`'s arms from `artifacts/op-destroy-arms.json`
(`scripts/op_destroy_arms.py`, Clang's reading of the body), each value with
its enumerator name and `rdkafka_op.h` line in a comment. Idempotent."""
import json
import pathlib
import re

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
MODULE = REPO / "providers/qcore-provider-librdkafka/contracts/librdkafka/client.qv"
ARMS = HERE.parent / "artifacts/op-destroy-arms.json"
HEADER = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source/lib/librdkafka-2.15.0/src/rdkafka_op.h")
BEGIN = "// BEGIN CG22-OP-CALLBACKS-AND-SERVED-QUEUES (scripts/write_contracts.py)"
END = "// END CG22-OP-CALLBACKS-AND-SERVED-QUEUES"

INTRO = """// # Served queues and the op destroyer
//
// Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES (gateway rules 12m-12o, contract
// specification C04.19.1-C04.19.3), on the user's decision of 2026-10-14:
// `supplied_owned` for an object the callee hands its callback, a member
// invocation `TAG.FIELD` whose candidates are the run's closed set of stores
// into the member (premise P-CG22-MEMBER-CLOSED-WORLD), and a type-switched
// release, one `when FIELD == VALUE` / `otherwise` clause per arm. Every body
// was read at `66910c10` and `046accf6` (byte-identical, `scripts/body.py` of
// the card's report); each declaration is pinned (C06.2).
//
// The queue serving family (`rdkafka_queue.c:398-680`): under `rkq->rkq_lock`
// a forward queue is kept (`rd_kafka_q_fwd_get`), served recursively and
// dropped (`rd_kafka_q_destroy`); otherwise each dequeued op goes to
// `rd_kafka_op_handle` (`rdkafka_op.c:1096-1124`), which runs the op's own
// `rko_op_cb` when it is a callback op (`rd_kafka_op_handle_std` ->
// `rd_kafka_op_call`), replaces the caller's `callback` and `opaque` with the
// op's `rko_serve` / `rko_serve_opaque` when set, destroys a handled op, and
// otherwise hands the op to `callback(rk, rkq, rko, cb_type, opaque)`, which
// takes it (`rd_kafka_q_serve0` asserts the result is not `PASS`; the pop
// variants hand a `PASS` op back as their result). `rkq`, `opaque` and the
// other arguments are only read: `resource_borrowed`. `cb_type` is
// `rd_kafka_q_cb_type_t` (`rdkafka_op.h:270-277`, no negative value: `u32`),
// `rd_ts_t` is `int64_t` (`rdtypes.h:44`), `rd_bool_t` is `uint8_t`
// (`rdtypes.h:49`). The op's own `rko_op_cb` borrows the op (`_`: a handled op
// is destroyed by the callee, a kept one is the callback's, which rule 13u
// refuses as effectful); `rko_serve` takes it as `callback` does.
//
// These declarations decide nothing in the corpus today: every callback the
// corpus passes (`rd_kafka_poll_cb`, `rd_kafka_consume_cb`,
// `rd_kafka_background_queue_serve`) calls a callback the application
// registered in `rk` (the pending `rd_kafka_destroy_app` decision) and has no
// contract, and the op members' closed sets hold candidates with no contract
// (`OP-CALLBACK-CANDIDATE-CONTRACTS`). They replace an uncontracted call with
// a named refusal. The served-queue premise on `rd_kafka_q_wait_result` and
// `rd_kafka_q_purge0` (C04.18, above) is unchanged.
"""


def serve(name, parameters, result, line, pop):
    params = ", ".join(f"{n}: {t}" for n, t in parameters)
    lines = [f"// `{name}` ({line}).",
             f"extern fn {name}({params}) -> {result}",
             "contract {",
             "    requires resource_contract_v1;"]
    for n, _ in parameters:
        lines.append(f"    ensures normal resource_borrowed({n});")
    lines.append("    ensures normal resource_callback_invoked(callback, _, _, supplied_owned, cb_type, opaque);")
    lines.append("    ensures normal resource_callback_member_invoked(supplied_owned, rd_kafka_op_s.rko_serve, _, _, supplied_owned, cb_type, _);")
    lines.append("    ensures normal resource_callback_member_invoked(supplied_owned, rd_kafka_op_s.rko_op_cb, _, _, _);")
    lines.append("    ensures normal resource_return_owned(nullable);" if pop else "    ensures normal resource_return_none();")
    lines.append("};")
    return "\n".join(lines)


def enumerator_lines():
    lines = {}
    for number, text in enumerate(HEADER.read_text(errors="replace").splitlines(), 1):
        match = re.match(r"\s*(RD_KAFKA_OP_\w+)\b", text)
        if match and match.group(1) not in lines:
            lines[match.group(1)] = number
    return lines


def destroy():
    data = json.loads(ARMS.read_text())
    groups, names = data["groups"], data["names"]
    where = enumerator_lines()
    lines = ["""// `rd_kafka_op_destroy(rko)` (`rdkafka_op.c:345-628`): when the op is a
// callback op with `rko_op_cb`, sets `rko_err = RD_KAFKA_RESP_ERR__DESTROY` and
// calls `rko_op_cb(rko->rko_rk, NULL, rko)` (asserting neither `YIELD` nor
// `KEEP`: the callback borrows the op); then `switch (rko->rko_type &
// ~RD_KAFKA_OP_FLAGMASK)` (`RD_KAFKA_OP_FLAGMASK = RD_KAFKA_OP_CB |
// RD_KAFKA_OP_REPLY = (1 << 29) | (1 << 30) = 1610612736`,
// `rdkafka_op.h:227-229`) releases the members of the op's union arm the case
// group hands to a call (`rd_free`, the list and partition-list destroyers,
// reference drops, `rd_kafka_msgq_purge`; `RD_IF_FREE` and `RD_LIST_FOREACH`
// expanded); `default: break;` releases none; then the op's `rko_rktp`,
// `rko_error` and reply queue, and `rd_free(rko)`. The arms below are exactly
// the payload paths under `rko_u` each value's case group hands to a call, as
// rule 12o's check reads them (`scripts/op_destroy_arms.py`); that each call
// releases its member stays a declared assumption. `rko` is read first
// (`rko->rko_type`): `resource_released(rko, nonnull)`.
extern fn rd_kafka_op_destroy(rko: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    requires resource_discriminant(rko, rko_type, 1610612736);
    ensures normal resource_released(rko, nonnull);
    ensures normal resource_callback_member_invoked(rko, rd_kafka_op_s.rko_op_cb, _, _, rko);"""]
    for value in sorted((v for v in groups if v != "default"), key=int):
        name = names.get(value, "?")
        lines.append(f"    // {name} = {value} (rdkafka_op.h:{where.get(name, '?')})")
        paths = groups[value]
        if paths:
            for path in paths:
                lines.append(f"    ensures normal resource_arm_released(rko, {path}) when rko_type == {value};")
        else:
            lines.append(f"    ensures normal resource_arm_empty(rko) when rko_type == {value};")
    default = groups.get("default", [])
    lines.append("    // `default: break;`")
    if default:
        for path in default:
            lines.append(f"    ensures normal resource_arm_released(rko, {path}) otherwise;")
    else:
        lines.append("    ensures normal resource_arm_empty(rko) otherwise;")
    lines.append("    ensures normal resource_return_none();")
    lines.append("};")
    return "\n".join(lines)


def main():
    q = ("rkq", "ptr<mut, u8>")
    cb = [("cb_type", "u32"), ("callback", "fnptr"), ("opaque", "ptr<mut, u8>")]
    blocks = [
        INTRO,
        serve("rd_kafka_q_serve0", [q, ("timeout_ms", "i32"), ("max_cnt", "i32"), *cb, ("is_consume_call", "u8")], "i32", "`rdkafka_queue.c:558-649`", False),
        serve("rd_kafka_q_serve", [q, ("timeout_ms", "i32"), ("max_cnt", "i32"), *cb], "i32", "`rdkafka_queue.c:652-660`, `rd_kafka_q_serve0(…, rd_false)`", False),
        serve("rd_kafka_q_serve_maybe_consume", [q, ("timeout_ms", "i32"), ("max_cnt", "i32"), *cb], "i32", "`rdkafka_queue.c:667-677`", False),
        serve("rd_kafka_q_pop_serve0", [q, ("timeout_us", "i64"), ("version", "i32"), *cb, ("is_consume_call", "u8")], "ptr<mut, u8>", "`rdkafka_queue.c:398-505`, static", True),
        serve("rd_kafka_q_pop_serve", [q, ("timeout_us", "i64"), ("version", "i32"), *cb], "ptr<mut, u8>", "`rdkafka_queue.c:507-515`", True),
        serve("rd_kafka_q_pop_serve_maybe_consume", [q, ("timeout_us", "i64"), ("version", "i32"), *cb], "ptr<mut, u8>", "`rdkafka_queue.c:522-533`", True),
        destroy(),
    ]
    section = BEGIN + "\n\n" + "\n\n".join(blocks) + "\n\n" + END + "\n"
    text = MODULE.read_text()
    if BEGIN in text:
        start = text.index(BEGIN)
        end = text.index(END, start) + len(END) + 1
        text = text[:start] + section + text[end:]
    else:
        text = text.rstrip("\n") + "\n\n" + section
    MODULE.write_text(text)
    print(section.count("extern fn"), "declarations,", section.count("resource_arm_"), "arm clauses")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
