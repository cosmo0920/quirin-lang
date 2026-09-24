#!/usr/bin/env python3
"""Re-check the pinned descriptor observations without rerunning any target.

Usage: verify_records.py REPORT_DIR SCRATCH_DIR

Extracts REPORT_DIR/records/observations.tar.gz into SCRATCH_DIR (which must
not exist), checks every member against manifest.json, replays every pinned
records.jsonl with the current `check_descriptor_observations` under the
current repository policy, and requires:
  * each accepted run's replay to reproduce its pinned report.json byte for
    byte, and each rejected run to be rejected again;
  * aggregate.py over the replayed reports to reproduce the pinned
    aggregate.json byte for byte.
Exit 0 only if all of that holds. A policy or checker change that moves any
result makes this fail; that is its purpose.
"""
import hashlib
import json
import pathlib
import subprocess
import sys
import tarfile

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
POLICY = REPO / "providers/quirin-provider-shared/contracts/library/descriptor_native_observation.qcore"
CHECKER = REPO / "target/release/examples/check_descriptor_observations"


def main() -> int:
    report, scratch = pathlib.Path(sys.argv[1]).resolve(), pathlib.Path(sys.argv[2]).resolve()
    scratch.mkdir(parents=True, exist_ok=False)
    manifest = json.loads((report / "records/manifest.json").read_text())
    archive = report / "records/observations.tar.gz"
    if hashlib.sha256(archive.read_bytes()).hexdigest() != manifest["archive_sha256"]:
        raise SystemExit("archive digest differs from manifest")
    with tarfile.open(archive) as members:
        members.extractall(scratch, filter="data")
    for name, digest in manifest["members"].items():
        if hashlib.sha256((scratch / name).read_bytes()).hexdigest() != digest:
            raise SystemExit(f"member digest differs: {name}")
    summary = json.loads((scratch / "summary.json").read_text())
    replayed = scratch / "replay"
    failures = []
    for run in summary["runs"]:
        name = run["name"]
        records = scratch / "runs" / name / "records.jsonl"
        checked = subprocess.run([str(CHECKER), str(POLICY), str(records)], text=True, capture_output=True)
        pinned = scratch / "runs" / name / "report.json"
        if run["checker_exit"] == 0:
            if checked.returncode != 0 or checked.stdout != pinned.read_text():
                failures.append(f"{name}: replay differs from the pinned report")
        elif checked.returncode == 0:
            failures.append(f"{name}: a pinned rejection is now accepted")
        target = replayed / "runs" / name
        target.mkdir(parents=True)
        if checked.returncode == 0:
            (target / "report.json").write_text(checked.stdout)
    (replayed / "summary.json").write_text((scratch / "summary.json").read_text())
    (replayed / "sites.json").write_text((scratch / "sites.json").read_text())
    subprocess.run([sys.executable, str(HERE / "aggregate.py"), str(replayed), str(replayed / "aggregate.json")],
                   check=True, capture_output=True)
    if (replayed / "aggregate.json").read_text() != (report / "aggregate.json").read_text():
        failures.append("aggregate differs from the pinned aggregate.json")
    print(json.dumps({"runs": len(summary["runs"]), "failures": failures}, indent=1))
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
