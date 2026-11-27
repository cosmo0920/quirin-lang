#!/usr/bin/env python3
"""Archive a measurement run's evidence into the report directory.

Usage: archive.py RUN_DIR OUT_DIR

Copies the no-build configurations, the selection, and the attributions
(the whole-checkout one gzipped), and writes `runs.json`: per run its exit,
wall time, outcome tallies from the sidecar (or the configured run's console
summary) and the SHA-256 of every report artifact, plus the jobs 1/16 check.
Reports themselves stay in the private run directory; their digests identify
them.
"""

import gzip
import hashlib
import json
import pathlib
import shutil
import sys


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read(path):
    return path.read_text().strip() if path.exists() else None


def main() -> int:
    run, out = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    out.mkdir(parents=True, exist_ok=True)
    for name in ("bare.json", "src-includes.json", "selection.json", "binary.sha256"):
        shutil.copyfile(run / name, out / name)
    for name in ("bare", "src-includes"):
        shutil.copyfile(run / f"attribution-{name}.json", out / f"attribution-{name}.json")
    with open(run / "attribution-root.json", "rb") as source, gzip.GzipFile(
        out / "attribution-root.json.gz", "wb", mtime=0
    ) as target:
        shutil.copyfileobj(source, target)
    runs = {}
    for name, report in (
        ("configured", run / "configured/contracts-only.json"),
        ("bare", run / "bare/report.json"),
        ("src-includes", run / "src-includes/report.json"),
        ("root", run / "root/report.json"),
    ):
        record = {
            "exit": int(read(run / f"{name}.exit")),
            "wall_seconds": int(read(run / f"{name}.end")) - int(read(run / f"{name}.start")),
            "artifacts": {
                path.name: sha256(path)
                for path in sorted(report.parent.glob(report.stem + "*.json"))
            },
        }
        log = (run / f"{name}.log").read_text().splitlines()
        record["summary"] = [
            line for line in log
            if line.startswith(("current revision:", "no-build:", "not prepared:"))
        ]
        sidecar = report.with_suffix(".contracts.json")
        if name != "configured":
            record["outcomes"] = json.loads(sidecar.read_text())["outcomes"]
        runs[name] = record
    runs["jobs-1-vs-16"] = {
        "run": "src-includes at --jobs 1 against the 16-job run",
        "exit": int(read(run / "jobs1.exit")),
        "report_cmp_exit": int(read(run / "jobs1-report.cmp.exit")),
        "sidecar_cmp_exit": int(read(run / "jobs1-sidecar.cmp.exit")),
    }
    (out / "runs.json").write_text(json.dumps(runs, indent=1) + "\n")
    print(json.dumps({k: {x: v[x] for x in ("exit", "wall_seconds")} if "wall_seconds" in v else v
                      for k, v in runs.items()}, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
