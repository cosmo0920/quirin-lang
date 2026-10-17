#!/usr/bin/env python3
"""Replace the two W48 follow-up entries with this card's measured residuals."""

import json
from pathlib import Path


def main() -> None:
    repo = Path(__file__).resolve().parents[4]
    path = repo / "docs/reviews/r0_3/acceptance_matrix.json"
    matrix = json.loads(path.read_text())
    entries = {row["id"]: row for row in matrix["named_followups"]}
    plan = entries["W48-BOTTOM-UP-RESIDUAL-PLAN"]
    plan["title"] = "Measured bottom-up callee-body plan after the read-only call-actual lift"
    plan["why"] = (
        "Re-measured by W48-POINTER-RETURN-THROUGH-CALL-ACTUAL-ROOT at base fa04aab0f "
        "plus the exact read-only rd_list_cnt actual frame (corpus 941/9/3884 -> "
        "952/9/3873; report artifacts/after/plan.json.gz): 260 Unknown functions "
        "wait on exactly one in-corpus callee body, versus 264 before. The "
        "hypothetical all-body ceiling remains 372; the no-extra-body closure "
        "is 4 same-unit and 12 cross-unit, versus 0 and 8 before. There are "
        "2,228 candidate bodies, versus 2,232. Largest single body is "
        "cJSON_DetachItemViaPointer: six callers, seven own leaves across "
        "branch and indirect-access effects. Other small single-body candidates "
        "and family counts are in the archived after plan and family files. "
        "These are hypothetical closures, not Safe verdicts. DISCHARGE: lift "
        "a callee's own blockers soundly with Rust/Lean parity or a pinned "
        "body-checked contract, then rerun the corpus arbiter and plan."
    )
    plan["source"] = "docs/reports/w48-pointer-return-readonly/README.md"
    plan["recorded_by"] = "W48-POINTER-RETURN-THROUGH-CALL-ACTUAL-ROOT"

    pointer = entries["W48-POINTER-RETURN-THROUGH-CALL-ACTUAL-ROOT"]
    pointer["title"] = "Remaining admin pointer return and callee-summary refusals after an exact read-only call frame"
    pointer["why"] = (
        "W48-POINTER-RETURN-THROUGH-CALL-ACTUAL-ROOT checked the included "
        "rd_list_cnt(const rd_list_t *rl) body as the sole read "
        "`return rl->rl_cnt;`, then replayed lexical and AST binding readers "
        "while omitting only direct `rd_list_cnt(&formal->field[.field]*)` "
        "actuals. On pinned corpus 66910c10, 11 rdkafka_admin.c functions "
        "became Safe (six admin_result_ret_* helpers and five direct public "
        "accessors); every move removed one return-expression leaf, with no "
        "other changed diagnostic or Safe regression. The old 24-function "
        "closure was an upper bound, not actual movement. "
        "rd_kafka_ConfigResource_configs still refuses its pointer return: "
        "the typed return plan does not admit `config->config.rl_elems`. "
        "Three public ACL result callers still refuse their uncontracted "
        "helper calls despite those helpers becoming Safe; other public "
        "helper callers need the same summary check. DISCHARGE: establish "
        "the remaining typed return and same-unit callee-summary premises "
        "without trusting a Borrowed mode as a read-only frame, with Lean "
        "parity and a fresh arbiter measurement."
    )
    pointer["source"] = "docs/reports/w48-pointer-return-readonly/README.md"
    pointer["recorded_by"] = "W48-POINTER-RETURN-THROUGH-CALL-ACTUAL-ROOT"
    path.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"named_followups {len(matrix['named_followups'])}")


if __name__ == "__main__":
    main()
