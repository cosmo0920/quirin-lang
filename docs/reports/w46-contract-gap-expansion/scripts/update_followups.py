#!/usr/bin/env python3
"""Apply this integration's reviewed follow-up decisions exactly once.

Registers the eight source-reviewed boundaries, confirms the existing CTraces
family convention, and links the TLS instance to the earlier retained-result
boundary. Regenerate the acceptance matrix after running this script.
"""

import json
import pathlib


ROOT = pathlib.Path(__file__).resolve().parents[4]
REPORT = ROOT / "docs/reports/w46-contract-gap-expansion"
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
MARK = "AMENDED (W46-CONTRACT-GAP-EXPANSION):"


def main():
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    ids = {entry["id"] for entry in entries}
    proposed = json.loads((REPORT / "followups.json").read_text())
    fluent = json.loads((ROOT / "docs/reports/w46-fluent-event-contract-gaps/followups.json").read_text())
    for item in fluent:
        item.update(source="docs/reports/w46-fluent-event-contract-gaps/README.md",
                    recorded_by="W46-CONTRACT-GAP-EXPANSION")
    proposed += fluent
    if any(item["id"] in ids for item in proposed):
        raise SystemExit("a proposed follow-up is already registered; inspect instead of duplicating")
    retired_id = "W36-CTRACES-FAMILY-FIRST"
    if retired_id not in ids:
        raise SystemExit("the convention follow-up was not registered on this base")
    retired = next(item for item in entries if item["id"] == retired_id)
    old_path = REPORT / "retired-followups.json"
    if old_path.exists():
        raise SystemExit("retirement evidence already exists; inspect instead of replacing")
    old_path.write_text(json.dumps({
        "entry": retired,
        "decision": "discharged by integrator confirmation",
        "discharging_card": "W46-CONTRACT-GAP-EXPANSION",
        "reason": "Confirm qcore-provider-ctraces ownership, the ctraces namespace and contracts/ctraces directory, and context_lifetime.qcore as the existing context/ID/span facade module. This confirms repository organization only; it proves no C body and does not prevent future modules for other families.",
    }, indent=2) + "\n")
    retained = "W43-RESULT-ALIAS-CONDITIONAL-AND-RETAINED"
    if retained not in ids:
        raise SystemExit("the retained-result follow-up is missing")
    for item in entries:
        if item["id"] == retained:
            item["why"] += (
                " " + MARK + " flb_output_thread_instance_get is another retained-storage result, "
                "tracked by W46-FLUENT-TLS-RETAINED-RESULT (six call leaves in the configured "
                "system baseline). The new flb_output_flush_id_get declaration returns only "
                "a scalar and does not publish the TLS pointer. The retained-result vocabulary "
                "gap is not discharged by that wrapper declaration."
            )
    before = len(entries)
    matrix["named_followups"] = [item for item in entries if item["id"] != retired_id] + proposed
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"follow-ups {before} -> {len(matrix['named_followups'])}; added 8, amended 1, retired 1")


if __name__ == "__main__":
    main()
