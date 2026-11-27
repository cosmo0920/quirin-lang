#!/usr/bin/env python3
"""write_contracts.py -- OP-CONSTRUCTOR-DECLARATIONS: write the card's section
of `qcore-provider-librdkafka/contracts/librdkafka/client.qv` (between its
markers, replacing a previous run's, placed after the CG22 op section): the
op constructors `rd_kafka_op_new0`, `rd_kafka_op_new_cb` and the request
family `rd_kafka_op_req0`, `rd_kafka_op_req`, `rd_kafka_op_req2`, every body
read by hand at `66910c10` and `046accf6` (`scripts/body.py`, byte-identical).
The maps-to token is read from the lexer's `MAPS_TO` constant
(`crates/qcore-frontend/src/lexer.rs`), so the spelling is one edit.
Idempotent."""
import pathlib
import re

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
MODULE = REPO / "providers/qcore-provider-librdkafka/contracts/librdkafka/client.qv"
LEXER = REPO / "crates/qcore-frontend/src/lexer.rs"
BEGIN = "// BEGIN OP-CONSTRUCTOR-DECLARATIONS (scripts/write_contracts.py)"
END = "// END OP-CONSTRUCTOR-DECLARATIONS"
AFTER = "// END CG22-OP-CALLBACKS-AND-SERVED-QUEUES\n"

MAPS_TO = re.search(r'pub const MAPS_TO: &str = "([^"]+)";', LEXER.read_text()).group(1)
FLAGMASK_CB = 1 << 29  # RD_KAFKA_OP_CB, rdkafka_op.h:227

INTRO = """// # Op constructors and requests
//
// Card OP-CONSTRUCTOR-DECLARATIONS (gateway rules 12o, 13w, 14e, contract
// specification C04.19.4), on the user's decisions of 2026-10-14: the
// result's discriminant `resource_discriminant_set(result, TAG.FIELD, P,
// FLAGS)` (the actual of `P` OR-ed with `FLAGS`, exact; a release selects the
// arm of that value under its own mask), a callback registered in the result's
// member `resource_callback_member_registered(result, TAG.FIELD, F)`, and the
// `quant` state clause `quant live #result: TAG.FIELD {maps} absent;` (the
// result is live and its member holds no callback). Every body was read at
// `66910c10` and `046accf6` (byte-identical, `scripts/body.py` of the card's
// report); each declaration is pinned (C06.2). `rd_kafka_op_type_t` is an
// enumeration with no negative value (`rdkafka_op.h:84-224`, with the flags
// `RD_KAFKA_OP_CB = 1 << 29`, `RD_KAFKA_OP_REPLY = 1 << 30`): `u32`.
//
// A stated state is read at a later call only while every store the run
// makes into the member is classified (rules 12o, 13w, 14e: a store into a
// constructor's own result or an automatic object, a null callback, a flag-or
// inside the mask, a case-guarded rewrite such as `rd_kafka_broker_op_serve`'s
// `PARTITION_LEAVE` -> `PARTITION_JOIN`, both in the destroyer's `otherwise`
// arm); anything else withdraws it, and the destroyer reads the run closure
// and the cautious join as before.""".format(maps=MAPS_TO)

PREMISE = (
    'requires resource_premise(asserted_allocation, "the result is non-null because it is '
    'allocated with rd_calloc (rd.h:139-143), which asserts its result, and the audited compile '
    'commands define no NDEBUG, so it never returns NULL in the audited build");'
)

MEMBERS = [
    "ensures normal resource_callback_member_invoked(supplied_owned, rd_kafka_op_s.rko_serve, _, _, supplied_owned, _, _);",
    "ensures normal resource_callback_member_invoked(supplied_owned, rd_kafka_op_s.rko_op_cb, _, _, _);",
]


def block(comment, signature, clauses):
    body = "\n".join("    " + clause for clause in ["requires resource_contract_v1;", *clauses])
    return f"{comment}\n{signature}\ncontract {{\n{body}\n}};"


