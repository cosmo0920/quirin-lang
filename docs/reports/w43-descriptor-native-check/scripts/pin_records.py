#!/usr/bin/env python3
"""Pin one measurement's observation records into the report directory.

Usage: pin_records.py MEASUREMENT_DIR REPORT_DIR

MEASUREMENT_DIR holds `targets/` (build_targets.py) and `run/`
(run_scenarios.py) plus `aggregate.json`. Writes REPORT_DIR/records/
observations.tar.gz (deterministic: sorted members, zero mtimes and owners)
with every run's records.jsonl, run.json, report.json (when accepted),
stdout, stderr and fluent-bit.conf, plus sites.json, summary.json and
build.json; REPORT_DIR/records/manifest.json with each member's SHA-256; and
copies aggregate.json and build.json beside it. Binaries, storage chunk
files and copied plugin DSOs are not pinned; their hashes are in build.json.
"""
import gzip
import hashlib
import io
import json
import pathlib
import shutil
import sys
import tarfile

KEEP = {"records.jsonl", "run.json", "report.json", "stdout", "stderr", "fluent-bit.conf"}


def main() -> int:
    measurement, report = map(pathlib.Path, sys.argv[1:3])
    out = report / "records"
    out.mkdir(parents=True, exist_ok=True)
    members = []
    for path in sorted((measurement / "run" / "runs").glob("*/*")):
        if path.name in KEEP and path.is_file():
            members.append((f"runs/{path.parent.name}/{path.name}", path))
    for name in ("sites.json", "summary.json"):
        members.append((name, measurement / "run" / name))
    members.append(("build.json", measurement / "targets" / "build.json"))
    manifest = {}
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w", format=tarfile.PAX_FORMAT) as archive:
        for name, path in members:
            data = path.read_bytes()
            manifest[name] = hashlib.sha256(data).hexdigest()
            info = tarfile.TarInfo(name)
            info.size = len(data)
            info.mtime = 0
            info.mode = 0o644
            info.uid = info.gid = 0
            info.uname = info.gname = ""
            archive.addfile(info, io.BytesIO(data))
    compressed = io.BytesIO()
    with gzip.GzipFile(fileobj=compressed, mode="wb", mtime=0, compresslevel=9) as stream:
        stream.write(buffer.getvalue())
    (out / "observations.tar.gz").write_bytes(compressed.getvalue())
    (out / "manifest.json").write_text(json.dumps({
        "schema_version": 1,
        "archive_sha256": hashlib.sha256(compressed.getvalue()).hexdigest(),
        "members": manifest,
    }, indent=1, sort_keys=True) + "\n")
    shutil.copyfile(measurement / "aggregate.json", report / "aggregate.json")
    shutil.copyfile(measurement / "targets" / "build.json", report / "build.json")
    print(json.dumps({"members": len(manifest), "archive_bytes": len(compressed.getvalue())}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
