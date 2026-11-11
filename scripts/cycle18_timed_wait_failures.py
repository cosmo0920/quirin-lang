#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Compile and run the bounded source-preserving timed-wait fault harness."""
import argparse, hashlib, json, pathlib, platform, subprocess

REVISION = "66910c10a4d7eafa810b84229cf2dcf7b0f26f97"
SCENARIOS = ["context_allocation_failure", "base_creation_failure",
             "fired_allocation_failure", "timer_creation_failure",
             "timer_registration_failure", "success_observation"]

def digest(path): return hashlib.sha256(path.read_bytes()).hexdigest()

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=pathlib.Path, required=True)
    parser.add_argument("--cc", default="/usr/bin/clang")
    args = parser.parse_args()
    if platform.system() != "Linux": raise SystemExit("Linux fault profile required")
    args.output.mkdir(parents=True, exist_ok=True)
    root = pathlib.Path(__file__).resolve().parents[1]
    source = root / "compiler/examples/cycle18_timed_wait_failures.c"
    header = root / "compiler/examples/cycle18_timed_wait_failures.h"
    wrapper_fixture = root / "compiler/tests/fixtures/monkey/wrappers.c"
    exe = args.output / "cycle18_timed_wait_failures"
    command = [args.cc, "-std=gnu17", "-Wall", "-Wextra", "-Werror",
               "-Wno-unused-variable", "-Wno-sign-compare",
               "-fsanitize=address,undefined", "-g", str(source), "-o", str(exe)]
    subprocess.run(command, check=True)
    import os
    environment = os.environ.copy()
    environment.update({"ASAN_OPTIONS": "detect_leaks=1:halt_on_error=1",
                        "UBSAN_OPTIONS": "halt_on_error=1:print_stacktrace=1"})
    run = subprocess.run([str(exe)], check=True, capture_output=True, env=environment)
    evidence = json.loads(run.stdout)
    if [row["scenario"] for row in evidence] != SCENARIOS: raise SystemExit("scenario set mismatch")
    if any(row["source_revision"] != REVISION for row in evidence): raise SystemExit("revision mismatch")
    evidence_path = args.output / "failure-observations.json"
    evidence_path.write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n")
    manifest = {"schema_version": 1, "status": "bounded_fault_injection",
        "source_revision": REVISION, "host": platform.platform(),
        "compiler": subprocess.check_output([args.cc, "--version"], text=True).splitlines()[0],
        "command": command, "harness": str(source), "harness_sha256": digest(source),
        "harness_header": str(header), "harness_header_sha256": digest(header),
        "wrapper_fixture": str(wrapper_fixture),
        "wrapper_fixture_sha256": digest(wrapper_fixture),
        "executable": str(exe), "executable_sha256": digest(exe),
        "observations": str(evidence_path), "observations_sha256": digest(evidence_path),
        "scope": {"source_body": "compiler/tests/fixtures/monkey/wrappers.c",
                  "dependency_behavior": "stubbed fault model; not a Libevent execution",
                  "faults": SCENARIOS[:-1], "success_is_all_path_safe": False}}
    manifest_path = args.output / "run-manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    print(manifest_path)

if __name__ == "__main__": main()
