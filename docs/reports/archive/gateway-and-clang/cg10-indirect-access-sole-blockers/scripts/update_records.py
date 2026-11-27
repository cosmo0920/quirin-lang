#!/usr/bin/env python3
"""CG10-INDIRECT-ACCESS-SOLE-BLOCKERS: the STATUS headline, the registered
follow-up count and the increment paragraph (wave-48-part4.md, newest first).
Idempotent: a paragraph or headline already naming the card is left alone.
"""
import json
import pathlib
import sys

CARD = "CG10-INDIRECT-ACCESS-SOLE-BLOCKERS"

PARAGRAPH = (
    "**`CG10-INDIRECT-ACCESS-SOLE-BLOCKERS` (2026-10-03, base `cdc2fa878`; prediction "
    "`c6613c89b`).** Round 17, the top-priority CG residual "
    "([report](../../reports/cg10-indirect-access-sole-blockers/README.md); gateway rules 12i and 12j; "
    "`QCore.ReturnCallActuals` and `QCore.RecordObjectPlace`, new `AuditReturnCallActuals.lean` and "
    "`AuditRecordObjectPlace.lean`, 284 → 286 audits). The 513 Unknown functions the access refusal "
    "blocks alone reproduced and were classified by the readers' own records (the measurement-only "
    "`indirect-census` streams and the parity rows; the census run byte-identical): 321 of them are a "
    "correction to the ranking — 218 call statements and 103 whole-call returns whose callee has no "
    "contract or summary in force, hidden behind the gate, which refuses before the contract lookup "
    "(the P4 callee closure; `W45-CALLEE-CASCADE-RESIDUAL` amended). The two largest liftable "
    "sub-classes were lifted. Rule 12i: a `return` whose one call the gate refused for its actuals is "
    "read with rule W44's call-argument access exactly as the same call as a statement, then the "
    "return position's unchanged acceptance (41 of 317 consultations admitted). Rule 12j: a `.`-only "
    "scalar place on a record object of the function's automatic storage is read with no root "
    "premise, since no pointer is read (544 place decisions admitted). Arbiter `d13ed4c1…` 654 / 9 / "
    "4,171 → `d2f5a3f7…` 695 / 9 / 4,130: **+41 Unknown → Safe** (14 by rule 12i, 15 by rule 12j, 12 "
    "through the summaries they validate), no Safe → non-Safe move, no leak or violation added; "
    "system 83 / 4 / 361 → 85 / 4 / 359 (`flb_engine_started`, twice), slice no status move; "
    "`mk_vhost_set` at the tip unchanged. Prediction +23 to +38, outcome above it by 3 (caller "
    "cascades). `CG10-INDIRECT-ACCESS-SOLE-RESIDUALS` registered with the ranked residual table; "
    "`CG10-TYPED-PLACE-RESIDUALS`, `CG12-VIEW-RESIDUAL-FORMS` and `CG13-UNION-LEAVES-OTHER-CAUSES` "
    "amended (ledger 697 → 698)."
)

HEADLINE_OLD = ("| Projected functions | **654 Safe / 9 provisional Unsafe / 4,171 Unknown**, 4,834 total "
                "(CAST-FORMALIZATION, gateway 12h,")
HEADLINE_NEW = (
    "| Projected functions | **695 Safe / 9 provisional Unsafe / 4,130 Unknown**, 4,834 total "
    "(CG10-INDIRECT-ACCESS-SOLE-BLOCKERS, gateway 12i and 12j, arbiter on main `cdc2fa878` plus the "
    "card: `d13ed4c1…` → `d2f5a3f7…`, **+41 Unknown → Safe**: 14 by rule 12i (a refused `return`'s one "
    "call read with rule W44's call-argument access: comparators, atomic getters, `rd_strcmp3`, "
    "`XXH_readLE32_align`/`XXH_readLE64_align`), 15 by rule 12j (`.`-only scalar places on automatic "
    "record objects: twelve librdkafka telemetry builders, `XXH32_reset`, `XXH64_reset`, "
    "`cnd_timedwait_ms`), 12 through the summaries they validate; no Safe → non-Safe move, no leak or "
    "violation added; system 83 / 4 / 361 → 85 / 4 / 359, slice no status move; of the 513 functions "
    "the access refusal blocked alone, 321 hide an uncontracted callee behind the gate "
    "([report](reports/cg10-indirect-access-sole-blockers/README.md)); before it 654 / 9 / 4,171, "
    "CAST-FORMALIZATION, gateway 12h,"
)


def main() -> int:
    root = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else pathlib.Path(".")
    status = root / "docs/STATUS.md"
    text = status.read_text()
    if CARD not in text.split("\n| Projected functions |", 1)[1][:400]:
        assert text.count(HEADLINE_OLD) == 1, "headline anchor"
        text = text.replace(HEADLINE_OLD, HEADLINE_NEW, 1)
    count = len(json.loads((root / "docs/reviews/r0_3/acceptance_matrix.json").read_text())[
        "named_followups"])
    old_row = "| Registered follow-ups | **697** (wave 48: "
    if old_row in text:
        text = text.replace(old_row, (
            f"| Registered follow-ups | **{count}** (wave 48: {CARD} registers "
            "`CG10-INDIRECT-ACCESS-SOLE-RESIDUALS` and amends `CG10-TYPED-PLACE-RESIDUALS`, "
            "`CG12-VIEW-RESIDUAL-FORMS`, `CG13-UNION-LEAVES-OTHER-CAUSES` and "
            "`W45-CALLEE-CASCADE-RESIDUAL`; before it 697: "), 1)
    status.write_text(text)
    record = root / "docs/development/increments/wave-48-part4.md"
    body = record.read_text()
    if f"**`{CARD}`" not in body:
        anchor = "**`CAST-FORMALIZATION` (2026-10-03"
        assert body.count(anchor) == 1
        body = body.replace(anchor, PARAGRAPH + "\n\n" + anchor, 1)
        record.write_text(body)
    print(count)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