def blocks():
    out = [INTRO]
    out.append(block(
        """// `rd_kafka_op_new0(source, type)` (`rdkafka_op.c:224-342`; the macro
// `rd_kafka_op_new(type)` is `rd_kafka_op_new0(NULL, type)`, `rdkafka_op.h:932`,
// `ENABLE_DEVEL` 0 in the audited build): the payload size
// `op2size[type & ~RD_KAFKA_OP_FLAGMASK]`, asserted non-zero; `rko =
// rd_calloc(1, ...)` (zeroed: every member, `rko_op_cb` and `rko_serve`
// included, is NULL); `rko->rko_type = type`, the full actual; `return rko`.
// `source` is stored only under `ENABLE_DEVEL`. Rule 14e checks the state
// against this body where it is defined.""",
        "extern fn rd_kafka_op_new0(source: ptr<const, u8>, type: u32) -> ptr<mut, u8>",
        [
            PREMISE,
            "ensures normal resource_borrowed(source);",
            "ensures normal resource_borrowed(type);",
            "ensures normal resource_discriminant_set(result, rd_kafka_op_s.rko_type, type, 0);",
            f"quant live #result: rd_kafka_op_s.rko_op_cb {MAPS_TO} absent;",
            "ensures normal resource_return_owned(nonnull);",
        ],
    ))
    out.append(block(
        f"""// `rd_kafka_op_new_cb(rk, type, cb)` (`rdkafka_op.c:785-793`):
// `rko = rd_kafka_op_new(type | RD_KAFKA_OP_CB)` (`RD_KAFKA_OP_CB = 1 << 29 =
// {FLAGMASK_CB}`, `rdkafka_op.h:227`), `rko->rko_op_cb = cb`, `rko->rko_rk = rk`
// (a plain pointer the op keeps, no reference taken: retained by the result,
// C04.14.2), `return rko`. `cb` is `rd_kafka_op_cb_t *`, a pointer to a
// function-type typedef (`rdkafka_op.h:302-304`).""",
        "extern fn rd_kafka_op_new_cb(rk: ptr<mut, u8>, type: u32, cb: fnptr) -> ptr<mut, u8>",
        [
            PREMISE,
            "ensures normal resource_borrowed(rk);",
            "ensures normal resource_borrowed(type);",
            "ensures normal resource_borrowed(cb);",
            "ensures normal resource_retained_by(rk, result);",
            f"ensures normal resource_discriminant_set(result, rd_kafka_op_s.rko_type, type, {FLAGMASK_CB});",
            "ensures normal resource_callback_member_registered(result, rd_kafka_op_s.rko_op_cb, cb);",
            "ensures normal resource_return_owned(nonnull);",
        ],
    ))
    request = """// The request family (`rdkafka_op.c:850-897`). `rd_kafka_op_req0(destq,
// recvq, rko, timeout_ms)`: `rd_kafka_op_set_replyq(rko, recvq, NULL)` (the op
// takes its own reference on `recvq`, `rd_kafka_q_keep`; the caller's is
// unchanged), `rd_kafka_q_enq(destq, rko)` (the op is the queue's; on a
// disabled queue `rd_kafka_op_reply` re-queues it to `recvq` or destroys it:
// the caller never holds it again), `NULL` when the enqueue fails, otherwise
// `rd_kafka_q_pop(recvq, ..., 0)`: an op or `NULL` on timeout. The pop serves
// `recvq` as `rd_kafka_q_pop_serve0` does with a null callback and
// `RD_KAFKA_Q_CB_RETURN`, so each dequeued op's `rko_serve` and `rko_op_cb` may
// run (the two member clauses, as the pop variants above state them). The
// returned op's type is the replier's (the request re-used with
// `RD_KAFKA_OP_REPLY`, `rd_kafka_op_new_reply`'s copy, or any op queued on
// `recvq`): no discriminant is stated. `rd_kafka_op_req(destq, rko,
// timeout_ms)` is `req0` over a temporary `rd_kafka_q_new(destq->rkq_rk)`
// dropped with `rd_kafka_q_destroy_owner`; `rd_kafka_op_req2(destq, type)` is
// `req` over `rd_kafka_op_new(type)` with `RD_POLL_INFINITE`."""
    out.append(block(
        request + "\n// `rd_kafka_op_req0` (`rdkafka_op.c:850-866`).",
        "extern fn rd_kafka_op_req0(destq: ptr<mut, u8>, recvq: ptr<mut, u8>, rko: ptr<mut, u8>, timeout_ms: i32) -> ptr<mut, u8>",
        [
            "ensures normal resource_borrowed(destq);",
            "ensures normal resource_borrowed(recvq);",
            "ensures normal resource_consumed(rko, nonnull);",
            "ensures normal resource_borrowed(timeout_ms);",
            *MEMBERS,
            "ensures normal resource_return_owned(nullable);",
        ],
    ))
    out.append(block(
        "// `rd_kafka_op_req` (`rdkafka_op.c:874-886`).",
        "extern fn rd_kafka_op_req(destq: ptr<mut, u8>, rko: ptr<mut, u8>, timeout_ms: i32) -> ptr<mut, u8>",
        [
            "ensures normal resource_borrowed(destq);",
            "ensures normal resource_consumed(rko, nonnull);",
            "ensures normal resource_borrowed(timeout_ms);",
            *MEMBERS,
            "ensures normal resource_return_owned(nullable);",
        ],
    ))
    out.append(block(
        "// `rd_kafka_op_req2` (`rdkafka_op.c:892-897`).",
        "extern fn rd_kafka_op_req2(destq: ptr<mut, u8>, type: u32) -> ptr<mut, u8>",
        [
            "ensures normal resource_borrowed(destq);",
            "ensures normal resource_borrowed(type);",
            *MEMBERS,
            "ensures normal resource_return_owned(nullable);",
        ],
    ))
    return out


def main():
    text = MODULE.read_text()
    section = BEGIN + "\n\n" + "\n\n".join(blocks()) + "\n\n" + END + "\n"
    if BEGIN in text:
        start = text.index(BEGIN)
        end = text.index(END, start) + len(END) + 1
        text = text[:start] + section + text[end:]
    else:
        at = text.index(AFTER) + len(AFTER)
        text = text[:at] + "\n" + section + text[at:]
    MODULE.write_text(text)
    print(f"wrote {MODULE.relative_to(REPO)}")


if __name__ == "__main__":
    main()
