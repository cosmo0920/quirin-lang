#!/usr/bin/env python3
"""Compare the three prior bounded CLI reports without normalizing identities.

Usage: check_existing_routes.py BASELINE CURRENT CONFIG_MAP OUTPUT
CONFIG_MAP maps each command to its existing configuration path. Evidence files
must remain available. Every subprocess exit and complete report is retained.
"""
import hashlib
import json
from pathlib import Path
import subprocess
import sys


def main():
    baseline, current, config_map, output = map(Path, sys.argv[1:])
    output = output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    selected = json.loads(config_map.read_text())
    assert set(selected) == {"wait-failures", "event-payload", "timeout-teardown"}
    results = []
    for command, config_path in selected.items():
        config = json.loads(Path(config_path).read_text())
        row = {"command": command, "runs": []}
        reports = []
        for label, binary in [("baseline", baseline), ("current", current)]:
            directory = output / command / label
            directory.mkdir(parents=True)
            report = directory / "report.json"
            configured = dict(config, report=str(report))
            path = directory / "config.json"
            path.write_text(json.dumps(configured, indent=2) + "\n")
            argv = [str(binary.resolve()), command, str(path)]
            run = subprocess.run(argv, capture_output=True, text=True, timeout=180)
            (directory / "run.log").write_text(run.stdout + run.stderr)
            (directory / "run.exit").write_text(str(run.returncode) + "\n")
            row["runs"].append({"argv": argv, "exit": run.returncode,
                "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest()})
            assert run.returncode == 0, f"{command}/{label} failed"
            reports.append(report.read_bytes())
        row["byte_identical"] = reports[0] == reports[1]
        row["report_sha256"] = [hashlib.sha256(x).hexdigest() for x in reports]
        results.append(row)
        (output / "results.json").write_text(json.dumps(results, indent=2) + "\n")
        assert row["byte_identical"], f"{command} changed its report"
    print("All three existing bounded reports remain byte-identical.")


if __name__ == "__main__":
    main()
