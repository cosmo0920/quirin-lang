#!/usr/bin/env python3
"""update_ledger.py -- CALLBACK-MEMBER-DATA-ONLY-UNITS: retire the card's own
entry (its DISCHARGE, "member facts collected from a unit the gateway does not
analyse (its initializers and stores), so the run's closure is exactly the
recorded one where every unit is read; measured: both members closed", holds:
section 3 of the report) and register the follow-on found by reading.
Acceptance matrix, indent=1, ensure_ascii=False; `dimensions.named_followups`
= list length. Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "CALLBACK-MEMBER-DATA-ONLY-UNITS"
SOURCE = "docs/reports/callback-member-data-only-units/README.md"
RUN = ("corpus 66910c10 527a4cad 935/9/3,890 -> 527a4cad 935/9/3,890 (byte-identical), "
       "binary d550f4f6, system 93/4/351 and slice 52/1/347/4 Pending byte-identical, "
       "null property byte-identical; base main f3e1d724c")

NEW = [
    {
        "id": "CALLBACK-MEMBER-EMBEDDED-RECORD-KEYS",
        "title": "A store through an embedded record is keyed by the outermost record, so the inner member's closure does not see it",
        "why": (
            f"Registered by {CARD} ({RUN}). Rule 12n's facts key a store by the record the "
            "member path is read from: x.inner.cb and p->inner.cb are outer.inner.cb, so a "
            "member spelled inner_tag.cb (TAG.FIELD on the embedded record) does not see "
            "the store, and outer.inner.cb does not see a store into inner_tag.cb through "
            "a pointer to the embedded object. Likewise a record value copied inside an "
            "initializer list that holds no function-pointer value (struct outer o = "
            "{ proto }) opens nothing. Found by reading callback_member_closure's member_key "
            "and InitListExpr arm. Corpus size for the declared members: 0 (struct "
            "rd_kafka_op_s is embedded by value nowhere in librdkafka; every store into "
            "rko_op_cb and rko_serve is through rko->). DISCHARGE: a store through an "
            "embedded record keyed (also) by the embedded record's own TAG.FIELD, and a "
            "record value copied into a field (assignment or initializer) opens the copied "
            "record's function-pointer members unless its source's closure is read, with "
            "tests for both and the corpus member sections re-measured."
        ),
    },
]


def main():
    matrix = json.loads(MATRIX.read_text())
    followups = [entry for entry in matrix["named_followups"] if entry["id"] != CARD]
    known = {entry["id"] for entry in followups}
    for entry in NEW:
        record = {"id": entry["id"], "title": entry["title"], "why": entry["why"],
                  "source": SOURCE, "recorded_by": CARD}
        if entry["id"] in known:
            followups = [record if e["id"] == entry["id"] else e for e in followups]
        else:
            followups.append(record)
    matrix["named_followups"] = followups
    matrix["dimensions"]["named_followups"] = len(followups)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print("named_followups", len(followups))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
