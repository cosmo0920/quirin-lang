#!/usr/bin/env python3
"""Apply W47-PREMISES-INSTANCE's ledger edits (idempotent).

Usage: update_ledger.py   (then regenerate with docs/reports/remaining-d02/build_matrix.py)

* retires nothing: no registered entry's DISCHARGE clause is met (the card
  has no entry of its own; W47-LOOP-OWNER-INSTANCE-FLOW keeps its ctx->ins,
  threaded-flag and engine-label parts);
* registers every entry of `followups.json` not yet registered;
* appends this card's measured amendment to W47-LOOP-OWNER-INSTANCE-FLOW and
  W47-INSTANCE-FLOW-CONTRACT-REVIEW, once each.

It applies what it can and reports, rather than refuses, an entry already
registered or amended, or missing. Running it again changes nothing.
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/w47-premises-instance"
RETIRE: list[str] = []
AMEND = {
    "W47-LOOP-OWNER-INSTANCE-FLOW": (
        " AMENDED by W47-PREMISES-INSTANCE: the premised census (--derive-premises) derives "
        "P-FRESH-CONTEXT (flb_create stores flb_config_init's one fresh flb_calloc config, both "
        "lists initialised empty at its top level) and P-REGISTERED-DESCRIPTORS (the generated "
        "flb_plugins_register, resolved by src/flb_config.c's -I order, registers 50 descriptors, "
        "\"tail\" matching in_tail_plugin), narrows P-HEAD-INSTANCE to P-CREATIONS-SUCCEED "
        "(W47-PREMISES-CREATIONS-SUCCEED) and the checkable part of P-INSTANCE-FIELDS-BY-MEMBER "
        "(W47-FIELDS-BY-MEMBER-VOID-ESCAPES), and derives P-INSTANCE where the operation names its "
        "callback's unassigned first parameter reached only by loop dispatches passing "
        "C->instance: the fixed test's 3 sites (discharged at 90978ac6 configured under these "
        "derived premises) and in_docker_events :159; the ctx->ins form stays stated "
        "(W47-P-INSTANCE-CONTEXT). The threaded-flag and engine-label parts are untouched."
    ),
    "W47-INSTANCE-FLOW-CONTRACT-REVIEW": (
        " AMENDED by W47-PREMISES-INSTANCE: the premised census now derives P-FRESH-CONTEXT and "
        "P-REGISTERED-DESCRIPTORS and narrows P-HEAD-INSTANCE to P-CREATIONS-SUCCEED; the held "
        "clauses stay correct and held, their reasons' wording is stale "
        "(W47-INSTANCE-CONTRACT-REASONS-AFTER-PREMISES). DISCHARGE unchanged."
    ),
}


def main() -> int:
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    entries = matrix["named_followups"]
    before = len(entries)
    ids = {entry["id"] for entry in entries}
    new = json.loads((REPORT / "followups.json").read_text(encoding="utf-8"))
    for entry in new:
        if entry["id"] in ids:
            print(f"{entry['id']}: already registered")
    for name in RETIRE:
        if name not in ids:
            print(f"{name}: not registered (already retired or never registered)")
    kept = [dict(entry) for entry in entries if entry["id"] not in RETIRE]
    retired = before - len(kept)
    amended = 0
    for entry in kept:
        text = AMEND.get(entry["id"])
        if text is None:
            continue
        if text.strip() in entry["why"]:
            print(f"{entry['id']}: already amended")
        else:
            entry["why"] += text
            amended += 1
    for missing in sorted(set(AMEND) - ids):
        print(f"{missing}: not registered, amendment not applied")
    added = [entry for entry in new if entry["id"] not in ids]
    kept.extend(added)
    if kept == entries:
        print(f"follow-ups {before}: already applied")
        return 0
    matrix["named_followups"] = kept
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"follow-ups {before} -> {len(kept)}; registered {len(added)}; retired {retired}; "
          f"amended {amended}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
