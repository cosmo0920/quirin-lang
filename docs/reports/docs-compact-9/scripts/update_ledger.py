#!/usr/bin/env python3
"""Apply DOCS-COMPACT-9's hand-maintained matrix edits once, on the merged tree.

Usage: update_ledger.py   (then regenerate with docs/reports/remaining-d02/build_matrix.py)

* appends every entry of `followups.json` (refusing a duplicate id);
* retires INTEGRATION-GATE-PATH-RULE-DRIFT into `retired-followups.json`, with
  its ground, refusing if it is not registered;
* amends four STABILITY-REEVAL-5 findings this card partly discharged, by an
  appended sentence (skipped when the marker is already present).
"""
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = ROOT / "docs/reports/docs-compact-9"
CARD = "DOCS-COMPACT-9"
MARK = "AMENDED (DOCS-COMPACT-9, wave-45 close):"

RETIRED = "INTEGRATION-GATE-PATH-RULE-DRIFT"
GROUND = (
    "Its discharge condition: 'the rule text, and any close tooling, name the path in use.' "
    "DOCS-COMPACT-9 corrected docs/agents/operations.md's integration rule to "
    "docs/reports/<card>/integration/gate.json, the path every integration gate since wave 43 "
    "uses. No close tooling names either path: `git grep integration-gate` over scripts/, "
    "providers/, *.rs and *.sh finds nothing, and no script reads integration/gate.json by a "
    "hard-coded path, so the rule was the only place the old path stood."
)

AMEND = {
    "NATIVE-OBSERVATION-UPSTREAM-WORDING": (
        " " + MARK + " docs/STATUS.md's native descriptor row now reads 'two double closes read "
        "against the Fluent Bit source: local observations, provisional candidates, not upstream "
        "defect claims'. Open: docs/reports/w43-descriptor-native-check/README.md:71 (a frozen "
        "report, which needs an appended note) and the wave-45 record's 'the one added leak' for "
        "`mk_http_error`, now docs/development/increments/wave-45.md (a history record this card "
        "moved nothing out of and did not reword)."
    ),
    "STATUS-LATEST-GATE-STALE": (
        " " + MARK + " the section now names the latest integration gate "
        "(docs/reports/w45-audit-subcommand-plugins/integration/gate.json: 5,995 / 0 / 11 over "
        "331 summaries, 207 Lean audits, 364 Python tests, 129 provider). Open: no check ties "
        "STATUS's figures to a gate.json."
    ),
    "GATEWAY-5L-ELEMENT-TRACKED-AFTER-FOREIGN-UNLINK": (
        " " + MARK + " rule 5l's text is revised to the post-10f state: the unlink of the current "
        "element is 10f's foreign unlink, which binds it at a fresh site, and the lexical guard is "
        "stated as not seeing an element 10f has bound. Open: the fixture that re-publishes a "
        "foreign-unlinked element into the iterated list under a different head spelling, pinned "
        "!= Safe."
    ),
    "CHECKPOINT-TWO-PHASE-RESUME-NEVER-REUSES": (
        " " + MARK + " docs/development/audit-checkpoints.md now states the measured behaviour "
        "(both phases seal to <dir>/<label>.qo, so a resumed run under a non-empty table reuses no "
        "group). Kept open at the integrator's direction: the reuse loss is a code defect, and the "
        "phase-qualified paths with their test remain the discharge."
    ),
}


def main() -> int:
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    entries = matrix["named_followups"]
    ids = [entry["id"] for entry in entries]
    before = len(entries)
    new = json.loads((REPORT / "followups.json").read_text(encoding="utf-8"))
    for entry in new:
        if entry["id"] in ids:
            raise SystemExit(f"duplicate follow-up id {entry['id']}")
    if RETIRED not in ids:
        raise SystemExit(f"{RETIRED} is not registered on this base")
    retired = [entry for entry in entries if entry["id"] == RETIRED]
    (REPORT / "retired-followups.json").write_text(
        json.dumps([{"entry": e, "discharged_by": CARD, "ground": GROUND} for e in retired],
                   indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    entries = [entry for entry in entries if entry["id"] != RETIRED]
    for entry in entries:
        text = AMEND.get(entry["id"])
        if text is None:
            continue
        if MARK not in entry["why"]:
            entry["why"] += text
    missing = sorted(set(AMEND) - {entry["id"] for entry in entries})
    if missing:
        raise SystemExit(f"cannot amend unregistered {missing}")
    entries.extend(new)
    matrix["named_followups"] = entries
    MATRIX.write_text(json.dumps(matrix, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"named_followups {before} -> {len(entries)} (+{len(new)} -1); amended {len(AMEND)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
