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

"""Build and run the bounded Cycle 18 timed-wait observer on Linux."""

import argparse
import hashlib
import json
import os
import pathlib
import platform
import subprocess

LIBEVENT_REVISION = "d82464a277d0f42703702c4dfd9af6af38595a83"
MONKEY_REVISION = "66910c10a4d7eafa810b84229cf2dcf7b0f26f97"
KINDS = ["base_create", "event_new", "event_add", "dispatch_begin", "activate",
         "callback_enter", "callback_exit", "dispatch_end", "event_free",
         "argument_end", "base_free"]


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

    if platform.system() != "Linux":
        raise SystemExit("the first timed-wait observer profile supports Linux only")
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
    harness_source = repository / "compiler/examples/cycle18_timed_wait_observation.c"
    harness_object = args.output / "cycle18_timed_wait_observation.o"
    harness_dep = args.output / "cycle18_timed_wait_observation.d"
    harness_command = common + [f'-DQCORE_MONKEY_EVENT_SOURCE="{event_source.resolve()}"',
                                "-MMD", "-MF", str(harness_dep), "-c",
                                str(harness_source), "-o", str(harness_object)]
    run(harness_command)
    executable = args.output / "cycle18_timed_wait_observation"
    link_map = args.output / "cycle18_timed_wait_observation.map"
    link_command = [args.cc, str(harness_object), str(library.resolve()),
                    "-Wl,--gc-sections", f"-Wl,-Map={link_map}", sanitizer,
                    "-pthread", "-o", str(executable)]
    run(link_command)

    observations_path = args.output / "observations.jsonl"
    environment = os.environ.copy()
    environment["ASAN_OPTIONS"] = "detect_leaks=1:halt_on_error=1"
    environment["UBSAN_OPTIONS"] = "halt_on_error=1:print_stacktrace=1"
    with observations_path.open("wb") as stream:
        run([str(executable)], env=environment, stdout=stream, timeout=30)
    records = [json.loads(line) for line in observations_path.read_text().splitlines()]
    if len(records) != 11 or [record["action"]["kind"] for record in records] != KINDS:
        raise SystemExit("native observer did not emit the exact bounded trace")
    envelope = {(record["run"], record["buffer"], record["dispatcher"]) for record in records}
    if len(envelope) != 1:
        raise SystemExit("native observer crossed dispatcher contexts")
    run_id, buffer_id, dispatcher_id = envelope.pop()
    created = records[0]["action"]
    event_new = records[1]["action"]
    context = {
        "run": run_id, "buffer": buffer_id, "dispatcher": dispatcher_id,
        "base": created["base"], "event": event_new["event"],
        "argument": event_new["argument"], "callback": event_new["callback"],
        "backend": created["backend"], "flags": event_new["flags"],
        "priority": event_new["priority"],
    }
    source_apis = [
        {"api": "mk_event_wait_2", "path": "mk_core/mk_event.c", "sha256": digest(event_source)},
        {"api": "_mk_event_wait_2", "path": "mk_core/mk_event_libevent.c", "sha256": digest(backend_source)},
        {"api": "cb_wait_2_timeout", "path": "mk_core/mk_event_libevent.c", "sha256": digest(backend_source)},
    ]
    bindings_path = args.output / "bindings.json"
    write_json(bindings_path, {
        "schema_version": 1, "profile": "cycle18.timed_wait.single_dispatcher.v1",
        "observations": {"path": "observations.jsonl", "sha256": digest(observations_path),
                         "records": len(records)},
        "context": context, "source_apis": source_apis,
    })

    cache = args.libevent_build / "CMakeCache.txt"
    manifest_path = args.output / "run-manifest.json"
    write_json(manifest_path, {
        "schema_version": 1, "status": "observed_success",
        "host": {"system": platform.system(), "release": platform.release(),
                 "machine": platform.machine()},
        "compiler": {"path": args.cc, "version": output([args.cc, "--version"]).splitlines()[0]},
        "monkey": {"source_revision": MONKEY_REVISION,
                   "inputs_path": str(args.monkey_inputs.resolve()),
                   "inputs_sha256": digest(args.monkey_inputs),
                   "configuration_object": str(monkey_object.resolve()),
                   "configuration_object_sha256": digest(monkey_object),
                   "source_compile_commands": str((args.output / "source-compile-commands.json").resolve()),
                   "source_compile_commands_sha256": digest(args.output / "source-compile-commands.json"),
                   "dependency_file": str(monkey_dep.resolve()),
                   "dependency_file_sha256": digest(monkey_dep)},
        "libevent": {"source_revision": LIBEVENT_REVISION,
                     "source": str(args.libevent_source.resolve()),
                     "archive": str(library.resolve()), "archive_sha256": digest(library),
                     "cmake_cache": str(cache.resolve()), "cmake_cache_sha256": digest(cache),
                     "cmake_command": cmake_command, "sanitizers": ["address", "undefined"],
                     "thread_support_configured": True,
                     "thread_adapter_archive_linked": False,
                     "thread_adapter_initialized": False},
        "observer": {"collection": "thread-local fixed 11-record dispatcher-owned buffer",
                     "callback_allocation": False, "callback_locking": False,
                     "callback_stdio": False, "flush": "after event base free",
                     "source": str(harness_source), "source_sha256": digest(harness_source),
                     "object": str(harness_object.resolve()), "object_sha256": digest(harness_object),
                     "dependency_file": str(harness_dep.resolve()),
                     "dependency_file_sha256": digest(harness_dep),
                     "compile_command": harness_command, "link_command": link_command,
                     "link_map": str(link_map.resolve()), "link_map_sha256": digest(link_map),
                     "executable": str(executable.resolve()), "executable_sha256": digest(executable)},
        "outputs": {"observations": str(observations_path.resolve()),
                    "observations_sha256": digest(observations_path),
                    "bindings": str(bindings_path.resolve()),
                    "bindings_sha256": digest(bindings_path)},
        "scope": {"wrapper": "mk_event_wait_2", "timeout_milliseconds": 2,
                  "success_only": True, "dispatcher_count": 1,
                  "synchronization_edges": 0, "backend": context["backend"]},
    })
    print(manifest_path)


if __name__ == "__main__":
    main()
