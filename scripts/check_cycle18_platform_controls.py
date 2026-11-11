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

"""Run rehashed semantic and artifact-role controls for the platform audit."""

import argparse
import copy
import hashlib
import json
import pathlib
import subprocess
import tempfile


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def absolute_config(path):
    base = path.parent
    value = json.loads(path.read_text())
    for key in ["matrix", "policy", "harness", "historical_validation", "libevent_source", "report"]:
        selected = pathlib.Path(value[key])
        value[key] = str(selected if selected.is_absolute() else (base / selected).resolve())
    for artifact_set in value["artifact_sets"]:
        for key in ["build_commands", "cmake_cache", "compile_commands", "libevent_core", "libevent_pthreads", "executable", "run_manifest"]:
            selected = pathlib.Path(artifact_set[key])
            artifact_set[key] = str(selected if selected.is_absolute() else (base / selected).resolve())
        artifact_set["runtime_records"] = [str(pathlib.Path(item) if pathlib.Path(item).is_absolute() else (base / item).resolve()) for item in artifact_set["runtime_records"]]
    return value


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", type=pathlib.Path, required=True)
    parser.add_argument("--quirin-audit", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    args = parser.parse_args()
    config_path = args.config.resolve()
    baseline = absolute_config(config_path)
    matrix = json.loads(pathlib.Path(baseline["matrix"]).read_text())
    results = []
    with tempfile.TemporaryDirectory(prefix="qcore-cycle18-platform-controls-") as directory:
        directory = pathlib.Path(directory)
        cases = []

        swapped = copy.deepcopy(baseline)
        selected = swapped["artifact_sets"][0]
        selected["libevent_core"], selected["libevent_pthreads"] = selected["libevent_pthreads"], selected["libevent_core"]
        manifest = json.loads(pathlib.Path(selected["run_manifest"]).read_text())
        manifest["artifacts_sha256"]["libevent_core"] = digest(pathlib.Path(selected["libevent_core"]))
        manifest["artifacts_sha256"]["libevent_pthreads"] = digest(pathlib.Path(selected["libevent_pthreads"]))
        manifest_path = directory / "role-swap-manifest.json"
        write(manifest_path, manifest)
        selected["run_manifest"] = str(manifest_path)
        cases.append(("rehashed_core_pthreads_role_swap", swapped))

        compile_role = copy.deepcopy(baseline)
        selected = compile_role["artifact_sets"][0]
        compile_commands = json.loads(pathlib.Path(selected["compile_commands"]).read_text())
        core_entry = next(
            entry for entry in compile_commands
            if entry["output"] == "CMakeFiles/event_core_static.dir/event.c.o"
        )
        core_entry["output"] = "CMakeFiles/event_static.dir/event.c.o"
        core_entry["command"] = core_entry["command"].replace(
            "CMakeFiles/event_core_static.dir/event.c.o",
            "CMakeFiles/event_static.dir/event.c.o",
        )
        compile_path = directory / "relabelled-compile-commands.json"
        write(compile_path, compile_commands)
        selected["compile_commands"] = str(compile_path)
        manifest = json.loads(pathlib.Path(selected["run_manifest"]).read_text())
        manifest["artifacts_sha256"]["compile_commands"] = digest(compile_path)
        manifest_path = directory / "compile-role-manifest.json"
        write(manifest_path, manifest)
        selected["run_manifest"] = str(manifest_path)
        cases.append(("rehashed_compile_archive_role_relabel", compile_role))

        same_thread = copy.deepcopy(baseline)
        same_matrix = copy.deepcopy(matrix)
        row = next(row for row in same_matrix["rows"] if row["policy"] == "linux_epoll_asan_profile")
        row["evidence"]["runtime"]["activating_thread"] = row["evidence"]["runtime"]["dispatch_thread"]
        selected = same_thread["artifact_sets"][0]
        runtime_index = next(index for index, item in enumerate(selected["runtime_records"]) if pathlib.Path(item).name == "runtime-epoll.json")
        runtime = json.loads(pathlib.Path(selected["runtime_records"][runtime_index]).read_text())
        runtime["activating_thread"] = runtime["dispatch_thread"]
        runtime_path = directory / "same-thread-runtime.json"
        write(runtime_path, runtime)
        selected["runtime_records"][runtime_index] = str(runtime_path)
        manifest = json.loads(pathlib.Path(selected["run_manifest"]).read_text())
        manifest["runtime_sha256"]["epoll"] = digest(runtime_path)
        manifest_path = directory / "same-thread-manifest.json"
        write(manifest_path, manifest)
        selected["run_manifest"] = str(manifest_path)
        matrix_path = directory / "same-thread-matrix.json"
        write(matrix_path, same_matrix)
        same_thread["matrix"] = str(matrix_path)
        cases.append(("rehashed_same_thread_identity", same_thread))

        transparent = copy.deepcopy(baseline)
        transparent_matrix = copy.deepcopy(matrix)
        transparent_matrix["rows"][0]["evidence"]["observer_transparency"] = "transparent"
        matrix_path = directory / "transparent-matrix.json"
        write(matrix_path, transparent_matrix)
        transparent["matrix"] = str(matrix_path)
        cases.append(("observer_transparency_assertion", transparent))

        relabelled = copy.deepcopy(baseline)
        relabelled_matrix = copy.deepcopy(matrix)
        relabelled_matrix["artifact"] = "monkey/callback-composition"
        matrix_path = directory / "relabelled-artifact-matrix.json"
        write(matrix_path, relabelled_matrix)
        relabelled["matrix"] = str(matrix_path)
        cases.append(("generic_probe_relabelled_as_monkey", relabelled))

        for name, config in cases:
            config["report"] = str(directory / f"{name}-report.json")
            mutated = directory / f"{name}.json"
            write(mutated, config)
            completed = subprocess.run([str(args.quirin_audit.resolve()), "native-platform-matrix", str(mutated)], text=True, capture_output=True)
            results.append({"name": name, "exit": completed.returncode, "refused": completed.returncode != 0, "stderr": completed.stderr.strip()})
    if not all(result["refused"] for result in results):
        raise SystemExit("one or more negative controls were admitted")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    write(args.output, {"schema_version": 1, "controls": results})
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
