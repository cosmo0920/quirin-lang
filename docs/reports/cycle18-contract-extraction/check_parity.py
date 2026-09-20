#!/usr/bin/env python3
"""Compare bounded CLI outcomes before and after executable policy extraction.

Arguments: BASELINE_EXECUTABLE CURRENT_EXECUTABLE CONFIG_JSON OUTPUT_DIRECTORY.
Runs one valid configured audit and the six frozen invalid-evidence controls
with each binary. Different policy/context identities are recorded separately
from the unchanged conditional result and selected lifecycle.
"""
import hashlib
import json
import pathlib
import subprocess
import sys


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n")


def main():
    baseline, current, configured, out = [pathlib.Path(x).resolve() for x in sys.argv[1:]]
    root = pathlib.Path(__file__).resolve().parents[3]
    config = json.loads(configured.read_text())
    out.mkdir(parents=True, exist_ok=False)
    evidence = {}
    reports = {}
    for label, executable in [("baseline", baseline), ("extracted", current)]:
        directory = out / label
        directory.mkdir()
        selected = dict(config)
        selected["report"] = str(directory / "report.json")
        write(directory / "config.json", selected)
        command = [str(executable), "observed-wait", str(directory / "config.json")]
        run = subprocess.run(command, capture_output=True, text=True, timeout=120)
        (directory / "audit.log").write_text(run.stdout + run.stderr)
        assert run.returncode == 0, f"{label}: positive audit failed"
        reports[label] = json.loads((directory / "report.json").read_text())
        negatives = [sys.executable, str(root / "docs/reports/cycle18-observed-integration/check_negative_controls.py"), str(executable), str(directory / "config.json"), str(directory / "negative-controls")]
        run_negatives = subprocess.run(negatives, capture_output=True, text=True, timeout=900)
        (directory / "negative-controls.log").write_text(run_negatives.stdout + run_negatives.stderr)
        assert run_negatives.returncode == 0, f"{label}: negative harness failed"
        rows = json.loads((directory / "negative-controls/results.json").read_text())
        evidence[label] = {"binary": str(executable), "binary_sha256": sha(executable), "command": command, "exit": run.returncode, "report_sha256": sha(directory / "report.json"), "negative_runner_command": negatives, "negative_runner_exit": run_negatives.returncode, "negative_outcomes": [{k: row[k] for k in ["control", "exit", "report_created"]} for row in rows]}
    before, after = reports["baseline"], reports["extracted"]
    assert before["status"] == after["status"] == "observed_composition_checked"
    fields = ["claim", "checked_records", "checked_operations", "selected_paths", "identity_mapping"]
    for key in fields:
        assert before["composition"][key] == after["composition"][key], key
    assert evidence["baseline"]["negative_outcomes"] == evidence["extracted"]["negative_outcomes"]
    differences = {key: {"before": before["context"].get(key), "after": after["context"].get(key)} for key in before["context"].keys() | after["context"].keys() if before["context"].get(key) != after["context"].get(key)}
    allowed = {"assumptions", "contracts_sha256", "observation_rules_sha256"}
    assert set(differences) <= allowed, f"unexpected context delta: {set(differences) - allowed}"
    evidence["comparison"] = {"status_identical": True, "composition_fields_identical": fields, "negative_outcomes_identical": True, "context_differences": differences, "note": "Policy digests and their aggregate/context identities are expected to change; they do not represent changed lifecycle results or expanded coverage."}
    write(out / "parity.json", evidence)
    print("One positive and six negative CLI outcomes agree; lifecycle and identity mapping agree.")


if __name__ == "__main__":
    main()
