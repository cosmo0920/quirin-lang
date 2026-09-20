#!/usr/bin/env python3
"""Exercise the configured audit with coherently rebound but invalid evidence.

Usage: script QUIRIN_AUDIT CONFIG.json OUTPUT_DIRECTORY
The input artifacts are never modified; each control gets private copies.
"""
import hashlib
import json
import pathlib
import subprocess
import sys


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n")


def main():
    executable, config_path, output_path = map(pathlib.Path, sys.argv[1:])
    original = json.loads(config_path.read_text())
    output_path.mkdir(parents=True, exist_ok=False)
    cases = ["callback-generation", "missing-record", "invented-synchronization",
             "unsupported-backend", "owned-callback-contract", "artifact-substitution"]
    results = []
    for name in cases:
        directory = output_path / name
        directory.mkdir()
        config = dict(original)
        for key in ["observations", "bindings", "run_manifest", "composition_contract"]:
            source = pathlib.Path(original[key])
            target = directory / source.name
            target.write_bytes(source.read_bytes())
            config[key] = str(target.resolve())
        config["report"] = str((directory / "report.json").resolve())
        observations = pathlib.Path(config["observations"])
        records = [json.loads(line) for line in observations.read_text().splitlines()]
        bindings = json.loads(pathlib.Path(config["bindings"]).read_text())
        manifest = json.loads(pathlib.Path(config["run_manifest"]).read_text())
        if name == "callback-generation":
            records[5]["action"]["event"]["generation"] += 1
        elif name == "missing-record":
            records.pop()
        elif name == "invented-synchronization":
            bindings["synchronization"] = [{"from": 1, "to": 2}]
        elif name == "unsupported-backend":
            records[0]["action"]["backend"] = "unrecognized-backend"
            bindings["context"]["backend"] = "unrecognized-backend"
        elif name == "owned-callback-contract":
            path = pathlib.Path(config["composition_contract"])
            content = path.read_text()
            needle = "borrow<'callback, const, MonkeyCallbackArgument>"
            assert content.count(needle) == 1
            path.write_text(content.replace(needle, "own<MonkeyCallbackArgument>"))
        elif name == "artifact-substitution":
            manifest["observer"]["executable_sha256"] = "0" * 64
        observations.write_text("".join(json.dumps(record) + "\n" for record in records))
        bindings["observations"]["sha256"] = digest(observations)
        write(pathlib.Path(config["bindings"]), bindings)
        manifest["outputs"]["observations_sha256"] = digest(observations)
        manifest["outputs"]["bindings_sha256"] = digest(pathlib.Path(config["bindings"]))
        write(pathlib.Path(config["run_manifest"]), manifest)
        configured = directory / "config.json"
        write(configured, config)
        command = [str(executable.resolve()), "observed-wait", str(configured.resolve())]
        completed = subprocess.run(command, capture_output=True, text=True, timeout=120)
        log = directory / "audit.log"
        log.write_text(completed.stdout + completed.stderr)
        row = {"control": name, "command": command, "exit": completed.returncode,
               "log": str(log.resolve()), "log_sha256": digest(log),
               "report_created": pathlib.Path(config["report"]).exists()}
        results.append(row)
        write(output_path / "results.json", results)
        if completed.returncode != 1 or row["report_created"]:
            raise SystemExit(f"negative control was not refused: {name}")
    print(f"{len(results)} invalid evidence controls refused; no reports issued")


if __name__ == "__main__":
    main()
