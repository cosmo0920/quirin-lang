#!/usr/bin/env python3
"""Attribute the measured migration's five-artifact differences without editing evidence.

Run from the repository root with the measured artifact directory as argument.
The raw exhaustive diff remains published separately; identity normalization
here is restricted to explicit fields and bound identifiers.
"""
import copy
import gzip
import importlib.util
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[3]
MEASURED = Path(sys.argv[1])
ARCHIVED = ROOT / "docs/reports/w42-priority-integration/artifacts"
spec = importlib.util.spec_from_file_location("artifact_diff", ROOT / "docs/reports/corpus-repin/diff_artifacts.py")
diff = importlib.util.module_from_spec(spec)
spec.loader.exec_module(diff)


def pair(stem, name):
    return json.loads(gzip.decompress((ARCHIVED / (stem + ".json.gz")).read_bytes())), json.loads((MEASURED / name).read_text())


def without_profile_identity(value):
    if isinstance(value, dict):
        return {key: without_profile_identity(item) for key, item in value.items() if key != "profile_digest"}
    if isinstance(value, list):
        return [without_profile_identity(item) for item in value]
    return value


checks = {}
for stem, name in [("source", "monkey-source.json"), ("chunk-contracts", "resource-diagnostics.chunk-contracts.json")]:
    before, after = pair(stem, name)
    assert without_profile_identity(before) == without_profile_identity(after), stem
    checks[stem] = "equal apart from profile_digest fields"
assert gzip.decompress((ARCHIVED / "diagnostics.json.gz").read_bytes()) == (MEASURED / "resource-diagnostics.json").read_bytes()
checks["diagnostics"] = "byte-identical"
before, after = pair("contracts", "resource-diagnostics.contracts.json")
a, collisions_a = diff.contract_sources(before)
b, collisions_b = diff.contract_sources(after)
assert not collisions_a and not collisions_b
assert {k:v["text"] for k,v in a.items()} == {k:v["text"] for k,v in b.items()}
assert len(a) == 33
for value in [before, after]:
    value.pop("sources")
    manifest = json.loads(value.pop("manifest"))
    assert manifest["schema_version"] == 1 and len(manifest["modules"]) == 33
    value["source_profile"].pop("profile_digest")
assert before == after
checks["contracts"] = "33 identical source texts; only source keys, manifest paths and profile digest differ"
before, after = pair("source-effects", "resource-diagnostics.source-effects.json")
assert len(before["results"]) == len(after["results"]) == 96
changed = []
for index, (left, right) in enumerate(zip(before["results"], after["results"])):
    if left == right:
        continue
    left = copy.deepcopy(left)
    # Only these three configured Boolean-field checks carry relocated profile
    # identities. Every other result must remain exactly equal.
    assert index in [16, 24, 63]
    old_id = left["task"]["obligations"][0]["obligation_id"]
    new_id = right["task"]["obligations"][0]["obligation_id"]
    assert left["bounds"][old_id] == right["bounds"][new_id]
    left["bounds"][new_id] = left["bounds"].pop(old_id)
    assert left["evidence"]["coverage"] == [old_id]
    left["evidence"]["coverage"] = [new_id]
    left["task"]["obligations"][0]["obligation_id"] = new_id
    paths = [
        ("evidence", "unit", "build_digest"),
        ("import_identity", "profile_digest"),
        ("task", "unit", "build_digest"),
        ("task", "contract_sources", "qcore:boolean-field-import:v1"),
        ("task", "contract_sources", "qcore:boolean-field-source-input:v2"),
        ("witness", "summary", "profile_assumptions", "profile_digest"),
        ("witness", "task_digest"),
    ]
    for path in paths:
        old, new = left, right
        for component in path[:-1]:
            old, new = old[component], new[component]
        assert len(old[path[-1]]) == len(new[path[-1]]) == 64
        old[path[-1]] = new[path[-1]]
    assert left == right, index
    changed.append(index)
assert changed == [16, 24, 63]
assert {k:v for k,v in before.items() if k != "results"} == {k:v for k,v in after.items() if k != "results"}
checks["source-effects"] = "93 exact results;3 differ only in checked identity fields and bound obligation identifiers; all bounds/verdicts equal"
print(json.dumps(checks, indent=2))
