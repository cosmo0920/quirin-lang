#!/usr/bin/env python3
"""Whether an arbiter run's contract sidecar differs from the archived
baseline in exactly the changes of W45-CONTRACT-REVISION-MODE and this card.

usage: contracts_difference.py RUN_DIR BASELINE_ARTIFACT_DIR ROOT=CANONICAL

The baseline (`docs/reports/w45-fb-retry-lifetime/integration/artifacts/`)
predates W45-CONTRACT-REVISION-MODE, whose sidecar changes are undone exactly
as its own `contracts_difference.py` undoes them (that script is run first and
its restoration reused). Card W45-USER-CONTRACT-PLUGINS changes the sidecar in
one way on the corpus: the `meaning` string of the `pending_human_review`
section, which now says that a function resting only on held calls is
`Pending`. The section's counts, entries and every other key are unchanged
(the corpus has no pending review, no pin mismatch and no checker binding, so
no `plugin_checks` key is written). This script restores the previous meaning
and reports whether the result equals the baseline as JSON. It is a disclosure
beside `baseline_identity.py`, never a substitute for it.
"""

import gzip
import importlib.util
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve()
ROOT = HERE.parents[4]
PREVIOUS = ROOT / "docs/reports/w45-contract-revision-mode/scripts/contracts_difference.py"

OLD_MEANING = (
    "declared contracts, or clauses of them, marked `resource_review_pending` (contract spec "
    "C06): each is loaded and validated but withheld, discharges nothing and accuses nothing, "
    "and every call site that would apply it is Unknown with reason code "
    "`awaiting-human-review`;"
)
NEW_MEANING = (
    "declared contracts, or clauses of them, marked `resource_review_pending` (contract spec "
    "C06): each is loaded and validated but withheld, discharges nothing and accuses nothing, "
    "and every call site that would apply it is held with reason code `awaiting-human-review`, "
    "and a function whose only unresolved diagnostics are held calls is Pending, never Safe;"
)


def main() -> int:
    spec = importlib.util.spec_from_file_location("revision_difference", PREVIOUS)
    previous = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(previous)
    run = pathlib.Path(sys.argv[1]) / "resource-diagnostics.contracts.json"
    baseline_dir = pathlib.Path(sys.argv[2])
    root, canonical = sys.argv[3].split("=", 1)
    manifest = json.loads((baseline_dir / "manifest.json").read_text())
    baseline = json.loads(gzip.decompress(
        (baseline_dir / manifest["contracts"]["file"]).read_bytes()))
    data = json.loads(run.read_text().replace(root, canonical))
    # W45-CONTRACT-REVISION-MODE's edits, undone as its own script does.
    revision_present = "contract_revision" in data and "contract_revision" not in baseline
    data.pop("contract_revision", None)
    for key, text in list(data["sources"].items()):
        if "/quirin-provider-libc/" in key:
            data["sources"][key] = previous.restore(text)
    digests = {}
    modules = data.get("contract_inclusion", {}).get("modules", {}).values()
    for module in modules:
        for key, included in module.get("included", {}).items():
            if key.startswith("quirin-provider-libc:"):
                old = included["identity"]["sha256"]
                included["source"] = previous.restore(included["source"])
                included["identity"]["sha256"] = previous.hashlib.sha256(
                    included["source"].encode()).hexdigest()
                digests[old] = included["identity"]["sha256"]
    for module in modules:
        for chain in module.get("inherited", {}).values():
            for link in chain:
                identity = link["module"]
                identity["sha256"] = digests.get(identity["sha256"], identity["sha256"])
    # This card's edit.
    section = data.get("pending_human_review", {})
    meaning = section.get("meaning", "")
    meaning_changed = NEW_MEANING in meaning
    section["meaning"] = meaning.replace(NEW_MEANING, OLD_MEANING)
    result = {
        "contract_revision_additive_key_present": revision_present,
        "plugin_checks_key_present": "plugin_checks" in data,
        "pending_human_review_meaning_restored": meaning_changed,
        "pending_human_review_counts": {
            key: section.get(key)
            for key in ("declarations", "markers", "held_functions", "held_call_sites")
        },
        "identical_after_restoring_both_cards_edits": data == baseline,
        "top_level_keys_differing": sorted(
            key for key in set(data) | set(baseline) if data.get(key) != baseline.get(key)),
    }
    print(json.dumps(result, indent=2))
    ok = (result["identical_after_restoring_both_cards_edits"] and revision_present
          and meaning_changed and not result["plugin_checks_key_present"])
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
