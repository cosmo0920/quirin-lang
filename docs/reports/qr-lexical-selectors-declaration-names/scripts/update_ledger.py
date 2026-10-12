#!/usr/bin/env python3
"""QR-LEXICAL-SELECTORS-DECLARATION-NAMES: retire its own entry (its DISCHARGE
holds) and register ROUTE-SELECTION-ORDER-RESTATED, in the acceptance matrix
(indent=1, ensure_ascii=False; `dimensions.named_followups` = list length).
Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = "docs/reports/qr-lexical-selectors-declaration-names/README.md"
CARD = "QR-LEXICAL-SELECTORS-DECLARATION-NAMES"
RETIRE = CARD
REGISTER = [
    {
        "id": "ROUTE-SELECTION-ORDER-RESTATED",
        "title": "The contract-route guard restates compile_elaborated_source's route order "
        "in route_selection::source_only_route, and two fail-closed arms behind it are driven "
        "by no fixture",
        "why": (
            "Recorded by " + CARD + " (base 4b5c77041; synthetic census 2,658 rows, 321 -> 0 "
            "selections moved by a declaration; tracked route census 1,436 rows byte-identical; "
            "corpus, system and slice arbiters byte-identical). A `module contract;` module is "
            "refused before routing when route_selection::source_only_route names a source-only "
            "route: a 21-entry list of the driver's selectors in the driver's order, written a "
            "second time beside the if-chain of compile_elaborated_source "
            "(crates/qcore-verification/src/driver.rs). A route added to the chain and not to the "
            "list is still refused, by the completeness check after routing (a contract module "
            "admitted on any route but the sequential one), but with a message that names the "
            "profile rather than the route, after that route has lowered it. Neither the "
            "completeness check nor the print card's guard at compile_scalar_calls is reached by "
            "any census row or test today (the pre-routing guard catches every contract module "
            "first), so both are unexercised arms (stability-review item 1), stated rather than "
            "claimed. DISCHARGE: one selector table that both the driver's route selection and "
            "the contract guard read (so the order is written once), or a fixture that drives "
            "each remaining arm."
        ),
        "source": REPORT,
        "recorded_by": CARD,
    },
]


def main():
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    followups = matrix["named_followups"]
    followups[:] = [f for f in followups if f.get("id") != RETIRE]
    by_id = {f["id"]: f for f in followups}
    for entry in REGISTER:
        if entry["id"] not in by_id:
            followups.append(entry)
    matrix["dimensions"]["named_followups"] = len(followups)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(len(followups))


if __name__ == "__main__":
    main()
