#!/usr/bin/env python3
"""GAPO-RETURN-CORRECTNESS: retire GAPO-RETURN-PROBE-COUNTER-RESTORATION and
GAPO-RETURN-POSITION-ROUTE-GAPS (their DISCHARGE clauses hold) and register
the card's residuals, in the acceptance matrix (indent=1,
ensure_ascii=False; `dimensions.named_followups` = list length).
Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = "docs/reports/gapo-return-correctness/README.md"
CARD = "GAPO-RETURN-CORRECTNESS"
RETIRE = {"GAPO-RETURN-PROBE-COUNTER-RESTORATION", "GAPO-RETURN-POSITION-ROUTE-GAPS"}
REGISTER = [
    {
        "id": "GAPO-UNCAST-ACTUALS-AFTER-LATER-LOWERINGS",
        "title": "Rule 12h's hook H-U0 reads only classify_simple's instruction at a statement: "
        "a call a later route re-classifies (W44's call actuals, 10u, 10y, 11w's widening, an "
        "elaborated piece whose synthetic tokens match no Clang node) keeps a cast actual read "
        "as no place",
        "why": (
            "Recorded by " + CARD + " (base 1a399e6a1). Rule 15z asks H-U0 at the return "
            "position's W4, B01 and W45-K classifications and makes rule 12i decline a returned "
            "call with such a cast, but at a statement H-U0 is still asked only right after "
            "classify_simple: W44's call_argument_access re-classifies a gate-refused statement "
            "with its probe and admits the call with an identity-preserving cast actual "
            "(`(void *) t`) read as no place, so a Borrowed actual's state is not checked there "
            "(the soundness gap rule 12h repaired for the first classification). Size, lexical "
            "and indicative only: 7 statements in 7 functions of the corpus whose admitted W44 "
            "actual reading spells `* )` (scripts/residual_casts.py over the task-AB parity "
            "stream); 10u reads actuals through value-preserving conversions already (K03), "
            "and 10y, 11w and elaborated pieces were not counted. DISCHARGE: H-U0 asked of every "
            "admitted re-classification (or the route declines a call with a hidden place, as "
            "rule 12i now does), with a corpus measurement."
        ),
        "source": REPORT,
        "recorded_by": CARD,
    },
    {
        "id": "GAPO-DECLINED-PROBE-CONSULTATIONS",
        "title": "A declined classification probe's contract lookups stay in rule 13f's "
        "consultation log (and its parity rows stay written), so a verdict row may list a "
        "dependency its decision did not use",
        "why": (
            "Recorded by " + CARD + " (base 1a399e6a1). Rule 15x restores every parser field a "
            "declined probe moved; ResourceContracts::contract notes each lookup in the thread's "
            "consultation log (VERDICT-CONTRACT-DEPENDENCIES, rule 13f), which is not parser "
            "state, so a W4/B01/W45-K or fallback probe that declined still adds its callee to the "
            "row's dependency set. The direction is conservative (a Safe may be reported as "
            "resting on one more declaration, never on fewer), but the set is then not exactly "
            "the decision's premises. Size not measured: no stream tells a declined probe's "
            "lookup from a kept one's. DISCHARGE: a consultation scope opened around each probe "
            "and dropped on decline (with a measurement of the rows whose dependency set "
            "shrinks), or a recorded decision that the over-approximation stands."
        ),
        "source": REPORT,
        "recorded_by": CARD,
    },
]


def main():
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    followups = matrix["named_followups"]
    followups[:] = [f for f in followups if f.get("id") not in RETIRE]
    by_id = {f["id"]: f for f in followups}
    for entry in REGISTER:
        if entry["id"] in by_id:
            by_id[entry["id"]].update(entry)
        else:
            followups.append(entry)
    matrix["dimensions"]["named_followups"] = len(followups)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(len(followups))


if __name__ == "__main__":
    main()
