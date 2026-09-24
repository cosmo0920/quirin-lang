#!/usr/bin/env python3
"""Copy the arbiter's private evidence from `measure/` into this report.

Usage: publish_evidence.py

Run records, the BEFORE identity, the four staged attributions and the full
one (gzip, mtime 0), their summaries, the exchange analysis and the totals.
Every source is this worktree's private `measure/` directory.
"""

import gzip
import json
import pathlib
import shutil

ROOT = pathlib.Path(__file__).resolve().parents[4]
M = ROOT / "measure"
R = ROOT / "docs/reports/w43-libc-printf-family"


def gz(source, target):
    target.write_bytes(gzip.compress(source.read_bytes(), compresslevel=9, mtime=0))


def main() -> int:
    shutil.copyfile(M / "before/run.json", R / "before-run.json")
    shutil.copyfile(M / "after/run.json", R / "after-run.json")
    for stage in ("printf", "fprintf", "snprintf"):
        shutil.copyfile(M / f"stages/{stage}/run.json", R / f"stage-{stage}-run.json")
    shutil.copyfile(M / "before-identity.json", R / "before-identity.json")
    for name in ("stage-summary.json", "exchange.json", "totals.json", "contracts-diff.json"):
        shutil.copyfile(M / name, R / name)
    for stage in ("printf", "fprintf", "snprintf", "sprintf", "full"):
        gz(M / f"attr-{stage}-closed.json", R / f"attribution-{stage}.json.gz")
    summary = json.loads((M / "attr-full-closed.json").read_text())["summary"]
    (R / "attribution-summary.json").write_text(json.dumps(summary, indent=1) + "\n")
    print(sorted(p.name for p in R.iterdir()))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
