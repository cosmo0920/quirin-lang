#!/usr/bin/env python3
"""runs.py OUT.json NAME... -- OUTER-CONTRACT: each named run's binary, build
head, corpus run.json binary digest, wall times and exits (private/oc/)."""
import json
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
OC = REPO / "private/oc"
out = {}
for name in sys.argv[2:]:
    out[name] = {
        "binary_sha256": (OC / "bin" / f"{name}.sha256").read_text().split()[0],
        "head": (OC / f"build-{name}.head").read_text().strip(),
        "corpus_run_binary_sha256": json.loads((OC / name / "run.json").read_text())["binary_sha256"],
        "seconds": {r: (OC / f"{name}{s}.seconds").read_text().strip()
                    for r, s in (("corpus", ""), ("system", "-system"), ("slice", "-slice"))},
        "exits": {r: (OC / f"{name}{s}.exit").read_text().strip()
                  for r, s in (("corpus", ""), ("system", "-system"), ("slice", "-slice"))},
    }
pathlib.Path(sys.argv[1]).write_text(json.dumps(out, indent=1) + "\n")
print(json.dumps(out, indent=1))
