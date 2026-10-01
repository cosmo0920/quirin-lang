#!/usr/bin/env python3
"""CG06-VALUE-CONNECTIVE-RESIDUALS (sibling-effect part): the ledger edit,
applied once (idempotent).

usage: update_ledger.py MATRIX_JSON

Amends CG06-VALUE-CONNECTIVE-RESIDUALS with what rule 11m landed and the
re-measured sizes (it is not retired: its other classes are owned elsewhere),
and keeps `dimensions.named_followups` equal to the list length. Writes the
matrix back in its own form (indent=1, ensure_ascii=False, trailing newline).
"""

import json
import pathlib
import sys

ID = "CG06-VALUE-CONNECTIVE-RESIDUALS"
AMEND = (
    " AMENDED (CG06-VALUE-CONNECTIVE-RESIDUALS, sibling-effect part, gateway"
    " rule 11m, main 0a22a998; docs/reports/cg06-connective-footprints): on"
    " that main the consultations are 2,485 (255 admitted) and the two"
    " sibling-effect classes reproduce exactly (1,133 and 4). Rule 11m"
    " composes them with CG05's contract footprints (one graph per arm,"
    " EvalGraph::check_footprints unchanged): 987 compose and 150 stay refused"
    " (143 an uncontracted sibling call: rd_kafka_err2str 46, rd_kafka_err2name"
    " 23, rd_kafka_fetch_pos2str 15, rd_kafka_broker_name 13, ...; 3"
    " overlapping footprints, mconn against rd_list_cnt; 2 an uncontracted"
    " deciding call; 1 a sibling increment; 1 an indirect sibling call). Of"
    " the 987, 2 are lowered (two cmetrics snprintf statements), 976 refuse on"
    " an edge an older route refuses (808 statements, 799 rd_kafka_log0: the"
    " lexical indirect-access gate first for 757, sizeof for 48), 5 exceed 8"
    " leaves, 4 have an undecided deciding operand. No function moved on any"
    " route. Sizes now: 1,387 edge refusals (CG10 places and"
    " CG05-CALL-ROOTED-PLACES, CG08 sizeof, CG11, CG16, uncontracted calls),"
    " 315 undecided deciding operands (CG04 / conditions), 284 I04, 148 sibling"
    " with an effect, 67 under a comma (CG02), 14 indirect callee (CG22), 9"
    " over 8 leaves, 2 deciding call beside a sibling, 2 nested assignment or"
    " declaration (CG02). The two sibling classes' DISCHARGE is now the"
    " uncontracted callees' contracts (CG05-UNCONTRACTED-INNER-CALLEES) and"
    " identity distinctness (CG05-FOOTPRINT-ADDRESS-CONSERVATISM); a sibling"
    " effect that is not a call stays refused by decision."
)


def main() -> int:
    path = pathlib.Path(sys.argv[1])
    matrix = json.loads(path.read_text())
    entries = matrix["named_followups"]
    for entry in entries:
        if entry["id"] == ID and AMEND not in entry["why"]:
            entry["why"] += AMEND
    matrix["dimensions"]["named_followups"] = len(entries)
    path.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"named_followups: {len(entries)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
