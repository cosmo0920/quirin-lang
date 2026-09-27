#!/usr/bin/env python3
"""Build and measure an integration stage once, preserving commands and exits.

Run only in the granted audit slot, never alongside the standing suite.
Usage: measure.py LABEL [--jobs N]
"""
import argparse
import datetime
import json
import pathlib
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("label")
    parser.add_argument("--jobs", type=int, default=8)
    args = parser.parse_args()
    if not args.label.replace("-", "").isalnum() or args.jobs < 1:
        raise SystemExit("invalid stage/jobs")
    root = pathlib.Path(__file__).resolve().parents[4]
    output = root / "private/w46-next" / args.label
    output.mkdir(parents=True, exist_ok=False)
    revision = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
    commands = [
        ("build-corpus", ["cargo", "build", "--release", "-p", "qcore-compiler",
                          "--example", "cycle19_fluent_bit_libraries_audit"]),
        ("build-system", ["cargo", "build", "--release", "-p", "qcore-audit",
                          "--bin", "quirin-audit"]),
        ("corpus", ["python3", "docs/reports/w43-return-expression/scripts/run_audit.py",
                    str(output / "corpus"), revision, "--jobs", str(args.jobs)]),
        ("system", ["python3", "docs/reports/w46-contract-gap-expansion/scripts/run_system.py",
                    str(output / "system"), "--jobs", str(min(args.jobs, 4))]),
    ]
    record = {"revision": revision, "stage": args.label, "steps": []}
    for name, command in commands:
        start = datetime.datetime.now(datetime.timezone.utc).isoformat()
        with (output / (name + ".log")).open("wb") as stream:
            result = subprocess.run(command, cwd=root, stdout=stream, stderr=subprocess.STDOUT)
        (output / (name + ".exit")).write_text(str(result.returncode) + "\n")
        record["steps"].append({
            "name": name, "command": command, "exit": result.returncode, "start": start,
            "end": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        })
        (output / "steps.json").write_text(json.dumps(record, indent=2) + "\n")
        print(name, result.returncode, flush=True)
        if result.returncode:
            raise SystemExit(result.returncode)


if __name__ == "__main__":
    main()
