#!/usr/bin/env python3
"""Archive primary diagnostics, identity and refreshed planning evidence.

Usage: archive.py BEFORE_DIAGNOSTICS AFTER_DIAGNOSTICS
Existing archives are refused. Planning numbers remain hypothetical bounds.
"""
from pathlib import Path
import gzip
import hashlib
import json
import subprocess
import sys

REPO = Path(__file__).resolve().parents[4]
REPORT = Path(__file__).resolve().parents[1]
PLANNING = REPO / "docs/reports/w48-bottom-up-callee-bodies/scripts"
COMPARE = REPO / "docs/reports/w44-cross-unit-summary/scripts/compare.py"


def digest(data):
    return hashlib.sha256(data).hexdigest()


def archive(label, path):
    out = REPORT / "artifacts" / label
    out.mkdir(parents=True, exist_ok=False)
    data = path.read_bytes()
    (out / "resource-diagnostics.json.gz").write_bytes(gzip.compress(data, mtime=0))
    document = json.loads(data)
    counts = {}
    for unit in document["units"]:
        for row in unit["functions"]:
            counts[row["status"]] = counts.get(row["status"], 0) + 1
    manifest = {"source": str(path), "sha256": digest(data), "counts": counts}
    for name in ["run.json", "invocation.json", "head", "binary.sha256", "link-map.sha256"]:
        candidates = [path.parent / name, path.parent.parent / name]
        candidate = next((p for p in candidates if p.exists()), None)
        if candidate is not None:
            (out / name).write_bytes(candidate.read_bytes())
    contracts = path.with_suffix(".contracts.json")
    if contracts.exists():
        content = contracts.read_bytes()
        (out / "contracts.json.gz").write_bytes(gzip.compress(content, mtime=0))
        manifest["contracts_sha256"] = digest(content)
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    private = REPO / "private/w48-bottom-up-batch2" / f"{label}-plan.json"
    subprocess.run([sys.executable, str(PLANNING / "bottom_up_plan.py"), str(path), str(private)], check=True)
    (out / "plan.json.gz").write_bytes(gzip.compress(private.read_bytes(), mtime=0))
    subprocess.run([sys.executable, str(PLANNING / "family_leverage.py"), str(path), str(out / "family.json")], check=True)
    return out


def main():
    before, after = map(lambda p: Path(p).resolve(), sys.argv[1:3])
    archive("before", before)
    out = archive("after", after)
    result = subprocess.run([sys.executable, str(COMPARE), str(before), str(after)], capture_output=True, check=True)
    (out / "compare.json").write_bytes(result.stdout)


if __name__ == "__main__":
    main()
