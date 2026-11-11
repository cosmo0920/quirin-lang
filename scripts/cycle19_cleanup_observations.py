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

"""Execute real pinned ChunkIO cleanup helpers with controlled syscall hooks.

No upstream file is modified and no OS resource is released or file deleted. Sanitized
execution evidence covers only the listed scenarios, not whole-source safety.
"""
import argparse
import json
from pathlib import Path
import sys
sys.dont_write_bytecode = True
from cycle19_native_observations import PIN, REPO, build_and_run, profile, run, sha


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fluent-bit", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root, output = args.fluent_bit.resolve(), args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    revision = run(["git", "-C", str(root), "rev-parse", "HEAD"], capture_output=True).stdout.strip()
    if revision != PIN:
        raise RuntimeError(f"expected {PIN}, found {revision}")
    # Check all tracked sources, not just the directly included helper body.
    if run(["git", "-C", str(root), "status", "--porcelain", "--untracked-files=no"], capture_output=True).stdout:
        raise RuntimeError("pinned source checkout has tracked modifications")
    database = root / "build/compile_commands.json"
    source = root / "lib/chunkio/src/cio_file_unix.c"
    entries = json.loads(database.read_text())
    entry = next(e for e in entries if Path(e["file"]).resolve() == source)
    flags = profile(entry) + [f'-DQCORE_FILE_CLEANUP_SOURCE="{source}"',
                              "-ffunction-sections", "-fdata-sections", "-Wl,--gc-sections"]
    artifact = build_and_run("file_cleanup_observation", entry, flags, [], output)
    if str(source) not in artifact["dependencies_sha256"]:
        raise RuntimeError("compiled harness did not include the actual selected source")
    observations = output / "file_cleanup_observation.jsonl"
    rows = [json.loads(line) for line in observations.read_text().splitlines()]
    expected = {"cio_file_native_close": {0: 1, 1: 1, 2: 3, 3: 3},
                "cio_file_native_unmap": {0: 1, 1: 1, 2: 4, 3: 4}}
    actual = {}
    for row in rows:
        counts = actual.setdefault(row["helper"], {})
        counts[row["mode"]] = counts.get(row["mode"], 0) + 1
    if actual != expected or len({row["case"] for row in rows}) != len(rows):
        raise RuntimeError(f"native scenario coverage mismatch: {actual}")
    contract = REPO / "providers/qcore-provider-chunkio/contracts/chunkio/filesystem_observation.qv"
    checked = run(["cargo", "run", "--quiet", "-p", "qcore-compiler", "--example",
                   "check_contract_observations", "--", str(contract), str(observations)],
                  cwd=REPO, capture_output=True)
    (output / "contract_result.json").write_text(checked.stdout)
    delete_artifact = build_and_run("file_delete_observation", entry, flags, [], output)
    if str(source) not in delete_artifact["dependencies_sha256"]:
        raise RuntimeError("delete harness did not include the actual selected source")
    delete_observations = output / "file_delete_observation.jsonl"
    delete_rows = [json.loads(line) for line in delete_observations.read_text().splitlines()]
    scenarios = {(row["fd"], row["values"]["mapping_active"], row["values"]["injected_result"])
                 for row in delete_rows}
    expected_scenarios = {(fd, mapped, result) for fd in (-1, 0, 7, 2147483647)
                          for mapped in (0, 1) for result in (-1, 0)}
    delete_counts = {mode: sum(row["mode"] == mode for row in delete_rows) for mode in range(5)}
    if (scenarios != expected_scenarios or len(delete_rows) != 16
            or {row["case"] for row in delete_rows} != set(range(16))
            or delete_counts != {0: 6, 1: 6, 2: 2, 3: 1, 4: 1}
            or any(row["function"] != "native_delete_observation"
                   or row["helper"] != "cio_file_native_delete" for row in delete_rows)):
        raise RuntimeError("native delete scenario coverage mismatch")
    delete_contract = REPO / "providers/qcore-provider-chunkio/contracts/chunkio/filesystem_delete_observation.qv"
    delete_checked = run(["cargo", "run", "--quiet", "-p", "qcore-compiler", "--example",
                          "check_contract_observations", "--", str(delete_contract),
                          str(delete_observations)], cwd=REPO, capture_output=True)
    (output / "delete_contract_result.json").write_text(delete_checked.stdout)
    metadata = {"fluent_bit_revision": revision, "database_sha256": sha(database),
                "clang": run(["clang", "--version"], capture_output=True).stdout,
                "native_execution": artifact, "contract_sha256": sha(contract),
                "harness_sha256": sha(REPO / "compiler/examples/file_cleanup_observation.c"),
                "delete_execution": delete_artifact,
                "delete_contract_sha256": sha(delete_contract),
                "delete_harness_sha256": sha(REPO / "compiler/examples/file_delete_observation.c"),
                "delete_scenario_counts": delete_counts,
                "driver_sha256": sha(Path(__file__)),
                "build_helper_sha256": sha(REPO / "scripts/cycle19_native_observations.py"),
                "scenario_counts": actual, "static_certificate": False,
                "os_cleanup_performed": False,
                "outcome_binding": "controlled close/munmap/unlink hooks return injected zero or minus one; diagnostic hook records errno"}
    (output / "metadata.json").write_text(json.dumps(metadata, indent=2) + "\n")
    print(checked.stdout.strip())
    print(delete_checked.stdout.strip())


if __name__ == "__main__":
    main()
