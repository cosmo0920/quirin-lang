#!/usr/bin/env python3
"""Exercise aggregate configuration/refusal boundaries against real leaf inputs."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n")


def main():
    binary, selected, output = map(lambda p: Path(p).resolve(), sys.argv[1:])
    output.mkdir(parents=True, exist_ok=False)
    config = json.loads(selected.read_text())
    for key in ["observed_wait", "wait_failures", "event_payload", "timeout_teardown", "policy"]:
        config[key] = str((selected.parent / config[key]).resolve())
    policy = Path(config["policy"])
    original_policy = policy.read_bytes()
    results = []
    for control in ["unknown_field", "substituted_policy", "serialized_report_as_input", "output_alias_policy", "output_alias_leaf"]:
        directory = output / control
        directory.mkdir()
        case = dict(config, report=str(directory / "report.json"))
        if control == "unknown_field":
            case["force_safe"] = True
        elif control == "substituted_policy":
            wrong = directory / "policy.qcore"
            wrong.write_bytes(original_policy + b"\n// substituted policy\n")
            case["policy"] = str(wrong)
        elif control == "serialized_report_as_input":
            fake = directory / "forged-report.json"
            write(fake, {"status": "observed_composition_checked", "verdict": "Safe"})
            case["observed_wait"] = str(fake)
        elif control == "output_alias_policy":
            case["report"] = str(policy)
        else:
            leaf_path = Path(config["timeout_teardown"])
            leaf = json.loads(leaf_path.read_text())
            case["report"] = str((leaf_path.parent / leaf["report"]).resolve())
        destination = Path(case["report"])
        before = destination.read_bytes() if destination.exists() else None
        path = directory / "config.json"
        write(path, case)
        command = [str(binary), "cycle18-composition", str(path)]
        run = subprocess.run(command, capture_output=True, text=True, timeout=600)
        (directory / "run.log").write_text(run.stdout + run.stderr)
        (directory / "run.exit").write_text(str(run.returncode) + "\n")
        after = destination.read_bytes() if destination.exists() else None
        created = before is None and after is not None
        results.append({"control": control, "command": command, "exit": run.returncode,
                        "report_created": created, "destination_unchanged": before == after,
                        "policy_unchanged": policy.read_bytes() == original_policy})
        write(output / "results.json", results)
        assert run.returncode != 0 and not created and before == after, control
        assert policy.read_bytes() == original_policy, control
    write(output / "identity.json", {"binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
                                    "config_sha256": hashlib.sha256(selected.read_bytes()).hexdigest()})
    print("Five aggregate controls refused; protected destinations unchanged.")


if __name__ == "__main__":
    main()
