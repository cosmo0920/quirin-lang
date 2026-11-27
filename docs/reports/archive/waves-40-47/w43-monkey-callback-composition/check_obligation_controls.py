#!/usr/bin/env python3
"""Configured negative controls for `quirin-audit cycle18-obligations`.

Usage: check_obligation_controls.py BINARY BASE_CONFIG OUTPUT_DIRECTORY

Each control writes one changed copy of BASE_CONFIG into OUTPUT_DIRECTORY,
runs BINARY on it, and keeps the complete output and exit status. A refusing
control must exit 1 and must not create its report. Baseline runs before and
after the controls must exit 0 and create their report. The summary is
written to OUTPUT_DIRECTORY/controls.json; nothing else is written.
"""

import hashlib
import json
import os
import subprocess
import sys

binary, base_path, output = (os.path.abspath(argument) for argument in sys.argv[1:4])
os.makedirs(output, exist_ok=False)
base = json.load(open(base_path))
payload_database = (
    "/home/cosmo/Documents/GitHub/qcore-c/private/cycle18-priority1/"
    "native-payload/output-6/source-compile-commands.json"
)
composition = json.load(open(base["composition"]))


def control(name, change, expect_success=False):
    config = json.loads(json.dumps(base))
    config["report"] = os.path.join(output, name + "-report.json")
    change(config)
    config_path = os.path.join(output, name + "-config.json")
    with open(config_path, "w") as handle:
        json.dump(config, handle, indent=2)
        handle.write("\n")
    report = config["report"]
    report_existed = os.path.exists(report)
    completed = subprocess.run(
        [binary, "cycle18-obligations", config_path],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    log = os.path.join(output, name + ".log")
    with open(log, "wb") as handle:
        handle.write(completed.stdout)
    created = not report_existed and os.path.exists(report)
    passed = (
        completed.returncode == 0 and created
        if expect_success
        else completed.returncode == 1 and not created
    )
    return {
        "control": name,
        "expect": "accept" if expect_success else "refuse",
        "exit": completed.returncode,
        "report_created": created,
        "passed": passed,
        "log": log,
        "output": completed.stdout.decode(errors="replace").strip()[-400:],
    }


def set_field(key, value):
    def change(config):
        config[key] = value

    return change


def alias_to(path_function):
    def change(config):
        config["report"] = path_function(config)

    return change


results = [
    control("m0-baseline", lambda config: None, expect_success=True),
    control("m1-unknown-field", set_field("discharged", ["registration_liveness"])),
    control("m2-schema-version", set_field("schema_version", 2)),
    control("m3-foreign-compile-database", set_field("source_compile_database", payload_database)),
    control("m4-report-aliases-aggregate-report", alias_to(lambda config: composition["report"])),
    control(
        "m5-report-aliases-own-config",
        alias_to(lambda config: os.path.join(output, "m5-report-aliases-own-config-config.json")),
    ),
    control(
        "m6-report-aliases-leaf-report",
        alias_to(lambda config: json.load(open(composition["timeout_teardown"]))["report"]),
    ),
    control("m7-baseline-after", lambda config: None, expect_success=True),
]
baselines = [
    hashlib.sha256(open(os.path.join(output, name + "-report.json"), "rb").read()).hexdigest()
    for name in ["m0-baseline", "m7-baseline-after"]
    if os.path.exists(os.path.join(output, name + "-report.json"))
]
summary = {
    "binary": binary,
    "binary_sha256": hashlib.sha256(open(binary, "rb").read()).hexdigest(),
    "base_config": base_path,
    "controls": results,
    "passed": sum(result["passed"] for result in results),
    "total": len(results),
    "baseline_reports_sha256": baselines,
}
with open(os.path.join(output, "controls.json"), "w") as handle:
    json.dump(summary, handle, indent=2)
    handle.write("\n")
print(json.dumps({"passed": summary["passed"], "total": summary["total"]}))
sys.exit(0 if summary["passed"] == summary["total"] else 1)
