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

"""Observe pinned native ChunkIO constructors and memory teardown under sanitizers.

All selected ChunkIO/CRC translation units are rebuilt with the pinned compilation
profile. The upstream checkout is read-only. Output contains trusted observer
records, not static safety certificates or exhaustive native caller coverage.
"""
import argparse
import json
import os
from pathlib import Path
import shlex
import subprocess
import sys
sys.dont_write_bytecode = True
from cycle19_native_observations import PIN, REPO, profile, run, sha
from check_teardown_trace import check_trace, load_trace


def build(root, output, observer="chunk_teardown_observation", syscall_wrappers=()):
    revision = run(["git", "-C", str(root), "rev-parse", "HEAD"], capture_output=True).stdout.strip()
    if revision != PIN or run(["git", "-C", str(root), "status", "--porcelain", "--untracked-files=no"], capture_output=True).stdout:
        raise RuntimeError("native teardown requires the clean pinned Fluent Bit checkout")
    database = root / "build/compile_commands.json"
    expected = {"cio_os.c", "cio_log.c", "cio_file.c", "cio_memfs.c", "cio_chunk.c", "cio_meta.c", "cio_scan.c", "cio_utils.c", "cio_stream.c", "cio_stats.c", "cio_error.c", "chunkio.c", "cio_file_unix.c", "crc32.c"}
    entries = [e for e in json.loads(database.read_text()) if Path(e["file"]).is_relative_to(root / "lib/chunkio") and Path(e["file"]).name in expected]
    if len(entries) != 14 or {Path(e["file"]).name for e in entries} != expected:
        raise RuntimeError("incomplete or duplicate ChunkIO compilation profile")
    commands, objects, dependencies = [], [], {}
    sanitizer = ["-UNDEBUG", "-g", "-O0", "-fsanitize=address,undefined", "-fno-omit-frame-pointer", "-fno-builtin", "-Werror=implicit-function-declaration"]
    for entry in entries:
        obj = output / (Path(entry["file"]).stem + ".o")
        dep = obj.with_suffix(".d")
        command = ["clang", *profile(entry), *sanitizer, "-finstrument-functions", "-MMD", "-MF", str(dep), "-c", entry["file"], "-o", str(obj)]
        run(command, cwd=entry["directory"], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        commands.append(command); objects.append(obj)
        paths = shlex.split(dep.read_text().replace("\\\n", "").split(":", 1)[1])
        for name in paths:
            path = Path(name)
            if not path.is_absolute(): path = Path(entry["directory"]) / path
            dependencies[str(path.resolve())] = sha(path)
        if str(Path(entry["file"]).resolve()) not in dependencies:
            raise RuntimeError("compiled dependency list omitted selected source")
    source = REPO / "compiler/examples" / (observer + ".c")
    binary = output / observer
    command = ["clang", *profile(entries[0]), *sanitizer, str(source), *map(str, objects), "-Wl,--wrap=malloc,--wrap=calloc,--wrap=strdup,--wrap=realloc,--wrap=free", "-lpthread", "-o", str(binary)]
    command[1:1] = ["-Wl,--wrap=" + name for name in syscall_wrappers]
    dep = output / "observer.d"
    command[1:1] = ["-MMD", "-MF", str(dep)]
    run(command, capture_output=True); commands.append(command)
    for name in shlex.split(dep.read_text().replace("\\\n", "").split(":", 1)[1]):
        path = Path(name).resolve()
        dependencies[str(path)] = sha(path)
    return binary, {"schema_version": 1, "fluent_bit_revision": revision, "database_sha256": sha(database), "compile_entries": entries, "commands": commands, "dependencies_sha256": dependencies, "harness_sha256": sha(source), "driver_sha256": sha(Path(__file__)), "build_helper_sha256": sha(REPO / "scripts/cycle19_native_observations.py"), "binary_sha256": sha(binary), "clang":run(["clang","--version"],capture_output=True).stdout, "instrumentation":"all 14 selected ChunkIO/CRC units and observer rebuilt with ASan/UBSan; libc allocation wrappers and compiler function hooks; system libraries retain their build", "static_certificate":False, "filesystem_exercised":False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fluent-bit", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root, output = args.fluent_bit.resolve(), args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    binary, metadata = build(root, output)
    trace = output / "trace.jsonl"
    environment = dict(os.environ, ASAN_OPTIONS="detect_leaks=1:halt_on_error=1", UBSAN_OPTIONS="halt_on_error=1:print_stacktrace=1")
    with trace.open("w") as stdout, (output/"native.stderr").open("w") as stderr:
        run([str(binary)], stdout=stdout, stderr=stderr, env=environment)
    observations = check_trace(load_trace(trace))
    for observation in observations:
        observation["values"].update(compiled_source_and_build_recorded=1, exclusive_instrumented_execution=1)
    observations_path = output / "observations.jsonl"
    observations_path.write_text("".join(json.dumps(row) + "\n" for row in observations))
    contract = REPO / "providers/qcore-provider-chunkio/contracts/chunkio/teardown_observation.qv"
    checked = run(["cargo", "run", "--quiet", "-p", "qcore-compiler", "--example", "check_contract_observations", "--", str(contract), str(observations_path)], cwd=REPO, capture_output=True)
    (output / "contract_result.json").write_text(checked.stdout)
    metadata["trace_checker_sha256"] = sha(REPO / "scripts/check_teardown_trace.py")
    metadata["contract_sha256"] = sha(contract)
    metadata["observations_sha256"] = sha(observations_path)
    metadata["contract_result_sha256"] = sha(output / "contract_result.json")
    metadata["trace_sha256"] = sha(trace)
    (output/"metadata.json").write_text(json.dumps(metadata,indent=2)+"\n")
    print(json.dumps({"trace":str(trace),"static_certificate":False}))


if __name__ == "__main__":
    main()
