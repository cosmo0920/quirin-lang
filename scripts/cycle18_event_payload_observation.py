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

"""Build and run the bounded Cycle 18 cb_event payload observer on Linux."""

import argparse
import hashlib
import json
import os
import pathlib
import platform
import subprocess

LIBEVENT_REVISION = "d82464a277d0f42703702c4dfd9af6af38595a83"
MONKEY_REVISION = "66910c10a4d7eafa810b84229cf2dcf7b0f26f97"


def run(command, *, cwd=None, env=None, stdout=None, timeout=300):
    subprocess.run(command, cwd=cwd, env=env, stdout=stdout, check=True,
                   timeout=timeout)


def output(command, *, cwd=None):
    return subprocess.check_output(command, cwd=cwd, text=True,
                                   timeout=30).strip()


def digest_bytes(value):
    return hashlib.sha256(value).hexdigest()


def digest(path):
    return digest_bytes(path.read_bytes())


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--monkey-inputs", type=pathlib.Path, required=True)
    parser.add_argument("--monkey-source", type=pathlib.Path, required=True)
    parser.add_argument("--monkey-build", type=pathlib.Path, required=True)
    parser.add_argument("--libevent-source", type=pathlib.Path, required=True)
    parser.add_argument("--libevent-build", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    parser.add_argument("--cc", default="/usr/bin/clang")
    args = parser.parse_args()
    for name in ["monkey_inputs", "monkey_source", "monkey_build", "libevent_source", "libevent_build", "output"]:
        setattr(args, name, getattr(args, name).resolve())

    if platform.system() != "Linux":
        raise SystemExit("the first cb_event payload observer profile supports Linux only")
    args.output.mkdir(parents=True, exist_ok=True)
    inputs = json.loads(args.monkey_inputs.read_text())
    if inputs.get("schema_version") != 1 or inputs.get("source_revision") != MONKEY_REVISION:
        raise SystemExit("Monkey input identity does not match the pinned source revision")
    event_source = args.monkey_source / "mk_core/mk_event.c"
    backend_source = args.monkey_source / "mk_core/mk_event_libevent.c"
    for relative, path in [("mk_core/mk_event.c", event_source),
                           ("mk_core/mk_event_libevent.c", backend_source)]:
        if digest(path) != inputs["source_sha256"][relative]:
            raise SystemExit(f"Monkey source digest mismatch: {relative}")
    revision = output(["git", "rev-parse", "HEAD"], cwd=args.libevent_source)
    if revision != LIBEVENT_REVISION:
        raise SystemExit("Libevent checkout is not the Cycle 18 pinned revision")

    sanitizer = "-fsanitize=address,undefined"
    cmake_command = [
        "cmake", "-S", str(args.libevent_source), "-B", str(args.libevent_build),
        "-DCMAKE_BUILD_TYPE=Debug", "-DEVENT__LIBRARY_TYPE=STATIC",
        "-DEVENT__DISABLE_OPENSSL=ON", "-DEVENT__DISABLE_MBEDTLS=ON",
        "-DEVENT__DISABLE_TESTS=ON", "-DEVENT__DISABLE_REGRESS=ON",
        "-DEVENT__DISABLE_BENCHMARK=ON", "-DEVENT__DISABLE_SAMPLES=ON",
        f"-DCMAKE_C_COMPILER={args.cc}", f"-DCMAKE_C_FLAGS={sanitizer}",
        f"-DCMAKE_EXE_LINKER_FLAGS={sanitizer}",
    ]
    run(cmake_command)
    run(["cmake", "--build", str(args.libevent_build), "--target", "event_core_static", "-j2"])
    library = args.libevent_build / "lib/libevent_core.a"
    if not library.is_file():
        raise SystemExit("pinned Libevent static archive was not produced")

    includes = [
        args.libevent_build / "include", args.libevent_source / "include",
        args.monkey_build / "include/monkey", args.monkey_build / "include",
        args.monkey_source / "include/monkey", args.monkey_source / "include",
        args.monkey_source / "mk_core",
    ]
    common = [args.cc, "-std=gnu17", "-DMK_EVENT_LOOP_LIBEVENT", "-DMK_THREADS_POSIX",
              "-ffunction-sections", "-fdata-sections", sanitizer, "-g"]
    for include in includes:
        common.extend(["-I", str(include.resolve())])

    monkey_object = args.output / "mk_event.actual-libevent.o"
    monkey_dep = args.output / "mk_event.actual-libevent.d"
    monkey_command = common + ["-MMD", "-MF", str(monkey_dep), "-c",
                               str(event_source.resolve()), "-o", str(monkey_object)]
    run(monkey_command)
    write_json(args.output / "source-compile-commands.json", [{
        "directory": str(args.output.resolve()), "file": str(event_source.resolve()),
        "output": str(monkey_object.resolve()), "arguments": monkey_command,
    }])

    repository = pathlib.Path(__file__).resolve().parents[1]
    harness_source = repository / "compiler/examples/monkey_observation.c"
    harness_object = args.output / "cycle18_event_payload_observation.o"
    harness_dep = args.output / "cycle18_event_payload_observation.d"
    harness_command = common + [f'-DQCORE_MONKEY_SOURCE="{event_source.resolve()}"', '-DQCORE_CB_EVENT_ONLY',
                                "-MMD", "-MF", str(harness_dep), "-c",
                                str(harness_source), "-o", str(harness_object)]
    run(harness_command)
    executable = args.output / "cycle18_event_payload_observation"
    link_map = args.output / "cycle18_event_payload_observation.map"
    monkey_library = args.output / "mk_utils.o"
    utils_dep = args.output / "mk_utils.d"
    utils_command = common + ["-MMD", "-MF", str(utils_dep), "-c",
                              str((args.monkey_source / "mk_core/mk_utils.c").resolve()),
                              "-o", str(monkey_library)]
    run(utils_command)
    link_command = [args.cc, str(harness_object), str(monkey_library.resolve()), str(library.resolve()),
                    "-Wl,--gc-sections", f"-Wl,-Map={link_map}", sanitizer,
                    "-pthread", "-o", str(executable)]
    run(link_command)

    observations_path = args.output / "observations.jsonl"
    bindings_path = args.output / "bindings.jsonl"
    allocation_path = args.output / "allocation.json"
    runtime_path = args.output / "runtime.json"
    environment = os.environ.copy()
    environment.update(ASAN_OPTIONS="detect_leaks=1:halt_on_error=1",
                       UBSAN_OPTIONS="halt_on_error=1:print_stacktrace=1",
                       QCORE_MONKEY_BINDINGS=str(bindings_path.resolve()),
                       QCORE_CB_EVENT_ALLOCATION=str(allocation_path.resolve()),
                       QCORE_CB_EVENT_RUNTIME=str(runtime_path.resolve()))
    with observations_path.open("wb") as stream:
        run([str(executable.resolve())], env=environment, stdout=stream, timeout=30)
    records = [json.loads(line) for line in observations_path.read_text().splitlines()]
    bindings = [json.loads(line) for line in bindings_path.read_text().splitlines()]
    entered = [row for row in records if row["action"].get("operation") == "callback_enter"]
    exited = [row for row in records if row["action"].get("operation") == "callback_exit"]
    if len(entered) != 4 or len(exited) != 4 or len([row for row in bindings if row["kind"] == "callback"]) != 4:
        raise SystemExit("expected four complete native cb_event executions")
    if any(row["action"].get("callback") != "cb_event" for row in entered):
        raise SystemExit("unexpected callback in dedicated payload run")
    if {row["flags"] for row in bindings if row["kind"] == "callback"} != {1, 2, 4, 6}:
        raise SystemExit("native payload flags did not cover the four source paths")
    if records[-1]["action"]["kind"] != "completed":
        raise SystemExit("incomplete native run")
    dependencies = {}
    for dep in [monkey_dep, harness_dep, utils_dep]:
        import shlex
        contents = dep.read_text().replace("\\\n", " ").split(":", 1)[1]
        for name in shlex.split(contents):
            path = pathlib.Path(name).resolve()
            dependencies[str(path)] = digest(path)
    manifest_path = args.output / "run-manifest.json"
    write_json(manifest_path, {
        "schema_version": 1, "status": "observed_success",
        "profile": "cycle18.cb_event.single_dispatcher.v1",
        "roles": {k: str(v.resolve()) for k, v in {
            "monkey_inputs": args.monkey_inputs,
            "source_compile_database": args.output / "source-compile-commands.json",
            "configuration_object": monkey_object,
            "observations": observations_path, "bindings": bindings_path,
            "allocation": allocation_path, "runtime": runtime_path,
            "observer_binary": executable, "observer_harness": harness_source,
            "monkey_object": harness_object, "monkey_support_object": monkey_library,
            "libevent_library": library,
        }.items()},
        "host": {"system": platform.system(), "release": platform.release(),
                 "machine": platform.machine()},
        "compiler": {"path": args.cc, "version": output([args.cc, "--version"]).splitlines()[0]},
        "source_revision": MONKEY_REVISION, "libevent_revision": LIBEVENT_REVISION,
        "files": {str(path.resolve()): digest(path) for path in [
            args.monkey_inputs, event_source, backend_source, monkey_object,
            args.output / "source-compile-commands.json", library, monkey_library,
            args.libevent_build / "CMakeCache.txt", harness_source, harness_object,
            monkey_dep, harness_dep, executable, link_map,
            observations_path, bindings_path, allocation_path, runtime_path]},
        "dependencies": dependencies,
        "commands": {"cmake": cmake_command, "source_compile": monkey_command,
                     "harness_compile": harness_command, "support_compile": utils_command, "link": link_command},
        "observer": {"callback_stdio": True, "callback_snapshots": True,
                     "instrumentation_changes_timing": True,
                     "production_layout": True, "fault_injection": False,
                     "activation": "controlled event_active flags; not readiness or timer evidence",
                     "monkey_support_object_sanitized": True,
                     "callback_translation_unit_and_libevent_sanitized": True},
        "runtime": json.loads(runtime_path.read_text()),
        "scope": {"callback": "cb_event", "executions": 4,
                  "source_paths": 4, "capacity": 8, "dispatcher_count": 1,
                  "universal_safety_certificate": False},
    })
    print(manifest_path)


if __name__ == "__main__":
    main()
