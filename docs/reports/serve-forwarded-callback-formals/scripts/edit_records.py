#!/usr/bin/env python3
"""edit_records.py -- SERVE-FORWARDED-CALLBACK-FORMALS: the STATUS rows and
the increment paragraph, from the measurement in artifacts/summary.json.
Idempotent (each edit is skipped when its text is present)."""
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
STATUS = REPO / "docs/STATUS.md"
PART = REPO / "docs/development/increments/wave-48-part7.md"
REPORT = "reports/serve-forwarded-callback-formals/README.md"


def edit(path, old, new, marker):
    text = path.read_text(encoding="utf-8")
    if marker in text:
        return
    assert text.count(old) == 1, (path, old[:60])
    path.write_text(text.replace(old, new), encoding="utf-8")


def main():
    projected = "| Projected functions | **936 Safe / 9 provisional Unsafe / 3,889 Unknown**, 4,834 total ("
    edit(
        STATUS,
        projected,
        projected
        + "SERVE-FORWARDED-CALLBACK-FORMALS, rule 15w: a callback formal handed on unchanged to the "
        "same clause position of a callee in force satisfies the caller's own invocation clause; "
        "arbiter on main `2afce9973` plus the card `ec4ccfeb…` byte-identical, **no status move**, "
        "system, slice and null property byte-identical; the four served-queue forwarders are exact "
        "forwards but their callees `rd_kafka_q_serve0` / `rd_kafka_q_pop_serve0` stay withdrawn in "
        f"`rdkafka_queue.c` ([report]({REPORT})); before it: **936 Safe / 9 provisional Unsafe / "
        "3,889 Unknown**, 4,834 total (",
        "SERVE-FORWARDED-CALLBACK-FORMALS, rule 15w",
    )
    callback = "the served-queue premise stays | [Report](reports/cg22-callback-registry-contract/README.md) |"
    edit(
        STATUS,
        callback,
        "the served-queue premise stays; rule 15w (SERVE-FORWARDED-CALLBACK-FORMALS) lets an "
        "invocation stand on an exact forward to a callee in force, without `13y` evidence (corpus: "
        "none stands, the serve family's callees being withdrawn) | "
        "[Report](reports/cg22-callback-registry-contract/README.md), "
        f"[15w]({REPORT}) |",
        "rule 15w (SERVE-FORWARDED-CALLBACK-FORMALS)",
    )
    followups = "| Registered follow-ups | **774** ("
    edit(
        STATUS,
        followups,
        followups
        + "wave 48: SERVE-FORWARDED-CALLBACK-FORMALS retires `SERVE-FORWARDED-CALLBACK-FORMALS` and "
        "registers `SERVE0-CALLBACK-HAND-ON-WITHDRAWN`: 774 → 774). Earlier: **774** (",
        "SERVE-FORWARDED-CALLBACK-FORMALS retires",
    )
    anchor = "**`OP-CALLBACK-CANDIDATE-CONTRACTS` (2026-10-14,"
    paragraph = (
        "**`SERVE-FORWARDED-CALLBACK-FORMALS` (2026-10-14, base `2afce9973`, rebased onto `89907068d`; prediction committed "
        "before implementation).** A callback formal handed on unchanged to the same clause position "
        "of a callee whose declaration is in force now satisfies the caller's own "
        "`resource_callback_invoked` clause ([report](../../reports/serve-forwarded-callback-formals/README.md), "
        "gateway rule [15w](../../agents/gateway.md#constraint-15w), rules 13u and 13y amended in "
        "place, Lean `QCore.CallbackForward`). A forward is exact when every mention of the formal is "
        "the whole actual of a direct call, the callee names that position in exactly one invocation "
        "clause (borrowed, never retained or registered) and its actuals correspond position by "
        "position through the call; the body check then stands without `13y` evidence (a declared "
        "assumption resting on the callee), the standing set is the least fixed point (never on itself "
        "or a withdrawn callee), and at the forwarding call the clause is the caller's declared "
        "invocation, decided at its own callers. Any deviation (reassignment, store, cast, a local "
        "invocation beside a forward, another position, an uncontracted or withdrawn callee) keeps "
        "the withdrawal; controls in `tests/serve_forwarded_callback_formals.rs`. Corpus **936 / 9 / "
        "3,889** (`ec4ccfeb…`), system, slice and the null property byte-identical, as predicted: the "
        "four served-queue forwarders are exact forwards, but `rd_kafka_q_serve0` and "
        "`rd_kafka_q_pop_serve0` stay withdrawn in `rdkafka_queue.c` (each hands `callback` to the "
        "undeclared `rd_kafka_op_handle` and to itself), so no forwarder's callee is in force. Lean "
        "audits 319 → 320. Ledger 774 → 774: the entry retired, "
        "`SERVE0-CALLBACK-HAND-ON-WITHDRAWN` registered.\n\n"
    )
    edit(PART, anchor, paragraph + anchor, "**`SERVE-FORWARDED-CALLBACK-FORMALS`")


if __name__ == "__main__":
    main()
