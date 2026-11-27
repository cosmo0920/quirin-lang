#!/usr/bin/env python3
"""Apply W47-ACCESS-PATH-POSITION-RESIDUALS' matrix edits once, on the merged tree.

Usage: update_ledger.py   (then regenerate with docs/reports/remaining-d02/build_matrix.py)

* retires nothing: W47-ACCESS-PATH-POSITION-RESIDUALS stays open, since the
  pointer-valued `?:` return, the `?:` actual and the nested-call actual are
  neither admitted nor closed by a keep-refused decision;
* registers nothing;
* amends three entries, each by an appended sentence that starts with MARK
  (skipped when the marker is already present).
"""
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
MARK = "AMENDED (W47-ACCESS-PATH-POSITION-RESIDUALS):"

AMEND = {
    "W47-ACCESS-PATH-POSITION-RESIDUALS": (
        " " + MARK + " sized on the round-1 baseline (66be515f, 510 / 10 / 4,314) by a parity-only "
        "residual-shape record (8,969 refused rows: 4,365 call actuals, 4,306 condition operands, 233 "
        "returns, 65 condition assignments; causes: root premise alone 2,919, an address 2,300, no "
        "unique node 2,040, T *const 335, union 331, array decay 208, function pointer 147, nested call "
        "143, sizeof 130, anonymous enum 120, cast 93, ?: 55, assignment 54, pointer arithmetic 45) and "
        "ranked by functions freed alone (no shape above 9). DISCHARGED under gateway rule 10n, each "
        "measured as its own step: an address &PATH compared or handed to a Borrowed/data-tail actual "
        "(a non-pointer member, never-bound roots; (&X)->f as X.f) +10 Safe; an ambiguous operand "
        "matched inside its controlling expression or return 0 (523 condition rows, 116 COND leaves); "
        "the anonymous inline enum member +1 (rd_kafka_mock_cgrp_classic_check_state); a scalar "
        "&&/||/?: return +1 (is_incremental): 510 -> 522 Safe, no correction, no new violation. "
        "OPEN (AFTER, docs/reports/w47-access-path-position-residuals/artifacts/after/rank-corpus.json, "
        "8,066 refused rows): root premise 4,367 (19 functions alone with alias, 8 strict; the "
        "never-bound closure's cards), no unique node 1,460 (939 of them call actuals of a callee "
        "called twice on one line), address 618 (element or pointer-member addresses handed to a "
        "callee, truthiness of an address: kept refused), union 367, T *const 332, a ?: actual 53 "
        "(needs a branching call lowering; rd_kafka_mk_brokername), nested call 143, a pointer-valued "
        "?: return (get_client_rack, get_group_id, get_group_instance_id, get_transactional_id; also "
        "casts), a call or assignment in a return (cJSON_SetNumberHelper, rd_kafka_sasl_cyrus_ready), "
        "and the rule 5j array-decay return (rd_kafka_name), which stays refused."
    ),
    "W47-TYPED-CONST-POINTER-VALUE": (
        " " + MARK + " the residual-shape record sizes the T *const value refusal at 335 refused "
        "position rows on the round-1 baseline (154 with no other unread construct), the sole "
        "blocker of 7 Unknown functions with their alias leaves (6 strict; e.g. cJSON_GetStringValue, "
        "rd_kafka_Node_host, rd_kafka_message_leader_epoch); unchanged by rule 10n."
    ),
    "W47-LEXICAL-CLOSURE-NESTED-ASSIGNMENT": (
        " " + MARK + " interlock: none of rule 10n's 12 Safe moves has a newly admitted row rooted in "
        "a name the AST binds and the lexical closure calls never bound (compare_steps.py's "
        "interlock, artifacts/after/corpus-comparison/comparison.json). After rule 10n the root premise "
        "is the largest refused-position cause: 4,367 rows, freeing 19 functions alone with their "
        "alias leaves (8 strict)."
    ),
}


def main() -> int:
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    entries = matrix["named_followups"]
    ids = [entry["id"] for entry in entries]
    before = len(entries)
    missing = sorted(set(AMEND) - set(ids))
    if missing:
        raise SystemExit(f"not registered on this base: {missing}")
    amended = 0
    for entry in entries:
        if entry["id"] in AMEND and MARK not in entry["why"]:
            entry["why"] = entry["why"] + AMEND[entry["id"]]
            amended += 1
    MATRIX.write_text(json.dumps(matrix, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"follow-ups {before} -> {len(entries)}; amended {amended}; registered 0; retired 0")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
