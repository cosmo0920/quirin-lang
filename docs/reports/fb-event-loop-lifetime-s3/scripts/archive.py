#!/usr/bin/env python3
"""Retain both primary report sets and run the existing exhaustive comparators.

Usage: archive.py BASE_DIAGNOSTICS TIP_DIAGNOSTICS ROUTE
A route archive must be fresh. Missing corpus sidecars fail closed.
"""
from pathlib import Path
import gzip
import hashlib
import json
import subprocess
import sys

REPO = Path(__file__).resolve().parents[4]
REPORT = Path(__file__).resolve().parents[1]
ARTIFACTS = {
    "diagnostics": "resource-diagnostics.json",
    "source": "monkey-source.json",
    "chunk-contracts": "resource-diagnostics.chunk-contracts.json",
    "source-effects": "resource-diagnostics.source-effects.json",
    "contracts": "resource-diagnostics.contracts.json",
}


def archive(path, destination, route):
    destination.mkdir(parents=True, exist_ok=False)
    fingerprints = {}
    for stem, name in ARTIFACTS.items():
        source = path if stem == "diagnostics" else path.parent / name
        if not source.is_file():
            if route == "corpus":
                raise FileNotFoundError(source)
            fingerprints[stem] = {"present": False}
            continue
        data = source.read_bytes()
        (destination / f"{stem}.json.gz").write_bytes(gzip.compress(data, mtime=0))
        fingerprints[stem] = {
            "present": True,
            "source": str(source),
            "sha256": hashlib.sha256(data).hexdigest(),
            "bytes": len(data),
        }
    counts = {}
    data = json.loads(path.read_bytes())
    for unit in data["units"]:
        for row in unit["functions"]:
            counts[row["status"]] = counts.get(row["status"], 0) + 1
    fingerprints["static_custody_counts"] = counts
    for name in ("head", "run.json", "invocation.json", f"{route}-invocation.json",
                 "binary.sha256", "link-map.sha256", f"{route}.exit"):
        source = next((p / name for p in (path.parent, path.parent.parent) if (p / name).is_file()), None)
        if source is not None:
            (destination / name).write_bytes(source.read_bytes())
    # Keep the independent null-property and runtime projections alongside
    # custody, without treating either projection's coverage as custody Safe.
    for name in (f"{route}-null.jsonl", f"{route}-null.summary.json", f"{route}-outer.json"):
        source = next((p / name for p in (path.parent, path.parent.parent) if (p / name).is_file()), None)
        if source is None:
            if name.endswith("-null.summary.json"):
                fingerprints[name] = {"present": False, "optional_derived_summary": True}
                continue
            raise FileNotFoundError(f"missing required projection: {name} beside {path}")
        data = source.read_bytes()
        (destination / f"{name}.gz").write_bytes(gzip.compress(data, mtime=0))
        fingerprints[name] = {"source": str(source), "bytes": len(data),
                              "sha256": hashlib.sha256(data).hexdigest()}
    (destination / "manifest.json").write_text(json.dumps(fingerprints, indent=2) + "\n")


def main():
    before, after = map(lambda text: Path(text).resolve(), sys.argv[1:3])
    route = sys.argv[3]
    if route not in {"corpus", "system", "slice"}:
        raise ValueError("route must be corpus, system or slice")
    out = REPORT / "artifacts" / route
    out.mkdir(parents=True, exist_ok=False)
    archive(before, out / "before", route)
    archive(after, out / "after", route)
    compared = subprocess.run(
        [sys.executable, str(REPO / "docs/reports/w44-cross-unit-summary/scripts/compare.py"), str(before), str(after)],
        capture_output=True, check=True,
    )
    (out / "row-comparison.json").write_bytes(compared.stdout)
    # The leaf comparator above keeps duplicate compile configurations. This
    # second record retains full diagnostics (including locations), dependency
    # carriers and every changed row, so count-only equality hides nothing.
    import importlib.util
    spec = importlib.util.spec_from_file_location("s3_row_compare", REPO / "docs/reports/w44-cross-unit-summary/scripts/compare.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    left, right = module.rows(before), module.rows(after)
    assert left.keys() == right.keys(), "selected denominator changed"
    changed = [{"key": key, "before": left[key], "after": right[key]}
               for key in left if left[key] != right[key]]
    encoded = (json.dumps(changed, indent=2) + "\n").encode()
    (out / "full-row-differences.json.gz").write_bytes(gzip.compress(encoded, mtime=0))
    (out / "strict-summary.json").write_text(json.dumps({
        "selected_rows": len(left), "changed_full_rows": len(changed),
        "changed_status_or_full_diagnostics": sum(
            row["before"]["status"] != row["after"]["status"] or
            row["before"].get("diagnostics") != row["after"].get("diagnostics")
            for row in changed),
        "raw_full_row_record": "full-row-differences.json.gz",
    }, indent=2) + "\n")
    subprocess.run([
        sys.executable, str(REPO / "docs/reports/corpus-repin/diff_artifacts.py"),
        "--archived", str(out / "before"), "--measured", str(after.parent),
        "--output", str(out / "artifact-diff.json"),
    ], check=True)
    print(json.dumps({"route": route, "archive": str(out), "comparators_exit": 0}))


if __name__ == "__main__":
    main()
