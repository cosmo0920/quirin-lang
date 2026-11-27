#!/usr/bin/env python3
"""Archive complete primary artifacts and exact run identities after success."""
import gzip
import hashlib
import json
from pathlib import Path
import sys

measurement, destination = map(Path, sys.argv[1:3])
destination.mkdir(parents=True, exist_ok=False)
files = ["resource-diagnostics.json",
         "resource-diagnostics.chunk-contracts.json",
         "resource-diagnostics.source-effects.json",
         "resource-diagnostics.contracts.json"]
manifest = {}
for route in ("corpus", "system", "slice"):
    assert (measurement / f"{route}.exit").read_text().strip() == "0", route
    output = destination / route
    output.mkdir()
    route_files = files + (["monkey-source.json"] if route == "corpus" else ["resource-diagnostics.invocation.json"])
    sources = [measurement / route / name for name in route_files]
    sources += [measurement / f"{route}-null.jsonl", measurement / f"{route}-outer.json",
                measurement / f"{route}-invocation.json", measurement / f"{route}.exit",
                measurement / f"{route}.log", measurement / f"{route}.stderr"]
    sources += [p for p in (measurement / route / "run.json", measurement / route / "audit.log") if p.exists()]
    manifest[route] = []
    for source in sources:
        data = source.read_bytes()
        (output / (source.name + ".gz")).write_bytes(gzip.compress(data, mtime=0))
        manifest[route].append({"source": str(source), "archive": f"{route}/{source.name}.gz",
                                "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()})
for name in ("head", "binary.sha256", "link-map.sha256"):
    (destination / name).write_bytes((measurement / name).read_bytes())
(destination / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
