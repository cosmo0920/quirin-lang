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

"""Run real Linux cleanup syscalls in exclusive, disposable /tmp fixtures."""
import argparse
import json
from pathlib import Path
import sys
sys.dont_write_bytecode = True
from cycle19_native_observations import PIN, REPO, build_and_run, profile, run, sha
from check_file_os_trace import check_trace, load_trace


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fluent-bit", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root, output = args.fluent_bit.resolve(), args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    if sys.platform != "linux":
        raise RuntimeError("OS endpoint profile requires Linux")
    if run(["git", "-C", str(root), "rev-parse", "HEAD"], capture_output=True).stdout.strip() != PIN:
        raise RuntimeError("wrong Fluent Bit source revision")
    if run(["git", "-C", str(root), "status", "--porcelain", "--untracked-files=no"], capture_output=True).stdout:
        raise RuntimeError("modified pinned sources")
    database = root / "build/compile_commands.json"
    source = root / "lib/chunkio/src/cio_file_unix.c"
    entries = [e for e in json.loads(database.read_text()) if Path(e["file"]).resolve() == source]
    if len(entries) != 1:
        raise RuntimeError("missing/duplicate Unix cleanup compilation entry")
    flags = profile(entries[0]) + [f'-DQCORE_FILE_CLEANUP_SOURCE="{source}"',
                                 "-ffunction-sections", "-fdata-sections", "-Wl,--gc-sections"]
    artifact = build_and_run("file_os_observation", entries[0], flags, [], output)
    if str(source) not in artifact["dependencies_sha256"]:
        raise RuntimeError("actual source missing from compiled dependency set")
    observations = check_trace(load_trace(output / "file_os_observation.jsonl"))
    for observation in observations:
        observation["values"]["compiled_source_and_build_recorded"] = 1
    path = output / "observations.jsonl"
    path.write_text("".join(json.dumps(row) + "\n" for row in observations))
    contract = REPO / "providers/qcore-provider-chunkio/contracts/chunkio/filesystem_os_observation.qv"
    checked = run(["cargo", "run", "--quiet", "-p", "qcore-compiler", "--example",
                   "check_contract_observations", "--", str(contract), str(path)], cwd=REPO, capture_output=True)
    (output / "contract_result.json").write_text(checked.stdout)
    metadata = {"schema_version": 1, "fluent_bit_revision": PIN, "database_sha256": sha(database),
                "clang": run(["clang", "--version"], capture_output=True).stdout,
                "native_execution": artifact, "binary_sha256": sha(output / "file_os_observation"),
                "driver_sha256": sha(Path(__file__)),
                "checker_sha256": sha(REPO / "scripts/check_file_os_trace.py"),
                "trace_loader_sha256": sha(REPO / "scripts/check_teardown_trace.py"),
                "build_helper_sha256": sha(REPO / "scripts/cycle19_native_observations.py"),
                "contract_sha256": sha(contract), "observations_sha256": sha(path),
                "contract_result_sha256": sha(output / "contract_result.json"),
                "os_cleanup_performed": True, "static_certificate": False, "durability_proved": False,
                "scope": "12 exclusive Linux close/unmap/delete helper calls; fresh one-page fixtures with inode hardlink witnesses; no parent teardown or concurrent resource reuse"}
    (output / "metadata.json").write_text(json.dumps(metadata, indent=2) + "\n")
    print(checked.stdout.strip())


if __name__ == "__main__":
    main()
