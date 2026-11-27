#!/usr/bin/env python3
"""edit_status.py -- MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS: the card's
STATUS rows (projected functions, contract dependencies, follow-ups) from the
measurement in artifacts/. Idempotent."""
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
STATUS = REPO / "docs/STATUS.md"
MARK = "MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS"
REPORT = "reports/member-path-unit-helpers/README.md"

EDITS = [
    (
        "| Projected functions | **936 Safe / 9 provisional Unsafe / 3,889 Unknown**, 4,834 total (",
        "| Projected functions | **936 Safe / 9 provisional Unsafe / 3,889 Unknown**, 4,834 total "
        f"({MARK}, rule 30's body check through the unit's static helpers: arbiter on main "
        "`5b1ceecd5` `71387bd3…` → `a2e95319…`, **no row moves** (statuses, leaves and every row's "
        "dependencies identical; only two units' body evidence differs); system and slice reports "
        f"byte-identical; null rows identical ([report]({REPORT})); before it: "
        "**936 Safe / 9 provisional Unsafe / 3,889 Unknown**, 4,834 total (",
    ),
    (
        "| Contract dependencies per verdict (rules 13f–13k) | ",
        "| Contract dependencies per verdict (rules 13f–13k) | "
        f"Since {MARK} (corpus binary `fd6e0ee2…`, base `5b1ceecd5`; no verdict or row moved): 428 "
        "declarations consulted, partly validated **45 → 46**, declared assumptions 319 → 318 "
        "(`rd_kafka_topic_partition_list_query_leaders_async_worker_op_cb` and "
        "`rd_kafka_DeleteRecords_leaders_queried_cb` partly validated through rule 30's path "
        "reading; `rd_kafka_admin_fanout_worker` corrected to a declared assumption, its formal "
        "passed to an indirect call after the store); the Safe roll-up unchanged "
        f"([standing](reports/member-path-unit-helpers/artifacts/standing.json)). ",
    ),
    (
        "| Registered follow-ups | **784** (wave 48: ",
        f"| Registered follow-ups | **784** (wave 48: {MARK} retires itself and registers "
        "`MEMBER-PATH-STATES-THROUGH-INDIRECT-AND-EXTERNAL-CALLS`, "
        f"[report]({REPORT}); before it **784**, wave 48: ",
    ),
]


def main():
    text = STATUS.read_text()
    for old, new in EDITS:
        if new in text:
            continue
        assert text.count(old) == 1, old
        text = text.replace(old, new)
    STATUS.write_text(text)
    print("ok")


main()
