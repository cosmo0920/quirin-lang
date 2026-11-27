#!/usr/bin/env python3
"""The revision-mode pilot of card W45-CONTRACT-REVISION-MODE, after its runs.

usage: pilot.py BINARY OUT_DIR [CARD_RUN VARIANT_RUN]   (from the repository root)

CARD_RUN and VARIANT_RUN name the card's two runs (default `card-slice` and
`variant-slice`; the integration review's rerun with the version-2 locks uses
`card2-slice` and `variant2-slice`).

Expects, under PRIVATE (below), the prepared attempts `rev-864c3d97` and
`rev-046accf6` with the runs `base-slice` (the base binary), `card-slice` (this
card's binary, the committed contracts) and `variant-slice` (this card's binary,
the second contract set of `pilot_variant.py`), each written by
`run-current.sh`. It writes to OUT_DIR:

* `base-vs-card.json` — per revision, whether the card's report is
  byte-identical to the base binary's and whether its contract sidecar differs
  only by the additive `contract_revision` key and the libc texts;
* `compare-revisions.json` — `quirin-audit compare` between the two revisions
  under the committed contracts;
* `compare-contract-sets.json` — `quirin-audit compare` between the committed
  contracts and the variant at 046accf6;
* `compare-variant-revisions.json` — between the two revisions under the variant;
* `summary.json` — the figures the report quotes.
"""
import hashlib
from collections import Counter
import json
import pathlib
import subprocess
import sys

PRIVATE = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w45-contract-revision-mode")
REVISIONS = ("rev-864c3d97", "rev-046accf6")
CARD = sys.argv[3] if len(sys.argv) > 3 else "card-slice"
VARIANT = sys.argv[4] if len(sys.argv) > 4 else "variant-slice"
MARKER = "    requires resource_revision(flexible);\n"
NOTE_START = "// Card W45-CONTRACT-REVISION-MODE: every contract in this module is\n"


def report(attempt, run):
    return PRIVATE / attempt / run / "resource-diagnostics.json"


def sidecar(attempt, run):
    return PRIVATE / attempt / run / "resource-diagnostics.contracts.json"


def restore_libc(text):
    if NOTE_START in text:
        start = text.index(NOTE_START)
        end = text.index("\n\n", start) + 2
        text = text[:start] + text[end:]
    return text.replace(MARKER, "")


def base_vs_card(attempt):
    base, card = report(attempt, "base-slice"), report(attempt, CARD)
    base_side = json.loads(sidecar(attempt, "base-slice").read_text())
    card_side = json.loads(sidecar(attempt, CARD).read_text())
    additive = "contract_revision" in card_side and "contract_revision" not in base_side
    card_side.pop("contract_revision", None)
    restored = 0
    for key, text in list(card_side["contracts"].items()):
        if "/quirin-provider-libc/" in key and restore_libc(text) != text:
            card_side["contracts"][key] = restore_libc(text)
            restored += 1
    for module in card_side.get("contract_inclusion", {}).get("modules", {}).values():
        digests = {}
        for key, included in module.get("included", {}).items():
            if key.startswith("quirin-provider-libc:"):
                old = included["identity"]["sha256"]
                included["source"] = restore_libc(included["source"])
                included["identity"]["sha256"] = hashlib.sha256(
                    included["source"].encode()).hexdigest()
                digests[old] = included["identity"]["sha256"]
        for chain in module.get("inherited", {}).values():
            for link in chain:
                link["module"]["sha256"] = digests.get(link["module"]["sha256"],
                                                       link["module"]["sha256"])
    # The libc texts are contract sources, so every compile-task identity's
    # `resource_contract_sources` digest (and the configuration's inventory
    # digest over it) moves with them; nothing else may.
    def masked(path):
        data = json.loads(path.read_text())
        for unit in data["units"]:
            configuration = unit.get("compile_configuration") or {}
            configuration.pop("inventory_sha256", None)
            configuration.get("dependencies", {}).pop("resource_contract_sources", None)
            identity = configuration.get("compile_task_identity") or {}
            identity.get("dependencies", {}).pop("resource_contract_sources", None)
        return data

    return {
        "report_byte_identical": base.read_bytes() == card.read_bytes(),
        "report_identical_after_masking_contract_source_digests": masked(base) == masked(card),
        "report_sha256": hashlib.sha256(card.read_bytes()).hexdigest(),
        "sidecar_additive_key_present": additive,
        "libc_texts_restored": restored,
        "sidecar_identical_after_restoring_the_card_edits": card_side == base_side,
        "differing_keys": sorted(k for k in set(card_side) | set(base_side)
                                 if card_side.get(k) != base_side.get(k)),
    }


def compare(binary, before, after, out):
    completed = subprocess.run([binary, "compare", "--before", str(before), "--after", str(after),
                                "--output", str(out)], capture_output=True, text=True)
    if completed.returncode != 0:
        raise SystemExit(f"compare failed: {completed.stderr}")
    return completed.stdout.strip()


def main():
    binary = str(pathlib.Path(sys.argv[1]).resolve())
    out = pathlib.Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=True)
    checks = {attempt: base_vs_card(attempt) for attempt in REVISIONS}
    (out / "base-vs-card.json").write_text(json.dumps(checks, indent=2) + "\n")
    lines = {
        "revisions": compare(binary, report(REVISIONS[0], CARD),
                             report(REVISIONS[1], CARD), out / "compare-revisions.json"),
        "contract_sets": compare(binary, report(REVISIONS[1], CARD),
                                 report(REVISIONS[1], VARIANT),
                                 out / "compare-contract-sets.json"),
        "variant_revisions": compare(binary, report(REVISIONS[0], VARIANT),
                                     report(REVISIONS[1], VARIANT),
                                     out / "compare-variant-revisions.json"),
    }
    sections = {}
    for attempt in REVISIONS:
        for run in (CARD, VARIANT):
            section = json.loads(sidecar(attempt, run).read_text())["contract_revision"]
            sections[f"{attempt}/{run}"] = {
                "source": section["source"], "counts": section["counts"],
                "pin_mismatches": [{k: m[k] for k in ("function", "module", "reason", "detail",
                                                      "held_functions", "held_call_sites")}
                                   for m in section["pin_mismatches"]],
                "flexible_checked": [c["function"] for c in section["flexible_checked"]],
                "standing": [{k: s[k] for k in ("name", "owner", "contracts")}
                             for s in section["assumptions"]["standing"]],
                # Which reviewed identity each in-force pin matched (lock
                # version 2), counted by the revisions of that identity.
                "in_force_by_identity": dict(sorted(Counter(
                    " + ".join(source.get("revision", source["kind"])[:8]
                               for source in record.get("matched_reviewed_at", []))
                    for record in section["pinned_in_force"]).items())),
                "ctr_destroy": next(
                    ({"in_force": "matched_reviewed_at" in r,
                      "matched_reviewed_at": r.get("matched_reviewed_at"),
                      "reason": r.get("reason")}
                     for r in section["pinned_in_force"] + section["pin_mismatches"]
                     if r["function"] == "ctr_destroy"), None),
            }
    summary = {"base_vs_card": checks, "compare_console": lines, "sections": sections}
    (out / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
