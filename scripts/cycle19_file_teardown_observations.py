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

"""Observe pinned native filesystem construction, teardown and residual custody."""
import argparse
import json
import os
from pathlib import Path
import sys
sys.dont_write_bytecode = True
from cycle19_native_observations import REPO, run, sha
from cycle19_teardown_observations import build
from check_file_teardown_trace import check_trace, load_trace


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fluent-bit", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if sys.platform != "linux": raise RuntimeError("requires the recorded Linux ABI")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    binary, metadata = build(args.fluent_bit.resolve(), output, "chunk_file_teardown_observation",
                             ("close","munmap","unlink","msync"))
    environment = dict(os.environ, ASAN_OPTIONS="detect_leaks=1:halt_on_error=1", UBSAN_OPTIONS="halt_on_error=1:print_stacktrace=1")
    with (output/"trace.jsonl").open("w") as stdout, (output/"native.stderr").open("w") as stderr:
        run([str(binary)],stdout=stdout,stderr=stderr,env=environment)
    observations = check_trace(load_trace(output/"trace.jsonl"))
    for row in observations: row["values"]["compiled_source_and_build_recorded"] = 1
    path = output/"observations.jsonl"
    path.write_text("".join(json.dumps(row)+"\n" for row in observations))
    contract = REPO/"providers/qcore-provider-chunkio/contracts/chunkio/filesystem_teardown_observation.qv"
    checked = run(["cargo","run","--quiet","-p","qcore-compiler","--example","check_contract_observations",
                   "--",str(contract),str(path)],cwd=REPO,capture_output=True)
    (output/"contract_result.json").write_text(checked.stdout)
    metadata.update(filesystem_exercised=True,durability_proved=False,
                    driver_sha256=sha(Path(__file__)),build_driver_sha256=sha(REPO/"scripts/cycle19_teardown_observations.py"),
                    checker_sha256=sha(REPO/"scripts/check_file_teardown_trace.py"),
                    trace_loader_sha256=sha(REPO/"scripts/check_teardown_trace.py"),
                    contract_sha256=sha(contract),trace_sha256=sha(output/"trace.jsonl"),
                    observations_sha256=sha(path),contract_result_sha256=sha(output/"contract_result.json"),
                    scope="six one-stream/one-file Linux executions; three real success paths and three EIO injections suppressing one syscall; OS residuals checked after parent free and cleaned by harness")
    (output/"metadata.json").write_text(json.dumps(metadata,indent=2)+"\n")
    print(checked.stdout.strip())


if __name__ == "__main__": main()
