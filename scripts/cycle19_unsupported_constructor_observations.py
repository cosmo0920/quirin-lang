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

"""Observe four unsupported stream-type constructor inputs with sanitizer leak checks."""
import argparse
import json
import os
from pathlib import Path
import sys
sys.dont_write_bytecode = True
from cycle19_teardown_observations import build
from cycle19_native_observations import REPO, run, sha
from check_teardown_trace import load_trace
from check_unsupported_constructor_trace import check_trace


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fluent-bit", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root, output = args.fluent_bit.resolve(), args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    binary, metadata = build(root, output, "chunk_unsupported_constructor_observation")
    trace = output / "trace.jsonl"
    environment = dict(os.environ, ASAN_OPTIONS="detect_leaks=1:halt_on_error=1", UBSAN_OPTIONS="halt_on_error=1:print_stacktrace=1")
    with trace.open("w") as stdout, (output / "native.stderr").open("w") as stderr:
        run([str(binary)], stdout=stdout, stderr=stderr, env=environment)
    observations = check_trace(load_trace(trace))
    for observation in observations:
        observation["values"].update(compiled_source_and_build_recorded=1, exclusive_instrumented_execution=1)
    observations_path = output / "observations.jsonl"
    observations_path.write_text("".join(json.dumps(row) + "\n" for row in observations))
    contract = REPO / "providers/qcore-provider-chunkio/contracts/chunkio/unsupported_constructor_observation.qv"
    checked = run(["cargo", "run", "--quiet", "-p", "qcore-compiler", "--example", "check_contract_observations", "--", str(contract), str(observations_path)], cwd=REPO, capture_output=True)
    (output / "contract_result.json").write_text(checked.stdout)
    for key, path in {
        "unsupported_driver": Path(__file__), "trace_checker": REPO / "scripts/check_unsupported_constructor_trace.py",
        "tree_checker": REPO / "scripts/check_teardown_trace.py", "contract": contract,
        "trace": trace, "observations": observations_path, "contract_result": output / "contract_result.json",
    }.items():
        metadata[key + "_sha256"] = sha(path)
    metadata["constructor_profile"] = "four 32-bit int inputs: INT_MIN, -1, 2, INT_MAX; no injected failures; empty surviving streams"
    metadata["outcome_boundaries"] = ["stream preserves unsupported type", "chunk open returns null and preserves the error sentinel", "two constructor allocations roll back before surviving-tree teardown", "four observations do not prove general reachability"]
    (output / "metadata.json").write_text(json.dumps(metadata, indent=2) + "\n")
    print(json.dumps({"trace": str(trace), "checked_executions": len(observations), "static_certificate": False}))


if __name__ == "__main__":
    main()
