#!/usr/bin/env python3
"""QR-PRINT-ROUTE-MATCHES-DECLARATION: retire its own entry, amend
W45-CALLEE-CASCADE-RESIDUAL (print is declared), and register
QR-LEXICAL-SELECTORS-DECLARATION-NAMES, in the acceptance matrix (indent=1,
ensure_ascii=False; `dimensions.named_followups` = list length). Idempotent."""
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = "docs/reports/qr-print-route-matches-declaration/README.md"
CARD = "QR-PRINT-ROUTE-MATCHES-DECLARATION"
RETIRE = CARD
RUN = (
    "corpus 66910c10, base da6c1f448, binaries 0be034f6 -> 54d41ce4, cdb0267b 933/9/3,892 -> "
    "bdf5f1c8 935/9/3,890; system 93/4/351 and slice 52/1/347/4 Pending, no row moves; null "
    "property unchanged"
)
AMEND = {
    "W45-CALLEE-CASCADE-RESIDUAL": (
        " AMENDED (" + CARD + ", " + RUN + "): cJSON's static print is declared as round 3 "
        "recorded it, now that the print-call route selection reads a call and a contract module "
        "is never compiled on source.scalar-calls.v1; cJSON_Print and cJSON_PrintUnformatted "
        "Unknown -> Safe on that one declared assumption. Population on scripts/rank.py: 481 -> "
        "479 Unknown functions blocked by uncontracted calls alone, 329 -> 327 decided by one "
        "callee alone; print is no longer among the remaining callees."
    ),
}
REGISTER = [
    {
        "id": "QR-LEXICAL-SELECTORS-DECLARATION-NAMES",
        "title": "Route selectors other than the print-call selector are still whole-token-stream "
        "tests, so a declaration named after a selector word is not proved to leave a module's "
        "route unchanged",
        "why": (
            "Measured by " + CARD + " (" + RUN + "; route census of 718 tracked .qr/.qv/.qcore/"
            ".qcorelib sources, two configurations, 1,436 rows, 0 moved by the fix): the print-call "
            "selector now classifies each `print (` pair against source function bodies and "
            "statement heads, and compile_scalar_calls refuses a `module contract;` module. The "
            "other routes' selectors (module_imports::concurrency_span, "
            "source_tasks::has_source_tasks, source_composed's line words, the buffer and socket "
            "intrinsics) still read the whole token stream; rule 14 excludes library binding spans "
            "and socket hosts from them, not extern declaration names or contract clauses. Every "
            "tracked module, the 214 contract modules included, is routed as on the base, so "
            "nothing tracked is misrouted. But the defect class is live: a contract module "
            "declaring `extern fn qcore_buffer_new(size: usize) -> ptr<mut, u8>` is refused "
            "`Q015 concurrency requires an explicit #import std.concurrency;` at the declaration "
            "name, while the same module naming `flb_noop` is admitted as a Contract module "
            "(artifacts/probe/, `quirin check` on the card's tip). Other selector words (log, "
            "flush, spawn, join, bytes, ...) are unmeasured, and the contract-mode guard covers "
            "only source.scalar-calls.v1. "
            "DISCHARGE: a census of each whole-token selector against declarations named after its "
            "words (one synthetic module per word and mode), and either a body-scoped reading or a "
            "contract-mode guard for every selector that a declaration name moves, with the route "
            "census unchanged."
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
    for key, text in AMEND.items():
        entry = by_id[key]
        if "AMENDED (" + CARD not in entry["why"]:
            entry["why"] += text
    for entry in REGISTER:
        if entry["id"] not in by_id:
            followups.append(entry)
    matrix["dimensions"]["named_followups"] = len(followups)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(len(followups))


if __name__ == "__main__":
    main()
