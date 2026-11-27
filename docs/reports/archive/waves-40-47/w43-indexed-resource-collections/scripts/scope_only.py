#!/usr/bin/env python3
"""Whether a run's diagnostics equal an archived baseline's except `scope`.

Usage: scope_only.py RUN_DIR BASELINE_ARTIFACT_DIR

Parses both diagnostics documents, records both `scope` strings, removes
them, and compares the rest structurally and after canonical re-serialisation.
Exits 0 only when everything but `scope` is equal.
"""

import gzip
import json
import pathlib
import sys


def main() -> int:
    run = json.loads((pathlib.Path(sys.argv[1]) / "resource-diagnostics.json").read_text())
    baseline = json.loads(gzip.decompress((pathlib.Path(sys.argv[2]) / "diagnostics.json.gz").read_bytes()))
    record = {"run_scope": run.pop("scope", None), "baseline_scope": baseline.pop("scope", None)}
    record["equal_except_scope"] = run == baseline
    record["canonical_equal_except_scope"] = json.dumps(run, sort_keys=True) == json.dumps(baseline, sort_keys=True)
    print(json.dumps(record, indent=1))
    return 0 if record["equal_except_scope"] else 1


if __name__ == "__main__":
    sys.exit(main())
