#!/usr/bin/env python3
"""edit_records.py -- LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED: the STATUS rows
(projected functions, contract dependencies, registered follow-ups) and the
increment paragraph, each prepended to its current text from the measured
runs (artifacts/summary.json, artifacts/rows-after.json). Idempotent."""
import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parents[4]
STATUS = REPO / "docs/STATUS.md"
INCREMENT = REPO / "docs/development/increments/wave-48-part8.md"
REPORT = "reports/librdkafka-plugin-premise/README.md"
CARD = "LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED"


def prepend_row(text, label, addition):
    pattern = re.compile(r"^(\| " + re.escape(label) + r" \| )", re.M)
    match = pattern.search(text)
    assert match, label
    if addition in text[match.start(): text.index("\n", match.start())]:
        return text
    return text[: match.end()] + addition + text[match.end():]


def main():
    text = STATUS.read_text()
    text = prepend_row(
        text, "Projected functions",
        f"**936 Safe / 9 provisional Unsafe / 3,889 Unknown**, 4,834 total ({CARD}, contracts only: "
        "the premise `rk_no_plugin_library` named on the 34 `client.qv` declarations whose bodies may "
        "run an rk_conf callback: arbiter on base `d708641bc` `71387bd3…` → `82214412…`, **no status "
        "move**, every row's status, leaves and `contract_dependencies` field identical; system and "
        "slice identical apart from the contract-source digests; null rows identical "
        f"([report]({REPORT})); before it: ")
    text = prepend_row(
        text, "Contract dependencies per verdict (rules 13f–13k)",
        f"Since {CARD} (corpus binary `f79d9846…`): **all 368 corpus rows resting on the 34 "
        "premised `client.qv` declarations list `rk_no_plugin_library`** (17 Safe, 351 Unknown; 16 "
        "Safe and 317 Unknown gained it), roll-up unchanged (936 Safe: 397 on no declaration, 108 only "
        "on validated declarations, 25 on a partly-validated one, 382 on a declared assumption, 24 "
        "only on a null premise); the 86 null views cannot carry a premise (a decision for the user) "
        f"([report]({REPORT})). ")
    text = prepend_row(
        text, "Registered follow-ups",
        f"**784** (wave 48: {CARD} amends `{CARD}` (residual: the null views and "
        "`queue_destructor.qv`, both decisions for the user) and registers "
        "`LIBRDKAFKA-CRASH-DUMP-PREMISE-UNSTATED`); before it ")
    # The old count of the row now follows "before it ".
    STATUS.write_text(text)

    inc = INCREMENT.read_text()
    if CARD not in inc:
        paragraph = (
            f"**`{CARD}` (2026-10-15, base `d708641bc`, rebased onto `5b1ceecd5`; census and prediction "
            "`9197085f1`, committed before any contract changed).** Under the user's decision U2 "
            "(\"List it in every dependent verdict\") the plugin premise `P-RK-NO-PLUGIN-LIBRARY` "
            "(`rk_no_plugin_library`) is now named on every `client.qv` declaration whose body may run an "
            f"rk_conf callback ([report](../../{REPORT})). A census over librdkafka's 94 preprocessed "
            "units at both revisions (identical) walked an over-approximating call graph from the 80 "
            "declarations that lacked it: 34 reach `rd_kafka_log_buf`'s `conf->log_cb` or an "
            "interceptor method and gain the clause (re-pinned, `EXPECTED_CONTRACTS` +1 each); 39 "
            "`client.qv`, 3 `refcount.qv`, 2 `typed_queue.qv` and 1 `topic_partition.qv` declarations "
            "reach none. Verdicts are identical on all three routes and so are the null rows; all 368 "
            "corpus rows resting on the 34 list the premise (16 Safe and 317 Unknown gained it). Not "
            "done, each a decision for the user: the 86 null views (bounds-contract-v1 cannot carry a "
            "premise; no syntax invented) and `queue_destructor.qv`'s model declaration (outside the "
            "resource view). Ledger 783 → 784: the entry is amended, and "
            "`LIBRDKAFKA-CRASH-DUMP-PREMISE-UNSTATED` is registered for `rd_kafka_crash` and "
            "`rd_kafka_dump0`.\n\n")
        anchor = "**`LEAN-4-35-TOOLCHAIN`"
        index = inc.index(anchor)
        inc = inc[:index] + paragraph + inc[index:]
        INCREMENT.write_text(inc)
    print("ok")


main()
