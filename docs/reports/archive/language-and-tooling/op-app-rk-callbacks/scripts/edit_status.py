#!/usr/bin/env python3
"""edit_status.py -- OP-APP-RK-CALLBACKS-VIA-POLL-CB: prepend the card's
figures to three docs/STATUS.md rows (projected functions, contract corpus,
registered follow-ups), each from the card's primary evidence
(artifacts/summary.json, the loader pin, the ledger). Idempotent."""
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
STATUS = REPO / "docs/STATUS.md"
MARK = "OP-APP-RK-CALLBACKS-VIA-POLL-CB"

ROWS = {
    "| Projected functions | ": (
        "**936 Safe / 9 provisional Unsafe / 3,889 Unknown**, 4,834 total "
        "(OP-APP-RK-CALLBACKS-VIA-POLL-CB, the application's rk_conf callbacks closed over the "
        "run's registrations, rule 12n amended: arbiter on main `b7facc899` `0a38c3b3…` → "
        "`620fbd75…`, **no status move**, 97 Unknown rows trade leaves (`rd_kafka_conf_new`'s 13 "
        "uncontracted leaves and one `rd_kafka_consume_cb` call decide; `rd_kafka_op_destroy`'s "
        "member reads open over the run, 118 → 131 `callback-member-open`), no leak, violation or "
        "Unsafe added; the member-facts pre-pass reads 845 of 1,444 production units, none "
        "unread, corpus wall 515 → 668 s; system and slice no row moves; null verdicts identical "
        "([report](reports/op-app-rk-callbacks/README.md)); before it: "
    ),
    "| Contract corpus | ": (
        "**455** resource-v1 declarations by the loader pin since OP-APP-RK-CALLBACKS-VIA-POLL-CB "
        "(in `librdkafka/client.qv` the seventeen rk_conf callback setters, `rd_kafka_conf_new` "
        "with its stated function defaults, `rd_kafka_consume_cb`, "
        "`rd_kafka_background_queue_serve` and `rd_kafka_oauthbearer_refresh_op`, withheld with "
        "the retaining declarations, 57 → 78, and `rd_kafka_log_print` and five OAUTHBEARER "
        "refreshers; in `fluent_bit/logging.qv` Fluent Bit's three registered callbacks; the "
        "nineteen `rk_log_callback` premises restated as `rk_no_plugin_library`; pinned at "
        "`66910c10` and `046accf6`; selection 61 and rejected 314 unchanged; "
        "[report](reports/op-app-rk-callbacks/README.md)). Before it: "
    ),
    "| Registered follow-ups | ": (
        "**776** (wave 48: OP-APP-RK-CALLBACKS-VIA-POLL-CB retires "
        "`CALLBACK-MEMBER-EMBEDDED-RECORD-KEYS` and `LIBRDKAFKA-LOG-CALLBACK-PREMISE-UNSTATED`, "
        "amends its own entry to `rd_kafka_poll_cb`, `OP-CALLBACK-CANDIDATES-APP-CALLBACKS` to "
        "`rd_kafka_toppar_op_serve` and `W48-CALLEE-DECISION-TABLE` (destroy_app), and registers "
        "`LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED`, `MEMBER-PREPASS-OTHER-ROUTES`, "
        "`MEMBER-PREPASS-WALL-TIME` and `RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION`: 774 → 776). "
        "Earlier: "
    ),
}


def main():
    lines = STATUS.read_text().split("\n")
    for prefix, text in ROWS.items():
        for index, line in enumerate(lines):
            if line.startswith(prefix):
                if MARK not in line[: len(prefix) + 400]:
                    lines[index] = prefix + text + line[len(prefix):]
                break
        else:
            raise SystemExit(f"no row {prefix!r}")
    STATUS.write_text("\n".join(lines))
    print("status rows updated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
